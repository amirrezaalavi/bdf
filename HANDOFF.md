# HANDOFF — pdfrtl

**Purpose:** let another machine (or a fresh agent with zero chat history) continue this
project. If something here is stale, fix it in the same commit that made it stale.

* **Updated:** 2026-09-27 (Phase 0 complete, P0 skeleton verified)
* **Repo:** `C:/Users/netcon/playground/ai/pdfrtl` (Windows) · `/mnt/c/Users/netcon/playground/ai/pdfrtl` (WSL)
* **Plan of record:** `.hermes/plans/2026-09-27_160341-pdfrtl-skeleton-and-p0-plan.md`
* **State:** workspace builds, all tests pass, licence gate designed, corpus harness runs.

## Where things stand (verified, not claimed)

| Item | State | Evidence |
|---|---|---|
| Rust workspace (`pdfrtl-core`, `pdfrtl-cli`, `pdfrtl-mcp`) | builds | `cargo test --workspace` → 11 tests pass |
| `pdfrtl inspect <file> --json` | works | exit 0, one-line JSON envelope, `pages: 1` |
| CLI contract (envelope + exit codes 0/2/3/4) | tested | `crates/pdfrtl-cli/tests/cli_contract.rs` (5 tests) |
| fmt + clippy `-D warnings` | clean | `scripts/wsl-build.sh` |
| Dependency ledger | generated | `docs/DEPS.md` from `cargo metadata` |
| Corpus harness | runs | `reports/2026-09-27T124930Z-corpus.md` (1 fixture, pass) |
| Synthetic fixture | oracle-verified | `pdftotext minimal-ltr.pdf -` → `Hello pdfrtl`; sha256 stable across runs |
| `cargo deny` gate | pending first run | `cargo install cargo-deny --locked` was running at handoff time |
| RTL text pipeline | not started | P1/P2 work; see plan Phase 4 |
| Corpus (real files, 4 languages) | not started | shopping list in `docs/CORPUS-ACQUISITION.md` |

## Reproduce from scratch on a new machine

```bash
# 1. get the code
git clone <repo> pdfrtl && cd pdfrtl

# 2. Rust (WSL/Linux/macOS). This host already had 1.97.1.
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --component rustfmt,clippy
. "$HOME/.cargo/env"

# 3. the gate
bash scripts/wsl-build.sh          # fmt, clippy -D warnings, tests, cargo deny
bash scripts/wsl-build.sh --quick  # everything except cargo deny (fast loop)

# 4. fixtures + corpus
python tools/gen_minimal_pdf.py                     # regenerate the synthetic fixture (byte-identical)
python3 scripts/gen-deps.py                         # refresh docs/DEPS.md
python3 scripts/run-corpus.py --update-hashes
python3 scripts/run-corpus.py --binary "$CARGO_TARGET_DIR/debug/pdfrtl"
```

## Environment traps on the current host (cost real time — read this)

| Trap | Symptom | Fix already applied |
|---|---|---|
| **No Rust toolchain on the Windows host** | `cargo: command not found` | build in WSL Ubuntu-26.04; rustc/cargo 1.97.1 lives in `~/.cargo` |
| Non-login shell has no cargo on PATH | `bash: cargo: command not found` | `. "$HOME/.cargo/env"` — `scripts/wsl-build.sh` does this |
| WSL resolver was 8.8.8.8/1.1.1.1 (blocked locally) | every `curl` returns `000` | `/etc/resolv.conf` → `nameserver 192.168.13.4` (FortiGate) |
| WSL has **no IPv6 route**, DNS returns AAAA | `curl` silently fails on some hosts | `/etc/gai.conf` → `precedence ::ffff:0:0/96 100` (prefer IPv4). Do not remove |
| crates.io CDN unreliable from this network | `cargo fetch` stalls | `~/.cargo/config.toml` → Aliyun sparse mirror (verified 200) |
| cargo over `/mnt/c` is slow | long builds | `CARGO_TARGET_DIR=$HOME/target-pdfrtl` (ext4), sources stay on Windows |
| `python3` does not exist on Windows | `command not found` | use `python` there, `python3` in WSL — scripts must tolerate both |
| Windows-side blocked hosts | connection `000` | SOCKS5 `127.0.0.1:10808`: `curl --socks5-hostname 127.0.0.1:10808 <url>` |
| git-bash mangles multi-line `for` loops passed to `wsl.exe` | loop body sees empty vars | prefer a script file inside WSL, or one command per call |

## Blocking decisions (see `docs/OPEN-QUESTIONS.md`)

1. **Q-001** paid tier: self-hosted licensed binary vs hosted SaaS (shapes the MCP design).
2. **Q-003** script scope: fa/ar/he/en only, or also Urdu/Kurdish/Syriac (+ vertical CJK?).
3. **Q-006** who reviews Persian/Arabic output, or is oracle-only review accepted for P0.
4. **Q-007** ownership/provenance of the `al-bdf-engine` RTL work (port vs re-derive).

## Next three tasks (in order)

1. **Verify the licence gate for real:** finish `cargo install cargo-deny --locked`, run
   `cargo deny check`, fix any allow-list fallout, record the output in `docs/reviews/`.
2. **Corpus tier 1:** pull 2–3 public-domain files (start with `fa` + `ar` + `he`
   literature), add manifest rows with real sha256 + provenance, mark any copyrighted
   textbook `redistributable: false` → `corpus/raw/private/`.
3. **Producer variants:** render the four fixed sentences (see
   `docs/CORPUS-ACQUISITION.md` §Tier 2) through Word and Chrome (both installed on this
   host) to create the first real visual-order fixtures — the ones the extraction heuristic
   must get right.

## What must not drift

* The invariant: **logical order or an explicit reason** — never silent reversal
  (`docs/decisions/0002`).
* Permissive-only dependencies in shipped default features (`docs/decisions/0003`).
* Authorship trailers on every commit (`AUTHORSHIP.md`).
* Determinism: fixtures byte-identical across runs; no timestamps in output.
