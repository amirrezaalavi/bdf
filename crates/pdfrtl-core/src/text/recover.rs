//! Page text recovery: content stream → units → lines → logical order + reasons.
//!
//! High-level recovery orchestrator connecting content stream walking (`state.rs`),
//! cluster script analysis (`cluster.rs`), and UAX #9 bidirectional order settlement (`bidi.rs`).

use crate::reasons::Reason;
use crate::text::bidi::{settle_line_by_bidi, LineOrder};
use crate::text::cluster::{has_rtl_run, visual_to_logical};
use crate::text::encoding::Font;
use crate::text::state::{collect_fonts, walk, Metrics, Unit, Walk};
use anyhow::Result;
use lopdf::Document;
use std::collections::HashMap;

pub use crate::text::bidi::{take_order_trace, take_outcome_counts};

/// Decompressed content cap per page: a page is text, not a bomb.
const MAX_PAGE_CONTENT: usize = 32 * 1024 * 1024;

/// One page's recovered text plus the reasons that justify its order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageText {
    pub page: u32,
    pub text: String,
    pub reasons: Vec<Reason>,
    /// Characters this page DECODED but did not emit, because their order is not
    /// established.
    pub unordered_chars: usize,
    /// Lines this page decoded but could not put in reading order, WITH their text.
    pub unproven: Vec<UnprovenLine>,
}

/// A line whose order could not be established, reported rather than discarded.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct UnprovenLine {
    /// Zero-based index of the line on its page, in the page's line order.
    pub line: usize,
    /// The text exactly as it was STORED. Not reading order, and not claimed to be.
    pub text: String,
    /// Why the order was not established — the same `Reason` vocabulary the page carries.
    pub reason: Reason,
}

impl PageText {
    /// True when every glyph this page showed was decoded with a justification.
    pub fn is_decoded(&self) -> bool {
        !self.reasons.iter().any(|reason| reason.is_unsupported())
    }

    /// True when everything in `text` was put in reading order by a named rule.
    pub fn is_ordered(&self) -> bool {
        !self.reasons.contains(&Reason::UnsupportedVisualOrder)
    }

    /// Withdraw text whose order was never established. Runs after [`settle_rtl_order`].
    fn withdraw_unordered(&mut self, withheld: Vec<usize>) {
        if !self.is_ordered() && !self.text.is_empty() {
            self.unordered_chars = self.text.chars().count();
            let stored: Vec<&str> = self.text.split('\n').collect();
            for index in withheld {
                if let Some(line) = stored.get(index) {
                    if !line.trim().is_empty() {
                        self.unproven.push(UnprovenLine {
                            line: index,
                            text: (*line).to_string(),
                            reason: Reason::UnsupportedVisualOrder,
                        });
                    }
                }
            }
            self.text.clear();
        }
    }
}

/// Recover logical text (and its justification) from one content stream.
pub fn recover_text(stream: &[u8], fonts: &HashMap<Vec<u8>, Font>) -> (String, Vec<Reason>) {
    let recovered = recover(stream, fonts, &HashMap::new());
    (recovered.text, recovered.reasons)
}

/// One page's recovery result.
struct Recovered {
    text: String,
    reasons: Vec<Reason>,
    pending: Vec<usize>,
    withheld: Vec<usize>,
}

fn recover(
    stream: &[u8],
    fonts: &HashMap<Vec<u8>, Font>,
    metrics: &HashMap<Vec<u8>, Metrics>,
) -> Recovered {
    assemble(walk(stream, fonts, metrics))
}

/// The units a producer wrote for one stream, in STREAM (visual) order — before any recovery.
pub fn stream_units(stream: &[u8], fonts: &HashMap<Vec<u8>, Font>) -> Vec<String> {
    walk(stream, fonts, &HashMap::new())
        .units
        .into_iter()
        .map(|unit| unit.text)
        .collect()
}

/// Recover every page of a loaded document, in page order.
pub fn extract_document(doc: &Document) -> Result<Vec<PageText>> {
    let mut pages = Vec::new();
    let producer = producer_fingerprint(doc);
    for (page_num, page_id) in doc.get_pages() {
        let page = match doc.get_page_content_with_limit(page_id, MAX_PAGE_CONTENT) {
            Ok(content) => {
                let (fonts, metrics) = collect_fonts(doc, page_id);
                let mut recovered = recover(&content, &fonts, &metrics);
                let mut page = PageText {
                    page: page_num,
                    text: recovered.text,
                    reasons: recovered.reasons,
                    unordered_chars: 0,
                    unproven: Vec::new(),
                };
                settle_rtl_order(&mut page, &producer, &recovered.pending);
                let withheld = std::mem::take(&mut recovered.withheld);
                page.withdraw_unordered(withheld);
                page
            }
            Err(_) => PageText {
                page: page_num,
                text: String::new(),
                reasons: vec![Reason::UnsupportedPageContent],
                unordered_chars: 0,
                unproven: Vec::new(),
            },
        };
        pages.push(page);
    }
    Ok(pages)
}

/// The document's `/Producer` and `/Creator`, lowercased.
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderConvention {
    Visual,
    Logical,
}

pub struct ProducerFamily {
    pub id: &'static str,
    pub all: &'static [&'static str],
    pub any: &'static [&'static str],
    pub convention: OrderConvention,
}

pub const PRODUCER_ALLOW_LIST: &[ProducerFamily] = &[
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
    ProducerFamily {
        id: "chromium-skia",
        all: &[],
        any: &["skia", "chrom"],
        convention: OrderConvention::Logical,
    },
];

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

fn stores_visual_order(fingerprint: &str) -> bool {
    producer_convention(fingerprint) == Some(OrderConvention::Visual)
}

fn stores_logical_order(fingerprint: &str) -> bool {
    producer_convention(fingerprint) == Some(OrderConvention::Logical)
}

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

struct Flags {
    actual_text: bool,
    to_unicode: bool,
    encoded: bool,
    producer_visual: bool,
    bidi_consistent: bool,
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
        bidi_consistent: false,
        reconstructed: false,
        refused: false,
        undecodable: walk.undecodable,
        refused_encoding: walk.refused_encoding,
    };

    // Group units onto visual lines by text-line origin with baseline tolerance, keeping appearance order.
    let units = walk.units;
    let mut keys: Vec<f64> = Vec::new();
    let mut groups: Vec<Vec<usize>> = Vec::new();
    const BASELINE_TOLERANCE: f64 = 1.0;
    for (index, unit) in units.iter().enumerate() {
        match keys
            .iter()
            .position(|&key| (key - unit.line).abs() <= BASELINE_TOLERANCE)
        {
            Some(at) => groups[at].push(index),
            None => {
                keys.push(unit.line);
                groups.push(vec![index]);
            }
        }
    }

    let mut lines: Vec<String> = Vec::new();
    let mut pending: Vec<usize> = Vec::new();
    let mut withheld: Vec<usize> = Vec::new();
    for group in &groups {
        let slice: Vec<Unit> = group.iter().map(|&i| units[i].clone()).collect();
        let marked = slice.iter().any(|unit| unit.reversed);
        let undecided = slice.len() > 1 && !marked;
        match recover_line(&slice) {
            Some((mut text, rebuilt)) => {
                if rebuilt {
                    flags.reconstructed = true;
                }
                if undecided && has_rtl_run(&text) {
                    match settle_line_by_bidi(&slice) {
                        LineOrder::Keep => flags.bidi_consistent = true,
                        LineOrder::Invert(order) => {
                            text = order.iter().map(|&i| slice[i].text.as_str()).collect();
                            flags.bidi_consistent = true;
                            flags.reconstructed = true;
                        }
                        LineOrder::Ambiguous | LineOrder::Unexplained => {
                            withheld.push(lines.len());
                            pending.push(lines.len());
                            if std::env::var_os("PDFRTL_TRACE_ORDER").is_some() {
                                let (ties, distinct, min, max) =
                                    crate::text::bidi::tie_span(&slice);
                                crate::text::bidi::trace_line(
                                    slice.len(),
                                    ties,
                                    distinct,
                                    min,
                                    max,
                                    crate::text::bidi::has_geometry(&slice),
                                );
                            }
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
        withheld,
    }
}

fn build_reasons(flags: Flags) -> Vec<Reason> {
    let mut reasons = Vec::new();
    if flags.actual_text {
        reasons.push(Reason::ActualText);
    }
    if flags.reconstructed {
        reasons.push(Reason::BidiReordered);
    } else if flags.to_unicode {
        reasons.push(Reason::ToUnicodeLogical);
    }
    if flags.encoded {
        reasons.push(Reason::EncodingMapped);
    }
    if flags.producer_visual {
        reasons.push(Reason::ProducerVisualOrderKnown);
    }
    if flags.bidi_consistent {
        reasons.push(Reason::BidiConsistent);
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

/// Recover one visual line.
fn recover_line(units: &[Unit]) -> Option<(String, bool)> {
    if !units.iter().any(|unit| unit.reversed) {
        let text = units.iter().map(|unit| unit.text.as_str()).collect();
        return Some((text, false));
    }

    let (runs, run_reversed) = crate::text::bidi::split_runs(units);
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

    let mut candidates: Vec<(Vec<usize>, bool)> = Vec::new();
    for base_rtl in [false, true] {
        let levels: Vec<unicode_bidi::Level> = (0..runs.len())
            .map(|run| {
                let text: String = join(&[run]).iter().map(|&i| texts[i]).collect();
                crate::text::bidi::run_level(&text, base_rtl)
            })
            .collect();
        let map = unicode_bidi::BidiInfo::reorder_visual(&levels);
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
        if crate::text::bidi::predicts_observed(&texts, &logical, base_rtl) {
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
        let mut kept: Vec<Vec<usize>> = Vec::new();
        for order in &distinct {
            let text: String = order.iter().map(|&i| texts[i]).collect();
            let base = crate::text::bidi::auto_base_rtl(&text);
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

#[cfg(test)]
pub mod tests {
    use super::*;

    fn placed(texts: &[(&str, f64)]) -> Vec<Unit> {
        texts
            .iter()
            .map(|&(text, x)| Unit {
                text: text.to_string(),
                line: 80.0,
                paint_x: x,
                x_ok: true,
                reversed: false,
                epoch: 0,
            })
            .collect()
    }

    #[test]
    fn rung3_outcome_for_a_date_line() {
        let units = placed(&[("تاریخ", 200.0), (" ", 160.0), ("1403/05/12", 80.0)]);
        assert!(matches!(settle_line_by_bidi(&units), LineOrder::Keep));
    }

    #[test]
    fn producer_fingerprint_allow_lists_are_pinned() {
        assert!(stores_visual_order("microsoft word 2016"));
        assert!(stores_visual_order("adobe indesign cc 2019"));
        assert!(stores_logical_order("skia/pdf m120"));
        assert!(!stores_visual_order("librecad 2.2.0"));
        assert!(!stores_logical_order("librecad 2.2.0"));
    }

    #[test]
    fn text_string_bom_with_odd_trailing_byte_drops_only_the_tail() {
        let odd_bom = [0xFE, 0xFF, 0x00, 0x41, 0x00];
        assert_eq!(pdf_text_string(&odd_bom), "A");
    }
}
