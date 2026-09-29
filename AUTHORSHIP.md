# AUTHORSHIP.md — who made what, and why it is recorded

This file exists because the project is **dual-licensed** (ADR 0001). Selling a
commercial licence requires that we own, or have the right to sublicense, everything in
the shipped code. That makes authorship a legal artifact, not a courtesy.

## Roles

| Role | Who | Artifact |
|---|---|---|
| Design decisions, correctness judgement, review sign-off | human (Yolka) | ADRs in `docs/decisions/`, expected text in `corpus/expected/`, review files in `docs/reviews/` |
| Implementation, tests, refactors | agents | code + tests, committed with trailers |

The human-authored artifacts are deliberate: under current US Copyright Office guidance
(*Copyright and AI, Part 2*), material generated purely by AI, with insufficient human
control over the expressive elements, is not protected by copyright — and prompts alone
do not constitute sufficient control. Reviewable human expression (specifications,
ADR reasoning, curated fixtures with verified ground truth) is what keeps the work
protectable.

## Commit trailers (required on every commit)

```
Designed-by: Yolka <alavi2004@outlook.com>
Implemented-by: <agent name / model>
Reviewed-by: <human or agent, date>
```

Check a commit: `git log -1 --format=%B | grep -E '^(Designed|Implemented|Reviewed)-by:'`

## Rules

1. Agents may not mark work as reviewed by a human. `Reviewed-by: Yolka` is written by
   Yolka only.
2. Fixture ground truth (`corpus/expected/*.json`) records `verified_by` +
   `verified_at`. Unverified fixtures are labelled `status: unverified` and cannot be
   cited for a released claim.
3. Outside contributors must sign a CLA before merge (text TBD; no external
   contributions accepted before then).
4. If a fixture or file's provenance is unclear, record it as unclear in the manifest —
   never infer a licence.

## Open dependency

`docs/OPEN-QUESTIONS.md` Q-007 covers the provenance of the RTL work in
`yolka-wiz/al-bdf-engine`. Until it is answered, that repository is treated as
**reference reading only**: its *documented failure modes* inform our design, but no code
is copied into this repository.
