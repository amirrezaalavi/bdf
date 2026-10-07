//! `ToUnicode` CMap parser: `begincodespacerange`, `beginbfchar`, `beginbfrange`.
//!
//! Parsing is total: an unreadable CMap yields an empty map, and the caller then reports
//! `Reason::UnsupportedBrokenToUnicode` for every glyph it cannot place. The library
//! never panics on producer input. The code byte width comes from
//! `begincodespacerange`, so 1-byte Type1 codes decode as single bytes.
use crate::text::tokenizer::{tokenize, TextOp, Token};
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
    pub fn parse(data: &[u8]) -> ToUnicode {
        let tokens = tokenize(data);
        let mut map: HashMap<u16, String> = HashMap::new();
        let mut code_len: Option<usize> = None;
        let mut mode = Mode::None;
        let mut pending: Vec<Vec<u8>> = Vec::new();
        let mut i = 0usize;

        while i < tokens.len() {
            match &tokens[i] {
                Token::Op(op, _) => {
                    match op {
                        TextOp::BeginCodeSpaceRange => {
                            mode = Mode::CodeSpace;
                            pending.clear();
                        }
                        TextOp::BeginBfChar => {
                            mode = Mode::BfChar;
                            pending.clear();
                        }
                        TextOp::BeginBfRange => {
                            mode = Mode::BfRange;
                            pending.clear();
                        }
                        TextOp::EndCodeSpaceRange | TextOp::EndBfChar | TextOp::EndBfRange => {
                            mode = Mode::None;
                            pending.clear();
                        }
                        _ => {}
                    }
                    i += 1;
                }
                Token::Str(bytes) => {
                    i += 1;
                    if mode == Mode::None {
                        continue;
                    }
                    pending.push(bytes.to_vec());
                    match mode {
                        Mode::CodeSpace => {
                            if pending.len() == 2 {
                                if code_len.is_none() {
                                    code_len = Some(pending[0].len());
                                }
                                pending.clear();
                            }
                        }
                        Mode::BfChar => {
                            if pending.len() == 2 {
                                if let (Some(code), Some(text)) =
                                    (to_u16(&pending[0]), utf16be_to_string(&pending[1]))
                                {
                                    map.insert(code, text);
                                }
                                pending.clear();
                            }
                        }
                        Mode::BfRange => {
                            if pending.len() == 3 {
                                insert_scalar_range(&mut map, &pending);
                                pending.clear();
                            }
                        }
                        Mode::None => {}
                    }
                }
                Token::ArrStart if mode == Mode::BfRange => {
                    i += 1;
                    let mut items: Vec<Vec<u8>> = Vec::new();
                    while i < tokens.len() && !matches!(tokens[i], Token::ArrEnd) {
                        if let Token::Str(bytes) = &tokens[i] {
                            items.push(bytes.to_vec());
                        }
                        i += 1;
                    }
                    if i < tokens.len() {
                        i += 1; // consume `]`
                    }
                    if pending.len() == 2 {
                        insert_array_range(&mut map, &pending[0], &pending[1], &items);
                    }
                    pending.clear();
                }
                _ => i += 1,
            }
        }

        let code_len = match code_len {
            // No codespacerange at all: the common Identity-H composite layout.
            None => 2,
            // Simple (1-byte) and composite (2-byte) codes both decode.
            Some(len @ (1 | 2)) => len,
            // Wider than 2 bytes: refuse rather than guess a width.
            Some(_) => 0,
        };
        ToUnicode { code_len, map }
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    None,
    CodeSpace,
    BfChar,
    BfRange,
}

fn to_u16(bytes: &[u8]) -> Option<u16> {
    match bytes {
        [b] => Some(u16::from(*b)),
        [hi, lo] => Some(u16::from_be_bytes([*hi, *lo])),
        _ => None,
    }
}

/// Decode UTF-16BE bytes; an odd byte count or an invalid surrogate pair gives `None`
/// so the map entry is dropped and the code surfaces as undecodable downstream.
fn utf16be_to_string(bytes: &[u8]) -> Option<String> {
    let units = decode_units(bytes)?;
    Some(
        char::decode_utf16(units)
            .map(|r| r.unwrap_or(char::REPLACEMENT_CHARACTER))
            .collect(),
    )
}

fn decode_units(bytes: &[u8]) -> Option<Vec<u16>> {
    if !bytes.len().is_multiple_of(2) {
        return None;
    }
    // Explicit index stepping, not `chunks_exact(2)`: the pair read must stay
    // byte-identical (UTF-16BE) and stable under every clippy version we gate on.
    let mut units = Vec::with_capacity(bytes.len() / 2);
    let mut i = 0usize;
    while i + 1 < bytes.len() {
        units.push(u16::from_be_bytes([bytes[i], bytes[i + 1]]));
        i += 2;
    }
    Some(units)
}

/// `<lo> <hi> <dst>`: successive codes map to `dst`, `dst+1`, … (big-endian carry).
fn insert_scalar_range(map: &mut HashMap<u16, String>, parts: &[Vec<u8>]) {
    let (lo, hi) = match (to_u16(&parts[0]), to_u16(&parts[1])) {
        (Some(lo), Some(hi)) if hi >= lo => (lo, hi),
        _ => return,
    };
    let base = match decode_units(&parts[2]) {
        Some(units) if !units.is_empty() => units,
        _ => return,
    };
    for (offset, code) in (lo..=hi).enumerate() {
        let mut units = base.clone();
        add_offset(&mut units, offset);
        let text: String = char::decode_utf16(units)
            .map(|r| r.unwrap_or(char::REPLACEMENT_CHARACTER))
            .collect();
        map.insert(code, text);
    }
}

/// `<lo> <hi> [<d0> <d1> …]`: each array entry maps to one code.
fn insert_array_range(
    map: &mut HashMap<u16, String>,
    lo_bytes: &[u8],
    hi_bytes: &[u8],
    items: &[Vec<u8>],
) {
    let (lo, hi) = match (to_u16(lo_bytes), to_u16(hi_bytes)) {
        (Some(lo), Some(hi)) if hi >= lo => (lo, hi),
        _ => return,
    };
    for (offset, item) in items.iter().enumerate() {
        let offset = match u16::try_from(offset) {
            Ok(offset) => offset,
            Err(_) => break,
        };
        let code = match lo.checked_add(offset) {
            Some(code) if code <= hi => code,
            _ => break,
        };
        if let Some(text) = utf16be_to_string(item) {
            map.insert(code, text);
        }
    }
}

/// Add `delta` to the last UTF-16 unit, carrying leftwards.
fn add_offset(units: &mut [u16], delta: usize) {
    let mut carry = delta;
    for slot in units.iter_mut().rev() {
        let sum = u32::from(*slot) + carry as u32;
        *slot = sum as u16;
        carry = (sum >> 16) as usize;
        if carry == 0 {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Odd-length UTF-16 payloads are refused outright, never half-read: the pair
    /// walk must stop at the last complete pair (the old `chunks_exact(2)` policy).
    #[test]
    fn odd_length_destination_is_refused_not_half_read() {
        assert_eq!(decode_units(&[0x00]), None, "one byte is not a UTF-16 unit");
        assert_eq!(decode_units(&[]), Some(Vec::new()), "zero pairs is fine");
        assert_eq!(decode_units(&[0x00, 0x41]), Some(vec![0x0041]));
        assert_eq!(
            decode_units(&[0x00, 0x41, 0x00]),
            None,
            "the trailing byte must not become a code unit"
        );

        // Behavioural form: the entry is dropped, so the code stays undecodable
        // downstream and surfaces as an explicit unsupported reason.
        let cmap = "\
1 begincodespacerange
<00> <FF>
endcodespacerange
1 beginbfchar
<41> <00414>
endbfchar";
        let parsed = ToUnicode::parse(cmap.as_bytes());
        assert!(
            parsed.is_empty(),
            "odd-length destination must be dropped, not mis-decoded"
        );
    }

    /// Even-length multi-unit destinations decode exactly, byte for byte.
    #[test]
    fn even_length_destination_decodes_pair_by_pair() {
        assert_eq!(
            decode_units(&[0x06, 0x44, 0x06, 0x27]),
            Some(vec![0x0644, 0x0627])
        );
    }
}
