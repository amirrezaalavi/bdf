# Research questions — for an outside research agent

**What this file is.** A queue of knowledge gaps that cannot be closed locally. Each entry
states what we need, why it blocks us, what we already measured, and what a usable answer
looks like. Answers come back **in this file, under the question**, and are then consumed
and deleted by the project agent.

**Who answers.** An external research agent with web/library/code access. You are not
asked to agree with our design; you are asked to establish **facts with sources**.

**Status 2026-09-29: the queue is empty.** Round 1 (Q-R1…Q-R10) was answered externally; the raw
answers, our accept/reject decisions and what they changed are recorded in
[`docs/research/2026-09-29-external-answers.md`](../research/2026-09-29-external-answers.md).
The questions were consumed and deleted, as designed. `Q-R6` is kept below as a tombstone only so
the numbering stays stable and nobody re-asks it. New questions are appended near the bottom.

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
`/ActualText` → `ToUnicode` + a measured painting compared against UAX #9 levels →
allow-listed producer fingerprint (each entry backed by a test we ran) → refuse
(`unsupported_*`, exit 3). A page whose order cannot be established is **withheld** from the
text output and counted instead. Measured so far, on our own fixtures: Chrome/Skia writes
per-cluster `/ActualText` + `/ReversedChars`; poppler flips every lam-alef pair
(`سلام` → `سالم`); Xpdf returns visual order; PDFium drops ZWNJ and returns per-cluster
visual order.

Our 51-file real-world archive, passed at page level after modelling glyph advance widths: 37
files fully order-verified, 14 refused. Those 14 span **8 producer families**: `Microsoft: Print
To PDF` (4), `Microsoft® Excel® LTSC / 2016 / 2013` (3), `cairo` (2), `Foxit Reader PDF Printer`
(1), `Mac OS X … Quartz PDFContext` (1), `Adobe Acrobat Pro 11` (1), `ReportLab` (1), and one
file with no producer string at all. Structurally they are alike: `/Type0` + `/Identity-H`, a
`/W` array **is** present, `/ToUnicode` **is** present and maps to base Arabic letters, and there
is **no** `/ActualText` and **no** `/ReversedChars` anywhere — the painting is the only evidence
in the file. Page behaviour differs: most refuse on *every* page, one 321-page file order-refuses
319 of 321 pages (228 of which decode), and three refuse only a minority (1/3, 2/3, 9/21).

Measured 2026-09-29, and the reason the round-1 answers sharpened: for these files poppler's
`pdftotext -layout -enc UTF-8` returns coherent **logical** Persian while `-raw` returns the same
words **reversed** — the stored order is painted-visual, and the information needed to recover it
is present in the file. Details and definitions: `docs/problems/0001…0008`,
`docs/decisions/0002…0006`, `docs/ROADMAP.md`.

---

## Consumed — do not re-ask

| id | question | outcome | where it landed |
|---|---|---|---|
| Q-R1 | decide visual-vs-logical for unknown producers | accepted: the measured painting decides; poppler's rule is a run-level reversal (`TextOutputDev::reorderText`) | research digest, ADR 0006 |
| Q-R2 | inverting a visual run (UAX #9 in reverse) | accepted: level-based L2 applied to the visual stream + 4 traps; the standard defines no inverse | ADR 0006, tests to come |
| Q-R3 | producer behaviour table | **hypotheses only** — rows marked "None known"/"UNVERIFIED" cannot become allow-list entries | ADR 0006 |
| Q-R4 | font-aware recovery without `/ToUnicode` | accepted: cmap inversion + `post` names + NFKC base letters + fail loudly, via `ttf-parser`; the invented tie-break rejected | W1.6 in ROADMAP |
| Q-R5 | presentation forms in extraction output | accepted: base letters in output, NFKC for search | reason vocabulary |
| Q-R6 | `lopdf` and incremental editing | answered by our own spike (`7086526`) — tombstone below | spike, ADR at W5 |
| Q-R7 | writer stack for RTL shaping in Rust | accepted: `harfrust` + `krilla`/`pdf-writer`, all permissive | ROADMAP W4 |
| Q-R8 | standards obligations for logical order | accepted: PDF/UA-1 §7.2/§8.2.3, ISO 32000 §14.9.4, WCAG PDF3 | product claims |
| Q-R9 | the Microsoft print/export family | partial: visual storage, no marker, never emits `/ActualText`; its "refuse forever" recommendation rejected | ADR 0006 |
| Q-R10 | independent measurement / lexical scoring | accepted: PDFium per-glyph boxes (BSD-3); lexical scoring rejected (no prior art, no margin) | ADR 0006 |

---

## Q-R6 — tombstone (resolved locally, do not spend effort)

**Answered by our own spike, not by research** (commit `7086526`, `spikes/lopdf-marked-content/`):
marked content *does* survive both save paths, but only an **incremental** save
(`IncrementalDocument::create_from` + save) preserves untouched bytes — a plain `Document::save`
re-serialised 24 of 25 objects, collapsed object streams and degraded `Object::Real(f32)`
precision. Never mutate `Stream::content` with a stale `/Length`. This stops blocking W5; the
verdict becomes an ADR when W5 starts.

---

## Open

None. Append new questions here, one `Q-Rn` block each, in the shape described above.
