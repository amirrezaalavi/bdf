# 0010 — A mirrored text matrix is a measurement, not a missing one

**Status:** fixed
**Date:** 2026-09-29
**Touches:** `Walker::push_unit` (crates/pdfrtl-core/src/text/recover.rs)

## What happened

The rung refused every line whose text matrix was mirrored, and mirrored matrices are how
right-to-left producers normally paint. Measured on the private archive, `hebrew-3.pdf`
carries **169** such lines, and the Microsoft print/Excel family (7 of the 14 order refusals)
refuses on *every* page.

## Why

Two lines apart in `push_unit`:

```rust
let x_scale = self.ctm.a * self.line.a + self.ctm.c * self.line.b;   // composed horizontal scale
let paint_x = origin_x + self.ctm.a * self.pen;                      // projected with ctm.a only
x_ok: x_scale > 0.0 && origin_x.is_finite(),
```

1. The pen was projected with the CTM's `a` alone, ignoring the text matrix's own scale. Under
   a mirrored matrix (`-1 0 0 -1 … Tm`) that points the wrong way, so a run's *own advance*
   ordered it backwards — a measurement that was wrong, not absent.
2. `x_ok` read the *sign* of the composed scale as "x cannot order this line" and refused it.
   The position actually compared, `paint_x`, is projected with that same scale, so the sign
   cancels out of every comparison made against it. The refusal was never justified.

## Fix

Project the pen with the composed scale (`x_scale * self.pen`) and reserve `x_ok` for a scale
that is genuinely unusable — zero or non-finite. A mirrored run is then measured like any
other, and only a degenerate matrix leaves a line without a position.

## Evidence

* New test `a_mirrored_text_matrix_still_orders_its_line` (a logical-order line under a
  mirrored matrix, which must be decided rather than refused): **fails before** the fix
  (6 passed, 1 failed) and **passes after** it.
* Whole core suite green, **including** `no_silent_reversal` — this change does not relax any
  comparison, so the invariant control is unaffected. Contrast `docs/problems/0009`, where a
  change to the *comparison* was caught by that same control.

## How this was found

Not by reading the code, but by temporarily instrumenting the rung. For the length of this
investigation `settle_line_by_bidi` emitted one line per gated line over `stderr` — unit
count, pen span, which branch returned — and the tally showed mirrored matrices as a leading
cause rather than ties, which is what the theory had predicted. The instrumentation is **not
in the shipped code**: the core must not print, and the slop gate enforces it
(`core must not print; the CLI owns stdout`). The method is recorded here so it can be
re-added properly when wanted: the core should *return* the diagnostic lines and the CLI
should own writing them out. Anything that ships must emit counts and flags only, never a
unit's text — a diagnostic that can dump a customer's document is not one you can leave in a
binary or paste into a commit message.
