# scripts/AGENT.md

Small, single-purpose tools. No frameworks, stdlib only, `python`/`python3` agnostic
(Windows has `python`; WSL has `python3`).

| script | purpose | when to run |
|---|---|---|
| `wsl-build.sh` | the full gate, mirroring CI: fmt → clippy `-D warnings` → `cargo test --workspace --locked` → slop greps → DEPS drift → `cargo deny` | before every commit |
| `gen-deps.py` | regenerates `docs/DEPS.md` from `cargo metadata` | after any dependency change (CI fails on drift) |
| `run-corpus.py` | runs every fixture through the CLI, writes `reports/*-corpus.md` | after any change that touches text handling |
| `db.py` | SQLite+FTS5 knowledge base: tasks, findings, pitfalls | while working — record the *why* as you go |

## Rules

1. **No silent success.** A script that cannot do its job exits non-zero with a reason.
   `run-corpus.py` exits 1 if any fixture fails; `db.py task-done` refuses without `--ref`.
2. **Evidence over narrative.** These scripts exist to produce artifact files
   (`reports/`, `docs/DEPS.md`) that a reviewer reads instead of trusting a summary.
3. **No new runtime.** stdlib Python only — no `jq`, no `pip install`. The repo must run
   on a bare machine.
4. **Determinism.** No timestamps inside fixture hashing or content; timestamps only in
   report filenames.
5. **Don't duplicate the gate in CI.** CI calls these same scripts; a check that exists
   only in CI will be broken by the next local commit.
