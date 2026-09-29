# 0011 — Two instruments that answered the same thing to both inputs

**Status:** both retired
**Date:** 2026-09-29
**Touches:** `scripts/classify-order-oracle.py` (research only, never shipped), a throwaway
`stream-vs-logical` probe, and the claims made from them

## What happened

Whether a file stores its text in logical reading order or in painted-visual order decides
whether the correct outcome for it is `Keep` or `Invert`. Two instruments were built to answer
that question for the 14 refused files. Both returned confident, tidy answers. Both could not
tell the two inputs apart.

**Instrument 1 — positions.** `classify-order-oracle.py` compared poppler `-raw` against
PDFium per-character boxes ("painted order") and labelled each page logical / visual /
undecided. It called 11 of the 14 files we order by producer name *logical*, which reads as
"we are inverting logical files" — an alarm about shipped code.

The decisive test — our emitted text scored against poppler's `-layout` logical reading and
against its reversal — showed all 14 emit **logical** text. Nothing was wrong. The labels were
confounded by exactly what `problems/0010` is about: a mirrored text matrix flips the painting
direction without changing the glyph sequence, so storage order cannot be inferred from
positions on a mirrored page.

**Instrument 2 — modes.** Its replacement compared poppler `-raw` ("stream order") with
poppler `-layout` ("logical") and reported `KEEP` for all 14 refused files. The control was
the 14 files we invert **and whose emitted text is verified logical** — for them the answer
must be `INVERT`. They came back `KEEP` as well. So `-raw` is not content-stream order for
RTL; poppler reorders in both modes, and an instrument that answers identically for a
visually-stored and a logically-stored file has no discriminating power. Its result is void.

## The rule

**An instrument must be shown to answer the other way on an input whose answer is known.**
A control that only confirms the direction you already expect is a mirror, not a control.

Earned alongside it, in the same session:

* Build the control *before* trusting the measurement, and choose one whose correct answer is
  the **opposite** of the one you are looking for.
* "Trusted tool plus plausible semantics" is not evidence. Both instruments rested on an
  assumed meaning of a third-party tool (`-raw` = stream order; per-char boxes = glyph order);
  neither assumption was measured.
* Exercise a new branch of a script through its new path before reading any result from it:
  the env-var path that selected the control died on a missing import, because only the old
  path had ever been run.
* No third-party tool decides storage order for RTL by itself (`docs/problems/0004`). The
  designated oracle stays **PDFium pixels plus a reading eye**; our own output can be judged
  for **order** against poppler's logical reading — that check does discriminate, and it is
  what cleared the 14 producer-ordered files.

## Consequence for the plan

The 14 refused files' storage convention is **undetermined**, and the plan does not need it.
A faithful prediction (`painted_map` per character) decides a line by reproducing the painted
sequence — self-contained evidence, no answer key. Verification is the invariant control, the
output-order check, and vision review on rendered pages.
