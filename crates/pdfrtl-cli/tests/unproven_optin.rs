//! The default envelope must not carry withheld TEXT.
//!
//! Review §3: a field that is "frequently reversed" sitting in the same JSON blob as the good
//! text is the silent-reversal hazard this project exists to stop, and the callers most likely
//! to be bitten are LLM/MCP consumers that read the WHOLE object. So the counts stay in the
//! default output and the text does not.

use std::process::Command;

fn pdfrtl() -> Command {
    Command::new(env!("CARGO_BIN_EXE_pdfrtl"))
}

/// A tracked fixture. Any of them works for the negative property below.
const FIXTURE: &str = "../../corpus/raw/synthetic/family-chrome-logical.pdf";

/// The private archive, when present. Measured 2026-10-03: NONE of the 14 tracked fixtures
/// withhold anything, so the positive case cannot be asserted against the public corpus.
const PRIVATE: &str = "../../corpus/raw/private/desktop-pdfs/arabic-3.pdf";

/// Resolve a path relative to this crate's manifest dir.
fn at(rel: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)
}

#[test]
fn default_json_carries_no_unproven_text() {
    let out = pdfrtl()
        .args(["--json", "extract"])
        .arg(at(FIXTURE))
        .output()
        .expect("pdfrtl runs");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("valid JSON envelope");

    let pages = v["data"]["pages"].as_array().expect("pages array");
    for p in pages {
        assert!(
            p["unproven"].as_array().map_or(0, |a| a.len()) == 0,
            "default envelope leaked unproven text on page {}",
            p["page"]
        );
    }
}

#[test]
fn default_json_still_reports_the_counts() {
    // The gate must be sized without reading the text: a caller that cannot see the withheld
    // lines must still be able to see HOW MANY there are.
    let out = pdfrtl()
        .args(["--json", "extract"])
        .arg(at(FIXTURE))
        .output()
        .expect("pdfrtl runs");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("valid JSON envelope");

    assert!(
        v["data"].get("unproven_lines").is_some(),
        "unproven_lines must be present even when the text is withheld from the output"
    );
    assert!(v["data"].get("unordered_chars").is_some());
}

/// The positive case: with the flag, withheld TEXT appears; without it, only counts.
#[test]
fn the_flag_reveals_withheld_text_when_a_file_withholds() {
    let path = at(PRIVATE);
    if !path.exists() {
        eprintln!(
            "SKIPPED with reason: the private archive is absent (public clone), and no tracked \
             fixture withholds text - measured 2026-10-03, unproven_lines == 0 on all 14. \
             Asserting this against a non-withholding fixture would pass vacuously \
             (problems/0006, docs/problems/0017)."
        );
        return;
    }

    let plain: serde_json::Value = serde_json::from_slice(
        &pdfrtl()
            .args(["--json", "extract"])
            .arg(&path)
            .output()
            .expect("pdfrtl runs")
            .stdout,
    )
    .expect("json");

    let flagged: serde_json::Value = serde_json::from_slice(
        &pdfrtl()
            .args(["--json", "--include-unproven", "extract"])
            .arg(&path)
            .output()
            .expect("pdfrtl runs")
            .stdout,
    )
    .expect("json");

    let count = |v: &serde_json::Value| -> usize {
        v["data"]["pages"]
            .as_array()
            .expect("pages")
            .iter()
            .map(|p| p["unproven"].as_array().map_or(0, |a| a.len()))
            .sum()
    };

    assert!(
        count(&plain) == 0,
        "default envelope must carry no withheld TEXT"
    );
    assert!(
        flagged["data"]["unproven_lines"].as_u64().unwrap_or(0) > 0,
        "precondition: this file must withhold something"
    );
    assert!(
        count(&flagged) > 0,
        "--include-unproven must reveal the withheld text"
    );
    assert_eq!(
        plain["data"]["text"], flagged["data"]["text"],
        "--include-unproven changed data.text; the proven field must never move"
    );
}

#[test]
fn the_flag_is_opt_in_and_does_not_change_the_main_text() {
    // data.text must be byte-identical with and without the flag. The field is additive
    // information about withheld text; it can never reach the proven field.
    let plain = pdfrtl()
        .args(["--json", "extract"])
        .arg(at(FIXTURE))
        .output()
        .expect("pdfrtl runs");
    let flagged = pdfrtl()
        .args(["--json", "--include-unproven", "extract"])
        .arg(at(FIXTURE))
        .output()
        .expect("pdfrtl runs");

    let a: serde_json::Value = serde_json::from_slice(&plain.stdout).expect("json");
    let b: serde_json::Value = serde_json::from_slice(&flagged.stdout).expect("json");

    assert_eq!(
        a["data"]["text"], b["data"]["text"],
        "--include-unproven changed data.text; the proven field must never move"
    );
}
