# pdfrtl — remaining roadmap after owner label review

**Updated:** 2026-10-04
**Branch:** `feat/baseline-reconcile`
**Current checkpoint:** P0 baseline reconciliation is complete locally and published through `scripts/publish-public.sh`; the sanitized public gate, CI, and remote read-back passed.
**Goal:** improve Persian extraction correctness without ever returning unproven order as trusted text, then release a tested Linux x86_64/arm64 follow-up.

## Verified state

- v0.1.0 is already released; this roadmap concerns follow-up work, not the first release.
- Positive script-evidence audit was added and its 18 decision tests passed. The audit itself is not a language oracle.
- The owner reviewed the disputed label groups; reviewed English-only records and the Persian RFP excerpts were relabelled locally. Remaining mixed/undetermined labels are unchanged.
- Private corpus identities and all private measurement output stay local; public documentation makes no private-corpus count claims.
- The owner said the displayed RFP extraction text seemed right. This is human feedback, not a rendered-page ground-truth fixture.
- Output contract remains: `data.text` is proven-only; withheld text is separate and opt-in via `--include-unproven`.
- `albdf` is a comparator, never an order oracle.

## Remaining checkpoints

### P1 — Public correctness controls (next)

**Objective:** prove important RTL success and refusal paths independently of the private corpus.

1. Audit the existing public fixtures and their assertions. Identify exact gaps: logical-order output, visual-order recovery, mixed Persian/Latin/digits, genuine geometric ties, and a refusal with withheld text.
2. Add/extend one fixture at a time with explicit provenance and constructed expected output. Assert preconditions (`ActualText`, `ReversedChars`, distinct positions/ties, and withheld counters) so tests cannot pass vacuously.
3. Add a second control that makes a deliberate whole-line reversal fail; keep word-shape probes as smoke tests only.
4. Run focused tests and the full `bash scripts/wsl-build.sh` gate. Keep private PDFs and expected text out of git.

**Gate:** the fixture suite catches false-success and whole-line reversal, and at least one public input demonstrably withholds text for the expected reason.

### P2 — Investigate the Persian refusal gap safely

**Objective:** reduce justified refusals only where evidence supports a unique answer.

1. Re-measure owner-local cases after P1 and group them by smallest failing shape, producer evidence and reason; use aggregate-only results in tracked docs.
2. TDD the smallest failing shape. Check candidate generation against the exhaustive short-line oracle before using it on real documents.
3. Keep proven candidates higher priority than generated hypotheses; generated candidates never override proven evidence. Preserve distinct ambiguity, no-solution and search-budget outcomes.
4. Re-run the full public suite and owner-local archive after every change; compare emitted/refused counts and reversal smoke checks. Never promote `albdf` output to ground truth.

**Gate:** no regression in `no_silent_reversal`; no false success on the oracle/fixture ladder; any emitted Persian output used as correctness evidence has human/page-grounded review.

### P3 — Keep product contract and docs synchronized

**Objective:** ensure claims match tested behavior.

1. Keep `README.md`, `CANARY.md`, `docs/ACCEPTANCE.md`, `docs/CLI.md`, `CHANGELOG.md`, `HANDOFF.md`, and `AGENTS.md` consistent with tests and the proven-only contract.
2. Retain the exact meaning of `bidi_consistent`: consistent with the painted output, not proof of logical order.
3. Do not publish private-corpus measurements. Never publish private file paths, names, document titles, hashes, audit rows, or extracted text.

**Gate:** docs describe the public evidence accurately, publish no private metrics, and pass the privacy scan.

### P4 — Follow-up release

**Objective:** publish a new release only after P1/P2 evidence is green.

1. Run `bash scripts/wsl-build.sh` and the public-staged preflight in `scripts/publish-public.sh`; do not bypass the preflight.
2. Use the repository publisher, which strips private manifest rows/reports, checks a deny-list, pushes a snapshot, waits for CI, and advances public `main` only on green CI.
3. Read back `origin/main`, inspect the published tree and CI, and verify the public clone gate.
4. Build/download Linux x86_64 and arm64 artifacts, verify `SHA256SUMS`, run architecture-appropriate smoke tests, and then tag a new version. Never overwrite v0.1.0.

**Gate:** clean local tree; green full and public-clone gates; privacy scan clean; remote contents/CI confirmed; both artifacts and checksums verified.

### Deferred scope

OCR, PDF generation/editing, GUI, MCP productization, broad script expansion, and enterprise signing/forms remain out of scope until extraction correctness and review coverage improve.

## Owner input

No immediate label decision blocks the next public-fixture work. The owner reviewed the disputed language groups; the owner-approved corrections are local. Before stronger real-world reading-order claims, request a side-by-side rendered-page review of representative outputs (or a second Persian-speaking reviewer) and record the exact adjudication; otherwise keep such claims explicitly unverified.

## Self-critique — cloud/SRE lens

| Severity | Risk | Mitigation |
|---|---|---|
| Critical | A false RTL order silently corrupts caller data. | Keep proven-only output and refusal fixtures as release gates; never trade correctness for extraction rate. |
| High | Private corpus is a single-owner acceptance dependency. | Expand redistributable synthetic controls; publish only sanitized aggregate metrics. |
| High | Human Persian review is a single point of failure. | Seek a second Persian reviewer for claims that exceed fixture construction; otherwise constrain claims. |
| Medium | Manifest errors distort per-language metrics. | Keep positive-evidence audit plus human adjudication; preserve mixed/unknown labels. |
| Medium | Public mirror can leak local metadata if pushed directly. | Publish only through `scripts/publish-public.sh`; require deny-list and public-tree CI. |
| Medium | Linux architecture artifacts can drift from tested source. | Verify downloaded assets, checksums, architectures, and smoke tests before release. |
| Low | Wider hypothesis search increases complexity and regressions. | Add candidates incrementally with a hard budget and exhaustive short-line oracle. |
