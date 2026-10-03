# 0013 — hebrew-1.pdf was never the broken file I said it was

**Status:** closed by measurement
**Date:** 2026-10-03
**Supersedes:** the C2 premise in `docs/plans/2026-10-03-canary.md`, and any earlier note
describing this file's output as "wrong-script garbage" (ROADMAP W1.6 row, the lane plan's
font-track framing).

## What I believed

1. "hebrew-1.pdf emits 47,897 characters of broken-CMap (wrong-script) text while reporting
   `ok:false`."
2. "It needs font-program recovery: parse the embedded font's cmap and reverse GID → Unicode."
   Nine fonts, zero `/ToUnicode`, `Identity-H`, no `/CIDToGIDMap`.

Both statements drove a workstream. Both are false.

## What the file actually is

An English/Spanish academic volume with Hebrew quotations — which the notes already said
in one place and contradicted in another. Measured on 2026-10-03:

* **All 29 pages emit readable text.** The opening is
  `Mizmor Le-David: Studies in Jewish Languages` / `CONSEJO SUPERIOR DE INVESTIGACIONES
  CIENTÍFICAS` / `Madrid, 2023` / `TABLE OF CONTENTS`. Every alphabetic character in the
  first 5,000 is LATIN.
* **The Latin decodes correctly through the fonts' `/Encoding`** — `Reason::EncodingMapped`
  on every page. That is the simple-font path working as designed, not a fallback.
* Every page reports `ok:false` with `unsupported_broken_to_unicode`, because the Hebrew runs
  use fonts with neither a usable `/ToUnicode` nor an embedded program. The page is honest:
  it keeps what it can prove and names what it cannot.

## Why font-program recovery cannot apply — at all

Page 1 carries four fonts:

| font | subtype | `/ToUnicode` | embedded program | `/CIDToGIDMap` |
|---|---|---|---|---|
| `/R11` | Type0 | no | **none** | no |
| `/R14` | TrueType | no | **none** | no |
| `/R16` | TrueType | no | **none** | no |
| `/R18` | Type1 | no | **none** | no |

`FontFile`, `FontFile2` and `FontFile3` are all absent. There is no font program to invert a
cmap from and no `post` table to read names from. The recovery technique planned for this file
does not apply to it, and no amount of implementation would change that.

## The rule

**A recovery plan needs the recovery material to exist.** Before scheduling "recover identity
from the embedded font", assert that the fonts actually embed a program. A file with no
`/FontFile*` and no `/ToUnicode` has nothing to recover from — the correct answer is a named
refusal, which is what the code already produced.

Corollary: **check the emitted text before characterising it.** The phrase "wrong script" was
an inference from a character count plus a remembered failure, not a reading of the output. One
`repr()` of the first 200 characters would have shown a table of contents.

## What C2 becomes

Not an implementation. A verification: the canary's refusal path for a file whose fonts offer
no recoverable mapping is correct, and the file is a good acceptance case for
`unsupported_broken_to_unicode`. No code change required.

## Reference note (checked, so it is not re-checked later)

`al-bdf-engine` cannot help either. Its RTL engine only ever *writes* a `/ToUnicode` CMap
(`pdfrtltextengine.cpp:591-630`); `pdffont.cpp:1777` loads `FontFile2` for the fallback-font
machinery, not for glyph→Unicode recovery. Its own `docs/PROBLEMS.md` **P4** is this exact
case — "Foreign PDFs with broken ToUnicode (glyphs → C0 control chars)" — and the fix was to
sanitise the characters so the content stream serialises through XML, not to recover them.
Its embedded program would also be MoPDF (AGPL), which `docs/decisions/0003` keeps as a CI
oracle and never ships.

The three traps its failure log did contribute, worth keeping: `ToUnicode` is one UTF-16 unit
per glyph so ligatures degrade; HarfBuzz ≥ 4 already emits RTL runs leftmost-first so
re-reversing mirrors text; and a short `/CIDToGIDMap` makes strict renderers paint Latin glyphs.