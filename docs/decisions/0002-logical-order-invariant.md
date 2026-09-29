# ADR 0002 — Text order invariant: logical order or an explicit `Reason`

* Status: accepted
* Date: 2026-09-27
* Deciders: Yolka

## Context

RTL text in PDFs is stored in **producer-dependent** order. Field evidence: LibreOffice,
mPDF, iTextSharp and Chrome store **visual** order (glyphs already reversed, one glyph per
`Tj`, `ToUnicode` often unmapped for presentation forms); ReportLab, wkhtmltopdf and
PDFreactor store **logical** order. Some producers also emit `/ActualText` marked content
that carries the exact logical string, and some `/ReversedChars`.

A naive implementation "fixes" RTL by reversing runs — which repairs the visual-order
family and **corrupts** the logical-order family. Every competing tool we surveyed has
this bug in some form; it is our differentiator and our biggest correctness risk.

## Decision

> Extraction returns **logical order**, or it fails with an explicit `Reason`.
> There is no third outcome. Blanket reversal is forbidden.

Precedence of recovery paths, in order:

1. `/ActualText` marked content — authoritative, verbatim logical string.
2. `ToUnicode` + UAX #9 levels (`bidi_reordered`) when the mapping is clean.
3. `ToUnicode` with runs already logical (`to_unicode_logical`).
4. Producer allow-list, populated only from inspected evidence
   (`producer_visual_order_known`).
5. Otherwise `unsupported_no_evidence` → exit code 3. We refuse.

## Consequences

* The CLI/MCP contract carries reasons, so callers can see *why* and can choose to accept
  or reject. Machine-checkable honesty beats a plausible guess.
* `pdfrtl-core`'s `Reason` enum is an API: adding a variant requires a fixture that
  triggers it.
* Some inputs will be "unsupported" that a naive tool would silently mangle. That is a
  feature: it is the difference between a tool you can trust and one you cannot.
* The corpus must contain fixtures for **both** families, plus at least one file whose
  order genuinely cannot be recovered, to prove we refuse instead of guessing.
