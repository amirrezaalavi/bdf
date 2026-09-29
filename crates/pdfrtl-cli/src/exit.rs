//! Process exit codes — part of the public contract (see `docs/CLI.md`).
//!
//! Callers are told to branch on these values rather than parse strings, so they are
//! pinned here, in a public module, with a test. `EXIT_UNSUPPORTED` is deliberately
//! present before its first use: it is the code that expresses the product invariant
//! ("we refuse to guess") and it is wired to a real path in P1.

/// The operation completed. Envelope has `ok: true`.
pub const EXIT_OK: u8 = 0;

/// Text order could not be established with justification: the envelope carries an
/// `unsupported_*` reason. Refusing is a correct outcome, not a failure.
pub const EXIT_UNSUPPORTED: u8 = 3;

/// io/parse failure: missing file, unreadable or corrupt PDF. Envelope has `ok: false`.
pub const EXIT_IO: u8 = 4;
