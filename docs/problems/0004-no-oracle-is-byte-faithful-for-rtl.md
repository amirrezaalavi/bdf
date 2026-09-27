# 0004 — every third-party extractor corrupts RTL differently, and none is byte-faithful

**Measured:** 2026-09-27, by the orchestrator, on the five generated fixtures, each compared
codepoint-by-codepoint against `corpus/generated/SOURCES.md` (the authority — see its rule 1).
**Corrects:** `docs/problems/0001` (wrong attribution: the `pdftotext` on this host is Xpdf,
not poppler) and the oracle note in `AGENTS.md`.

## Why this was measured

Two independent lanes disagreed about what the oracles do, and a documented problem statement
rested on a tool name nobody had checked. So: run every available extractor, in every mode, on
the same files, and compare the raw codepoints against the authority strings.

## The matrix

| extractor / mode | order | ZWNJ | lam-alef | usable as an oracle? |
|---|---|---|---|---|
| **pdfrtl (ours)** | logical ✅ | preserved ✅ | intact ✅ | it is the thing under test |
| Xpdf 4.00, layout, `-enc UTF-8` | logical on pure-RTL lines ✅ / **run-level visual** on the mixed line ❌ | **present but displaced, plus a spurious space** ❌ | intact | order only, with reservations |
| Xpdf 4.00, `-raw`, `-enc UTF-8` | visual ❌ | present ✅ | intact | no |
| Xpdf 4.00, any mode, **no** `-enc UTF-8` | **all RTL characters silently dropped** ❌ | n/a | n/a | no |
| poppler 26.01, layout, `-enc UTF-8` | logical ✅ | preserved ✅ | **every pair flipped** ❌ | order only — never text identity |
| poppler 26.01, `-raw`, `-enc UTF-8` | visual/scrambled ❌ | ❌ | ❌ | no |
| PDFium 5.13 (`pypdfium2`), text | per-cluster visual, matches neither logical nor reversed ❌ | **dropped** (0 where 1 expected) ❌ | n/a | pixels only, never text |
| PDFium 5.13, rendered pixels | correct ✅ | correct ✅ | correct ✅ | **this is the oracle** |

## The raw evidence

fa-plain, expected `U+0633 U+0644 U+0627 U+0645 U+0020 U+062F U+0646 U+06CC U+0627` (`سلام دنیا`):

```
ours             : U+0633 U+0644 U+0627 U+0645 U+0020 U+062F U+0646 U+06CC U+0627   byte-exact ✅
poppler layout   : U+0633 U+0627 U+0644 U+0645 ...   → سالم دنیا   ← lam-alef flipped ❌
xpdf -raw        : U+0627 U+06CC U+0646 U+062F U+0020 U+0645 U+0627 U+0644 U+0633   ← exact reverse ❌
xpdf default     : no Arabic bytes at all                                            ← dropped ❌
```

fa-zwnj-lamalef, expected `نیم\u200cفاصله و لا اله الا الله — ۱۴۰۳/۰۵/۱۲`:

```
ours             : نی م\u200c فاصله و لا اله الا الله — ۱۴۰۳/۰۵/۱۲      byte-exact ✅
xpdf layout      : نی [SPACE]\u200c مفاصله و لا اله الا الله — ...        ZWNJ moved, space injected ❌
poppler layout   : نی م\u200c فاصله و ال اله اال هللا — ...              ZWNJ kept, lam-alef flipped ❌
```

mixed-fa-en, expected `گزارش فنی pdfrtl v0.1 — ISO 32000-1 §9.7.4.3 — 42%`:

```
ours             : گزارش فنی pdfrtl v0.1 — ISO 32000-1 §9.7.4.3 — 42%   byte-exact ✅
xpdf layout      : pdfrtl v0.1 — ISO 32000-1 §9.7.4.3 — 42% گزارش فنی   ← runs swapped ❌
```

## The attribution error

Problem 0001 and the AGENTS.md note said "**poppler** drops Persian". The `pdftotext` on the
Windows `PATH` is:

```
pdftotext version 4.00
Copyright 1996-2017 Glyph & Cog, LLC        → /mingw64/bin/pdftotext   (Xpdf, not poppler)
```

Real poppler is present in WSL and behaves differently:

```
pdftotext version 26.01.0
Copyright 2005-2026 The Poppler Developers  → poppler
```

The conclusion that survived is narrower and tool-dependent: **any mode without `-enc UTF-8`
drops RTL characters** (the default charset is Latin-1-ish, and `LANG`/`LC_ALL` do not change
it), while with `-enc UTF-8` both families return Persian — Xpdf in visual/displaced form,
poppler in logical form with lam-alef flipped. Every recorded result must name the binary that
produced it (`pdftotext -v`).

## Why this strengthens the project rather than threatening it

`سلام` → `سالم` is the ideal demonstration of a wrong answer that survives review: it is a
plausible-looking word, and every other character in the line is correct. That is exactly the
failure class `SOURCES.md` rule 4 was written for, and it is why "looks right" is not evidence.

## Consequences for the verification policy

1. Ground truth is `corpus/generated/SOURCES.md` **plus a human/vision read of the rendered
   page**. No extractor's output is ever ground truth.
2. poppler layout + `-enc UTF-8` is a cross-check for **order only**. It must not be used to
   check text identity: it flips lam-alef pairs silently.
3. PDFium is the **pixel** oracle. Its text output must never be used to check ZWNJ fidelity.
4. Xpdf's layout mode must not be used for the mixed-direction case: it returns run-level
   visual order there while looking correct on pure-RTL lines.
5. Any evidence quoting a third-party extractor records `pdftotext -v` output alongside it.
