# Research questions — for an outside research agent

**What this file is.** A queue of knowledge gaps that cannot be closed locally. Each entry
states what we need, why it blocks us, what we already measured, and what a usable answer
looks like. Answers come back **in this file, under the question**, and are then consumed
and deleted by the project agent.

**Who answers.** An external research agent with web/library/code access. You are not
asked to agree with our design; you are asked to establish **facts with sources**.

---

## Ground rules for answering

1. **One answer per `Q-Rn` block, inserted directly under it.** Do not rewrite the question.
2. **Cite something real for every factual claim** — a URL plus, where it exists, a
   version or a clause: ISO 32000-1/2 clause, UAX #9 section, RFC, release notes, or
   `file:line` in an open-source implementation (`poppler`, `pdf.js`, `PDFium`, `MuPDF`,
   `fontTools`, `harfbuzz`). A claim without a source is worth nothing to us; if you could
   not verify it, write **UNVERIFIED** and say what you tried.
3. **Prefer primary and adversarial sources over blog summaries.** If two implementations
   disagree, that disagreement *is* the finding — report both with references.
4. **Distinguish "the spec says" from "this implementation does".** We need both, labelled.
5. **Say what your answer changes.** Each answer ends with one line:
   `Impact: <what we would do differently now>`.
6. **No guessing, no filling gaps with plausible prose.** "Nobody documents this" is a
   valid, useful answer. We would rather refuse a file than reverse it wrongly.

**Context you need about the project** (so your answers land):

`pdfrtl` is a headless RTL-first PDF library (Rust) for Arabic/Persian/Hebrew alongside
Latin. Its single invariant: *extraction returns logical order, or it fails with an explicit
reason — silent reversal is a bug, there is no third outcome.* Order recovery is a ladder:
`/ActualText` → `ToUnicode` + UAX #9 levels → allow-listed producer fingerprint → refuse
(`unsupported_*`, exit 3). A page whose order cannot be established is **withheld** from the
text output and counted instead. Measured so far, on our own fixtures: Chrome/Skia writes
per-cluster `/ActualText` + `/ReversedChars`; poppler flips every lam-alef pair
(`سلام` → `سالم`); Xpdf returns visual order; PDFium drops ZWNJ and returns per-cluster
visual order. 51 real-world archive files: 37 order-verified, 14 refused (mostly Persian).
Details: `docs/problems/0001…0006`, `docs/ROADMAP.md`.

---

## Q-R1 — Deciding "visual or logical" for RTL runs whose producer is unknown or generic

**Blocking:** 14 of our 51 real files. They carry `/ToUnicode` that maps codes to **base**
Arabic letters (no presentation forms), no `/ActualText`, no `/ReversedChars`, and a producer
string that is either generic (`Adobe PDF Library …`, `Microsoft® Word 2013`) or absent. We
have no evidence to name their stored order, so we withhold them rather than risk a silent
reversal.

**Need:** any *reliable* in-file signal or algorithm that decides whether a stored RTL
sequence is painted-visual or logical, specifically:

- Is GID order in `/CIDToGIDMap` (or the CID→GID assignment for an `Identity-H` font)
  correlated with logical order, and does any implementation rely on that?
- Do the glyph **advance widths** (`/W` array) or the placement operators (`Td`/`TJ`
  offsets) reveal reversed ordering for a run?
- Does the font's `cmap` (when present) or subset ordering carry the signal?
- What do `poppler`, `pdf.js`, `PDFium`, `MuPDF` actually do with such a file — do any of
  them ever *reorder* RTL runs, and by what rule? Give `file:line` references.
- Same question for Acrobat itself: what does Acrobat's copy-paste return for such files?
  If there is a documented statement (or a reproducible report) about its RTL handling of
  Adobe-produced PDFs, that is the benchmark we care about.

**Acceptable evidence:** implementation source references, spec clauses, or a documented
producer behaviour with a version. A decision rule we can encode **with its failure modes**
is the ideal answer. "There is no such signal for producer family X" is equally valuable —
it tells us to keep refusing and to say so in our docs.

**Impact line required.** e.g. `Impact: we would implement detection rule R for family F and
keep refusing F' — 8 of 14 files recovered, 6 still withheld honestly.`

---

## Q-R2 — Inverting a visual-order run to logical order (UAX #9 in reverse)

**Blocking:** our rung-3 implementation for mixed content.

**Need:** the accepted algorithm for turning a *painted-visual* RTL run/line back into
logical order, including: Latin runs and digits inside an RTL line (e.g. `۱۴۰۳/۰۵/۱۲` must
end up in the right place and unreversed), brackets and BD16 mirroring, ligature clusters
(lam-alef) that must stay whole, and combining marks. Which implementation should we treat
as authoritative (ICU `ubidi`, `pdf.js` bidi, poppler `TextOutputDev`, MuPDF), and what are
the documented traps? Does the standard say anything about recovering logical order from a
visual stream at all (ISO 32000, UAX #9 scope), or is this entirely implementation-defined?

**Acceptable evidence:** code references plus a worked example on a known string.

**Impact:** the ordering core of the product.

---

## Q-R3 — Producer behaviour table, with versions and the reason behind it

**Need:** for each producer/version below, does it store RTL runs **visual** or **logical**,
what marker (if any) does it emit (`/ReversedChars`, `/ActualText` per cluster, presentation
forms, nothing), and *why* — which library/engine inside it makes that choice:

Chrome/Skia PDF print · LibreOffice · Microsoft Word 2013–365 PDF export · Adobe InDesign
17–20 (Win/Mac) · Adobe Acrobat Pro 11 · Adobe Distiller / "Adobe PDF Library" · macOS
Quartz/Preview · Nitro PrimoPDF · ReportLab · wkhtmltopdf · PDFreactor · mPDF · iTextSharp ·
LaTeX (pdfTeX/LuaTeX/XeTeX, with and without `bidi`/`arabxetex`) · Ghostscript (ps2pdf).

**Acceptable evidence:** reproducible reports, maintainer statements, issue trackers, source
code. We have our own measurements for a few of these and will cross-check — an independent
source that contradicts us is exactly what we want.

**Impact:** the allow-list we ship (each entry must be backed by a test).

---

## Q-R4 — Font-aware recovery when `ToUnicode` is absent

**Blocking:** `hebrew-1.pdf` (29 pages; 9 fonts, **0** `/ToUnicode`, `Identity-H`, no
`/CIDToGIDMap`) is the fixture; the case is common in older/consumer-produced files.

**Need:** the correct, deterministic procedure to parse an embedded TrueType/OpenType font
and map **GID → Unicode** for Arabic/Hebrew, including: which table to trust (`cmap` subtable
format 4 vs 12, `post` names, the subset's ordering), how to resolve the ambiguity when many
GIDs map to one or several presentation forms, what to do when the subset's `cmap` is
stripped/empty, and how to fail loudly instead of guessing. Tooling that already does this
(`fontTools`, `harfbuzz` GID→Unicode, `ttf-parser`) with version-specific caveats.

**Acceptable evidence:** spec clauses (OpenType `cmap`/`post`, ISO 32000 §9.6–9.10) plus
working references.

**Impact:** decides whether we can read a whole class of files or must refuse them.

---

## Q-R5 — Presentation forms (U+FE70–FEFF) in extraction output

**Need:** when a file's `/ToUnicode` maps glyphs to Arabic presentation forms, what is the
correct behaviour for an extractor that promises *logical text* — keep the forms, fold to
base letters (NFKC), or keep both? What do Acrobat, PDFium, poppler and `pdf.js` emit, with
references? Is there a documented reason any of them chooses as it does? Does folding change
offset/cursor semantics for search-and-replace, and is there a known convention (e.g. in
PDF/UA or accessibility tooling) for which form should be *stored*?

**Impact:** our reason vocabulary, and the split between extraction output and search
normalization.

---

## Q-R6 — Incremental editing in Rust: does `lopdf` survive marked content?

**Blocking:** whether our editor is a patch (incremental update) or a rewrite (parse → model →
re-emit). Cheapest high-value unknown left in the plan.

**Need:** current (2026) state of `lopdf` — does `Document::load` → `save` preserve
marked-content operators (`BDC`/`EMC`) and per-glyph `/ActualText` spans, and what are the
known data-loss reports? Same question for any Rust crate that can do **incremental update**
edits (append-only, preserving unknown objects/streams). If the answer is "none of them can",
name what does exist (even non-Rust) and the technique it uses.

**Acceptable evidence:** crate docs, source references, issue links, and any maintainer
statement about byte-preservation guarantees.

**Impact:** decide W5 (editor) shape before a line of editor code exists.

---

## Q-R7 — Writer stack for RTL shaping in Rust (state of the art, 2026)

**Need:** for generating Arabic/Persian/Hebrew PDFs in Rust: `harfrust` vs `rustybuzz`
(cluster API, RTL run output order, maturity, licence, maintenance), plus `krilla` /
`pdf-writer` readiness for Type0/`Identity-H` fonts, `/ToUnicode`, 65536-entry
`/CIDToGIDMap` and `/ActualText` spans — with version numbers and any open issues that would
bite us. What combination would you pick, and what is known to be broken?

**Impact:** the generation phase's dependency choice (must stay permissive-licensed).

---

## Q-R8 — Standards: what is actually *required* about logical order and `/ActualText`

**Need:** PDF/UA-1 (ISO 14289-1), PDF 2.0 (ISO 32000-2, esp. §14.8 marked content /
`/ActualText`, and the 2.0 changes to `ActualText` and structure), and WCAG's PDF guidance:
what is **normatively required** about logical reading order, tag structure and `/ActualText`
for RTL documents, with clause numbers? What do accessibility checkers (PAC, Adobe's checker)
flag on a Persian document whose text is visual-order or splits ligatures? Is there any
requirement that makes *our* invariant a conformance obligation rather than a nicety?

**Impact:** the enterprise/accessibility tier's requirement list, and citable language for
the product's claims.
