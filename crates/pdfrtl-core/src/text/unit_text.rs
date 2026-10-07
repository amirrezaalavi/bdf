//! Compact inline string representation for text units.
//!
//! Stores strings up to 23 bytes inline without heap allocation.
//! Falls back to `Box<str>` for longer spans.

use std::borrow::Borrow;
use std::fmt;
use std::ops::Deref;

pub const INLINE_CAPACITY: usize = 23;

/// A space-efficient string type optimized for glyph units and short clusters.
/// Fits in 32 bytes on 64-bit platforms (24 bytes payload + 8 bytes discriminant/padding).
#[derive(Clone, Eq)]
pub enum UnitText {
    Inline { len: u8, buf: [u8; INLINE_CAPACITY] },
    Heap(Box<str>),
}

impl UnitText {
    /// Create a new `UnitText` from any string slice.
    #[inline]
    pub fn new(s: &str) -> Self {
        let bytes = s.as_bytes();
        if bytes.len() <= INLINE_CAPACITY {
            let mut buf = [0u8; INLINE_CAPACITY];
            buf[..bytes.len()].copy_from_slice(bytes);
            UnitText::Inline {
                len: bytes.len() as u8,
                buf,
            }
        } else {
            UnitText::Heap(s.into())
        }
    }

    /// Access the underlying string slice.
    #[inline]
    pub fn as_str(&self) -> &str {
        match self {
            UnitText::Inline { len, buf } => {
                // Safety: buf was constructed from valid UTF-8 of length `*len`
                unsafe { std::str::from_utf8_unchecked(&buf[..*len as usize]) }
            }
            UnitText::Heap(s) => s.as_ref(),
        }
    }

    /// True if the string is stored inline without heap allocation.
    #[inline]
    pub fn is_inline(&self) -> bool {
        matches!(self, UnitText::Inline { .. })
    }
}

impl Deref for UnitText {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl AsRef<str> for UnitText {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<[u8]> for UnitText {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.as_str().as_bytes()
    }
}

impl Borrow<str> for UnitText {
    #[inline]
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for UnitText {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Debug for UnitText {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl From<&str> for UnitText {
    #[inline]
    fn from(s: &str) -> Self {
        UnitText::new(s)
    }
}

impl From<String> for UnitText {
    #[inline]
    fn from(s: String) -> Self {
        if s.len() <= INLINE_CAPACITY {
            UnitText::new(&s)
        } else {
            UnitText::Heap(s.into_boxed_str())
        }
    }
}

impl From<&String> for UnitText {
    #[inline]
    fn from(s: &String) -> Self {
        UnitText::new(s.as_str())
    }
}

impl From<char> for UnitText {
    #[inline]
    fn from(c: char) -> Self {
        let mut buf = [0u8; INLINE_CAPACITY];
        let s = c.encode_utf8(&mut buf);
        let len = s.len() as u8;
        UnitText::Inline { len, buf }
    }
}

impl PartialEq for UnitText {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl PartialEq<str> for UnitText {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for UnitText {
    #[inline]
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<String> for UnitText {
    #[inline]
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other.as_str()
    }
}

impl std::hash::Hash for UnitText {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_strings_are_inline() {
        let text = UnitText::new("سلام");
        assert!(text.is_inline());
        assert_eq!(text.as_str(), "سلام");
        assert_eq!(text, "سلام");
        assert_eq!(text.len(), 8);
    }

    #[test]
    fn single_char_is_inline() {
        let text = UnitText::from('ی');
        assert!(text.is_inline());
        assert_eq!(text, "ی");
    }

    #[test]
    fn long_strings_use_heap() {
        let long_str = "این یک رشته بسیار طولانی برای تست حافظه هیپ است";
        let text = UnitText::new(long_str);
        assert!(!text.is_inline());
        assert_eq!(text.as_str(), long_str);
        assert_eq!(text, long_str);
    }

    #[test]
    fn exact_capacity_boundary() {
        let exact_23 = "12345678901234567890123";
        assert_eq!(exact_23.len(), 23);
        let text = UnitText::new(exact_23);
        assert!(text.is_inline());
        assert_eq!(text, exact_23);

        let over_24 = "123456789012345678901234";
        assert_eq!(over_24.len(), 24);
        let text2 = UnitText::new(over_24);
        assert!(!text2.is_inline());
        assert_eq!(text2, over_24);
    }
}
