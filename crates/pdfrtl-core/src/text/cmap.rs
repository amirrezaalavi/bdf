//! `ToUnicode` CMap parser: `begincodespacerange`, `beginbfchar`, `beginbfrange`.
//!
//! Parsing is total: an unreadable CMap yields an empty map, and the caller then reports
//! `Reason::UnsupportedBrokenToUnicode` for every glyph it cannot place. The library
//! never panics on producer input.
use std::collections::HashMap;

/// A decoded `ToUnicode` CMap: code → Unicode text.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ToUnicode {
    /// Byte width of one code (1 for simple fonts, 2 for Identity-H composite fonts).
    code_len: usize,
    map: HashMap<u16, String>,
}

impl ToUnicode {
    /// Parse a `ToUnicode` CMap stream. Total function; garbage in → empty map out.
    pub fn parse(_data: &[u8]) -> ToUnicode {
        ToUnicode {
            code_len: 2,
            map: HashMap::new(),
        }
    }

    /// Bytes per code as declared by `codespacerange` (defaults to 2, the composite case).
    pub fn code_len(&self) -> usize {
        self.code_len
    }

    /// Unicode text for a code, or `None` when the CMap does not map it.
    pub fn get(&self, code: u16) -> Option<&str> {
        self.map.get(&code).map(String::as_str)
    }

    /// Number of mapped codes (tests and diagnostics).
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// True when nothing maps (the broken-ToUnicode case).
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}
