# CANARY — pdfrtl / bdf

**A single static Linux binary that extracts text from Arabic, Persian and Hebrew PDFs, and
tells you when it cannot be trusted.**

This is the first version meant to be run by someone other than its author. It is deliberately
small in scope and deliberately loud about its limits.

---

## 1. What you get

```
pdfrtl extract --json <file.pdf>
```

One static Linux x86-64 binary, no runtime dependencies, no configuration. Output is a single
JSON object on stdout. Exit codes are part of the contract:

| code | meaning |
|---|---|
| 0 | Extracted. Every page's text is in logical order. |
| 2 | Bad usage — wrong verb or missing argument. |
| 3 | **Refused.** Some text could not be proven correct and was withheld. See `reasons`. |
| 4 | I/O or parse failure. |

**There is no third outcome.** Every page is either returned in logical (reading) order, or it is
withheld and the reason is named. The tool never emits text it cannot vouch for, and it never
guesses.

### Withheld text is reported, not thrown away

Text that was decoded but could not be ordered is **not discarded**. It appears in a separate
field, `data.pages[].unproven`, as `{line, text, reason}` — and `data.text` stays **proven-only**.

Text whose order could not be established is kept separate from the proven `data.text` field. The text is available only through `--include-unproven`; by default the envelope contains counts without the withheld text.

## 2. The one property that matters

> Extraction returns **logical order**, or it fails with an explicit reason.
> A silent reversal is a bug. There is no third outcome.

Right-to-left storage in real PDFs is **producer-dependent**: some tools write text in the order
you read it, others write it in the order they painted it. A blanket "reverse RTL runs" therefore
corrupts the files that were already correct — and the corruption is invisible to anyone who
cannot read the script. So the tool asks the file, in this order:

1. `/ActualText` marked content — the producer's own answer. Trusted.
2. `/ReversedChars` — the producer's declaration that the run is stored visually.
3. **The painting itself** — each unit's position is measured from the declared glyph widths,
   and the stored order is tested against it under UAX #9.
4. A producer fingerprint, for families verified individually.
5. **Refuse**, naming the reason.

## 3. What it can and cannot do — public evidence

Private-corpus performance measurements and language-label audit results remain owner-local and are not published. The public clone contains redistributable synthetic fixtures; its CI and `scripts/verify-clone.sh` are the reproducible evidence for the published build.

The owner reviewed recently displayed Persian RFP extraction text and said it seemed right. This is a useful review of those excerpts, not a rendered-page comparison or a construction-verified public fixture. Do not treat the private label corrections as proof that the general RTL-order problem is solved.

### Known limitations, stated plainly

* **Reading order is not proven on every RTL input.** Ambiguous text is withheld with a reason; no release claim should imply complete recovery.
* **Scanned documents without a text layer are out of scope.** OCR is deliberately not implemented.
* **Fonts with no usable character mapping are refused per font.** Text from a font that cannot be mapped is withheld and `unsupported_broken_to_unicode` is reported; other text on the page may still be returned.
* **Generation, editing, signing, forms, MCP and OCR are not implemented.** This binary only reads.
* **No page reordering for multi-column layouts.** A `Reason` is required before that is attempted; none is guessed.
### What "correct" means here, and who checked it

No automated test can confirm that Persian text *reads* correctly — only a person who reads the
script can. Every correctness claim in this document rests on:

* synthetic fixtures whose expected output is checked against **construction**, and
* a **human read of rendered pages** for the real archive.

No third-party extractor was used as an oracle, because there isn't one: measured, poppler flips
every lam-alef pair (`سلام` → `سالم`), Xpdf returns visual order, and PDFium drops ZWNJ. Details
in `docs/problems/0004`.

## 4. Licence

**No licence is granted.** This is shared by invitation only. The repository carries
`License: TBD — all rights reserved`. The planned long-term posture is AGPL-3.0-or-later plus a
commercial dual licence (`docs/decisions/0001`), but that is **not** in force, and the canary
binary carries no licence at all.

*Historical note, stated rather than hidden:* the public repository contained AGPL licence files
in its earliest commits. They were removed on 2026-10-03 (commit `4012e68`). Versions already
published under that file were offered under AGPL-3.0; removing it does not withdraw that grant.

## 5. How to verify what you were given

```bash
# Does it run with nothing installed?
ldd ./pdfrtl            # -> "statically linked"

# Does it behave, not just run?
pdfrtl extract --json some.pdf
echo $?                 # 0 = extracted, 3 = refused, and the reasons say why

# What was withheld, if anything (may be reversed — read the reason)
pdfrtl extract --json some.pdf | jq '.data.unproven_lines, .data.pages[0].unproven'

# Does it match the source build exactly?
pdfrtl --version
```

### Contract changes a caller should know about

* **`bidi_verified` is now `bidi_consistent`** (2026-10-03). The old name claimed the painting
  *proves* the original order; it does not. Two different logical strings can paint identically —
  measured, `docs/problems/0015` — so a forward match is consistency, not proof. This is a
  breaking change to the serialized reason code.
* **`unproven` is opt-in** (`--include-unproven`, 2026-10-03). The default envelope reports the
  **counts** only; the withheld text is behind a flag. That text is frequently reversed, and an
  LLM or MCP caller reads the whole JSON object without asking for a field by name. `data.text`
  is unchanged and still proven-only.

The canary binary's **output** was verified identical to the glibc build across all 14 public
fixtures. A *reproducible build* — rebuilding the committed binary byte-for-byte from source —
is **not** verified and no artefact for it is committed; treat that as an open claim, not a fact.

## 6. What the author would like tested

1. **A refusal you believe is wrong.** Report the file and the reason code. Refusals are the
   interesting output; a wrong refusal is a real defect.
2. **Text you believe is reversed.** This is the defect class the project exists to prevent, and
   a single confirmed instance matters more than any amount of aggregate counting.
3. **A file it handles that is not in the corpus.** Real-world producers are more varied than
   anything measured here.

## 7. Where the code and its history live

`https://github.com/amirrezaalavi/bdf` — including `docs/problems/`, a log of every failure found
and fixed, and `AGENTS.md`, the rules that keep the project's one invariant true.