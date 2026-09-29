# 0002 — What Chrome actually writes for RTL text (and how to invert it)

* **Found:** 2026-09-27, by dumping real Chrome output (`tools/dump_streams.py --raw`)
* **Fixture:** `corpus/raw/generated/chrome/fa-plain.pdf` (source text: `سلام دنیا`)
* **Status:** understood — this is the specification the extractor is built against

## The raw content stream (Chrome / Skia, `--headless --print-to-pdf`)

```
q
3.125 0 0 3.125 212.5 212.5 cm
/NonStruct <</MCID 0 >>BDC
BT
/ReversedChars BMC                      ← (1) the whole RTL run is flagged as stored reversed
/Span<</ActualText <FEFF0627> >> BDC    ← (2) per-cluster ActualText, in VISUAL order
/F4 24 Tf
1 0 0 -1 554.28125 40 Tm
<038E> Tj                               ←   ا
EMC
/Span<</ActualText <FEFF06CC> >> BDC
<03F4> Tj                               ←   ی  (U+06CC — Persian yeh, not Arabic U+064A)
EMC
/Span<</ActualText <FEFF0646> >> BDC
<03E7> Tj                               ←   ن
EMC
<03A9000303E1> Tj                       ← (3) THREE CIDs with NO ActualText: د ␠ م
/Span<</ActualText <FEFF06440627> >> BDC
<03FC> Tj                               ←   لا  (lam-alef ligature, LOGICAL inside: ل then ا)
EMC
/Span<</ActualText <FEFF0633> >> BDC
<03B3> Tj                               ←   س
EMC
EMC                                     ← closes /ReversedChars
ET
```

## Four facts that define the extractor

1. **Storage order is visual (left-to-right), and Chrome tells us so**: the RTL run is wrapped
   in `/ReversedChars BMC … EMC` (PDF 2.0 §14.8.2.3.3). We do not have to guess, and we must
   not "reverse anything that looks RTL" — the marker is the evidence.
2. **`/ActualText` is attached per glyph *cluster*, not per run**, and each value is
   **logical order inside itself**. `لا` = U+0644 U+0627 = `ل` then `ا` — a ligature is one
   cluster whose internal order is already logical. This is why a naive "reverse the string"
   destroys ligatures.
3. **Some clusters carry no `ActualText` at all** (here: the 3-CID run `د ␠ م`). Those must be
   decoded from the CID via the font's `ToUnicode` CMap, one unit per CID. Chrome is not
   obliged to annotate everything, so an extractor that only reads `/ActualText` silently
   loses characters — and a test suite that only uses `/ActualText` fixtures will never notice.
4. **Digits and Latin inside the RTL line keep their own internal order** (see
   `fa-zwnj-lamalef`: the date `۱۴۰۳/۰۵/۱۲` appears in the ActualText list
   `۱ ۰ ۳ ۰ ۱ ۲ …`, i.e. right-to-left at *cluster* level while the number itself is not
   character-reversed). This is where bidi levels must drive the inversion, not a blunt
   reverse of the whole string.

## The inversion rule (our algorithm)

Build a list of **units** by walking the content stream inside each `/ReversedChars` run:

| stream element | becomes |
|---|---|
| `<CID> Tj` inside a `/Span<</ActualText …>>` | **one unit** = the decoded ActualText (ligatures stay whole: `لا`) |
| `<CID> Tj` with no `ActualText` | **one unit per CID**, decoded via `ToUnicode` |

Then, for a `/ReversedChars` run, **reverse the unit order** and concatenate.

Worked example (`سلام دنیا`), units in stream order:

```
ا   ی   ن   د   ␠   م   لا   س        (visual)
```
reversed:
```
س   لا   م   ␠   د   ن   ی   ا        (logical)  →  "سلام دنیا"  ✓
```

Note the middle run: it had no `ActualText`, but splitting it per CID (`د`, `␠`, `م`) is what
makes the reversal land the space and both letters correctly. Treating that run as one unit
would have produced `م ␠ د` in the wrong place — a one-character-looking bug that silently
corrupts every two-word line.

## Verified consequences

* `pdftotext` on these real Chrome files returns **only the Latin runs** — for
  `mixed-fa-en.pdf` it printed `pdfrtl v0.1 -- ISO 32000-1 §9.7.4.3 -- 42%` and dropped every
  Persian character. Combined with problem 0001, poppler is now permanently demoted to an
  LTR/structure oracle.
* The `Reason` codes map cleanly onto this: `/ActualText` present → `Reason::ActualText`;
  `/ReversedChars` without full annotation → `Reason::ProducerVisualOrderKnown` plus
  `ToUnicode` decoding; anything we cannot decode → `UnsupportedBrokenToUnicode`, exit 3.

## Reproduce

```bash
python tools/make_producer_fixtures.py --sets fa-plain
python tools/dump_streams.py corpus/raw/generated/chrome/fa-plain.pdf --raw
python tools/dump_streams.py corpus/raw/generated/chrome/fa-plain.pdf --actual-text
pdftotext corpus/raw/generated/chrome/mixed-fa-en.pdf -      # Latin only — Persian dropped
```

## Follow-ups

* Windows/Word fixtures next: if Word stores **logical** order, we have both families in the
  corpus and the producer allow-list becomes evidence-based rather than assumed.
* The extractor must never be "fixed" by reversing a string: reversal happens at **unit**
  level, driven by `/ReversedChars`, and is asserted by codepoint comparison (visual review
  cannot see the difference — see `corpus/generated/SOURCES.md`).
