# 0015 — A perfect forward match is consistency, not proof of the original order

**Status:** accepted — changes what the pipeline may claim
**Date:** 2026-10-03
**Answers:** `docs/research/2026-10-03-external-answers-round2.md` (Q-R11), independently verified below
**Touches:** `Reason::BidiVerified`, `settle_line_by_bidi`, `CANARY.md`, `docs/decisions/`

## The finding

**There is no standard-defined procedure that recovers a unique original logical order from
painted positions in the general case.** UAX #9 defines the *forward* direction only; ICU
documents its inverse as approximate.

The decisive part is a counterexample, and it is reproduced here with **our own Unicode
implementation** (`unicode-bidi` 0.3 — the crate the product actually uses), not only in ICU:

| logical | levels | visual map | painted |
|---|---|---|---|
| `אבa 12` | `1 1 2 2 2 2` | `[2,3,4,5,1,0]` | `a 12בא` |
| `אב12 a` | `1 1 2 2 1 2` | `[5,4,2,3,1,0]` | `a 12בא` |

Measured, `unicode-bidi` 0.3: **same visual string, different characters, known paragraph
direction.** Every character is distinct, so this is not two histories producing an
indistinguishable rendering — they land on the *same identifiable painted characters*.

**Therefore:** given the painting, more than one logical order is compatible. Advance widths do
not help; neither does the absence of position ties. Geometry alone cannot decide.

## What this changes

`settle_line_by_bidi` reports `LineOrder::Keep` / `Invert` when a hypothesis *reproduces* the
painted order, and the caller records **`Reason::BidiVerified`** — a name that asserts the
painting *established* the order. After this finding that name overstates the evidence:

* "this candidate reproduces the painting" — **true, testable, and what we measure.**
* "the painting proves the original logical order" — **false in general**, by the counterexample.

So the *behaviour* is defensible; the *claim* is not. Two consequences:

1. **`Reason::BidiVerified` is a misnomer** and must be renamed or its documented meaning
   narrowed to "the stored order is consistent with the measured painting". The public JSON
   envelope carries this reason, so the rename is a contract change.
2. **Uniqueness within a candidate set is not uniqueness of the answer.** Accepting one match
   among generated candidates proves only that one member of *that* list fits. The answer's
   bounded-solver sketch makes this explicit: uniqueness is **model-relative**, and a search
   that hits its budget must refuse even if it has already found a match.

## What this does NOT change

* The invariant. Refusing is still correct, and the answer confirms our refusal policy is
  *stricter* than pdf.js, Poppler, MuPDF and PDFium — none of which expose an
  enumerate-and-refuse step in their bidi paths.
* The refusal vocabulary. `unsupported_visual_order` remains the right answer for E1.
* The ladder. `/ActualText` first is unchanged and remains the only true provenance.

## The rule

**Never let a consistency check be reported as a proof.** If a check confirms that a hypothesis
is *compatible with* the evidence, the reason code and the documentation must say exactly that.
This is the same family as the earlier lessons in this repo — a control that reports more than it
established (`problems/0006`, `0009`, `0011`, `0012`, `0014`), and here it is a *name* that
reports more than the code establishes.

## Verified here, not taken on trust

The research answer was checked before being believed, in the same Unicode implementation the
product uses. Reproducer: `~/verify-pdfrtl/verify-e1.py`. It also hit — and had to fix — the byte
vs character trap documented in `painted_map_for`: the *range* argument to
`reordered_levels_per_char` is in bytes, while the levels and the visual map are per character.