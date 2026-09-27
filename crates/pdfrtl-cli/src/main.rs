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
use pdfrtl_cli::exit::{EXIT_IO, EXIT_OK};
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
    }
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
