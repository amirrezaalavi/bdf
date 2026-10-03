# bdf (working name: pdfrtl) — RTL-first PDF core for agents

> **Licence: TBD — all rights reserved.** No licence file is granted yet; this repository is shared
> by invitation only. The planned long-term posture is AGPL-3.0-or-later plus a commercial
> dual licence (`docs/decisions/0001`), but **nothing here is AGPL today**, and the canary build
> carries no licence at all.

**Last updated:** 2026-10-03
**Private-corpus measurement snapshot:** 2026-09-29
**Canonical project:** `https://github.com/amirrezaalavi/bdf`
**Development environment:** WSL Ubuntu-26.04; Rust/Cargo 1.98.1 pinned in `rust-toolchain.toml` and CI.
**Current public `main` at start of this documentation update:** `9411645b` (CI green).

Language **Rust 1.98.1** (pinned exactly, on both sides — `docs/problems/0003`). Licence posture
decided: **AGPL-3.0-or-later + a commercial dual licence** (`docs/decisions/0001`), with a
permissive-only dependency rule enforced by `cargo deny` (`docs/decisions/0003`). The *final*
licence is deferred until the POC is proven — it blocks shipping an SDK, not building.
Owner: Yolka / Almas Shabake Tek + bornarad.co.

**Current extraction snapshot (private archive, 51 files, measured 2026-10-03, produced by
`scripts/by_language.py`):** **Persian, the focus, is 25 of those files — 13 emit every decoded
character, 1 is partial, 8 are refused entirely, 3 have no text layer; the median Persian file emits
100% of its decoded text.** Hebrew is 4/4. Across all languages: 31 fully emitted, 4 partial, 10
refused, 6 no-text. See [docs/ACCEPTANCE.md](docs/ACCEPTANCE.md).
These numbers are owner-local and cannot be reproduced from the public clone without the customer
PDFs.

**Trying the canary?** Read [`CANARY.md`](CANARY.md) — what the binary does, what it refuses,
and how to verify what you were handed. Prebuilt static Linux binaries (x86_64 and arm64) are on
the [releases page](https://github.com/amirrezaalavi/bdf/releases); `bash scripts/build-release.sh`
builds the same two files locally. Text that was decoded but could not be ordered is reported
under `data.pages[].unproven` (behind `--include-unproven`) rather than discarded; `data.text`
stays proven-only. See [`docs/CLI.md`](docs/CLI.md) for the full contract and
[`docs/ACCEPTANCE.md`](docs/ACCEPTANCE.md) for the measured numbers.

Plan/decisions: `docs/plans/2026-09-29-lane-plan.md` · live state: `HANDOFF.md` ·
contributor rules: `AGENTS.md` · owner questions: `docs/OPEN-QUESTIONS.md`.

---

## 0. Clone and run it

```bash
git clone https://github.com/amirrezaalavi/bdf.git
cd bdf

# The toolchain comes from rust-toolchain.toml — rustup installs 1.98.1 automatically.
# Keep build artifacts out of the repo:
export CARGO_TARGET_DIR="$HOME/target-bdf"

cargo build --workspace
bash scripts/wsl-build.sh        # the FULL local gate — this is exactly what CI runs
```

Then prove the clone is healthy before trusting it:

```bash
bash scripts/verify-clone.sh
```

`verify-clone.sh` reports the toolchain actually in use, builds, runs the tests, and states which
corpus files are present and which are *expected* to be missing. It **exits non-zero** if the clone
is broken — a verifier that reports success without looking is the defect class this repo has hit
four times (`docs/problems/0006`, `0009`, `0011`, `0012`).

**The 51 real-world corpus PDFs are deliberately not in this repository.** They are customer
documents (contracts, invoices, reports) kept out of git on purpose, and
`scripts/publish-public.sh` carries a deny-list that refuses to publish them. A clone therefore
contains the **14 redistributable synthetic fixtures**, which is enough to build, test and gate. To
work against the real archive, place the files in `corpus/raw/private/desktop-pdfs/` — that path is
gitignored, and only hashes and metadata are ever committed, never the documents.

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

## 4. Where the work actually is

**Shipped and measured** — `crates/pdfrtl-core/src/text/recover.rs`, `crates/pdfrtl-cli`:

- `ToUnicode` CMap parsing (1- and 2-byte codes, `codespacerange`/`bfchar`/`bfrange`)
- unit model with `/ReversedChars` inversion, `/ActualText` spans preserved whole
- **the order ladder**: `/ActualText` → geometry (UAX #9 run comparison against *measured* painted
  positions) → producer fingerprint → **refuse with a reason**. A tie in the painted order is
  *unanswerable* and falls through rather than guessing
- per-page granularity, `unsupported_*` reasons, stable exit codes
- CLI `pdfrtl extract --json`; exit codes 0/2/3/4 documented in `docs/CLI.md`

**Not started:** generation/writer (W4), editing (W5), enterprise (W6), OCR (W7 — deferred by
owner), MCP server (W8.1).

**The order of work is in `docs/plans/2026-09-29-lane-plan.md`.** In short: per-font withholding for
broken decoders, then font-program recovery for fonts with no usable `/ToUnicode`, then the order
recovery that makes the geometry decide instead of refusing.

---

## 5. Why the corpus is two problems, not one

Measured on the real archive, and it is the single most important thing to understand before
changing the pipeline:

1. **Chrome-exported fixtures mark their RTL runs.** `/ReversedChars` plus per-cluster
   `/ActualText`. The producer has already answered the ordering question — recovery is a mechanical
   walk.
2. **The real archive does neither.** Those files carry only `ToUnicode`, and their stored order is
   producer-dependent and not self-declaring. 14 of them still refuse because the geometry ties and
   nothing else can decide.

A fix for one family proves nothing about the other. See `docs/problems/0011` for two instruments
built to tell them apart and **retired** because they could not.

**No third-party tool is a reference for RTL** — measured, not assumed (`docs/problems/0004`):
poppler flips every lam-alef pair, Xpdf returns visual order, PDFium drops ZWNJ. Ground truth is a
rendered page read by a human or by vision. That is why the critical path is *review capacity*, not
code volume.

---

## 6. What we learned from `al-bdf-engine` (and what we carry over)

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
2. **`lopdf` marked-content round-trip** — **measured, incremental-only**: a plain `Document::save`
   re-serialised 24 of 25 objects and degraded `Object::Real(f32)` precision. Use
   `IncrementalDocument` for edits; never mutate `Stream::content` with a stale `/Length`.
3. **No pure-Rust renderer of record** — PDFium stays the optional oracle; never gate a release on it.
4. **Producer detection has no ground truth** — the corpus *is* the ground truth, and half of it
   cannot be published. Fail loudly rather than guess.
5. **Licence/authorship** — an AGPL+commercial model requires owning enforceable copyright; keep
   human-authored specs, corpus, and review records (purely AI-generated material is not
   copyrightable in the US).
6. **Adoption** — lead with the fixture corpus + failure table, not with architecture.

---

## 8. Where to read next

| question | file |
|---|---|
| Clone, build, verify a fresh checkout | this file, §0 |
| What is being worked on, and why | `docs/plans/2026-09-29-lane-plan.md` |
| Current state and open threads | `HANDOFF.md` |
| What NOT to do (hard-won) | `AGENTS.md` |
| Exit codes and JSON envelope | `docs/CLI.md` |
| Decisions and why | `docs/decisions/` (0001–0006) |
| Failures with root causes | `docs/problems/` (0001–0012) |
| Dependency licences | `docs/DEPS.md` (CI-checked against `deny.toml`) |
| The corpus itself | `corpus/README.md`, `corpus/AGENT.md` |
| Effort estimates | `docs/ESTIMATE.md` |
