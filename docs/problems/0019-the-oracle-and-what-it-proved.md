# 0019 — the oracle, and what it actually proved

**Date:** 2026-10-03
**Status:** resolved (the oracle is built and cross-checked); the redirect it caused is in §4
**Superseded conclusion:** the first version of this file claimed geometry admits two orders even
for distinct positions. **That was wrong**, and the oracle's cross-check is what caught it.

## What the oracle is

`crates/pdfrtl-core/src/text/oracle.rs` — *"how many logical orders does a line admit, given its
measured positions and its script classes?"* Two independent implementations:

* `admissible_orders_closed_form` — a product over the sizes of the groups of units that share a
  painted position, times a direction factor;
* `admissible_orders_by_enumeration` — generate every permutation (≤ 8 units), and keep the ones
  whose renderer walk lands each unit where the measurement put it.

`closed_form_agrees_with_enumeration` asserts they agree for **every tie pattern** up to n = 5 and
for five representative shapes at n = 6…8. One implementation would make "the oracle agrees with
the code" unfalsifiable — the same defect as verifying a gate with the code it gates.

## Three bugs the cross-check caught, in order

**1. The enumeration was vacuous.** It painted a candidate by sorting units by `x` and reversing
by direction — which depends only on the *multiset* of positions, not on the candidate's own
sequence. Every permutation therefore painted identically and every order counted as admissible,
so two units at *distinct* positions reported **2**. Fixed by making admissibility
sequence-sensitive: walking the candidate the way a renderer traverses a paragraph, each next
unit must sit at the next measured position *in the direction the paragraph runs*.

**2. The tie-break used the unit index, not the candidate's rank.** Subtle version of the same
defect: sorting by `x` and breaking ties by `u < v` erased the candidate entirely.

**3. The direction factor was a sum where it should be a union.** With four units all at one
position and no strong character, the closed form said `2 × 4! = 48` and the enumeration said
`4! = 24`. Hand-checked: when one tie group spans every unit, `walk_matches` already holds for
every order under **both** directions, so the direction choice excludes nothing and the union is
the same 24 orders. Multiplying double-counted it.

## What the oracle actually says

| line | admissible orders | unique? |
|---|---|---|
| units at **distinct** positions, strong RTL present | `1` (product of `1!`) | **yes** |
| units at distinct positions, **no** strong character (UAX #9 P3 inherits an unmeasured level) | `2` — one per direction | no |
| one tied pair among distinct units | `2` | no |
| k units tied at one position | `k!` | no unless k = 1 |

So my first claim was wrong on the important case: **with the base direction fixed by the line's
own characters, distinct positions really do admit exactly one order.** The ladder's tie check is
sound, and `settle_line_by_bidi` is right to refuse a tie.

The genuine limit is narrower and sharper than I first wrote: **a line with no strong character
(anything but strong RTL/LTR — all digits, all neutrals) has an unmeasured base direction**, so
geometry admits two orders even when every position is distinct. That is a real class of line: a
page that is nothing but `1403 / 05 / 12` and punctuation.

## What this means for the remaining work

* The `neither` bucket (the 8 refusing Persian files) is **not** explained by the base direction —
  those lines contain Persian text, so their direction is fixed. Their blocker remains what
  `problems/0018` measured: lines whose painting neither existing hypothesis reproduces.
* The research's proposed level-enumeration solver (critique §P-7.2) is **not invalidated** by
  this, because it assumes the base direction is known, and for the Persian corpus it is.
* A new, smaller finding worth a test: **digit-only and punctuation-only lines are ambiguous even
  when perfectly positioned.** The ladder currently decides them (positions distinct, direction
  inferred from the candidate). That is a place where a decision rests on an assumption rather
  than a measurement. Recorded for the next lane; not fixed here.

## Rules this exercise earned

1. **An oracle's first disagreement is usually the oracle correcting you — but verify which side is
   wrong before agreeing.** I twice assumed the oracle was right, and once it was, and once it was
   my *expectation* that was wrong. Working the case out by hand (`RTL walk admits only [1,0]`,
   `LTR walk admits only [0,1]`) settled in one step what three rounds of patching had not.
2. **State the expected arithmetic in the test, in the comment.** Two of the three bugs were found
   by tests that said `// 2!·2!·2! = 8` rather than merely "the two implementations agree".
3. **A test that only asks two implementations to agree cannot tell you which is wrong.** It caught
   a disagreement; it did not tell me the answer. The hand-check did.