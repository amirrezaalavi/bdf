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
    /// Producer is allow-listed as storing visual order (recorded per producer, never guessed).
    ProducerVisualOrderKnown,
    /// No reliable recovery path exists — refuse rather than emit reversed text.
    UnsupportedNoEvidence,
    /// `ToUnicode` is missing or maps to C0 control characters.
    UnsupportedBrokenToUnicode,
}

impl Reason {
    /// True when this reason means "we could not produce logical order".
    pub fn is_unsupported(self) -> bool {
        matches!(
            self,
            Reason::UnsupportedNoEvidence | Reason::UnsupportedBrokenToUnicode
        )
    }

    /// Stable machine identifier; identical to the serialized form.
    pub fn as_str(self) -> &'static str {
        match self {
            Reason::ActualText => "actual_text",
            Reason::ToUnicodeLogical => "to_unicode_logical",
            Reason::BidiReordered => "bidi_reordered",
            Reason::ProducerVisualOrderKnown => "producer_visual_order_known",
            Reason::UnsupportedNoEvidence => "unsupported_no_evidence",
            Reason::UnsupportedBrokenToUnicode => "unsupported_broken_to_unicode",
        }
    }
}
