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


---

## Research answer to Q-R11 — 2026-10-03

### Executive finding

**There is no standard-defined procedure that recovers a unique original logical order from painted positions in the general case.** UAX #9 specifies the forward direction; ICU explicitly calls its inverse operation approximate. More decisively, the local counterexample below has two different logical strings with the same visual character identities under the same RTL paragraph direction. Consequently, a perfect forward match is a consistency check, not a uniqueness certificate. [S1, S2; experiment E1]

**Recommendation (my opinion):** keep the current candidates and add generators driven by the measured visual sequence, but strengthen what “unique” means. Accepting one match among a few generated candidates establishes uniqueness only within that candidate list. For the stated invariant, acceptance needs either trustworthy order metadata/provenance or a complete search in an explicitly supported model. A search that reaches its budget must refuse, even if it has found one match.

Scope: the supplied fixture counts and project behavior are taken from `RESEARCH-QUESTIONS.md`; I did not inspect the repository, run the canary, or independently reproduce its 9,878 refused lines. The external source findings below are code inspection, not end-to-end PDF extraction benchmarks.

### Q-R11.1 — decision procedure, multiple LTR runs, and uniqueness

#### What the standard provides

**Spec:** UAX #9 resolves levels from a logical paragraph, including weak and neutral context, then reorders individual lines. L2 reverses contiguous sequences at descending level thresholds. It does not specify assigning levels from measured PDF coordinates or recovering the original storage convention. Paragraph processing matters: treating each PDF line as a standalone paragraph is an extra model assumption. Relevant sections: §3, §3.3, §3.4/L1–L2, §4.3/HL1–HL5. [S1]

**Implementation:** ICU offers `UBIDI_REORDER_INVERSE_NUMBERS_AS_L`, `UBIDI_REORDER_INVERSE_LIKE_DIRECT`, and `UBIDI_REORDER_INVERSE_FOR_NUMBERS_SPECIAL`. Its documentation describes approximate inverse transformations; some modes/options insert LRM/RLM to ensure a round trip. That produces a display-equivalent representation, not evidence of the original characters or order. Use these as proposed hypotheses, never as a provenance oracle. Reject invented controls in a byte-faithful candidate. [S2]

#### E1 — actual ambiguity with no controls, known base direction, and unique characters

I ran ICU 74's `ubidi_setPara` with paragraph level **1**, no supplied embedding levels, and `ubidi_getVisualMap`. These are two different logical inputs:

| Logical code-point sequence | Resolved levels in logical order | Visual code-point sequence, left to right |
|---|---|---|
| `U+05D0 U+05D1 U+0061 U+0020 U+0031 U+0032` | `1 1 2 2 2 2` | `U+0061 U+0020 U+0031 U+0032 U+05D1 U+05D0` |
| `U+05D0 U+05D1 U+0031 U+0032 U+0020 U+0061` | `1 1 2 2 1 2` | `U+0061 U+0020 U+0031 U+0032 U+05D1 U+05D0` |

Readable forms, whose browser display should not be used to judge order: logical input 1 is `אבa 12`; input 2 is `אב12 a`. Both give visual `a 12בא`.

The visual maps, expressed as logical indices in left-to-right display order, were respectively `[2,3,4,5,1,0]` and `[5,4,2,3,1,0]`. Each character is unique, so both maps land on the **same identifiable painted characters**, not merely on an indistinguishable string. The numeric substring remains `12` in both.

**My deduction:** positions alone cannot distinguish these histories. Assign the same font metrics and per-character painted positions to the common visual sequence; the two possible logical histories remain compatible. Advance widths and lack of position ties do not resolve this particular ambiguity. A PDF stream may provide additional provenance, but geometry alone does not.

The example is a measured counterexample to universal uniqueness, not a claim that your particular Persian canary line is ambiguous. Reproduction code appears below. ICU API semantics are documented in [S2].

#### Exact forward-only check: useful, but only one direction

**Proposed comparison:** let `S` contain the stored units with persistent IDs. Expand their actual Unicode characters while retaining `(unit_id, character_offset)`. Resolve the forward UBA under an explicitly chosen paragraph/context model, apply line-level reordering, and compare the predicted visual ID sequence with the measured ID sequence.

Compare **permutations of IDs**, not just bidi classes, levels, or the resulting text. Repeated letters can otherwise conceal a wrong permutation. Levels alone do not identify the corresponding glyph positions.

If the match holds, `S` is *consistent with being logical*. If it fails, `S` is inconsistent with that model; it is not thereby proven visual, because the producer may have used another paragraph context, explicit formatting, segmented layout, or another ordering convention. [Model inference; the context mechanisms exist in S1 §4.3.]

This check can remain the fast path. It cannot recover a visual stream, and E1 shows that a passing stored candidate need not exclude a different passing candidate. Calling it “order verified” requires the additional assumption that the supported candidate family covers the possible producer histories.

#### Additive candidate generation that handles several LTR islands

**Proposal, not a Unicode inverse standard:** generate from the measured visual sequence `V`, not solely from `reverse(stored)`. Distinguish three operations:

1. Reverse the order of all painted units.
2. Restore the internal order of every hypothesized even-level island.
3. Run the existing full forward verifier on each result.

The difficult part is the island boundaries. Separate Latin tokens, numbers, spaces, and punctuation can share one resolved even-level island; splitting at every token or joining every non-Arabic token is not justified. E1's space has level 2 in one history and level 1 in the other. Neither “reverse RTL letters only” nor a base/non-base classifier captures all possibilities.

Keep `stored` and the working `invert_units` result. Add hypotheses for plausible island boundaries, including a contiguous date and adjacent Latin/date sequences. Deduplicate by recovered logical text and, when required for geometry verification, by the ID permutation. Multiple generators producing the same answer do not establish ambiguity.

An ICU inverse result can supply another seed, provided its ID mapping is preserved and no characters are silently added, removed, mirrored, or normalized before the permutation check. [Proposal informed by S2.]

**UNVERIFIED:** whether any of these seeds recovers the actual 9-unit fixture or `arabic-3.pdf`. I did not have their complete character payloads, stored IDs, geometry, unit boundaries, and paragraph context. The token labels alone do not define a reproducible forward permutation.

#### A complete restricted solver: enumerate levels, not arbitrary unit permutations

This is a **derived algorithm proposal**, not a production-engine precedent or standard-blessed inverse. Its completeness is conditional on the model you support.

Define a deliberately narrow model first, for example: one RTL paragraph on one line, no lost formatting controls or external embeddings, exact decoded characters, and units that are valid atomic reordering clusters. A model permitting only resolved levels 1 and 2 is a useful first target; it must explicitly exclude inputs needing additional levels or context.

```text
matches = set()

for visual_level_assignment in COMPLETE_assignments_for_supported_model(V):
    pairs = zip(V.ids, visual_level_assignment)

    # Invert the L2 reversal operations, carrying levels with identities.
    # Forward L2 visits thresholds high -> low; undo in reverse order.
    for threshold in ascending(lowest_odd_level, highest_level):
        reverse_each_maximal_segment(pairs, level >= threshold)

    candidate_ids = ids(pairs)
    candidate = original_character_payloads(candidate_ids)
    result = full_forward_UBA(candidate, model.paragraph_context,
                              model.line_boundaries)

    if result.visual_ids != V.ids:
        continue
    if result.visual_levels != visual_level_assignment:
        continue
    matches.add(candidate.logical_text)
    if count(matches) >= 2:
        refuse(ambiguous_visual_order)

if search_budget_exhausted:
    refuse(order_search_incomplete)
else if count(matches) == 0:
    refuse(unsupported_visual_order)
else:
    accept(the_only_match, supported_model_id)
```

**Why this is complete within the model (my argument):** any valid forward result has a resolved visual level assignment. A complete enumerator visits that assignment; undoing its known reversal operations recovers its logical permutation. Re-running the forward algorithm rejects assignments that cannot arise from the real characters and context. This argument depends on faithfully modelling L1, removed controls, clusters, mirroring, and line/paragraph boundaries; the sketch covers the permutation core only.

For a 1/2 model, inversion simplifies to reverse the whole line and reverse each resulting level-2 island. If you can prove certain characters' levels fixed in that model, search only unresolved boundaries/neutral spans. At character level, RTL strong letters are ordinarily at 1 and LTR letters/numbers at 2 in this narrow model; **do not transfer that simplification blindly to a unit containing mixed characters or to explicit embeddings**. [Model deduction from S1 I2; this is not a general classifier.]

This changes the search variables, not the worst-case difficulty. With `k` independent binary level choices there are still `2^k` assignments. Coalescing spans or pruning is safe only with a proof that it retains every supported solution. There is no verified linear-time, geometry-only uniqueness solver in the inspected sources.

**When acceptance is unique:** exactly one distinct output survives a **completed** search over every candidate in the stated supported model. That is model-relative uniqueness. To claim the original order, you also need evidence that the producer/document belongs to that model. A single match in a partial search is insufficient; no-match means no solution *in that model*, not no conceivable Unicode rendering history.

**Impact: Keep the two existing candidates; add visual-sequence generators and an explicitly bounded solver, and distinguish consistency, model-relative uniqueness, ambiguity, and incomplete search.**

### Production engines — the requested code paths

All line numbers below are source-file line numbers in the pinned revisions linked in the references. I fetched the files and checked that the initially inspected snapshots matched those revisions. The PDFium official snapshot is handled separately from its lagging GitHub mirror.

| Engine | Actual inspected decision/reordering behavior | Relation to your strict invariant |
|---|---|---|
| pdf.js | `bidi(str, startLevel, vertical)` gets characters, not per-character measured positions. Automatic base choice uses a 30% RTL threshold for sufficiently long strings. It implements a UBA subset, skips most embedding handling, then reverses level segments. `runBidiTransform` passes the accumulated chunk string. [S3, S4] | This path does not compare alternative logical hypotheses against painted geometry or certify uniqueness. |
| Poppler | `reorderText` scans the supplied sequence using strong RTL/LTR and numeric classification. In RTL-primary mode it scans from the end, emitting RTL sections backwards and LTR sections forwards, repeatedly. It can insert directional embeddings in encoded output. `primaryLR` uses a page-wide strong-character balance. [S5] | Multiple LTR sections are handled procedurally, not by proving an inverse or counting exact geometry matches. |
| MuPDF | PDF interpretation uses `guess_bidi_level` to supply directional hints. Structured-text extraction compares pen motion against expected logical/visual advances, flags likely visual RTL characters with `bidi = 3`, and later reverses marked bidi spans. [S6, S7] | It provides a concrete geometry heuristic, but not a global uniqueness test for mixed-run logical candidates. |
| PDFium | `CFX_BidiString` groups directional segments and establishes an overall direction. `CloseTempLine` emits/reverses segments based on direction; the current official path has special handling for logical `/ActualText`. [S10, S11] | The inspected bidi stage emits a chosen transformation; it does not implement your exact-forward-match and refusal contract. |

**pdf.js details:** `src/core/bidi.js:120–193` covers classification/base selection; `:389–424` covers level reversals. `src/core/evaluator.js:2690–2705` is the chunk caller. The bidi function has no geometry argument. [S3, S4]

**Poppler pseudocode, paraphrased from `TextOutputDev.cc:206–318`:**

```text
if primary direction is RTL:
    start at the end of the supplied sequence
    while characters remain:
        emit backward until an LTR letter or number is encountered
        collect backward until an RTL letter is encountered
        emit that collected LTR section forward
```

This can process several LTR sections. Its numeric treatment and placement of neutral characters are implementation choices, not a uniqueness theorem. Direction selection is at `:3311–3327`. Note that the function is a file-local `reorderText` in this snapshot, rather than a `TextOutputDev::reorderText` member. [S5]

**MuPDF distinction:** `fz_bidi_fragment_text`/`create_levels` in `bidi.c:687–780,806+` and `bidi-std.c` implement forward level resolution/reordering infrastructure. They are not themselves the PDF geometry decision procedure. For PDF extraction, inspect `pdf-op-run.c:1303–1333,1445–1459`, then `stext-device.c:905–960,523–553,1773–1810`. Conflating these two paths would overstate what `fz_bidi` proves. [S6–S9]

**PDFium source locations:** official `core/fxcrt/fx_bidi.cpp:18–106` covers direction classification, segment counts, and segment-order reversal; `core/fpdftext/cpdf_textpage.cpp:815–873` covers output, including the `/ActualText` exception. Overall direction uses a strict `nR2L > nL2R` comparison in this official revision. [S10, S11]

**PDFium freshness caveat:** the `chromium/pdfium` GitHub mirror I fetched contains an older variant. The official Gitiles main snapshot already includes `/ActualText` RTL changes; I cite its source separately. Do not use a moving branch URL or this report's mirror revision as evidence of today's official implementation. [S10, S11]

### Q-R11.2 — dates and numeric levels

**Spec:** ASCII digits have initial type EN; Persian extended Arabic-Indic digits also have EN, whereas Arabic-Indic digits have AN. Solidus `/` has CS. These are Unicode character properties, not resolved levels. [S12; local property check agrees.]

W2 can change EN to AN after an Arabic strong character. W3 then changes AL to R. W4 absorbs a CS between equal numeric types into that numeric type. W7 can change remaining EN to L after L. In an unembedded RTL paragraph, I2 puts the resulting numbers/L characters at level 2. Thus an ordinary date is an internally LTR numeric span, not a base-level RTL atomic character. Adjacency/context affects the resolved type; explicit formatting can change the applicable embedding level. [S1 W2–W7, I2.]

**Measured E2, ICU 74, base level 1:** in `تاریخ: 1403/05/12`, all ten date characters, including both slashes, resolve to level 2 and retain their internal order. The same holds for `تاریخ: ۱۴۰۳/۰۵/۱۲`. In `PDF 1403/05/12`, every character in that string resolves to level 2. These are measured examples, not a universal statement about dates in arbitrary embeddings.

**My implementation advice:** a date can be an atomic extraction unit *for preserving its payload*, while its actual characters remain available to the forward predictor. Do not assign that unit level 1 simply because the surrounding prose is Persian. Nor should “date” require calendar parsing; preserve the literal numeric/separator sequence.

**Can positions reveal the producer's convention?** They reveal the painted arrangement and can refute a proposed convention. They do not generally identify one originating convention; E1 already supplies two compatible histories. Local monotonic advances can provide a heuristic like MuPDF's, but that is different from globally unique recovery. [E1; S7.]

**Impact: Preserve numeric payloads; let full character/context resolution determine levels, and do not treat a date as intrinsically base-RTL.**

### Q-R11.3 — what happens when nothing matches?

The inspected engines do **not** expose your proposed “enumerate logical candidates → forward-test against geometry → refuse if none” step in these bidi paths. Therefore they have no directly corresponding “zero valid candidates” branch to cite. They perform their chosen transform and continue extraction; MuPDF's motion heuristic may also split lines on large unexpected jumps. That does not mean every error everywhere in these engines is ignored. [S3–S11, the paths identified above.]

No inspected path uses a producer allow-list as the fallback for this subproblem. **UNVERIFIED:** a claim that no other engine module/version contains any producer-specific workaround. This report inspected the relevant ordering paths, not every source file in every release.

**Finding:** your explicit order-based refusal policy is stricter than these inspected extraction paths. Their output is precedent for practical heuristics, not evidence that every painted line has a unique original logical representation. Coherent Poppler output on your archive shows that its heuristic is useful there; it does not independently establish uniqueness or byte fidelity.

**Impact: Keep refusal on zero solutions, ambiguous solutions, and unfinished search; use engine behavior for hypothesis generation and adversarial tests rather than as an order oracle.**

### My opinion on the project design

Your refusal policy is defensible. The part I would change is the meaning attached to a successful forward check. “This candidate reproduces the painting” is a strong, testable claim. “The painting proves the original logical order” is stronger, and E1 shows why the first does not imply the second.

I would make the next canary experiment small and reviewable:

1. Log one refused real line with exact stored IDs, character payloads, painted IDs, unit boundaries, widths, and surrounding paragraph context. Retain it as a fixture.
2. Preserve the existing two candidates and their tests. Add candidates derived from visual level islands; treat Poppler/ICU transforms as seeds.
3. Add E1 as an ambiguity fixture. Both outputs must pass the geometry predictor; the strict path must refuse rather than prefer whichever generator runs first.
4. For tiny fixtures, exhaustively enumerate all unit permutations and independently compare the bounded solver's solution set. This is an oracle for testing completeness of the restricted search, not an algorithm for 210-unit production lines.
5. Add separate outcomes for no supported solution, multiple distinct solutions, insufficient paragraph/model evidence, and search budget exhausted. If the public vocabulary must stay stable, retain those distinctions internally.
6. Only then measure the recovered/refused counts across the canary. Report how many successes have complete model-relative uniqueness and how many merely have one known matching hypothesis.

I would avoid a fingerprint saying “this producer is always visual” unless measured fixtures support the particular convention you rely on. Likewise, I would not infer one global storage convention from a few RTL character transitions: PDF streams can paint different text objects in different orders.

**UNVERIFIED:** a practical search bound sufficient for your 40–210-unit lines, or an existing complete inverse solver suitable for direct Rust adoption. The bounded solver above needs implementation, differential checks, and profiling before it can be recommended as a canary fix. No claim in this report guarantees a recovered-page count.

### Reproducing E1 and E2

This minimal Python probe uses ICU 74 installed in the research environment. For another ICU version, change the symbol suffix. It performs a forward bidi calculation only; it does not shape or render a PDF.

```python
import ctypes as C
import ctypes.util

lib = C.CDLL(ctypes.util.find_library("icuuc"))

def bind(name, args, result=None):
    f = getattr(lib, name + "_74")
    f.argtypes, f.restype = args, result
    return f

open_bidi = bind("ubidi_open", [], C.c_void_p)
close_bidi = bind("ubidi_close", [C.c_void_p])
set_para = bind("ubidi_setPara", [
    C.c_void_p, C.POINTER(C.c_uint16), C.c_int32, C.c_uint8,
    C.c_void_p, C.POINTER(C.c_int)])
visual_map = bind("ubidi_getVisualMap", [
    C.c_void_p, C.POINTER(C.c_int32), C.POINTER(C.c_int)])
level_at = bind("ubidi_getLevelAt", [C.c_void_p, C.c_int32], C.c_uint8)

# These examples use only BMP code points, so one UTF-16 unit per character.
p = open_bidi()
try:
    for s in ["\u05d0\u05d1a 12", "\u05d0\u05d112 a",
              "تاریخ: 1403/05/12", "تاریخ: ۱۴۰۳/۰۵/۱۲",
              "PDF 1403/05/12"]:
        src = (C.c_uint16 * len(s))(*map(ord, s))
        err = C.c_int(0)
        set_para(p, src, len(s), 1, None, C.byref(err))
        indices = (C.c_int32 * len(s))()
        visual_map(p, indices, C.byref(err))
        assert err.value <= 0, err.value
        visual = "".join(s[i] for i in indices)
        print([f"U+{ord(c):04X}" for c in s])
        print(list(indices), [level_at(p, i) for i in range(len(s))])
        print([f"U+{ord(c):04X}" for c in visual])
finally:
    close_bidi(p)
```

### Sources and pinned code references

Source URLs below are ordinary links so this Markdown remains useful outside ChatGPT. “Spec,” “implementation,” “measured,” “proposal,” and “UNVERIFIED” above deliberately distinguish different kinds of evidence.

[S1]: https://www.unicode.org/reports/tr9/tr9-52.html
[S2]: https://unicode-org.github.io/icu-docs/apidoc/released/icu4c/ubidi_8h.html
[S3]: https://github.com/mozilla/pdf.js/blob/ef62f31f2486d7cf2c3854b2db4520b9ca389b2f/src/core/bidi.js
[S4]: https://github.com/mozilla/pdf.js/blob/ef62f31f2486d7cf2c3854b2db4520b9ca389b2f/src/core/evaluator.js
[S5]: https://github.com/tsdgeos/poppler_mirror/blob/03e8c2c9696c3f556f9ad7fbe9e5177907d625ef/poppler/TextOutputDev.cc
[S6]: https://github.com/ArtifexSoftware/mupdf/blob/d0f5d7f37cd0dceabc88c6bd8b9080a5cb951530/source/pdf/pdf-op-run.c
[S7]: https://github.com/ArtifexSoftware/mupdf/blob/d0f5d7f37cd0dceabc88c6bd8b9080a5cb951530/source/fitz/stext-device.c
[S8]: https://github.com/ArtifexSoftware/mupdf/blob/d0f5d7f37cd0dceabc88c6bd8b9080a5cb951530/source/fitz/bidi.c
[S9]: https://github.com/ArtifexSoftware/mupdf/blob/d0f5d7f37cd0dceabc88c6bd8b9080a5cb951530/source/fitz/bidi-std.c
[S10]: https://pdfium.googlesource.com/pdfium/+/190a9673fe7dd888dc26734b7159976edd18f514/core/fxcrt/fx_bidi.cpp
[S11]: https://pdfium.googlesource.com/pdfium/+/190a9673fe7dd888dc26734b7159976edd18f514/core/fpdftext/cpdf_textpage.cpp
[S12]: https://www.unicode.org/Public/UCD/latest/ucd/UnicodeData.txt

- **[S1] [UAX #9, revision 52](https://www.unicode.org/reports/tr9/tr9-52.html):** forward algorithm, contextual rules, line reordering, higher-level protocols. Pin your implementation's actual Unicode tables separately; this report does not assume `unicode-bidi 0.3` implements this revision.
- **[S2] [ICU C bidi API](https://unicode-org.github.io/icu-docs/apidoc/released/icu4c/ubidi_8h.html):** `ubidi_setInverse`, `ubidi_setReorderingMode`, `UBIDI_OPTION_INSERT_MARKS`, visual maps and levels. The documentation retrieved identifies itself as ICU 78.3; the local experiment uses ICU 74.
- **[S3] [pdf.js bidi.js](https://github.com/mozilla/pdf.js/blob/ef62f31f2486d7cf2c3854b2db4520b9ca389b2f/src/core/bidi.js)** and **[S4] [evaluator.js](https://github.com/mozilla/pdf.js/blob/ef62f31f2486d7cf2c3854b2db4520b9ca389b2f/src/core/evaluator.js)**, commit `ef62f31f2486d7cf2c3854b2db4520b9ca389b2f`.
- **[S5] [Poppler TextOutputDev.cc](https://github.com/tsdgeos/poppler_mirror/blob/03e8c2c9696c3f556f9ad7fbe9e5177907d625ef/poppler/TextOutputDev.cc)**, commit `03e8c2c9696c3f556f9ad7fbe9e5177907d625ef`, from the `tsdgeos/poppler_mirror` mirror. Upstream project: https://gitlab.freedesktop.org/poppler/poppler.
- **[S6] [MuPDF pdf-op-run.c](https://github.com/ArtifexSoftware/mupdf/blob/d0f5d7f37cd0dceabc88c6bd8b9080a5cb951530/source/pdf/pdf-op-run.c)**, **[S7] [stext-device.c](https://github.com/ArtifexSoftware/mupdf/blob/d0f5d7f37cd0dceabc88c6bd8b9080a5cb951530/source/fitz/stext-device.c)**, **[S8] [bidi.c](https://github.com/ArtifexSoftware/mupdf/blob/d0f5d7f37cd0dceabc88c6bd8b9080a5cb951530/source/fitz/bidi.c)**, **[S9] [bidi-std.c](https://github.com/ArtifexSoftware/mupdf/blob/d0f5d7f37cd0dceabc88c6bd8b9080a5cb951530/source/fitz/bidi-std.c)**, commit `d0f5d7f37cd0dceabc88c6bd8b9080a5cb951530`.
- **[S10] [PDFium fx_bidi.cpp](https://pdfium.googlesource.com/pdfium/+/190a9673fe7dd888dc26734b7159976edd18f514/core/fxcrt/fx_bidi.cpp)** and **[S11] [cpdf_textpage.cpp](https://pdfium.googlesource.com/pdfium/+/190a9673fe7dd888dc26734b7159976edd18f514/core/fpdftext/cpdf_textpage.cpp)**, official Gitiles commit `190a9673fe7dd888dc26734b7159976edd18f514`.
- **[S12] [Unicode character property data](https://www.unicode.org/Public/UCD/latest/ucd/UnicodeData.txt):** U+0030–0039, U+002F, U+0660–0669, U+06F0–06F9. Initial bidi class is field 5 (zero-based field 4). Local Python `unicodedata.bidirectional` was also checked for representative characters; it agreed.
