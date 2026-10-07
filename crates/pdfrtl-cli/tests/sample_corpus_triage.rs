//! Regression test over external sample PDFs in `/Users/amiralavi/playground/testing/pdf_sample`.
//!
//! If the directory is absent (e.g. running in an isolated CI container), the test is skipped gracefully.
use std::path::Path;
use std::process::Command;

const SAMPLE_DIR: &str = "/Users/amiralavi/playground/testing/pdf_sample";

#[test]
fn sample_corpus_does_not_panic_and_honors_exit_contract() {
    let dir = Path::new(SAMPLE_DIR);
    if !dir.exists() || !dir.is_dir() {
        eprintln!("sample_corpus_triage: directory {SAMPLE_DIR} not found; skipping");
        return;
    }

    let bin = env!("CARGO_BIN_EXE_pdfrtl");

    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    let mut tested = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("pdf") {
            continue;
        }

        tested += 1;
        eprintln!("Testing {}", path.display());

        // 1. inspect verb: must exit 0 and produce valid JSON
        let inspect_output = Command::new(bin)
            .args(["inspect", "--json", path.to_str().unwrap()])
            .output()
            .expect("exec inspect");

        assert_eq!(
            inspect_output.status.code(),
            Some(0),
            "inspect failed on {}",
            path.display()
        );

        let inspect_val: serde_json::Value =
            serde_json::from_slice(&inspect_output.stdout).expect("valid json from inspect");
        assert_eq!(inspect_val["ok"], true);

        // 2. extract verb: must exit 0 or 3 (never panic, never exit 4 or other unhandled codes)
        let extract_output = Command::new(bin)
            .args(["extract", "--json", "--include-unproven", path.to_str().unwrap()])
            .output()
            .expect("exec extract");

        let code = extract_output.status.code().expect("exit code");
        assert!(
            code == 0 || code == 3,
            "extract exited with unexpected code {code} on {}",
            path.display()
        );

        let extract_val: serde_json::Value =
            serde_json::from_slice(&extract_output.stdout).expect("valid json from extract");
        assert!(extract_val.get("ok").is_some());
        assert!(extract_val.get("reasons").is_some());
    }

    assert!(tested >= 10, "expected at least 10 sample PDFs tested, found {tested}");
}
