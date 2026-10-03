//! Prints the order trace for a real corpus file. Diagnostic only, and NOT committed to the
//! repo's suite in this form: it needs a private corpus file, and it prints. What it does
//! commit is the shape — the core RETURNS the trace (`take_order_trace`), and the reader is
//! the only thing that prints, because the core must not (AGENTS.md, slop gate).
//!
//! Run it directly:
//!   PDFRTL_TRACE_ORDER=1 PDFRTL_FILE=... cargo test -p pdfrtl-core --test order_trace -- --nocapture

use pdfrtl_core::text::recover::{take_order_trace, take_outcome_counts};

#[test]
fn print_order_trace_for_a_real_file() {
    let Ok(path) = std::env::var("PDFRTL_FILE") else {
        // No file given: assert the trace is empty, which is also the control that proves
        // the env gate works — a trace that records when the gate is off is a broken gate.
        assert!(
            take_order_trace().is_empty(),
            "with PDFRTL_TRACE_ORDER unset the trace stays empty"
        );
        return;
    };
    unsafe { std::env::set_var("PDFRTL_TRACE_ORDER", "1") };

    let pages = pdfrtl_core::extract(std::path::Path::new(&path)).expect("loads");
    let trace = take_order_trace();

    println!("file: {}", path);
    println!("pages: {}", pages.len());
    println!("traced undecidable lines: {}", trace.len());
    for line in trace.iter().take(12) {
        println!("  {line}");
    }

    // Summarise the signatures rather than dumping every line.
    let mass_tie = trace
        .iter()
        .filter(|l| l.contains("span=0.000..0.000"))
        .count();
    let zero_geom = trace
        .iter()
        .filter(|l| l.contains("geometry=false"))
        .count();
    let [both_fit, stored_only, reversed_only, neither] = take_outcome_counts();
    println!("--- rung 3 outcomes across the file ---");
    println!("  stored_only   (decided, kept):        {stored_only}");
    println!("  reversed_only (decided, inverted):     {reversed_only}");
    println!("  both_fit      (REFUSED as ambiguous):  {both_fit}");
    println!("  neither       (REFUSED unexplained):  {neither}");
    let decided = stored_only + reversed_only;
    let refused = both_fit + neither;
    println!(
        "  decided {decided}  refused {refused}  -> both_fit is {:.2}% of all lines",
        if decided + refused == 0 {
            0.0
        } else {
            100.0 * both_fit as f64 / (decided + refused) as f64
        }
    );

    println!("mass ties (span 0..0): {mass_tie}");
    println!("lines without usable geometry: {zero_geom}");

    assert!(
        !trace.is_empty(),
        "the trace recorded nothing — is the gate on?"
    );
}
