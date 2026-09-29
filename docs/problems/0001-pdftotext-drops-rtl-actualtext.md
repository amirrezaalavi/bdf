# 0001 — `pdftotext` silently drops Arabic-script text that comes from `/ActualText`

* **Found:** 2026-09-27, while validating the first RTL fixture
* **Severity:** high for our *verification strategy*, not a bug in our code
* **Status:** understood, policy recorded below
* **CORRECTED 2026-09-27 — two things in this file are wrong, and the fix is in
  [`0004`](0004-no-oracle-is-byte-faithful-for-rtl.md).** (a) The tool was never poppler:
  the `pdftotext` on this host's `PATH` is **Xpdf 4.00** (`Copyright 1996-2017 Glyph & Cog,
  LLC`, `/mingw64/bin/pdftotext`). (b) The cause is not the font — it is the **output
  charset**. With `-enc UTF-8`, real poppler returns the Persian in logical order (but flips
  every lam-alef pair: `سلام` → `سالم`) and Xpdf returns it in visual order; without it, both
  drop RTL characters whatever `LANG`/`LC_ALL` say. Read the correction before quoting
  anything below: the *conclusion that no extractor can be ground truth* stands, the
  *reason* does not.

## What we saw

The generator `tools/gen_actualtext_fixture.py` writes the Persian strings `سلام دنیا`
and `می‌روم` into `/ActualText` marked content over dummy Latin glyphs. Expected: an
extractor honours `/ActualText` (ISO 32000-1 §14.9.4). Actual: `pdftotext` printed
**nothing** — not even the placeholder dots.

## The experiment

Five variants of the same page, same writer, one variable each
(`C:/Users/netcon/AppData/Local/hermes/profiles/yolka/cache/scratch/mc_probe.py`):

| variant | content | `pdftotext` output |
|---|---|---|
| A | plain `(.....) Tj`, no marked content | `.....` |
| B | `/Span <</ActualText <FEFF…Persian…>>> BDC (.....) Tj EMC` | **`''` (empty)** |
| C | same, literal string syntax (malformed by the probe) | `3D'E` (garbage — probe bug) |
| D | `/Span <</Lang (fa)>> BDC … EMC`, no `/ActualText` | `.....` |
| E | `/Span <</ActualText <FEFF…"Hello"…>>> BDC … EMC` | **`Hello`** |
| F | two runs: Persian `/ActualText`, then Latin `/ActualText " end"` | **`end`** — the Persian run vanished |

## Conclusion

`pdftotext` **does** implement `/ActualText` substitution (E proves the mechanism, and in
B/F the placeholder glyphs are correctly *replaced*), but it **silently discards
Arabic-script characters** in this configuration. Most likely cause: the substituted text
is emitted through the *font's* encoding/Unicode map, and the fixture's font is Helvetica
(standard encoding, no Arabic), so the codepoints are unmappable and dropped without a
warning. The failure is font-gated, not script-gated in the abstract — but the practical
consequence is what matters.

## Consequences for this project

1. **No `pdftotext` mode is an oracle for RTL text identity.** *(Revised 2026-09-27: the
   original text here said a RTL comparison against poppler is invalid — too strong, and it
   named the wrong binary.)* With `-enc UTF-8`, poppler's layout mode **may** be used to
   cross-check *order* only; it flips every lam-alef pair, so it must never be used to judge
   text identity. Xpdf's layout mode may not be used for mixed-direction lines (run-level
   visual order) and any mode without `-enc UTF-8` returns no RTL at all. Also true, and the
   only thing that survived unchanged: poppler/Xpdf are LTR-control and structural oracles,
   never ground truth. See `0004`.
2. **PDFium moves from "nice to have" to required** for RTL verification: pixel goldens
   plus its own text extraction (P2 task). Until then, RTL fixtures are asserted against
   human-verified expected text (see `corpus/AGENT.md` rule 5), not against another tool.
3. **The synthetic fixture stays.** It is still the correct "best case" fixture: the text is
   present in the file and authoritative; our reader must return it verbatim, including the
   ZWNJ in `می‌روم` and the lam-alef in `سلام`. What changed is only who is allowed to
   judge us.
4. **This is product evidence, not just an internal note.** It is a reproducible
   demonstration that mainstream OSS tooling loses Middle-Eastern text silently — the exact
   failure mode this project exists to fix, and a concrete "before" for the sales story.

## Reproduce

```bash
cd /path/to/scratch && python mc_probe.py     # prints the table above
python tools/gen_actualtext_fixture.py && pdftotext corpus/raw/synthetic/actualtext-fa.pdf -
# expected with a correct extractor: two Persian lines; observed with poppler: blank page
```

## Follow-ups

* P2: pin PDFium binaries, add the pixel-golden lane, and re-run this fixture there.
* P1: `extract --json` must return the two Persian strings for this fixture — it becomes
  the first RTL assertion in the corpus, with `Reason::ActualText`.
