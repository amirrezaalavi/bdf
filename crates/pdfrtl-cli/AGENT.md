# crates/pdfrtl-cli/AGENT.md

The agent-facing product surface. `pdfrtl <verb> --json` is the API; the MCP server (P2)
is a thin adapter over exactly these functions.

## Layout

| path | what lives here |
|---|---|
| `src/main.rs` | argument parsing, dispatch, human mode |
| `src/lib.rs` | the public contract surface (so tests and the MCP adapter can import it) |
| `src/exit.rs` | exit codes — public API, pinned by tests, documented in `docs/CLI.md` |
| `tests/cli_contract.rs` | the contract tests. If these pass, agents can drive the tool |

## Rules specific to this crate

1. **One JSON line on stdout, always.** In `--json` mode even failures are JSON
   (`ok: false`). Never print progress, warnings or prose to stdout: an agent parsing
   stdout must not have to guess which line is the answer.
2. **Exit codes are part of the API** (`src/exit.rs`); callers branch on them, so they are
   never renumbered. New codes require a test and a `docs/CLI.md` row.
3. **No `unwrap()` on user input.** Bad flags are clap's problem (exit 2); bad files are
   exit 4 with the file named in the message.
4. **The MCP adapter must call these functions**, not shell out to the binary and not
   re-implement logic. One behaviour, one implementation.
5. **Adding a verb:** add the clap variant, the contract test (success + failure path),
   the `docs/CLI.md` section, and a reason code if it touches text — same commit.

## Commands

```bash
cargo run -q -p pdfrtl-cli -- --json inspect ../../corpus/raw/synthetic/minimal-ltr.pdf
cargo test -p pdfrtl-cli
cargo clippy -p pdfrtl-cli --all-targets -- -D warnings
```

Expected JSON (one line):

```json
{"data":{"creator":"pdfrtl-gen","encrypted":false,"font_count":null,"has_actual_text":null,
"pages":1,"path":"../../corpus/raw/synthetic/minimal-ltr.pdf","pdf_version":"1.7",
"producer":"pdfrtl-gen"},"ok":true,"reasons":[]}
```
