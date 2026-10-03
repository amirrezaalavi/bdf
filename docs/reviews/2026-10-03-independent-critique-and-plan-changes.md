# Independent critique of the work so far, and proposed plan changes

**Date:** 2026-10-03
**Author:** Zed coding agent (Claude), asked by the owner for a candid outside read.
**Audience:** the next agent. Read this after `AGENTS.md`, `HANDOFF.md` and
`docs/plans/2026-10-03-canary.md`, and before touching `recover.rs`.
**Status:** opinion plus a few checks I ran myself. Every item is labelled as one of:

* **[VERIFIED]** I ran or read it in this session.
* **[INFERRED]** I derived it from numbers or text in the repo and did not re-measure it.
* **[OPINION]** judgement. Challenge it.

**What I did not do.** I did not read `recover.rs` line by line (2,217 lines). I did not run
`wsl-build.sh`, clippy, `cargo deny` or the private-corpus validation. I did not look at the
private PDFs. Nothing here changes an extraction result. Where I say "the code does X" I mean the
docs say so or I read the function head.

---

## 0. TL;DR

The engineering discipline is unusually good. That covers the invariant, refusing instead of
guessing, the problem log, the deterministic fixtures and the licence red line. The weak points
are not discipline. They are **direction, measurement framing, and process drift.**

1. **HEAD is red, and the docs say green.** [VERIFIED] See §1.1.
2. **The project has spent most of its recent effort on one file** (`arabic-3.pdf`, a 321-page
   scan with a partial text layer). That file also distorts the headline statistic. See §2.
3. **The all-or-nothing contract ("no third outcome") is too coarse for the evidence the project
   itself has produced.** Its own research (`problems/0015`) says the guarantee it advertises
   cannot be delivered by geometry alone. The right response is a *labelled confidence tier*, not
   more refusing. This needs an owner decision. See §3.
4. **"Evidence" has drifted.** Reviews are signed by the same agent that wrote the code, and
   some "evidence-backed" claims lean on a tool the repo itself says cannot be trusted for RTL.
   See §4.
5. **The docs are now bigger than the code, contradict each other, and the plans are lab
   notebooks.** The next agent will lose time to this. See §5.
6. **Recommended plan change:** stop chasing arabic-3. Build the bounded solver the round-2 research
   describes. Do it against small, committed, redistributable fixtures cut from real refusals. Add a
   confidence-tier ADR. Ship the canary with honest per-file numbers. See §7.

---

## 1. Verified facts about the repo as it stands

### 1.1 HEAD does not pass the test suite [VERIFIED]

I ran, at `20d5957`:

```
CARGO_TARGET_DIR=/home/netcon/target-bdf cargo test --workspace --locked --no-fail-fast
```

Result: every target passes **except** one:

```
Running tests/hypothesis_space.rs
test a_long_mixed_line_is_decided_and_its_digits_survive ... FAILED
test result: FAILED. 2 passed; 1 failed
```

That test came from commit `3778156` ("test(order): RED for the hypothesis space; three units
decide, nine refuse"). It was committed deliberately red, which is correct TDD. But:

* `AGENTS.md` rule 2 says *"Never commit red."* and `HANDOFF.md` says the gate is GREEN.
  `scripts/wsl-build.sh` runs `cargo test --workspace`, so **the gate cannot be green at HEAD.**
  Either the docs are wrong or the rule is.
* Any later `--quick` gate run, or any agent who trusts HANDOFF, will "discover" a regression that
  is really an expected-red test.
* A suite that is red by design trains people to ignore red.

**Fix, in this order:**
1. Move RED tests onto the lane branch (`lane/hypothesis-space`) until they go green. `master`
   stays green. That is what the rules already say about lanes.
2. Do **not** `#[ignore]` the test. `AGENTS.md` forbids skipping, and an ignored test is the
   cheapest way to lose the acceptance criterion.
3. Add a line to `HANDOFF.md` that states the exact test count and the date it was last run
   green. A bare "GREEN" is a claim. This repo's own `problems/0006`/`0011` say so.

### 1.2 Review trailers are not independent review [VERIFIED]

From the last 40 commits: 33 have `Implemented-by: hermes-agent (yolka profile, orchestrator)`.
The `Reviewed-by:` lines are mostly the same agent or a description of the test it ran itself.
Examples: *"whole core suite including no_silent_reversal…"*, *"hermes-agent (yolka profile,
2026-10-03)"*, *"pending orchestrator"*, *"nobody yet — gate not run"*.

`AGENTS.md` rule 8 says: *"Agent self-assessment does not count."* for any change that alters
RTL text output. `docs/reviews/` contains exactly two files, both about the CI/deny gate. **There
is no human-reviewed fixture diff for any RTL text change.** The strongest "human" artefact is
`reports/vision-review-2026-09-27.md`. The roadmap describes it as *"vision-checked here"*, which
is an agent looking at a picture.

This matters because the project's central claim is *"nobody who cannot read the script can
detect a wrong reversal."* The same limit applies to the agent doing the reviewing.

### 1.3 Contradictory numbers in reader-facing docs [VERIFIED]

| Claim | Where | Conflicts with |
|---|---|---|
| "12 files refused entirely" | `CANARY.md` §3 | "14 refused" in `HANDOFF.md`, `ROADMAP.md` |
| "partial 12 / refused 14 / no-text 4 / decoded 38" | `HANDOFF.md` | 38+12+14+4 = 68, which is more than 51. Categories overlap, and the docs never say how. |
| Plan of record is `2026-09-29-lane-plan.md` | `HANDOFF.md` | `2026-10-03-canary.md` says it "supersedes the phase list" |
| "No decision currently blocks the next extraction work" | `HANDOFF.md` | `OPEN-QUESTIONS.md`: all 10 questions `open`, and its own rule 4 says the exit condition is zero open |
| "23 files have no text layer" | `phase2-order-recovery.md` | "19 archive files" in ROADMAP W7, "4 of 51" in ROADMAP fact 1 |
| Effort "3–5 days" for W1 | `ROADMAP.md` | W1 has been open for days past that. The estimate has not been revisited. |

None of these is fatal. Together they mean a new agent cannot tell which number is current.

---

## 2. Where the effort is going

### 2.1 One file is steering the project [INFERRED from the repo's own numbers]

* `arabic-3.pdf` decodes **1,187,033** characters. `CANARY.md` reports **1,271,919** characters
  withheld across the whole archive.
* So **roughly 93% of all "withheld" characters come from this single file.** Without it, withheld
  is on the order of 85k against 1.04M emitted.
* `CANARY.md` §3 says in bold *"The withheld number is larger than the emitted number. That is the
  honest state of the work."* That sentence is **misleading as written.** It is an artefact of
  summing characters across files of wildly different sizes. A reader who sees it will think the
  tool refuses more than it answers. For most documents that is not what the numbers show.

**Change the headline metric.** Report *per file*: the fraction of characters emitted, and
the median and worst-case across files. Report character sums only as a footnote, and always with
the largest single file called out.

### 2.2 The arabic-3 investigation shows a pattern [VERIFIED from `canary.md` C3a]

The plan records **five wrong root-cause hypotheses in a row**: the `/W` parse, a zero font size,
frozen pens on undecodable fonts, `Tm` resets, and the tie rule. A sixth ("the proxy char is *the*
blocker") was implemented and did not move the file. Each was cheap to state, wrong, and was
caught only by measurement afterwards.

Credit where due. The project learned to instrument first (`PDFRTL_TRACE_ORDER`, `distinct_x`),
and the problem log is honest. But the plan file says outright that "a fifth wrong fix in a row"
was nearly written. The cost is real: roughly nine commits (`78cd978` through `3778156`) with no
change to any archive number.

Two further points about this file:

* The owner's Edge paste shows broken text in the viewer too, because of non-contiguous
  `/ToUnicode` entries and three fonts with none (C3a). **Even a perfect order solver may yield
  garbled output here.** The plan notes this and then proceeds as if order is "the whole game".
  That is inconsistent. The file is a poor acceptance target.
* The plan itself asks whether a 321-page scanned WHO volume "is arguably out of scope for a canary
  whose purpose is to be correct". That question is sitting in the middle of C3a, **unanswered
  and unowned.** It should be decided explicitly and moved to `OPEN-QUESTIONS.md`.

### 2.3 The tool is optimising a fixture, not a user [OPINION]

The stated canary goal is *"A named person… downloads one static binary… gets either correct
logical text or a clear refusal."* Of the three audiences that goal implies (a Persian reader, a
search indexer, an LLM consumer), none is served by the current effort on a 1,100-lines-per-page
corner case. A far more common request will be "extract this ordinary Word/Chrome/LibreOffice
document". The project has measured that path less than it measured arabic-3.

---

## 3. The "no third outcome" contract, and the evidence against it

`AGENTS.md`: *"Extraction returns logical order, or it fails with an explicit Reason… There is no
third outcome."*

`docs/problems/0015` (and the round-2 research) establish that **a successful geometry match
cannot prove the logical order**. Two different logical strings can paint the same glyph sequence
(`אבa 12` and `אב12 a`). The project handled this well by renaming `bidi_verified` to
`bidi_consistent`. But follow the consequence through:

1. The strict claim "logical order, proven" is **already unattainable from geometry alone** for
   mixed lines. The only genuinely proven rungs are `/ActualText` and (arguably) `/ReversedChars`.
2. Every other emitted line is already "consistent with the evidence under a model". That *is* a
   third outcome. The code ships it today and calls it logical.
3. The refusal side is therefore stricter than the acceptance side justifies. A line the solver
   refuses because *two* candidates fit is treated the same as a line that no candidate fits.
   Those cases are very different for a reader.

### Proposal: confidence tiers (needs an owner ADR) [OPINION]

Keep the invariant. **Never silently reverse.** Replace the binary with an explicit, machine-readable
tier on every emitted line or page:

| Tier | Meaning | In `data.text`? |
|---|---|---|
| `proven` | `/ActualText` or producer-declared order | yes |
| `unique_in_model` | complete bounded search found exactly one candidate | yes |
| `consistent_ambiguous` | more than one candidate fits; best-ranked one offered, alternatives listed | **no.** Separate field. |
| `inconsistent` / `no_evidence` | nothing fits | no, refuse (current behaviour) |

This is not weakening the invariant. It makes the invariant's boundary explicit. The research
(§"My opinion", step 5) asks for exactly these separate outcomes: *no supported solution,
multiple distinct solutions, insufficient model evidence, search budget exhausted*.

### The `unproven` field is a footgun [VERIFIED from `CANARY.md`; OPINION on the risk]

`CANARY.md` says the withheld text "is frequently **reversed**: on the `arabic-*` files… the stored
order is the wrong reading for 92–99% of lines", and it is then shipped in
`data.pages[].unproven`.

Consider who calls this tool: scripts, indexers and (given the MCP crate) LLM agents that read the
*whole JSON object*. A field that is "frequently reversed Persian" in the same blob as the good text
is exactly the silent-reversal hazard the project exists to stop. Mitigations, cheapest first:

1. Make it opt-in: `--include-unproven`. Default output omits it and reports only counts.
2. When present, call it what it is: `unproven_stored_order` and include a per-line
   `stored_order_likely: "visual"|"unknown"` hint. Do not make the consumer guess.
3. Add a contract test that the default envelope contains **no** unproven text.

---

## 4. Evidence quality

### 4.1 The allow-list's "evidence" uses an instrument the project distrusts [VERIFIED docs; OPINION]

`lane-plan.md` says the 14 allow-listed files "score as logical against poppler's `-layout`
reading (0.78–0.92 vs 0.33–0.58)" and concludes the allow-list is "evidence-backed". But:

* `problems/0004` shows poppler/Xpdf **cannot be the reference for RTL** (flipped lam-alef,
  visual order). `AGENTS.md` says "verification tools only", "may never judge text identity".
* `problems/0011` records two instruments that could not discriminate, and `0015` is a reminder
  that a lexical score is a *consistency* signal.
* The check is document-level. The research says explicitly not to "infer one global storage
  convention", because streams can paint different text objects in different orders.
* The allow-list *inverts* text. A wrong entry is a silent reversal.

The numbers are decent evidence that the entries are probably right. They are **not** the "pixels
read by an eye" that rule 8 requires. Suggest:

* Mark each `ProducerFamily` entry with `evidence: {kind, reviewer, date}`; the entry's test
  should fail if `reviewer` is empty.
* Use the allow-list as a **hint that must agree with geometry** where geometry is available. If
  geometry contradicts the producer rule on a line, downgrade that line's tier instead of
  trusting the fingerprint.

### 4.2 The measurement cannot be reproduced by anyone else [VERIFIED]

All headline numbers come from 51 private customer PDFs. That is understandable. But it means:

* A second human or agent cannot check a single claim in `CANARY.md` §3.
* The project's own `problems/0006` ("a gate may not pass vacuously") is not applied to this
  measurement. There is no committed script whose *output schema* is aggregate-only and safe to
  publish, no checksum of the corpus manifest and no recorded run date in a stable file.
  (`reports/validate-archive.json` exists but is gitignored or private.)

Suggest: `scripts/validate-archive.sh` writes an **aggregate-only** JSON (counts per reason,
per-file fraction emitted, no names, no hashes, no titles). It is committed once per milestone
with the corpus manifest's own hash. A second machine with the same corpus can then diff it.

### 4.3 The "byte-identical to glibc" claim [UNVERIFIED]

`CANARY.md` §5 says the musl canary is *"byte-for-byte reproducible… verified identical in output
across all 14 public fixtures."* Those are two different claims (reproducible *build* vs identical
*output*). I found no artefact backing the first. `AGENTS.md` rule 5 asks for raw output in
`reports/` or `docs/reviews/`. Check it or soften the sentence.

---

## 5. Process and documentation drift

### 5.1 Plans are being used as lab notebooks

`docs/plans/2026-10-03-canary.md` is 293 lines. About 180 of them are C3a's investigation history:
retracted claims, "ROOT CAUSE FOUND", "withdrawn before it was written", then "Implemented, and it did
NOT move the file". That content is valuable, but it belongs in `docs/problems/` (and mostly
already is, in 0009/0010/0011/0014). In the plan it makes it nearly impossible to answer "what are
we doing next?" without reading the whole history.

**Rule:** a plan states *goal, acceptance, order, status*, with one line per past attempt linking to
a problem file. Narrative goes to `docs/problems/`.

### 5.2 Several source-of-truth files are stale

* `OPEN-QUESTIONS.md` is a template of ten `open` items from the first day (AGPL tier, GUI scope,
  names…). Some are superseded by the canary decisions (licence, scope, platform). Rule 3 in that
  file says answers become ADRs; the canary decisions were taken but never recorded there. Close
  Q-002, Q-004, Q-009, Q-010 and the licence part of Q-001 and point to the canary plan or an ADR.
* `ROADMAP.md` "Sequence" section is a numbered list whose item 1 got eaten by a status note
  (the list starts at "2."). Its effort table predates the canary.
* `HANDOFF.md` "Next work" still lists the order from before the canary re-scoped, and says Phase 1b
  is `hebrew-1.pdf` cmap inversion, while the canary plan says that task was **closed** (C2,
  `problems/0013`). Next agent: trust the canary plan over HANDOFF for ordering and fix HANDOFF.
* `phase2-order-recovery.md` says "23 files have no text layer"; other docs say 19 or 4.

### 5.3 Docs outweigh code [VERIFIED]

Approx. size: `recover.rs` 2,217 lines, all other hand-written `src/` about 1,500, tests about
2,000. Against that: 15 problem records, 6 ADRs, 3 plans, 2 research files, `KNOWLEDGE.md`,
`ESTIMATE.md`, `RESEARCH-QUESTIONS.md`, `HANDOFF.md`, `ROADMAP.md`, `CANARY.md`. Documentation of
failures is a strength. But there is no single index, so each new agent re-reads all of it.

Suggest a `docs/INDEX.md` of ten lines: "read these four in this order, everything else is
reference". Archive superseded plans into `docs/plans/archive/` with a banner.

### 5.4 One monolith blocks the lane model [VERIFIED]

`AGENTS.md` rule 3 is "one agent per file". `recover.rs` holds the walker, font metrics, producer
allow-list, pen model, bidi rung, hypothesis candidates, and withholding. All of the order work
lands in one file. That serialises every lane through it and makes review hard.

Split by *responsibility*, not size. A proposed layout (a suggestion, I did not check call graphs):

```
text/walker.rs      content-stream walk, Tj/TJ, Tm, units
text/metrics.rs     /W, /Widths, DW, pen advance
text/producer.rs    fingerprint + allow-list + OrderConvention
text/order/mod.rs   ladder + LineOrder + Reason decisions
text/order/bidi.rs  painted_map_for, hypothesis generation, bounded solver
text/withhold.rs    unordered/unproven bookkeeping
```

Do it as pure moves (no behavioural diffs) in a single refactor commit with the suite green and
`no_silent_reversal` unchanged. Do it *before* the solver lane, so the lane has its own file.

### 5.5 `no_silent_reversal` is necessary, not sufficient [VERIFIED; the repo already says it]

`problems/0014` records that the invariant control stayed green while a change *increased
refusals* and failed an older test. Add a **refusal ratchet**: a committed
`expected_decided.json` of (fixture → decided lines) that may only grow. A change that reduces
decided lines on any public fixture fails the gate unless the commit also edits the ratchet with a
justification. This makes problem 0014's rule 3 mechanical rather than a thing to remember.

---

## 6. Things that are good and must not be lost

* The invariant and its test control (`no_silent_reversal`), plus the rename from `bidi_verified`
  to `bidi_consistent`. This is the single most important integrity act in the repo.
* `problems/` as a failure log with root cause and rule extracted. Most teams never write this.
* Refusing per font, per line, with `unproven` accounting rather than dropping text.
* Pinned toolchain, `cargo deny`, `DEPS.md` row per dependency, deterministic fixtures.
* The decision to bundle engines (lopdf, harfrust, unicode-bidi, krilla) rather than write them.
* Treating oracles (poppler, Xpdf, PDFium) as *measured and fallible*, with a matrix. Keep that
  discipline, and apply it to the allow-list too (§4.1).
* Honest negative results (e.g. `hebrew-1.pdf` was never the broken file). The cost of publishing
  a retraction is low and the repo does it well.

---

## 7. Proposed changes to the plan

Ordered. Each item has an acceptance check. Items 1–3 are small and should land first.

### P-1. Make `master` green again (hours)
* Move `hypothesis_space.rs`'s RED test to `lane/hypothesis-space`; keep it as the acceptance test.
* Run `bash scripts/wsl-build.sh` (full, with cargo-deny) and record raw output in `reports/`.
* Update `HANDOFF.md` with the real test count and date.
* **Acceptance:** full gate green on `master`; output file committed.

### P-2. Fix the numbers in the reader-facing docs (hours)
* One table, one definition for each category (decoded / order-consistent / partial / refused /
  no-text), with categories that are **disjoint** and sum to 51.
* Remove or rewrite "withheld is larger than emitted"; add per-file emitted fraction (§2.1).
* Reconcile 12 vs 14, and 4 vs 19 vs 23 no-text.
* **Acceptance:** `grep -rn "refused" CANARY.md HANDOFF.md docs/ROADMAP.md` shows one number.

### P-3. Decide the arabic-3 question explicitly (owner, minutes)
Options: (a) out of canary scope, listed as a known limitation; (b) keep as a stretch target, not
an acceptance gate. **My recommendation is (a).** Record it in `OPEN-QUESTIONS.md` then an ADR.
Effect: the canary's acceptance no longer depends on one 321-page file whose text layer is itself
defective.

### P-4. Record the confidence-tier ADR (owner decision, then small code)
See §3. Until the owner decides, do not change the contract. Draft the ADR with three options:
(i) keep binary, (ii) add tiers behind a flag, (iii) tiers in the default envelope. I recommend (ii)
first. Same pass: make `unproven` opt-in (§3).

### P-5. Cut real lines into public fixtures (the research agent's step 1) (1–2 days)
The round-2 answer says to log one refused real line with IDs, payloads, geometry, unit
boundaries and context, and keep it as a fixture. Do this with **redistributable text**:
rewrite the line's *shape* (script mix, digits, date, Latin run) with public-domain or invented
words in the generator, and keep geometry and unit structure. This yields public fixtures that
exercise the same hypothesis-space failures without publishing customer text.
* **Acceptance:** at least 6 public fixtures with ≥2 LTR islands; each has an expected logical
  string checked by a human who reads the script.

### P-6. Split `recover.rs` before the solver lane starts (half a day)
See §5.4. Pure move; suite unchanged. **Acceptance:** same test results, same `sha256sum` of
`extract --json` on every public fixture, before and after.

### P-7. Build the bounded solver as designed in the round-2 research (the real C3) (1–2 weeks)
Do not bring back the "classify then reverse runs" idea that failed in `problems/0014`.
Concretely:
1. Add **candidates** additively (rule 1 of 0014): keep `stored` and `invert_units`.
2. Level-enumeration solver for the {1,2} model, as sketched in the research. [OPINION] The
   search space is smaller than the research implies, because strong RTL letters are fixed at
   level 1 and LTR letters and digits at level 2 in an RTL paragraph. Only runs of *neutrals*
   (space, punctuation, parentheses, `/`) between islands are free. So the branching variable is
   "level of each neutral span", not each unit. I have not tested this. Verify it against an
   exhaustive-permutation oracle on lines of ≤8 units (the research's step 4) before trusting it.
3. A hard search budget. **Budget exhausted ⇒ refuse with a distinct reason**, even if one match
   was found (research: "A search that reaches its budget must refuse").
4. Keep E1 (`אבa 12` / `אב12 a`) as a committed fixture asserting `consistent_ambiguous`.
5. Distinct internal outcomes: `NoSolution`, `Ambiguous`, `Exhausted`, `Unique`.
* **Acceptance:** `hypothesis_space` tests green; refusal ratchet (§5.5) shows no loss; E1 refuses
  as ambiguous; `no_silent_reversal` green; a human read of the rendered pages for any file whose
  output changes.

### P-8. Get a real, named human reviewer (owner)
Q-006 is still open and is the single largest project risk. Until it is answered: no `Reviewed-by`
trailer on an RTL-output change may name an agent. Use `Reviewed-by: none (agent-only; see
docs/reviews/...)` and be honest. Provide the reviewer a 1-page-per-fixture
checklist, with rendered PNG, expected string, extracted string. `scripts/render_pages.py` already
produces the PNGs.

### P-9. Make the archive measurement shareable (half a day)
See §4.2: aggregate-only output, committed per milestone.

### P-10. Re-examine the producer allow-list (1 day)
See §4.1. Add `evidence` metadata and a per-line geometry cross-check.

### P-11. Docs hygiene (half a day)
* `docs/INDEX.md`; archive superseded plans.
* Strip C3a history out of the canary plan into `docs/problems/0016-...`; leave a link.
* Close stale items in `OPEN-QUESTIONS.md`.
* Soften or evidence the "byte-for-byte reproducible" sentence (§4.3).

### Ordering after the canary (unchanged in spirit)
Writer → editing → MCP, as today. One addition: **before any writer work, decide what the tier
vocabulary means for generated text** (a generated PDF can always carry `/ActualText`, so it can be
`proven` by construction. That is the strongest argument for the writer: it creates the ground
truth the extractor currently lacks). I'd consider moving a *minimal* RTL-only writer earlier, for
fixture generation, because the project's biggest bottleneck is "no ground truth without a human".
That is an opinion; weigh it against scope.

---

## 8. Risks I would watch, ranked

| # | Risk | Why |
|---|---|---|
| 1 | Single human oracle, not yet identified (Q-006) | Every RTL correctness claim rests on one person's eye. |
| 2 | Private-corpus dependence | Headline metrics are unreproducible by anyone else (§4.2). |
| 3 | Canary ships numbers that read worse than the truth, or better | §2.1 and §1.3 show both directions of misstatement. |
| 4 | `unproven` text consumed as if it were proven | §3. Likely with LLM/MCP callers. |
| 5 | Allow-list inversion applied to a mixed-order document | §4.1. Silent. |
| 6 | Effort sink in a single hostile file | §2. |
| 7 | One-file core makes parallel lanes collide | §5.4. |
| 8 | Documentation sprawl | §5.1–5.3. |

---

## 9. Things I would ask the owner

1. Is `arabic-3.pdf` in or out of canary scope? (P-3)
2. Do you accept confidence tiers in the output contract, with `unproven` opt-in? (P-4)
3. Who is the named human reviewer, and how many hours per week? (P-8)
4. Is a minimal RTL writer for fixture ground truth worth pulling forward? (§7, last paragraph)

---

## 10. Quick start for the next agent

1. `git --no-pager log --oneline -5` should show `20d5957` or later.
2. Reproduce my red check: `CARGO_TARGET_DIR=$HOME/target-bdf cargo test --workspace --locked
   --no-fail-fast` and expect exactly one failure in `hypothesis_space`.
3. Do P-1, P-2 and P-6 first. They are small and unblock everything.
4. Do not start P-7 until P-5 fixtures exist. Do not change a comparison before the fixtures
   are in place, because `problems/0009`, `0014` and `0015` all say so.
5. Any change that alters RTL text output needs a human-reviewed diff in `docs/reviews/`. Do not
   write "Reviewed-by: <yourself>".

## Appendix A — Commands run for this review

```
cargo test --workspace --locked --no-fail-fast      # 1 failing test: hypothesis_space
git --no-pager log --oneline                         # 63 commits
git --no-pager log --format=%b -n 40 | grep -E "Implemented-by|Reviewed-by"
find crates -name '*.rs' | xargs wc -l               # recover.rs 2217, tables.rs 4451 (generated)
grep -rnE "unwrap\(\)|expect\(|println!|eprintln!|dbg!" crates/pdfrtl-core/src   # no hits
```

The slop grep returned nothing in the core, so the "core must not print" and "no unwrap" rules
hold at HEAD. Clippy, fmt and `cargo deny` were not run.
