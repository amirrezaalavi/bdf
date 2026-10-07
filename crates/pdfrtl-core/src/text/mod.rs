//! Logical-order text recovery (P1): `/ReversedChars` + `/ActualText` + `ToUnicode`.
//!
//! The producer evidence we build on is documented in `docs/problems/0002`:
//! Chrome/Skia wraps an RTL run in `/ReversedChars BMC … EMC` (PDF 2.0 §14.8.2.3.3),
//! attaches `/ActualText` per glyph *cluster* in visual order, and leaves some clusters
//! unannotated. Recovery therefore works on **units**, never on characters:
//!
//! | stream element | becomes |
//! |---|---|
//! | `<CID> Tj` inside `/Span<</ActualText …>>>` | one unit = the decoded ActualText |
//! | `<CID> Tj` with no ActualText | one unit per CID, decoded via the font's `ToUnicode` |
//!
//! Units inside a `/ReversedChars` run are reversed *as units* and concatenated; lines
//! made of several positioned runs are ordered by UAX #9 levels (candidate orders are
//! verified by re-running UAX #9 forward and comparing against the observed visual
//! order — see `recover`).
use anyhow::{Context, Result};
use lopdf::Document;
use std::collections::HashMap;
use std::path::Path;

pub mod bidi;
pub mod cluster;
pub mod cmap;
pub mod encoding;
pub mod oracle;
pub mod recover;
pub mod state;
mod tables;
pub mod tokenizer;

pub use cmap::ToUnicode;
pub use encoding::{BaseEncoding, Font, SimpleEncoding};
pub use recover::{recover_text, stream_units, PageText};

/// Recover logical text from a PDF file. Never guesses: every page comes back with the
/// [`crate::Reason`]s that justify its order, including an `unsupported_*` reason when
/// no trustworthy path exists (a correct outcome, not an error). Pages fail
/// individually: a page we cannot decode never costs us the other pages.
pub fn extract(path: &Path) -> Result<Vec<PageText>> {
    let doc = Document::load(path).with_context(|| format!("loading {}", path.display()))?;
    extract_document(&doc)
}

/// [`extract`] for an already-loaded document (the MCP adapter and tests use this).
pub fn extract_document(doc: &Document) -> Result<Vec<PageText>> {
    recover::extract_document(doc)
}

/// Font name → decoded font for one page's resources.
pub type FontMap = HashMap<Vec<u8>, Font>;
