# 0009 — A weaker test is not a stronger measurement

**Status:** withdrawn (attempt preserved, not shipped)
**Date:** 2026-09-29
**Touches:** `settle_line_by_bidi`, `predicts_painted` (crates/pdfrtl-core/src/text/recover.rs)

## What was attempted

`settle_line_by_bidi` compared a hypothesised reading against the painted sequence with
**equality**: `painted_map(...) == Some(painted.to_vec())`. Any tie in `paint_x` made the line
`Ambiguous`, and on real files ties are the norm — `persian-7` tied on **311 of 311** gated
lines, `hebrew-4` on 754 of 770 — so ordinary Persian pages could not be recovered from
geometry at all, whatever the file declared. The decision fell through to the producer
fingerprint, and only files whose `/Producer` was already allow-listed could be emitted.

The attempt replaced equality with **consistency**: a reading is refuted only when a pair of
units that are *strictly* apart in `paint_x` appears in the opposite order in that reading.
Ties then constrain nothing and the partially-measured line still decides.

The unit tests went 7/8 → 8/8 after the change, including a new test asserting that a line
whose units all share one origin is *still* refused. On its own evidence the change looked
like a pure gain.

## What caught it

The invariant control, `crates/pdfrtl-core/tests/no_silent_reversal.rs` — the word-shape
check that ADR 0002 requires:

```
hebrew-4.pdf: found 1 occurrence(s) of the REVERSED form of "פניות" — silent reversal (ADR 0002/0004)
```

One real Hebrew word, emitted backwards, in a 51-file archive. That is the single outcome the
project's central invariant forbids.

## Why it happened

Equality had been doing an unintended job. Under equality a tie made the line undecidable, so
the unsafe reading could never win on a tied line:

* `keeps` — "the stored order, read as a logical paragraph, paints back consistently";
* `inverts` — the same claim for the inverted reading, gated on `stored_fits_paint`
  ("the producer stored what it painted").

Relax the comparison and `keeps` becomes reachable for a line whose producer *does* store
visual order: the tie mutes the reversal evidence, so the identity reading no longer
contradicts the measurement. `inverts` stays false — the gate is not satisfied, and the rival
reading fails a prediction — and `(keeps=true, inverts=false)` returns **Keep** on a
visual-stored line. The producer's own convention, allow-listed and measured, is overruled by
a weaker test.

## The real defect underneath

`painted_map` builds its proxy as **one character per unit**. A unit whose text is a whole
word collapses to a single bidi class, so the predicted painting loses the run structure
*inside* the unit — which is exactly where a mixed line (Persian plus digits, or Persian plus
Latin) diverges from a straight reversal. Consistency is only as sound as the prediction it
compares against, and that prediction was not faithful. Equality had been hiding the
unfaithfulness behind its own strictness.

## Rule

**Relax a comparison only after the prediction it is compared against is faithful.** Measure
the prediction first; loosening a test exposes whatever the test was accidentally protecting.
And when a change touches the order ladder, run the **whole** suite: the invariant control
lives in its own target, so a single-target run (`--test bidi_rung`) reports green while the
guard fails.
