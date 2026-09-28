//! Level-1 inspection: what the file *is*, before any text handling.
//!
//! P0 scope: page count, Info-dictionary producer/creator, PDF version,
//! encryption flag. Text-level facts (fonts, `ToUnicode`, `/ActualText`) come in P1
//! and are reported as `Option::None` until then — never as a misleading `false`.
use anyhow::{Context, Result};
use lopdf::{Document, Object};
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

/// Decode a PDF text string: UTF-16 when it carries a BOM (Microsoft Word writes
/// its `/Producer` that way), otherwise UTF-8 when the bytes are valid UTF-8,
/// otherwise Latin-1 — PDF strings are never UTF-8 by spec, and `%C2%AE`-style
/// lossy decoding is how `Microsoft(R) Word` turned into `Microsoft? Word`.
fn decode_pdf_text(bytes: &[u8]) -> String {
    let utf16 = |be: bool| -> String {
        let mut units = Vec::with_capacity(bytes.len() / 2);
        let mut i = 2;
        while i + 1 < bytes.len() {
            let pair = [bytes[i], bytes[i + 1]];
            units.push(if be {
                u16::from_be_bytes(pair)
            } else {
                u16::from_le_bytes(pair)
            });
            i += 2;
        }
        String::from_utf16_lossy(&units)
    };
    if bytes.starts_with(&[0xFE, 0xFF]) {
        utf16(true)
    } else if bytes.starts_with(&[0xFF, 0xFE]) {
        utf16(false)
    } else if let Ok(text) = std::str::from_utf8(bytes) {
        text.to_string()
    } else {
        bytes.iter().map(|&b| b as char).collect()
    }
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
            // Producers routinely store `/Producer` as an indirect reference.
            .and_then(|o| match o {
                Object::Reference(id) => doc.get_object(*id).ok(),
                other => Some(other),
            })
            .and_then(|o| o.as_str().ok())
            .map(decode_pdf_text)
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
