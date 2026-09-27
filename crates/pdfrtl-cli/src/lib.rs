//! pdfrtl CLI library surface.
//!
//! The binary is thin; the *contract* lives here as public API so integration tests can
//! assert it and so the MCP adapter (P2) can reuse the same constants instead of
//! hard-coding numbers.
pub mod exit;
