# crates/pdfrtl-core/AGENT.md

The core library: document model, RTL text pipeline, extraction, generation. **No I/O
beyond reading/writing PDFs, no printing, no CLI concerns.**

## Layout

| path | what lives here |
|---|---|
| `src/lib.rs` | public surface: `DocInfo`, `Reason`, `inspect` |
| `src/reasons.rs` | the invariant as a type — the most load-bearing file in the repo |
| `src/docinfo.rs` | level-1 file facts (pages, producer, encryption) |
| `src/text/` (P1) | normalizer → bidi → shaper → emitter; one module per stage |

## Rules specific to this crate

1. **No `unwrap()` / `expect()` / `panic!`** outside tests — CI enforces this with grep.
   Return `Result` and let the caller decide. A panic in a library is a bug in a service.
2. **No printing.** The CLI owns stdout. Diagnostics are returned, not emitted.
3. **Every text-producing function returns a `Reason`** or a type that carries one. If you
   cannot justify the order, return `unsupported_*` — that is a correct outcome, not a
   failure. See `docs/decisions/0002-logical-order-invariant.md`.
4. **Panics are not error handling** for programmer errors either: prefer types that make
   illegal states unrepresentable (this is why `Reason` is an enum and not a string).
5. **Nothing about the RTL pipeline may be "temporarily simplified".** A reversed output
   that looks right in a Latin fixture is the classic silent-corruption bug.

## Commands

```bash
cargo test -p pdfrtl-core                 # all core tests
cargo test -p pdfrtl-core --test reasons  # the invariant contract
cargo clippy -p pdfrtl-core --all-targets -- -D warnings
```

## Adding pipeline stages (P1+)

- One module per stage, each with a pure function (`&[u8] -> Result<Vec<Run>>` style) and
  unit tests driven by fixtures in `crates/pdfrtl-core/tests/`.
- The stage order is: normalize → bidi → shape → emit. Do not merge stages: the
  documented failure modes (see `docs/problems/`) all come from stages being entangled.
- Every new `Reason` variant needs a fixture that triggers it, in the same commit.
