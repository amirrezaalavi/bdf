# pdfrtl CLI — the agent contract

The CLI is the product surface. The MCP server is a thin adapter over it, so anything
described here is also what an agent gets through MCP.

## Envelope

Every verb prints **exactly one JSON object** on stdout, on success and on failure:

```json
{"ok": true, "data": { }, "reasons": []}
```

* `ok` — did the operation complete.
* `data` — the verb payload; on failure `{"error": "<message naming the input file>"}`.
* `reasons` — array of reason codes justifying how text order was produced.
  Empty for verbs that do not touch text.
  A non-empty `reasons` with `ok: true` means "produced, and here is why".
  An `unsupported_*` reason means we **refused to guess**.

No prose, no logging, no progress output on stdout. Diagnostics go to stderr.

## Exit codes

| code | meaning |
|---|---|
| 0 | completed |
| 2 | usage error (bad flags/subcommand — produced by clap) |
| 3 | unsupported: text order could not be established with justification |
| 4 | io/parse error (missing file, corrupt PDF, unreadable) |

Callers should branch on the **exit code**, not on string matching.

Extraction is **page-level** (ADR 0004): `ok: true` + exit 0 means *every* page
decoded **and** every page's order was established; if some pages were refused the
envelope keeps `ok: false` + exit 3 **and still carries the text** of every page whose
order *was* established, with each refused page listing its own reason. Recovered text
is never discarded, unproven text is never emitted (see below), and no file whose RTL
order is unestablished returns `ok: true`.

### Text is order-gated — there is no third outcome

`data.text` and `data.pages[].text` contain only text whose order a named rule
established. A page that decoded characters it cannot order reports:

```json
{"page": 7, "text": "", "ok": false, "reasons": ["unsupported_visual_order"],
 "unordered_chars": 4021}
```

* `text` is `""`. The characters are in the file and we read them — reading is not
  ordering — so they are not handed out as if they were reading order.
* `unordered_chars` is the honest count of those characters (page level) and
  `data.unordered_chars` the document total: *decoded, order unestablished*.
* the page's own `reasons` name the missing rung of the ladder; `ok` is `false`.

`ok` means "every glyph decoded **and** an order rule fired". A page can be
`ok: false` while its text is usable — part of a font we refused to map, with the
order established all the same; `unordered_chars: 0` is what distinguishes that case.
A caller that reads `data.text` and ignores `reasons` therefore still cannot be
handed reversed text.

## Reason codes

| code | meaning | ok? |
|---|---|---|
| `actual_text` | `/ActualText` marked content carried the logical text verbatim (authoritative) | yes |
| `to_unicode_logical` | `ToUnicode` mapped cleanly and the producer is measured to store logical order | yes |
| `bidi_reordered` | `ToUnicode` mapped cleanly; lines marked `/ReversedChars` were reordered to logical | yes |
| `encoding_mapped` | simple 8-bit font decoded via `/Encoding` + `/Differences` (ADR 0004) | yes |
| `producer_visual_order_known` | producer measured to store visual order; RTL lines inverted back to logical (ADR 0004) | yes |
| `unsupported_font_encoding` | font has neither `/ToUnicode` nor a usable `/Encoding` (e.g. `/Symbol`) — refused | no |
| `unsupported_visual_order` | RTL run order could not be established for the page — its characters are counted in `unordered_chars`, never emitted (ADR 0004) | no |
| `unsupported_no_evidence` | no trustworthy recovery path — refused | no |
| `unsupported_broken_to_unicode` | `ToUnicode` missing or maps to C0 controls | no |
| `unsupported_page_content` | content stream could not be tokenised/decoded (odd-length string, syntax) | no |

## Verbs

### `pdfrtl inspect <file>`

Level-1 facts. Never decodes text.

```json
{"ok":true,"data":{"path":"corpus/raw/synthetic/minimal-ltr.pdf","pages":1,
"producer":"pdfrtl-gen","creator":"pdfrtl-gen","pdf_version":"1.7","encrypted":false,
"has_actual_text":null,"font_count":null},"reasons":[]}
```

`has_actual_text` and `font_count` are `null` until P1 analyses them — `null` means
"not analysed", never "no".

### `pdfrtl extract <file>` (P1)

Planned fields per text run: `page`, `text` (logical order), `reason`, `object_id`,
`char_range`, `bbox`, `font`. Streaming: `--jsonl` emits one run per line.

### `pdfrtl search`, `generate`, `edit`, `sign` (P2–P4)

Reserved names; see `.hermes/plans/` for the phased scope. Adding a verb requires: a
reason code if it touches text, a fixture, and a `docs/CLI.md` section in the same
commit.

## Forward compatibility

* `--schema` (P1) prints the JSON Schema of the current envelope; agents should
  validate against it rather than assuming field order.
* New fields may be added; existing field names and exit codes will not change
  without a major version bump.
