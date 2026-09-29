# Phase 2 — order recovery for real producers (plan)

Status 2026-09-29. Supersedes the "Recommended sequence" section of `docs/ROADMAP.md`, which
is now a pointer to this file. Reference: `docs/ROADMAP.md` (state), `docs/RESEARCH-QUESTIONS.md`
(what we cannot answer alone).

## Why this phase exists

Phase 1 landed honest extraction: the order invariant is enforced end to end, unproven pages
are withheld and counted, and the public CI gate is green under public conditions. What is
left is the hard half of the product's promise.

Measured on the 51-file real archive: **37 order-verified, 14 refused** (`unsupported_visual_order`),
and 23 files have no text layer at all. The 14 are the remains of the priority you set on
2026-09-27 — "implementing the correct search and reading is more critical".

## The two honest outcomes, and nothing between them

For each refused file we may only:

* **establish the order with evidence** (a named rule, recorded as a `Reason`), or
* **refuse loudly** (`unsupported_*`, exit 3, text withheld and counted).

No third option. A wrong reversal is undetectable to anyone who cannot read the script, which
is exactly why a heuristic guess is banned by ADR 0002.

## Workstreams

| # | item | state | acceptance |
|---|---|---|---|
| W1.7a | **Decide the signal for unknown/generic producers** | blocked on `Q-R1` | a written rule with its failure modes; where no rule exists, we keep refusing and say so in `docs/problems/` |
| W1.7b | **Rung 3 done properly** — UAX #9 comparison per run/line instead of an approximation | starting | unit tests over synthetic visual-order streams, including a Latin run and digits inside an RTL line |
| W1.7c | **Allow-list entries backed by tests** — every producer in the fingerprint list has a fixture asserting its order convention | starting | removing a fixture makes the control fail, not skip (`docs/problems/0005` is the precedent) |
| W1.7d | **Redistributable fixtures per producer family** | starting | a public fixture that exercises *inversion*, not just the `/ActualText` rung |
| W1.7e | **Re-run the 51-file validation** | after a–d | two numbers, never one: files decoded, files order-verified, plus the rule per file |
| W1.6 | **Font-aware recovery when `ToUnicode` is absent** | next | `hebrew-1.pdf` (29 pp, 9 fonts, 0 `/ToUnicode`, `Identity-H`) extracts its Hebrew, or refuses with a reason naming the glyph that defeated it; procedure per `Q-R4` |
| W2 | Reading order: line/run assembly, mixed scripts, digits/dates inside RTL, `he-eng-ar.pdf` | after W1.7 | fixtures byte-exact against `corpus/generated/SOURCES.md` and the corresponding human/vision read |
| W2.4 | Search normalization merge | branch landed | merged with `docs/SEARCH-NORMALIZATION.md` recording every rule and its rationale |
| W5.1 | **`lopdf` marked-content spike** | parallel, cheap | a written verdict: patch or rewrite, with the evidence that decides it (also `Q-R6`) |

## Acceptance test (write the test before the code)

Run the word-shape control over all 51 archive files plus the public fixtures:

* for a pure-RTL word of **four or more** letters, a correct output contains the logical form
  and **zero** occurrences of its character-reversed form (three-letter words give false
  positives: `דוח` matches inside `חודש`);
* every file reports **the rule that established its order**;
* the report prints **two** numbers — decoded, and order-verified. The second may not rise
  without the first.

A file where both forms appear means mixed per-run order; that needs its own `Reason`, not a
document-level verdict.

## What would make us stop and re-plan

* If `Q-R1` comes back "no reliable signal exists for producer family X", then X stays refused
  by design — the deliverable becomes a precise, documented refusal plus a support path for
  the user ("this document stores RTL in a way no evidence can resolve"), not a guess.
* If `W5.1` says `lopdf` drops marked content, the editor becomes a rewrite and its estimate
  changes; that decision belongs before any editor code exists.
* If the human review capacity becomes the bottleneck (it is the only oracle for reading
  correctness), we reduce fixture count rather than lower the standard.
