#!/usr/bin/env bash
# Full local gate for pdfrtl. Sources live on the Windows checkout; build artifacts
# go to ext4 ($CARGO_TARGET_DIR) because cargo over /mnt/c is dramatically slower.
#
# Usage:  bash scripts/wsl-build.sh            (run from WSL)
#         bash scripts/wsl-build.sh --quick    (skip cargo-deny)
set -euo pipefail

REPO="/mnt/c/Users/netcon/playground/ai/pdfrtl"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/target-pdfrtl}"
# Non-login shells do not get ~/.cargo/bin on PATH; source rustup's env like rustup tells you to.
if [ -f "$HOME/.cargo/env" ]; then . "$HOME/.cargo/env"; fi
cd "$REPO"

echo "== rustc: $(rustc -V) =="
echo "== target dir: $CARGO_TARGET_DIR =="

echo "-- fmt --"
cargo fmt --all -- --check

echo "-- clippy (warnings are errors) --"
cargo clippy --workspace --all-targets -- -D warnings

echo "-- tests --"
cargo test --workspace

if [ "${1:-}" != "--quick" ]; then
  echo "-- license/dependency gate --"
  if command -v cargo-deny >/dev/null 2>&1; then
    cargo deny check
  else
    echo "cargo-deny not installed; install with: cargo install cargo-deny --locked" >&2
    exit 1
  fi
fi

echo "== gate green =="
