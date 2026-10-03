//! ACCEPTANCE CRITERIA for the hypothesis space of rung 3 — NOT part of the default suite.
//!
//! `hypothesis_space.rs` is a REQUIRED feature-gated target, not an ordinary integration test,
//! because ONE test in it is deliberately red: it states an outcome the pipeline does not yet
//! produce. A committed failing test inside `cargo test --workspace` makes the whole gate red
//! forever and blocks publication, which is not what a specification should do. Run it
//! explicitly:
//!
//!     cargo test -p pdfrtl-core --features hypothesis-space --test hypothesis_space
//!
//! Two of the three tests PASS today and guard regressions; the third is the RED one. When it
//! turns green, delete the `required-features` line below and move the file back into the
//! default suite — that is the moment the feature is done.
//!
//! Extended 2026-10-03 (P-7) with the cases the extension must NOT break: the pure-RTL tie that
//! must keep refusing, and the guarantee that the extension does not decide everything. A
//! widening that turns refusals into decisions wholesale is the silent-reversal hazard, and only
//! `no_silent_reversal` would catch it — which is too late.
//!
//! RED tests for the hypothesis space of rung 3.
//!
//! Measured on real files (2026-10-03): `arabic-3.pdf` has ~208 units on a line and 9,878
//! lines across the file refuse. The cause is not the comparison and not the prediction — both
//! were fixed and measured. The cause is that the hypothesis space has only TWO members:
//! the stored order, and `invert_units` (a whole-unit reversal plus embedded left-to-right
//! runs). A real line mixes digits, Latin runs, punctuation and spaces at arbitrary points, so
//! no real line is a pure reversal and neither hypothesis reproduces the painting.
//!
//! Each test below is a line the two-member space CANNOT decide, and whose correct answer is a
//! specific one that only a full UAX #9 level pattern can express. They are the acceptance
//! criteria for the hypothesis space: implement until these pass, with `no_silent_reversal`
//! watching throughout.

use pdfrtl_core::text::recover::take_order_trace;
use pdfrtl_core::Reason;

fn pdf_for(name: &str, units: &[Placed]) -> std::path::PathBuf {
    let mut stream = String::from("BT\n/F1 12 Tf\n");
    for unit in units {
        stream.push_str(&format!(
            "1 0 0 1 {} 80 Tm\n/Span<</ActualText <{}> >> BDC (.) Tj EMC\n",
            unit.x,
            actual_text(unit.text)
        ));
    }
    stream.push_str("ET\n");

    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 800 140] /Contents 4 0 R >>".to_string(),
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
    let path = std::env::temp_dir().join(format!("pdfrtl-hypothesis-{name}.pdf"));
    std::fs::write(&path, pdf).expect("writes probe");
    path
}

struct Placed {
    text: &'static str,
    x: f64,
}

/// A PDF whose units ALL share one text origin, so every unit TIES on paint_x.
///
/// This is the shape that reaches rung 3 and finds no measured painted order
/// (`settle_line_by_bidi` returns `Ambiguous` on any tie). It is what a producer with no font
/// widths leaves behind, and it is why 8 Persian files refuse today.
///
/// Uses real glyph codes and a `/ToUnicode` CMap rather than `/ActualText`, so the text DECODES
/// and the line is genuinely decided by the order rung — an `/ActualText` fixture would be
/// decided at rung 1 and would prove nothing here.
fn tied_pdf(name: &str, logical: &[&'static str]) -> std::path::PathBuf {
    let mut table: Vec<char> = Vec::new();
    for line in logical {
        for ch in line.chars() {
            if !table.contains(&ch) {
                table.push(ch);
            }
        }
    }
    let code = |ch: char| table.iter().position(|c| *c == ch).unwrap_or(0) + 1;

    let mut cmap = String::from(
        "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n\
         /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n\
         /CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n\
         1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n",
    );
    cmap.push_str(&format!("{} beginbfchar\n", table.len()));
    for (index, ch) in table.iter().enumerate() {
        cmap.push_str(&format!("<{:04X}> <{:04X}>\n", index + 1, *ch as u32));
    }
    cmap.push_str("endbfchar\nendcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n");

    // VISUAL order (the reversal of logical), one Tj per character, all at ONE origin.
    let mut stream = String::from("BT\n/F1 12 Tf\n-1 0 0 -1 200 80 Tm\n");
    for line in logical {
        for ch in line.chars().rev() {
            stream.push_str(&format!("<{:04X}> Tj\n", code(ch)));
        }
    }
    stream.push_str("ET\n");

    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 140] /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>"
            .to_string(),
        format!("<< /Length {} >>\nstream\n{stream}endstream", stream.len()),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /ToUnicode 6 0 R >>".to_string(),
        format!("<< /Length {} >>\nstream\n{cmap}endstream", cmap.len()),
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
            "trailer\n<< /Size {} /Root 1 0 R /Info 7 0 R >>\nstartxref\n{startxref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    let path = std::env::temp_dir().join(format!("pdfrtl-tied-{name}.pdf"));
    std::fs::write(&path, pdf).expect("writes probe");
    path
}

/// A tied pure-RTL line must KEEP refusing.
///
/// Two readings fit a tied pair equally well, so there is no unique answer and a coin flip is
/// banned (ADR 0002). If an extension of the hypothesis space starts deciding lines like this,
/// it is deciding them by assumption — this is the test that catches it.
#[cfg(feature = "hypothesis-space")]
#[test]
fn a_tied_pure_rtl_line_still_refuses() {
    let path = tied_pdf("pure-rtl", &["سلام دنیا"]);
    let pages = pdfrtl_core::extract(&path).expect("loads");
    let _ = std::fs::remove_file(&path);
    let page = pages.into_iter().next().expect("one page");

    assert!(
        page.reasons.contains(&Reason::UnsupportedVisualOrder),
        "a tied pure-RTL line has no unique reading and must keep refusing; a decision here is \
         a coin flip. text={:?} reasons={:?}",
        page.text,
        page.reasons
    );
    assert!(
        page.text.is_empty(),
        "nothing may be emitted for it: {:?}",
        page.text
    );
}

/// A tied line with ONE LTR island SHOULD become decidable — this is the case P-7 exists for.
///
/// A single digit run inside Persian fixes the level pattern, so exactly one reading reproduces
/// the stored order and the line stops being a guess.
#[cfg(feature = "hypothesis-space")]
#[test]
fn a_tied_single_island_line_becomes_decided() {
    let path = tied_pdf("single-island", &["گزارش سال 1403"]);
    let pages = pdfrtl_core::extract(&path).expect("loads");
    let _ = std::fs::remove_file(&path);
    let page = pages.into_iter().next().expect("one page");
    let _ = take_order_trace();

    assert!(
        !page.reasons.contains(&Reason::UnsupportedVisualOrder),
        "a single LTR island fixes the level pattern, so the line has one reading and the \
         hypothesis space must express it. text={:?} reasons={:?}",
        page.text,
        page.reasons
    );
    assert!(
        page.text.contains("1403"),
        "the digit run survives in reading order: {:?}",
        page.text
    );
    assert!(
        !page.text.contains("3041"),
        "the digits were reversed - the silent corruption ADR 0002 bans: {:?}",
        page.text
    );
}

fn actual_text(text: &str) -> String {
    let mut hex = String::from("FEFF");
    for unit in text.encode_utf16() {
        hex.push_str(&format!("{unit:04X}"));
    }
    hex
}

/// A line with an LTR island in the middle of RTL: the shape of every real line, and the
/// shape `invert_units` cannot express. The correct output keeps the Latin run in reading
/// order inside the reversed Persian — a level pattern, not a reversal.
///
/// Stored order is the logical one: the producer wrote it down, not painted.
#[test]
fn an_ltr_island_inside_rtl_is_decided() {
    // Painted left-to-right: two Persian words around a Latin token, the Latin NOT reversed.
    let units = [
        Placed {
            text: "سلام",
            x: 500.0,
        },
        Placed {
            text: "PDF",
            x: 400.0,
        },
        Placed {
            text: "کتاب",
            x: 300.0,
        },
    ];
    let path = pdf_for("island", &units);
    let pages = pdfrtl_core::extract(&path).expect("loads");
    let _ = std::fs::remove_file(&path);
    let page = pages.into_iter().next().expect("one page");
    let _ = take_order_trace();

    assert!(
        !page.reasons.contains(&Reason::UnsupportedVisualOrder),
        "an LTR island inside RTL is decidable: the producer stored logical order and the \
         painted order confirms it. Refusing here is the two-member hypothesis space \
         failing. text={:?} reasons={:?}",
        page.text,
        page.reasons
    );
    assert!(
        page.text.contains("سلام") && page.text.contains("PDF") && page.text.contains("کتاب"),
        "all three survive: {:?}",
        page.text
    );
    assert!(
        !page.text.contains(&"FP".to_string()) && page.text.contains("PDF"),
        "the Latin island keeps reading order, it is not mirrored: {:?}",
        page.text
    );
}

/// The same line stored in PAINTED (visual) order, which is what a producer that paints as it
/// writes leaves in the file. The rung must invert it back to logical — and the Latin island
/// must come back readable, which a whole-line character reversal would break.
#[test]
fn a_visual_order_line_with_an_ltr_island_is_inverted_to_logical() {
    let units = [
        Placed {
            text: "کتاب",
            x: 300.0,
        },
        Placed {
            text: "PDF",
            x: 400.0,
        },
        Placed {
            text: "سلام",
            x: 500.0,
        },
    ];
    let path = pdf_for("visual-island", &units);
    let pages = pdfrtl_core::extract(&path).expect("loads");
    let _ = std::fs::remove_file(&path);
    let page = pages.into_iter().next().expect("one page");

    assert!(
        !page.reasons.contains(&Reason::UnsupportedVisualOrder),
        "a visual-order line is decidable too: the painted order says which reading it is. \
         text={:?} reasons={:?}",
        page.text,
        page.reasons
    );
    assert!(
        page.text.contains("سلام") && page.text.contains("کتاب") && page.text.contains("PDF"),
        "logical order is recovered: {:?}",
        page.text
    );
}

/// The shape that actually fails, from the measurement: a LONG mixed line. The three-unit
/// island above decides, because `invert_units` handles one LTR run inside a reversed line.
/// `arabic-3.pdf`'s lines hold ~208 units with digits, slashes, spaces and Latin scattered
/// through Persian — several LTR runs, several number runs, punctuation. Each is a separate
/// level run, and `invert_units` models one.
///
/// Requirement: a long mixed line is decided, and the digits survive UNREVERSED — `1403/05/12`
/// must not come back as `21/50/3041`, which is the specific corruption ADR 0002 bans.
#[cfg(feature = "hypothesis-space")]
#[test]
fn a_long_mixed_line_is_decided_and_its_digits_survive() {
    // One line, painted right-to-left as a producer would: a date, a Latin token, a
    // two-digit count, a Persian phrase, a parenthetical with its own digit.
    let logical = [
        "تاریخ:",
        "1403/05/12",
        "PDF",
        "شماره",
        "25",
        "(ویرایش",
        "3",
        ")",
        "کتاب",
    ];
    let mut units = Vec::new();
    let mut x = 700.0;
    for text in logical {
        units.push(Placed { text, x });
        x -= 60.0;
    }

    let path = pdf_for("long-mixed", &units);
    let pages = pdfrtl_core::extract(&path).expect("loads");
    let _ = std::fs::remove_file(&path);
    let page = pages.into_iter().next().expect("one page");
    let _ = take_order_trace();

    assert!(
        !page.reasons.contains(&Reason::UnsupportedVisualOrder),
        "a long mixed line is still one line with one order; the hypothesis space must be \
         able to express it. text={:?} reasons={:?}",
        page.text,
        page.reasons
    );
    assert!(
        page.text.contains("1403/05/12"),
        "the date survives with its digits in reading order, not reversed: {:?}",
        page.text
    );
    assert!(
        !page.text.contains("21/50/3041"),
        "a reversed date is the silent corruption ADR 0002 bans: {:?}",
        page.text
    );
    assert!(
        page.text.contains("PDF") && page.text.contains("کتاب"),
        "the Latin token and the Persian phrase both survive: {:?}",
        page.text
    );
}
