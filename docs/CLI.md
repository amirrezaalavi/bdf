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

## Reason codes

| code | meaning | ok? |
|---|---|---|
| `actual_text` | `/ActualText` marked content carried the logical text verbatim (authoritative) | yes |
| `to_unicode_logical` | `ToUnicode` mapped cleanly and run order was already logical | yes |
| `bidi_reordered` | `ToUnicode` mapped cleanly; order reconstructed from UAX #9 levels | yes |
| `producer_visual_order_known` | producer is allow-listed as storing visual order | yes |
| `unsupported_no_evidence` | no trustworthy recovery path — refused | no |
| `unsupported_broken_to_unicode` | `ToUnicode` missing or maps to C0 controls | no |

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
