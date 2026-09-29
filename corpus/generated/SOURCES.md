# Fixed sentence set — the source text for every generated fixture

One file, reused by every producer (determinism rule). Each sentence isolates a known
hard case, so when a producer gets it wrong we know *which* mechanism failed.

Priority order (decided 2026-09-27): **fa + en first, then ar, then he.** The `mixed`
line is generated from the start, because the bidi boundary is where most extractors
actually break.

| key | text (logical order, exactly as it must come back out) | what it isolates |
|---|---|---|
| `fa-zwnj-lamalef` | `نیم‌فاصله و لا اله الا الله — ۱۴۰۳/۰۵/۱۲` | ZWNJ (U+200C), lam-alef (U+0644 U+0627), Persian digits (U+06F0–U+06F9), RTL punctuation |
| `fa-zwnj-word` | `می‌روم` | ZWNJ inside a word; renders identically to `میروم` but is a different codepoint sequence — visual review cannot catch this one |
| `fa-plain` | `سلام دنیا` | lam-alef at a word start; the ligature that degrades in `ToUnicode` |
| `en-control` | `pdfrtl v0.1 — ISO 32000-1` | LTR control: digits, hyphen, section sign, Latin |
| `mixed-fa-en` | `گزارش فنی pdfrtl v0.1 — ISO 32000-1 §9.7.4.3 — 42%` | RTL run + LTR identifier + Latin punctuation in one line — the real bidi boundary |
| `ar-harakat` | `اللغة العربية مُشكَّلةٌ بالحركات — ٢٠٢٤` | Arabic diacritics (tashkeel) and Arabic-Indic digits |
| `he-niqqud` | `שָׁלוֹם עוֹלָם — 2024` | Hebrew niqqud with Latin digits |

## Rules

1. **This file is the ground truth for the text**, not any producer's output. A fixture's
   `expected/<id>.json` copies from here; it does not transcribe what a tool happened to emit.
2. **Byte-exact.** Copy these strings with a copy-paste that preserves ZWNJ and combining
   marks. If a normalizer ever "helpfully" rewrites them, that is the bug to chase.
3. **Do not edit in place to make a test pass.** Changing this file invalidates every
   generated fixture at once; regenerate them in the same commit if it must change.
4. `fa-zwnj-word` exists specifically because **visual review cannot detect the difference**
   between `می‌روم` (U+200C) and `میروم`. Only a codepoint comparison can. That is why the
   corpus asserts text, and why "looks right" is never accepted as evidence.

## Producers

Rendered by `tools/make_producer_fixtures.py` (Chrome now; Word and the LaTeX/ReportLab
container producers later). Fixtures land in `corpus/raw/generated/<producer>/<id>.pdf`.
