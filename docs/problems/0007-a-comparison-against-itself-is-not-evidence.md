# 0007 — a comparison against itself is not evidence: rung 3 silently reversed a line

**Status:** fixed 2026-09-29 · **Found by:** `crates/pdfrtl-cli/tests/cli_contract.rs` (the invariant
control) · **Severity:** silent wrong text — the worst class this project has, because nothing
downstream can tell it apart from correct text.

## What happened

On a 636-byte probe PDF — two `/ActualText` clusters on one line, no `/ReversedChars`, producer
`pdfrtl-test` — extraction returned:

```
exit 0 · ok true · data.text "لس" · reasons [actual_text, bidi_reordered, bidi_verified] · unordered_chars 0
```

The line is **provably undecidable**: with two units the two hypotheses are exact mirrors of each
other, so exactly one of them always "matches" and the match carries no information. The correct
outcome is refusal — `exit 3`, `data.text ""`, `unordered_chars 2`, a page reason of
`unsupported_visual_order` — which the control demanded and the code did not deliver.

## Why it existed

`settle_line_by_bidi` builds the painted order by sorting the units on their painted `x`, breaking
ties by stream index. Ties are not a corner case: a unit's `x` comes from its **text origin**
(`Td`/`Tm`) — `pdfrtl` does not model glyph advance widths — so any page that leaves several
clusters at one origin ties. And when it ties, `painted` **is** stream order: the hypothesis is fed
the very thing the rung exists to interrogate.

For a pure-RTL line that makes `keeps` unreachable (the bidi algorithm reverses an RTL run) and
`inverts` free: `Invert` by elimination, reported to the caller as `bidi_verified`. The rung's whole
purpose is "ask the file, not the producer" — a comparison whose evidence is derived from the
sequence under test answers nothing, and its answer was indistinguishable from a real one.

## Why nothing else caught it

- **The output is text.** `لس` reads as plausibly as `سل` to anyone who does not read the script —
  which is every automated check in the repo and most reviewers.
- **It arrived unverified.** The lane that wrote it died before running the gate (`status=unknown`),
  so the code reached CI without its author ever executing it once.
- **The one thing that caught it already existed.** The control asserting refusal for that input was
  written before rung 3 was, and it was the only artifact in the repo that encoded *"this input has
  no answer"*. It turned a shipped silent-reversal into a five-minute fix.

It was caught by the second lane's run of the gate, then reproduced byte-for-byte from the test's own
fixture builder before any code was changed — the fix was designed against an executed counterexample,
not against a suspicion.

## The fix

`settle_line_by_bidi` now returns `Ambiguous` when the measured painting contains a tie at equal `x`
— that is, when the painted order was never measured. `Ambiguous` already means "fall through to the
producer fingerprint, and refuse outright when the family is unknown", so the line is withheld rather
than guessed:

```
exit 3 · ok false · text "" · reasons [actual_text, unsupported_visual_order] · unordered_chars 2
```

The control is untouched and green. The lane's own probe fixture had the same defect in miniature — it
asserted a `BidiVerified` verdict while leaving every cluster at one origin — and now places each
cluster with its own absolute text matrix, so its claim ("the positions decide") is true by
construction instead of assumed.

## Consequences

- **A measurement must be measured.** Deriving the evidence a check tests from the data under test is
  circular *by construction*; the only fixes are an independent signal or a refusal. Same family as
  `0006` (a control that cannot see its input) — that one is about inputs, this one about evidence.
- **`pdfrtl` does not model advance widths.** `x` is the text origin only, so a page that positions
  clusters by pen advance alone has no measured painting and is refused by rung 3. Honest, but it
  means rung 3 is blind to a whole class of real files until widths are modelled, and the producer
  fingerprint will carry those. Follow-up, not silently accepted.
- **Operators with the wrong operand count are ignored silently.** Writing `20 80 Tm` (two operands
  instead of six) split a line in two instead of being flagged: the operand reader returns `None` and
  the operator is skipped. A file we cannot parse fully is not a file whose order we established, so
  this should surface in the reasons.
- **`Td` is relative.** Two lanes and the orchestrator each wrote an "absolute" `Td` in a fixture on
  the first attempt. Fixtures that place text must use `Tm` (six operands). There is no shortcut, and
  no comment claiming a measurement should be trusted until the code that reads it is read.

## Evidence

| item | value |
|---|---|
| counterexample | 636 bytes: `BT /F1 12 Tf 20 80 Td` + two `/Span<</ActualText …>> BDC (.) Tj EMC`, `/Producer (pdfrtl-test)`, no font resource |
| before | exit 0 · ok true · text `لس` · reasons `[actual_text, bidi_reordered, bidi_verified]` · `unordered_chars` 0 |
| after | exit 3 · ok false · text `""` · reasons `[actual_text, unsupported_visual_order]` · `unordered_chars` 2 |
| gate | 13 suites green + `cargo deny` (fmt, clippy `-D warnings`, `cargo test --workspace --locked`, slop greps, deps-drift) |
| commits | `fix(order): rung 3 may only settle a line whose painting was MEASURED` on `lane/w17-order-recovery` |

## Process note

Two delegated lanes failed on this bug before the orchestrator fixed it: one died mid-work and left
the code unverified, the second reported "Python is unavailable in the WSL shell" (`python` is, but
`python3` is the interpreter) and produced nothing in 84 seconds. A subtle correctness bug in the
project's central invariant is not a task to keep re-delegating; reproduce it, read the code, fix it,
and let the control adjudicate.
