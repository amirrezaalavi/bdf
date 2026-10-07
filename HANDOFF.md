# HANDOFF — pdfrtl

**Updated:** 2026-10-07
**Repository:** `https://github.com/amirrezaalavi/bdf` (feature branch `feat/phase0-phase1-core-robustness` merged into `main`)
**Development root:** `/Users/amiralavi/playground/testing/bdf`
**Branch:** `main`
**Current release:** `v0.1.0` (Linux x86_64 and arm64); Phase 0 & Phase 1 merged into `main`.
**Plan of record:** Master Development Plan (Phases 0 through 5)

## Current checkpoint

**Phase 0 (Environment Baseline & Real-World Corpus Triage) & Phase 1 (Extractor Robustness & Decoupling) are COMPLETE and merged into `main`:**

- **Phase 0:**
  - Pinned Rust 1.98.1 installed and verified on host.
  - Automated triage tooling (`scripts/triage_pdf_sample.py`, `scripts/triage-pdf-sample.sh`) and baseline report (`reports/baseline-pdf-sample.md`) cataloging all 10 real-world Persian sample documents in `/Users/amiralavi/playground/testing/pdf_sample`.
  - Integration test `crates/pdfrtl-cli/tests/sample_corpus_triage.rs` enforcing the zero-panic and CLI exit contract across all 10 sample files.
- **Phase 1:**
  - Spatial baseline $\epsilon$-clustering: Replaced float truncation with adaptive `BASELINE_TOLERANCE = 1.0` pt, correctly assembling lines across mathematical formulas (e.g. `final.pdf`) and mixed scripts.
  - Excluded zero-advance diacritics and ZWNJ (`\u{200c}`) from spurious tie ambiguity refusals in `text::bidi`.
  - Resolved sequential RTL coordinate progressions as logical order (`LineOrder::Keep`), fixing reversed runs on dates and numbers.
  - Modularized `crates/pdfrtl-core/src/text/recover.rs` (previously 2,270 lines) into focused, clean modules:
    - `crates/pdfrtl-core/src/text/cluster.rs` (script categorization & cluster inversion)
    - `crates/pdfrtl-core/src/text/state.rs` (graphics state, CTM transforms, font decoders)
    - `crates/pdfrtl-core/src/text/bidi.rs` (UAX #9 bidi settlements & line order tracing)
    - `crates/pdfrtl-core/src/text/recover.rs` (orchestrator streamlined to ~450 lines)
  - All 66 tests passing in `cargo test --workspace`, 0 clippy warnings (`-D warnings`), and `cargo fmt` clean.
  - Feature branch `feat/phase0-phase1-core-robustness` pushed to remote and merged into `main`.

## Immediate next actions (Phase 2: High-Performance Engine)

1. **Zero-Copy Stream Lexer:** Refactor `tokenizer.rs` so `Token<'a>` borrows byte slices (`&'a [u8]`) without allocating `Vec<u8>` for every name, string, and operator; parse text operators into typed opcode enum `Op`.
2. **Compact Inline Units:** Replace heap-allocated `Unit { text: String }` with small inline strings (`SmallString<[u8; 16]>` / `CompactString`) to eliminate allocation overhead for 95%+ of units.
3. **Eliminate Global Mutexes:** Replace `static ORDER_TRACE` and `static OUTCOME_COUNTS` with structured logging/metrics.
4. **Parallel & Streaming Page Iterator:** Expose `doc.extract_pages_iter()` and optional multi-threaded page extraction via Rayon.

## Remaining checkpoints

- **P1 — Public correctness controls:** logical and visual-order fixtures, mixed Persian/Latin/digits, a genuine tied-geometry refusal, deliberate reversal detection, and explicit precondition assertions.
- **P2 — Safe refusal reduction:** categorize current Persian `unsupported_visual_order` cases; test candidate generation against the exhaustive short-line oracle; do not weaken proven-only output.
- **P3 — Contract/docs:** keep `data.text` proven-only, `--include-unproven` opt-in, and `bidi_consistent` described as consistency rather than proof. Keep docs and tests synchronized.
- **P4 — Follow-up release:** after P1/P2 evidence is green, run full and public gates, verify public CI/read-back, and download/test x86_64 + arm64 release artifacts and checksums. Never overwrite v0.1.0.
- OCR, writer/editing, GUI, MCP productization, and broader script support remain deferred.

## Invariants and privacy

- Correct logical order or an explicit reason; no silent reversal.
- `data.text` contains proven text only. Unproven lines are separately reported and their text is opt-in.
- `albdf` is a differential comparator, not an oracle.
- Keep private PDFs, identifiers, titles, hashes, raw audit rows, and per-document text out of public commits. `reports/` is excluded by the publisher.
- Use Rust/Cargo 1.98.1 from the pinned toolchain. In WSL set `export CARGO_TARGET_DIR="$HOME/target-bdf"`; the Windows checkout is a read-only mirror.

## Cloud/SRE self-critique

| Severity | Risk | Mitigation |
|---|---|---|
| Critical | Wrong RTL order silently corrupts caller data. | Proven-only output and non-vacuous refusal tests are release gates. |
| High | Private corpus and one Persian-reading owner are single points of validation. | Add redistributable fixtures and seek a second reviewer for stronger real-world claims. |
| High | Direct source-branch pushes can leak local metadata. | Publish only through the sanitizer/deny-list/preflight script and read back remote state. |
| Medium | Mixed/undetermined labels may still distort per-language metrics. | Preserve unresolved labels; make no further corrections without positive evidence and human review. |
| Medium | Architecture artifacts can drift from source/tests. | Verify downloaded binaries, SHA256, architecture, and smoke behavior before release. |
| Low | Broad candidate search raises complexity and regression risk. | Extend hypotheses incrementally, bounded by the short-line oracle. |
