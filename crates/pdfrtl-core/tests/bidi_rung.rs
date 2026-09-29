//! W1.7b: rung 3 of the order ladder — the UAX #9 comparison.
//!
//! For a line with no `/ReversedChars` of its own we have two readings and one
//! question: which of them reproduces the sequence the producer actually painted?
//! Exactly one answer is allowed; anything else is a refusal (ADR 0002). Every
//! stream below is synthetic and built from `/ActualText` units, so the characters
//! are never in doubt — only their ORDER is under test, which is the whole point
//! of the rung.
//!
//! The four cases UAX #9 itself has to survive: an embedded Latin run, a date
//! (`1403/05/12`) that must not come out as `21/50/3041`, paired brackets, and a
//! lam-alef ligature that must stay one unit.

use pdfrtl_core::Reason;
use std::path::PathBuf;

/// A unit's text plus the x it is painted at. The x IS the evidence: it is how
/// the rung reads the sequence the producer painted (see `settle_line_by_bidi`).
struct Placed {
    text: &'static str,
    x: f64,
}

impl Placed {
    fn new(text: &'static str, x: f64) -> Placed {
        Placed { text, x }
    }
}

/// UTF-16BE hex for an `/ActualText` value, BOM included (ISO 32000-1 §14.9.4).
fn actual_text(text: &str) -> String {
    let mut hex = String::from("FEFF");
    for unit in text.encode_utf16() {
        hex.push_str(&format!("{unit:04X}"));
    }
    hex
}

/// One line: each unit on its own text matrix at its own x, all on one baseline.
/// Equal x is legal and means "one origin, painted in stream order" — a whole
/// class of producers writes exactly that.
fn content(units: &[Placed]) -> String {
    let mut stream = String::from("BT\n/F1 12 Tf\n");
    for unit in units {
        stream.push_str(&format!(
            "1 0 0 -1 {} 80 Tm\n/Span<</ActualText <{}> >> BDC (.) Tj EMC\n",
            unit.x,
            actual_text(unit.text)
        ));
    }
    stream.push_str("ET\n");
    stream
}

/// Build a one-page PDF around a content stream, with a fixed `/Producer`.
/// No font resource: `/ActualText` short-circuits decoding, so nothing but the
/// positions and the order can decide the outcome.
fn probe(name: &str, producer: &str, units: &[Placed]) -> PathBuf {
    let text = content(units);
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 140] /Contents 4 0 R >>".to_string(),
        format!("<< /Length {} >>\nstream\n{}\nendstream", text.len(), text),
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
            "trailer\n<< /Size {} /Root 1 0 R /Info 5 0 R >>\nstartxref\n{}\n%%EOF\n",
            objects.len() + 1,
            startxref
        )
        .as_bytes(),
    );
    let path = std::env::temp_dir().join(format!("pdfrtl-bidi-rung-{name}.pdf"));
    std::fs::write(&path, pdf).expect("writes probe");
    path
}

fn extract(name: &str, units: &[Placed]) -> pdfrtl_core::PageText {
    let path = probe(name, "pdfrtl-test", units);
    let pages = pdfrtl_core::extract(&path).expect("loads");
    let _ = std::fs::remove_file(&path);
    pages.into_iter().next().expect("one page")
}

/// An RTL line with a Latin run inside it, stored in LOGICAL order and *positioned*
/// the way the bidi algorithm paints it (Latin leftmost): the positions say the
/// stored sequence is not the painted one, so the mirrored reading cannot even
/// claim the producer stored what it painted. Keep, no inversion.
#[test]
fn rtl_line_with_a_latin_run_is_kept_when_the_positions_say_logical() {
    // Logical: سلام pdfrtl. Base RTL paints the Latin run leftmost, so its x is
    // the smallest of the three.
    let page = extract(
        "latin-run",
        &[
            Placed::new("سلام", 300.0),
            Placed::new(" ", 200.0),
            Placed::new("pdfrtl", 100.0),
        ],
    );
    assert_eq!(
        page.text, "سلام pdfrtl",
        "logical order kept: {:?}",
        page.text
    );
    assert_eq!(page.reasons, vec![Reason::ActualText, Reason::BidiVerified]);
    assert!(page.is_ordered(), "the comparison established the order");
}

/// A date inside an RTL line. The file stores the PAINTED sequence (one origin,
/// painted left to right), so rung 3 must invert — and the numeric run has to come
/// back unreversed, because UAX #9 keeps European numbers in reading order (L1/L2).
#[test]
fn digits_of_a_date_survive_inversion_as_one_unreversed_run() {
    let page = extract(
        "date",
        &[
            Placed::new("1", 10.0),
            Placed::new("4", 20.0),
            Placed::new("0", 30.0),
            Placed::new("3", 40.0),
            Placed::new("/", 50.0),
            Placed::new("0", 60.0),
            Placed::new("5", 70.0),
            Placed::new("/", 80.0),
            Placed::new("1", 90.0),
            Placed::new("2", 100.0),
            Placed::new(" ", 110.0),
            Placed::new("تاریخ", 120.0),
        ],
    );
    assert_eq!(
        page.text, "تاریخ 1403/05/12",
        "the date is one run, in reading order: {:?}",
        page.text
    );
    assert!(page.reasons.contains(&Reason::BidiReordered));
    assert!(page.reasons.contains(&Reason::BidiVerified));
    assert!(page.is_ordered());
}

/// Paired brackets around an RTL word (BD16/L4 territory). Mirroring is a
/// RENDERING step: the characters in the file are what they are, and the rung
/// must restore their ORDER — `(` first in logical order — without swapping a
/// bracket for its partner.
#[test]
fn brackets_keep_their_paired_positions_through_the_inversion() {
    let page = extract(
        "brackets",
        &[
            Placed::new(")", 10.0),
            Placed::new("سلام", 20.0),
            Placed::new("(", 30.0),
        ],
    );
    assert_eq!(
        page.text, "(سلام)",
        "paired, in logical order: {:?}",
        page.text
    );
    assert!(page.is_ordered());
    assert!(!page.text.contains(")س"), "no bracket came out backwards");
}

/// A lam-alef ligature is ONE unit (one code, two characters). Inverting the line
/// must move the unit, never split it: reversing its characters is the exact
/// silent corruption ADR 0002 forbids (`سلام` → `سالم`).
#[test]
fn lam_alef_cluster_stays_whole_when_the_line_is_inverted() {
    // The ligature is written as a single /ActualText unit, the way a ToUnicode
    // CMap maps one ligature code to U+0644 U+0627.
    let page = extract(
        "lam-alef",
        &[
            Placed::new("لا", 10.0),
            Placed::new(" ", 20.0),
            Placed::new("سلام", 30.0),
        ],
    );
    assert_eq!(page.text, "سلام لا", "ligature intact: {:?}", page.text);
    assert!(
        page.text.ends_with("لا"),
        "one lam-alef, two characters, in that order: {:?}",
        page.text
    );
    assert!(page.is_ordered());
}

/// A line whose painted order BOTH readings reproduce: an English line with an
/// Arabic word and an Arabic line with an English word paint identically at one
/// origin. That is a coin flip, and a coin flip is banned — the answer is REFUSE,
/// not a guess (see the `Ambiguous` arm of `settle_line_by_bidi`).
#[test]
fn ambiguous_line_is_refused_never_coin_flipped() {
    let page = extract(
        "ambiguous",
        &[
            Placed::new("pdfrtl", 10.0),
            Placed::new(" ", 20.0),
            Placed::new("سلام", 30.0),
        ],
    );
    assert_eq!(
        page.reasons,
        vec![Reason::ActualText, Reason::UnsupportedVisualOrder],
        "both readings fit: refuse instead of flipping a coin"
    );
    assert!(!page.is_ordered());
    assert_eq!(page.text, "", "unestablished order is withdrawn");
    assert_eq!(page.unordered_chars, "pdfrtl سلام".chars().count());
}

/// A line whose painted order NEITHER reading reproduces — positions that are
/// neither the stored order nor its mirror. The file contradicts the model, so
/// the rung gives no opinion and the ladder refuses.
#[test]
fn line_contradicted_by_its_own_positions_is_refused() {
    // Painted (by x): unit0, unit2, unit1 — a rotation of neither reading.
    let page = extract(
        "contradicted",
        &[
            Placed::new("ا", 10.0),
            Placed::new("ب", 30.0),
            Placed::new("ج", 20.0),
        ],
    );
    assert_eq!(
        page.reasons,
        vec![Reason::ActualText, Reason::UnsupportedVisualOrder],
        "no reading fits the painting: {:?}",
        page.reasons
    );
    assert_eq!(page.text, "");
    assert_eq!(page.unordered_chars, 3);
}
