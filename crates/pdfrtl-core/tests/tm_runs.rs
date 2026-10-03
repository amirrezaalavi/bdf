//! RED test for the order rung's tie behaviour on `Tm`-positioned runs.
//!
//! Why this fixture exists (measured 2026-10-03 on arabic-3.pdf page 85): that page issues
//! **215 `Tm` operators and no `Td`**, and several of those `Tm`s share one baseline
//! (`1 0 0 1 62.16 795.54`, `1 0 0 1 55.44 795.54`, `1 0 0 1 244.6 795.54`). Four
//! hypotheses about why the units tie were checked and three were wrong — a misread `/W`
//! (our parser is correct), a zero font size (`Tf` sets 10.98), and undecodable fonts
//! freezing the pen (`/F3`–`/F5` are selected but paint nothing). So the fixture below
//! reproduces the *shape* — several runs on one baseline, each with its own origin — and
//! states the required outcome. If it passes already, the tie is elsewhere and this file
//! earns its keep by failing to fail.
//!
//! Required: a line made of two `Tm` runs on the SAME baseline, each holding one RTL
//! `/ActualText` unit, must be DECIDED (its text returned) rather than refused. Two runs
//! are distinguishable because each `Tm` gives its own origin — that is what a producer is
//! saying when it positions text with `Tm`.

use pdfrtl_core::Reason;

/// Two `/ActualText` units, each on its own `Tm`, sharing one baseline.
fn two_runs_one_baseline() -> String {
    format!(
        "BT\n\
         /F1 12 Tf\n\
         1 0 0 -1 100 80 Tm\n\
         /Span<</ActualText <{}> >> BDC (.) Tj EMC\n\
         1 0 0 -1 60 80 Tm\n\
         /Span<</ActualText <{}> >> BDC (.) Tj EMC\n\
         ET\n",
        actual_text("سلام"),
        actual_text("علي")
    )
}

/// UTF-16BE hex for an `/ActualText` value, BOM included (ISO 32000-1 §14.9.4).
fn actual_text(text: &str) -> String {
    let mut hex = String::from("FEFF");
    for unit in text.encode_utf16() {
        hex.push_str(&format!("{unit:04X}"));
    }
    hex
}

fn build(name: &str, stream: &str) -> std::path::PathBuf {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 140] /Contents 4 0 R >>".to_string(),
        format!(
            "<< /Length {} >>\nstream\n{stream}\nendstream",
            stream.len()
        ),
        "<< /Producer (pdfrtl-test) /Creator (pdfrtl-test) >>".to_string(),
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
    let path = std::env::temp_dir().join(format!("pdfrtl-tm-runs-{name}.pdf"));
    std::fs::write(&path, pdf).expect("writes probe");
    path
}

#[test]
fn two_tm_runs_on_one_baseline_are_decided_not_refused() {
    let stream = two_runs_one_baseline();
    let path = build("runs", &stream);
    let pages = pdfrtl_core::extract(&path).expect("loads");
    let _ = std::fs::remove_file(&path);
    let page = pages.into_iter().next().expect("one page");

    let refused_order = page
        .reasons
        .iter()
        .any(|r| matches!(r, Reason::UnsupportedVisualOrder));
    assert!(
        !refused_order,
        "two Tm runs on one baseline are distinguishable by their own origins, so the \
         order rung can decide them; refusing means the pen model collapses every run \
         to the same x. page text={:?} reasons={:?}",
        page.text, page.reasons
    );
    assert!(
        page.text.contains("سلام") && page.text.contains("علي"),
        "both runs' text is returned: {:?}",
        page.text
    );
}

/// arabic-3.pdf page 85 paints 3,101 glyphs from `/F1` across **215 `Tm` operators**, each
/// preceded by its own `Tf` (10.98 / 12), with no `Td` at all. Two runs decide fine; the
/// question is whether density changes the verdict — a producer that re-positions per word
/// is extremely common, and if 215 runs refuse where 2 succeed, the defect is in how the
/// pen or the line key behaves once runs accumulate.
#[test]
fn many_tm_runs_on_one_baseline_still_decide() {
    // Ten runs on one baseline, each with its own Tm and its own Tf, painted right-to-left.
    let words = [
        "سلام",
        "علي",
        "محمد",
        "کتاب",
        "مدرسه",
        "شهر",
        "کشور",
        "دوست",
        "خانه",
        "دریا",
    ];
    let mut stream = String::from("BT\n");
    let mut x = 340.0;
    for word in words {
        stream.push_str(&format!(
            "/F1 10.98 Tf\n1 0 0 -1 {x} 80 Tm\n/Span<</ActualText <{}> >> BDC (.) Tj EMC\n",
            actual_text(word)
        ));
        x -= 30.0;
    }
    stream.push_str("ET\n");

    let path = build("many", &stream);
    let pages = pdfrtl_core::extract(&path).expect("loads");
    let _ = std::fs::remove_file(&path);
    let page = pages.into_iter().next().expect("one page");

    let refused_order = page
        .reasons
        .iter()
        .any(|r| matches!(r, Reason::UnsupportedVisualOrder));
    assert!(
        !refused_order,
        "ten Tm runs on one baseline must decide as two did; a density-dependent refusal \
         points at the line key or the pen. text={:?} reasons={:?}",
        page.text, page.reasons
    );
    for word in words {
        assert!(
            page.text.contains(word),
            "every run's text survives: missing {word:?} in {:?}",
            page.text
        );
    }
}
