# corpus/AGENT.md

The corpus is the evidence behind every correctness claim. Treat it as product code.

## Rules

1. **No fixture without a manifest row** in `corpus/manifest.json`. A file the manifest
   does not know about is silently untested — worse than absent.
2. **Provenance is mandatory:** `source_url`, `licence`, `sha256`. Unknown licence ⇒ write
   `unknown` in the `licence` field; never infer one.
3. **`redistributable: false` ⇒ `raw/private/`** (gitignored). Commit the manifest row and
   the hash, never the file. Scanned textbooks are copyrighted; this is not negotiable.
4. **Determinism.** Synthetic fixtures must be byte-identical across runs; generated
   fixtures record the producing tool + version in `notes`.
5. **Ground truth is human-verified.** `expected/<id>.json` carries `verified_by` +
   `verified_at`. Unverified fixtures are `status: unverified` and cannot support a
   released claim. Agents may propose expected text; only the human flips it to verified.
6. **One axis per fixture.** Script, producer, and feature (lam-alef, ZWNJ, niqqud, digit
   folding) are separate fixtures. Mixed cases are named `*-mixed-*` and live in the
   integration tier.
7. **A failure is a fixture too.** Every file that defeats us becomes a fixture labelled
   `expect_unsupported`, so the honest-refusal path stays tested forever.

## Layout

```
corpus/raw/synthetic/   hand-generated, deterministic (tools/gen_*.py)
corpus/raw/generated/   produced by Word/Chrome/LibreOffice/LaTeX/ReportLab/mPDF
corpus/raw/private/     copyrighted/local-only (gitignored, hash-recorded)
corpus/expected/<id>.json   the logical text we assert, + who verified it
corpus/manifest.json    the index of record
```

## Commands

```bash
python3 scripts/run-corpus.py --update-hashes                 # synthetic/generated only
python3 scripts/run-corpus.py --binary "$CARGO_TARGET_DIR/debug/pdfrtl"
python3 scripts/run-corpus.py --only fa- --only ar-            # subset
```

Report lands in `reports/<timestamp>-corpus.md`; `skip` means a private fixture absent on
this machine (never silently ignored).

## The producer axis (why this corpus is special)

The same sentence rendered by different producers is the highest-value test in the
project: producers disagree about storage order (visual vs logical) and about whether
`/ActualText` exists. `producer_class` in the manifest is filled from **inspected
evidence** (glyph order + `ToUnicode` + marked content), never guessed — it is the only
thing the extraction heuristic is allowed to know about a producer in advance.
