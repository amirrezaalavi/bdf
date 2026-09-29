# HANDOFF — pdfrtl

**Purpose:** let another machine (or a fresh agent with zero chat history) continue this
project. If something here is stale, fix it in the same commit that made it stale.

* **Updated:** 2026-09-29 (Phase 2: order recovery; rung 3 landed and fixed; dev root moved to WSL)
* **Repo:** `~/playground/ai/bdf` (WSL Ubuntu-26.04, ext4) — the dev/build root ·
  `C:/Users/netcon/playground/ai/pdfrtl` (Windows) is a **read-only mirror**, do not build there
* **Plan of record:** `.hermes/plans/2026-09-27_160341-pdfrtl-skeleton-and-p0-plan.md`
* **State:** workspace builds, all tests pass, licence gate designed, corpus harness runs.

## Where things stand (verified, not claimed)

| Item | State | Evidence |
|---|---|---|
| Rust workspace (`pdfrtl-core`, `pdfrtl-cli`, `pdfrtl-mcp`) | builds | `bash scripts/wsl-build.sh` → gate GREEN: fmt, clippy `-D warnings`, 13 test suites, slop greps, deps-drift, `cargo deny` |
| CLI contract (envelope + exit codes 0/2/3/4) | tested | `crates/pdfrtl-cli/tests/cli_contract.rs` (8 tests, including the invariant control) |
| Order invariant | **enforced** | a page whose order is unproven is withheld and counted (`unordered_chars`), never emitted as text — ADR 0002/0004, `docs/problems/0007` |
| Text extraction | real-world capable | per-page granularity; `/ActualText`, `/ReversedChars`, UAX #9 and producer-fingerprint rungs |
| Archive validation | **measured, honestly** | **37 / 51** order-verified (14 of them RTL), **14 refuse** `unsupported_visual_order`, 1,035,606 chars emitted / 1,275,183 withheld — `reports/validate-archive.md` |
| Verification lane | in use | PDFium pixels + vision review (`docs/problems/0004`), `scripts/render_pages.py` |
| Corpus | 61 rows local / 10 public | private bytes stay gitignored; hashes, metadata and triage travel in git (`corpus/manifest.json`) |
| Public mirror | clean, scratch | `github.com/amirrezaalavi/bdf` — 10 manifest rows / 0 private; history reset deferred (ADR 0005) |
| Generation / editing | not started | the `lopdf` spike says incremental save only (commit `7086526`); no editor code yet |
| Knowledge gaps | queued | `docs/RESEARCH-QUESTIONS.md` — 8 questions for an outside research agent |

## Reproduce from scratch on a new machine

```bash
# 1. get the code
git clone <repo> pdfrtl && cd pdfrtl

# 2. Rust (WSL/Linux/macOS). Pinned to 1.98.1 by rust-toolchain.toml; ci.yml pins the same.
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
| **No Rust toolchain on the Windows host** | `cargo: command not found` | build in WSL Ubuntu-26.04; rustc/cargo 1.98.1 lives in `~/.cargo` (pinned by `rust-toolchain.toml` — never `stable`) |
| Non-login shell has no cargo on PATH | `bash: cargo: command not found` | `. "$HOME/.cargo/env"` — `scripts/wsl-build.sh` does this |
| WSL resolver was 8.8.8.8/1.1.1.1 (blocked locally) | every `curl` returns `000` | `/etc/resolv.conf` → `nameserver 192.168.13.4` (FortiGate) |
| WSL has **no IPv6 route**, DNS returns AAAA | `curl` silently fails on some hosts | `/etc/gai.conf` → `precedence ::ffff:0:0/96 100` (prefer IPv4). Do not remove |
| crates.io CDN unreliable from this network | `cargo fetch` stalls | `~/.cargo/config.toml` → Aliyun sparse mirror (verified 200) |
| cargo over `/mnt/c` is slow | long builds | the dev root itself moved to ext4: `~/playground/ai/bdf`, `CARGO_TARGET_DIR=$HOME/target-bdf`; the Windows checkout is a read-only mirror |
| `python3` does not exist on Windows | `command not found` | use `python` there, `python3` in WSL — scripts must tolerate both |
| Windows-side blocked hosts | connection `000` | SOCKS5 `127.0.0.1:10808`: `curl --socks5-hostname 127.0.0.1:10808 <url>` |
| git-bash mangles multi-line `for` loops passed to `wsl.exe` | loop body sees empty vars | prefer a script file inside WSL, or one command per call |
| **WSL `/tmp` is wiped between separate `wsl.exe` invocations** | a log or fixture written by one call is gone by the next; the follow-up reports a bogus IO error (exit 4 on a file that was never there) | keep cross-call artifacts under `$HOME` (e.g. `~/verify-pdfrtl/`), never `/tmp` |
| Only `python3` exists in WSL 26.04 (`python` is absent) | `python: command not found` | use `python3` in WSL; a script that says `python` fails there and reads as "no interpreter installed" |

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
