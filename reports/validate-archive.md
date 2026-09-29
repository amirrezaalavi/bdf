# Archive validation — `pdfrtl extract` over the private corpus

* generated: `2026-09-29T115928Z`
* command: `python3 scripts/validate-archive.py --binary /home/netcon/target-bdf/debug/pdfrtl`
* input: `corpus/raw/private/desktop-pdfs/*.pdf` — 51 files, read only (never modified, never copied)

## Headline — two claims, counted separately (ADR 0004)

| claim | files | meaning |
|---|---|---|
| **fully decoded** | **38** / 51 | every page decoded — no `unsupported_*` decode refusal anywhere |
| **logical order verified** | **37** / 51 | every page's order established by a named rule — the rule is named per file below |

Of the 37, **14** files actually emit right-to-left text — and that is every file in the corpus that emits any RTL text at all (15 of 51). The other 23 were verified as having nothing to order: no RTL text, or no text layer. Every file is accounted for by exactly one row below.

| emission | files | meaning |
|---|---|---|
| text emitted, all pages decoded | 23 | exit 0, `ok: true` |
| text emitted, some pages refused | 12 | exit 3, `ok: false`; `data.text` holds only order-established pages |
| no text emitted — refused | 12 | a refusal is recorded; decoded characters survive as `unordered_chars` |
| no text layer | 4 | nothing to extract (image-only or blank): no refusal, nothing withheld |

Characters emitted (order-established): **1,038,880**. Characters decoded but withheld (order unestablished): **1,271,919**.

> "Decoded" and "ordered" are different claims. A file can read every glyph and still refuse to say which order those glyphs are in — that is what `unordered_chars` counts, and why this report has no single "recovered" number. Text whose order is unestablished is never present in `data.text`; it is counted instead.

## Why pages still fail

| reason | kind | files affected | pages affected |
|---|---|---|---|
| `unsupported_broken_to_unicode` | decode | 11 | 11 |
| `unsupported_font_encoding` | decode | 4 | 4 |
| `unsupported_page_content` | decode | 2 | 2 |
| `unsupported_visual_order` | order | 14 | 14 |

## Producer families

| producer | status | files |
|---|---|---|
| Microsoft® Word LTSC | full | 7 |
| pdfplot16.hdi 16.01.051.00000 | full | 6 |
| Microsoft: Print To PDF | refused | 6 |
| Adobe PDF Library 17.0 | partial | 4 |
| Microsoft: Print To PDF | full | 2 |
| Synopse PDF engine 1.18.6395 | full | 2 |
| Adobe Acrobat Pro 11.0.0 | partial | 1 |
| Adobe PDF Library 16.0.7 | partial | 1 |
| Microsoft® Excel® LTSC | partial | 1 |
| PDFlib+PDI 9.0.2p2 (C++/Win64) | full | 1 |
| Qt 4.8.7 | full | 1 |
| Mac OS X 10.10 Quartz PDFContext | partial | 1 |
| Nitro PDF PrimoPDF | partial | 1 |
| Microsoft® Word 2013 | partial | 1 |
| unknown | refused | 1 |
| Microsoft® Excel® 2016 | refused | 1 |
| cairo 1.18.4 (https://cairographics.org) | refused | 1 |
| Microsoft® Word 2021 | full | 1 |
| Acrobat Distiller 11.0 (Windows) | full | 1 |
| Stimulsoft Reports | full | 1 |
| ReportLab PDF Library - (opensource) | refused | 1 |
| Corel PDF Engine Version 18.1.0.661 | partial | 1 |
| Nitro PDF Pro 14 (14.39.0.18) | no_text | 1 |
| Foxit Reader PDF Printer Version 9.4.0.1491 | partial | 1 |
| Microsoft® Word 2024 | full | 1 |
| intsig.com pdf producer | no_text | 1 |
| Adobe Acrobat 21.1 Image Conversion Plug-in | no_text | 1 |
| Pdftools SDK | no_text | 1 |
| cairo 1.18.0 (https://cairographics.org) | refused | 1 |
| Microsoft® Excel® 2013 | refused | 1 |

## Per file

| file | status | decoded pages | ordered pages | emitted chars | withheld chars | order rule | reasons | first 60 chars |
|---|---|---|---|---|---|---|---|---|
| `arabic-1.pdf` | partial | 4/91 | 91/91 | 171266 | 0 | yes — actual_text, bidi_reordered, producer_visual_order_known | actual_text, bidi_reordered, bidi_verified, producer_visual_order_known, unsupported_broken_to_unicode | المبادئ التوجيهيّة لمنظّمة  ⏎ ّ ⏎ ّ ⏎ الصحّة العالميّة بشأن  ⏎ ّ ⏎ ّ ⏎ م |
| `arabic-2.pdf` | partial | 20/62 | 62/62 | 200299 | 0 | yes — actual_text, bidi_reordered, producer_visual_order_known | actual_text, bidi_reordered, bidi_verified, producer_visual_order_known, unsupported_broken_to_unicode |  إطار منظمة الصحة ⏎  العالمية للإستجابة ⏎ ئ ⏎ للطوار ⏎ اإلجراءات الد |
| `arabic-3.pdf` | partial | 228/321 | 2/321 | 5 | 1187033 | no — unsupported_visual_order | bidi_verified, unsupported_broken_to_unicode, encoding_mapped, unsupported_font_encoding, unsupported_visual_order |    ⏎    |
| `arabic-4.pdf` | partial | 10/185 | 185/185 | 116762 | 0 | yes — actual_text, to_unicode_logical, bidi_reordered, producer_visual_order_known | actual_text, to_unicode_logical, bidi_reordered, bidi_verified, producer_visual_order_known, unsupported_broken_to_unicode, encoding_mapped, unsupported_font_encoding | تقرير التنمية االنسانية العربية للعام   ⏎  يشمل الجميع ويعزز ا |
| `english-1.pdf` | partial | 3/3 | 2/3 | 3832 | 2784 | no — unsupported_visual_order | encoding_mapped, unsupported_visual_order | No. : ⏎ 12345 ⏎ kA : ⏎ 65KA50KA36KA36KA25kA ⏎ IP : ⏎ 4242424242 ⏎ QTY : ⏎  |
| `english-asnad-9-32.pdf` | full | 1/1 | 1/1 | 1642 | 0 | yes — bidi_reordered, producer_visual_order_known | bidi_reordered, bidi_verified, producer_visual_order_known, encoding_mapped |    ⏎   ⏎   ⏎   ⏎   ⏎ الزامات مربوطه در عملیات فیبرکشی و نیز فیوژن و چی |
| `english-asnad-9-39.pdf` | full | 1/1 | 1/1 | 2576 | 0 | yes — producer_visual_order_known | producer_visual_order_known, encoding_mapped |    ⏎   ⏎   ⏎   ⏎   ⏎  . دیواره لوله را نیز در فواصل مختلف سوراخ خواهد  |
| `english-asnad-9-40.pdf` | full | 1/1 | 1/1 | 1324 | 0 | yes — producer_visual_order_known | producer_visual_order_known, encoding_mapped |    ⏎   ⏎   ⏎   ⏎   ⏎   در محیط باشد توان سیستم خنک کننده ارتباط مستقیم |
| `english-asnad-9-42.pdf` | full | 1/1 | 1/1 | 1642 | 0 | yes — bidi_reordered, producer_visual_order_known | bidi_reordered, bidi_verified, producer_visual_order_known, encoding_mapped |    ⏎   ⏎   ⏎   ⏎   ⏎ الزامات مربوطه در عملیات فیبرکشی و نیز فیوژن و چی |
| `english-asnad-9-43.pdf` | full | 1/1 | 1/1 | 1098 | 0 | yes — producer_visual_order_known | producer_visual_order_known, encoding_mapped |    ⏎   ⏎   ⏎   ⏎   ⏎  : سیستمهای کنترل محیطی.1.15.1ِ ⏎  این سیستم از یک  |
| `english-cdid-7jlm9p-r12-en.pdf` | full | 75/75 | 75/75 | 60897 | 0 | yes — to_unicode_logical | to_unicode_logical | Galaxy7000250–500kVAUPS03/2018www.schneider-electric.com ⏎ Ins |
| `english-nexans-fibre-patch-panels-5.pdf` | full | 3/3 | 3/3 | 1257 | 0 | yes — to_unicode_logical | to_unicode_logical | CONTACT ⏎ USSUSTAINABILITYNEWSROOMEnglish ⏎     ⏎ ProductsSegments |
| `english-power-room.pdf` | full | 1/1 | 1/1 | 103 | 0 | yes — to_unicode_logical | to_unicode_logical | POWER ROOM ⏎ SPLIT ⏎ SPLIT ⏎ ATS ⏎ MDP ⏎ MDP ⏎ 80cm ⏎ 80cm100cm ⏎ UDP ⏎ UPS 1 ⏎  |
| `english-sad-prpoposal.pdf` | full | 11/11 | 11/11 | 8105 | 0 | yes — producer_visual_order_known | producer_visual_order_known, encoding_mapped |   ⏎   ⏎   ⏎   ⏎ PROJECT:  ⏎  شرکت صاد   تاسنتر ید  رساختیز ی سازنهیبه  |
| `he-eng-ar.pdf` | partial | 3/3 | 1/3 | 366 | 7604 | no — unsupported_visual_order | to_unicode_logical, encoding_mapped, unsupported_visual_order | Proceedings of the Human Language Technology Conference (HLT |
| `hebrew-1.pdf` | partial | 0/29 | 29/29 | 47897 | 0 | yes — no RTL text | unsupported_broken_to_unicode, encoding_mapped |   ⏎ ORA (RODRIGUE) SCHWARZWALD  ⏎ OFRA TIROSH ⏎ - ⏎ Mizmor Le-David: |
| `hebrew-2.pdf` | partial | 0/4 | 4/4 | 19 | 0 | yes — actual_text | actual_text, unsupported_broken_to_unicode |   ⏎    ⏎    ⏎   ⏎   ⏎    ⏎    ⏎   |
| `hebrew-3.pdf` | partial | 0/44 | 44/44 | 46608 | 0 | yes — actual_text, bidi_reordered, producer_visual_order_known | actual_text, bidi_reordered, bidi_verified, producer_visual_order_known, unsupported_broken_to_unicode, encoding_mapped, unsupported_font_encoding | אגף בכיר לביקורת פנים ולתלונות הציבור ⏎ דין וחשבון שנתי ⏎ מספר 1 |
| `hebrew-4.pdf` | partial | 2/56 | 56/56 | 35607 | 0 | yes — to_unicode_logical, producer_visual_order_known | to_unicode_logical, producer_visual_order_known, unsupported_broken_to_unicode, encoding_mapped, unsupported_font_encoding |   ⏎   ⏎   ⏎  מדינת ישראל ⏎  קליטת העלייההמשרד ל ⏎  ת הפנימית ופניות הצ |
| `persian-1.pdf` | refused | 12/13 | 0/13 | 0 | 8475 | no — unsupported_visual_order | bidi_verified, unsupported_broken_to_unicode, unsupported_visual_order |  |
| `persian-2.pdf` | refused | 1/2 | 0/2 | 0 | 2076 | no — unsupported_visual_order | unsupported_broken_to_unicode, unsupported_visual_order |  |
| `persian-3.pdf` | refused | 2/2 | 0/2 | 0 | 1981 | no — unsupported_visual_order | bidi_verified, unsupported_visual_order |  |
| `persian-4.pdf` | refused | 1/1 | 0/1 | 0 | 1488 | no — unsupported_visual_order | bidi_verified, encoding_mapped, unsupported_visual_order |  |
| `persian-5.pdf` | full | 1/1 | 1/1 | 38 | 0 | yes — to_unicode_logical | to_unicode_logical | 24000 Btu/Hr ⏎ 24000 Btu/Hr ⏎ 24000 Btu/Hr |
| `persian-6.pdf` | refused | 1/1 | 0/1 | 0 | 759 | no — unsupported_visual_order | bidi_verified, unsupported_visual_order |  |
| `persian-7.pdf` | full | 10/10 | 10/10 | 29046 | 0 | yes — producer_visual_order_known | producer_visual_order_known, encoding_mapped |   ⏎  طرف دوم  طرف اول  ⏎  های نوین توسعه امن و پویایی سامانه گرو |
| `persian-8.pdf` | refused | 0/1 | 1/1 | 0 | 0 | yes — no text to order (no text layer) | unsupported_page_content |  |
| `persian-archietectural-1.pdf` | full | 1/1 | 1/1 | 72 | 0 | yes — to_unicode_logical | to_unicode_logical | 112cm ⏎ 4cm ⏎ 237cm ⏎ 7cm ⏎ 0.7 THK ⏎ 244cm ⏎ main entrance door install |
| `persian-catalog-afsharnejad-1.pdf` | full | 31/31 | 31/31 | 14 | 0 | yes — no RTL text | encoding_mapped | ØPwj ⏎ ØPwj ⏎ 5430 |
| `persian-chasis-roof-it02.pdf` | full | 1/1 | 1/1 | 55 | 0 | yes — to_unicode_logical | to_unicode_logical | IT-02 ⏎ LADDER ⏎ Walk Way ⏎ condenser & fan box support frame |
| `persian-hebrew-report-it011-revision.pdf` | full | 33/33 | 33/33 | 50367 | 0 | yes — to_unicode_logical | to_unicode_logical | Date7/13/2026 ⏎ Created with DIALux ⏎ BANK SADERAT ⏎ BANK SADERAT ⏎  |
| `persian-hld-7-summary-fa.pdf` | refused | 3/3 | 0/3 | 0 | 6333 | no — unsupported_visual_order | bidi_verified, unsupported_visual_order |  |
| `persian-inverter-hybrid-growatt-30kw.pdf` | partial | 1/2 | 2/2 | 3710 | 0 | yes — to_unicode_logical | to_unicode_logical, unsupported_broken_to_unicode | ·Multiple MPPTs and battery input ⏎ ·Support DG input and smar |
| `persian-irancell-expansion-layout.pdf` | refused | 0/1 | 1/1 | 0 | 0 | yes — no text to order (no text layer) | unsupported_page_content |  |
| `persian-panel-6.pdf` | no_text | 5/5 | 5/5 | 0 | 0 | yes — no text to order (no text layer) | - |  |
| `persian-panel1.pdf` | refused | 1/1 | 0/1 | 0 | 1629 | no — unsupported_visual_order | unsupported_visual_order |  |
| `persian-payvast1-2.pdf` | partial | 21/21 | 12/21 | 3335 | 5172 | no — unsupported_visual_order | to_unicode_logical, bidi_reordered, bidi_verified, unsupported_visual_order | QTY ⏎ P52534-B21HPE DL380 Gen11 8SFF NC CTO Svr1 ⏎ P48813-B21HPE |
| `persian-proposal-vb-revised-fa-office.pdf` | full | 8/8 | 8/8 | 12532 | 0 | yes — producer_visual_order_known | producer_visual_order_known, encoding_mapped | افزاری، مدیریت شبکه و پایش  پروپوزال ارائه خدمات پشتیبانی نر |
| `persian-report-dg-revision.pdf` | full | 34/34 | 34/34 | 47730 | 0 | yes — to_unicode_logical | to_unicode_logical | Date ⏎ 7/13/2026 ⏎ Created with DIALux ⏎ BANK SADERAT ⏎ BANK SADERAT |
| `persian-report-it02-revision.pdf` | full | 44/44 | 44/44 | 60403 | 0 | yes — to_unicode_logical | to_unicode_logical | Date ⏎ 7/13/2026 ⏎ Created with DIALux ⏎ BANK SADERAT ⏎ BANK SADERAT |
| `persian-report-noc-revision.pdf` | refused | 29/29 | 0/29 | 0 | 42107 | no — unsupported_visual_order | unsupported_visual_order |  |
| `persian-server-room.pdf` | full | 1/1 | 1/1 | 191 | 0 | yes — to_unicode_logical | to_unicode_logical | SPLIT ⏎ SPLIT ⏎ SERVER ROOM ⏎ 400cm ⏎ 550cm ⏎ RACKRACKRACKRACK ⏎      ⏎ 88 |
| `persian-spec.pdf` | full | 1/1 | 1/1 | 334 | 0 | yes — to_unicode_logical | to_unicode_logical | SPLIT ⏎ SPLIT ⏎ SERVER ROOM ⏎ 200cm ⏎ POWER ROOM ⏎ SPLIT ⏎ SPLIT ⏎ ATS ⏎ MDP |
| `persian-tabriz-expansion-data-center.pdf` | full | 50/50 | 50/50 | 118454 | 0 | yes — to_unicode_logical, producer_visual_order_known | to_unicode_logical, producer_visual_order_known, encoding_mapped | Procurement Request   ⏎ Tabriz Expansion Data Center  ⏎    ⏎ Confi |
| `unknown-1.pdf` | full | 7/7 | 7/7 | 5635 | 0 | yes — to_unicode_logical | to_unicode_logical, encoding_mapped | Version 7.4.8 ⏎ PVsyst - Simulation report ⏎ Grid-Connected Syst |
| `unknown-camscanner-18-08-2025-12-45.pdf` | no_text | 1/1 | 1/1 | 0 | 0 | yes — no text to order (no text layer) | - |  |
| `unknown-honeywell-cataluge-fire-supression.pdf` | no_text | 20/20 | 20/20 | 0 | 0 | yes — no text to order (no text layer) | - |  |
| `unknown-ketab.pdf` | no_text | 9/9 | 9/9 | 0 | 0 | yes — no text to order (no text layer) | - |  |
| `unknown-resume-devops-1.pdf` | refused | 1/1 | 0/1 | 0 | 2636 | no — unsupported_visual_order | bidi_verified, unsupported_visual_order |  |
| `unknown-roof2.pdf` | full | 7/7 | 7/7 | 5659 | 0 | yes — to_unicode_logical | to_unicode_logical, encoding_mapped | Version 7.4.8 ⏎ PVsyst - Simulation report ⏎ Grid-Connected Syst |
| `unknown-solid-cable-3-50-25-afsharnezhad-khorasan-datasheet.pdf` | refused | 1/1 | 0/1 | 0 | 1842 | no — unsupported_visual_order | encoding_mapped, unsupported_visual_order |  |

## Independent order probe (logical vs character-reversed)

For a pure-RTL word, correct logical output contains the logical form and **zero** occurrences of its character-reversed form. Every word is 4+ characters and is counted twice: as a plain substring (the strict reading — zero means zero anywhere in the text) and with word boundaries added so a short word cannot match inside a longer one. This probe reads only the emitted text; it does not read our own reasons.

| file | word | raw logical | raw reversed | word-boundary logical | word-boundary reversed | verdict |
|---|---|---|---|---|---|---|
| `arabic-1.pdf` | منظمة | 92 | 0 | 52 | 0 | logical only |
| `arabic-1.pdf` | الصحة | 146 | 0 | 142 | 0 | logical only |
| `arabic-1.pdf` | إطار | 21 | 0 | 12 | 0 | logical only |
| `arabic-2.pdf` | منظمة | 530 | 0 | 28 | 0 | logical only |
| `arabic-2.pdf` | الصحة | 223 | 0 | 207 | 0 | logical only |
| `arabic-2.pdf` | إطار | 53 | 0 | 45 | 0 | logical only |
| `arabic-4.pdf` | منظمة | 59 | 0 | 42 | 0 | logical only |
| `arabic-4.pdf` | الصحة | 19 | 0 | 14 | 0 | logical only |
| `arabic-4.pdf` | إطار | 20 | 0 | 9 | 0 | logical only |
| `english-asnad-9-40.pdf` | شرکت | 1 | 0 | 1 | 0 | logical only |
| `english-asnad-9-43.pdf` | مدیریت | 1 | 0 | 1 | 0 | logical only |
| `english-sad-prpoposal.pdf` | شرکت | 14 | 0 | 14 | 0 | logical only |
| `hebrew-3.pdf` | פניות | 17 | 0 | 12 | 0 | logical only |
| `hebrew-3.pdf` | ציבור | 96 | 0 | 7 | 0 | logical only |
| `hebrew-4.pdf` | פניות | 131 | 0 | 50 | 0 | logical only |
| `hebrew-4.pdf` | ציבור | 83 | 0 | 23 | 0 | logical only |
| `persian-7.pdf` | شرکت | 21 | 0 | 17 | 0 | logical only |
| `persian-7.pdf` | قرارداد | 65 | 0 | 50 | 0 | logical only |
| `persian-7.pdf` | مدیریت | 8 | 0 | 6 | 0 | logical only |
| `persian-proposal-vb-revised-fa-office.pdf` | شرکت | 5 | 0 | 5 | 0 | logical only |
| `persian-proposal-vb-revised-fa-office.pdf` | قرارداد | 8 | 0 | 7 | 0 | logical only |
| `persian-proposal-vb-revised-fa-office.pdf` | مدیریت | 15 | 0 | 13 | 0 | logical only |

Probe words matched: 22; of those, 22 were logical-only (the passing verdict).

No file returned both the logical and the character-reversed form of a probe word.

## Why `arabic-3.pdf` is withheld — the number that must survive

This is the file the rule exists for: 321 pages that decode cleanly and whose RTL order nothing in the file establishes. Before the order gate the same extraction put **1,187,357 characters** of visual-order Arabic into `data.text` while `ok` was already `false` — a caller reading `data.text` and ignoring the reason got reversed Arabic:

| probe word | logical | reversed |
|---|---|---|
| التقرير | 0 | 79 |
| المرأة | 0 | 449 |
| النساء | 0 | 541 |

Measured on `reports/verify/postfix-arabic-3.json` (pre-fix run kept as evidence). The withheld count reconciles exactly:

* sum of all page texts: 1,187,037 characters;
* the old joined `data.text` added 320 inter-page separators → 1,187,357;
* withheld now: **1,187,033** characters on the 319 pages carrying `unsupported_visual_order`; the other 2 pages contributed 4 characters of whitespace, which is the only text still emitted.

Nothing was dropped: the text is still decoded, counted and reported. It is no longer presented as logical order.

## Reading this report

* `full` = every page decoded and ordered (`ok: true`, exit 0).
* `partial` = at least one page refused: `ok` is `false` (exit 3) and `data.text` contains ONLY the pages whose order was established. Each refused page lists its own reason inside `data.pages[].reasons` (ADR 0004).
* `refused` = no page emitted text AND a refusal is on record: `ok: false`, exit 3, `data.error` names the file and the reason; `unordered_chars` says what was decoded anyway.
* `no_text` = the file carries no text layer at all (scan/blank): nothing was refused and nothing was withheld, so it is not counted as a failure.
* `order rule` names the rule that established the file's order. `producer_visual_order_known` counts because its fingerprint family is backed by tests in the repo: `recover.rs::tests::producer_fingerprint_allow_lists_are_pinned` pins the allow-lists themselves, and `tests/no_silent_reversal.rs` checks the outcome on Word (`persian-7`, `hebrew-4`) and InDesign (`arabic-2`) when the private subset is present locally. **A fingerprint with no backing test does not count.**
* `no RTL text` means there was no right-to-left text to order.
* Reasons come from the CLI envelope verbatim; nothing here is inferred.
* A clean decode alone is *not* evidence of order (ADR 0002/0004).
