# pdfrtl — what's left

Status 2026-09-27. Every claim here has an artifact behind it; the evidence is named.

## Where we actually are

| area | state | evidence |
|---|---|---|
| Workspace, gates, CI | **done** | `cargo fmt`/`clippy -D warnings`/11 tests/`cargo deny` (advisories, bans, licenses, sources all ok); `.github/workflows/ci.yml` |
| Licence posture (Option A) | **decided, deferred** | ADR 0001–0003; permissive-only shipped features gated by `cargo deny`; final choice deferred per your "POC first" |
| Corpus | **59 fixtures** | 7 synthetic/own-work + 51 real archive (gitignored, hashes committed) + 1 bundle |
| Real-archive profile | **measured** | 27 Type0 fonts, 21 `CIDToGIDMap`, 19 with no text layer, 2 with `/ActualText`, **0** with `/ReversedChars` |
| Producer coverage | **broad** | Chrome (generated) + InDesign ×4, Acrobat Pro, Word 2013, Nitro PrimoPDF, macOS Quartz (real, from your archive) |
| Extraction (P1) | **red phase** | `text/tokenizer.rs` (378 lines, total lexer) and `tests/text_recovery.rs` (226 lines) are real; `text/cmap.rs::ToUnicode::parse` and `text/recover.rs` are stubs that return empty |
| Oracles | **partial** | `pdftotext` proven to drop Persian on real Chrome output; `pypdf` for structure; PDFium/`pypdfium2` being wired now |
| Human verification | **1 open** | `synthetic-actualtext-fa-001` still `unverified`; `hebrew-1.pdf` name-vs-content disagreement; 7 `unknown-N.pdf` files |

## The two facts that shape everything left

1. **The corpus is two producer families, and a fix for one is not a fix for the other.**
   Chrome marks an RTL run with `/ReversedChars` and annotates clusters with `/ActualText`.
   Your real Persian/Arabic/Hebrew archive does neither — those files carry only `ToUnicode`,
   and 19 of them have no text layer at all. Both paths must exist, and they fail differently.
2. **No third-party tool can be the reference for RTL.** poppler drops Arabic-script
   `/ActualText` payloads outright (measured on our own fixtures); PDFium and PDF.js return
   visual order. Ground truth for logical order is a human reading the page — which is why the
   vision/render lane exists and why the critical path is *review capacity*, not code volume.

## Workstreams

| # | workstream | state | notes |
|---|---|---|---|
| W1 | **Extraction (P1)** | in progress | the POC's core |
| W1.1 | `ToUnicode` CMap parser (`codespacerange`, `bfchar`, `bfrange`, 1- and 2-byte codes) | stub → implement | 1-byte codes matter for simple-font files (`hebrew-2.pdf` is Type1) |
| W1.2 | Unit model + `/ReversedChars` inversion + run order via UAX #9 | stub → implement | the hard part is the LTR run *inside* an RTL line (`۱۴۰۳/۰۵/۱۲` must end up last, digits unreversed) |
| W1.3 | Reasons + exit-code-3 refusal path wired end-to-end | pending | `Reason` vocabulary already exists; tests already assert it |
| W1.4 | `pdfrtl extract --json` CLI verb | pending | envelope shape fixed in the plan |
| W1.5 | Validate against the 51-file archive | not started | classify every file as *extracted with reason* or *refused with reason*; the Arabic docs (91/62/321/185 pages) and the 5 Hebrew files are the real test |
| W1.6 | Extraction on files with **no** `ToUnicode` (`hebrew-1.pdf`: 9 fonts, 0 maps) | not started | expected outcome is a loud refusal, or font-cmap recovery — decide which |
| W2 | **Bidi, mixed script, search (P2)** | not started | |
| W2.1 | Line/run assembly, UAX #9 levels, BD16 brackets, mirroring | not started | |
| W2.2 | Digits, dates and Latin runs inside RTL text | not started | covered by fixtures already |
| W2.3 | Arabic + Hebrew + English in one paragraph (`he-eng-ar.pdf`) | not started | real fixture available |
| W2.4 | Search/index normalization (NFKC, harakat, ZWNJ for matching only, digit folding, lam-alef collapse) | not started | needed for the `search` verb and for AI-agent retrieval |
| W2.5 | Multi-column / table reading order | not started | needs a Reason, never a silent guess |
| W3 | **Verification lane** | partly in flight | PDFium pixels + vision review (dispatched) |
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
| W7 | **Scans / OCR** | **needs your decision** | 19 archive files have no text layer. In scope for pdfrtl, or a separate tool that feeds it? |
| W8 | **Packaging & product** | not started | |
| W8.1 | MCP server over the core (the paid-tier surface) | not started | transport decision (Rust SDK vs TS over CLI) still open |
| W8.2 | Docker image + single static binary per platform (linux x64/arm64, macOS, Windows) | partial | `Dockerfile` exists and is wired to the corpus harness |
| W8.3 | Release automation, MSRV, `cargo vet`, reproducible builds | not started | |
| W8.4 | Final licence choice (AGPL+commercial vs permissive core) | deferred by you | must land before shipping an SDK, not before the POC |
| W9 | **Continuity** | ongoing | |
| W9.1 | Knowledge DB (SQLite+FTS5 + markdown export) | exists | keep feeding: every bug we hit goes in |
| W9.2 | `AGENTS.md` per crate, `HANDOFF.md` at milestones | exists | refresh after W1 lands |

## Recommended sequence

1. **Now, in parallel (no file overlap):** W1 completion (Rust) ∥ W3 render/oracle lane.
2. **Next:** W1.5 validation across the 51-file archive. This is where the POC either holds or
   teaches us something that changes the estimate. Report per file: extracted / refused, with
   the reason, and a hand-checkable sample.
3. **Then:** W5.1 spike (hours, not days) — it gates whether W5 is a patch or a rewrite, and the
   answer belongs in the estimate before any editor code is written.
4. **Then:** W2 (bidi/mixed/search) — the differentiator that makes the paid tier worth paying
   for, and the thing no existing tool does for RTL.
5. **Then:** W4 (writer) → W6 → W8.

## What only you can settle

| item | why it blocks |
|---|---|
| `hebrew-1.pdf`: Hebrew by name, Arabic-script + Latin by content, zero `ToUnicode` | determines whether it is a refusal fixture or a Hebrew fixture |
| The 7 `unknown-N.pdf` files (no text layer, no language marker) | language labels for the corpus |
| Human read of the 4 Persian fixtures' expected text | promotes them from `unverified` to asserted |
| W7: is OCR in scope? | 19 of your files are images-only; that is a different product surface |
| W8.4: final licence | only blocks shipping, not building |

## Effort (agent-time, with human review on the critical path)

| phase | agent-time | human review |
|---|---|---|
| W1 finish + W1.5 validation | 2–4 days | ~1 hour (reading pages, spot-checking output) |
| W2 bidi/mixed/search | 1–2 weeks | hours |
| W4 writer | 2–3 weeks | hours per milestone |
| W5 editor (after spike) | 2–3 weeks | hours |
| W6 + W8 | 2 weeks | — |

The estimates from `docs/ESTIMATE.md` predate the real archive. They hold for the code; what
changed is that *validation* is now a first-class workstream with 51 real files instead of a
handful of synthetic ones.
