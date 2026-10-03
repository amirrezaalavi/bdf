# HANDOFF — pdfrtl

**Purpose:** let another machine (or a fresh agent with zero chat history) continue this
project. If something here is stale, fix it in the same commit that made it stale.

* **Updated:** 2026-10-03 — handoff refreshed for a fresh public clone
* **Repo:** `https://github.com/amirrezaalavi/bdf` (public mirror; `main` was `9411645b` before this handoff update)
* **Development environment:** WSL Ubuntu-26.04, `~/playground/ai/bdf`; exact Rust/Cargo 1.98.1 from `rust-toolchain.toml`
* **Private corpus:** 51 customer PDFs stay owner-local and gitignored. Public clone has 14 synthetic fixtures; never treat the missing private PDFs as a broken clone or publish them.
* **Plan of record:** `docs/plans/2026-09-29-lane-plan.md`; current priorities below.
* **State:** extraction works but remains incomplete; generation/editing are not implemented.
* **New clone:** follow `README.md` §0, then run `bash scripts/verify-clone.sh` and `bash scripts/wsl-build.sh`.

## Where things stand (verified, not claimed)

| Item | State | Evidence |
|---|---|---|
| Rust workspace (`pdfrtl-core`, `pdfrtl-cli`, `pdfrtl-mcp`) | builds | `bash scripts/wsl-build.sh` → gate GREEN: fmt, clippy `-D warnings`, 13 test suites, slop greps, deps-drift, `cargo deny` |
| CLI contract (envelope + exit codes 0/2/3/4) | tested | `crates/pdfrtl-cli/tests/cli_contract.rs` (8 tests, including the invariant control) |
| Text extraction | implemented, incomplete | `pdfrtl extract --json`; unsupported or ambiguous text is withheld with a reason |
| Archive validation | owner-local measurement | 38 fully decoded / 37 order-verified / 12 partial / 14 refused / 4 no-text; 1,038,880 emitted / 1,271,919 withheld. Cannot be reproduced from public clone without customer files. |
| Public fixtures / private PDFs | 14 fixtures in git; 51 real-world PDFs local only | Never commit the private PDFs or their identifying metadata. |
| Public mirror | current before this doc update: `9411645b` | publisher fix tested by forcing clone ahead of remote; CI passed and fix read back from remote |
| Generation / editing / MCP | not implemented | Editing spike says incremental save only (`7086526`). |
| OCR | deferred | Owner priority: correct reading/search first. |

## Reproduce from scratch on a new machine

```bash
git clone https://github.com/amirrezaalavi/bdf.git
cd bdf
# Install rustup if needed; rust-toolchain.toml pins 1.98.1.
export CARGO_TARGET_DIR="$HOME/target-bdf"
bash scripts/verify-clone.sh   # builds/tests and reports fixture + private-corpus status
bash scripts/wsl-build.sh      # full local gate, matching CI
```

The public clone has 14 redistributable PDF fixtures. The 51 real-world PDFs are private customer files and intentionally absent; archive results cannot be reproduced without owner-arranged access. Do not invent or publish a fetch location.

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

## Owner decisions

No decision currently blocks the next extraction work. Product/licensing questions and their status are tracked in `docs/OPEN-QUESTIONS.md`; verify that file before making a product-scope decision.
## Next work (in order)

1. **Phase 1a:** two-font RED fixture — one correct `/ToUnicode`, one broken — then withhold only text from the broken font; report the private archive delta.
2. **Phase 1b:** attempt embedded-font cmap inversion for `hebrew-1.pdf`; recover only what can be proved, otherwise refuse loudly; visually review rendered output.
3. **Phase 2:** faithful per-character prediction, then guarded order-consistency work; do not relax comparisons before the prediction is faithful.
4. **Handoff hygiene:** `tools/triage_pdfs.py` should refuse empty input; check/remove the unmatched `Unicode-DFS-2016` allowance; split the RTL skill into reference files when stable.
5. Only after extraction is reliable: writer, editing, MCP, packaging and enterprise work.

A red gate never moves on. Publishing goes through `scripts/publish-public.sh`, then verify the remote.

## What must not drift

* The invariant: **logical order or an explicit reason** — never silent reversal
  (`docs/decisions/0002`).
* Permissive-only dependencies in shipped default features (`docs/decisions/0003`).
* Authorship trailers on every commit (`AUTHORSHIP.md`).
* Determinism: fixtures byte-identical across runs; no timestamps in output.
