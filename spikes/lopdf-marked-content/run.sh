#!/usr/bin/env bash
# One-command reproduction for the lopdf marked-content spike.
# Run from WSL (the Windows host has no Rust toolchain):
#   wsl.exe -d Ubuntu-26.04 -- bash /mnt/c/Users/netcon/playground/ai/pdfrtl/spikes/lopdf-marked-content/run.sh
# Produces: out/report.txt (raw evidence pasted into FINDINGS.md) + out/*.pdf
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
OUT="$HERE/out"
mkdir -p "$OUT"

if [ -f "$HOME/.cargo/env" ]; then . "$HOME/.cargo/env"; fi
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/target-pdfrtl-spike}"

# Adversarial input (a): a file with object streams + a cross-reference stream,
# produced by qpdf (oracle only, never shipped).
if command -v qpdf >/dev/null 2>&1; then
  qpdf --static-id --object-streams=generate \
    "$ROOT/corpus/raw/synthetic/actualtext-fa.pdf" "$OUT/in-qpdf-objstm.pdf"
  echo "qpdf: wrote $OUT/in-qpdf-objstm.pdf"
else
  echo "WARN: qpdf not found - E8 adversarial input not generated" >&2
fi

REPORT="$OUT/report.txt"
set -o pipefail
# --quiet keeps cargo's own status lines ("Finished ... in 0.42s") out of the report,
# otherwise the report's bytes depend on how long the build took and the artefact is
# not reproducible across invocations.
cargo run --quiet --manifest-path "$HERE/Cargo.toml" --release 2>&1 | tee "$REPORT"

# Determinism (AGENTS rule 4): run the whole spike a second time and require every
# artefact to be byte-identical.
sha256sum "$OUT"/*.pdf 2>/dev/null | sed "s|$OUT/||" > "$OUT/.sha-run1.txt" || true
cargo run --quiet --manifest-path "$HERE/Cargo.toml" --release > /dev/null 2>&1
sha256sum "$OUT"/*.pdf 2>/dev/null | sed "s|$OUT/||" > "$OUT/.sha-run2.txt" || true

{
  echo
  echo "===== ORACLE: tool versions ====="
  qpdf --version | head -1 || true
  pdftotext -v 2>&1 | head -1 || true
  rustc -V

  echo
  echo "===== ORACLE: determinism - second full run vs first (sha256 of out/*.pdf) ====="
  if diff -q "$OUT/.sha-run1.txt" "$OUT/.sha-run2.txt" >/dev/null; then
    echo "  IDENTICAL across two runs:"
    sed 's/^/    /' "$OUT/.sha-run1.txt"
  else
    echo "  DIFFERS between runs:"
    diff "$OUT/.sha-run1.txt" "$OUT/.sha-run2.txt" | sed 's/^/    /' || true
  fi

  echo
  echo "===== ORACLE: qpdf --check on every produced file ====="
  for f in "$OUT"/*.pdf; do
    [ -e "$f" ] || continue
    echo "--- $(basename "$f")"
    qpdf --check "$f" 2>&1 | grep -E "ERROR|WARNING|No syntax|not encrypted|not linearized" | sed 's/^/    /' || true
  done

  echo
  echo "===== ORACLE: qpdf --show-object structural identity, original vs lopdf round-trip ====="
  cmp_obj() { # $1 orig, $2 saved, rest = object ids
    local orig="$1" saved="$2"; shift 2
    for id in "$@"; do
      qpdf --show-object="$id" "$orig" > "$OUT/.o1" 2>/dev/null || true
      qpdf --show-object="$id" "$saved" > "$OUT/.o2" 2>/dev/null || true
      if cmp -s "$OUT/.o1" "$OUT/.o2"; then
        echo "    obj $id: structurally IDENTICAL (qpdf-normalised)"
      else
        echo "    obj $id: DIFFERS:"
        diff "$OUT/.o1" "$OUT/.o2" | sed 's/^/        /' || true
      fi
    done
  }
  echo "--- fa-zwnj-lamalef.pdf vs e1-fa-zwnj-lamalef.pdf"
  cmp_obj "$ROOT/corpus/raw/generated/chrome/fa-zwnj-lamalef.pdf" "$OUT/e1-fa-zwnj-lamalef.pdf" 1 2 3 4 6 18 22
  echo "--- actualtext-fa.pdf vs e1-actualtext-fa.pdf"
  cmp_obj "$ROOT/corpus/raw/synthetic/actualtext-fa.pdf" "$OUT/e1-actualtext-fa.pdf" 1 2 3 4 5 6 7
  echo "--- minimal-ltr.pdf vs e1-minimal-ltr.pdf"
  cmp_obj "$ROOT/corpus/raw/synthetic/minimal-ltr.pdf" "$OUT/e1-minimal-ltr.pdf" 1 2 3 4 5 6 7

  echo
  echo "===== ORACLE: pdftotext identity, original vs unmodified round-trip ====="
  for pair in \
    "corpus/raw/generated/chrome/fa-zwnj-lamalef.pdf:e1-fa-zwnj-lamalef.pdf" \
    "corpus/raw/synthetic/actualtext-fa.pdf:e1-actualtext-fa.pdf" \
    "corpus/raw/synthetic/minimal-ltr.pdf:e1-minimal-ltr.pdf" \
    "corpus/raw/synthetic/actualtext-fa.pdf:e8-objstm-rt.pdf" \
    "corpus/raw/synthetic/actualtext-fa.pdf:e9-roundtrip.pdf"; do
    src="${pair%%:*}"; dst="${pair##*:}"
    [ -e "$OUT/$dst" ] || { echo "  $dst: MISSING"; continue; }
    pdftotext -enc UTF-8 "$ROOT/$src" "$OUT/.a.txt" 2>/dev/null || true
    pdftotext -enc UTF-8 "$OUT/$dst" "$OUT/.b.txt" 2>/dev/null || true
    if diff -q "$OUT/.a.txt" "$OUT/.b.txt" >/dev/null 2>&1; then
      echo "  $dst: extracted text IDENTICAL to original ($(wc -c < "$OUT/.a.txt") bytes)"
    else
      echo "  $dst: extracted text DIFFERS:"
      diff "$OUT/.a.txt" "$OUT/.b.txt" | sed 's/^/      /' || true
    fi
  done

  echo
  echo "===== sha256 of inputs (fixtures) and outputs (all produced PDFs) ====="
  sha256sum "$ROOT"/corpus/raw/generated/chrome/fa-zwnj-lamalef.pdf \
            "$ROOT"/corpus/raw/synthetic/actualtext-fa.pdf \
            "$ROOT"/corpus/raw/synthetic/minimal-ltr.pdf \
            "$OUT"/*.pdf 2>/dev/null | sed "s|$ROOT/||g"
} | tee -a "$REPORT"

echo
echo "report: $REPORT"
