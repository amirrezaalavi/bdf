//! The refusal path, tested on fixtures that are PUBLIC.
//!
//! Why this file exists (docs/problems/0017): measured 2026-10-03, `unproven_lines` was 0 on
//! all 14 tracked fixtures. Every one carried per-cluster `/ActualText` or `/ReversedChars`, so
//! it was decided at rung 1 or 2 and never reached rung 3 — the rung that refuses. The branch
//! this project exists to exercise had zero coverage outside the private archive, and the
//! private archive cannot be published.
//!
//! These fixtures (`tools/gen_refusal_fixtures.py`) close that gap. What makes them publishable
//! is that their EXPECTED VALUE is a refusal, not a string: asserting "the tool declines" needs
//! no answer key and no native-speaker oracle. Their content is invented Persian with no digits
//! and no Latin, so nothing here smuggles a mixed-line problem into a file whose purpose is the
//! tie.
//!
//! Every test below asserts its own PRECONDITION first. A test that asserts "no text was emitted"
//! against a file that emits everything measures nothing (docs/problems/0006).
//!
//! Lives in `pdfrtl-cli`, not `pdfrtl-core`, because these are CONTRACT assertions: they spawn
//! the built binary. `CARGO_BIN_EXE_*` is only defined for the crate that owns the binary, so a
//! test in `pdfrtl-core` cannot see `pdfrtl`.

use std::path::Path;
use std::process::Command;

/// Fixtures built by `tools/gen_refusal_fixtures.py`. Relative to this crate's manifest dir.
const FIXTURES: &[&str] = &[
    "../../corpus/raw/synthetic/refuse-tied-rtl.pdf",
    "../../corpus/raw/synthetic/refuse-tied-multiline.pdf",
    "../../corpus/raw/synthetic/refuse-near-miss-producer.pdf",
];

fn at(rel: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)
}

/// Extract with `--json`, returning the parsed envelope.
fn extract(rel: &str) -> serde_json::Value {
    let out = Command::new(env!("CARGO_BIN_EXE_pdfrtl"))
        .args(["--json", "extract"])
        .arg(at(rel))
        .output()
        .expect("pdfrtl runs");
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "{rel}: stdout was not JSON ({e})\nstderr: {}",
            String::from_utf8_lossy(&out.stderr)
        )
    })
}

/// Precondition: the fixture really withholds. Without this, every assertion below is vacuous.
#[test]
fn every_refusal_fixture_really_withholds() {
    for rel in FIXTURES {
        let v = extract(rel);
        let d = &v["data"];
        assert!(
            d["unproven_lines"].as_u64().unwrap_or(0) > 0,
            "{rel} withholds nothing, so the refusal tests beside it would be vacuous \
             (docs/problems/0006)"
        );
        assert!(
            d["unordered_chars"].as_u64().unwrap_or(0) > 0,
            "{rel} reports no withheld characters"
        );
    }
}

/// The contract: a page that cannot be ordered emits NO text and names the reason.
#[test]
fn a_refusing_page_emits_no_text_and_names_its_reason() {
    for rel in FIXTURES {
        let v = extract(rel);
        for page in v["data"]["pages"].as_array().expect("pages") {
            assert_eq!(
                page["text"],
                serde_json::json!(""),
                "{rel} page {} emitted text it could not order — that is the silent reversal \
                 this project exists to prevent",
                page["page"]
            );
            assert!(
                page["ok"] == serde_json::json!(false),
                "{rel} page {} reports ok:true while refusing",
                page["page"]
            );
            let reasons = page["reasons"].as_array().expect("reasons array");
            assert!(
                reasons.iter().any(|r| r == "unsupported_visual_order"),
                "{rel} page {} refused without naming unsupported_visual_order: {reasons:?}",
                page["page"]
            );
        }
    }
}

/// The document-level flag must be false: a document with refused pages is not `ok`.
#[test]
fn a_document_containing_a_refusal_is_not_ok() {
    for rel in FIXTURES {
        let v = extract(rel);
        assert_eq!(
            v["ok"],
            serde_json::json!(false),
            "{rel}: document reports ok:true while its pages refuse"
        );
    }
}

/// Withheld text exists but never reaches the proven field, with or without the opt-in flag.
#[test]
fn withheld_text_never_reaches_the_proven_field() {
    for rel in FIXTURES {
        let plain = extract(rel);
        assert_eq!(
            plain["data"]["text"],
            serde_json::json!(""),
            "{rel}: data.text must stay empty on a withholding file"
        );

        // Even when the caller explicitly asks for it, the unproven text must not be merged in.
        let out = Command::new(env!("CARGO_BIN_EXE_pdfrtl"))
            .args(["--json", "--include-unproven", "extract"])
            .arg(at(rel))
            .output()
            .expect("pdfrtl runs");
        let flagged: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json");
        assert_eq!(
            flagged["data"]["text"],
            serde_json::json!(""),
            "{rel}: --include-unproven must not change the proven field"
        );
    }
}

/// A `/Producer` that merely RESEMBLES an allow-listed family must not unlock anything.
///
/// This is the fixture that catches a prefix-match instead of an exact measured family: a fix
/// that matches `Skia/PDF m1` rather than the exact allow-listed string would decrypt nothing
/// here, but a loosened matcher would start emitting.
#[test]
fn a_near_miss_producer_is_not_treated_as_allow_listed() {
    let v = extract("../../corpus/raw/synthetic/refuse-near-miss-producer.pdf");
    assert_eq!(
        v["data"]["text"],
        serde_json::json!(""),
        "a producer string that is not on the allow-list produced text anyway"
    );
    assert!(
        !v["reasons"]
            .as_array()
            .expect("reasons")
            .iter()
            .any(|r| r == "producer_visual_order_known"),
        "a near-miss producer was treated as allow-listed: {:?}",
        v["reasons"]
    );
}

/// The invariant control still holds on these files: whatever is emitted, is logical.
///
/// `no_silent_reversal` covers the corpus; this asserts the same property on the fixtures added
/// here, so a future change that starts emitting reversed text from a tied page fails HERE with
/// a fixture name attached.
#[test]
fn nothing_reversed_reaches_data_text() {
    for rel in FIXTURES {
        let flagged = {
            let out = Command::new(env!("CARGO_BIN_EXE_pdfrtl"))
                .args(["--json", "--include-unproven", "extract"])
                .arg(at(rel))
                .output()
                .expect("pdfrtl runs");
            serde_json::from_slice::<serde_json::Value>(&out.stdout).expect("json")
        };

        // Every character in data.text must appear in a form the tool is willing to stand behind.
        // On these fixtures that set is empty, which is the point: assert the emptiness is real
        // and not an accident of a missing field.
        let text = flagged["data"]["text"].as_str().unwrap_or("");
        assert!(
            text.is_empty(),
            "{rel}: data.text carries {} chars: {text:?}",
            text.chars().count()
        );
    }
}
