# 0003 — the local gate and CI were different compilers, so the gate lied

**Found:** 2026-09-27 · **Impact:** three red CI runs on `main`; two agent reports
claimed "gate green" that were true locally and false upstream.

## Symptom

Every CI run on `main` failed at the same step while the local gate passed:

```
job gate -> failure
   ok   5. fmt
   FAIL 6. clippy (warnings are errors)
   ok   7. tests
jep drip, deny, slop, oracle -> success
```

The failing lint, from the raw job log (ANSI-stripped — CI sets
`CARGO_TERM_COLOR=always`, so a naive `grep 'error\['` finds nothing):

```
error: using `chunks_exact` with a constant chunk size
  --> crates/pdfrtl-core/src/text/cmap.rs:172   (and recover.rs:364)
   |  .chunks_exact(2)
   = help: ... clippy/rust-1.98.0/index.html#chunks_exact_to_as_chunks
error: could not compile `pdfrtl-core` (lib test) due to 2 previous errors
##[error]Process completed with exit code 101.
```

## Root cause

`rust-toolchain.toml` pinned the **channel**, not a version:

```toml
[toolchain]
channel = "stable"
```

Its stated rationale was the opposite of what happened: the comment claimed the
exact compiler was pinned "in CI instead" via `dtolnay/rust-toolchain@1.97.1`.
That pin was decorative — the action runs `rustup default 1.97.1`, and a
`rust-toolchain.toml` in the repository **overrides the default**. So:

| side | what `stable` resolved to | clippy |
|---|---|---|
| GitHub runner | 1.98.1 (`48a229cea`, 2026-09-01) | 1.98.x — has `chunks_exact_to_as_chunks` |
| this machine's `~/.rustup` | 1.97.1 (never updated) | 0.1.97 — lint does not exist |

The lint was introduced after 1.97, so it could not fire locally at all. No amount
of re-running the local gate would ever have shown it.

## Why the mirror made it worse

`mirrors.aliyun.com/rustup` — the natural place to fetch a newer toolchain on this
network — is **stale**: it advertises `channel-rust-stable.toml` as **1.96.0** and
404s on the 1.97/1.98 tarballs. Following it would have moved the local toolchain
*backwards*. `static.rust-lang.org` is reachable from WSL and serves 1.98.1.

## Fix

1. Pin the version on both sides, because "stable" is not a version:
   `rust-toolchain.toml` → `channel = "1.98.1"`, and CI →
   `dtolnay/rust-toolchain@1.98.1`. Change them together or the drift returns.
2. Installed 1.98.1 locally (from `static.rust-lang.org`, not the Aliyun mirror)
   and re-ran the gate under it: the lint is gone after the code fix below.
3. Fixed the two sites — `cmap.rs` and `recover.rs` — with explicit index stepping
   instead of `.chunks_exact(2)`. The byte-level policy is byte-identical (a trailing
   odd byte is still dropped) and pinned by a test. `as_chunks`, which the lint
   suggests, is nightly-only and does not compile on stable — do not take the hint
   literally.
4. `scripts/publish-public.sh` now pushes the snapshot to a `snapshot/<timestamp>`
   branch, waits for the CI conclusion on that exact SHA (`scripts/ci-wait.sh`), and
   only fast-forwards `main` if CI passed. "No push to main until CI is green" is now
   enforced by a script rather than by remembering.

## Lessons

- **A gate that does not run the compilers CI runs is not a gate.** Pin exact
  versions on both sides; a floating channel silently becomes two toolchains.
- **A toolchain pin in a workflow can be overridden by a file in the repo.** If both
  exist, verify which one wins instead of assuming.
- **Check a dependency mirror's freshness before trusting it.** A stale mirror can
  pin you to an older toolchain than the one you asked for.
- **After publishing, read the CI conclusion for that SHA.** A successful `git push`
  says nothing about whether the code passed; here the push succeeded and CI was red.
