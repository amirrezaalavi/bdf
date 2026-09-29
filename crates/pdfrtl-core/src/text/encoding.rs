//! Simple 8-bit font encodings: decode byte codes through `/Encoding` when a font
//! carries no `/ToUnicode` CMap (ADR 0004).
//!
//! Why this exists: ordinary Word / LibreOffice / LaTeX PDFs write Latin text with a
//! `/Type1` or `/TrueType` font and a predefined 8-bit encoding. For those fonts the
//! encoding **is** the mapping — no `/ToUnicode` is generated, and refusing the file
//! made the extractor useless on real documents.
//!
//! What decodes and what does not:
//!
//! | font state | outcome |
//! |---|---|
//! | `/ToUnicode` present | CMap wins (unchanged behaviour) |
//! | `/Encoding` name: `WinAnsiEncoding`, `MacRomanEncoding`, `StandardEncoding`, `PDFDocEncoding` | byte codes decode via `tables.rs` |
//! | `/Encoding` dict: `/BaseEncoding` + `/Differences` | base table, then glyph-name overrides via the Adobe Glyph List |
//! | `/Symbol`, `/ZapfDingbats`, `/MacExpertEncoding`, unknown name, no `/Encoding` at all | **refused** ([`Font::Refused`]) — those byte maps are not Unicode, and inventing characters is forbidden (ADR 0002) |
//!
//! The tables are generated (`tools/gen-encoding-tables.py`) from sources that are
//! cross-checked against each other; provenance and the cross-check results are in
//! the header of `tables.rs`.
use super::cmap::ToUnicode;
use super::tables::{AGL, MACROMAN, PDFDOC, STANDARD, UNDEF, WINANSI};
use std::collections::HashMap;

/// One page's font, as far as text decoding is concerned.
#[derive(Debug, Clone)]
pub enum Font {
    /// `/ToUnicode` CMap — the producer told us the mapping.
    ToUnicode(ToUnicode),
    /// Simple 8-bit font decoded through its `/Encoding`.
    Simple(SimpleEncoding),
    /// The encoding is not Unicode-mappable and there is no `/ToUnicode`:
    /// `/Symbol`, `/ZapfDingbats`, `/MacExpertEncoding`, an unknown name, or no
    /// `/Encoding` at all. Using it would invent text.
    Refused,
}

impl From<ToUnicode> for Font {
    fn from(cmap: ToUnicode) -> Font {
        Font::ToUnicode(cmap)
    }
}

/// A predefined8-bit base encoding named by `/Encoding` or `/BaseEncoding`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaseEncoding {
    WinAnsi,
    MacRoman,
    Standard,
    PdfDoc,
}

impl BaseEncoding {
    /// A predefined encoding name we can map to Unicode. `None` for everything
    /// else — `/MacExpertEncoding`, `/SymbolEncoding`, `/ZapfDingbatsEncoding`,
    /// `/Identity-H` on a simple font, or a name we have never seen.
    pub fn by_name(name: &[u8]) -> Option<BaseEncoding> {
        match name {
            b"WinAnsiEncoding" => Some(BaseEncoding::WinAnsi),
            b"MacRomanEncoding" => Some(BaseEncoding::MacRoman),
            b"StandardEncoding" => Some(BaseEncoding::Standard),
            b"PDFDocEncoding" => Some(BaseEncoding::PdfDoc),
            _ => None,
        }
    }

    fn table(self) -> &'static [u16; 256] {
        match self {
            BaseEncoding::WinAnsi => &WINANSI,
            BaseEncoding::MacRoman => &MACROMAN,
            BaseEncoding::Standard => &STANDARD,
            BaseEncoding::PdfDoc => &PDFDOC,
        }
    }
}

/// A resolved simple-font encoding: the base table plus `/Differences` overrides.
#[derive(Debug, Clone)]
pub struct SimpleEncoding {
    base: &'static [u16; 256],
    /// `None` value = a `/Differences` glyph name with no Unicode mapping we can
    /// justify: decoding that code is an explicit refusal, never a guess.
    overrides: HashMap<u8, Option<String>>,
}

impl SimpleEncoding {
    /// The predefined base encoding alone (no `/Differences`).
    pub fn new(base: BaseEncoding) -> SimpleEncoding {
        SimpleEncoding {
            base: base.table(),
            overrides: HashMap::new(),
        }
    }

    /// Apply `/Differences` entries as `(code, glyph name)` pairs. Later entries
    /// win, exactly as the producer wrote them: the base encoding supplies every
    /// code first and these override, so `Aacute`-style names line up with the
    /// *base* table rather than being re-derived from the glyph list.
    pub fn apply_differences<I: IntoIterator<Item = (u8, String)>>(&mut self, entries: I) {
        for (code, glyph) in entries {
            self.overrides.insert(code, glyph_to_unicode(&glyph));
        }
    }

    /// Unicode text for one byte code, or `None` when this encoding defines no
    /// Unicode for it (undefined slot, control character, or an unresolvable
    /// glyph name) — the caller then reports an explicit unsupported reason.
    pub fn decode(&self, code: u8) -> Option<String> {
        if let Some(entry) = self.overrides.get(&code) {
            return entry.clone();
        }
        match self.base[usize::from(code)] {
            UNDEF => None,
            scalar => char::from_u32(u32::from(scalar)).map(|c| c.to_string()),
        }
    }
}

/// Glyph name → Unicode text, following the Adobe Glyph List rules:
///
/// 1. the name is in the AGL (`Aacute`, `quotesingle`, `behfinalarabic`, …);
/// 2. `uniXXXX` form — four hex digits per UTF-16 unit, repeated;
/// 3. `uXXXX`/`uXXXXX`/`uXXXXXX` form — one code point;
/// 4. a variant suffix is dropped and the head re-tried (`hyphen.case` → `hyphen`);
/// 5. finally, a bare two-hex-digit name (`4E`) is read as the code point `U+00XX`
///    — a producer convention for "glyph number = character code". Checked last so
///    real AGL names such as `AE` are never shadowed, and restricted to uppercase
///    digits so ligature names like `ff` cannot be misread.
///
/// `None` means "no mapping we can justify" — the code surfaces as an explicit
/// unsupported reason instead of a plausible-looking wrong character.
pub fn glyph_to_unicode(name: &str) -> Option<String> {
    let name = name.strip_prefix('/').unwrap_or(name);
    if name.is_empty() {
        return None;
    }
    if let Some(text) = agl_lookup(name) {
        return Some(text.to_string());
    }
    if let Some(text) = uni_form(name) {
        return Some(text);
    }
    if let Some(text) = u_form(name) {
        return Some(text);
    }
    if let Some(head) = name.split_once('.').map(|(head, _)| head) {
        if let Some(text) = agl_lookup(head) {
            return Some(text.to_string());
        }
    }
    two_digit_form(name)
}

/// Exact Adobe Glyph List lookup (the table is sorted by name).
fn agl_lookup(name: &str) -> Option<&'static str> {
    AGL.binary_search_by(|entry| entry.0.cmp(name))
        .ok()
        .map(|index| AGL[index].1)
}

/// `uniXXXX[XXXX…]` → the concatenation of those UTF-16 code units.
fn uni_form(name: &str) -> Option<String> {
    let hex = name.strip_prefix("uni")?;
    if hex.is_empty() || !hex.len().is_multiple_of(4) {
        return None;
    }
    if !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let mut units = Vec::with_capacity(hex.len() / 4);
    for at in (0..hex.len()).step_by(4) {
        // Cannot fail after the checks above, but the library refuses (None) rather than panics.
        let Ok(unit) = u16::from_str_radix(&hex[at..at + 4], 16) else {
            return None;
        };
        units.push(unit);
    }
    utf16_to_string(&units)
}

/// `uXXXX[XX]` → one code point (4–6 hex digits).
fn u_form(name: &str) -> Option<String> {
    let hex = name.strip_prefix('u')?;
    if !(4..=6).contains(&hex.len()) {
        return None;
    }
    if !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let code_point = u32::from_str_radix(hex, 16).ok()?;
    char::from_u32(code_point).map(|c| c.to_string())
}

/// Bare `XX` (uppercase hex digits) → `U+00XX`.
fn two_digit_form(name: &str) -> Option<String> {
    if name.len() != 2 {
        return None;
    }
    if !name
        .bytes()
        .all(|byte| byte.is_ascii_digit() || (b'A'..=b'F').contains(&byte))
    {
        return None;
    }
    let code_point = u32::from_str_radix(name, 16).ok()?;
    char::from_u32(code_point).map(|c| c.to_string())
}

/// UTF-16 code units → text; an unpaired surrogate is refused, not replaced.
fn utf16_to_string(units: &[u16]) -> Option<String> {
    let mut text = String::with_capacity(units.len());
    let mut index = 0usize;
    while index < units.len() {
        let unit = units[index];
        let code_point = if (0xD800..=0xDBFF).contains(&unit) {
            let low = *units.get(index + 1)?;
            if !(0xDC00..=0xDFFF).contains(&low) {
                return None;
            }
            index += 1;
            u32::from(unit - 0xD800) * 1024 + u32::from(low - 0xDC00) + 0x10000
        } else if (0xDC00..=0xDFFF).contains(&unit) {
            return None;
        } else {
            u32::from(unit)
        };
        text.push(char::from_u32(code_point)?);
        index += 1;
    }
    Some(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Latin-1, Windows punctuation and the undefined slots of WinAnsi.
    #[test]
    fn winansi_decodes_latin_and_refuses_undefined_codes() {
        let enc = SimpleEncoding::new(BaseEncoding::WinAnsi);
        assert_eq!(enc.decode(0x41).as_deref(), Some("A"));
        assert_eq!(enc.decode(0x20).as_deref(), Some(" "));
        assert_eq!(enc.decode(0xA9).as_deref(), Some("©"));
        assert_eq!(enc.decode(0x80).as_deref(), Some("€"));
        assert_eq!(enc.decode(0x92).as_deref(), Some("\u{2019}"));
        assert_eq!(
            enc.decode(0xA0).as_deref(),
            Some("\u{A0}"),
            "0xA0 is nbspace"
        );
        assert_eq!(enc.decode(0x81), None, "ISO-undefined slot has no Unicode");
        assert_eq!(enc.decode(0x7F), None, "DEL is not text");
        assert_eq!(enc.decode(0x09), None, "C0 control is not text");
    }

    /// Base encoding and `/Differences` must line up: the base table decides the
    /// code, the glyph name only overrides it.
    #[test]
    fn differences_override_the_base_table_without_rederiving_it() {
        let mut enc = SimpleEncoding::new(BaseEncoding::WinAnsi);
        assert_eq!(
            enc.decode(0x27).as_deref(),
            Some("'"),
            "WinAnsi quotesingle"
        );
        enc.apply_differences([(0x27, "quoteright".to_string())]);
        assert_eq!(
            enc.decode(0x27).as_deref(),
            Some("\u{2019}"),
            "the Differences entry wins"
        );
        assert_eq!(
            enc.decode(0x41).as_deref(),
            Some("A"),
            "untouched codes stay"
        );

        // …and an unresolvable name makes that one code explicitly undecodable.
        enc.apply_differences([(0x41, "lamalefhamzaabovefinalarabicX".to_string())]);
        assert_eq!(
            enc.decode(0x41),
            None,
            "no AGL entry, no invented character"
        );
        assert_eq!(
            enc.decode(0x27).as_deref(),
            Some("\u{2019}"),
            "other codes survive"
        );
    }

    /// The glyph-list forms the archive actually uses.
    #[test]
    fn glyph_names_follow_the_adobe_rules() {
        assert_eq!(glyph_to_unicode("Aacute").as_deref(), Some("\u{C1}"));
        assert_eq!(glyph_to_unicode("Iacute").as_deref(), Some("\u{CD}"));
        assert_eq!(glyph_to_unicode("quotesingle").as_deref(), Some("'"));
        assert_eq!(glyph_to_unicode("space").as_deref(), Some(" "));
        assert_eq!(glyph_to_unicode("nbspace").as_deref(), Some("\u{A0}"));
        assert_eq!(glyph_to_unicode("uni05D0").as_deref(), Some("\u{5D0}"));
        assert_eq!(glyph_to_unicode("uni05B7").as_deref(), Some("\u{5B7}"));
        assert_eq!(glyph_to_unicode("uni00410042").as_deref(), Some("AB"));
        assert_eq!(glyph_to_unicode("u0041").as_deref(), Some("A"));
        assert_eq!(glyph_to_unicode("u1F600").as_deref(), Some("\u{1F600}"));
        assert_eq!(glyph_to_unicode("hyphen.case").as_deref(), Some("-"));
        assert_eq!(glyph_to_unicode("parenright.case").as_deref(), Some(")"));
        // Arabic presentation-form names, as written by the archive's Type1 producer.
        assert_eq!(
            glyph_to_unicode("behfinalarabic").as_deref(),
            Some("\u{FE90}")
        );
        assert_eq!(
            glyph_to_unicode("lamalefhamzaabovefinalarabic").as_deref(),
            Some("\u{FEF8}")
        );
        // The bare two-digit form, and the guard that keeps real names safe.
        assert_eq!(glyph_to_unicode("4E").as_deref(), Some("N"));
        assert_eq!(
            glyph_to_unicode("AE").as_deref(),
            Some("\u{C6}"),
            "AGL first"
        );
        assert_eq!(
            glyph_to_unicode("ff").as_deref(),
            Some("\u{FB00}"),
            "lowercase is a name"
        );
        // Nothing to justify → None.
        assert_eq!(glyph_to_unicode("lamalefhamzaabovefinalarabicX"), None);
        assert_eq!(glyph_to_unicode(""), None);
    }

    /// Predefined encodings we refuse by name — their maps are not Unicode.
    #[test]
    fn symbol_and_zapfdingbats_encodings_are_not_recognised() {
        assert_eq!(BaseEncoding::by_name(b"SymbolEncoding"), None);
        assert_eq!(BaseEncoding::by_name(b"ZapfDingbatsEncoding"), None);
        assert_eq!(BaseEncoding::by_name(b"MacExpertEncoding"), None);
        assert_eq!(BaseEncoding::by_name(b"Identity-H"), None);
        assert_eq!(BaseEncoding::by_name(b"NotARealEncoding"), None);
        assert_eq!(
            BaseEncoding::by_name(b"WinAnsiEncoding"),
            Some(BaseEncoding::WinAnsi)
        );
    }

    /// MacRoman must be Mac OS Roman, not a Latin-1 look-alike.
    #[test]
    fn macroman_keeps_its_own_positions() {
        let enc = SimpleEncoding::new(BaseEncoding::MacRoman);
        assert_eq!(enc.decode(0x41).as_deref(), Some("A"));
        assert_eq!(
            enc.decode(0xA5).as_deref(),
            Some("\u{2022}"),
            "bullet at 0245"
        );
        assert_eq!(
            enc.decode(0xCA).as_deref(),
            Some("\u{A0}"),
            "nbspace at 0312"
        );
        assert_eq!(
            enc.decode(0xDB).as_deref(),
            Some("\u{A4}"),
            "currency, per spec"
        );
        assert_eq!(enc.decode(0x7F), None);
    }
}
