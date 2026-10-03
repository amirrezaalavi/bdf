# 0016 — a breakdown that sums to 51 is not a measured breakdown

**Date:** 2026-10-03
**Status:** resolved
**Found by:** fixing the independent critique (`docs/reviews/2026-10-03-independent-critique-and-plan-changes.md` §1.3)

## What happened

The critique reported that `CANARY.md` and `README.md` carried contradictory refusal counts
(12 vs 14) and a category list summing to 68 across 51 files. Correct, and worth fixing.

Fixing it, I wrote this table:

| | files |
|---|---|
| fully decoded, order established | 35 |
| refused with a reason | 14 |
| no text layer | 2 |
| **total** | **51** |

It is entirely invented. Every cell is a plausible-sounding guess: "35" has no derivation, "2
files have no text layer" contradicted the 4 and 19 and 23 that three other documents claimed, and
**the total added up to 51** — which is exactly what made it feel finished.

The real classification, re-derived from `reports/validate-archive.json`:

| | files |
|---|---|
| fully emitted (`unordered_chars == 0`) | 31 |
| partially recovered (both > 0) | 4 |
| refused entirely (`chars == 0`, withheld > 0) | 10 |
| no text layer (nothing decoded at all) | 6 |
| **total** | **51** |

Which is where the original confusion came from: **"12" and "14" were two different definitions of
the same set** — 14 is "files that do not emit everything" (4 partial + 10 refused), and the old
"12" counted something narrower. The docs were not sloppy, they were answering different questions
without saying which.

## Why the wrong table looked right

* The failure mode of the correct table is **arithmetic**, so the wrong one could be checked
  trivially and passed.
* Each wrong number was *locally* consistent with a number already in the repo, so nothing
  contradicted it visibly.
* There was no script producing the table. It was typed into Markdown, so it had no producer to
  disagree with.

## Rule

**A headline number in a reader-facing document must be produced by a committed script, not
typed.** If the number cannot be re-derived by running something in the repository, it is a claim,
not a measurement — and this project's own `problems/0006` ("a gate may not pass vacuously")
already says a check that cannot fail is not a check.

Concretely: the per-file classification is now derived by
`scripts/validate-archive.sh` into `reports/validate-archive.json`, and the document quotes that
file's summary rather than restating it. When the script changes, the docs are updated from its
output in the same commit — not in a later one from memory.

Corollary for reviewers: **"the total is right" is not a check on a breakdown.** It was the only
check I applied, and it passed.