#!/usr/bin/env bash
# Full local gate for pdfrtl. Sources live on the Windows checkout; build artifacts
# go to ext4 ($CARGO_TARGET_DIR) because cargo over /mnt/c is dramatically slower.
#
# This script mirrors .github/workflows/ci.yml command for command — CI is the
# authority, never the other way round. Same compiler (rust-toolchain.toml pins
# 1.98.1, as does the workflow), same `cargo test --workspace --locked`, the same
# slop greps, the same DEPS.md drift check. If a CI command changes, change this
# script in the same commit.
#
# Usage:  bash scripts/wsl-build.sh                 (run from WSL)
#         bash scripts/wsl-build.sh --quick         (skip cargo-deny)
#         PDFRTL_ROOT=/mnt/<other/checkout> bash scripts/wsl-build.sh --quick
#             gate a DIFFERENT tree than this checkout — scripts/publish-public.sh
#             uses that to run the CI gate on the staged snapshot under PUBLIC
#             conditions (no corpus/raw/private) before anything is pushed.
set -euo pipefail

REPO="${PDFRTL_ROOT:-/mnt/c/Users/netcon/playground/ai/pdfrtl}"
# The target dir is keyed to the tree being gated. Test binaries bake in their source
# tree (`env!("CARGO_MANIFEST_DIR")` is the corpus root the tests read), and cargo reuses
# artifacts across checkouts that share a relative layout and file mtimes — so gating a
# staged snapshot with binaries built from THIS checkout would silently test the private
# corpus instead of the snapshot. Same failure class as docs/problems/0003: a gate that
# passes while CI fails is worse than no gate. Foreign tree => its own target dir, and a
# stale pairing is a hard error rather than a silent pass.
if [ -n "${PDFRTL_ROOT:-}" ]; then
  export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/target-pdfrtl-preflight}"
else
  export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/target-pdfrtl}"
fi
# Non-login shells do not get ~/.cargo/bin on PATH; source rustup's env like rustup tells you to.
if [ -f "$HOME/.cargo/env" ]; then . "$HOME/.cargo/env"; fi
GATE_ROOT="$(cd "$REPO" && pwd -P)"
GATE_MARK="$CARGO_TARGET_DIR/.pdfrtl-gate-root"
if [ -f "$GATE_MARK" ] && [ "$(cat "$GATE_MARK")" != "$GATE_ROOT" ]; then
  echo "ERROR: $CARGO_TARGET_DIR holds artifacts built for a different tree:" >&2
  echo "  built for: $(cat "$GATE_MARK")" >&2
  echo "  gating:    $GATE_ROOT" >&2
  echo "Reusing them would gate the wrong tree (the baked CARGO_MANIFEST_DIR decides which" >&2
  echo "corpus the tests read). Point CARGO_TARGET_DIR at a clean directory for this tree." >&2
  exit 5
fi
mkdir -p "$CARGO_TARGET_DIR"
printf '%s\n' "$GATE_ROOT" > "$GATE_MARK"
cd "$REPO"

echo "== rustc: $(rustc -V) =="
echo "== gate root: $REPO =="
echo "== target dir: $CARGO_TARGET_DIR =="

echo "-- cargo fmt --all -- --check --"
cargo fmt --all -- --check

echo "-- cargo clippy --workspace --all-targets -- -D warnings --"
cargo clippy --workspace --all-targets -- -D warnings

echo "-- cargo test --workspace --locked --"
cargo test --workspace --locked

echo "-- slop gate (CI job 'slop': no unwrap/expect/todo/dbg/prints in shipped code) --"
if grep -rnE '\b(unwrap|expect)\(' crates/pdfrtl-core/src --include='*.rs' | grep -v '^\s*//'; then
  echo "unwrap()/expect() in library code — return Result instead" >&2
  exit 1
fi
if grep -rnE '\b(todo|unimplemented|dbg)!' crates/*/src --include='*.rs'; then
  echo "todo!/unimplemented!/dbg! left in shipped code" >&2
  exit 1
fi
if grep -rnE '\b(println|print|eprintln)!' crates/pdfrtl-core/src --include='*.rs'; then
  echo "core must not print; the CLI owns stdout" >&2
  exit 1
fi
echo "slop gate clean"

echo "-- dependency ledger (CI job 'deps-drift') --"
python3 scripts/gen-deps.py --check

if [ "${1:-}" != "--quick" ]; then
  echo "-- license/dependency gate (CI job 'deny') --"
  if command -v cargo-deny >/dev/null 2>&1; then
    cargo deny check
  else
    echo "cargo-deny not installed; install with: cargo install cargo-deny --locked" >&2
    exit 1
  fi
fi

echo "== gate green =="
