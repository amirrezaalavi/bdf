//! Why a text run came out in the order it did — or why we refuse to guess.
//!
//! This enum IS the product invariant. Extraction never returns text whose order
//! we cannot justify: it returns logical order plus a `Reason`, or it fails with
//! an explicit `unsupported_*` reason. Blanket reversal of RTL runs is forbidden,
//! because storage order is producer-dependent — reversing a file that already
//! stores logical order corrupts it.
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    /// `/ActualText` marked content carried the logical text verbatim (authoritative).
    ActualText,
    /// `ToUnicode` mapped cleanly to base letters and run order was already logical.
    ToUnicodeLogical,
    /// `ToUnicode` mapped cleanly; run order was reconstructed from UAX #9 levels.
    BidiReordered,
    /// Rung 3 found the stored order **consistent with** the sequence the producer actually
    /// painted: UAX #9 was run over both readings and exactly one reproduced the measured
    /// painting — kept as logical, or inverted when the comparison proved the mirror.
    ///
    /// **This is consistency, not proof of the original order.** Measured 2026-10-03
    /// (`docs/problems/0015`): two different logical strings with distinct characters and a
    /// known paragraph direction paint to the SAME visual sequence — `אבa 12` and `אב12 a`
    /// both paint as `a 12בא`, reproduced with `unicode-bidi` 0.3, the crate used here. Geometry
    /// alone therefore cannot always recover which was stored; uniqueness, when claimed, is
    /// uniqueness **within the candidate set that was tried**.
    ///
    /// Named `bidi_consistent` rather than `bidi_verified` because "verified" asserted the
    /// stronger claim. Renamed 2026-10-03; the serialized identifier changed with it.
    BidiConsistent,
    /// Producer is allow-listed as storing visual order (recorded per producer, never guessed).
    ProducerVisualOrderKnown,
    /// No reliable recovery path exists — refuse rather than emit reversed text.
    UnsupportedNoEvidence,
    /// `ToUnicode` is missing or maps to C0 control characters.
    UnsupportedBrokenToUnicode,
    /// Codes were decoded through the font's own `/Encoding` (a predefined base
    /// encoding plus `/Differences` glyph names) — no `/ToUnicode` was needed.
    /// The archive's Word/LibreOffice/LaTeX Latin text decodes this way.
    EncodingMapped,
    /// The font's byte map is not Unicode-mappable and there is no `/ToUnicode`:
    /// `/Symbol`, `/ZapfDingbats`, `/MacExpertEncoding`, an unknown or missing
    /// `/Encoding`, or a `/Differences` glyph name we cannot resolve.
    UnsupportedFontEncoding,
    /// The page's content stream could not be read. This page contributes no text;
    /// every other page is unaffected (ADR 0004).
    UnsupportedPageContent,
    /// The page is right-to-left dominant and the producer is one that stores
    /// *visual* order: the characters come back, the order does not count as
    /// logical (ADR 0004).
    UnsupportedVisualOrder,
}

impl Reason {
    /// True when this reason means "we could not produce logical order".
    pub fn is_unsupported(self) -> bool {
        matches!(
            self,
            Reason::UnsupportedNoEvidence
                | Reason::UnsupportedBrokenToUnicode
                | Reason::UnsupportedFontEncoding
                | Reason::UnsupportedPageContent
                | Reason::UnsupportedVisualOrder
        )
    }

    /// Stable machine identifier; identical to the serialized form.
    pub fn as_str(self) -> &'static str {
        match self {
            Reason::ActualText => "actual_text",
            Reason::ToUnicodeLogical => "to_unicode_logical",
            Reason::BidiReordered => "bidi_reordered",
            Reason::BidiConsistent => "bidi_consistent",
            Reason::ProducerVisualOrderKnown => "producer_visual_order_known",
            Reason::UnsupportedNoEvidence => "unsupported_no_evidence",
            Reason::UnsupportedBrokenToUnicode => "unsupported_broken_to_unicode",
            Reason::EncodingMapped => "encoding_mapped",
            Reason::UnsupportedFontEncoding => "unsupported_font_encoding",
            Reason::UnsupportedPageContent => "unsupported_page_content",
            Reason::UnsupportedVisualOrder => "unsupported_visual_order",
        }
    }
}
