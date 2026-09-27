//! TDD: the CLI contract. These tests are what make the product agent-drivable,
//! so they are stricter than the library tests: exit codes and envelope shape only.

use std::process::Command;

const FIXTURE: &str = "../../corpus/raw/synthetic/minimal-ltr.pdf";

fn pdfrtl() -> Command {
    Command::new(env!("CARGO_BIN_EXE_pdfrtl"))
}

#[test]
fn inspect_emits_one_json_envelope_and_exits_zero() {
    let out = pdfrtl()
        .args(["--json", "inspect", FIXTURE])
        .output()
        .expect("binary runs");

    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8(out.stdout).expect("stdout is utf-8");
    assert_eq!(
        stdout.lines().count(),
        1,
        "exactly one JSON line on stdout, got: {stdout}"
    );

    let v: serde_json::Value = serde_json::from_str(&stdout).expect("stdout is JSON");
    assert_eq!(v["ok"], serde_json::json!(true));
    assert_eq!(v["data"]["pages"], serde_json::json!(1));
    assert_eq!(v["data"]["producer"], serde_json::json!("pdfrtl-gen"));
    assert_eq!(v["reasons"], serde_json::json!([]));
}

#[test]
fn missing_file_is_exit_4_with_ok_false_and_names_the_file() {
    let out = pdfrtl()
        .args(["--json", "inspect", "../../corpus/raw/synthetic/nope.pdf"])
        .output()
        .expect("binary runs");

    assert_eq!(out.status.code(), Some(4));

    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout is JSON even on failure");
    assert_eq!(v["ok"], serde_json::json!(false));
    assert!(
        v["data"]["error"]
            .as_str()
            .unwrap_or_default()
            .contains("nope.pdf"),
        "error payload must name the file: {v}"
    );
}

#[test]
fn unknown_subcommand_is_exit_2() {
    let out = pdfrtl().args(["nonsense"]).output().expect("binary runs");
    assert_eq!(out.status.code(), Some(2), "usage errors are exit 2");
}

#[test]
fn exit_codes_are_pinned_in_the_library() {
    // Callers branch on these numbers instead of parsing text, so they are public API.
    // Pinned here (not in the binary) so the contract is assertable from outside.
    assert_eq!(pdfrtl_cli::exit::EXIT_OK, 0);
    assert_eq!(pdfrtl_cli::exit::EXIT_UNSUPPORTED, 3);
    assert_eq!(pdfrtl_cli::exit::EXIT_IO, 4);
}

#[test]
fn human_mode_prints_no_json_braces_on_stdout() {
    // Human mode exists so a person (or the future GUI) can read output, but stdout must
    // never mix modes: an agent that forgets --json must see parseable failure, not prose.
    let out = pdfrtl()
        .args(["inspect", FIXTURE])
        .output()
        .expect("binary runs");
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains('{'),
        "human mode must not print JSON: {stdout}"
    );
    assert!(
        stdout.contains("pages:     1"),
        "human mode shows facts: {stdout}"
    );
}

// --- `extract` (P1) -------------------------------------------------------

const CHROME_LAMALEF: &str = "../../corpus/raw/generated/chrome/fa-zwnj-lamalef.pdf";

#[test]
fn extract_json_envelope_carries_logical_text_and_pages() {
    let out = pdfrtl()
        .args(["--json", "extract", CHROME_LAMALEF])
        .output()
        .expect("binary runs");

    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).expect("stdout is utf-8");
    assert_eq!(stdout.lines().count(), 1, "one JSON line, got: {stdout}");

    let v: serde_json::Value = serde_json::from_str(&stdout).expect("stdout is JSON");
    assert_eq!(v["ok"], serde_json::json!(true));
    let text = v["data"]["text"].as_str().expect("data.text is a string");
    let expected = "نیم\u{200C}فاصله و لا اله الا الله — ۱۴۰۳/۰۵/۱۲";
    assert!(expected.contains('\u{200c}'), "guard against a lost ZWNJ");
    assert!(
        text.contains(expected),
        "original sentence with ZWNJ intact, got: {text:?}"
    );
    assert_eq!(v["data"]["pages"][0]["page"], serde_json::json!(1));
    let reasons = v["reasons"].as_array().expect("reasons is an array");
    assert!(
        reasons.iter().any(|r| r.as_str() == Some("actual_text")),
        "reasons: {reasons:?}"
    );
    assert!(
        reasons
            .iter()
            .any(|r| r.as_str() == Some("producer_visual_order_known")),
        "reasons: {reasons:?}"
    );
    // The em-dash + date ordering is run-level reconstruction, reported as such —
    // never a fixture-specific special case (docs/problems/0002).
    assert!(
        reasons.iter().any(|r| r.as_str() == Some("bidi_reordered")),
        "run order was reconstructed: {reasons:?}"
    );
}

/// Builds the smallest PDF whose only glyph is a CID no font can decode:
/// no /ActualText, no /ToUnicode. Refusing (exit 3) is the specified behaviour.
fn write_undecodable_cid_pdf() -> std::path::PathBuf {
    let content = "BT /F1 12 Tf 10 700 Td <0041> Tj ET";
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
           /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>"
            .to_string(),
        "<< /Type /Font /Subtype /Type0 /BaseFont /X /Encoding /Identity-H \
           /DescendantFonts [6 0 R] >>"
            .to_string(),
        format!(
            "<< /Length {} >>\nstream\n{}\nendstream",
            content.len(),
            content
        ),
        "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /X \
           /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> \
           /CIDToGIDMap /Identity >>"
            .to_string(),
    ];

    let mut pdf: Vec<u8> = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (i, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", i + 1, object).as_bytes());
    }
    let startxref = pdf.len();
    let mut table = format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1);
    for offset in &offsets {
        table.push_str(&format!("{offset:010} 00000 n \n"));
    }
    pdf.extend_from_slice(table.as_bytes());
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{startxref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );

    let path =
        std::env::temp_dir().join(format!("pdfrtl-undecodable-cid-{}.pdf", std::process::id()));
    std::fs::write(&path, pdf).expect("temp pdf is writable");
    path
}

#[test]
fn extract_undecodable_cid_is_exit_3_with_unsupported_reason() {
    let path = write_undecodable_cid_pdf();
    let out = pdfrtl()
        .args(["--json", "extract"])
        .arg(&path)
        .output()
        .expect("binary runs");
    let _ = std::fs::remove_file(&path);

    assert_eq!(
        out.status.code(),
        Some(i32::from(pdfrtl_cli::exit::EXIT_UNSUPPORTED)),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("stdout is JSON");
    assert_eq!(v["ok"], serde_json::json!(false));
    assert_eq!(
        v["reasons"],
        serde_json::json!(["unsupported_broken_to_unicode"])
    );
    let error = v["data"]["error"].as_str().unwrap_or_default();
    assert!(
        error.contains("pdfrtl-undecodable-cid"),
        "error payload names the input file: {error}"
    );
}
