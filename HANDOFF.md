# HANDOFF — pdfrtl

**Updated:** 2026-10-04
**Repository:** `https://github.com/amirrezaalavi/bdf` (public mirror; publish only through the repository's privacy-gated publisher)
**Development root:** WSL Ubuntu-26.04, `/home/netcon/playground/ai/bdf`
**Branch:** `feat/baseline-reconcile`
**Current release:** `v0.1.0` (Linux x86_64 and arm64); follow-up release not started.
**Plan of record:** `.hermes/plans/2026-10-04_pdfrtl-next-roadmap.md`

## Current checkpoint

P0 baseline reconciliation is complete locally and published through the privacy-gated publisher. The publisher's public preflight passed; GitHub CI passed its five checks; I fetched `origin/main` and verified the published tree, sanitized manifest, current plan, handoff, and absence of private reports.

- `scripts/audit_languages.py` classifies positive script evidence; `tests/test_audit_languages.py` passed all 18 cases. Script evidence is not a language oracle.
- The owner-reviewed label corrections are applied to the local private manifest; remaining mixed/undetermined labels are untouched. The public publisher strips private manifest rows, so the private corrections are not exposed on GitHub.
- The owner said the displayed Persian RFP extraction excerpts seemed right. This is not a rendered-page side-by-side test or a construction-verified fixture.
- Public documentation omits private corpus measurements and identities. Raw audit reports remain owner-local outside the repository.
- Local checkpoint commit: `fd1b79b` on `feat/baseline-reconcile`. Public snapshot was independently read back from `origin/main`; do not push the private source branch directly.

## Immediate next actions

1. Start P1 from the roadmap: build public, construction-grounded RTL success/refusal controls before changing order-recovery behavior.
2. Run focused tests, then the full `bash scripts/wsl-build.sh` gate after each extraction change.
3. Publish future checkpoints only with `bash scripts/publish-public.sh`; verify CI and read back `origin/main` after each publish.
4. Before a follow-up release, verify/download/test the Linux x86_64 and arm64 binaries and checksums. Do not overwrite v0.1.0.

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
