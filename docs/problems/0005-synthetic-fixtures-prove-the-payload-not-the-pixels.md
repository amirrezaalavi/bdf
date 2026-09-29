# 0005 — the synthetic fixtures prove the payload, not the pixels

**Found:** 2026-09-29, while doing the standing human text review of
`corpus/raw/synthetic/actualtext-{fa,ar,he}.pdf`.
**Severity:** not a bug in the extractor; a *coverage claim that was too broad*.
**Status:** understood, description corrected, one policy decision left open (below).

## What the fixtures actually contain

The generator says it in one line (`tools/gen_actualtext_fixture.py`):

```
PLACEHOLDER = "."  # Helvetica renders Latin; real glyphs are irrelevant to this fixture
```

Each line is a single marked-content run whose drawn text is **ASCII periods, one per
character of the sentence**, in Helvetica, wrapped in
`/Span << /ActualText <UTF-16BE of the sentence> >> BDC (....) Tj EMC`. There is no
Arabic/Hebrew/Persian glyph anywhere in the file, no embedded font, no `/ReversedChars`,
and no un-annotated run.

Rendered to PNG and read by a vision model, all three pages show dots. That is the fixture
rendering *correctly*, not a font failure — and it is why a human cannot text-verify these
pages by looking at them.

## What that means (corrected description)

| Claim | True? |
|---|---|
| Extraction of these files returns the sentence in logical order | yes — and by construction the /ActualText payload is the only place the text exists |
| They test the `/ActualText` rung of the order ladder | **yes, and harder than a realistic file does**: the drawn glyphs carry no recoverable text, so an extractor that ignores `/ActualText` returns `"........"` and fails loudly |
| They mimic Chrome's real structure | **no.** Real Chrome emits per-cluster `/ActualText` in visual order, un-annotated runs, and a `/ReversedChars` wrapper around the whole thing (docs/problems/0002). These have none of that |
| They exercise visual→logical inversion | **no.** No visual-order signal is present, so no inversion is performed |
| They can be human-verified by reading the rendered page | **no** — the page is dots |

Real Chrome fixtures (`fa-zwnj-lamalef`, `mixed-fa-en`, …) *do* render readable script and
remain the visual-review fixtures. Public coverage of the inversion therefore comes from
`reversed_chars_marker_is_order_evidence_no_matter_the_producer` (two byte-identical content
streams differing only in the `/ReversedChars` marker) plus the real archive locally — not
from these files.

## Consequence 1 — the harness asserts nothing about them

`run-corpus.py` reports all three as
`skip — expected text present but unverified by a human`. The corpus rule (rule 5) exists so
that an *agent's guess* never becomes an assertion. Here there is no guess: the expected
string **is** the generator's input, and the generator is in the repo. Treating them as
"unverified" leaves three committed fixtures asserting nothing, and inflates the honest skip
count.

**Recommendation (needs the owner's call, since it amends rule 5):** add a distinct
provenance for this class — `verified_by: "construction"` (or `source: generator`) — and let
the harness assert it, because the payload is the fixture's own source and can be regenerated
byte-identically. Keep `unverified → asserted requires a human` for fixtures whose text was
*inferred* from a real document.

## Consequence 2 — do not cite them as producer-fidelity evidence

Any future claim of the form "our corpus covers Chrome RTL" must not lean on these files.
They cover the `/ActualText` rung across three scripts. To cover the inversion publicly we
would need a fixture with real glyph outlines plus a visual-order signal (embedded subset
font + `/ReversedChars`), or a public-domain real document — a separate piece of work, and
worth doing only if the inversion is to be gate-protected on the public mirror.

## Verified by

- Vision read of `reports/render/actualtext-{fa,ar,he}/page-1.png` — dots, no script.
- `tools/gen_actualtext_fixture.py:56,65-75` — the drawn string is `"." * len(sentence)`.
- `pdfrtl --json extract` on all three: `exit=0`, `ok=true`, reasons `["actual_text"]`,
  text exactly the sentence, `unordered_chars=0`.
