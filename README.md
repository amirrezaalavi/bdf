# pdfrtl — RTL-first, agent-driven PDF core (working name)

**Status:** Phase 0 complete — the workspace builds, `clippy -D warnings` is clean, 11 tests pass,
and the corpus harness produces a report. There is **no extraction or generation yet** (that is P1/P2).
Plan of record: `.hermes/plans/2026-09-27_160341-pdfrtl-skeleton-and-p0-plan.md` · live state: `HANDOFF.md` ·
blocking decisions: `docs/OPEN-QUESTIONS.md` · traps already paid for: `AGENTS.md`.
Language decided: **Rust 1.97.1**. Licence decided: **AGPL-3.0-or-later + a commercial dual licence**
(`docs/decisions/0001`), with a permissive-only dependency rule enforced by `cargo deny` (`docs/decisions/0003`).
Owner: Yolka / Almas Shabake Tek + bornarad.co.

---

## 1. What this is

A headless PDF **library + CLI + MCP server** that is correct for Arabic, Persian and Hebrew
(including mixed RTL/LTR), fast, feature-rich, multi-platform, and embeddable as one structure —
built for AI agents and programs first, humans second.

**What we build:** the RTL text pipeline, the agent contract (JSON/JSONL), producer-aware logical-order
recovery, bidi-aware search, page-parallel scheduling, and a thin signing/forms/conformance layer.

**What we do not build (bundled instead):** the PDF engine itself — parsing/xref/repair (QPDF, lopdf),
rendering (PDFium, feature-gated), the writer primitives (krilla / pdf-writer), font subsetting
(skrifa/subsetter), crypto (RustCrypto), OCR (Tesseract), conformance validation (veraPDF, MPL branch).

**The product invariant:** every extraction either returns **logical order** or fails with an
**explicit reason code**. No silent reversal, no best-effort guesses. That single property is what no
existing tool offers.

---

## 2. Priorities as engineering requirements

| # | User priority | Engineering translation |
|---|---|---|
| 1 | Full RTL + mixed-language support | bidi levels + shaping + `/ActualText` emission + producer-aware recovery; verified per producer, not per claim |
| 2 | Fast, multithreaded where possible | `Send + Sync` document model, rayon over pages/objects, per-run text pipeline (embarrassingly parallel) |
| 3 | Feature-rich, enterprise-capable | edit/redact/forms/PAdES/PDF-A bundling plan; basic features in the core, heavy ones behind features |
| 4 | Multi-arch / multi-OS | 7 targets from day one (win/mac/linux × x64/arm64 + musl + wasm), pinned toolchain |
| 5 | One bundled structure, embeddable | one static CLI + one core crate + C ABI; **no GUI, no Qt, no copyleft in the default feature set** |
| 6 | Agents drive it | `--json` on every verb, `--schema`, JSONL streaming, stable provenance ids, deterministic exit codes, MCP adapter |

---

## 3. Architecture

```
pdfrtl/                     # cargo workspace
├── crates/
│   ├── pdfrtl-core/        # document model, RTL text pipeline, extract, edit, generate
│   ├── pdfrtl-cli/         # `pdfrtl <verb> --json` — the primary product surface
│   ├── pdfrtl-mcp/         # MCP server (thin: wraps the CLI/JSON contract)
│   └── pdfrtl-bindings/    # (P3) C ABI, PyO3, napi-rs, wasm-bindgen as satellites
├── corpus/                 # producer × script fixtures + expected results  ← first-class artifact
├── oracles/                # CI-only: albdf (C++), mutool, qpdf, pdfium, verapdf
└── docs/                   # this file, ESTIMATE.md, ADRs, PROBLEMS.md (our own)
```

**Engine map**

| Concern | Dependency | Licence | Notes |
|---|---|---|---|
| Parse/edit objects | `lopdf` 0.45 | MIT | `Send + Sync`, rayon, wasm; **verify marked-content round-trip** (§7 risk 2) |
| Write / marked content | `krilla` 0.8 + `pdf-writer` 0.15 | MIT | already emits `/ActualText` + `/ReversedChars`, subsets fonts, writes CID `ToUnicode` |
| Shaping | `harfrust` 0.13 | MIT | HarfBuzz-parity port; `rustybuzz` is archived — do not use |
| Bidi | `unicode-bidi` 0.3 | MIT/Apache | UAX #9, same impl typst and pdf-inspector use |
| Script/line-break props | `icu_properties`, `icu_segmenter` | Unicode-3.0 | pure Rust, `compiled_data` |
| Fonts | `fontdb`, `skrifa`, `read-fonts`, `subsetter` | MIT/Apache | |
| Render / pixel oracle | PDFium via `pdfium-render` | BSD-3 | **optional feature, runtime-loaded**, never required |
| Crypto / PAdES | RustCrypto (`cms`, `x509-cert`, `der`, `sha2`) + `rustls` | MIT/Apache | PAdES-baseline first |
| OCR (optional) | Tesseract `ara`/`fas`/`heb` | Apache-2.0 | behind a feature, provenance per page |

**The text pipeline (ours):** itemize (script × direction × style) → bidi levels → reorder runs →
shape each run (`harfrust`, absolute cluster offsets) → seriate with GPOS advances → emit with
subset font + `ToUnicode`, wrapping reversed clusters in `/ReversedChars` and non-1:1 clusters in
`/Span <</ActualText …>>` in logical order. Never let styling split an Arabic word.

**Agent contract:** `pdfrtl <verb> --json` for every read and write verb (`inspect`, `extract`,
`search`, `render`, `generate`, `edit`, `redact`, `sign`, `validate`), `--schema` to emit JSON Schema,
JSONL for streaming, exit codes documented in `docs/CLI.md`, and honest `reason` codes for anything
we refuse to do. The MCP server is an adapter over that contract, not a second implementation.

---

## 4. Scope by phase

| Phase | Deliverable | Exit criteria |
|---|---|---|
| **P0 — oracle + corpus** | `inspect`/`extract --json`, fixture corpus, failure table | Every fixture classified correct/incorrect **with a reason code**; CI runs PDFium pixels + `qpdf --check` + albdf oracle |
| **P1 — read** | logical-order extraction, bidi-aware search, provenance ids, JSONL | Extracted text equals logical source for every supported producer; unsupported producers fail loudly |
| **P2 — write** | hrl generation: shaping → bidi → krilla emission with `ToUnicode`/`ActualText`/`ReversedChars` | Render golden + extraction round-trip + Acrobat/Chrome copy-paste spot-check |
| **P3 — edit** | content rewriting with marked-content preservation, page ops, redaction with proof, bindings | Edit→re-extract invariants hold; signatures survive incremental update |
| **P4 — enterprise** | PAdES/LTV, forms + RTL appearance streams, PDF/A-2b + PDF/UA-1 validation | Signature validates in Adobe + `pdfsig`; veraPDF (MPL) passes |

---

## 5. Licence posture (decision open, structure now)

Their stated plan — *free GUI bundle under an open licence, paid core + MCP* — has one structural
trap: **the free GUI bundles the core, so the core cannot be the paid part.** Paywall the
**agent/enterprise surface**, not the thing the free product ships.

Keep two options open by construction: separate crates (`pdfrtl-core` vs `pdfrtl-mcp` /
`pdfrtl-enterprise`) so each can carry a different licence later without a rewrite, and keep a
`LICENSE`-less repo marked `License: TBD — all rights reserved` in the README.

| Option | Free tier | Paid tier | Protects against | Watch out |
|---|---|---|---|---|
| **A (recommended): AGPL core + commercial** | GUI + CLI + core, AGPL-3.0 | commercial core licence + MCP + enterprise features | closed-source resale **and** hosted clones (§13) | requires owning 100% of copyright — see the AI-authorship warning |
| B: permissive core + paid service | everything MIT/Apache | MCP + hosted + enterprise features + support | nothing code-wise | a thin MCP wrapper is ~1k LOC — anyone can rebuild it from the MIT core |
| C: source-available core (BUSL/FSL) | GUI + CLI, non-compete terms | same, with support | competing hosted services | not OSI open source; "open licence" marketing becomes muddy |

Mandatory regardless of choice: `THIRD-PARTY-NOTICES.md` + `DEPS.md` (licence + evidence URL per
dependency), CI gate `cargo-deny` (deny GPL/AGPL/LGPL in default features) + `cargo vet`, and a CLA
before accepting outside contributions.

---

## 6. What we learned from `al-bdf-engine` (and what we carry over)

The fork is the most RTL-complete PDF work found in the survey, and most of its value is **written
down knowledge**, not code:

| Asset in the fork | What it gives us |
|---|---|
| `pdfrtltextnormalizer` (325 LOC) | a documented 6-step search normalisation algorithm: NFKC fold of presentation forms → strip tashkeel/tatweel → strip ZWNJ/ZWJ → unify Persian↔Arabic letters → fold digits → collapse lam-alef (logical *and* visual) + `invertToLogical()` |
| `docs/PROBLEMS.md` (241 lines) | **the bug corpus**: P1–P6, R#1–R#6, S#1–S#3 — 15+ named failure modes with root cause and fix |
| 7 ADRs + `docs/research/002-rtl-reference-implementations.md` | design rationale and the reference implementations (Skia/cairo/fpdf2 #1802) already triaged |
| 27 named RTL test cases (≈1.3k LOC) | ready-made acceptance criteria: lam-alef round-trip, Persian extraction, Hebrew round-trip, mixed bidi levels, digit folding, mark offsets, RTL font embedding, FreeText/form AP |
| `AGENTS.md` + per-folder agent guides + anti-slop gate | a proven agent-workflow contract (determinism rules, evidence-gated task closure) we can seed our own repo with |

**The three lessons worth the most:**
1. **`/CIDToGIDMap` for 2-byte CID fonts must be a 65536-entry stream** (ISO 32000-1 §9.7.4.3) —
   a short array silently made every RTL glyph render as Latin `A` in Ghostscript, and their golden
   tests never caught it because they never pixel-verified their own output. → we pixel-verify from P0.
2. **HarfBuzz ≥4 already emits RTL runs leftmost-first** — re-applying the classic "reverse RTL"
   fix mirrors the text. The trap is documented in the wild (fpdf2 #1802) and is *wrong for modern HB*.
3. **`ToUnicode` is one UTF-16 unit per glyph, so ligatures degrade** (`لا` → `ل`) — `/ActualText` is
   the only place exact logical text can live, which is why emission and extraction are one problem.

---

## 7. Top risks

1. **krilla/hayro are single-maintainer crates** — vendor the pinned revision, keep our own emitter
   as a tested fallback, own the emission spec.
2. **`lopdf` marked-content round-trip** may not carry `BDC/EMC` through a content rewrite (the fork
   had to patch its own editor for exactly this). Verify in P0; budget a fork/upstream patch.
3. **No pure-Rust renderer of record** — PDFium stays the optional oracle; never gate a release on hayro.
4. **Producer detection has no ground truth** — the corpus *is* the ground truth; publish it, and fail
   loudly rather than guess.
5. **Licence/authorship** — an AGPL+commercial model requires owning enforceable copyright; keep
   human-authored specs, corpus, and review records (US Copyright Office: purely AI-generated
   material is not protected, prompts alone are not sufficient control).
6. **Adoption** — lead with the fixture corpus + failure table, not with architecture.

---

## 8. Docs

- `docs/ESTIMATE.md` — port map from albdf, agent/human day estimates, LOC projection, risks
- `../yolka/tmp/pdf-rtl-research/00-SYNTHESIS.md` — full research synthesis (licences, standards, stack, self-critique)
- `../yolka/tmp/pdf-rtl-research/01..06-*.md` — the six source reports
