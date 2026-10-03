#!/usr/bin/env bash
# Build the static release binaries for both architectures, and verify each one.
#
# Why a script and not a remembered command: a release artifact that was produced by an
# undocumented one-liner cannot be reproduced, and CANARY.md already had to withdraw a
# "byte-for-byte reproducible" claim because nothing recorded how the binary was made
# (docs/reviews/2026-10-03-independent-critique-and-plan-changes.md §4.3).
#
# aarch64 is cross-linked with Rust's bundled `rust-lld`, so no cross-toolchain package is
# needed. That works because the musl target needs no C runtime: the whole point of musl is a
# self-contained libc, so `rust-lld` alone is sufficient to link it.
#
# Usage:
#   bash scripts/build-release.sh            # both architectures, into dist/
#   bash scripts/build-release.sh x86_64     # one of them
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

declare -A TARGETS=(
  [x86_64]="x86_64-unknown-linux-musl"
  [arm64]="aarch64-unknown-linux-musl"
)

WANTED=("${@:-}")
if [ ${#WANTED[@]} -eq 0 ] || [ "${WANTED[0]}" = "" ]; then
  WANTED=(x86_64 arm64)
fi

mkdir -p dist
: > dist/SHA256SUMS

for arch in "${WANTED[@]}"; do
  target="${TARGETS[$arch]:-}"
  if [ -z "$target" ]; then
    echo "unknown architecture: $arch (have: ${!TARGETS[*]})" >&2
    exit 2
  fi
  echo "== building $arch ($target)"
  rustup target list --installed | grep -qx "$target" || {
    echo "   rustup target add $target" >&2
    exit 2
  }

  # The linker override is what lets aarch64 link from an x86_64 host.
  case "$target" in
    aarch64-*) linker_var="CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=rust-lld" ;;
    *)         linker_var="" ;;
  esac

  env $linker_var \
    CARGO_TARGET_DIR="$HOME/target-bdf-$arch" \
    cargo build --release --locked -p pdfrtl-cli --target "$target"

  src="target/$target/release/pdfrtl"
  [ -f "$src" ] || src="$HOME/target-bdf-$arch/$target/release/pdfrtl"
  out="dist/pdfrtl-$arch-linux-musl"
  cp "$src" "$out"
  chmod +x "$out"

  # --- verify the artifact, do not trust the build log --------------------------
  # `file` prints "static-pie linked" for a static-PIE, which is still static, so matching the
  # exact phrase "statically linked" rejects a correct binary. Test the property instead:
  # a static binary has no dynamic dependencies at all. `readelf -d` is that test, and the
  # phrase check is only a sanity net for a non-PIE static build.
  echo "   verifying $out"
  if readelf -d "$out" 2>/dev/null | grep -q NEEDED; then
    echo "   FAIL: dynamic dependencies present:" >&2
    readelf -d "$out" | grep NEEDED >&2
    exit 1
  fi
  if ! file "$out" | grep -qi 'static'; then
    echo "   FAIL: not a static binary: $(file "$out")" >&2
    exit 1
  fi

  case "$arch" in
    x86_64) expected="x86-64" ;;
    arm64)  expected="aarch64" ;;
  esac
  if ! file "$out" | grep -qi "$expected"; then
    echo "   FAIL: wrong architecture, expected $expected" >&2
    exit 1
  fi
  file "$out" | sed 's/.*: /   /'
  sha256sum "$out" | awk '{print "   sha256 " substr($1,1,16) "…"}'
done

( cd dist && sha256sum pdfrtl-*-linux-musl >> SHA256SUMS )
echo
echo "== dist/"
ls -l dist/ | sed 's/^/   /'
echo
echo "== checksum file"
sed 's/^/   /' dist/SHA256SUMS
echo
if [ "$(uname -m)" = "x86_64" ] && [ -f dist/pdfrtl-x86_64-linux-musl ]; then
  echo "== smoke test: the x86_64 binary runs here and returns logical Persian"
  ./dist/pdfrtl-x86_64-linux-musl --version
  ./dist/pdfrtl-x86_64-linux-musl --json extract corpus/raw/synthetic/reversed-fa.pdf \
    | head -c 240
  echo
fi
echo "== the aarch64 binary cannot be executed on this x86_64 host; it was checked by ELF header only"