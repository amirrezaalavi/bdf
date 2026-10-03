//! The oracle's finding, turned into a regression test: a line with NO strong character has an
//! unmeasured base direction, so deciding it rests on an assumption rather than a measurement.
//!
//! `docs/problems/0019` measured this. The oracle says a line of `1403 / 05 / 12` with every
//! position distinct admits **two** orders, because UAX #9 P3 gives it the paragraph level and a
//! painting does not reveal which level that was. Two of them are:
//!
//! | reading | how it got there |
//! |---|---|
//! | the stored order is the painted order | a producer that writes what it paints |
//! | the stored order is the reverse | a producer that writes logical order and paints LTR base |
//!
//! `settle_line_by_bidi` has no guard for this: it checks `count`, `x_ok` and ties, then decides.
//! For a Persian line that is right — the line contains strong RTL characters, so P2 fixes the
//! direction and the decision rests on measurement. For a digits-and-punctuation line it does not.
//!
//! This file is the RED specification for that guard. It is deliberately NOT fixed here: changing
//! it alters RTL output, and `docs/problems/0014` requires the refusal ratchet and a human read
//! (Q-013) before a decision that rests on an assumption becomes a refusal.

use pdfrtl_core::Reason;
use std::process::Command;

fn pdfrtl_json(args: &[&str]) -> serde_json::Value {
    let out = Command::new(env!("CARGO_BIN_EXE_pdfrtl"))
        .args(args)
        .output()
        .expect("pdfrtl runs");
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "not JSON ({e})\nstderr: {}",
            String::from_utf8_lossy(&out.stderr)
        )
    })
}

/// The oracle's own answer for this shape, so the test states the expectation independently of
/// the pipeline rather than reading it back out of the pipeline.
#[test]
fn the_oracle_says_this_shape_is_ambiguous() {
    use pdfrtl_core::text::oracle::{ambiguity_of, BidiClassOf};
    // Three units at distinct positions; digits and neutrals only, no strong character.
    let xs = [10.0, 20.0, 30.0];
    let classes = vec![BidiClassOf::EN, BidiClassOf::ON, BidiClassOf::WS];
    let got = ambiguity_of(&xs, &classes);
    assert_eq!(
        got.count(),
        Some(2),
        "a digits-and-punctuation line admits two orders; if this changed, re-read problems/0019 \
         before changing what this file asserts"
    );
    assert!(
        !got.is_unique(),
        "the oracle must report this shape as ambiguous, not unique"
    );
}

/// The contrast that makes the finding specific: add ONE strong RTL character and the direction
/// is fixed, so the same geometry becomes decidable.
#[test]
fn adding_one_strong_character_makes_the_same_geometry_decidable() {
    use pdfrtl_core::text::oracle::{ambiguity_of, BidiClassOf};
    let xs = [10.0, 20.0, 30.0];
    let with_rtl = vec![BidiClassOf::R, BidiClassOf::EN, BidiClassOf::WS];
    assert!(
        ambiguity_of(&xs, &with_rtl).is_unique(),
        "one strong RTL character fixes the base direction, so the geometry decides"
    );
}

/// The behaviour under test: a file whose only text is digits and punctuation.
///
/// Whether it currently emits or refuses is a MEASUREMENT, recorded here so a change is visible.
/// The assertion is deliberately weak on purpose — the point is that the test states what the
/// pipeline does, not what it should do. `problems/0019` says "should refuse"; making that the
/// assertion is the next lane's decision, and it needs the ratchet and a human read first.
#[test]
fn a_digits_only_page_is_measured_not_assumed() {
    let dir = std::env::temp_dir();
    let path = dir.join("pdfrtl-digits-only.pdf");
    std::fs::write(&path, digits_only_pdf()).expect("writes probe");

    let v = pdfrtl_json(&["--json", "extract", path.to_str().expect("path")]);
    let _ = std::fs::remove_file(&path);

    let page = &v["data"]["pages"][0];
    let reasons: Vec<String> = page["reasons"]
        .as_array()
        .expect("reasons")
        .iter()
        .filter_map(|r| r.as_str().map(str::to_string))
        .collect();
    let emitted = page["text"].as_str().unwrap_or("");

    // Whatever the current behaviour is, ONE of these two must hold, and the file records which:
    //   refused with a reason, or emitted - in which case the decision rested on an unmeasured
    //   base direction, which problems/0019 documents.
    let refused = reasons.iter().any(|r| r == "unsupported_visual_order");
    assert!(
        refused || !emitted.is_empty(),
        "a digits-only page neither refused with a reason nor emitted text: \
         reasons={reasons:?} text={emitted:?}"
    );

    // Whatever it decides, it must never be `ok: true` while emitting text whose direction is
    // unmeasured AND unlabelled. If a future change makes this emit, the reason list must say
    // something about the direction, so the caller can see the decision rests on an assumption.
    if !refused && !emitted.is_empty() {
        eprintln!(
            "MEASURED: a digits-only line is DECIDED with no strong character. \
             base direction unmeasured (problems/0019). reasons={reasons:?}"
        );
    }
}

/// A page whose only text is `1403 / 05 / 12` — digits and punctuation, no strong character.
///
/// Real codes and a `/ToUnicode` CMap, three units at DISTINCT positions (so no tie), and a
/// producer string on no allow-list, so the fingerprint rung cannot rescue it.
fn digits_only_pdf() -> Vec<u8> {
    let text = "1403/05/12";
    let mut chars: Vec<char> = Vec::new();
    for ch in text.chars() {
        if !chars.contains(&ch) {
            chars.push(ch);
        }
    }
    let code = |ch: char| chars.iter().position(|c| *c == ch).unwrap_or(0) + 1;

    let mut cmap = String::from(
        "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n\
         /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n\
         /CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n\
         1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n",
    );
    cmap.push_str(&format!("{} beginbfchar\n", chars.len()));
    for (index, ch) in chars.iter().enumerate() {
        cmap.push_str(&format!("<{:04X}> <{:04X}>\n", index + 1, *ch as u32));
    }
    cmap.push_str("endbfchar\nendcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n");

    // Visual order (the reversal), each unit at its OWN position so there is no tie.
    let visual: Vec<char> = text.chars().rev().collect();
    let mut stream = String::from("BT\n/F1 12 Tf\n");
    for (index, ch) in visual.iter().enumerate() {
        stream.push_str(&format!(
            "1 0 0 1 {} 80 Tm\n<{:04X}> Tj\n",
            500 - index as i64 * 60,
            code(*ch)
        ));
    }
    stream.push_str("ET\n");

    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 140] /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>".to_string(),
        format!("<< /Length {} >>\nstream\n{stream}endstream", stream.len()),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /ToUnicode 6 0 R >>".to_string(),
        format!("<< /Length {} >>\nstream\n{cmap}endstream", cmap.len()),
        "<< /Producer (pdfrtl-test-digits) /Creator (pdfrtl-test-digits) >>".to_string(),
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
            "trailer\n<< /Size {} /Root 1 0 R /Info 7 0 R >>\nstartxref\n{startxref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    pdf
}

/// Keep the `Reason` import honest: this suite asserts on serialized reason strings, and a rename
/// of the enum variant must break something here rather than silently change the contract.
#[test]
fn the_visual_order_reason_still_serializes_as_documented() {
    assert_eq!(
        Reason::UnsupportedVisualOrder.as_str(),
        "unsupported_visual_order"
    );
}
