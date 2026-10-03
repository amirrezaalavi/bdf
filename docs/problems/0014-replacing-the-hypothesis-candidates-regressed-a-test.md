# 0014 — Replacing the hypothesis candidates regressed a passing test

**Status:** attempt reverted; the lesson kept
**Date:** 2026-10-03
**Touches:** `crates/pdfrtl-core/src/text/recover.rs` (`settle_line_by_bidi`)

## What was attempted

Rung 3 tested exactly two hypotheses — the stored order, and `invert_units` (a whole-unit
reversal that keeps embedded left-to-right runs intact). Measured on real files, that space is
too small: a 9-unit line with **two** left-to-right runs (`1403/05/12` and `PDF`, plus a digit
inside parentheses) matches neither and refuses, and `arabic-3.pdf` refuses 9,878 lines for
exactly that reason.

The attempt **replaced** the two candidates with a different construction: classify each unit as
base-direction or not, group the *painted* order into maximal runs of each class, and emit the
order obtained by reversing the sequence of runs while keeping each run internally ordered.

## Why it was wrong

The old `invert_units` candidate was **not** a special case of the new one, and replacing it
removed the only candidate that produced the right answer for a date line. The diagnostic made
this unambiguous:

```
painted = [0, 1, 2, ..., 11]
keep    = Some([11, 10, 0, 1, ...])   <- identity predicts a reversed visual order
inv     = Some([0, 1, 2, ..., 11])    <- inverted predicts IDENTITY, which IS painted  ✅
```

So the correct answer was always "invert", and the new candidate set no longer contained it.
Result: a previously-passing test (`an_ltr_island_inside_rtl_is_decided`) began failing, while
the test the change was written for still failed.

Reverted with `git checkout -- crates/pdfrtl-core/src/text/recover.rs`; the committed RED tests
were kept as the acceptance criteria for a correct attempt.

## Why the gate did not stop it earlier

`no_silent_reversal` stayed **green** throughout. The invariant control watches for *emitting*
reversed text, and this change produced refusals, not reversals — it was safe in the sense the
invariant cares about, and wrong in the sense the tests care about. That is worth stating
plainly: **the invariant control is necessary and not sufficient.** A change that refuses more
than before is not caught by a control that only catches reversals.

## The rules

1. **Extend the candidate set; never replace it.** Every hypothesis that was ever accepted must
   remain in the space, or a case that used to be decided becomes a refusal. Additive only.
2. **Accept only on a unique match.** With more candidates the ambiguity risk rises, so
   "exactly one matched" becomes the load-bearing acceptance rule rather than a nicety.
3. **A refusal regression is a failure**, even when no reversal occurs and the invariant control
   is green.
4. **Bisect before implementing.** The three-unit fixtures pass today and the nine-unit one
   fails; that bisection is what located the boundary, and it was available *before* any of this
   code was written.

## Where the design question now lives

`docs/RESEARCH-QUESTIONS.md` **Q-R11** asks whether the correct formulation is to *solve* for
the order consistent with the painted positions rather than to *enumerate* candidates at all,
what production engines do, and what they do when nothing matches. The answer changes whether
this is a cheap one-pass computation or a search — which is why it is worth asking before
writing more code.