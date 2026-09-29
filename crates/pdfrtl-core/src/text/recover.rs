//! Page text recovery: content stream → units → lines → logical order + reasons.
//!
//! The unit model is docs/problems/0002: inside `/ReversedChars BMC … EMC` a
//! `/Span<</ActualText …>>> Tj` is ONE unit (the decoded ActualText — authoritative,
//! returned verbatim, no NFKC folding: folding presentation forms belongs to the
//! search/index path, not to extraction) and a bare `<CID> Tj` is ONE unit PER CID,
//! decoded through the font's `ToUnicode`.
//!
//! Order reasoning, per visual line:
//! * a line with no `/ReversedChars` run stores logical order already — the stream
//!   order is emitted untouched (blanket reversal would corrupt it);
//! * a line with a `/ReversedChars` run stores VISUAL order: each reversed run's UNIT
//!   order is reversed (unit level, never character level — digits inside an LTR run
//!   stay unreversed) and the RUN order is reconstructed from UAX #9 levels;
//! * a reconstructed order is accepted only if running UAX #9 forward on it reproduces
//!   the observed stream order. If two orders both reproduce it, the paragraph's own
//!   direction (P2/P3) breaks the tie — and if that still leaves two, the line is
//!   refused with `Reason::UnsupportedNoEvidence` rather than guessed at.
use crate::reasons::Reason;
use crate::text::cmap::ToUnicode;
use crate::text::encoding::{BaseEncoding, Font, SimpleEncoding};
use crate::text::tokenizer::{parse_value, tokenize, Token, Value};
use anyhow::Result;
use lopdf::{Dictionary, Document, Object, ObjectId};
use std::collections::HashMap;
use unicode_bidi::{bidi_class, BidiClass, BidiInfo, Level};

/// Decompressed content cap per page: a page is text, not a bomb.
const MAX_PAGE_CONTENT: usize = 32 * 1024 * 1024;

/// Simple-font subtypes whose byte codes come from `/Encoding` (PDF spec §9.6.6).
const SIMPLE_FONT_SUBTYPES: &[&[u8]] = &[b"Type1", b"MMType1", b"TrueType"];

/// One page's recovered text plus the reasons that justify its order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageText {
    pub page: u32,
    pub text: String,
    pub reasons: Vec<Reason>,
    /// Characters this page DECODED but did not emit, because their order is not
    /// established. Only the count is kept: the honest "we read it but cannot yet
    /// order it" number survives, the unorderable characters are not handed out.
    pub unordered_chars: usize,
}

impl PageText {
    /// True when every glyph this page showed was decoded with a justification.
    /// A page that failed still carries whatever text it did recover — partial
    /// text is reported, never dropped (ADR 0004).
    pub fn is_decoded(&self) -> bool {
        !self.reasons.iter().any(|reason| reason.is_unsupported())
    }

    /// True when everything in `text` was put in reading order by a named rule.
    ///
    /// False for a page carrying `unsupported_visual_order`: its characters were
    /// decoded but their order was not established, so they live in
    /// [`PageText::unordered_chars`] and `text` is empty (ADR 0004 — extraction
    /// returns logical order or it fails; there is no third outcome).
    pub fn is_ordered(&self) -> bool {
        !self.reasons.contains(&Reason::UnsupportedVisualOrder)
    }

    /// Withdraw text whose order was never established. Runs after
    /// [`settle_rtl_order`], which is what can add the refusal.
    fn withdraw_unordered(&mut self) {
        if !self.is_ordered() && !self.text.is_empty() {
            self.unordered_chars = self.text.chars().count();
            self.text.clear();
        }
    }
}

/// Recover logical text (and its justification) from one content stream.
///
/// No producer fingerprint is applied here — `recover_text` is the pure
/// content-stream ladder. The producer rung is consulted by
/// [`extract_document`], which is where `/Producer` is known.
pub fn recover_text(stream: &[u8], fonts: &HashMap<Vec<u8>, Font>) -> (String, Vec<Reason>) {
    let recovered = recover(stream, fonts);
    (recovered.text, recovered.reasons)
}

/// One page's recovery result: text, its justifications, and the lines that are
/// still waiting for the producer rung of the ladder.
struct Recovered {
    text: String,
    reasons: Vec<Reason>,
    /// Indices (into the emitted line order) of lines that carry NO order evidence
    /// of their own — see [`settle_rtl_order`].
    pending: Vec<usize>,
}

fn recover(stream: &[u8], fonts: &HashMap<Vec<u8>, Font>) -> Recovered {
    assemble(walk(stream, fonts))
}

/// The units a producer wrote for one stream, in STREAM (visual) order — before any
/// recovery. Exposed so tests can pin the unit model itself.
pub fn stream_units(stream: &[u8], fonts: &HashMap<Vec<u8>, Font>) -> Vec<String> {
    walk(stream, fonts)
        .units
        .into_iter()
        .map(|unit| unit.text)
        .collect()
}

/// Recover every page of a loaded document, in page order.
///
/// Failures are **page-level**: a content stream we cannot decode costs us that
/// page (reported as `unsupported_page_content`), never the pages around it.
pub fn extract_document(doc: &Document) -> Result<Vec<PageText>> {
    let mut pages = Vec::new();
    let producer = producer_fingerprint(doc);
    for (page_num, page_id) in doc.get_pages() {
        let page = match doc.get_page_content_with_limit(page_id, MAX_PAGE_CONTENT) {
            Ok(content) => {
                let fonts = collect_fonts(doc, page_id);
                let recovered = recover(&content, &fonts);
                let mut page = PageText {
                    page: page_num,
                    text: recovered.text,
                    reasons: recovered.reasons,
                    unordered_chars: 0,
                };
                settle_rtl_order(&mut page, &producer, &recovered.pending);
                page.withdraw_unordered();
                page
            }
            // The document loaded; this one page did not. Keep the page in the
            // result with its own reason instead of failing the whole file.
            Err(_) => PageText {
                page: page_num,
                text: String::new(),
                reasons: vec![Reason::UnsupportedPageContent],
                unordered_chars: 0,
            },
        };
        pages.push(page);
    }
    Ok(pages)
}

/// The document's `/Producer` and `/Creator`, lowercased: the producer fingerprint
/// used to decide right-to-left order (ADR 0004).
fn producer_fingerprint(doc: &Document) -> String {
    let info = doc
        .trailer
        .get(b"Info")
        .ok()
        .and_then(|obj| obj.as_reference().ok())
        .and_then(|id| doc.get_object(id).ok())
        .and_then(|obj| obj.as_dict().ok());
    let get = |key: &[u8]| -> String {
        info.and_then(|dict| dict.get(key).ok())
            .and_then(|obj| obj.as_str().ok())
            .map(pdf_text_string)
            .unwrap_or_default()
    };
    let mut fingerprint = get(b"Producer");
    fingerprint.push(' ');
    fingerprint.push_str(&get(b"Creator"));
    fingerprint.to_ascii_lowercase()
}

/// A PDF text string: UTF-16BE when it carries a BOM (Microsoft Word writes its
/// `/Producer` that way), otherwise bytes read as UTF-8.
fn pdf_text_string(bytes: &[u8]) -> String {
    let utf16 = |be: bool| -> String {
        let mut units = Vec::with_capacity(bytes.len() / 2);
        let mut i = 2;
        while i + 1 < bytes.len() {
            let pair = [bytes[i], bytes[i + 1]];
            units.push(if be {
                u16::from_be_bytes(pair)
            } else {
                u16::from_le_bytes(pair)
            });
            i += 2;
        }
        String::from_utf16_lossy(&units)
    };
    if bytes.starts_with(&[0xFE, 0xFF]) {
        utf16(true)
    } else if bytes.starts_with(&[0xFF, 0xFE]) {
        utf16(false)
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

/// The storage convention a measured producer family writes its text in (ADR 0004).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderConvention {
    /// The stored sequence is the painted one: invert it to reach logical order.
    Visual,
    /// The stored sequence is already logical: invert nothing.
    Logical,
}

/// One measured producer family, keyed by its `/Producer` fingerprint.
///
/// Every entry MUST have a fixture in the corpus asserting its convention: the
/// coverage control in `crates/pdfrtl-core/tests/producer_allowlist.rs` enumerates
/// this table, so a family added here with no fixture fails the gate, and a fixture
/// deleted from the corpus fails it too rather than silently dropping out. That is
/// the rule docs/problems/0005 set for allow-list entries.
pub struct ProducerFamily {
    /// Stable id the coverage control matches fixtures against.
    pub id: &'static str,
    /// Every needle must appear in the lowercased fingerprint.
    pub all: &'static [&'static str],
    /// At least one needle must appear when the list is not empty.
    pub any: &'static [&'static str],
    pub convention: OrderConvention,
}

/// The whole fingerprint allow-list — the last rung of the order ladder.
pub const PRODUCER_ALLOW_LIST: &[ProducerFamily] = &[
    // Visual: measured on this corpus (ADR 0004) — every Word/InDesign RTL file
    // decodes to the mirror of what pdftotext and pypdf call logical, and none of
    // them carries `/ReversedChars`.
    ProducerFamily {
        id: "microsoft-word",
        all: &["microsoft", "word"],
        any: &[],
        convention: OrderConvention::Visual,
    },
    ProducerFamily {
        id: "adobe-indesign",
        all: &[],
        any: &["indesign"],
        convention: OrderConvention::Visual,
    },
    // Logical: Chromium's fixtures extract byte-exact with no inversion at all.
    ProducerFamily {
        id: "chromium-skia",
        all: &[],
        any: &["skia", "chrom"],
        convention: OrderConvention::Logical,
    },
];

/// The allow-listed convention for this fingerprint, if it is allow-listed at all.
fn producer_convention(fingerprint: &str) -> Option<OrderConvention> {
    PRODUCER_ALLOW_LIST
        .iter()
        .find(|family| {
            family.all.iter().all(|needle| fingerprint.contains(needle))
                && (family.any.is_empty()
                    || family.any.iter().any(|needle| fingerprint.contains(needle)))
        })
        .map(|family| family.convention)
}

/// Producers that store right-to-left text in *visual* order (ADR 0004).
fn stores_visual_order(fingerprint: &str) -> bool {
    producer_convention(fingerprint) == Some(OrderConvention::Visual)
}

/// Producers measured to store right-to-left text in *logical* order (ADR 0004).
fn stores_logical_order(fingerprint: &str) -> bool {
    producer_convention(fingerprint) == Some(OrderConvention::Logical)
}

fn is_rtl(ch: char) -> bool {
    let cp = ch as u32;
    matches!(
        cp,
        0x0590..=0x05FF
            | 0x0600..=0x06FF
            | 0x0750..=0x077F
            | 0x08A0..=0x08FF
            | 0xFB50..=0xFDFF
            | 0xFE70..=0xFEFF
    )
}

/// Combining marks and bidi/format controls that must stay glued to their base
/// when a line is inverted — reversing a base away from its marks corrupts the
/// text just as surely as reversing the line.
fn is_combining(ch: char) -> bool {
    let cp = ch as u32;
    matches!(
        cp,
        0x0300..=0x036F
            | 0x0483..=0x0489
            | 0x0591..=0x05C7
            | 0x0610..=0x061A
            | 0x064B..=0x065F
            | 0x0670
            | 0x06D6..=0x06DC
            | 0x06DF..=0x06E4
            | 0x06E7..=0x06E8
            | 0x06EA..=0x06ED
            | 0x0711
            | 0x0730..=0x074A
            | 0x07A6..=0x07B0
            | 0x0900..=0x0903
            | 0x093A..=0x094F
            | 0x0951..=0x0957
            | 0x0E31
            | 0x0E34..=0x0E3A
            | 0x0E47..=0x0E4E
            | 0x200C..=0x200F
            | 0x202A..=0x202E
            | 0x2060..=0x2064
            | 0xFE00..=0xFE0F
            | 0xFE20..=0xFE2F
    )
}

/// A character that can *start* an embedded left-to-right run: Latin/Greek
/// letters and digits (never a right-to-left one).
fn ltr_start(ch: char) -> bool {
    ch.is_alphanumeric() && !is_rtl(ch) && (ch as u32) < 0x0900
}

/// A unit whose text begins with a combining/format character: it belongs to the
/// preceding unit's cluster and has to move with it when the line is inverted.
fn starts_combining(text: &str) -> bool {
    text.chars().next().is_some_and(is_combining)
}

/// Characters allowed *inside* an LTR run: the ASCII punctuation that occurs in
/// Latin tokens (URLs, decimals, dates).
fn ltr_run_char(ch: char) -> bool {
    matches!(
        ch,
        '.' | ',' | ':' | '/' | '-' | '_' | '@' | '#' | '%' | '?' | '!' | '&' | '=' | '+' | '\''
    ) || ltr_start(ch)
}

/// A line worth settling: at least two right-to-left letters (a stray mark is not
/// evidence of an RTL run).
fn has_rtl_run(text: &str) -> bool {
    text.chars().filter(|ch| is_rtl(*ch)).count() >= 2
}

/// Visual -> logical for one line of base-direction RTL: reverse the line in
/// clusters (base + combining marks), then put embedded left-to-right runs back
/// into reading order. This is the inverse of the UAX #9 L2 reversal the renderer
/// performed when it drew the line left to right.
fn visual_to_logical(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    if !chars.iter().any(|ch| is_rtl(*ch)) {
        return line.to_string();
    }

    // Cluster: a base character with the marks/format controls glued to it.
    let mut clusters: Vec<Vec<char>> = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let mut j = i + 1;
        while j < chars.len() && is_combining(chars[j]) {
            j += 1;
        }
        clusters.push(chars[i..j].to_vec());
        i = j;
    }
    clusters.reverse();

    let starts_ltr = |cluster: &[char]| cluster.first().is_some_and(|ch| ltr_start(*ch));
    let is_space = |cluster: &[char]| cluster.first() == Some(&' ');
    let ltr_after_space = |clusters: &[Vec<char>], index: usize| {
        let mut next = index;
        while next < clusters.len() && is_space(&clusters[next]) {
            next += 1;
        }
        next < clusters.len() && starts_ltr(&clusters[next])
    };

    let mut k = 0;
    while k < clusters.len() {
        if starts_ltr(&clusters[k]) {
            let mut end = k + 1;
            while end < clusters.len() {
                let head = clusters[end].first().copied();
                if head.is_some_and(ltr_run_char)
                    || (is_space(&clusters[end]) && ltr_after_space(&clusters, end))
                {
                    end += 1;
                } else {
                    break;
                }
            }
            clusters[k..end].reverse();
            k = end;
        } else {
            k += 1;
        }
    }

    clusters.into_iter().flatten().collect()
}

/// Rung 3 of the order ladder — the UAX #9 comparison.
///
/// For a line with no order evidence of its own (no `/ReversedChars`, more than one
/// unit) we do not ask *who* produced the file first; we ask the file itself. Two
/// readings are possible, and both are checked by running the bidi algorithm
/// FORWARD over the hypothesised logical order and seeing which one reproduces the
/// sequence the producer actually painted:
///
/// * `Keep`    — the computation reproduces the painting from the stored order, so
///               the stored order **is** logical: no inversion.
/// * `Invert`  — only the mirrored reading reproduces it, so the producer stored
///               the painted (visual) sequence: invert, once, and the result is
///               verified the same way.
/// * `Ambiguous` — both readings reproduce the painting. That is a coin flip, and
///               a coin flip is banned: the answer is REFUSE (the line falls
///               through to the producer fingerprint, and is refused outright when
///               the fingerprint does not know the family). It happens because one
///               line does not say which base direction it was written in — an
///               English line with an Arabic word and an Arabic line with an
///               English word paint identically.
/// * `Unexplained` — neither reading reproduces the painting: the file contradicts
///               the model, so we refuse instead of picking a side.
///
/// Both outcomes are `false` in the ladder — no third outcome exists (ADR 0002).
#[derive(Debug)]
enum LineOrder {
    Keep,
    Invert(Vec<usize>),
    Ambiguous,
    Unexplained,
}

/// Compare a line's stored sequence with its painted sequence under UAX #9.
///
/// The painted order is read from placement: units are ordered by their painted x,
/// and units that share one text origin keep stream order — showing a string is a
/// single operation that advances the pen through the glyphs in emission order
/// (ISO 32000-1 §9.4.4), so there is nothing else for the origin to disagree with.
/// When the composed matrix runs right-to-left (`x_ok` is false) x orders the line
/// backwards and the rung reports no opinion instead of a wrong one.
fn settle_line_by_bidi(units: &[Unit]) -> LineOrder {
    let count = units.len();
    if count < 2 || units.iter().any(|unit| !unit.x_ok) {
        return LineOrder::Unexplained;
    }

    // The painted (visual) order: left to right by position, ties in stream order.
    let mut painted: Vec<usize> = (0..count).collect();
    painted.sort_by(|&left, &right| {
        units[left]
            .x
            .total_cmp(&units[right].x)
            .then(left.cmp(&right))
    });

    let identity: Vec<usize> = (0..count).collect();
    let inverted = invert_units(units);
    // The visual reading is a claim about the FILE: "the producer stored what it
    // painted". Only the positions can support it — when the painted order and the
    // stored order disagree, the producer demonstrably did not store what it
    // painted, so the claim is refuted before any bidi run gets to rescue it.
    let stored_is_painted = painted == identity;
    let keeps = predicts_painted(units, &identity, &painted);
    let inverts =
        stored_is_painted && inverted != identity && predicts_painted(units, &inverted, &painted);
    match (keeps, inverts) {
        (true, false) => LineOrder::Keep,
        (false, true) => LineOrder::Invert(inverted),
        (true, true) => LineOrder::Ambiguous,
        (false, false) => LineOrder::Unexplained,
    }
}

/// Run UAX #9 forward over a hypothesised logical order and check that the levels
/// the algorithm computes paint back into `painted`.
///
/// The paragraph direction is NOT ours to pick: UAX #9 P2/P3 derives it from the
/// hypothesis's own first strong character, so each reading is judged as the
/// complete paragraph it claims to be. Two readings that each satisfy the standard
/// this way are the `Ambiguous` case — a coin flip, which is refused.
fn predicts_painted(units: &[Unit], logical: &[usize], painted: &[usize]) -> bool {
    let proxy: String = logical
        .iter()
        .map(|&i| proxy_char(&units[i].text))
        .collect();
    painted_map(&proxy, logical) == Some(painted.to_vec())
}

/// The visual order UAX #9 paints for a hypothesised logical order: `map[j]` is the
/// logical offset shown at painted position `j`. `None` when the proxy and the
/// logical order disagree in length (a malformed hypothesis, not an answer).
fn painted_map(proxy: &str, logical: &[usize]) -> Option<Vec<usize>> {
    let info = BidiInfo::new(proxy, None);
    let para = info.paragraphs.first()?;
    let levels = info.reordered_levels_per_char(para, 0..proxy.len());
    if levels.len() != logical.len() {
        return None;
    }
    let map = BidiInfo::reorder_visual(&levels);
    if map.len() != logical.len() {
        return None;
    }
    Some(map.into_iter().map(|position| logical[position]).collect())
}

/// Visual -> logical for one line of base-direction RTL, at UNIT granularity:
/// reverse the unit order in clusters (a base unit plus the combining/format units
/// glued to it), then put embedded left-to-right unit runs back into reading order.
///
/// Unit granularity, not character granularity: a lam-alef ligature arrives as ONE
/// unit (one code, two characters) and has to stay whole — reversing its
/// characters would turn `سلام` into `سالم`, the silent corruption ADR 0002 bans.
fn invert_units(units: &[Unit]) -> Vec<usize> {
    if !units.iter().any(|unit| unit.text.chars().any(is_rtl)) {
        return (0..units.len()).collect();
    }

    let mut clusters: Vec<Vec<usize>> = Vec::new();
    let mut index = 0;
    while index < units.len() {
        let mut end = index + 1;
        while end < units.len() && starts_combining(&units[end].text) {
            end += 1;
        }
        clusters.push((index..end).collect());
        index = end;
    }
    clusters.reverse();

    let first_char = |cluster: &[usize]| {
        cluster
            .first()
            .and_then(|&unit| units[unit].text.chars().next())
    };
    let starts_ltr = |cluster: &[usize]| first_char(cluster).is_some_and(ltr_start);
    let is_space = |cluster: &[usize]| first_char(cluster) == Some(' ');
    let ltr_after_space = |clusters: &[Vec<usize>], position: usize| {
        let mut next = position;
        while next < clusters.len() && is_space(&clusters[next]) {
            next += 1;
        }
        next < clusters.len() && starts_ltr(&clusters[next])
    };

    let mut at = 0;
    while at < clusters.len() {
        if starts_ltr(&clusters[at]) {
            let mut end = at + 1;
            while end < clusters.len() {
                let head = first_char(&clusters[end]);
                if head.is_some_and(ltr_run_char)
                    || (is_space(&clusters[end]) && ltr_after_space(&clusters, end))
                {
                    end += 1;
                } else {
                    break;
                }
            }
            clusters[at..end].reverse();
            at = end;
        } else {
            at += 1;
        }
    }

    clusters.into_iter().flatten().collect()
}

/// Settle the lines that carry no order evidence of their own, using the producer.
///
/// `pending` names exactly those lines: a right-to-left run of two or more units
/// that the producer did **not** wrap in `/ReversedChars`, and for which the UAX #9
/// comparison (`settle_line_by_bidi`) was not decisive. Every other line on the
/// page is already settled, and by one of three page-local facts:
///
/// * the producer marked the run as mirrored (`/ReversedChars`) — `recover_line`
///   inverted the UNIT order and verified the result against UAX #9;
/// * the line is a single unit — one element has exactly one order, so there is no
///   ordering decision left for anyone to get wrong (this is what the synthetic
///   `/ActualText` fixture is: one marked sequence per line);
/// * rung 3 verified the stored order against the file's own painted positions.
///
/// Consulting the fingerprint for the whole page instead — the previous behaviour —
/// both refused pages that were already settled and re-inverted lines that were
/// already settled. The fingerprint is the LAST rung of the ladder, not the first
/// (ADR 0004).
fn settle_rtl_order(page: &mut PageText, fingerprint: &str, pending: &[usize]) {
    if pending.is_empty() {
        return;
    }
    if stores_visual_order(fingerprint) {
        let mut lines: Vec<String> = page.text.split('\n').map(str::to_string).collect();
        for &index in pending {
            if let Some(line) = lines.get_mut(index) {
                if has_rtl_run(line) {
                    *line = visual_to_logical(line);
                }
            }
        }
        page.text = lines.join("\n");
        page.reasons
            .retain(|reason| !matches!(reason, Reason::ToUnicodeLogical));
        if !page.reasons.contains(&Reason::ProducerVisualOrderKnown) {
            page.reasons.push(Reason::ProducerVisualOrderKnown);
        }
    } else if stores_logical_order(fingerprint) {
        return;
    } else {
        page.reasons
            .retain(|reason| !matches!(reason, Reason::ToUnicodeLogical | Reason::BidiReordered));
        page.reasons.push(Reason::UnsupportedVisualOrder);
    }
    page.reasons.sort_by_key(|reason| *reason as u32);
}

// --- the walk -------------------------------------------------------------

/// One unit of producer evidence on a visual line.
#[derive(Debug, Clone)]
struct Unit {
    text: String,
    /// Quantised text-line origin (the matrix `f`), grouping units onto one line.
    line: i64,
    /// Painted x of this unit's text origin (CTM ∘ text matrix), the position the
    /// glyphs are actually drawn from. The UAX #9 rung reads the painted run order
    /// from it — see [`settle_line_by_bidi`].
    x: f64,
    /// False when the composed matrix does not run left-to-right (`a ≤ 0`): then x
    /// no longer orders the glyphs the way they paint, and the rung must not read it.
    x_ok: bool,
    /// Inside `/ReversedChars`: the producer stored this run in visual order.
    reversed: bool,
    /// Bumped by each `Tm`; a new text matrix starts a new run.
    epoch: u64,
}

struct Walk {
    units: Vec<Unit>,
    used_actual_text: bool,
    used_to_unicode: bool,
    used_encoding: bool,
    undecodable: bool,
    refused_encoding: bool,
}

#[derive(Clone, Copy)]
struct Mat {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    e: f64,
    f: f64,
}

impl Mat {
    fn identity() -> Mat {
        Mat {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: 0.0,
            f: 0.0,
        }
    }

    /// `T(tx,ty) × self` — PDF composes row-vector matrices on the left.
    fn translated(&self, tx: f64, ty: f64) -> Mat {
        Mat {
            a: self.a,
            b: self.b,
            c: self.c,
            d: self.d,
            e: tx * self.a + ty * self.c + self.e,
            f: tx * self.b + ty * self.d + self.f,
        }
    }

    /// `self × rhs` for row-vector matrices: a point goes through `rhs` first, then
    /// `self`. `cm` uses this to pre-multiply the CTM (ISO 32000-1 §8.3.3), and the
    /// painted x of a text origin needs the CTM to be right: a page-level flip or
    /// scale would otherwise reverse the very run order the UAX #9 rung compares.
    fn multiplied(&self, rhs: &Mat) -> Mat {
        Mat {
            a: self.a * rhs.a + self.b * rhs.c,
            b: self.a * rhs.b + self.b * rhs.d,
            c: self.c * rhs.a + self.d * rhs.c,
            d: self.c * rhs.b + self.d * rhs.d,
            e: self.e * rhs.a + self.f * rhs.c + rhs.e,
            f: self.e * rhs.b + self.f * rhs.d + rhs.f,
        }
    }
}

struct Mark {
    tag: Vec<u8>,
    actual_text: Option<String>,
}

struct Walker<'a> {
    fonts: &'a HashMap<Vec<u8>, Font>,
    marks: Vec<Mark>,
    font: Option<Vec<u8>>,
    line: Mat,
    /// Graphics-state matrix (`q`/`Q`/`cm`). Only its x-direction is used: to read
    /// the painted order of a line's runs from their positions, the composed
    /// mapping from text space to device space has to be the real one.
    ctm: Mat,
    ctm_stack: Vec<Mat>,
    leading: f64,
    epoch: u64,
    units: Vec<Unit>,
    used_actual_text: bool,
    used_to_unicode: bool,
    used_encoding: bool,
    undecodable: bool,
    refused_encoding: bool,
}

fn num(ops: &[Value], index: usize) -> Option<f64> {
    ops.get(index).and_then(|value| match value {
        Value::Num(n) => Some(*n),
        _ => None,
    })
}

impl<'a> Walker<'a> {
    fn new(fonts: &'a HashMap<Vec<u8>, Font>) -> Walker<'a> {
        Walker {
            fonts,
            marks: Vec::new(),
            font: None,
            line: Mat::identity(),
            ctm: Mat::identity(),
            ctm_stack: Vec::new(),
            leading: 0.0,
            epoch: 0,
            units: Vec::new(),
            used_actual_text: false,
            used_to_unicode: false,
            used_encoding: false,
            undecodable: false,
            refused_encoding: false,
        }
    }

    fn op(&mut self, op: &[u8], ops: &[Value]) {
        match op {
            b"BT" => self.line = Mat::identity(),
            b"q" => self.ctm_stack.push(self.ctm),
            b"Q" => {
                if let Some(saved) = self.ctm_stack.pop() {
                    self.ctm = saved;
                }
            }
            b"cm" => {
                if let (Some(a), Some(b), Some(c), Some(d), Some(e), Some(f)) = (
                    num(ops, 0),
                    num(ops, 1),
                    num(ops, 2),
                    num(ops, 3),
                    num(ops, 4),
                    num(ops, 5),
                ) {
                    // CTM' = cm × CTM (ISO 32000-1 §8.3.3).
                    self.ctm = Mat { a, b, c, d, e, f }.multiplied(&self.ctm);
                }
            }
            b"Tm" => {
                if let (Some(a), Some(b), Some(c), Some(d), Some(e), Some(f)) = (
                    num(ops, 0),
                    num(ops, 1),
                    num(ops, 2),
                    num(ops, 3),
                    num(ops, 4),
                    num(ops, 5),
                ) {
                    self.line = Mat { a, b, c, d, e, f };
                    self.epoch += 1;
                }
            }
            b"Td" => {
                if let (Some(tx), Some(ty)) = (num(ops, 0), num(ops, 1)) {
                    self.line = self.line.translated(tx, ty);
                }
            }
            b"TD" => {
                if let (Some(tx), Some(ty)) = (num(ops, 0), num(ops, 1)) {
                    self.leading = -ty;
                    self.line = self.line.translated(tx, ty);
                }
            }
            b"T*" => self.line = self.line.translated(0.0, -self.leading),
            b"TL" => {
                if let Some(value) = num(ops, 0) {
                    self.leading = value;
                }
            }
            b"Tf" => {
                self.font = match ops.first() {
                    Some(Value::Name(name)) => Some(name.clone()),
                    _ => None,
                };
            }
            b"Tj" => {
                if let Some(Value::Str(bytes)) = ops.first() {
                    self.show(bytes);
                }
            }
            b"TJ" => {
                if let Some(Value::Arr(items)) = ops.first() {
                    for item in items {
                        if let Value::Str(bytes) = item {
                            self.show(bytes);
                        }
                    }
                }
            }
            b"'" => {
                self.line = self.line.translated(0.0, -self.leading);
                if let Some(Value::Str(bytes)) = ops.first() {
                    self.show(bytes);
                }
            }
            b"\"" => {
                if let Some(Value::Str(bytes)) = ops.get(2) {
                    self.line = self.line.translated(0.0, -self.leading);
                    self.show(bytes);
                }
            }
            b"BMC" => {
                if let Some(Value::Name(tag)) = ops.first() {
                    self.marks.push(Mark {
                        tag: tag.clone(),
                        actual_text: None,
                    });
                }
            }
            b"BDC" => {
                if let (Some(Value::Name(tag)), Some(value)) = (ops.first(), ops.get(1)) {
                    let actual_text = match value {
                        Value::Dict(entries) => dict_actual_text(entries),
                        _ => None,
                    };
                    self.marks.push(Mark {
                        tag: tag.clone(),
                        actual_text,
                    });
                }
            }
            b"EMC" => {
                self.marks.pop();
            }
            _ => {}
        }
    }

    /// One `Tj`/`TJ` element → zero or more units.
    fn show(&mut self, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        // /ActualText wins: the innermost marked sequence carries the logical text
        // verbatim. Presentation forms in ToUnicode are NOT folded here.
        let actual = self
            .marks
            .iter()
            .rev()
            .find_map(|mark| mark.actual_text.clone());
        if let Some(text) = actual.filter(|text| !text.is_empty()) {
            self.used_actual_text = true;
            self.push_unit(text);
            return;
        }
        let fonts = self.fonts;
        let font = match self.font.as_ref().and_then(|name| fonts.get(name)) {
            Some(font) => font,
            None => {
                // No entry for this font name: either the resource is missing or the
                // font has no decodable mapping at all (a CID font without `/ToUnicode`).
                self.undecodable = true;
                return;
            }
        };
        match font {
            // Encoding we refuse to interpret: `/Symbol`, `/ZapfDingbats`,
            // `/MacExpertEncoding`, an unknown name, or no `/Encoding` at all.
            // Their byte maps are not Unicode — say so instead of inventing text.
            Font::Refused => self.refused_encoding = true,
            Font::Simple(encoding) => {
                for &code in bytes {
                    match encoding.decode(code) {
                        Some(text) => {
                            self.used_encoding = true;
                            if !text.is_empty() {
                                self.push_unit(text);
                            }
                        }
                        // Undefined slot or a `/Differences` name with no Unicode
                        // mapping we can justify: an explicit refusal, per code.
                        None => self.refused_encoding = true,
                    }
                }
            }
            Font::ToUnicode(cmap) => {
                let width = cmap.code_len();
                if width == 0 || width > 2 {
                    self.undecodable = true;
                    return;
                }
                let mut i = 0usize;
                while i < bytes.len() {
                    if i + width > bytes.len() {
                        // A trailing half-code is not decodable; say so instead of inventing.
                        self.undecodable = true;
                        break;
                    }
                    let code = match &bytes[i..i + width] {
                        [b] => u16::from(*b),
                        [hi, lo] => u16::from_be_bytes([*hi, *lo]),
                        _ => {
                            self.undecodable = true;
                            break;
                        }
                    };
                    match cmap.get(code) {
                        Some(text) => {
                            self.used_to_unicode = true;
                            if !text.is_empty() {
                                self.push_unit(text.to_string());
                            }
                        }
                        None => {
                            self.undecodable = true;
                        }
                    }
                    i += width;
                }
            }
        }
    }

    fn push_unit(&mut self, text: String) {
        let reversed = self.marks.iter().any(|mark| mark.tag == b"ReversedChars");
        let line = (self.line.f * 1000.0).round() as i64;
        // Painted x of this text origin, plus whether the composed matrix still runs
        // left-to-right (a <= 0 mirrors the line, so x would order it backwards).
        let x = self.ctm.a * self.line.e + self.ctm.c * self.line.f + self.ctm.e;
        let x_scale = self.ctm.a * self.line.a + self.ctm.c * self.line.b;
        self.units.push(Unit {
            text,
            line,
            x,
            x_ok: x_scale > 0.0 && x.is_finite(),
            reversed,
            epoch: self.epoch,
        });
    }

    fn finish(self) -> Walk {
        Walk {
            units: self.units,
            used_actual_text: self.used_actual_text,
            used_to_unicode: self.used_to_unicode,
            used_encoding: self.used_encoding,
            undecodable: self.undecodable,
            refused_encoding: self.refused_encoding,
        }
    }
}

fn dict_actual_text(entries: &[(Vec<u8>, Value)]) -> Option<String> {
    entries
        .iter()
        .find(|(key, _)| key == b"ActualText")
        .and_then(|(_, value)| match value {
            Value::Str(bytes) => Some(decode_text_string(bytes)),
            _ => None,
        })
}

/// PDF text string: UTF-16BE with BOM, UTF-16LE with BOM, else UTF-8, else byte-per-char.
fn decode_text_string(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0xFE, 0xFF]) {
        return lossy_utf16(&bytes[2..], true);
    }
    if bytes.starts_with(&[0xFF, 0xFE]) {
        return lossy_utf16(&bytes[2..], false);
    }
    match std::str::from_utf8(bytes) {
        Ok(text) => text.to_string(),
        Err(_) => bytes.iter().map(|&b| b as char).collect(),
    }
}

fn lossy_utf16(bytes: &[u8], big_endian: bool) -> String {
    // Explicit index stepping, not `chunks_exact(2)`: a trailing odd byte is dropped
    // exactly as `chunks_exact` dropped it — that policy is pinned by a test.
    let mut units: Vec<u16> = Vec::with_capacity(bytes.len() / 2);
    let mut i = 0usize;
    while i + 1 < bytes.len() {
        let pair = [bytes[i], bytes[i + 1]];
        units.push(if big_endian {
            u16::from_be_bytes(pair)
        } else {
            u16::from_le_bytes(pair)
        });
        i += 2;
    }
    char::decode_utf16(units)
        .map(|r| r.unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect()
}

// --- token flattening -----------------------------------------------------

enum Item {
    Value(Value),
    Op(Vec<u8>),
}

fn flatten(tokens: &[Token]) -> Vec<Item> {
    let mut items = Vec::new();
    let mut i = 0usize;
    while i < tokens.len() {
        match &tokens[i] {
            Token::Op(op) => {
                items.push(Item::Op(op.clone()));
                i += 1;
            }
            Token::ArrStart | Token::DictStart => {
                let before = i;
                match parse_value(tokens, &mut i) {
                    Some(value) => items.push(Item::Value(value)),
                    None => {
                        if i == before {
                            i += 1;
                        }
                    }
                }
            }
            Token::Name(name) => {
                items.push(Item::Value(Value::Name(name.clone())));
                i += 1;
            }
            Token::Num(n) => {
                items.push(Item::Value(Value::Num(*n)));
                i += 1;
            }
            Token::Str(bytes) => {
                items.push(Item::Value(Value::Str(bytes.clone())));
                i += 1;
            }
            Token::ArrEnd | Token::DictEnd | Token::Reference(..) => i += 1,
        }
    }
    items
}

fn walk(stream: &[u8], fonts: &HashMap<Vec<u8>, Font>) -> Walk {
    let tokens = tokenize(stream);
    let items = flatten(&tokens);
    let mut walker = Walker::new(fonts);
    let mut pending: Vec<Value> = Vec::new();
    for item in &items {
        match item {
            Item::Op(op) => {
                walker.op(op, &pending);
                pending.clear();
            }
            Item::Value(value) => pending.push(value.clone()),
        }
    }
    walker.finish()
}

// --- order recovery -------------------------------------------------------

struct Flags {
    actual_text: bool,
    to_unicode: bool,
    encoded: bool,
    producer_visual: bool,
    /// Rung 3 decided this page: the stored order was compared against the order
    /// the producer painted, with UAX #9 (`settle_line_by_bidi`).
    bidi_verified: bool,
    reconstructed: bool,
    refused: bool,
    undecodable: bool,
    refused_encoding: bool,
}

fn assemble(walk: Walk) -> Recovered {
    let mut flags = Flags {
        actual_text: walk.used_actual_text,
        to_unicode: walk.used_to_unicode,
        encoded: walk.used_encoding,
        producer_visual: walk.units.iter().any(|unit| unit.reversed),
        bidi_verified: false,
        reconstructed: false,
        refused: false,
        undecodable: walk.undecodable,
        refused_encoding: walk.refused_encoding,
    };

    // Group units onto visual lines by text-line origin, keeping first-appearance order.
    let units = walk.units;
    let mut keys: Vec<i64> = Vec::new();
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for (index, unit) in units.iter().enumerate() {
        match keys.iter().position(|&key| key == unit.line) {
            Some(at) => groups[at].push(index),
            None => {
                keys.push(unit.line);
                groups.push(vec![index]);
            }
        }
    }

    let mut lines: Vec<String> = Vec::new();
    let mut pending: Vec<usize> = Vec::new();
    for group in &groups {
        let slice: Vec<Unit> = group.iter().map(|&i| units[i].clone()).collect();
        // Does this line still owe the ladder an order justification?
        //   * one unit  → one element, one order: nothing to decide;
        //   * `/ReversedChars` → the producer told us, `recover_line` verified it;
        //   * two or more units in an unmarked RTL line → rung 3, the UAX #9
        //     comparison against the painted order ([`settle_line_by_bidi`]);
        //   * only when that comparison is not decisive does the line fall through
        //     to the producer fingerprint (`settle_rtl_order`).
        let marked = slice.iter().any(|unit| unit.reversed);
        let undecided = slice.len() > 1 && !marked;
        match recover_line(&slice) {
            Some((mut text, rebuilt)) => {
                if rebuilt {
                    flags.reconstructed = true;
                }
                if undecided && has_rtl_run(&text) {
                    match settle_line_by_bidi(&slice) {
                        LineOrder::Keep => flags.bidi_verified = true,
                        LineOrder::Invert(order) => {
                            text = order.iter().map(|&i| slice[i].text.as_str()).collect();
                            flags.bidi_verified = true;
                            flags.reconstructed = true;
                        }
                        // Neither reading reproduces the painting — or both do, which
                        // would be a coin flip. The line is NOT guessed here: it goes
                        // to the producer rung, which either knows the family or
                        // refuses the line with an explicit reason.
                        LineOrder::Ambiguous | LineOrder::Unexplained => {
                            pending.push(lines.len());
                        }
                    }
                }
                lines.push(text);
            }
            None => {
                flags.refused = true;
                lines.push(String::new());
            }
        }
    }
    Recovered {
        text: lines.join("\n"),
        reasons: build_reasons(flags),
        pending,
    }
}

fn build_reasons(flags: Flags) -> Vec<Reason> {
    let mut reasons = Vec::new();
    if flags.actual_text {
        reasons.push(Reason::ActualText);
    }
    if flags.reconstructed {
        // Run order was reconstructed from UAX #9 levels: that supersedes the
        // "already logical" claim of ToUnicodeLogical for this page.
        reasons.push(Reason::BidiReordered);
    } else if flags.to_unicode {
        reasons.push(Reason::ToUnicodeLogical);
    }
    if flags.encoded {
        // Codes came from the font's own /Encoding, not from a ToUnicode CMap.
        reasons.push(Reason::EncodingMapped);
    }
    if flags.producer_visual {
        reasons.push(Reason::ProducerVisualOrderKnown);
    }
    if flags.bidi_verified {
        // Rung 3 established the order: the stored sequence matched the file's own
        // painted positions under UAX #9 (kept as logical, or inverted when the
        // comparison proved the mirror). The most specific rule, so it is last.
        reasons.push(Reason::BidiVerified);
    }
    if flags.refused {
        reasons.push(Reason::UnsupportedNoEvidence);
    }
    if flags.refused_encoding {
        reasons.push(Reason::UnsupportedFontEncoding);
    }
    if flags.undecodable {
        reasons.push(Reason::UnsupportedBrokenToUnicode);
    }
    reasons
}

/// Recover one visual line. `Some((text, reconstructed))`, or `None` when no candidate
/// order can be justified — the caller then refuses the line instead of guessing.
fn recover_line(units: &[Unit]) -> Option<(String, bool)> {
    // No /ReversedChars anywhere: the producer stored logical order already.
    if !units.iter().any(|unit| unit.reversed) {
        let text = units.iter().map(|unit| unit.text.as_str()).collect();
        return Some((text, false));
    }

    let (runs, run_reversed) = split_runs(units);
    let texts: Vec<&str> = units.iter().map(|unit| unit.text.as_str()).collect();
    let join = |order: &[usize]| -> Vec<usize> {
        let mut out = Vec::new();
        for &run in order {
            let mut indices = runs[run].clone();
            if run_reversed[run] {
                indices.reverse();
            }
            out.extend(indices);
        }
        out
    };

    let identity: Vec<usize> = (0..runs.len()).collect();
    let mut flipped = identity.clone();
    flipped.reverse();

    // Candidate run orders: L2 at run granularity (both base directions, both index
    // conventions), plus the identity and full-flip extremes.
    let mut candidates: Vec<(Vec<usize>, bool)> = Vec::new();
    for base_rtl in [false, true] {
        let levels: Vec<Level> = (0..runs.len())
            .map(|run| {
                let text: String = join(&[run]).iter().map(|&i| texts[i]).collect();
                run_level(&text, base_rtl)
            })
            .collect();
        let map = BidiInfo::reorder_visual(&levels);
        let mut inverted = vec![0usize; map.len()];
        for (at, &logical) in map.iter().enumerate() {
            if logical < inverted.len() {
                inverted[logical] = at;
            }
        }
        candidates.push((map, base_rtl));
        candidates.push((inverted, base_rtl));
    }
    for order in [&identity, &flipped] {
        for base_rtl in [false, true] {
            candidates.push((order.clone(), base_rtl));
        }
    }

    let mut verified: Vec<(Vec<usize>, bool)> = Vec::new();
    for (order, base_rtl) in candidates {
        let logical = join(&order);
        if verified
            .iter()
            .any(|(seen, seen_base)| *seen == logical && *seen_base == base_rtl)
        {
            continue;
        }
        if predicts_observed(&texts, &logical, base_rtl) {
            verified.push((logical, base_rtl));
        }
    }
    if verified.is_empty() {
        return None;
    }

    let mut distinct: Vec<Vec<usize>> = Vec::new();
    for (order, _) in &verified {
        if !distinct.contains(order) {
            distinct.push(order.clone());
        }
    }

    let chosen = if distinct.len() == 1 {
        distinct.pop()?
    } else {
        // Two orders both reproduce the observed visual under some base direction:
        // only the paragraph's own direction (UAX #9 P2/P3) may break the tie.
        let mut kept: Vec<Vec<usize>> = Vec::new();
        for order in &distinct {
            let text: String = order.iter().map(|&i| texts[i]).collect();
            let base = auto_base_rtl(&text);
            if verified
                .iter()
                .any(|(seen, seen_base)| seen == order && *seen_base == base)
                && !kept.contains(order)
            {
                kept.push(order.clone());
            }
        }
        if kept.len() != 1 {
            return None;
        }
        kept.pop()?
    };

    let rebuilt = chosen != join(&identity);
    let text = chosen.iter().map(|&i| texts[i]).collect();
    Some((text, rebuilt))
}

/// Split a line into runs: a new run starts when the producer switches in or out of
/// `/ReversedChars` or starts a new text matrix (`Tm`).
fn split_runs(units: &[Unit]) -> (Vec<Vec<usize>>, Vec<bool>) {
    let mut runs: Vec<Vec<usize>> = Vec::new();
    let mut reversed: Vec<bool> = Vec::new();
    let mut current: Vec<usize> = Vec::new();
    let mut current_reversed = false;
    let mut current_epoch = 0u64;
    for (index, unit) in units.iter().enumerate() {
        if index == 0 || unit.reversed != current_reversed || unit.epoch != current_epoch {
            if index > 0 {
                runs.push(std::mem::take(&mut current));
                reversed.push(current_reversed);
            }
            current_reversed = unit.reversed;
            current_epoch = unit.epoch;
        }
        current.push(index);
    }
    runs.push(current);
    reversed.push(current_reversed);
    (runs, reversed)
}

/// Run UAX #9 forward on a hypothesised logical order and check that it reproduces the
/// observed stream order (units stored visually = `0..n`).
fn predicts_observed(texts: &[&str], logical: &[usize], base_rtl: bool) -> bool {
    let proxy: String = logical.iter().map(|&i| proxy_char(texts[i])).collect();
    let base = if base_rtl { Level::rtl() } else { Level::ltr() };
    let info = BidiInfo::new(&proxy, Some(base));
    let para = match info.paragraphs.first() {
        Some(para) => para,
        None => return false,
    };
    let levels = info.reordered_levels_per_char(para, 0..proxy.len());
    if levels.len() != logical.len() {
        return false;
    }
    let map = BidiInfo::reorder_visual(&levels);
    // predicted visual[i] = logical[map[i]]; observed visual[i] = i (stream order).
    map.len() == logical.len()
        && map
            .iter()
            .enumerate()
            .all(|(visual, &logical_pos)| logical.get(logical_pos) == Some(&visual))
}

/// UAX #9 P2/P3: the paragraph direction of a finished text, used only as a tie-break.
fn auto_base_rtl(text: &str) -> bool {
    let info = BidiInfo::new(text, None);
    info.paragraphs
        .first()
        .map(|para| para.level.is_rtl())
        .unwrap_or(false)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Dir {
    Rtl,
    Ltr,
    Neutral,
}

fn first_strong(text: &str) -> Dir {
    for c in text.chars() {
        match bidi_class(c) {
            BidiClass::R | BidiClass::AL => return Dir::Rtl,
            BidiClass::L => return Dir::Ltr,
            _ => {}
        }
    }
    Dir::Neutral
}

/// Embedding level for a whole run under a given paragraph direction.
fn run_level(text: &str, base_rtl: bool) -> Level {
    let even = if base_rtl { 2 } else { 0 };
    match first_strong(text) {
        Dir::Rtl => Level::from(1),
        Dir::Ltr | Dir::Neutral => Level::from(even),
    }
}

/// One direction-preserving proxy char per unit — a unit is a single cluster, so its
/// first character's bidi class stands for the whole unit.
fn proxy_char(text: &str) -> char {
    let first = match text.chars().next() {
        Some(c) => c,
        None => return '!',
    };
    if text.chars().all(|c| bidi_class(c) == BidiClass::WS) {
        return ' ';
    }
    match bidi_class(first) {
        BidiClass::R | BidiClass::AL => '\u{05D0}',
        BidiClass::L => 'A',
        BidiClass::AN => '\u{0665}',
        BidiClass::EN => '5',
        BidiClass::WS => ' ',
        BidiClass::CS => '.',
        BidiClass::ES => '-',
        BidiClass::ET => '%',
        // ZWNJ and friends: boundary neutral — a generic neutral proxy.
        BidiClass::BN | BidiClass::S | BidiClass::B | BidiClass::ON | BidiClass::NSM => '!',
        _ => '.',
    }
}

// --- document plumbing ----------------------------------------------------

/// Per-page fonts, following inherited `/Resources`: `/ToUnicode` when the producer
/// gave us one, otherwise the `/Encoding` of a simple font — and an explicit
/// [`Font::Refused`] when neither can be turned into Unicode.
fn collect_fonts(doc: &Document, page_id: ObjectId) -> HashMap<Vec<u8>, Font> {
    let mut fonts = HashMap::new();
    let resources = match find_resources(doc, page_id) {
        Some(resources) => resources,
        None => return fonts,
    };
    let font_dict = match resources
        .get_deref(b"Font", doc)
        .ok()
        .and_then(|value| value.as_dict().ok())
    {
        Some(dict) => dict,
        None => return fonts,
    };
    for (name, value) in font_dict.iter() {
        let font = match resolve(doc, value) {
            Some(font) => font,
            None => continue,
        };
        let dict = match font.as_dict() {
            Ok(dict) => dict,
            Err(_) => continue,
        };
        // `/ToUnicode` wins when it is readable and maps anything.
        if let Some(cmap) = to_unicode(doc, dict) {
            fonts.insert(name.clone(), Font::ToUnicode(cmap));
            continue;
        }
        // Only simple fonts have a byte-level `/Encoding`. A CID font without a
        // usable `/ToUnicode` has no mapping we could read — it stays out of the
        // map and its codes surface as `unsupported_broken_to_unicode`.
        let is_simple = dict
            .get_deref(b"Subtype", doc)
            .ok()
            .and_then(|value| match value {
                Object::Name(subtype) => Some(subtype.as_slice()),
                _ => None,
            })
            .is_some_and(|subtype| SIMPLE_FONT_SUBTYPES.contains(&subtype));
        if !is_simple {
            continue;
        }
        let font = match simple_font_encoding(doc, dict) {
            Some(encoding) => Font::Simple(encoding),
            None => Font::Refused,
        };
        fonts.insert(name.clone(), font);
    }
    fonts
}

/// A page's `/ToUnicode`, decoded and non-empty — `None` when it is absent,
/// unreadable or maps nothing (we then fall back to the font's encoding).
fn to_unicode(doc: &Document, font: &Dictionary) -> Option<ToUnicode> {
    let value = font.get_deref(b"ToUnicode", doc).ok()?;
    let stream = value.as_stream().ok()?;
    let bytes = stream.decompressed_content().ok()?;
    let cmap = ToUnicode::parse(&bytes);
    if cmap.is_empty() {
        None
    } else {
        Some(cmap)
    }
}

/// Decode a simple font's `/Encoding` into a byte→Unicode table.
///
/// Returns `None` — meaning "refuse this font" — for every encoding whose byte map
/// is not Unicode and that we therefore must not guess: `/Symbol`, `/ZapfDingbats`
/// and `/MacExpertEncoding` by name, a font with no `/Encoding` at all (its
/// built-in encoding lives in the font program we do not parse), and symbolic
/// fonts that omit `/BaseEncoding`.
fn simple_font_encoding(doc: &Document, font: &Dictionary) -> Option<SimpleEncoding> {
    let encoding = resolve(doc, font.get(b"Encoding").ok()?)?;
    match encoding {
        Object::Name(name) => BaseEncoding::by_name(name).map(SimpleEncoding::new),
        Object::Dictionary(entries) => {
            let base = match entries.get(b"BaseEncoding") {
                // An explicitly named base we do not know → refuse, do not fall back.
                Ok(value) => match resolve(doc, value) {
                    Some(Object::Name(name)) => BaseEncoding::by_name(name)?,
                    _ => return None,
                },
                // Absent: ISO 32000-1 §9.6.6.1 says StandardEncoding for a
                // non-symbolic font and the font's built-in encoding for a symbolic
                // one — and the symbolic built-ins are exactly `/Symbol` and
                // `/ZapfDingbats`, whose maps are not Unicode.
                Err(_) => {
                    if looks_symbolic(doc, font) {
                        return None;
                    }
                    BaseEncoding::Standard
                }
            };
            let mut decoded = SimpleEncoding::new(base);
            decoded.apply_differences(differences(doc, entries));
            Some(decoded)
        }
        _ => None,
    }
}

/// `/Differences [code /glyph /glyph … code /glyph …]` as `(code, glyph name)` pairs.
fn differences(doc: &Document, entries: &Dictionary) -> Vec<(u8, String)> {
    let mut pairs = Vec::new();
    let array = match entries.get_deref(b"Differences", doc) {
        Ok(Object::Array(array)) => array,
        _ => return pairs,
    };
    let mut code: Option<u8> = None;
    for item in array {
        match item {
            Object::Integer(value) => code = u8::try_from(*value).ok(),
            Object::Name(name) => {
                if let Some(current) = code {
                    pairs.push((current, String::from_utf8_lossy(name).into_owned()));
                    code = current.checked_add(1);
                }
            }
            _ => {}
        }
    }
    pairs
}

/// Cheap symbolic-font check: `/Symbol` and `/ZapfDingbats` (and their subset
/// variants) in `/BaseFont`. We do not read the font program's `OS/2` flags, so
/// this is a name heuristic — it only ever decides between decoding and refusing,
/// and refusing is the safe side.
fn looks_symbolic(doc: &Document, font: &Dictionary) -> bool {
    let base_font = match font.get_deref(b"BaseFont", doc) {
        Ok(Object::Name(name)) => name.clone(),
        _ => return false,
    };
    let lower: Vec<u8> = base_font.iter().map(u8::to_ascii_lowercase).collect();
    lower.windows(6).any(|w| w == b"symbol") || lower.windows(4).any(|w| w == b"zapf")
}

/// `/Resources` on the page, or the nearest ancestor's (PDF inheritance).
fn find_resources(doc: &Document, mut page_id: ObjectId) -> Option<&Dictionary> {
    for _ in 0..32 {
        let dict = doc.get_dictionary(page_id).ok()?;
        if let Ok(value) = dict.get_deref(b"Resources", doc) {
            if let Ok(resources) = value.as_dict() {
                return Some(resources);
            }
        }
        let parent = dict
            .get(b"Parent")
            .ok()
            .and_then(|value| value.as_reference().ok())?;
        page_id = parent;
    }
    None
}

fn resolve<'a>(doc: &'a Document, value: &'a Object) -> Option<&'a Object> {
    match value {
        Object::Reference(id) => doc.get_object(*id).ok(),
        other => Some(other),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        decode_text_string, invert_units, painted_map, proxy_char, settle_line_by_bidi,
        stores_logical_order, stores_visual_order, LineOrder, Unit,
    };

    /// A visual line of units at known painted x — the input rung 3 reads.
    fn placed(texts: &[(&str, f64)]) -> Vec<Unit> {
        texts
            .iter()
            .map(|&(text, x)| Unit {
                text: text.to_string(),
                line: 80_000,
                x,
                x_ok: true,
                reversed: false,
                epoch: 0,
            })
            .collect()
    }

    /// W1.7b probe: what rung 3 answers for the date line (see bidi_rung.rs).
    #[test]
    fn rung3_outcome_for_a_date_line() {
        let units = placed(&[
            ("1", 10.0),
            ("4", 20.0),
            ("0", 30.0),
            ("3", 40.0),
            ("/", 50.0),
            ("0", 60.0),
            ("5", 70.0),
            ("/", 80.0),
            ("1", 90.0),
            ("2", 100.0),
            (" ", 110.0),
            ("تاریخ", 120.0),
        ]);
        let outcome = settle_line_by_bidi(&units);
        let identity: Vec<usize> = (0..units.len()).collect();
        let mut painted = identity.clone();
        painted.sort_by(|&left, &right| {
            units[left]
                .x
                .total_cmp(&units[right].x)
                .then(left.cmp(&right))
        });
        let inverted = invert_units(&units);
        let proxy_of = |order: &[usize]| -> String {
            order.iter().map(|&i| proxy_char(&units[i].text)).collect()
        };
        let diag = format!(
            "painted={painted:?} inverted={inverted:?} keep={:?} inv={:?}",
            painted_map(&proxy_of(&identity), &identity),
            painted_map(&proxy_of(&inverted), &inverted),
        );
        assert!(
            matches!(outcome, LineOrder::Invert(_)),
            "expected Invert, got {outcome:?} {diag}"
        );
    }

    /// The producer allow-lists ARE the last rung of the ladder (ADR 0004), so a
    /// fingerprint family only counts as order evidence while a test pins it.
    /// Strings are the lowercased `/Producer` + `/Creator` concatenation the
    /// extractor actually sees.
    #[test]
    fn producer_fingerprint_allow_lists_are_pinned() {
        // Visual-order families, measured on this corpus: InDesign (via /Creator)
        // and Word.
        assert!(stores_visual_order(
            "adobe pdf library 17.0 adobe indesign 19.4 (windows)"
        ));
        assert!(stores_visual_order(
            "adobe pdf library 16.0.7 adobe indesign 17.3 (macintosh)"
        ));
        assert!(stores_visual_order(
            "microsoft® word 2021 microsoft® word 2021"
        ));
        assert!(stores_visual_order(
            "microsoft® word ltsc microsoft® word ltsc"
        ));

        // Logical-order family: Chromium/Skia fixtures extract byte-exact.
        assert!(stores_logical_order("skia pdfium skia"));
        assert!(stores_logical_order("chromium chromium"));

        // Unknown producers must fall through to a refusal, never to a guess —
        // note `Microsoft: Print To PDF`, which carries "microsoft" but not "word".
        for unknown in [
            "adobe acrobat pro 11.0.0 adobe acrobat pro 11.0.0",
            "adobe pdf library 17.0",
            "pdfrtl-gen pdfrtl-gen",
            "microsoft: print to pdf ",
        ] {
            assert!(!stores_visual_order(unknown), "visual: {unknown}");
            assert!(!stores_logical_order(unknown), "logical: {unknown}");
        }
    }

    /// UTF-16BE with a BOM and a trailing odd byte: complete pairs decode, the odd
    /// byte is dropped — never mis-paired with a neighbour (that would shift the
    /// whole string). Same policy as `chunks_exact(2)` had; pinned here so the
    /// index-stepped rewrite cannot drift.
    #[test]
    fn text_string_bom_with_odd_trailing_byte_drops_only_the_tail() {
        let be = [0xFE, 0xFF, 0x00, 0x41, 0x00, 0x42, 0x00];
        assert_eq!(decode_text_string(&be), "AB");

        let le = [0xFF, 0xFE, 0x41, 0x00, 0x42];
        assert_eq!(
            decode_text_string(&le),
            "A",
            "LE tail byte is dropped, not treated as a high surrogate"
        );
    }
}
