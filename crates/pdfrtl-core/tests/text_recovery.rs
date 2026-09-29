//! P1: logical-order recovery for producer PDFs that store RTL runs in visual order.
//!
//! Ground truth is `corpus/generated/SOURCES.md` (copied byte-exactly; ZWNJ written as an
//! explicit `\u{200C}` escape so a copy-paste accident cannot silently pass — see the
//! `assert!(expected.contains('\u{200c}'))` guards below).
//!
//! The algorithm under test is docs/problems/0002: inside a `/ReversedChars BMC … EMC`
//! run, a `<CID> Tj` inside `/Span<</ActualText …>>>` is ONE unit (the decoded ActualText)
//! and a bare `<CID> Tj` is ONE unit PER CID via `ToUnicode`; then the UNIT order is
//! reversed — never the string.

use pdfrtl_core::text::{recover_text, stream_units, Font, ToUnicode};
use pdfrtl_core::Reason;
use std::collections::HashMap;
use std::path::Path;

const CHROME: &str = "../../corpus/raw/generated/chrome";

/// The raw content stream quoted verbatim in docs/problems/0002 (arrow comments stripped).
const PROBLEM_0002_STREAM: &str = r#"q
3.125 0 0 3.125 212.5 212.5 cm
/NonStruct <</MCID 0 >>BDC
BT
/ReversedChars BMC
/Span<</ActualText <FEFF0627> >> BDC
/F4 24 Tf
1 0 0 -1 554.28125 40 Tm
<038E> Tj
EMC
/Span<</ActualText <FEFF06CC> >> BDC
<03F4> Tj
EMC
/Span<</ActualText <FEFF0646> >> BDC
<03E7> Tj
EMC
<03A9000303E1> Tj
/Span<</ActualText <FEFF06440627> >> BDC
<03FC> Tj
EMC
/Span<</ActualText <FEFF0633> >> BDC
<03B3> Tj
EMC
EMC
ET"#;

/// The `/F4` ToUnicode CMap exactly as Chrome wrote it for `fa-plain.pdf`.
const FA_PLAIN_TOUNICODE: &str = r#"/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CIDSystemInfo
<<  /Registry (Adobe)
/Ordering (UCS)
/Supplement 0
>> def
/CMapName /Adobe-Identity-UCS def
/CMapType 2 def
1 begincodespacerange
<0000> <FFFF>
endcodespacerange
8 beginbfchar
<0003> <0020>
<038E> <FE8E>
<03A9> <062F>
<03B3> <FEB3>
<03E1> <0645>
<03E7> <FEE7>
<03F4> <FBFF>
<03FC> <FEFC>
endbfchar
endcmap
CMapName currentdict /CMap defineresource pop
end
end"#;

fn fonts(name: &[u8], cmap: &str) -> HashMap<Vec<u8>, Font> {
    let mut map = HashMap::new();
    map.insert(
        name.to_vec(),
        Font::ToUnicode(ToUnicode::parse(cmap.as_bytes())),
    );
    map
}

fn first_line(text: &str) -> &str {
    text.split('\n').next().unwrap_or_default()
}

/// docs/problems/0002 worked example: the exact raw stream must give `سلام دنیا`,
/// and it must get there by reversing UNITS, not characters.
#[test]
fn problem_0002_stream_recovers_salam_donya_by_unit_reversal() {
    let fonts = fonts(b"F4", FA_PLAIN_TOUNICODE);

    // 1. The units the stream actually declares, in stream (visual) order.
    let units = stream_units(PROBLEM_0002_STREAM.as_bytes(), &fonts);
    assert_eq!(
        units,
        vec![
            "\u{0627}",
            "\u{06CC}",
            "\u{0646}",
            "\u{062F}",
            " ",
            "\u{0645}",
            "\u{0644}\u{0627}",
            "\u{0633}"
        ],
        "the 3-CID run must split into one unit per CID (د, space, م) and لا must stay whole"
    );
    assert!(
        units.contains(&"\u{0644}\u{0627}".to_string()),
        "the lam-alef ligature is ONE unit"
    );

    // 2. Recovered text.
    let (text, reasons) = recover_text(PROBLEM_0002_STREAM.as_bytes(), &fonts);
    let expected = "سلام دنیا";
    assert_eq!(text, expected);

    // 3. Why unit-level: a whole-string reversal of the visual text splits the lam-alef
    //    into ا+ل and yields "سالم دنیا" — a different, plausible-looking Persian word.
    //    That is the silent bug: wrong text, no error, no visible damage.
    let visual: String = units.concat();
    let string_reversal: String = visual.chars().rev().collect();
    assert_eq!(string_reversal, "سالم دنیا");
    assert_ne!(
        string_reversal, expected,
        "string reversal is the classic silent bug"
    );
    assert!(
        expected.starts_with("\u{0633}\u{0644}\u{0627}"),
        "the answer keeps لا whole where string reversal would not"
    );
    // …while reversing the unit ORDER reproduces it exactly.
    let unit_reversal: String = units.iter().rev().cloned().collect();
    assert_eq!(unit_reversal, expected, "reversal happens at unit level");
    assert_eq!(text, unit_reversal);

    // 4. Justification for every part of the text.
    assert_eq!(
        reasons,
        vec![
            Reason::ActualText,
            Reason::ToUnicodeLogical,
            Reason::ProducerVisualOrderKnown,
        ]
    );
}

#[test]
fn chrome_fa_plain_matches_sources() {
    let pages = pdfrtl_core::extract(Path::new(&format!("{CHROME}/fa-plain.pdf"))).expect("loads");
    let text = &pages[0].text;
    let expected = "سلام دنیا";
    assert_eq!(first_line(text), expected, "page text was {text:?}");
    assert!(text.contains(expected));
    assert!(
        text.contains("\u{0644}\u{0627}"),
        "lam-alef survives: {text:?}"
    );
    assert!(pages[0].reasons.contains(&Reason::ActualText));
    assert!(pages[0].reasons.contains(&Reason::ProducerVisualOrderKnown));
    assert!(pages[0].reasons.contains(&Reason::ToUnicodeLogical));
    assert!(
        !pages[0].reasons.iter().any(|r| r.is_unsupported()),
        "Chrome's ToUnicode is complete for this fixture: {:?}",
        pages[0].reasons
    );
}

#[test]
fn chrome_fa_zwnj_word_keeps_u200c() {
    let pages =
        pdfrtl_core::extract(Path::new(&format!("{CHROME}/fa-zwnj-word.pdf"))).expect("loads");
    let text = &pages[0].text;
    let expected = "می\u{200C}روم";
    assert!(
        expected.contains('\u{200c}'),
        "the guard against a lost ZWNJ"
    );
    assert_eq!(first_line(text), expected, "page text was {text:?}");
    assert!(
        text.contains('\u{200c}'),
        "U+200C must survive extraction: {:?}",
        text.chars().map(|c| c as u32).collect::<Vec<_>>()
    );
    assert_ne!(
        first_line(text),
        "میروم",
        "ZWNJ loss is invisible to the eye"
    );
}

#[test]
fn chrome_fa_zwnj_lamalef_matches_sources_with_the_date_last() {
    let pages =
        pdfrtl_core::extract(Path::new(&format!("{CHROME}/fa-zwnj-lamalef.pdf"))).expect("loads");
    let text = &pages[0].text;
    let expected = "نیم\u{200C}فاصله و لا اله الا الله — ۱۴۰۳/۰۵/۱۲";
    assert!(
        expected.contains('\u{200c}'),
        "the guard against a lost ZWNJ"
    );
    assert!(
        expected.contains('\u{0644}'),
        "lam-alef in the source sentence"
    );
    assert_eq!(first_line(text), expected, "page text was {text:?}");
    assert!(text.contains('\u{200c}'), "U+200C survives: {text:?}");
    // The date is a separate content-stream block drawn to the LEFT of the RTL run, so
    // it only lands at the end once run order is reconstructed (UAX #9 levels).
    assert!(
        pages[0].reasons.contains(&Reason::BidiReordered),
        "run order was reconstructed: {:?}",
        pages[0].reasons
    );
}

/// docs/problems/0001 P1: the synthetic /ActualText fixture must come back verbatim.
#[test]
fn synthetic_actualtext_fa_returns_both_lines() {
    let pages = pdfrtl_core::extract(Path::new("../../corpus/raw/synthetic/actualtext-fa.pdf"))
        .expect("loads");
    let text = &pages[0].text;
    let expected = "سلام دنیا\nمی\u{200C}روم";
    assert!(
        expected.contains('\u{200c}'),
        "the guard against a lost ZWNJ"
    );
    assert_eq!(text, expected, "page text was {text:?}");
    assert_eq!(pages[0].reasons, vec![Reason::ActualText]);
}

/// Deliverable (c): a CID with no ToUnicode entry and no /ActualText is refused,
/// not guessed at. Built from an in-memory content stream — no fixture file.
#[test]
fn undecodable_cid_reports_unsupported_broken_to_unicode() {
    let stream = "BT /F1 12 Tf 10 700 Td <0041> Tj ET";
    // A real CMap — but it maps <0003>, not <0041>.
    let cmap = "\
/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
1 begincodespacerange
<0000> <FFFF>
endcodespacerange
1 beginbfchar
<0003> <0020>
endbfchar
endcmap
end
end";
    let fonts = fonts(b"F1", cmap);
    let (text, reasons) = recover_text(stream.as_bytes(), &fonts);
    assert!(
        reasons.contains(&Reason::UnsupportedBrokenToUnicode),
        "undecodable CID must be reported as unsupported_broken_to_unicode, got {reasons:?}"
    );
    assert!(reasons.iter().any(|r| r.is_unsupported()));
    assert_eq!(
        text, "",
        "nothing may be invented for a glyph we cannot decode"
    );

    // Missing ToUnicode entirely is the same refusal (the Reason's own definition).
    let empty: HashMap<Vec<u8>, Font> = HashMap::new();
    let (_, reasons) = recover_text(stream.as_bytes(), &empty);
    assert!(reasons.contains(&Reason::UnsupportedBrokenToUnicode));
}

/// Build a one-page PDF around a content stream, with a fixed `/Producer`+`/Creator`.
/// The font is deliberately absent: `/ActualText` short-circuits decoding
/// (ISO 32000-1 §14.9.4), so nothing else in the file can decide the outcome.
fn write_probe_pdf(name: &str, producer: &str, content: &str) -> std::path::PathBuf {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 120] /Contents 4 0 R >>".to_string(),
        format!(
            "<< /Length {} >>\nstream\n{}\nendstream",
            content.len(),
            content
        ),
        format!("<< /Producer ({producer}) /Creator ({producer}) >>"),
    ];
    let mut pdf: Vec<u8> = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", index + 1, object).as_bytes());
    }
    let startxref = pdf.len();
    let mut table = format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1);
    for offset in &offsets {
        table.push_str(&format!("{offset:010} 00000 n \n"));
    }
    pdf.extend_from_slice(table.as_bytes());
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R /Info 5 0 R >>\nstartxref\n{startxref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    let path = std::env::temp_dir().join(format!("pdfrtl-{name}-{}.pdf", std::process::id()));
    std::fs::write(&path, pdf).expect("temp pdf is writable");
    path
}

/// `دنیا` stored the way a producer that mirrors its runs stores it: clusters in
/// VISUAL order (ا ی ن د) inside `/ReversedChars BMC … EMC`, one `/ActualText`
/// per cluster. The producer, though, is one we have NEVER measured
/// (`pdfrtl-test`) — the marker, not the fingerprint, is what establishes order
/// (ADR 0004 rung 2).
const MIRRORED_DONYA: &str = "BT /F1 12 Tf 20 80 Td\n\
/ReversedChars BMC\n\
/Span<</ActualText <FEFF0627> >> BDC (.) Tj EMC\n\
/Span<</ActualText <FEFF06CC> >> BDC (.) Tj EMC\n\
/Span<</ActualText <FEFF0646> >> BDC (.) Tj EMC\n\
/Span<</ActualText <FEFF062F> >> BDC (.) Tj EMC\n\
EMC\nET";

/// The SAME four clusters with the marker deleted. The marker is gone, the layout
/// is not: nothing repositions these clusters, so the producer painted them in
/// stream order (a `Tj` advances the pen through the glyphs it shows), and rung 3 —
/// the UAX #9 comparison against that painted order — finds exactly one reading
/// that reproduces the painting. The marker changes which RULE settles the line,
/// not whether the line can be settled: same text, named differently.
const UNMARKED_DONYA: &str = "BT /F1 12 Tf 20 80 Td\n\
/Span<</ActualText <FEFF0627> >> BDC (.) Tj EMC\n\
/Span<</ActualText <FEFF06CC> >> BDC (.) Tj EMC\n\
/Span<</ActualText <FEFF0646> >> BDC (.) Tj EMC\n\
/Span<</ActualText <FEFF062F> >> BDC (.) Tj EMC\n\
ET";

/// `/ReversedChars` is page-local order evidence; lines that carry none go to
/// rung 3 first and the producer fingerprint last. One marker changes the RULE
/// that answers, and nothing else about the file does — that is the ladder.
#[test]
fn reversed_chars_marker_is_order_evidence_no_matter_the_producer() {
    let marked = write_probe_pdf("reversed-marker", "pdfrtl-test", MIRRORED_DONYA);
    let pages = pdfrtl_core::extract(&marked).expect("loads");
    let _ = std::fs::remove_file(&marked);
    assert_eq!(pages[0].text, "دنیا", "unit order reversed, not the string");
    assert_eq!(
        pages[0].reasons,
        vec![Reason::ActualText, Reason::ProducerVisualOrderKnown],
        "the marker is the evidence; no fingerprint was needed"
    );
    assert!(pages[0].is_ordered(), "this page's order IS established");

    let unmarked = write_probe_pdf("unmarked", "pdfrtl-test", UNMARKED_DONYA);
    let pages = pdfrtl_core::extract(&unmarked).expect("loads");
    let _ = std::fs::remove_file(&unmarked);
    assert_eq!(
        pages[0].reasons,
        vec![
            Reason::ActualText,
            Reason::BidiReordered,
            Reason::BidiVerified
        ],
        "no marker: the painted positions decide, and the answer is the same text"
    );
    assert_eq!(
        pages[0].text, "دنیا",
        "the clusters were painted left to right, so the stored sequence is visual"
    );
    assert!(
        pages[0].is_ordered(),
        "the comparison established the order"
    );
    assert_eq!(
        pages[0].unordered_chars, 0,
        "nothing withheld: it was decidable"
    );
}

/// The RTL/LTR boundary line: this is where run-order reconstruction earns its keep.
/// If any fixture were special-cased, this one would be first to break.
#[test]
fn chrome_mixed_fa_en_recovers_the_rtl_run_first() {
    let pages =
        pdfrtl_core::extract(Path::new(&format!("{CHROME}/mixed-fa-en.pdf"))).expect("loads");
    let text = &pages[0].text;
    let expected = "گزارش فنی pdfrtl v0.1 — ISO 32000-1 §9.7.4.3 — 42%";
    assert_eq!(first_line(text), expected, "page text was {text:?}");
    assert!(
        pages[0].reasons.contains(&Reason::BidiReordered),
        "run order was reconstructed: {:?}",
        pages[0].reasons
    );
}

/// The LTR control stores logical order already: pass it through, one clean reason.
#[test]
fn chrome_en_control_is_passed_through_untouched() {
    let pages =
        pdfrtl_core::extract(Path::new(&format!("{CHROME}/en-control.pdf"))).expect("loads");
    let text = &pages[0].text;
    assert_eq!(
        first_line(text),
        "pdfrtl v0.1 — ISO 32000-1",
        "page text was {text:?}"
    );
    assert_eq!(pages[0].reasons, vec![Reason::ToUnicodeLogical]);
}

/// Type1 simple fonts encode one byte per code (hebrew-2.pdf): the codespacerange
/// width must drive decoding instead of a guessed 2, and garbage CMaps stay total.
#[test]
fn one_byte_codespace_decodes_simple_font_codes() {
    let cmap = "1 begincodespacerange\n<00> <FF>\nendcodespacerange\n2 beginbfchar\n<4E> <05D0>\n<61> <0041>\nendbfchar";
    let parsed = ToUnicode::parse(cmap.as_bytes());
    assert_eq!(
        parsed.code_len(),
        1,
        "one byte per code, from codespacerange"
    );
    assert_eq!(parsed.get(0x4E), Some("\u{05D0}"));

    // One-byte stream: each byte is its own code, each code one unit.
    let stream = "BT /F1 12 Tf 10 700 Td <4E61> Tj ET";
    let fonts = fonts(b"F1", cmap);
    let (text, reasons) = recover_text(stream.as_bytes(), &fonts);
    assert_eq!(text, "\u{05D0}A");
    assert!(
        !reasons.iter().any(|reason| reason.is_unsupported()),
        "{reasons:?}"
    );

    // Garbage in, empty map out — total, no panic, no guess.
    assert!(ToUnicode::parse(b"\x00\x01 not-a-cmap << [( \xff").is_empty());
}
