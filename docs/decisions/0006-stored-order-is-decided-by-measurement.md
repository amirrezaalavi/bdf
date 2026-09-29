# 0006 — Stored order is decided by measurement; no third-party reordering ships

Status: accepted (2026-09-29). Supersedes nothing; extends ADR `0002` (logical order or a loud
failure) rather than relaxing it.

## Context

14 of our 51 real archive files are order-refused. Triaging them (2026-09-29) showed they are
structurally alike — `/Type0` + `/Identity-H`, a `/W` array present, `/ToUnicode` present and
mapping to **base** letters, no `/ActualText`, no `/ReversedChars` — so the ladder's first two
rungs cannot apply and the painting is the only evidence in the file. They span 8 producer
families, 7 of them Microsoft (`Print To PDF` and Office export).

Two measurements decided this ADR:

1. After modelling glyph advance widths (`2936662`), we still refuse all 14. So the refusal is
   not "the file declares nothing"; it is that our comparison does not reach a verdict.
2. poppler's `pdftotext -layout -enc UTF-8` returns coherent **logical** Persian for the same
   files while `-raw` returns the same words **reversed**. External research (`Q-R1`, in
   `docs/research/2026-09-29-external-answers.md`) names the mechanism: `TextOutputDev`'s
   `reorderText` reverses maximal RTL runs when `rawOrder=false`.

Together: the content stream is painted-visual, and **the information needed to recover logical
order is present in the file's geometry**. The refusal is honest but not final.

## Decision

1. **Geometry decides, a name only disambiguates.** Rung 3 compares the *measured* painted
   order against the bidi-derived logical order for the same line. A producer fingerprint may
   break a tie between two measured hypotheses; it may never be the sole reason text is emitted.
2. **No shipped dependency reorders our text.** poppler (GPL) and MuPDF (AGPL) are CI oracles,
   like the existing pixel oracles; the Unicode licence is fine, but the *decision* stays ours
   and testable. We implement the inverse of UAX #9 rule L2 ourselves, with a test per trap
   (digit runs, BD16 brackets, lam-alef clusters, NSM marks).
3. **PDFium is the independent oracle, not a dependency of the default build.** Its
   `FPDFText_GetCharBox` / `FPDFText_GetCharOrigin` (BSD-3, stable public API) give per-glyph
   positions from a second implementation. We use them to classify files in research and in CI,
   and to validate our own verdicts. Putting PDFium into the shipped default build is a separate
   decision with a separate cost, and is not taken here.
4. **The allow-list stays empty until an entry has a test we ran.** A published producer table
   is a set of hypotheses. Each accepted entry needs a fixture, an expected order, and the
   reason it is allowed to emit.
5. **Lexical scoring is excluded from the product.** No prior art, no defensible margin, severe
   failure modes; it may live as an offline research instrument only.
6. **Refusing is still a complete answer.** For any page the geometry cannot decide, we withhold
   the text, count it, and name the reason. Nothing in this ADR weakens that: it only removes
   "we have not measured it yet" as a cause.

## Consequences

- CI gains a PDFium-based oracle lane (`pypdfium2`, venv — never linked into the shipped
  crates), which needs the same oracle-vs-gate discipline as `qpdf`/poppler do today.
- The reason vocabulary must be able to say *which* two hypotheses were compared and which won;
  `bidi_reordered` already exists and keeps its meaning.
- If the geometry still cannot decide for a family after this work, that family is documented as
  permanently refused with its reason — an explicit, written outcome instead of a suspicion.
- The Microsoft family (7 of 14) is the first target, and the most valuable: it is the most
  common real-world family we have seen.
