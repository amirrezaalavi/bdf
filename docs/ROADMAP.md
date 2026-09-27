# pdfrtl — what's left

Status 2026-09-27. Every claim here has an artifact behind it; the evidence is named.

## Where we actually are

| area | state | evidence |
|---|---|---|
| Workspace, gates, CI | **done** | `cargo fmt`/`clippy -D warnings`/tests/`cargo deny` (advisories, bans, licenses, sources all ok); `.github/workflows/ci.yml` |
| Public repo | **live** | `github.com/amirrezaalavi/bdf` — public, throwaway by your call. Private-corpus fixtures are **not** published: not their names, not their hashes, not their metadata |
| Licence posture (Option A) | **decided, deferred** | ADR 0001–0003; permissive-only shipped features gated by `cargo deny`; final choice deferred per your "POC first" |
| Corpus | **59 fixtures** | 8 own-work/generated + 51 real archive (local-only, hashes committed locally) |
| Real-archive profile | **measured** | 27 Type0 fonts, 21 `CIDToGIDMap`, 19 with no text layer, 2 with `/ActualText`, **0** with `/ReversedChars` |
| Producer coverage | **broad** | Chrome (generated) + InDesign ×4, Acrobat Pro, Word 2013, Nitro PrimoPDF, macOS Quartz (real, from your archive) |
| Extraction (P1) | **in flight** | `text/tokenizer.rs` (378 lines, total lexer), `text/cmap.rs` (235 lines, `ToUnicode` parser just landed) and `tests/text_recovery.rs` (226 lines) are real; `text/recover.rs` is still the red-phase stub |
| Search/index normalization (W2.4) | **in flight** | agent on branch `feat/search-normalizer` in its own clone of the public repo; delivers `search/normalize.rs` + tests + `docs/SEARCH-NORMALIZATION.md` |
| Oracles | **partial** | `pdftotext` proven to drop Persian on real Chrome output; `pypdf` for structure; PDFium/`pypdfium2` being wired now |
| Human verification | **3 open** | 4 Persian fixtures' expected text; `hebrew-1.pdf` reclassified (see fact 3) |

## The three facts that shape everything left

1. **The corpus is two producer families, and a fix for one is not a fix for the other.**
   Chrome marks an RTL run with `/ReversedChars` and annotates clusters with `/ActualText`.
   Your real Persian/Arabic/Hebrew archive does neither — those files carry only `ToUnicode`,
   and 19 of them have no text layer at all. Both paths must exist, and they fail differently.
2. **No third-party tool can be the reference for RTL.** poppler drops Arabic-script
   `/ActualText` payloads outright (measured on our own fixtures); PDFium and PDF.js return
   visual order. Ground truth for logical order is a human reading the page — which is why the
   vision/render lane exists and why the critical path is *review capacity*, not code volume.
3. **"No `ToUnicode`" is a real, common, recoverable case — and it is not an OCR case.**
   `hebrew-1.pdf` is an English volume quoting Hebrew. Its Hebrew sits in `Identity-H` fonts
   with **no `ToUnicode` and no `CIDToGIDMap`**, so the text layer yields garbage codepoints
   (Sinhala/Malayalam range). The answer is to parse the embedded font's own cmap and reverse
   GID → Unicode. That is W1.6, and it is what "font aware" has to mean in practice.

## Workstreams

| # | workstream | state | notes |
|---|---|---|---|
| W1 | **Extraction (P1)** | in progress | the POC's core |
| W1.1 | `ToUnicode` CMap parser (`codespacerange`, `bfchar`, `bfrange`, 1- and 2-byte codes) | **landed** | 1-byte codes matter for simple-font files (`hebrew-2.pdf` is Type1) |
| W1.2 | Unit model + `/ReversedChars` inversion + run order via UAX #9 | in progress | the hard part is the LTR run *inside* an RTL line (`۱۴۰۳/۰۵/۱۲` must end up last, digits unreversed) |
| W1.3 | Reasons + exit-code-3 refusal path wired end-to-end | pending | `Reason` vocabulary already exists; tests already assert it |
| W1.4 | `pdfrtl extract --json` CLI verb | pending | envelope shape fixed in the plan; `main.rs` already touched |
| W1.5 | Validate against the 51-file archive | not started | classify every file as *extracted with reason* or *refused with reason*; the Arabic docs (91/62/321/185 pages) and the 5 Hebrew files are the real test |
| W1.6 | **Font-aware recovery where `ToUnicode` is absent** | scoped, new | `hebrew-1.pdf` is the fixture (9 fonts, 0 `ToUnicode`, `Identity-H`, no `CIDToGIDMap`): parse the embedded font cmap, reverse GID→Unicode, refuse loudly when a glyph cannot be placed |
| W2 | **Bidi, mixed script, correct reading (P2)** | **started — your priority** | your call 2026-09-27: "implementing the correct search and reading is more critical" |
| W2.1 | Line/run assembly, UAX #9 levels, BD16 brackets, mirroring | not started | |
| W2.2 | Digits, dates and Latin runs inside RTL text | not started | covered by fixtures already |
| W2.3 | Arabic + Hebrew + English in one paragraph (`he-eng-ar.pdf`) | not started | real fixture available |
| W2.4 | Search/index normalization (NFKC, harakat, ZWNJ for matching only, digit folding, lam-alef collapse, Hebrew niqqud) | **in flight** | `docs/SEARCH-NORMALIZATION.md` will be the rule record; search-side only, never applied to output |
| W2.5 | Multi-column / table reading order | not started | needs a Reason, never a silent guess |
| W3 | **Verification lane** | in flight | PDFium pixels + vision review (dispatched) |
| W3.1 | PDFium pixel goldens (`pypdfium2`), fixed scale, non-blank assertion | **running** | |
| W3.2 | Rendered PNGs for human/vision review of every RTL fixture | **running** | this is the lane you asked for ("I'll review, you could too, with vision") |
| W3.3 | Differential oracle report: ours vs PDFium vs poppler, with the documented policy that they can't be *right* for RTL | **running** | |
| W3.4 | CI: pinned PDFium/qpdf oracles, corpus + goldens on every push | pending | poppler/mutool stay CI-only, never shipped |
| W4 | **Generation / writer (P3)** | not started | the biggest new-code chunk |
| W4.1 | Shaping + subsetting: Type0/Identity-H, `ToUnicode`, 65536-entry `CIDToGIDMap`, cluster → `/ActualText` | not started | `CIDToGIDMap` is the al-bdf "Latin glyphs instead of Persian" trap — already documented in the knowledge DB |
| W4.2 | Layout API that *cannot* split a shaping run across styles | not started | design constraint from the pipeline skill; retrofitting it later is expensive |
| W4.3 | Round-trip proof: generate → our extractor → PDFium pixels → human eyes | not started | |
| W5 | **Editing (P4)** | blocked on a spike | |
| W5.1 | Spike: does `lopdf` carry `BDC`/`EMC` through load→save? | not started | **decides patch vs rewrite** for the whole editor; cheapest high-value question left |
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
| W8.3 | Release automation, MSRV, `cargo vet`, reproducible builds | not started | |
| W8.4 | Final licence choice (AGPL+commercial vs permissive core) | deferred by you | must land before shipping an SDK, not before the POC |
| W9 | **Continuity** | ongoing | |
| W9.1 | Knowledge DB (SQLite+FTS5 + markdown export) | exists | keep feeding: every bug we hit goes in |
| W9.2 | `AGENTS.md` per crate, `HANDOFF.md` at milestones | exists | refresh after W1 lands |
| W9.3 | Publishing: `scripts/publish-public.sh` snapshots the tree to the public mirror (private material excluded by construction) | **exists** | code-writing subagents push their own branch instead; a bare `git add -A` is forbidden for them |

## Recommended sequence

1. **Now, in parallel (no file overlap):** W1 completion (Rust, local lab) ∥ W2.4 search
   normalizer (own clone of the public repo) ∥ W3 render/oracle lane.
2. **Next:** W1.5 validation across the 51-file archive. This is where the POC either holds or
   teaches us something that changes the estimate. Report per file: extracted / refused, with
   the reason, and a hand-checkable sample.
3. **Then:** W5.1 spike (hours, not days) — it gates whether W5 is a patch or a rewrite, and the
   answer belongs in the estimate before any editor code is written.
4. **Then:** W2 proper (line/run assembly, mixed-script reading order) — **your call: this is the
   critical path**, ahead of the writer, because "correct reading and search" is the product's
   reason to exist and the thing no existing tool does for RTL.
5. **Then:** W4 (writer) → W6 → W8.

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

The estimates from `docs/ESTIMATE.md` predate the real archive. They hold for the code; what
changed is that *validation* is now a first-class workstream with 51 real files instead of a
handful of synthetic ones.
