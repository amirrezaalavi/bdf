# Open questions — the blocking-decision loop

**Status legend:** `open` (blocks work, someone must answer before a task can start) ·
`answered` · `parked` (does not block; decided later).

**Rules**
1. A question may be `open` only if a task in the active plan cannot start without it.
2. One question per entry; no compound questions.
3. When answered: set `status: answered`, fill `answer:`, then write the decision as an
   ADR in `docs/decisions/` (answers that live only in chat are lost).
4. The plan is re-planned whenever an answer lands. **Exit condition: zero `open`.**
5. Askers should batch these to the upstream agent/human in one pass rather than one at
   a time — each round trip costs more than the question.

---

## Q-001 — Where does the paid tier run?
- **status:** open
- **blocks:** Phase 2 task 2.6 (MCP transport + packaging), Phase 4 lane W4
- **question:** Is the paid product a **self-hosted licensed binary** (customer runs the
  MCP server inside their own infrastructure) or a **hosted service we operate**?
- **why it matters:** a hosted service interacts with AGPL §13 (network users must be
  offered the source) — which is fine and even protective, but it changes what the
  commercial licence must grant, and whether the MCP server may be closed source at all.
  Self-hosted + commercial licence is the configuration Option A was designed for.

## Q-002 — Which side headlines v1: extraction or generation?
- **status:** open
- **blocks:** Phase 3 corpus priorities; Phase P1/P2 ordering; the demo
- **question:** For the first sellable version, is the headline capability **reading
  existing RTL PDFs correctly** (extraction/search) or **producing correct RTL PDFs**
  (generation)?
- **why it matters:** both need the same shaping/bidi core, but the fixtures, the oracles
  and the sales story differ. Extraction has no off-the-shelf competitor (our wedge);
  generation competes with krilla/Typst/LaTeX-shaped tools.

## Q-003 — Exact language/script scope for v1
- **status:** open
- **blocks:** corpus acquisition (Phase 3), shaper script tables, font set
- **question:** Is v1 exactly **Arabic + Persian + Hebrew + English**, or do we also
  need Urdu, Kurdish (Sorani), Pashto, Syriac, Aramaic — and do we need any vertical
  script (CJK) for "next to LTR languages"?
- **why it matters:** each additional script costs fixtures, a native-speaker reviewer,
  and font licensing; vertical scripts are a separate layout engine, not a flag.

## Q-004 — Is a GUI in this repository's scope?
- **status:** open
- **blocks:** Phase 6 packaging; whether a C ABI crate is P0
- **question:** Does the free GUI bundle consume `pdfrtl` via the **CLI/C ABI** (separate
  repo, e.g. a Tauri/Qt shell), or is a GUI part of this workspace?
- **why it matters:** keeping the GUI out preserves the thin-core architecture (priority
  #5: "bundled in a single structure, integrable into another library") and keeps the
  paid core separable. Putting it in changes the licence story for the bundle.

## Q-005 — May fixture PDFs with copyrighted content live in this repo?
- **status:** open
- **blocks:** Phase 3 task 3.1 (manifest `redistributable` field), public corpus release
- **question:** Can we commit real-world PDFs (school textbooks, scanned books) into a
  **public** repo, or must non-public-domain fixtures stay local (gitignored) with only
  sha256 + provenance recorded?
- **why it matters:** redistribution of copyrighted textbooks is a real legal exposure,
  and the corpus is also the evidence behind our correctness claims (we want to publish
  it, but only what we may).

## Q-006 — Who reviews Persian/Arabic output?
- **status:** open
- **blocks:** Phase 5 review gate; P0 acceptance criteria
- **question:** Is there a native-speaker reviewer available (you, a colleague, a paid
  reviewer) — or do we accept **oracle-only review** (pixels + cross-tool extraction) for
  P0 and label that limitation in the reports?
- **why it matters:** RTL correctness cannot be fully proven by oracles; but waiting for
  a reviewer stalls P0. The honest middle path is oracle-only + explicit labelling until
  a reviewer exists.

## Q-007 — Can we reuse `al-bdf-engine` code?
- **status:** open
- **blocks:** Phase 4 lane W1 (how much is a port vs. a fresh design), the estimate
- **question:** Who owns the RTL work committed in `yolka-wiz/al-bdf-engine` (the fork is
  GPL-3.0 and its RTL files are fork-authored), and may we **relicense that logic** into
  an AGPL/double-licensed Rust core?
- **why it matters:** if yes, we port the design and the 27 pinned tests directly. If
  ownership is unclear, we re-derive the same logic from the documented failure modes
  (+2–3 agent-days) and keep the licence story clean.

## Q-008 — Project name and public org
- **status:** open
- **blocks:** crate names (published?), repo URL, README title, domain
- **question:** What is the product called, and in which GitHub org/user does the public
  repo live?
- **why it matters:** crate names and the `repository` field are baked into the workspace
  and hard to change after first publication; the MCP server name is visible to every
  agent that connects to it.

## Q-009 — Is P0 internal, or the first customer demo?
- **status:** open
- **blocks:** how much polish goes into P0 (docs, error messages, a demo page)
- **question:** Is P0 (`inspect`/`extract --json` + corpus report) an internal milestone
  or something you will show to a customer?
- **why it matters:** demo-grade means error messages, a one-command demo and English/
  Persian docs; internal-grade means the corpus report is enough. Scope follows this.

## Q-010 — Packaging targets for v1
- **status:** open
- **blocks:** Phase 6 Dockerfile/musl work, CI matrix
- **question:** Which artifacts must v1 ship: static musl binaries per OS/arch
  (linux-x86_64/aarch64, windows-x86_64, macos-aarch64), a container image, or
  language bindings first?
- **why it matters:** bindings (PyO3/napi-rs) are a P3-sized effort; a static binary is
  nearly free. Publishing order affects what "v1 sellable" means.

## Q-011 — Is `arabic-3.pdf` a target or a known limitation?
- **status:** open
- **blocks:** nothing today; it decides whether the canary's acceptance gate depends on one file
- **question:** Drop that one 321-page scanned volume as an acceptance target and list it as a
  known limitation instead?
- **why it matters:** it is 93.3% of all withheld characters in the archive, and its own text
  layer is defective (broken even in Edge), so it may be unfixable for reasons unrelated to reading
  order. The owner has said one file is not a reference for general work and can be replaced.
  **Recommendation: drop as a target, keep as a file.**

## Q-012 — Should the output state *how sure* it is?
- **status:** open
- **blocks:** any change to the output contract
- **question:** Replace today's binary "ordered / refused" with a per-page label —
  `proven` (the file states its own order) · `matches_the_drawing` (our reconstruction reproduces
  it exactly) · `several_readings_match` (more than one fits; we return the best and list the
  others) · `no_evidence` (refuse, today's behaviour)?
- **why it matters:** our own measurement (`docs/problems/0015`) shows two different Persian/Hebrew
  strings can draw **identically**, so "matches the drawing" is not proof. Today a line with two
  possible readings is treated exactly like a line with none — both are dropped, though those are
  very different situations for a reader.
  **Not changed unilaterally: this is the output contract, so it is the owner's decision.**
  Sequence: after the public refusal fixtures exist (P-5), because the four cases need tests that
  tell them apart.

## Q-013 — Who reads the output and confirms it is right?
- **status:** open (supersedes nothing; sharpens Q-006 for the canary)
- **blocks:** the honest completion of C7 and of every correctness claim
- **question:** Can the owner (or a colleague) read three rendered pages and confirm the extracted
  text matches what is written there — and how many hours per week are available?
- **why it matters:** every number in this repository proves characters were *ordered*, not that a
  Persian speaker read them and found them correct. `AGENTS.md` rule 8 excludes agent
  self-assessment, and an AI reading Persian is exactly what that rule excludes. **This is the
  single largest risk and only the owner can close it.** We make it cheap: one page per file,
  picture + expected text + extracted text, produced by the existing `scripts/render_pages.py`.

## Q-014 — Build a minimal RTL writer for ground-truth fixtures?
- **status:** parked (does not block the canary)
- **question:** Build a minimal RTL PDF writer that emits `/ActualText`, so fixtures we generate
  are provably correct by construction?
- **why it matters:** we cannot publish the customer's documents as fixtures, so we invent
  look-alikes — and then have no independent answer for what an invented file *should* say. A
  writer removes that: we make the file, so we know the answer. **Recommendation: not now.** The
  fixture actually needed today is one synthetic file that refuses, which is a small change to the
  existing generator (`docs/problems/0017`), not a new writer. Revisit after the canary.

---

## Answered

_(none yet — answers land here and become ADRs)_

---

## Plain-language version

The four questions above, without jargon — see also `docs/reviews/2026-10-03-review-disposition.md`.

**Q-011 — the one weird book.** One 321-page scanned book from your own files is the worst input
in the corpus: even Edge shows its text broken, so its text layer is defective independently of
anything we do. Should it stop being a target and just be named as a known limitation?
*Recommendation: yes. Cost of answering: one line.*

**Q-012 — saying how sure we are.** Today the tool says only "this is in reading order" or "I
refuse". We measured that two genuinely different strings can draw **the same letters in the same
places**, so "it matches the drawing" is not certainty — it means "it matches, and I could not find
a second reading that also matches". A label saying which of those we did would be more honest:
*the file states its own order* / *matches the drawing* / *several readings fit* / *nothing fits*.
Today a line with two possible readings is dropped exactly like a line with none, which is unfair
to the reader. *Not changed yet — it changes the output contract, so it is your call.*

**Q-013 — the human read.** Every number we publish proves characters were *ordered*, not that a
Persian speaker read them and found them correct. Our own rule says an agent's own check does not
count. Nobody has done that read yet. It is about ten minutes for three pages, and we can produce
the comparison sheets. *This is the one only you can do.*

**Q-014 — a writer, later.** A tool that *creates* RTL PDFs could make our test files correct by
construction. Nice, but it is a new component and we do not need it for the canary. *Later.*
