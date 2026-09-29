# ADR 0004 — Partial extraction with explicit reasons, and order claims that carry evidence

* Status: accepted
* Date: 2026-09-27
* Deciders: Yolka

## Context

The real archive (`corpus/raw/private/desktop-pdfs`, 51 files) is mixed and messy: one
document contains decodable pages next to undecodable ones, simple 8-bit fonts next to
broken `/ToUnicode` CMaps, and — worst of all — visual-order RTL runs next to
logical-order ones.

Two habits from the earlier implementation survived that contact badly.

**All-or-nothing refusals.** One undecodable code anywhere discarded the entire
document. A 321-page file with 450,691 text operators reported `ok: false` and *zero*
characters, so nothing was observable: no per-page reason, no way to tell "the file is
unreadable" from "one font in it is unusual". The refusal destroyed its own diagnostic
value.

**Order claims without evidence.** `to_unicode_logical` said "this page is in reading
order" because the `/ToUnicode` CMaps decoded cleanly. A `/ToUnicode` CMap says which
code means which glyph; it says nothing about the order the producer wrote the codes
in. Measured against `pdftotext`, `persian-7` returned `ok: true`, exit 0 and 29,046
characters in which `شرکت` appeared 21 times *reversed* and 0 times in logical order —
visual text reported as success. That is exactly the third outcome ADR 0002 forbids,
and it is worse than a refusal, because a caller can detect a refusal and cannot detect
a silent reversal.

## Decision

> Partial extraction is allowed and must always be labelled. Silent dropping is
> forbidden. Text whose order is not established is never reported as success.

### Page granularity

Each page independently reports either its text or its reason. A page that cannot be
decoded does not destroy the other pages, and text that *was* recovered is never
discarded. Every failed page carries its own `Reason`; there are no silent drops.

### Order is decided by named evidence only (ADR 0002's ladder, as implemented)

Evidence is collected **per visual line**, and the producer fingerprint is the LAST
rung — it is consulted only for lines the earlier rungs did not settle:

1. `/ActualText` — the producer's verbatim logical string (`actual_text`).
2. `/ReversedChars` marked lines — the producer says the run was mirrored, so the
   UNIT order is inverted and checked against UAX #9 (`bidi_reordered` when a run
   order had to be reconstructed, `producer_visual_order_known` for the marker).
3. **One unit per line.** A line that is a single marked sequence, or a single glyph,
   has exactly one possible order: there is no ordering decision left for anyone to
   get wrong. That is arithmetic, not a claim about the producer — and it is what
   makes the synthetic `/ActualText` fixture (`one marked sequence per line`)
   establishable rather than guessable.
4. Producer fingerprint from `/Producer` + `/Creator` (UTF-16 aware), recorded from
   inspected evidence, never guessed per document, and applied only to the lines
   still waiting for it:
   * Chromium/Skia → stores logical order; nothing is inverted (`to_unicode_logical`).
   * Microsoft Word / Adobe InDesign → stores **visual** order; every pending line
     with an RTL run is inverted back (cluster-safe: combining marks stay with their
     base, embedded LTR runs restored) and labelled `producer_visual_order_known`.
5. Any other producer with a pending RTL line → `unsupported_visual_order` → refuse
   (exit 3), page-granular.

Consulting the fingerprint for the whole page instead — the first implementation —
refused pages whose order was already settled, and would have re-inverted lines the
producer had already marked. `crates/pdfrtl-core/tests/text_recovery.rs::reversed_chars_marker_is_order_evidence_no_matter_the_producer`
pins rungs 2 and 4 with one marker as the only variable;
`crates/pdfrtl-core/src/text/recover.rs`'s `producer_fingerprint_allow_lists_are_pinned`
pins the allow-lists themselves — **a fingerprint family with no backing test does not
count as order evidence.**

What is *not* evidence: a clean `/ToUnicode` decode (see Context), and a text that
"looks plausible" in either direction. Where both orders would look plausible we
refuse — there is no third outcome.

**Reporting.** `reports/validate-archive.*` counts two separate numbers: files fully
decoded, and files whose logical order is verified. "Recovered" means the second one.

**Output gating.** Characters whose order is unestablished are never emitted: the page
reports `text: ""` plus `unordered_chars`, and `data.text` skips that page entirely.
Decoding is preserved as a count, order is preserved as a refusal — see
`docs/CLI.md` → *Text is order-gated*. The envelope carries `unordered_chars` at page
and document level for exactly this number.

**Control.** `crates/pdfrtl-core/tests/no_silent_reversal.rs` asserts, per language,
that extracted text contains the logical form of a pure-RTL word and zero occurrences
of its character-reversed form (word-boundary matched, so `דוח` inside `חודש` counts in
neither direction). It runs on the redistributable fixtures everywhere and on a
private-archive subset when present locally, skipping cleanly otherwise.

## Envelope and exit-code semantics (as implemented)

| outcome | `ok` | exit |
|---|---|---|
| every page decoded **and** ordered | `true` | 0 |
| some pages refused (decode or order) | `false` | 3 |
| no page emitted text | `false` | 3 |
| document could not be opened | `false` | 4 |

* Success keeps the existing shape: `{"ok":true,"data":{"text":…,"unordered_chars":0,
  "pages":[{"page":N,"text":…,"unordered_chars":0}]}, "reasons":[…]}`.
* Partial results use the same `data` shape with `ok:false` plus an `error` summary;
  each page entry carries `page`, `text`, `ok`, `unordered_chars` and its own `reasons`.
  `text` is `""` for a page whose order was not established — the decoded characters
  survive only as `unordered_chars` (page level, and `data.unordered_chars` for the
  document).
* In non-JSON mode, partial results still print the order-established text to stdout;
  the refusal summary (including the withheld character count) goes to stderr, and the
  exit code stays 3.

## Consequences

* A caller can now trust `ok: true`: it means every page decoded *and* the order rule
  that fired is named in `reasons`.
* Partial files are usable (200,300 characters recovered from `arabic-2` instead of
  zero) while remaining honestly labelled — `ok: false`, exit 3, per-page reasons.
* A file whose pages decode but whose order cannot be established (Acrobat output
  outside the allow-list, e.g. `arabic-3`, 1,187,357 characters) returns
  `data.text: ""`, `data.unordered_chars: 1187357`, `ok: false`, exit 3. The number
  says what we did; the refusal says what we refuse to claim. Previously the same
  call returned those 1.18M characters in visual order with a refusal attached —
  a caller reading only `data.text` got silently reversed Arabic.
* Two producer families are still refused outright for RTL: producers outside the
  fingerprint allow-list (`unsupported_visual_order`) and fonts without a usable
  mapping (`unsupported_broken_to_unicode`). Each refusal names its page.
* The allow-lists are evidence, not speculation: they may grow when a producer is
  *measured* to store a given order, and never by loosening an assertion.
