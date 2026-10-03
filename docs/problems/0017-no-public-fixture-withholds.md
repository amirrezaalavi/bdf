# 0017 — no tracked fixture withholds text, so the refusal path has no public coverage

**Date:** 2026-10-03
**Status:** open
**Found by:** writing the `--include-unproven` contract test
(`crates/pdfrtl-cli/tests/unproven_optin.rs`)

## What happened

The new test suite asserts that the default JSON envelope carries no withheld text and that
`--include-unproven` reveals it. I pointed it at `family-word-visual.pdf`, on the reasoning that a
"visual order" fixture must withhold.

Measured, over **all 14 tracked fixtures**:

```
corpus/raw/generated/chrome/all-sets.pdf             unproven_lines: 0
corpus/raw/generated/chrome/fa-plain.pdf             unproven_lines: 0
corpus/raw/synthetic/actualtext-fa.pdf               unproven_lines: 0
corpus/raw/synthetic/family-word-visual.pdf          unproven_lines: 0
corpus/raw/synthetic/family-indesign-visual.pdf      unproven_lines: 0
…                                          (0 on every one)
```

**Not one public fixture withholds anything.** Three of the four new tests would have passed
against a fixture that never withheld a character — which is the vacuous gate `problems/0006`
warns about, written by the very person who wrote that warning.

## Why

Every tracked fixture was built by `pdfrtl-gen` or Chrome, which write per-cluster `/ActualText`
and/or `/ReversedChars`. Those are rungs 1 and 2 of the ladder — *proven* — so they never reach
the rung-3 withholding path. The refusal path is only reachable on real producer output, which is
private.

So the public corpus has **zero coverage of the branch this project exists to exercise**, and the
only files that reach it cannot be published.

## Rule

**A test that asserts a property must first assert its own precondition.** "The default envelope
leaks nothing" measured against a file that withholds nothing is a statement about a code path
that never ran. The suite now carries `the_fixture_really_withholds` as a first-class test, and
the positive case skips **with a stated reason** when the private archive is absent rather than
passing quietly.

## What would fix it

A **public fixture that withholds** — a synthetic file whose RTL run is stored in visual order with
**no** `/ActualText` and **no** `/ReversedChars`, and whose geometry genuinely ties (so rung 3
cannot decide it). That is buildable with invented words and the existing `pdfrtl-gen` generator,
and it is the same artefact the critique asks for in P-5. Until it exists, the refusal path is
tested only on the private archive.

*Cross-reference: `docs/problems/0006` (a gate may not pass vacuously).*