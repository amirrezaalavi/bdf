# corpus/ — fixtures, provenance rules, and how to run them

This directory is the evidence behind every correctness claim we make. Treat it as the
product: a claim without a fixture is a marketing sentence.

## Layout

| path | contents | committed? |
|---|---|---|
| `raw/synthetic/` | hand-generated, deterministic fixtures (`tools/gen_*.py`) | yes |
| `raw/generated/` | same text rendered by real producers (Word, LibreOffice, Chrome, XeLaTeX, ReportLab, mPDF, wkhtmltopdf) | yes |
| `raw/private/` | copyrighted/NDA files used locally only — **gitignored** | no (sha256 only, via manifest) |
| `expected/` | human-verified expected logical text per fixture | yes |
| `manifest.json` | one row per fixture: provenance, licence, producer, sha256 | yes |

## Rules

1. **Every fixture has a manifest row before it is used in a test.** A file without a row
   is invisible to the harness and will not be tested.
2. **`redistributable: false` ⇒ the file lives in `raw/private/`** and only its sha256,
   `source_url`, `licence` and notes are committed. Never commit scanned textbooks.
3. **Determinism.** Synthetic fixtures must be byte-identical across runs
   (`sha256sum` twice). Generated fixtures must record the producing tool + version.
4. **Ground truth is human-verified.** `expected/<id>.json` carries `verified_by` and
   `verified_at`. If nobody verified it, the fixture is `status: unverified` and cannot be
   used to justify a released claim.
5. **One axis per fixture.** A fixture should isolate one variable (script, producer, or
   feature like lam-alef / ZWNJ / niqqud / digit folding). Mixed fixtures are for the
   "integration" tier, clearly named `*-mixed-*`.

## Manifest row schema

```json
{
  "id": "fa-generated-word-001",
  "path": "corpus/raw/generated/word/fa-zwnj-lamalef.pdf",
  "language": "fa",
  "script": "Arab",
  "direction": "rtl",
  "producer": "Skia/PDF m154",
  "producer_tool": "Google Chrome (headless print-to-pdf)",
  "producer_class": "visual",
  "licence": "own-work",
  "source_url": null,
  "sha256": "…",
  "redistributable": true,
  "status": "expected-recorded",
  "notes": "ZWNJ + lam-alef + Persian digits in one sentence"
}
```

`producer` must be **exactly what the file's `/Info /Producer` reports** (verified by the
harness); `producer_tool` is the human-facing name of the tool that made it. They differ
often — Chrome reports `Skia/PDF m154`. Recording the tool name in `producer` makes the
harness fail, which is how this field was cleaned up in the first place.

`producer_class` ∈ `visual` | `logical` | `unknown` — this field is how the extraction
heuristic is allowed to know anything. It is filled from evidence (a fixture whose
`/ActualText`/glyph order we inspected), never guessed. See `docs/problems/0002` for the
worked Chrome example.

## Running

```bash
bash scripts/run-corpus.sh                       # all fixtures, writes reports/<date>-corpus.md
bash scripts/run-corpus.sh --only fa-           # subset by id prefix
```

Each row in the report must show: fixture id · expected vs actual (pass/fail) · the
`Reason` code returned · the exit code. A fixture we cannot handle is a **pass** if and
only if it returns the honest `unsupported_*` reason — refusing is a correct outcome,
guessing is not.

---

**Public mirror note:** private-corpus fixtures (real customer documents) are registered and hashed only in the author's local working copy, which is why this repo lists fewer fixtures than `docs/ROADMAP.md` describes. Nothing about those documents is published here: not their names, not their hashes, not their metadata.
