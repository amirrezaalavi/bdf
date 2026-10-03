# Disposition of the 2026-10-03 independent critique

**Reviewed document:** `docs/reviews/2026-10-03-independent-critique-and-plan-changes.md`
(Zed coding agent, Claude, asked by the owner).
**Date of disposition:** 2026-10-03.

Every claim was re-checked against the repository before agreeing or disagreeing. Three were
already stale; several were correct and are fixed here; the plan-level items are answered in §2.

## 1. Claims checked, and what is true

| § | claim | verdict |
|---|---|---|
| 1.1 | HEAD is red; `AGENTS.md` says never commit red | **Stale — fixed before the review landed.** The RED test is now a `--features hypothesis-space` target. Measured at `b4b2a05`: **15 suites ok, 0 failing**. The reviewer's "reproduce one failure" step no longer reproduces. |
| 1.2 | No human-reviewed fixture diff exists for any RTL change; `Reviewed-by` names an agent | **Correct.** Unfixed. `AGENTS.md` rule 8 requires a human read; none has happened. Tracked as Q-006. |
| 1.3 | Reader-facing docs contradict (12 vs 14 refused; 38+12+14+4 > 51) | **Correct.** Fixed in `CANARY.md` and `README.md` with a measured, disjoint classification that sums to 51. |
| 2.1 | `arabic-3.pdf` is ~93% of all withheld characters | **Correct, measured: 93.3%** (1,187,033 / 1,271,919). The "withheld > emitted" headline was an artefact of summing characters across files of very different sizes. Removed. |
| 2.2 | Five wrong root-cause hypotheses in a row on `arabic-3` | **Correct.** Recorded in `docs/problems/0009`–`0015`. See §3. |
| 3 | `unproven` in the default envelope is a footgun for LLM/MCP callers | **Correct.** Accepted as **P-4**, needs an owner decision — see §2 Q2. Not changed unilaterally. |
| 4.1 | The allow-list's "evidence" uses poppler, which `problems/0004` says cannot judge RTL | **Correct.** Accepted as P-10. |
| 4.2 | The archive measurement is not reproducible by anyone else | **Correct.** Accepted as P-9. |
| 4.3 | "byte-for-byte reproducible" is two claims and only one is evidenced | **Correct.** The claim is now narrowed to what was measured — output identical across the 14 public fixtures — and states plainly that a reproducible *build* is unverified. |
| 5.x | Plans are lab notebooks; `HANDOFF.md`/`ROADMAP.md`/`OPEN-QUESTIONS.md` are stale; docs outweigh code | **Correct.** Accepted as P-11 and partly done. |

### One number I got wrong while fixing the review's own number

Rewriting the contradictory counts, I first wrote a 35 / 14 / 2 split that **also** summed to 51
but matched no measurement. It was inference dressed as a table. Re-derived from
`reports/validate-archive.json`:

| | files |
|---|---|
| fully emitted | 31 |
| partially recovered | 4 |
| refused entirely | 10 |
| no text layer | 6 |
| **total** | **51** |

This is the only version committed. The lesson — a total that adds up is not evidence that a
breakdown is right — belongs with `problems/0006` and is why the derivation is now a committed
script rather than a hand-edited table.

## 2. Owner questions this review raises

The review asks four questions. Three need the owner and are **not** answered here:

1. **Is `arabic-3.pdf` in or out of canary scope?** (P-3) The reviewer's recommendation is
   "out". Evidence for it: 93.3% of all withheld characters, a defective text layer of its own
   (non-contiguous `/ToUnicode`, three fonts with none — broken text in Edge too), and five failed
   root-cause hypotheses spent on it. Keeping it in scope makes the canary's acceptance depend
   on one file that may be unfixable for reasons unrelated to order.
2. **Do you accept confidence tiers in the output contract, with `unproven` opt-in?** (P-4)
   The reviewer's own analysis supports the current binary invariant being stricter than the
   evidence justifies, and `problems/0015` is the project's own measurement saying so. **Not
   changed unilaterally** — this is a contract change and it is the owner's call.
3. **Who is the named human reviewer, and how many hours per week?** (P-8) The single largest
   project risk by the reviewer's own ranking.
4. Is a minimal RTL writer worth pulling forward to generate ground-truth fixtures? An opinion;
   not costed here.

## 3. Where I disagree, and why

* **§2.2 "the plan asks whether arabic-3 is out of scope — unanswered and unowned."** Agreed on
  the facts, but the question *was* in the owner's court and stayed there deliberately: an
  acceptance target is a scope decision, not an engineering one. It is now P-3 Q1 rather than a
  sentence buried mid-investigation.
* **§5.4 "split `recover.rs` before the solver lane" (P-6).** Accepted as sequencing, not
  urgency. A pure-move refactor of a 2,217-line file that every order change lands in is worth
  doing — but it is not a prerequisite for the solver, and doing it first spends a day on churn
  with no new capability. It goes **after** the fixtures (P-5), because the fixtures are what
  make the refactor safe to verify.
* **§1.2 "do not write `Reviewed-by: <yourself>`".** Agreed and already the rule. The stronger
  form the reviewer proposes — `Reviewed-by: none (agent-only; see docs/reviews/…)` — is adopted.
* **§4.3 "soften the sentence".** Not softened: **corrected**. The distinction between "output
  verified identical" and "build reproducible" is real, and the sentence asserted the second
  while only measuring the first.

## 4. What was done, in this pass

* `CANARY.md` — measured disjoint classification; per-file fraction and median; the 93.3%
  artefact stated; the reproducibility claim narrowed.
* `README.md` — one count, matching the table.
* `docs/problems/0016-…` — the mis-derived split recorded as a failure, per `problems/0006`.

## 5. Not done, and why

* The confidence-tier ADR, `unproven` opt-in, the fixture cut (P-5), the `recover.rs` split (P-6),
  the bounded solver (P-7), the aggregate-only archive report (P-9) and the allow-list
  re-examination (P-10) are **accepted and queued, not implemented**. P-7 in particular must not
  start before P-5: `problems/0009`, `0014` and `0015` all say a comparison must not be changed
  before the fixtures that would catch the regression are in place.
* Q-006 (a named human reviewer) blocks the honest completion of C7.