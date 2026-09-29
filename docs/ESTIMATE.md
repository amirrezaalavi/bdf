# Estimate — pdfrtl v1 (Rust core + CLI + MCP)

Companion to `../README.md`. Basis: measured `al-bdf-engine` sources (file LOC below are real
`wc -l` numbers from the fork clone at `~/playground/ai/yolka/tmp/albdf-recon/albdf`), the six
research reports, and the assumption in §8 that **agents implement and a human reviews**.

**Units.** `agent-day` = one autonomous implement→test→commit cycle with review feedback available
(~6 productive hours). `human-day` = the owner's own hours: review, deciding "what is correct",
collecting fixtures, licence/packaging calls. **The human column is the critical path, not the agent column.**

**Confidence.** Scope shape: high. Agent-days: ±50% (agent throughput varies more than the work does).
Calendar: the human review rate dominates.

---

## 1. What we port from albdf (the RTL craft)

| albdf artifact | measured LOC | What is actually inside | Rust target | agent-days | Risk |
|---|---|---|---|---|---|
| `pdfrtltextnormalizer.{h,cpp}` | 325 | the 6-step search normalisation (NFKC presentation-form fold, tashkeel/tatweel strip, ZWNJ/ZWJ strip, Persian↔Arabic letter unification, digit folding, lam-alef collapse logical+visual) + `invertToLogical()` | near-verbatim logic on `unicode-normalization` | 1.5 | low |
| `pdfrtltextengine.{h,cpp}` | 748 | shaping→emission engine: font dict (Type0/Identity-H), `/W` from hmtx, `ToUnicode` from clusters, `/ActualText` wrap, glyph reversal policy, mark text-rise, cluster↔glyph mapping | policy + API re-implemented over **krilla/pdf-writer** (which already does subsetting, CID ToUnicode, ActualText, ReversedChars) → ~400–600 our LOC | 4 | medium — emission spec decisions land here |
| `pdfpagecontentrewriter.{h,cpp}` + `/ActualText` preservation on re-edit (fork commits `1d7b8c69`, `80548d8c`) | 405 + generator 214 | content-stream write-back that keeps BDC/EMC marked content alive across a rewrite | `lopdf`-based marked-content-aware rewriter | 4 | **high** — depends on whether `lopdf` carries marked content; budget a patch/PR |
| `pdftextsearchengine.{h,cpp}` | 433 | logical-query search: page joined into visual order with geometry-aware separators, matches mapped back to item spans, normalizer on both sides | bidi-aware search over our extract output | 2.5 | medium |
| `pdfbidi.{h,cpp}` | 334 | FriBidi seam: paragraph direction, level runs, log→vis | `unicode-bidi` adapter (~200 LOC) | 0.5 | low |
| `pdfshaper.{h,cpp}` | 231 | HarfBuzz seam: script/lang, cluster level, absolute offsets, advance/mark extraction | `harfrust` adapter | 1 | low–medium (the "absolute cluster offsets" trap) |
| RTL/form test suites (`tst_rtl*`, `tst_actualtexttest`) | 1,305 (27 named cases) | acceptance criteria: lam-alef round-trip & re-edit, Persian extraction, Hebrew round-trip, mixed bidi levels, digit folding, mark offsets, RTL font embedding, FreeText/form AP | ported as corpus fixtures + assertions | 2 | low |
| `docs/PROBLEMS.md`, 7 ADRs, `docs/research/002` | 241 + ADRs | 15+ named failure modes with root cause, plus the reference-implementation triage | design input, not code | 0.5 | none — this is the cheapest value in the whole fork |
| FreeText / form-field RTL appearance streams (R#5, R#6) | part of `pdfdocumentbuilder` | direct `/AP` generation with an embedded Type0 font, bypassing the painter | same idea, but trivially simpler: we own the writer | 2 | low (P3/P4 scope) |
| **Subtotal — porting the RTL craft** | **~4,000 LOC read, ~1,800 LOC ported** | | **~18 agent-days** | | |

Human counterpart for this block: **≈5 days** (judging what "correct" means per producer, reviewing
RTL output — this is the single most valuable human hour in the project).

## 2. What we do NOT port

| Item | Size | Why not |
|---|---|---|
| PDF engine guts (xref, filters, JBIG2/JPX/CCITT decoders, font parsing, colour) | **62,375 LOC** `Pdf4QtLibCore` | Rewriting this is years of work and the worst possible place for agent code — plausible-looking parser bugs, silent corruption, no oracle. Bundle QPDF + lopdf + krilla + PDFium instead |
| GUI layer (`Pdf4QtLibWidgets`/`LibGui`/`Editor`/`Viewer`/`PageMaster`) | **~54,000 LOC** | Not needed headless; byte-identical to upstream anyway |
| Qt build/toolchain (`Qt6 6.8` + vcpkg manifest, no `builtin-baseline`) | — | Non-reproducible by construction; also the LGPLv3/tivoization problem for a sold product |
| The fork's GPL-3.0 authored files as source | ~10,009 lines | Optional to read; **not copyable** into a differently-licensed crate. We own them and may relicense, but mixing licences now would foreclose the §5 options |

## 3. What we build new (no upstream exists)

| Item | agent-days | Note |
|---|---|---|
| Producer-aware logical-order recovery + reason codes | 4 | the core differentiator; the heuristic must be evidence-based, never "reverse if RTL" |
| Fixture corpus harness (producer × script × expected-result) | 4 | **first artifact**; ships publicly |
| JSON/JSONL contract: `--json` on every verb, `--schema`, exit codes, provenance ids | 3 | the real API; the CLI is thin over it |
| Page/object-parallel scheduler (rayon) + deterministic output | 2 | krilla builds pages independently |
| MCP server (tools: inspect/extract/search/generate/edit/sign) | 2 | thin adapter over the CLI contract — note this is small, which matters for §5 licence design |
| CI: 7-target matrix, pinned toolchain, `cargo-deny`+`cargo vet`, oracle lanes | 3 | includes the pure-Rust lane with PDFium/JPX features off |
| Extract layer (text with positions, reading order, tables, columns) | 6 | over lopdf, with PDFium as cross-check |
| Writer layer (generate: runs → krilla, fonts, tagging) | 8 | beyond the RTL engine itself |

## 4. Phase roll-up

| Phase | agent-days | human-days | Cumulative calendar (part-time, human-bound) |
|---|---|---|---|
| **P0** corpus + failure table + `inspect/extract --json` + CI | 10 | 5 | 2–3 weeks |
| **P1** logical-order extraction + bidi search + provenance JSONL | 14 | 6 | 4–5 weeks |
| **P2** generation with `ToUnicode`/`ActualText`/`ReversedChars` | 16 | 6 | 7–8 weeks |
| **MCP + agent surface** | 4 | 2 | **(v1 sellable here) 8–10 weeks** |
| **v1 total** | **44** | **19** | ≈ 8–12 weeks part-time, ≈ 4–6 weeks full-time |
| P3 edit/redact/bindings (C ABI, PyO3, napi) | 20 | 6 | +4–5 weeks |
| P4 PAdES/LTV, forms, PDF/A + PDF/UA validation | 20 | 6 | +4–5 weeks |
| **Full product total** | **84** | **31** | ≈ 4–6 months part-time |

## 5. Code projection (v1)

| Component | Rust LOC |
|---|---|
| `pdfrtl-core` (document model, RTL pipeline, extract, generate, edit) | 6,000–9,000 |
| `pdfrtl-cli` (verbs, JSON contract, schema) | 2,000–3,000 |
| `pdfrtl-mcp` | 500–1,000 |
| tests + corpus harness + oracles | 4,000–6,000 |
| **v1 total** | **13,000–19,000** |

For scale: this is **~20–30% of the C++ we are not writing**, at a fraction of the review cost,
because the engine is bundled and only the differentiators are ours.

## 6. What could blow this up

| Risk | Impact | Mitigation |
|---|---|---|
| `lopdf` cannot carry marked content through a rewrite (P0 finding) | +5–10 agent-days, possible fork | Test this in the first week; if it fails, do content rewriting via a custom tokenizer or extend `lopdf` upstream |
| krilla/hayro upstream stalls (single maintainer) | writer blocked on bugfixes | Vendor the pinned revision; keep our own emitter as a fallback; own the emission spec |
| Producer-detection ground truth is wrong | the differentiator becomes noise | Publish the corpus, fail loudly with reason codes, treat detection as a per-producer allowlist that grows |
| **Human review capacity** (RTL judgement) | silent correctness debt — the worst outcome for a paid RTL product | If review capacity is uncertain, add a native-speaker reviewer to P0/P1; the oracle-based loop (pixels + cross-tool extraction) carries most of the load |
| Scope creep into OCR/VLM/chunking/PDF-A-first | schedule doubles | Explicitly out of v1: OCR stays a bundled optional feature, chunking/embeddings are a *paid* layer, PDF/A is P4 |

## 7. Verification strategy (this is what makes the estimate real)

1. **Differential oracle:** the albdf C++ CLI stays in CI; `albdf extract --json` vs
   `pdfrtl extract --json` on the corpus, diff-driven.
2. **Pixel oracle:** PDFium render golden per fixture — the fork's near-miss (every RTL glyph painted
   as Latin `A` while its tests were green) is the reason this is mandatory, not optional.
3. **Structure oracle:** `qpdf --check` on every generated file; `verapdf` (MPL branch) in the P4 lane.
4. **Copy-paste oracle:** assert `/ActualText` content equals the logical source string.
5. **Invariant property test:** "logical order **or** explicit reason code" — no third outcome.

## 8. Assumptions that would invalidate the estimate

- Agents run with terminal + test access and CI feedback on every commit (no manual staging loops).
- No new engine is written; a production-grade pure-Rust renderer is *not* a v1 goal.
- The corpus can be assembled from public sources plus our own samples (Word, LibreOffice, Chrome,
  InDesign, LaTeX/XeLaTeX, ReportLab, mPDF outputs of the same sentences in Persian/Arabic/Hebrew).
- The human reviewer can read Persian and Arabic well enough to judge output — or an oracle-only
  review is explicitly accepted as sufficient for v1.
- Licence decision lands before the first public release; the crate split keeps both options open.

## 9. Suggested next commit (P0, week 1)

1. Cargo workspace (`pdfrtl-core`, `pdfrtl-cli`, `pdfrtl-mcp`) + pinned toolchain + `cargo-deny` config.
2. `pdfrtl inspect <file> --json` (metadata, fonts, `ToUnicode` sanity, `/ActualText` presence, producer).
3. Corpus harness + 12 fixtures (4 sentences × 3 producers) with expected logical text.
4. `lopdf` marked-content round-trip spike → decides whether P3 is a patch or a rewrite.
5. `THIRD-PARTY-NOTICES.md` + `DEPS.md` from `pdf-rtl-research/04-capability-matrix.md`.
