# pdfrtl — what's left

Status **2026-10-03**. Current execution order: `docs/plans/2026-09-29-lane-plan.md`.
Private-corpus measurements below are from 2026-09-29; the archive is owner-local, not in the public clone.

## Where we actually are

| area | state | evidence |
|---|---|---|
| Workspace, gates, CI | **working** | Fresh public clone builds; pinned Rust 1.98.1; CI 5/5 green on `9411645b` |
| Order invariant | **enforced** | ADR 0002 + 0004; unproven order is withheld with an explicit reason |
| Extraction | **implemented, incomplete** | CLI `pdfrtl extract --json`; mixed producer/order recovery still refuses cases it cannot prove |
| Private archive validation | **measured locally** | 38 fully decoded, 37 order-verified, 12 partial, 14 refused, 4 no-text; 1,038,880 emitted / 1,271,919 withheld. PDFs are not in the public repo. |
| Public clone fixtures | **available** | 14 redistributable PDF fixtures; sufficient for build, tests, and the public CI gate |
| Public mirror | **published** | `main` = `9411645b`; publisher reset-on-divergence fix tested and remotely verified; history-reset deferral remains ADR 0005 |
| Search normalization | **verify before relying on it** | A branch existed; check branch/CI/merge state rather than treat old status notes as current |
| Generation, editing, MCP, enterprise | **not implemented** | Future scope; writer stack chosen, editing spike established incremental-only saves |
| OCR | **deferred** | Owner decision: reading/search correctness first |

## The three facts that shape everything left

1. **The corpus is two producer families, and a fix for one is not a fix for the other.**
   Chrome marks an RTL run with `/ReversedChars` and annotates clusters with `/ActualText`.
   Your real Persian/Arabic/Hebrew archive does neither — those files carry only `ToUnicode`,
   and **4 of the 51 have no text layer**. The other counts and the archive are owner-local; a fresh public clone cannot repeat this measurement without access to the customer files.
2. **No third-party tool can be the reference for RTL — measured, not assumed.** On the same
   five fixtures, against the authority strings: ours is byte-exact, poppler flips every
   lam-alef pair (`سلام` → `سالم`, and that survives a casual read), Xpdf returns visual order
   and, on the mixed fa/en line, swaps the runs; PDFium drops ZWNJ and returns per-cluster
   visual order. Also: the `pdftotext` on this host's `PATH` is **Xpdf 4.00, not poppler** —
   earlier notes said poppler and were wrong. Full matrix and raw codepoints:
   `docs/problems/0004-no-oracle-is-byte-faithful-for-rtl.md`. Ground truth is a human or
   vision read of the rendered page, which is why the critical path is *review capacity*,
   not code volume.
3. **"No `ToUnicode`" is a real, common, recoverable case — and it is not an OCR case.**
   `hebrew-1.pdf` is an English volume quoting Hebrew. Its Hebrew sits in `Identity-H` fonts
   with **no `ToUnicode` and no `CIDToGIDMap`**, so the text layer yields garbage codepoints
   (Sinhala/Malayalam range). The answer is to parse the embedded font's own cmap and reverse
   GID → Unicode. That is W1.6, and it is what "font aware" has to mean in practice.

## Workstreams

| # | workstream | state | notes |
|---|---|---|---|
| W1 | **Extraction (P1)** | in progress | The core works; font decoding and order evidence are the remaining correctness gaps |
| W1.1 | `ToUnicode` CMap parser (1- and 2-byte codes) | **landed** | Simple-font and Type0 mappings are supported where the map is usable |
| W1.2 | Unit model + `/ReversedChars` inversion + run-order recovery | **partially landed** | `/ActualText`, measured geometry and fingerprint/refusal rungs exist; mixed-script edge cases remain |
| W1.3 | Reasons + exit-code-3 refusal path | **landed** | `Reason` vocabulary and CLI contract tests exist |
| W1.4 | `pdfrtl extract --json` CLI verb | **landed** | Exit contract documented in `docs/CLI.md` |
| W1.5 | Validate against 51-file archive | **measured locally** | Archive is owner-only; public clone cannot repeat these numbers |
| W1.6 | Font-aware recovery | **next after 1a** | First withhold broken font text per-font; then investigate embedded-font cmap inversion for `hebrew-1.pdf` |
| W2 | **Bidi, mixed script, correct reading (P2)** | **planned, after font work** | Faithful per-character prediction before any consistency relaxation |
| W2.4 | Search/index normalization | check current branch state | Search-side only; never normalize extracted output |
| W2.5 | Multi-column / table reading order | not started | needs a Reason, never a silent guess |
| W3 | **Verification lane** | **landed** | PDFium pixels + vision review; the oracle matrix is now recorded in `docs/problems/0004` |
| W3.1 | PDFium pixel goldens (`pypdfium2`), fixed scale, non-blank assertion | **landed** | `scripts/render_pages.py`. Deviation: the 99.5%-white rule alone rejects correct one-line A4 renders (they measure 99.6–99.9% white), so blank now requires white ≥99.5% **and** <256 ink pixels; verified in both directions (fixture exits 0, empty PDF exits 1) |
| W3.2 | Rendered PNGs for human/vision review of every RTL fixture | **landed** | 8 pages in `reports/render/*/page-1.png` + `reports/vision-review-2026-09-27.md` checklist. Vision-checked here: `fa-zwnj-lamalef` renders the expected sentence with the ZWNJ, em dash and Persian date in correct visual positions |
| W3.3 | Differential oracle report | **landed** | `reports/oracle-extraction.md` + the orchestrator's independent matrix in `docs/problems/0004`. Findings: Xpdf≠poppler on this host; lam-alef flipped by poppler; ZWNJ dropped by PDFium; no mode without `-enc UTF-8` returns any RTL |
| W3.4 | CI: pinned PDFium/qpdf oracles, corpus + goldens on every push | pending | poppler/mutool stay CI-only, never shipped |
| W4 | **Generation / writer (P3)** | not started | the biggest new-code chunk |
| W4.1 | Shaping + subsetting: Type0/Identity-H, `ToUnicode`, 65536-entry `CIDToGIDMap`, cluster → `/ActualText` | not started | `CIDToGIDMap` is the al-bdf "Latin glyphs instead of Persian" trap — already documented in the knowledge DB |
| W4.2 | Layout API that *cannot* split a shaping run across styles | not started | design constraint from the pipeline skill; retrofitting it later is expensive |
| W4.3 | Round-trip proof: generate → our extractor → PDFium pixels → human eyes | not started | |
| W5 | **Editing (P4)** | blocked on a spike | |
| W5.1 | Spike: does `lopdf` carry `BDC`/`EMC` through load→save? | **done** (`7086526`) | **decides patch vs rewrite**: incremental-only — `IncrementalDocument::create_from` + save preserves untouched bytes (27282→28209 B); a plain `Document::save` re-serialised 24/25 objects and degraded `Object::Real(f32)`; never mutate `Stream::content` with a stale `/Length` |
| W5.2 | Text replacement preserving `/ActualText` spans | not started | |
| W5.3 | Redaction, asserted via pixels (never extract-after-redact) | not started | |
| W6 | **Enterprise features** | not started | |
| W6.1 | Signatures: create/verify, RTL signature appearance | not started | |
| W6.2 | Forms/annotations with RTL `/AP` streams (no GUI painter — base-14 fonts cannot shape Arabic) | not started | |
| W6.3 | Encryption/permissions | not started | none of the 51 archive files are encrypted; need a fixture |
| W7 | **Scans / OCR** | **deferred (your call, 2026-09-27)** | 19 archive files have no text layer. Out of scope for now: correct reading and search come first |
| W8 | **Packaging & product** | not started | |
| W8.1 | MCP server over the core (the paid-tier surface) | not started | transport decision (Rust SDK vs TS over CLI) still open |
| W8.2 | Docker image + single static binary per platform (linux x64/arm64, macOS, Windows) | partial | `Dockerfile` exists and is wired to the corpus harness |
| W8.3 | Release automation, MSRV, `cargo vet`, reproducible builds | not started | toolchain now pinned to **1.98.1** on both sides (local + CI) after a gate-drift incident cost 3 red CI runs (`docs/problems/0003`); `rust-version = "1.97"` in Cargo.toml is an **untested** MSRV claim — needs an MSRV job or a bump |
| W8.4 | Final licence choice (AGPL+commercial vs permissive core) | deferred by you | must land before shipping an SDK, not before the POC |
| W9 | **Continuity** | ongoing | |
| W9.1 | Knowledge DB (SQLite+FTS5 + markdown export) | exists | keep feeding: every bug we hit goes in |
| W9.2 | `AGENTS.md` per crate, `HANDOFF.md` at milestones | exists | refresh after W1 lands |
| W9.3 | Publishing: `scripts/publish-public.sh` snapshots the tree to the public mirror (private material excluded by construction) | **exists** | code-writing subagents push their own branch instead; a bare `git add -A` is forbidden for them |

## Sequence

Phase 2 runs in this order, detailed in `docs/plans/2026-09-29-lane-plan.md`:

   **Status 2026-09-30:** order recovery now uses measured geometry where possible, then a
   producer fingerprint, otherwise it refuses. The mirrored-matrix measurement defect is fixed
   (`5ecdf1b`, `docs/problems/0010`). Archive validation remains 38 fully decoded / 37
   order-verified / 14 refused; the refused files' stored-order convention is **undetermined**.
   Both research instruments built to answer that question were retired as non-discriminating
   (`docs/problems/0011`). Do not use the old rung-3 alarm or old allow-list claims as current evidence.
2. **W1.6 font-aware recovery** (`hebrew-1.pdf`) — procedure in `docs/research/2026-09-29-external-answers.md`.
3. **W2 reading order** only after font work: faithful per-character prediction first, guarded consistency relaxation second.
4. **W5.1 editing spike** is already complete; incremental-save-only is the result (`docs/problems/0012` is publisher-only; see earlier editing spike commit `7086526`).
5. **Then** writer → editing → enterprise/MCP, only after extraction correctness is established.

A red gate never moves on; `main` only receives CI-passing snapshots.

## What only you can settle

| item | state |
|---|---|
| ~~`hebrew-1.pdf`: Hebrew by name, Arabic-script + Latin by content~~ | **answered**: an English volume quoting Hebrew; the Hebrew extracts as garbage because its fonts have no `ToUnicode`. Now the fixture for W1.6 (font-aware recovery) |
| ~~The 7 `unknown-N.pdf` files (no text layer, no language marker)~~ | **dropped by you** — their language label stays `undetermined` |
| ~~W7: is OCR in scope?~~ | **answered**: deferred; search and reading correctness come first |
| Human read of the 4 Persian fixtures' expected text | **still open** — promotes them from `unverified` to asserted |
| W8.4: final licence | **still open** — blocks shipping only |

## Effort (agent-time, with human review on the critical path)

| phase | agent-time | human review |
|---|---|---|
| W1 finish + W1.5 validation + W1.6 font-aware recovery | 3–5 days | ~1 hour (reading pages, spot-checking output) |
| W2 bidi/mixed/reading order (normalizer already in flight) | 1–2 weeks | hours |
| W4 writer | 2–3 weeks | hours per milestone |
| W5 editor (after spike) | 2–3 weeks | hours |
| W6 + W8 | 2 weeks | — |

| Severity | Risk | Mitigation |
|---|---|---|
| High | Private customer PDFs are a single-person dependency; a fresh public clone cannot reproduce archive measurements. | Disclose the limit; arrange access owner-to-owner; never commit the files or identifying metadata. |
| High | Incorrect RTL order can look plausible while being wrong. | Refuse without evidence; keep the per-font and order fixtures; render and visually review text changes. |
| Medium | Toolchain drift can make local success disagree with CI. | Pin Rust 1.98.1 identically in `rust-toolchain.toml` and CI. |
| Medium | A failed publish can leave a stale scratch clone. | Reset the scratch clone from remote before staging; verify published contents on remote. |
| Low | Writer/editor/MCP scope can grow before read correctness is proven. | Defer those features until extraction passes the planned acceptance checks. |