//! TDD: level-1 inspection against the committed synthetic fixture.
//!
//! Fixture: corpus/raw/synthetic/minimal-ltr.pdf, produced by tools/gen_minimal_pdf.py
//! (regenerate with `python tools/gen_minimal_pdf.py`, byte-identical every run).

use pdfrtl_core::inspect;
use std::path::Path;

const FIXTURE: &str = "../../corpus/raw/synthetic/minimal-ltr.pdf";

#[test]
fn inspect_reports_pages_and_producer() {
    let info = inspect(Path::new(FIXTURE)).expect("fixture must parse");
    assert_eq!(info.pages, 1);
    assert_eq!(info.producer.as_deref(), Some("pdfrtl-gen"));
    assert_eq!(info.creator.as_deref(), Some("pdfrtl-gen"));
    assert!(!info.encrypted);
    assert_eq!(
        info.has_actual_text, None,
        "P0 must report None, not a misleading false"
    );
}

#[test]
fn inspect_rejects_a_missing_file_with_context() {
    let err = inspect(Path::new("../../corpus/raw/synthetic/does-not-exist.pdf"))
        .expect_err("missing file must be an error");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("does-not-exist.pdf"),
        "error must name the file, got: {msg}"
    );
}
