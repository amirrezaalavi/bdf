# HANDOFF — pdfrtl

**Updated:** 2026-10-07
**Repository:** `https://github.com/amirrezaalavi/bdf` (feature branch `feat/phase0-phase1-core-robustness` merged into `main`)
**Development root:** `/Users/amiralavi/playground/testing/bdf`
**Branch:** `main`
**Current release:** `v0.1.0` (Linux x86_64 and arm64); Phase 0, Phase 1 & Phase 2 merged into `main`.
**Plan of record:** Master Development Plan (Phases 0 through 5)

## Current checkpoint

**Phase 0 (Environment Baseline & Real-World Corpus Triage), Phase 1 (Extractor Robustness & Decoupling), and Phase 2 (High-Performance Engine) are COMPLETE and merged into `main`:**

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
- **Phase 2:**
  - **`UnitText` Compact Inline Small String:** Implemented 23-byte inline buffer (`[u8; 23]` + `len: u8`) in `crates/pdfrtl-core/src/text/unit_text.rs`, fitting in 32 bytes on 64-bit platforms. Eliminates heap allocations for >99% of text units, CIDs, and ligature clusters while seamlessly dereferencing to `&str`.
  - **Zero-Copy Stream Lexer:** Refactored `tokenizer.rs` so `Token<'a>` and `Value<'a>` borrow byte slices (`Cow<'a, [u8]>`), allocating zero bytes for names, numbers, operators, and unescaped literal strings.
  - **Typed Opcode Parser:** Implemented `TextOp` enum with fast integer dispatch in `Walker::op`.
  - **Bypassed Intermediate Allocations:** Walk loop streams tokens directly into `Walker::op` without allocating intermediate `Vec<Item>` or cloning values.
  - **Lock-Free Thread-Local Tracing:** Replaced global `static ORDER_TRACE` and `static OUTCOME_COUNTS` mutexes with `thread_local!` storage, enabling contention-free concurrent multi-threaded extraction.
  - **Streaming & Parallel Page Extraction:** Added `extract_page` and streaming `extract_pages_iter` iterator in `recover.rs` and `mod.rs`.
  - **Benchmarking & Validation:** Added `crates/pdfrtl-core/tests/phase2_performance.rs`. Extraction throughput measured across the 10 real-world sample PDFs showed a ~25% reduction in CPU user cycles (0.072s vs 0.094s).
  - All 78 tests passing in `cargo test --workspace`, 0 clippy warnings (`-D warnings`), and `cargo fmt` clean.
  - Feature branch `feat/phase0-phase1-core-robustness` merged into `main` and pushed to remote origin.

## Immediate next actions (Phase 3: The "Agent-Ready" Contract & MCP Tools)

1. **Spatial Bounding Boxes in Output:** Extend `PageText` with structured layout models: `TextBlock` $\rightarrow$ `TextLine` $\rightarrow$ `TextSpan`, exposing bounding boxes `[x0, y0, x1, y1]` in PDF coordinate space.
2. **CLI Output Flavors:** Add `pdfrtl extract --format blocks` (structured hierarchical JSON for agents) alongside standard `--json`.
3. **Productize the MCP Server (`crates/pdfrtl-mcp`):** Implement live MCP tools over stdio (`inspect`, `extract_text`, `extract_layout`, `search`).

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
