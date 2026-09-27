# Corpus acquisition — what to pull, and where

This is the shopping list the human runs: copyrighted or login-gated sources that agents
cannot fetch. Everything an agent *can* generate itself is in `corpus/raw/generated/`.

Rules reminder (`corpus/README.md`): record `source_url`, `licence`, `sha256` in
`corpus/manifest.json` in the same commit. `redistributable: false` ⇒ file goes in
`corpus/raw/private/` (gitignored) and only its hash is committed.

## Tier 1 — public domain / CC (safe to publish fixtures from)

These are classic works whose PDFs are widely mirrored. Verify each file's own
jurisdiction/licence before committing; prefer Wikisource/Sefaria/Internet Archive copies
that state a licence.

| # | language | candidate | why it is a good test | likely source |
|---|---|---|---|---|
| 1 | fa | *گلستان سعدی* (Saadi, Gulistan) | classic Persian prose, high proportion of ZWNJ and Arabic-style yeh/keheh | Wikisource fa, Internet Archive |
| 2 | fa | *کلیله و دمنه* (Nasrallah Monshi) | long prose, lam-alef and tashkeel in older orthography | Wikisource fa, آذرمهر/ganjoor exports |
| 3 | fa | *مثنوی معنوی* (Rumi) | poetry → line-level layout, mixed Persian/Arabic | ganjoor, Internet Archive |
| 4 | fa | *دیوان حافظ* | poetry, diacritics in some editions | Wikisource fa |
| 5 | fa | Iranian school textbook, *ریاضی پنجم دبستان* | real-world mixed fa/en (numbers, formulas), the exact case you cited | chap.sch.ir (⚠ copyrighted: `redistributable: false`) |
| 6 | ar | *كليلة ودمنة* (Ibn al-Muqaffa) | classic Arabic prose, harakat, lam-alef | Wikisource ar, shamela.ws |
| 7 | ar | *ألف ليلة وليلة* | long text, diacritics, mixed narrative | Wikisource ar, Internet Archive |
| 8 | ar | *مقدمة ابن خلدون* | dense prose, tables of contents, footnotes | shamela.ws, مكتبة نور |
| 9 | ar | *مقامات الحريري* | ornate style: heavy tashkeel + ligatures — the hardest Arabic text class | Wikisource ar |
| 10 | he | תנ"ך (Tanakh) export | bare Hebrew + niqqud in some editions; CC0 via Sefaria | Sefaria (API/PDF export) |
| 11 | he | משנה (Mishnah) | niqqud + Aramaic-latin mix, small type | Sefaria |
| 12 | he | *ספר לימוד חשבון כיתה ה* (Hebrew maths textbook) | real-world he/en mixed, the Hebrew counterpart of case 5 | Israeli ministry/open textbook sites (⚠ verify licence) |
| 13 | en | *Pride and Prejudice* (Gutenberg) | LTR control; sanity baseline for extraction | Gutenberg |
| 14 | en | *Moby-Dick* | LTR control with long paragraphs/punctuation | Gutenberg |
| 15 | en | 2–3 arXiv papers with tables/figures | math layout, CID fonts, ligatures; LTR control for structure | arXiv |
| 16 | mixed | any fa/ar/he document containing Latin identifiers, URLs, formulas | the **hardest and most valuable** class: bidi boundary behaviour | ours (generate) |

## Search queries to run (copy-paste)

```text
کتاب درسی ریاضی پایه پنجم دبستان pdf        # fa textbook
کتاب فارسی پنجم دبستان pdf
گلستان سعدی pdf                            # fa literature
کلیله و دمنه pdf
مثنوی معنوی pdf
كتاب اللغة العربية للصف الخامس pdf          # ar textbook
مقدمة ابن خلدون pdf
ألف ليلة وليلة pdf
مقامات الحريري pdf
ספר לימוד חשבון כיתה ה pdf                  # he textbook
תנ"ך pdf                                   # he scripture
ספרות עברית קלאסית pdf
persian arabic hebrew bilingual pdf form    # mixed-direction forms
```

Practical notes for whoever runs these:
* Prefer files that are **text-based** (selectable text) over scans — scans test OCR, not
  our text pipeline. Both are useful, but label them: `notes: "scanned, text layer absent"`.
* A scan with a bad vendor text layer is *treasure*: it is exactly the `unsupported_*`
  path we must handle honestly.
* Prefer files ≥3 pages with paragraphs (not one-line certificates) so reading order and
  bidi boundaries are actually exercised.
* Save with a stable name: `<lang>-<source>-<shortslug>.pdf`, e.g. `fa-wikisource-gulistan.pdf`.

## Tier 2 — producer variants (agents generate these; no external download)

One fixed sentence set per script, rendered by each available producer. The same text
through different producers is what exposes the visual-vs-logical split — this is the
single most valuable part of the corpus.

| producer | present on this Windows host | notes |
|---|---|---|
| Microsoft Word | <yes> `WINWORD.EXE` found | "Save as PDF" — expect visual order per field evidence |
| Chrome / Edge print-to-PDF | <yes> Chrome + Edge found | expect visual order, per-glyph `Tj`, `/ReversedChars` |
| LibreOffice | **not installed** | install when needed; known visual-order producer |
| XeLaTeX / LuaLaTeX | no TeX on host | produce inside the Docker/WSL image |
| ReportLab (Python) | no Python PDF lib on host yet | logical-order producer; `pip install reportlab` in a venv |
| mPDF / wkhtmltopdf | no PHP/wkhtmltopdf | one visual and one logical producer, both cheap in a container |
| InDesign | unknown | only if available; high-quality output worth having |

Sentence set (define in `corpus/generated/SOURCES.md`, one file, reused by every
producer — determinism rule):

1. **fa**: `نیمفاصله و لا اله الا الله — ۱۴۰۳/۰۵/۱۲` (ZWNJ, lam-alef, Persian digits)
2. **ar**: `اللغة العربية مُشكَّلةٌ بالحركات — ٢٠٢٤` (harakat, lam-alef, Arabic-Indic digits)
3. **he**: `שָׁלוֹם עוֹלָם — 2024` (niqqud)
4. **mixed**: `گزارش فنی pdfrtl v0.1 — ISO 32000-1 §9.7.4.3 — 42%` (RTL + Latin + punctuation)

## Tier 3 — negative fixtures (must be generated on purpose)

| fixture | purpose |
|---|---|
| `*-no-tounicode.pdf` | font without `ToUnicode` and no `/ActualText` → must return `unsupported_broken_to_unicode`, exit 3 |
| `*-encrypted.pdf` | password-protected → `inspect` must report `encrypted: true` and `extract` must fail cleanly (exit 4), not crash |
| `*-empty-page.pdf` | zero text on a page → empty result, `ok: true`, no reason |
| `*-broken-xref.pdf` | truncated file → `ok: false`, exit 4 with the file named |

These prove the honesty half of the contract. A tool that never says "I don't know" is
not trustworthy, and this is the cheapest way to demonstrate the opposite.
