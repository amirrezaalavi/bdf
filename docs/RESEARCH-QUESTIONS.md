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

### Q-R11 — deciding stored RTL order when a line mixes SEVERAL left-to-right runs

**Priority: blocking the canary.** Nothing else in the order ladder moves until this is answered.

*Numbered Q-R11, not Q-R2, because round 1's Q-R2 is consumed and adjacent: it established that
"inverting a visual run" has **no standard-defined inverse** (level-based L2 over the visual
stream, with four traps). This question is about the step **before** inversion — deciding which
candidate logical order the measured positions support at all. Read both together.*

#### Context

`bdf` (`github.com/amirrezaalavi/bdf`) extracts text from Persian/Arabic/Hebrew PDFs. Its
invariant: **logical order, or an explicit refusal — never a silent reversal.** The order ladder
(`crates/pdfrtl-core/src/text/recover.rs`) is: `/ActualText` → `/ReversedChars` → **geometry
rung (rung 3)** → producer fingerprint → refuse (`unsupported_visual_order`).

#### What we know, all measured

* Rung 3 builds a line's **painted order** by sorting units by `paint_x`, where `paint_x` is the
  pen projected through the composed matrix — text-space advance accumulated from declared `/W`
  widths, projected once. Real lines carry **40–210 units**.
* A hypothesis is accepted only if `predicts_painted` holds: run UAX #9 (`unicode-bidi` 0.3)
  forward over the hypothesis's **real characters**, collapse the visual result back to unit
  positions, and reproduce the measured `painted` order exactly.
* Today only **two** candidates are tried: the stored order, and `invert_units` (whole-unit
  reversal keeping embedded left-to-right runs intact).
* Bisection from a 9-unit synthetic fixture (`crates/pdfrtl-core/tests/hypothesis_space.rs`):

  | line | result |
  |---|---|
  | 3 units, Persian + **one** Latin token | decided |
  | 3 units, same line stored visually | decided (inverted) |
  | 9 units: `تاریخ:` `1403/05/12` `PDF` `شماره` `25` `(ویرایش` `3` `)` `کتاب` | **refused**, empty text |

  So `invert_units` handles **one** embedded LTR run and fails on the second.
* Not a corner case: `arabic-3.pdf` (321 pages) refuses **9,878 lines** for this reason.
  Geometry there is healthy — `geometry=true`, ~480 pt spans, **0** mass ties, 177–178 distinct
  positions per 208-unit line — so the blocker is the **hypothesis space**, not the positions.
* The corruption we must never emit: `1403/05/12` must survive with digits in reading order;
  `21/50/3041` is the banned reversal.

#### What we tried and rejected — do not re-suggest

1. **Relaxing the tie test.** Measured dead: lines with **zero** ties are refused too.
2. **A more faithful per-character prediction.** Implemented and green (`8c62805`); `arabic-3.pdf`
   unchanged (9,878 → 9,878, 0/321 pages). Necessary, not sufficient.
3. **Replacing** the two candidates with a base/non-base run classifier. Attempted and reverted:
   it dropped the working `invert_units` candidate and regressed a passing test. Any change here
   must be **additive** — keep the existing candidates, add more, accept only on a **unique**
   match.

#### Questions

**Q-R11.1 (main).** What is the correct, standard-blessed way to decide, from measured painted
positions, whether a stored RTL line is logical or visual — **for lines containing several
left-to-right runs** (dates, digit runs, Latin words, punctuation)?

* Is the right formulation to **enumerate candidate logical orders and test each**, or to
  **solve** for the unique order consistent with the painted positions — e.g. assign each unit
  its UAX #9 level *from its painted position* and derive the permutation from that? If solving,
  what is the algorithm, and how is **uniqueness** established? Our acceptance rule is "exactly
  one candidate matched".
* How do production engines make this decision? Concretely requested, with `file:line`:
  **pdf.js** (`src/core/bidi.js`), **Poppler** (`TextOutputDev::reorderText`),
  **MuPDF** (`fz_bidi`, `bidi-std.c`), **PDFium**. Round 1 told us none of them is
  byte-faithful for RTL, but we have **not** asked how each decides *this* sub-question.
* Is there a formulation that avoids hypothesising altogether — comparing the **stored
  sequence's own UAX #9 level classes** against the painted order and accepting only if they
  agree? If so, exactly what is compared?

**Q-R11.2.** Inside RTL text, must a Persian date like `1403/05/12` be one unit at the base RTL
level, or a sequence of EN runs at level 2? Does the answer depend on adjacent characters under
UAX #9 W-rules — and can a producer's storage convention be inferred from positions alone?

**Q-R11.3.** What do engines do when the stored sequence matches **no** valid UAX #9 visual
output of the painted order? Refuse, trust a fingerprint, or emit the stored order anyway? We
forbid the third option; we want the precedent, because it tells us whether refusal is normal or
whether we are being stricter than every production engine.

#### Why the answer changes the design

If the approach is **solving**, the implementation is different and probably cheaper: one pass
assigning levels from positions. If it is **enumeration**, search-space and pruning dominate,
because a 210-unit line admits astronomically many candidates and the right one must be found
without exhausting them. Given round 1's finding that the standard defines no inverse, we also
want to know whether a *forward-only* check (predict-from-stored vs painted) can replace
inversion altogether.

#### What a usable answer looks like

* A named algorithm with its standard clause(s) or spec section.
* Pseudocode or a `file:line` reference for at least one production engine.
* An explicit statement of **when the answer is unique** — our invariant depends on it.
* What a real engine does in the "matches nothing" case.

**Out of scope:** OCR, generation, rendering, licensing, and whether any tool produces correct RTL
text — already measured as no (`docs/problems/0004`). We are not asking for an oracle, only for
the decision procedure.
