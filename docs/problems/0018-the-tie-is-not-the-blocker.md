# 0018 — the tie is not the blocker; my solver was decoration

**Date:** 2026-10-03
**Status:** open (the underlying problem is unsolved; the approach below is withdrawn)
**Found by:** attempting the critique's P-7 (extend rung 3's hypothesis space)

## What I tried

`settle_line_by_bidi` refuses a line the moment any two units share a `paint_x`, because a tie
means the painted order was never *measured* — `painted` fell back to stream order, which is the
very thing being interrogated. That refusal is correct (docs/problems/0005, 0007).

I assumed the tie was the blocker for the 8 refusing Persian files and wrote `solve_tied_line`:
three candidates (stored, `invert_units`, and a new `reverse_by_script`), each accepted if it is a
*well-formed RTL line under UAX #9*, and a line decided only when exactly one candidate qualifies.

## Why it did not work

**My discriminator was vacuous.** "Is this reading a well-formed RTL line?" accepts every candidate
equally well — all three are permutations of the same units and all three render. So uniqueness
never held, every tied line stayed `Ambiguous`, and the measured numbers did not move by one
character:

```
                    before   after
8 Persian files     0 emitted, unchanged exactly
acceptance tests    3 passed, 2 failed — unchanged
whole suite         17 suites ok, 0 failed
```

I reverted it. Shipping a solver that adds a candidate generator, a budget constant and a
per-line cost while deciding nothing is decoration wearing a commit message.

## The finding that matters — the tie is NOT the blocker

The critique's plan (C3b, and my own acceptance test) says the Persian files refuse because their
units tie at one origin. **Measured with a real parser (`pypdf`, not a regex sweep): 7 of the 8
refusing Persian files declare `/W` on every Type0 font**, so the pen advances and their units do
NOT all share one origin — they never reach the tie branch:

| file | fonts | Type0 | with `/W` |
|---|---|---|---|
| persian-1 | 54 | 54 | 54 |
| persian-2 | 5 | 5 | 5 |
| persian-3 | 11 | 11 | 11 |
| persian-4 | 12 | 5 | 5 |
| persian-6 | 9 | 5 | 5 |
| persian-panel1 | 6 | 6 | 6 |
| persian-report-noc-revision | 108 | 108 | 108 |
| **persian-hld-7-summary-fa** | 9 | **0** | **0** |

So the tie refusal is *correct* and is the reason for **one** file, not eight. The real blocker is
the one already recorded in `docs/plans/2026-10-03-canary.md` C3a: lines whose painted form
**neither** hypothesis reproduces — measured at 842 / 1,347 / 340 lines on `arabic-1/2/4`, which
decide 1,778 / 1,956 / 2,706. That is the `neither` bucket, and it needs a hypothesis that is not a
whole-unit reversal.

## Rule

**A widening that decides nothing is not a partial success — it is a failed experiment.** Measure
the corpus before and after and require the number to move; "no regressions, suite green" is the
signature of a change that does nothing, and I was about to commit exactly that as progress.

Related, and the deeper lesson: **I chose the blocker by assumption rather than by measurement.**
The width measurement (`/W` declared on 8/8 files) was already in the repo, in C3b's own table.
I read the plan, saw "tie", and built for the tie.

## What the next attempt needs

A candidate that reproduces a *mixed* line's painting, where the levels of each island's
neutrals (space, `/`, `-`, parentheses) differ from every whole-unit reversal. Per the round-2
research the branching variable is "the level of each neutral span", not each unit — and that claim
must be checked against an exhaustive-permutation oracle on short lines **before** it is believed,
exactly as the critique says ([OPINION] in its §P-7.2, not yet tested).

Do not build it until the oracle exists.