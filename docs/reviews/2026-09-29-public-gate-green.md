# Review — the public mirror's CI gate, green under PUBLIC conditions (2026-09-29)

**Scope:** the four changes that make a published snapshot pass CI again: the `slop`
job's single library `.expect()`, the `no-silent-reversal` coverage guard that could
never be satisfied on the public mirror, the missing pre-flight in
`scripts/publish-public.sh`, and the local gate drifting from CI's command line.

**Binary/versions:** rustc 1.98.1 (48a229cea 2026-09-01) · cargo 1.98.1 · target dirs
`$HOME/target-pdfrtl` (this checkout), `$HOME/target-pdfrtl-public` (staged snapshot),
`$HOME/target-pdfrtl-preflight` (publish pre-flight) · sources on `/mnt/c`.

## What changed

| # | CI symptom | file:line | change |
|---|---|---|---|
| A | `slop`: `expect(` in library code | `crates/pdfrtl-core/src/text/encoding.rs:170-177` | `uni_form` collects with `let Ok(unit) = … else { return None }`. The parse cannot fail after the ASCII-hexdigit + multiple-of-4 checks above it; if it ever did, the library now *refuses the glyph name* (`None` → explicit unsupported reason) instead of panicking. `filter_map(..ok())` was rejected: dropping a unit silently shortens the string, which is a wrong answer, not a refusal. |
| B | `gate`: `no-silent-reversal control ran on only 3 word(s)` on the public mirror | `tools/gen_actualtext_fixture.py:49-53`, `crates/pdfrtl-core/tests/no_silent_reversal.rs:50-59` and `:152-159`, `corpus/manifest.json`, `corpus/expected/synthetic-actualtext-{ar,he}-001.json` | the generator now builds `actualtext-ar.pdf` and `actualtext-he.pdf` (same Chrome-shaped `/ActualText`-per-line structure as the fa fixture); both are committed, manifested, and added to `CASES`. The word-count assert is unchanged and a **per-language** assert was added, so `corpus/raw/private/` absent is still green while a missing language's fixture is still red. |
| C | process hole: a red snapshot was pushed and only CI noticed | `scripts/publish-public.sh:111-149` | after the deny-list passes, the CI gate is run **inside the staged snapshot** (no `corpus/raw/private`) with `scripts/wsl-build.sh --quick`; failure aborts with the failing command, exit 4, before the mirror tree is touched. `--skip-preflight` opts out, documented in the header. |
| D | local gate ≠ CI command line | `scripts/wsl-build.sh:19-45,58-77` | `cargo test --workspace --locked`, plus the CI jobs the local gate was missing (slop greps, `gen-deps.py --check`). A foreign tree (`PDFRTL_ROOT`) gets its **own** target dir. |

### Why the target dir is part of fix C/D

Test binaries bake in `env!("CARGO_MANIFEST_DIR")` — that is how `no_silent_reversal`
finds `corpus/`. Cargo reuses artifacts across checkouts that share a relative layout
and file mtimes (the staged snapshot is a `tar` copy: same layout, preserved mtimes), so
running the gate on the staged tree with *this* checkout's target dir reused *this*
checkout's binaries and the control silently read the private corpus again — a gate that
passes while CI fails, i.e. the exact failure class of `docs/problems/0003`. Observed
directly:

```
$ strings -a $HOME/target-pdfrtl/debug/deps/no_silent_reversal-5684494cf84962b5 | grep -m1 ai/pdfrtl
/mnt/c/Users/netcon/playground/ai/pdfrtl/crates/pdfrtl-core      <- private checkout, not the stage
$ strings -a $HOME/target-pdfrtl-preflight/debug/deps/no_silent_reversal-5684494cf84962b5 | grep -m1 mirror
/mnt/c/Users/netcon/playground/ai/bdf-test-mirror/.stage/crates/pdfrtl-core   <- staged tree
```

`wsl-build.sh` therefore keys the target dir to the tree it gates and refuses (exit 5)
a pairing whose recorded root differs.

## New fixtures (built by our own tooling, reproducible)

| fixture | sentences (one `/ActualText` run per line) | sha256 |
|---|---|---|
| `corpus/raw/synthetic/actualtext-ar.pdf` | `السلام عليكم` / `مرحبا بالعالم` | `99f6563b47490c7ae92ec60bd3064fe468a9034a7ea4528656f7bb8e6eaad953` |
| `corpus/raw/synthetic/actualtext-he.pdf` | `שלום עולם` / `תודה רבה על העזרה` | `319c586b5e952a89b72d8b10d7b74fdaa880ab8df678416b462144a9c4bfd6b5` |

`python3 tools/gen_actualtext_fixture.py ar|he` regenerates them byte-identically
(two runs, same hashes); the no-argument invocation still regenerates `actualtext-fa.pdf`
at its original hash `0d6ac126911261200bbd011fc795aa11a6b7cab2d088cbf1ec1c6710524438e5`.
Expected text: `corpus/expected/synthetic-actualtext-ar-001.json` and
`…-he-001.json`, same shape as the fa file, `verified_by: null` — per `corpus/AGENT.md`
rule 5 only a human flips these to `verified`.

Asserted words (pure RTL, four or more letters, no ZWNJ, no niqqud):

| fixture | words | logical / reversed counts in the extracted text |
|---|---|---|
| ar | `السلام` (6), `مرحبا` (5) | 1 / 0, 1 / 0 |
| he | `שלום` (4), `עולם` (4), `תודה` (4) | 1 / 0, 1 / 0, 1 / 0 |

CLI, raw:

```
$ pdfrtl --json extract corpus/raw/synthetic/actualtext-ar.pdf
{"data":{"pages":[{"ok":true,"page":1,"reasons":["actual_text"],"text":"السلام عليكم\nمرحبا بالعالم","unordered_chars":0}],"text":"السلام عليكم\nمرحبا بالعالم","unordered_chars":0},"ok":true,"reasons":["actual_text"]}   exit=0
$ pdfrtl --json extract corpus/raw/synthetic/actualtext-he.pdf
{"data":{"pages":[{"ok":true,"page":1,"reasons":["actual_text"],"text":"שלום עולם\nתודה רבה על העזרה","unordered_chars":0}],"text":"שלום עולם\nתודה רבה על העזרה","unordered_chars":0},"ok":true,"reasons":["actual_text"]}   exit=0
```

## Proof 1 — no `unwrap`/`expect` in library code

```
$ grep -rnE '\b(unwrap|expect)\(' crates/pdfrtl-core/src --include='*.rs'
grep exit=1 (1 = no matches)
```

(identical to the CI `slop` job's grep: empty output, so CI's `if grep …; then` is false.)

## Proof 2 — local full gate (private corpus present)

```
$ bash scripts/wsl-build.sh          # GATE_EXIT=0
== rustc: rustc 1.98.1 (48a229cea 2026-09-01) ==
== gate root: /mnt/c/Users/netcon/playground/ai/pdfrtl ==
== target dir: /home/netcon/target-pdfrtl ==
-- cargo fmt --all -- --check --
-- cargo clippy --workspace --all-targets -- -D warnings --
-- cargo test --workspace --locked --
…
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out        (cli_contract)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out        (pdfrtl_core unit)
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out        (docinfo)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out        (no_silent_reversal)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out        (reasons)
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out       (text_recovery)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out        (pdfrtl_mcp)
-- slop gate (CI job 'slop': no unwrap/expect/todo/dbg/prints in shipped code) --
slop gate clean
-- dependency ledger (CI job 'deps-drift') --
docs/DEPS.md is up to date
-- license/dependency gate (CI job 'deny') --
advisories ok, bans ok, licenses ok, sources ok
== gate green ==
```

## Proof 3 — PUBLIC conditions: staged snapshot, exact CI commands

Staged with `scripts/publish-public.sh`'s own tar excludes (no `.git`, no
`corpus/raw/private`, no `reports/`) plus its manifest strip, in a **fresh** target dir:

```
### corpus/raw in the staged tree:
generated
synthetic
### corpus/raw/private in the staged tree:
ls: cannot access 'corpus/raw/private': No such file or directory
### private manifest rows left:
0
### command 1: cargo fmt --all -- --check
exit=0
### command 2: cargo clippy --workspace --all-targets -- -D warnings
    Checking pdfrtl-core v0.0.1 (/mnt/c/Users/netcon/playground/ai/pdfrtl-public-stage/crates/pdfrtl-core)
    Checking pdfrtl-cli v0.0.1 (/mnt/c/Users/netcon/playground/ai/pdfrtl-public-stage/crates/pdfrtl-cli)
    Checking pdfrtl-mcp v0.0.1 (/mnt/c/Users/netcon/playground/ai/pdfrtl-public-stage/crates/pdfrtl-mcp)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 7.50s
exit=0
### command 3: cargo test --workspace --locked
    Finished `test` profile [unoptimized + debuginfo] target(s) in 9.02s
     Running tests/cli_contract.rs …        test result: ok. 8 passed; 0 failed …
     Running unittests src/lib.rs (pdfrtl_core) …  test result: ok. 9 passed; 0 failed …
     Running tests/docinfo.rs …             test result: ok. 2 passed; 0 failed …
     Running tests/no_silent_reversal.rs …  test result: ok. 1 passed; 0 failed …
     Running tests/reasons.rs …             test result: ok. 3 passed; 0 failed …
     Running tests/text_recovery.rs …       test result: ok. 10 passed; 0 failed …
     Running unittests src/lib.rs (pdfrtl_mcp) …  test result: ok. 1 passed; 0 failed …
exit=0
### the control, uncaptured (which cases ran, which were skipped)
running 1 test
skip (not present locally): corpus/raw/private/desktop-pdfs/persian-7.pdf
skip (not present locally): corpus/raw/private/desktop-pdfs/arabic-2.pdf
skip (not present locally): corpus/raw/private/desktop-pdfs/hebrew-4.pdf
test rtl_text_is_never_silently_reversed ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
exit=0
```

The control ran its cases (8 public words: 3 fa from the Chrome fixtures + 2 ar + 3 he)
and only skipped the private subset.

### The guard still fails when a language's fixtures are missing

With `actualtext-he.pdf` removed from the staged tree:

```
$ cargo test --locked --test no_silent_reversal -- --nocapture
skip (not present locally): corpus/raw/synthetic/actualtext-he.pdf
skip (not present locally): corpus/raw/private/desktop-pdfs/persian-7.pdf
skip (not present locally): corpus/raw/private/desktop-pdfs/arabic-2.pdf
skip (not present locally): corpus/raw/private/desktop-pdfs/hebrew-4.pdf
thread 'rtl_text_is_never_silently_reversed' panicked at crates/pdfrtl-core/tests/no_silent_reversal.rs:153:9:
no-silent-reversal control never ran a he fixture (4 file(s) skipped, 5 word(s) checked) — fa/ar/he must each be covered by a redistributable fixture
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```

Note `5 word(s) checked`: the old `checked >= 4` guard alone would have **passed** here
(5 ≥ 4) with Hebrew silently uncovered. That is why the per-language assert exists.

## Proof 4 — the publish pre-flight (run against a local throwaway remote, never GitHub)

Failure path — a badly formatted `const` in the staged tree:

```
== staging working tree (private corpus and reports excluded) ==
deny-list clean (70 private identities checked)
== pre-flight: CI gate on the staged snapshot ==
== gate root: /mnt/c/Users/netcon/playground/ai/bdf-test-mirror/.stage ==
== target dir: /home/netcon/target-pdfrtl-preflight ==
-- cargo fmt --all -- --check --
Diff in …/.stage/crates/pdfrtl-core/src/lib.rs:11:
-const PREFLIGHT_PROBE:u8=1;
+const PREFLIGHT_PROBE: u8 = 1;
PREFLIGHT FAILED - the staged snapshot is NOT publishable, nothing was pushed.
  failing command: cargo fmt --all -- --check
  This was a PUBLIC-CONDITIONS gate: it ran on the staged tree, which has NO
  corpus/raw/private, using the exact commands from .github/workflows/ci.yml.
  main and the remote are untouched. Log tail:
PUBLISH_PIPELINE_EXIT=4
$ git -C <test remote> for-each-ref     # nothing — no ref was created
$ git -C <test mirror> status --short   # ?? .stage/  only — working tree not replaced
```

Happy path — the same run with the probe removed:

```
== staging working tree (private corpus and reports excluded) ==
deny-list clean (70 private identities checked)
== pre-flight: CI gate on the staged snapshot ==
== gate root: /mnt/c/Users/netcon/playground/ai/bdf-test-mirror/.stage ==
== target dir: /home/netcon/target-pdfrtl-preflight ==
-- cargo fmt --all -- --check --
-- cargo clippy --workspace --all-targets -- -D warnings --
-- cargo test --workspace --locked --
-- slop gate (CI job 'slop': no unwrap/expect/todo/dbg/prints in shipped code) --
slop gate clean
-- dependency ledger (CI job 'deps-drift') --
docs/DEPS.md is up to date
== gate green ==
== pre-flight green: the staged snapshot passes the CI gate under public conditions ==
== replacing mirror working tree ==
== pushing snapshot/20260929T060744Z (main stays untouched until CI passes) ==
ci-wait: waiting — no check runs registered yet
```

`ci-wait` was killed at that point on purpose: the remote here was a local bare repo, so
GitHub could never have checks for that SHA. The push target was
`C:/Users/netcon/playground/ai/bdf-test-remote.git`; the real mirror
(`~/playground/ai/bdf-snapshot`) and `origin/main` were never contacted.

## Corpus harness

```
$ python3 scripts/run-corpus.py --binary "$CARGO_TARGET_DIR/debug/pdfrtl" --only synthetic-actualtext-
skip  synthetic-actualtext-fa-001  -  expected text present but unverified by a human — not asserted
skip  synthetic-actualtext-ar-001  -  expected text present but unverified by a human — not asserted
skip  synthetic-actualtext-he-001  -  expected text present but unverified by a human — not asserted
report: reports/2026-09-29T061112Z-corpus.md
CORPUS_EXIT=0
```

P0 (sha256, `pages`, producer `pdfrtl-gen` matches `/Info /Producer`) passes for both new
fixtures; extraction equality stays `skip` until a human sets `verified_by`.

## Still open (human)

1. **Text review (AGENTS.md rule 8):** the ar/he sentences and their expected JSON need a
   human read before `verified_by` / `status: verified`. Nothing here asserts a claim
   beyond "the `/ActualText` payload comes back verbatim".
2. **`AGENTS.md` still says `cargo test --workspace`** for `wsl-build.sh` (line 60);
   `AGENTS.md` is a protected file, so it was not edited — the script is the authority
   now, please update that line by hand.
3. `--skip-preflight` has never been used; its only purpose is an emergency publish.
