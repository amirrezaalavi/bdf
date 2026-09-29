# Answers to research questions — round 1 (consumed 2026-09-29)

**Provenance.** Written by an external research agent answering `docs/RESEARCH-QUESTIONS.md`
Q-R1…Q-R10 as they stood on 2026-09-29. The full answers follow verbatim below this digest.
The questions were consumed and deleted from the queue, as the queue's own rules require.

**How to read this.** The digest is *our* judgement, not the agent's: what we accept because we
corroborated it, what we accept on its sources, what we reject, and what it changes. Where the
answer and our own measurements disagree, our measurement wins and the disagreement is recorded.

## Accepted, corroborated by our own measurement

- **Q-R1 / Q-R9 — the information is in the file.** poppler's `-layout` returns coherent
  logical Persian for our Microsoft-family files while `-raw` returns the same words reversed.
  We measured exactly that before the answer arrived, on two refused files. The answer names the
  mechanism (`TextOutputDev::reorderText`, a maximal-RTL-run reversal when `rawOrder=false`).
  The property that matters: our 14 refusals are a **measurement** problem, not an evidence
  problem — the geometry is in the file, and we now model `/W`.
- **Q-R2 — invert with levels, not characters.** The inverse of UAX #9 rule L2 is the L2
  reversal applied to the visual stream (it is involutive), with four traps to test: digit runs
  are even-level and must not be reversed, BD16 bracket mirroring, lam-alef clusters stay whole,
  NSM marks inherit the level of the character before them. The standard defines logical→visual
  only, so the inverse is implementation-defined and our own tests are the only evidence that
  can justify ours.
- **Q-R4 — font-aware recovery has a procedure.** Invert the embedded font's `cmap`, fall back
  to `post` glyph names (`uni0645` → U+0645), prefer base letters via NFKC, and **fail loudly**
  when neither the cmap nor the names exist. `ttf-parser` is the permissive Rust implementation.
  This unblocks W1.6.
- **Q-R5 — base letters in output, never presentation forms.** `/ToUnicode` should yield base
  letters; forms are presentation. Search normalization folds with NFKC. Adds
  `presentation_forms_folded` to the reason vocabulary.
- **Q-R7 — the writer stack.** `harfrust` (MIT, maintained by the HarfBuzz organisation) over
  `rustybuzz` (semi-abandoned), with `krilla` + `pdf-writer` for Type0/`Identity-H`,
  `/ToUnicode`, `/CIDToGIDMap` and `/ActualText` spans. Recorded for W4; the known risk (a young
  writer above a fast-moving low-level one) is unchanged by the answer.
- **Q-R8 — citations for the invariant.** PDF/UA-1 §7.2 (logical reading order), §8.2.3
  (logical content order), ISO 32000 §14.9.4 (`/ActualText`), WCAG PDF3. A Persian document
  whose logical order cannot be established fails PDF/UA-1, so our invariant is a precondition
  for conformance rather than a nicety — citable for the enterprise tier.
- **Q-R10 — an independent measurement exists.** PDFium's `FPDFText_GetCharBox` /
  `FPDFText_GetCharOrigin` expose per-character user-space boxes from the stable public API, and
  PDFium is BSD-3, so it is a legitimate oracle; MuPDF (AGPL) and poppler (GPL) stay CI-only,
  exactly like the existing pixel oracles.

## Rejected

- **Lexical scoring (word-list agreement) as a shipped rung.** The answer's own conclusion: no
  prior art, no documented margin, severe failure modes (transliteration, mixed scripts, proper
  names, numbers). Our invariant permits a *named* reason with documented failure modes; this
  has no defensible margin to document, so it stays out of the product. Recorded here so it is
  not proposed again without new evidence.
- **"Refuse the Microsoft family permanently" (Q-R9's impact line).** Not the answer while a
  documented mechanism proves the information is present. We refuse *until the geometry
  decides*, and say which reason; the moment it decides, we invert and label `bidi_reordered`.
  What we keep from the answer: this family stores visual order and emits neither `/ActualText`
  nor `/ReversedChars`, so more than half of the ladder cannot apply to it.
- **Claims we could not corroborate and will not act on:** a "PDFium build 8044" bidi regression
  (no usable version reference), a report that Excel "reverses RTL text order in some versions"
  (no source), and every producer-table row marked "None known" or "UNVERIFIED". A table is a
  set of hypotheses for tests, never allow-list entries.
- **The invented tie-break in Q-R4** ("prefer the GID whose Unicode value has the lowest
  NFKC-decomposed code point"): not in any spec. Our ambiguity policy must either be documented
  as ours or become a refusal; it cannot be presented as a standard rule.

## What this changes

1. Order recovery stays **measured**: rung 3 decides from geometry; a producer name only
   disambiguates. See ADR `0006`.
2. The next two steps: classify the 14 with the independent oracle (PDFium boxes), then
   instrument rung 3 to see which branch refuses and why — the answer key, then the diagnosis.
3. W1.6 is unblocked with a procedure; W4's stack is chosen; the accessibility claims have
   clauses behind them.
4. The allow-list stays **empty** on purpose: an entry needs a test we ran ourselves.

---

# Answers to research questions

---

## Q-R1 — Deciding "visual or logical" for RTL runs whose producer is unknown or generic

**Answer:**

**1. Does any implementation rely on GID/CID ordering?**
No implementation we found uses GID or CID ordering as a signal for logical vs. visual order. PDFium's bidi reordering derives direction from character **Unicode classes** (via UAX #9), not from glyph IDs. Poppler's `reorderText()` operates on a `Unicode*` array after character decoding, also ignoring GID order. **UNVERIFIED** whether any implementation uses CID ordering; we found no evidence.

**2. Do advance widths or placement operators reveal reversed ordering?**
`FPDFText_GetCharBox` and `FPDFText_GetCharOrigin` expose per-character bounding boxes in PDF user space, which can reveal whether painted glyphs run right-to-left or left-to-right on the page. However, neither poppler, PDFium, nor pdf.js currently uses bbox geometry as a **reordering input** — they use it only for output positioning. MuPDF's `stext` device similarly exposes bbox per span but does not reorder based on geometry. **No implementation uses `/W` array shapes for order detection.**

**3. Does the font cmap carry the signal?**
No. When `/ToUnicode` is absent, GID→Unicode must be resolved from the embedded font's `cmap` table, but the cmap maps Unicode→GID, not the reverse. fontTools' `getBestCmap()` returns a `{codepoint: glyph_name}` dictionary; reversing it to GID→Unicode is possible but ambiguous when multiple Unicodes map to one GID (common with presentation forms). **No implementation we found uses cmap ordering as a logical-vs-visual signal.**

**4. What do poppler, pdf.js, PDFium, MuPDF actually do?**
- **poppler**: `pdftotext -layout` uses `TextOutputDev` with `rawOrder=false`; `-raw` uses `rawOrder=true`. The `reorderText()` function (in `TextOutputDev.cc`) implements a **non-ICU fallback** that scans for RTL runs and reverses them, using `unicodeTypeL()`/`unicodeTypeR()` to detect LTR/RTL characters. A patch exists to use ICU's bidi algorithm instead, with fribidi fallback. The reordering is **visual → logical**.
- **PDFium**: `CPDF_TextPage` applies UAX #9 bidi reordering via `fx_bidi` (derived from `cpdf_textpage.cpp`) to convert visual order to logical reading order. A regression in build 8044 reversed ordinary RTL word order without `/ActualText`; the fix restores whole-line segment reversal for predominantly RTL lines whose first and last strong segments are RTL.
- **pdf.js**: `src/core/bidi.js` implements a subset of UAX #9. It does **not** reorder automatically in `getTextContent`; the output follows text-rendering operator order. Applications must call the bidi function explicitly.
- **MuPDF**: `fz_bidi` in `bidi-std.c` implements UAX #9 rule L2 (reverse runs at each level). The `stext` device detects visual/logical RTL layouts but returns text in content-stream order; bidi must be applied by the consumer.

**5. What does Acrobat do?**
User reports indicate Acrobat Pro copy-paste returns **logical order** for RTL PDFs that display correctly. This is consistent with Acrobat applying bidi reordering on extraction. **UNVERIFIED** as an official documented behavior.

**6. What rule does poppler use for the Microsoft-family files?**
The `reorderText()` function scans the decoded Unicode string and reverses RTL runs. For `-layout` mode (`primaryLR` derived from page direction), it outputs LTR sections forward and RTL sections backward. For the Microsoft files, `-layout` returns logical Persian because the function reverses the visual-order RTL runs back to logical order. `-raw` returns reversed because it skips this reordering. The function uses `unicodeTypeR()` to detect RTL characters and reverses maximal RTL runs, handling lam-alef clusters as single units and leaving digits in LTR runs.

**Impact:** We would implement a detection rule based on **UAX #9 character-class analysis** of the decoded `/ToUnicode` sequence combined with **bbox geometry from `/W` and Td/TJ offsets** to classify stored order. Poppler's `reorderText()` provides a reference algorithm for visual→logical reversal that we can encode with its failure modes (mixed-direction lines, neutral characters at boundaries). We would keep refusing files where the character-class signal is ambiguous (e.g., all-neutral runs) — potentially recovering 5–7 of the 14 files while honestly withholding the rest.

---

## Q-R2 — Inverting a visual-order run to logical order (UAX #9 in reverse)

**Answer:**

**Algorithm:** UAX #9 rule L2 defines the visual reordering: from the highest level down to the lowest odd level, reverse every maximal contiguous run of characters at that level or above. To invert visual→logical, the same L2 reversal is applied to the visual stream: the operation is **involutive** — reversing the same level runs in the same order recovers the logical order. The embedding levels must be computed from the **logical** text; in a visual-order stream, levels must be **derived from the visual order** by detecting RTL runs and assigning them odd levels.

**Digits:** A run of digits forms an even (LTR) embedding level inside RTL text and is therefore **not mirrored** when the surrounding RTL is reversed. The `reverse_rtl_keep_numbers` function demonstrates this: separator characters (`.` `,` `:` and Arabic decimal/thousands separators) are treated as part of the number only when between two digits, so `1,000` and `3.14` stay intact while a trailing comma reverses as an ordinary neutral.

**Brackets and BD16 mirroring:** UAX #9 rule L4 (BD16) requires mirroring paired brackets when the resolved direction is RTL. Implementations must apply mirroring during the visual→logical pass: PDFium's `fx_bidi.rs` explicitly calls `mirror_char`. The trap is **double-mirroring** — if the PDF already stores mirrored glyphs, reversing without checking produces unmirrored brackets.

**Ligature clusters (lam-alef):** A lam-alef ligature is a single glyph representing two characters. On reversal, the cluster must **stay whole** — reversing it would produce alef-lam. Implementations handle this by treating the cluster as a single unit with its own directionality. Poppler's `reorderText()` reverses maximal RTL runs at the character level, which preserves cluster integrity if clusters are stored as single code points.

**Combining marks:** Marks must remain attached to their base character. In visual order, a mark follows its base in the painting stream; in logical order, it may precede or follow depending on the script. UAX #9 treats combining marks (class NSM) as transparent to direction, so they inherit the level of the preceding character.

**Authoritative implementation:** ICU `ubidi` is the reference implementation of UAX #9 and is used by poppler (as an optional patch) and Chromium (for paragraph layout). `pdf.js` bidi is a lightweight subset suitable for simple cases. MuPDF's `bidi-std.c` is derived from the Unicode reference implementation.

**Standards scope:** ISO 32000 does not normatively define logical order recovery from visual streams. UAX #9 defines logical→visual rendering, not the inverse. **The inverse is entirely implementation-defined.** The one normative hook is `/ActualText` (ISO 32000-1 §14.9.4 / 32000-2 §14.9.4), which carries the logical text explicitly.

**Impact:** We would adopt ICU `ubidi` (or a faithful Rust port) as the authoritative reversal engine, with explicit handling for digit runs, bracket mirroring, and cluster integrity. We would implement the inverse as a **level-based L2 reversal on the visual stream**, not a naive character reversal, and add a regression test for each trap (digits, brackets, lam-alef, combining marks).

---

## Q-R3 — Producer behaviour table, with versions and the reason behind it

**Answer:**

| Producer | Stores RTL as | Marker emitted | Reason / internal library |
|---|---|---|---|
| **Chrome/Skia PDF print** | Visual (per-cluster) | `/ActualText` + `/ReversedChars` | Skia PDF backend emits `/ActualText` per cluster; `/ReversedChars` for glyph runs |
| **LibreOffice** | Logical (via HarfBuzz shaping) | None known | Uses Cairo/HarfBuzz for shaping; PDF output from Cairo may vary |
| **Microsoft Word 2013–365** | Visual | None known | Office PDF exporter uses its own layout engine; no `/ActualText` in our observations |
| **Microsoft Print To PDF (XPS→PDF)** | Visual | None known | Windows XPS→PDF path; no `/ActualText` |
| **Microsoft Excel LTSC/2016/2013** | Visual | None known | Office export path; no `/ActualText` |
| **Adobe InDesign 17–20** | Logical | `/ActualText` on `/Span` | InDesign emits `/ActualText` for drop caps and RTL spans |
| **Adobe Acrobat Pro 11** | Visual or logical | Varies | Acrobat's PDF Maker uses its own engine |
| **Adobe Distiller / “Adobe PDF Library”** | Visual | None known | Distiller uses PostScript→PDF conversion |
| **macOS Quartz/Preview** | Visual | None known | Quartz PDF context |
| **Nitro PrimoPDF** | Unknown | None known | UNVERIFIED |
| **ReportLab** | Logical | None known | ReportLab writes logical order |
| **wkhtmltopdf** | Visual | None known | Qt WebKit rendering |
| **PDFreactor** | Unknown | Unknown | UNVERIFIED |
| **mPDF** | Visual | None known | PHP library; may reverse RTL |
| **iTextSharp** | Logical (configurable) | None by default | iText can emit `/ActualText` if configured |
| **LaTeX (pdfTeX/LuaTeX/XeTeX)** | Logical (with bidi packages) | None by default | XeTeX with `arabxetex` shapes via HarfBuzz |
| **Ghostscript (ps2pdf)** | Visual | None | PostScript→PDF; PS stores visual order |

**Impact:** We would ship an allow-list with **Chrome/Skia** (visual, `/ActualText` present → recover via `/ActualText`) and **Adobe InDesign** (logical, `/ActualText` present → recover via `/ActualText`). For **Microsoft Print To PDF** and **Office exporters** (visual, no marker), we would keep refusing until Q-R9 is answered. For **LibreOffice**, we would test to confirm logical storage and allow-list if confirmed.

---

## Q-R4 — Font-aware recovery when `ToUnicode` is absent

**Answer:**

**Procedure:**
1. Parse the embedded TrueType/OpenType font using `ttf-parser` or `fontTools`.
2. Read the `cmap` table and call `getBestCmap()` — this returns the best Unicode subtable in preference order: (3,10), (0,6), (0,4), (3,1), (0,3), (0,2), (0,1), (0,0).
3. Build the **reverse mapping** GID→Unicode by inverting the `{codepoint: glyph_name}` dictionary. When multiple Unicodes map to one GID (presentation forms), choose the **base letter** by applying NFKC decomposition to the Unicode values.
4. If the subset's `cmap` is stripped/empty, fall back to `post` table glyph names (e.g., `uni0645` → U+0645). If `post` is format 3 (no names), **fail loudly** with `unsupported_font_no_unicode`.
5. Resolve ambiguity when many GIDs map to one or several presentation forms: prefer the GID whose Unicode value has the **lowest NFKC-decomposed code point** (base form).

**Tooling:**
- **fontTools** (Python): `TTFont.getBestCmap()` returns the preferred Unicode cmap. `TTFont.getGlyphOrder()` gives GID→glyph name.
- **ttf-parser** (Rust): `Face::tables().cmap` provides subtable iteration; `glyph_index(char)` maps Unicode→GID. Reverse mapping requires manual inversion.
- **harfbuzz**: `hb_face_collect_unicodes()` collects all Unicode values in the font, but does not provide GID→Unicode. HarfBuzz's cluster field maps glyphs to input string positions, not to Unicode values.

**Spec clauses:** OpenType `cmap` spec defines formats 4 (BMP) and 12 (full Unicode). Format 4 uses segmented mapping and works best if glyph order is sorted by Unicode. Format 12 is required for supplementary-plane characters. ISO 32000 §9.6–9.10 covers font descriptors and CIDFonts.

**Impact:** We would implement a font-aware GID→Unicode resolver using `ttf-parser` with the preference order above, falling back to `post` names, and **failing explicitly** when neither is available. This could recover `hebrew-1.pdf` and similar files, adding a rung to the ladder (`font_cmap_derived`) with documented failure modes.

---

## Q-R5 — Presentation forms (U+FE70–FEFF) in extraction output

**Answer:**

**Acrobat:** Acrobat copy-paste returns **logical order** with base letters when the PDF contains proper `/ActualText` or when Acrobat's bidi engine reconstructs logical order. When `/ToUnicode` maps to presentation forms directly, Acrobat may keep the forms or fold them depending on the context. **UNVERIFIED** for the no-`/ActualText` case.

**PDFium:** `fx_bidi` applies UAX #9 reordering and maps presentation forms back to base letters via NFKC normalization in the text extraction path. The `mirror_char` function handles bracket mirroring.

**poppler:** `reorderText()` reverses visual runs but does **not** fold presentation forms; it outputs whatever Unicode the `/ToUnicode` map provides. If `/ToUnicode` maps to U+FE70–FEFF, poppler outputs those forms.

**pdf.js:** The bidi function operates on character classes; presentation forms have class AL (Arabic Letter) and are treated as RTL characters. pdf.js does not fold them to base letters by default.

**Convention:** PDF/UA-1 recommends that `/ActualText` carry the **original logical text** before shaping, and that presentation forms in `/ToUnicode` be avoided in favour of base letters. Accessibility tooling expects base letters for search and screen readers.

**Offset/cursor semantics:** Folding to base letters changes string length (e.g., lam-alef ligature U+FEFB → two characters U+0644 U+0627), breaking offset-based search. The convention is to **store base letters** in extraction output and use presentation forms only for rendering.

**Impact:** Our reason vocabulary would include `presentation_forms_folded` and `presentation_forms_preserved`. We would fold to base letters in extraction output (for search compatibility) and keep presentation forms only in a separate rendering layer. Search normalization would apply NFKC.

---

## Q-R7 — Writer stack for RTL shaping in Rust (state of the art, 2026)

**Answer:**

**harfrust vs. rustybuzz:**
- **harfrust** (v0.13.3, MIT licence) is a Rust port of HarfBuzz, forked from rustybuzz to explore porting from `ttf-parser` to `read-fonts`/`fontations`. It is actively maintained by the HarfBuzz organisation (305 stars, 22 forks). It includes shaping, subsetting, font parsing, and Unicode tables.
- **rustybuzz** (v0.20.1) is a complete HarfBuzz shaping algorithm port to Rust, passing 98% of HarfBuzz tests but **slower** (1.5–2× slower) and in a **semi-abandoned state** — "effectively dead and will be archived eventually".

**Cluster API:** Both expose a `cluster` field mapping each glyph back to its source byte offset in the input string. Cluster levels 0 (monotone graphemes), 1 (monotone characters), and 2 (characters) control merging. For RTL, clusters are guaranteed monotonic (never decrease) in the output.

**RTL run output order:** When `Direction::RightToLeft` is set, the buffer outputs glyphs in **visual order** (rightmost first). The cluster field still points to logical input positions.

**krilla / pdf-writer readiness:**
- **krilla** (v0.6.0/0.7.0) is a high-level Rust PDF creation library built on `pdf-writer`. It supports **excellent OpenType font support** including color fonts and **great subsetting** for CFF and TTF fonts. Typst has switched its PDF backend to krilla.
- **pdf-writer** (Typst's low-level PDF writer) provides access to Type0/`Identity-H` fonts, `/ToUnicode`, `/CIDToGIDMap`, and `/ActualText` spans. krilla abstracts these.

**Recommended combination:** `harfrust` for shaping + `krilla` for PDF generation. `harfrust` is actively maintained, MIT-licensed, and has the full HarfBuzz feature set. `krilla` is the most mature high-level Rust PDF writer for Type0 fonts. **Known issue:** `rustybuzz` is semi-abandoned; do not depend on it for new projects.

**Impact:** We would use `harfrust` + `krilla`, with `pdf-writer` as the low-level escape hatch for `/ActualText` spans and `/CIDToGIDMap` control. Both are permissively licensed (MIT/Apache-2.0).

---

## Q-R8 — Standards: what is actually required about logical order and `/ActualText`

**Answer:**

**PDF/UA-1 (ISO 14289-1):**
- **Clause 7.2**: "Content shall be marked in the structure tree with semantically appropriate tags in a **logical reading order**".
- **Clause 7.7**: Formula structure elements require an `Alt` attribute; `ActualText` is recommended for small pieces such as ligatures.
- **Clause 8.2**: Logical structure — 8.2.2 "Real content" requires tagging; 8.2.3 "Logical content order" requires the tag tree to reflect the document's logical reading order.

**PDF 2.0 (ISO 32000-2):**
- **§14.9.4** (ActualText): "The `ActualText` value shall be used as a replacement, not a description, for the content, providing text that is equivalent to what a person would see". The value is a character substitution for the content enclosed by the structure element.
- **§14.8** (Marked content): Defines marked-content sequences and structure elements.

**WCAG (PDF techniques):**
- **PDF3**: "Ensuring correct tab and reading order in PDF documents" — the intent is that users can navigate in a logical order consistent with the meaning of the content.
- **G4_02**: Order of marked content sequences within a tag must match the author's intent; for RTL documents, lines are ordered from right to left on the page.

**Accessibility checkers:**
- Adobe's checker flags "Logical Reading Order" as **Needs manual check** — it cannot be automated.
- PAC (PDF Accessibility Checker) checks tag structure and `ActualText` presence but does not verify RTL logical order.

**Is our invariant a conformance obligation?** PDF/UA-1 **normatively requires** logical reading order in the tag tree and semantically appropriate tags. A Persian document with visual-order text and no `/ActualText` **fails PDF/UA-1** because the content's logical order cannot be established from the tag tree. Our invariant (extraction returns logical order or fails explicitly) is therefore **aligned with conformance** — it is a precondition for meeting the standard's requirement.

**Impact:** We can cite PDF/UA-1 §7.2 and WCAG PDF3 as normative basis for our invariant. The enterprise tier's requirement list would include: tag tree with logical reading order, `/ActualText` for shaped/ligated content, and base-letter `/ToUnicode` for search.

---

## Q-R9 — The Microsoft print/export family: 7 of our 14 refusals *(top priority)*

**Answer:**

**1. Storage order and internal library:**
- **Microsoft Print To PDF (XPS→PDF path)**: Produces PDF v1.7 via the Windows XPS→PDF conversion pipeline. The printer driver version varies by Windows build (10.0.19041.1, 10.0.22621.1, 10.0.26100.1882). The XPS→PDF converter stores RTL text in **visual order** — glyphs are positioned right-to-left on the page, and the content stream paints them in that order.
- **Office PDF exporter (Excel 2013/2016/LTSC)**: Uses Office's own PDF export engine, **separate from** the XPS→PDF path. Reports indicate that Excel PDF export reverses RTL text order in some versions. The exporter does **not** consider RTL text direction in column ordering.

**2. Version dependence:** Driver version changes with Windows build; Office version changes the export engine. **UNVERIFIED** whether the storage order changes between versions — no documented statement found.

**3. Machine-checkable marker:**
- **Producer string**: `Microsoft: Print To PDF` vs `Microsoft® Excel® LTSC / 2016 / 2013` distinguishes the two paths.
- **XMP metadata**: The XPS→PDF path may include XMP metadata identifying the converter.
- **Font subsetting pattern**: Both use `/Type0` + `/Identity-H` with a `/W` array and `/ToUnicode` mapping to base Arabic letters. **No distinguishing marker found** beyond the producer string.
- **Code ordering**: Visual order means the first glyph in the content stream is the rightmost painted glyph.

**4. Does this family emit `/ActualText` or `/ReversedChars`?** We have **never seen either** from this family. No documentation found stating it is never emitted. The absence is consistent with the visual-order storage model: the producer has no need to emit logical-order metadata because it assumes the consumer will reconstruct order from geometry.

**Impact:** We would **keep refusing** the Microsoft Print To PDF and Office exporter families permanently, with a written reason: `unsupported_microsoft_visual_no_marker`. We would document that no in-file marker reliably distinguishes visual from logical for this family, and that the absence of `/ActualText`/`/ReversedChars` is consistent but not documented. If a future Windows/Office version adds `/ActualText`, we would re-evaluate.

---

## Q-R10 — An independent measurement, and lexical scoring as an admissible reason

**Answer:**

**1. Independent painting measurement:**
- **PDFium**: `FPDFText_GetCharBox` returns the bounding box (left, right, bottom, top) of a character in PDF user space. `FPDFText_GetCharOrigin` returns the origin point. Both are stable C API functions. PDFium is **BSD-licensed** (permissive) and can be shipped.
- **MuPDF**: The `stext` device exposes per-span bounding boxes and per-character origins in JSON output. MuPDF is **AGPL-licensed** — may be used as a CI oracle but not shipped.
- **poppler**: `TextOutputDev` exposes word and character bounding boxes via `TextWord` and `TextPage` APIs. Poppler is **GPL-licensed** — not permissive for shipping.

**Correctness about RTL positions:** PDFium's char boxes are derived from the text object's positioning operators and font metrics, not from heuristics. They reflect the painted position of each glyph. For RTL text stored in visual order, the first character in the extraction stream has the **rightmost** bbox. This is a reliable signal for order detection. **UNVERIFIED** whether PDFium's bbox is correct for all RTL cases (e.g., with kerning or ligatures).

**API stability:** `FPDFText_GetCharBox` is part of the stable public API and has been available since PDFium's early versions. It is safe to depend on for classification.

**2. Lexical scoring as an admissible reason:**
- **Prior art**: No published research or tooling was found that uses word-list scoring to decide logical vs. visual storage in PDF extraction. The approach is used informally by human readers but has no documented algorithmic precedent. **UNVERIFIED** whether any production tool uses this.
- **Word lists**: SUBTLEX-AR (freely accessible Arabic word frequency database), Buckwalter & Parkinson's frequency dictionary (5,000 headwords), and the Arabic Internet Corpus at Leeds are available. Licensing varies — some are academic-use only.
- **Margin defensibility**: No documented margin threshold exists. The failure modes are severe: transliteration, mixed scripts, proper names, and numbers produce false positives.
- **Honest answer**: There is **no prior art and no defensible margin** for lexical scoring as a product feature.

**Impact:** We would ship PDFium (BSD) as the independent painting measurement engine, using `FPDFText_GetCharBox` to classify stored order by comparing the first character's bbox x-coordinate to the last character's. We would **not** ship lexical scoring as a named reason; it would remain an offline research instrument only, consistent with our invariant that silent heuristics are not permitted.