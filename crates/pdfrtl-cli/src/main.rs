//! pdfrtl CLI.
//!
//! Contract (see docs/CLI.md):
//!   * every verb prints exactly ONE JSON envelope on stdout — no prose, no logs;
//!   * envelope shape: `{"ok": bool, "data": <verb payload>, "reasons": [<Reason>…]}`;
//!   * exit codes: 0 ok · 2 usage (clap) · 3 unsupported text (with a Reason) · 4 io/parse error.
//!
//! `reasons` is empty for verbs that do not touch text. A non-empty `reasons` with
//! `ok: true` means "produced, with this justification"; `ok: false` + an
//! `unsupported_*` reason means "refused to guess" (exit 3).
//!
//! `extract` is page-granular (ADR 0004): `ok` is true only when EVERY page decoded,
//! but a partially decoded document still returns the text it recovered, labelled
//! per page as `{"page": N, "text": …, "ok": bool, "reasons": […], "unordered_chars": N}`
//! — no page's text is ever discarded because another page failed.
//!
//! Order gating: text is logical or it is not emitted. A page whose ORDER was never
//! established (`unsupported_visual_order`) reports `text: ""` and its decoded
//! characters as `unordered_chars`, at page level and in `data.unordered_chars`.
//! A caller that reads `data.text` and ignores `reasons` still cannot be handed
//! reversed text.
use clap::{Parser, Subcommand};
use pdfrtl_cli::exit::{EXIT_IO, EXIT_OK, EXIT_UNSUPPORTED};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "pdfrtl", version, about = "RTL-first PDF toolkit for agents")]
struct Cli {
    /// Emit the JSON envelope on stdout (default is human-readable text)
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Report document facts: pages, producer, encryption, marked content
    Inspect {
        /// Path to a PDF file
        file: PathBuf,
    },
    /// Recover logical-order text: /ReversedChars runs, /ActualText clusters, ToUnicode
    Extract {
        /// Path to a PDF file
        file: PathBuf,
    },
}

fn envelope<T: serde::Serialize>(ok: bool, data: &T, reasons: &[pdfrtl_core::Reason]) -> String {
    let value = serde_json::json!({
        "ok": ok,
        "data": data,
        "reasons": reasons.iter().map(|r| r.as_str()).collect::<Vec<_>>(),
    });
    value.to_string()
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.cmd {
        Cmd::Inspect { file } => match pdfrtl_core::inspect(&file) {
            Ok(info) => {
                if cli.json {
                    println!("{}", envelope(true, &info, &[]));
                } else {
                    print!("{}", human(&info));
                }
                ExitCode::from(EXIT_OK)
            }
            Err(err) => {
                let message = format!("{err:#}");
                if cli.json {
                    let payload = serde_json::json!({ "error": message });
                    println!("{}", envelope(false, &payload, &[]));
                } else {
                    // Humans get stderr; stdout stays clean for piping.
                    eprintln!("pdfrtl: {message}");
                }
                ExitCode::from(EXIT_IO)
            }
        },
        Cmd::Extract { file } => run_extract(cli.json, &file),
    }
}

/// `pdfrtl extract <file>`: recover logical text page by page, or refuse — per page.
///
/// * every page decoded → `ok: true`, exit 0 (the shape callers rely on);
/// * SOME pages decoded → `ok: false`, exit 3, **and** the text of every page whose
///   ORDER was established stays in `data.text` / `data.pages`. Characters we
///   decoded but could not order are counted in `unordered_chars`, never emitted
///   (ADR 0004) — decoding is never thrown away, unproven text is never shown;
/// * NO page decoded → `ok: false`, exit 3, `data.error` naming the file.
///
/// Refusing to guess is a correct outcome, so io errors stay exit 4 and this path
/// never borrows that code: exit 4 means "the document itself could not be opened".
fn run_extract(json: bool, file: &std::path::Path) -> ExitCode {
    let pages = match pdfrtl_core::extract(file) {
        Ok(pages) => pages,
        Err(err) => {
            let message = format!("{err:#}");
            if json {
                let payload = serde_json::json!({ "error": message });
                println!("{}", envelope(false, &payload, &[]));
            } else {
                eprintln!("pdfrtl: {message}");
            }
            return ExitCode::from(EXIT_IO);
        }
    };

    let mut reasons: Vec<pdfrtl_core::Reason> = Vec::new();
    for page in &pages {
        for reason in &page.reasons {
            if !reasons.contains(reason) {
                reasons.push(*reason);
            }
        }
    }
    reasons.sort_by_key(|reason| *reason as u32);

    let decoded = pages.iter().filter(|page| page.is_decoded()).count();
    let total = pages.len();
    let all_decoded = decoded == total;
    // `data.text` is the text we stand behind: a page whose order was never
    // established has had its characters withdrawn into `unordered_chars`, and a
    // blank page contributes nothing — so neither can pad or pollute the output.
    let text = pages
        .iter()
        .filter(|page| !page.text.is_empty())
        .map(|page| page.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let emitted = pages.iter().filter(|page| !page.text.is_empty()).count();
    let withheld: usize = pages.iter().map(|page| page.unordered_chars).sum();
    let withheld_note = if withheld > 0 {
        format!("; {withheld} characters decoded but withheld (order unestablished)")
    } else {
        String::new()
    };

    if all_decoded {
        if json {
            println!(
                "{}",
                envelope(true, &extract_payload(&text, &pages), &reasons)
            );
        } else {
            println!("{text}");
        }
        return ExitCode::from(EXIT_OK);
    }

    let summary = if emitted == 0 {
        format!(
            "{}: refused to guess text order ({}){}",
            file.display(),
            reasons
                .iter()
                .find(|reason| reason.is_unsupported())
                .map(|reason| reason.as_str())
                .unwrap_or("unsupported_no_evidence"),
            withheld_note
        )
    } else {
        format!(
            "{}: text from {} of {} pages; {} page(s) refused ({}){}",
            file.display(),
            emitted,
            total,
            total - decoded,
            reasons
                .iter()
                .filter(|reason| reason.is_unsupported())
                .map(|reason| reason.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            withheld_note
        )
    };

    if json {
        let mut payload = extract_payload(&text, &pages);
        payload["error"] = serde_json::json!(summary);
        println!("{}", envelope(false, &payload, &reasons));
    } else {
        if !text.is_empty() {
            println!("{text}");
        }
        eprintln!("pdfrtl: {summary}");
    }
    ExitCode::from(EXIT_UNSUPPORTED)
}

/// The `extract` payload: full text plus one entry per page, each labelled with
/// whether that page decoded and why (or why not).
///
/// Order gating (ADR 0004): a page whose order was never established reports
/// `text: ""` and its decoded characters as `unordered_chars`. There is no
/// third outcome — `text` is logical or it is empty.
///
/// `unproven` carries the TEXT of those withheld lines, in the order it was STORED and never
/// merged into `text`. A caller that reads `text` alone still gets proven-or-nothing; a caller
/// that wants the unproven text opts in by reading a field whose name says what it is. On a
/// producer that stores visual order this text is REVERSED, which is exactly why it does not
/// belong in `text`.
fn extract_payload(text: &str, pages: &[pdfrtl_core::PageText]) -> serde_json::Value {
    let page_payload: Vec<serde_json::Value> = pages
        .iter()
        .map(|page| {
            serde_json::json!({
                "page": page.page,
                "text": page.text,
                "ok": page.is_decoded(),
                "reasons": page.reasons.iter().map(|r| r.as_str()).collect::<Vec<_>>(),
                "unordered_chars": page.unordered_chars,
                "unproven": page.unproven,
            })
        })
        .collect();
    let unordered: usize = pages.iter().map(|page| page.unordered_chars).sum();
    let unproven_lines: usize = pages.iter().map(|page| page.unproven.len()).sum();
    serde_json::json!({
        "text": text,
        "unordered_chars": unordered,
        "unproven_lines": unproven_lines,
        "pages": page_payload,
    })
}

/// Human-readable rendering of `inspect`. Never used by agents (they pass `--json`),
/// so this is free of contract obligations — but it must stay deterministic.
fn human(info: &pdfrtl_core::DocInfo) -> String {
    format!(
        "{}\n  pages:     {}\n  producer:  {}\n  creator:   {}\n  pdf ver:   {}\n  encrypted: {}\n",
        info.path,
        info.pages,
        info.producer.as_deref().unwrap_or("-"),
        info.creator.as_deref().unwrap_or("-"),
        info.pdf_version.as_deref().unwrap_or("-"),
        info.encrypted,
    )
}
