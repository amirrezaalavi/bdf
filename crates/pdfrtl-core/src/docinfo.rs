//! Level-1 inspection: what the file *is*, before any text handling.
//!
//! P0 scope: page count, Info-dictionary producer/creator, PDF version,
//! encryption flag. Text-level facts (fonts, `ToUnicode`, `/ActualText`) come in P1
//! and are reported as `Option::None` until then — never as a misleading `false`.
use anyhow::{Context, Result};
use lopdf::Document;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct DocInfo {
    pub path: String,
    pub pages: u32,
    pub producer: Option<String>,
    pub creator: Option<String>,
    pub pdf_version: Option<String>,
    pub encrypted: bool,
    /// `None` until P1 analyses marked content. Not `Some(false)`.
    pub has_actual_text: Option<bool>,
    /// `None` until P1. Number of embedded fonts.
    pub font_count: Option<u32>,
}

/// Inspect a PDF file. Never decodes text; safe on arbitrary input.
pub fn inspect(path: &Path) -> Result<DocInfo> {
    let doc = Document::load(path).with_context(|| format!("loading {}", path.display()))?;

    let info_dict = doc
        .trailer
        .get(b"Info")
        .ok()
        .and_then(|o| o.as_reference().ok())
        .and_then(|id| doc.get_object(id).ok())
        .and_then(|o| o.as_dict().ok());

    let get_info_str = |key: &[u8]| -> Option<String> {
        info_dict
            .and_then(|d| d.get(key).ok())
            .and_then(|o| o.as_str().ok())
            .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
    };

    Ok(DocInfo {
        path: path.display().to_string(),
        pages: doc.get_pages().len() as u32,
        producer: get_info_str(b"Producer"),
        creator: get_info_str(b"Creator"),
        pdf_version: Some(doc.version.clone()),
        encrypted: doc.is_encrypted(),
        has_actual_text: None,
        font_count: None,
    })
}
