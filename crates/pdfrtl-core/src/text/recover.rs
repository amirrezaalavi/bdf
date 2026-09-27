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
use crate::text::tokenizer::{parse_value, tokenize, Token, Value};
use anyhow::{Context, Result};
use lopdf::{Dictionary, Document, Object, ObjectId};
use std::collections::HashMap;
use unicode_bidi::{bidi_class, BidiClass, BidiInfo, Level};

/// Decompressed content cap per page: a page is text, not a bomb.
const MAX_PAGE_CONTENT: usize = 32 * 1024 * 1024;

/// One page's recovered text plus the reasons that justify its order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageText {
    pub page: u32,
    pub text: String,
    pub reasons: Vec<Reason>,
}

/// Recover logical text (and its justification) from one content stream.
pub fn recover_text(stream: &[u8], fonts: &HashMap<Vec<u8>, ToUnicode>) -> (String, Vec<Reason>) {
    assemble(walk(stream, fonts))
}

/// The units a producer wrote for one stream, in STREAM (visual) order — before any
/// recovery. Exposed so tests can pin the unit model itself.
pub fn stream_units(stream: &[u8], fonts: &HashMap<Vec<u8>, ToUnicode>) -> Vec<String> {
    walk(stream, fonts)
        .units
        .into_iter()
        .map(|unit| unit.text)
        .collect()
}

/// Recover every page of a loaded document, in page order.
pub fn extract_document(doc: &Document) -> Result<Vec<PageText>> {
    let mut pages = Vec::new();
    for (page_num, page_id) in doc.get_pages() {
        let content = doc
            .get_page_content_with_limit(page_id, MAX_PAGE_CONTENT)
            .with_context(|| format!("decoding content of page {page_num}"))?;
        let fonts = collect_fonts(doc, page_id);
        let (text, reasons) = recover_text(&content, &fonts);
        pages.push(PageText {
            page: page_num,
            text,
            reasons,
        });
    }
    Ok(pages)
}

// --- the walk -------------------------------------------------------------

/// One unit of producer evidence on a visual line.
#[derive(Debug, Clone)]
struct Unit {
    text: String,
    /// Quantised text-line origin (the matrix `f`), grouping units onto one line.
    line: i64,
    /// Inside `/ReversedChars`: the producer stored this run in visual order.
    reversed: bool,
    /// Bumped by each `Tm`; a new text matrix starts a new run.
    epoch: u64,
}

struct Walk {
    units: Vec<Unit>,
    used_actual_text: bool,
    used_to_unicode: bool,
    undecodable: bool,
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
}

struct Mark {
    tag: Vec<u8>,
    actual_text: Option<String>,
}

struct Walker<'a> {
    fonts: &'a HashMap<Vec<u8>, ToUnicode>,
    marks: Vec<Mark>,
    font: Option<Vec<u8>>,
    line: Mat,
    leading: f64,
    epoch: u64,
    units: Vec<Unit>,
    used_actual_text: bool,
    used_to_unicode: bool,
    undecodable: bool,
}

fn num(ops: &[Value], index: usize) -> Option<f64> {
    ops.get(index).and_then(|value| match value {
        Value::Num(n) => Some(*n),
        _ => None,
    })
}

impl<'a> Walker<'a> {
    fn new(fonts: &'a HashMap<Vec<u8>, ToUnicode>) -> Walker<'a> {
        Walker {
            fonts,
            marks: Vec::new(),
            font: None,
            line: Mat::identity(),
            leading: 0.0,
            epoch: 0,
            units: Vec::new(),
            used_actual_text: false,
            used_to_unicode: false,
            undecodable: false,
        }
    }

    fn op(&mut self, op: &[u8], ops: &[Value]) {
        match op {
            b"BT" => self.line = Mat::identity(),
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
        let cmap = match self.font.as_ref().and_then(|name| self.fonts.get(name)) {
            Some(cmap) => cmap,
            None => {
                self.undecodable = true;
                return;
            }
        };
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
                None => self.undecodable = true,
            }
            i += width;
        }
    }

    fn push_unit(&mut self, text: String) {
        let reversed = self.marks.iter().any(|mark| mark.tag == b"ReversedChars");
        let line = (self.line.f * 1000.0).round() as i64;
        self.units.push(Unit {
            text,
            line,
            reversed,
            epoch: self.epoch,
        });
    }

    fn finish(self) -> Walk {
        Walk {
            units: self.units,
            used_actual_text: self.used_actual_text,
            used_to_unicode: self.used_to_unicode,
            undecodable: self.undecodable,
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
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|c| {
            if big_endian {
                u16::from_be_bytes([c[0], c[1]])
            } else {
                u16::from_le_bytes([c[0], c[1]])
            }
        })
        .collect();
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

fn walk(stream: &[u8], fonts: &HashMap<Vec<u8>, ToUnicode>) -> Walk {
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
    producer_visual: bool,
    reconstructed: bool,
    refused: bool,
    undecodable: bool,
}

fn assemble(walk: Walk) -> (String, Vec<Reason>) {
    let mut flags = Flags {
        actual_text: walk.used_actual_text,
        to_unicode: walk.used_to_unicode,
        producer_visual: walk.units.iter().any(|unit| unit.reversed),
        reconstructed: false,
        refused: false,
        undecodable: walk.undecodable,
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
    for group in &groups {
        let slice: Vec<Unit> = group.iter().map(|&i| units[i].clone()).collect();
        match recover_line(&slice) {
            Some((text, rebuilt)) => {
                if rebuilt {
                    flags.reconstructed = true;
                }
                lines.push(text);
            }
            None => {
                flags.refused = true;
                lines.push(String::new());
            }
        }
    }
    (lines.join("\n"), build_reasons(flags))
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
    if flags.producer_visual {
        reasons.push(Reason::ProducerVisualOrderKnown);
    }
    if flags.refused {
        reasons.push(Reason::UnsupportedNoEvidence);
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

/// Per-page `/ToUnicode` maps, following inherited `/Resources`.
fn collect_fonts(doc: &Document, page_id: ObjectId) -> HashMap<Vec<u8>, ToUnicode> {
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
        let unicode = match dict.get_deref(b"ToUnicode", doc) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let stream = match unicode.as_stream() {
            Ok(stream) => stream,
            Err(_) => continue,
        };
        if let Ok(bytes) = stream.decompressed_content() {
            fonts.insert(name.clone(), ToUnicode::parse(&bytes));
        }
    }
    fonts
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
