//! Search/index-side text normalisation for Arabic, Persian and Hebrew.
//!
//! # Invariant: this never touches extracted text
//!
//! pdfrtl returns extracted text **verbatim** — ZWNJ stays, lam-alef ligatures
//! stay, harakat stay, digits keep the producer's codepoints (see
//! `AGENTS.md`, "The one invariant", and `docs/decisions/0002`). Every function
//! in this module exists for exactly one purpose: building a *comparison key*
//! so that a query can find text a human reads as identical but the PDF encodes
//! differently. Normalise the indexed text and the query with the same rules,
//! then compare — and never write the result back to a user-visible string.
//!
//! The naming is part of the contract: [`normalize_for_search`],
//! [`normalize_digits`]. There is deliberately no bare `normalize()` here,
//! because a bare name reads like a general-purpose text fixer, and applying
//! one to extraction output would silently corrupt it.
//!
//! Rule table, rationale and known limits: `docs/SEARCH-NORMALIZATION.md`.
//!
//! ```
//! use pdfrtl_core::search::{normalize_for_search, normalize_digits};
//!
//! // Both spellings — with and without the zero-width non-joiner — produce the
//! // same key, so a query typed either way matches:
//! assert_eq!(
//!     normalize_for_search("\u{0645}\u{06CC}\u{200C}\u{0631}\u{0648}\u{0645}"),
//!     normalize_for_search("\u{0645}\u{06CC}\u{0631}\u{0648}\u{0645}"),
//! );
//! ```

pub mod normalize;

pub use normalize::{normalize_digits, normalize_for_search, NormalizationStep};
