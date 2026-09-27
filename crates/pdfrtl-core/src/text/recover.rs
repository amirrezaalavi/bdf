//! Page text recovery — STUB (TDD red phase).
use crate::reasons::Reason;
use crate::text::cmap::ToUnicode;
use anyhow::Result;
use lopdf::Document;
use std::collections::HashMap;

/// One page's recovered text plus the reasons that justify its order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageText {
    pub page: u32,
    pub text: String,
    pub reasons: Vec<Reason>,
}

/// STUB: recover logical text from one content stream.
pub fn recover_text(_stream: &[u8], _fonts: &HashMap<Vec<u8>, ToUnicode>) -> (String, Vec<Reason>) {
    (String::new(), Vec::new())
}

/// STUB: the units a producer wrote, in stream order.
pub fn stream_units(_stream: &[u8], _fonts: &HashMap<Vec<u8>, ToUnicode>) -> Vec<String> {
    Vec::new()
}

/// STUB: per-page recovery for a loaded document.
pub fn extract_document(_doc: &Document) -> Result<Vec<PageText>> {
    Ok(Vec::new())
}
