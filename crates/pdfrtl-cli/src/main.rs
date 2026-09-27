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

/// `pdfrtl extract <file>`: recover logical text, or refuse with a reason.
///
/// Exit 0 when every page came back with a justification, exit 3 (and `ok: false`)
/// when an `unsupported_*` reason appeared — refusing to guess is a correct outcome,
/// so io errors stay exit 4 and this path never borrows that code.
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

    if let Some(bad) = reasons.iter().find(|reason| reason.is_unsupported()) {
        let message = format!(
            "{}: refused to guess text order ({})",
            file.display(),
            bad.as_str()
        );
        if json {
            let payload = serde_json::json!({ "error": message });
            println!("{}", envelope(false, &payload, &reasons));
        } else {
            eprintln!("pdfrtl: {message}");
        }
        return ExitCode::from(EXIT_UNSUPPORTED);
    }

    let text = pages
        .iter()
        .map(|page| page.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    if json {
        let page_payload: Vec<serde_json::Value> = pages
            .iter()
            .map(|page| serde_json::json!({ "page": page.page, "text": page.text }))
            .collect();
        let payload = serde_json::json!({ "text": text, "pages": page_payload });
        println!("{}", envelope(true, &payload, &reasons));
    } else {
        println!("{text}");
    }
    ExitCode::from(EXIT_OK)
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
