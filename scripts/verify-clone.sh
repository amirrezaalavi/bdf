#!/usr/bin/env bash
# verify-clone.sh — prove that a fresh clone of this repo is usable.
#
# WHY THIS EXISTS
# ---------------
# The public repo deliberately does NOT contain the 51 real-world corpus PDFs: they are customer
# documents (contracts, invoices, reports) and publishing them would be a legal and privacy
# problem. A clone therefore builds and tests fine, but has only the redistributable synthetic
# fixtures. Without this script the next person cannot tell "working" from "silently missing
# everything" — and this repo has shipped four controls that reported success while looking at
# nothing (docs/problems/0006, 0009, 0011, 0012).
#
# THE RULES THIS SCRIPT FOLLOWS (learned the hard way; see AGENTS.md)
# ------------------------------------------------------------------------
#  * Report the WORK DONE, not just the verdict: "checked N files" is auditable, "clean" is not.
#  * A check that may skip itself because its input is missing must FIRST prove the input was
#    supposed to be there. `exists() -> skip` is a hole that reports success.
#  * Never pipe a check's stderr to /dev/null: a NameError in an instrument whose result you are
#    about to read is the finding, not noise.
#  * Exit non-zero on a broken clone. A verifier that always exits 0 is decoration.
#
# Usage:  bash scripts/verify-clone.sh [--quick]
#           --quick   skip the test run (build + inventory only)
#
# Exit codes: 0 = clone is usable; 1 = clone is BROKEN; 2 = the script itself is broken.

set -uo pipefail   # deliberately NOT -e: see "self-abort" in AGENTS.md. We report, we do not die.

QUICK=0
[ "${1:-}" = "--quick" ] && QUICK=1

FAILURES=0
note_fail() { printf '  FAIL  %s\n' "$1"; FAILURES=$((FAILURES + 1)); }
note_ok()   { printf '  ok    %s\n' "$1"; }

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "== verify-clone: $ROOT"
echo "   git $(git rev-parse --short HEAD 2>/dev/null || echo '(not a git repo)') on branch $(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo '?')"

# ---------------------------------------------------------------- 1. repository shape
echo
echo "-- repository shape"
REQUIRED=(Cargo.toml AGENTS.md HANDOFF.md README.md rust-toolchain.toml deny.toml .gitignore
          scripts/wsl-build.sh .github/workflows/ci.yml corpus/manifest.json)
MISSING=0
for f in "${REQUIRED[@]}"; do
  if [ -e "$f" ]; then note_ok "$f"; else note_fail "$f is missing"; MISSING=$((MISSING + 1)); fi
done
if [ "$MISSING" -gt 0 ]; then
  echo "   -> $MISSING required file(s) missing: this is not a usable clone."
fi

# ---------------------------------------------------------------- 2. toolchain
echo
echo "-- toolchain"
if ! command -v rustc >/dev/null 2>&1; then
  note_fail "rustc not on PATH. Install rustup (https://rustup.rs); rust-toolchain.toml pins the exact version."
else
  PINNED="$(sed -n 's/^channel *= *"\(.*\)"/\1/p' rust-toolchain.toml 2>/dev/null | head -1)"
  ACTUAL="$(rustc --version 2>/dev/null | awk '{print $2}')"
  echo "   pinned: ${PINNED:-<none>}   in use: $ACTUAL"
  if [ -n "$PINNED" ]; then
    # A gate that does not run the compilers CI runs is not a gate (docs/problems/0003), so a
    # mismatch here is a named failure, not a warning.
    case "$ACTUAL" in
      "$PINNED"*) note_ok "toolchain matches the pin ($ACTUAL)" ;;
      *) note_fail "toolchain drift: repo pins $PINNED but this machine has $ACTUAL. CI will disagree with you." ;;
    esac
  else
    note_fail "rust-toolchain.toml has no channel pin"
  fi
fi

# ---------------------------------------------------------------- 3. build
echo
echo "-- build"
BUILD_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/target-bdf}"
if [ ! -d "$BUILD_TARGET_DIR" ]; then
  echo "   CARGO_TARGET_DIR=$BUILD_TARGET_DIR (does not exist yet; cargo will create it)"
fi
if command -v cargo >/dev/null 2>&1; then
  if CARGO_TARGET_DIR="$BUILD_TARGET_DIR" cargo build --workspace --locked > /tmp/verify-clone-build.log 2>&1; then
    note_ok "cargo build --workspace --locked"
  else
    note_fail "cargo build failed (tail of /tmp/verify-clone-build.log):"
    tail -n 15 /tmp/verify-clone-build.log | sed 's/^/         /'
  fi
else
  note_fail "cargo not on PATH"
fi

# ---------------------------------------------------------------- 4. tests
if [ "$QUICK" -eq 0 ]; then
  echo
  echo "-- tests"
  if command -v cargo >/dev/null 2>&1; then
    if CARGO_TARGET_DIR="$BUILD_TARGET_DIR" cargo test --workspace --locked > /tmp/verify-clone-test.log 2>&1; then
      SUMMARY="$(grep -cE '^test result: ok' /tmp/verify-clone-test.log 2>/dev/null)"
      note_ok "cargo test --workspace --locked ($SUMMARY suite(s) reported ok)"
    else
      note_fail "tests failed (tail of /tmp/verify-clone-test.log):"
      tail -n 20 /tmp/verify-clone-test.log | sed 's/^/         /'
    fi
  fi
fi

# ---------------------------------------------------------------- 5. corpus inventory
# This is the part a newcomer most needs and least expects: state EXACTLY what is here and what is
# legitimately absent. Do not report a healthy number without saying how many files it covered.
echo
echo "-- corpus inventory (public clone has fixtures only; the real archive is deliberately absent)"
PDF_TOTAL=0
if [ -d corpus ]; then
  PDF_TOTAL="$(find corpus -type f -name '*.pdf' 2>/dev/null | wc -l | tr -d ' ')"
else
  note_fail "corpus/ directory missing"
fi
echo "   pdf files committed: $PDF_TOTAL"
if [ "$PDF_TOTAL" -eq 0 ]; then
  note_fail "no pdf fixtures at all — tests that need fixtures will pass vacuously or fail"
else
  note_ok "$PDF_TOTAL redistributable fixture(s) present"
fi

if [ -d corpus/raw/private/desktop-pdfs ]; then
  PRIVATE="$(find corpus/raw/private/desktop-pdfs -type f -name '*.pdf' 2>/dev/null | wc -l | tr -d ' ')"
  echo "   private real-world archive: $PRIVATE file(s) (owner-only, gitignored)"
  if [ "$PRIVATE" -eq 0 ]; then
    note_fail "the private archive directory exists but contains no PDFs — either empty or wrong path"
  fi
else
  echo "   private real-world archive: ABSENT (expected on a clone — see README section 0)"
  echo "   -> real-archive validation numbers in the docs will NOT be reproducible here."
  echo "   -> 14 of the 51 archive files refuse with unsupported_visual_order; that path needs the archive."
fi

# The public manifest uses a `fixtures` key; tolerate the historical `files`/`rows` shape too.
# A parsed empty list is not success: the manifest is supposed to describe real fixture inputs.
if [ -f corpus/manifest.json ]; then
  echo "   manifest rows:"
  MANIFEST_RESULT="$(python3 - <<'PY'
import json, os, sys
try:
    m = json.load(open('corpus/manifest.json'))
except Exception as e:
    print(f"ERROR: cannot parse corpus/manifest.json: {e}")
    sys.exit(2)
rows = m if isinstance(m, list) else m.get('fixtures', m.get('files', m.get('rows', [])))
if not isinstance(rows, list) or not rows:
    print("ERROR: corpus manifest has no fixture rows")
    sys.exit(2)
present = missing = 0
for r in rows:
    p = r.get('path') or r.get('file') or ''
    if not p:
        missing += 1
        print(f"MISSING PATH: {r}")
    elif os.path.isfile(p):
        present += 1
    else:
        missing += 1
        print(f"MISSING FILE: {p}")
print(f"rows: {len(rows)}  present: {present}  missing: {missing}")
sys.exit(1 if missing else 0)
PY
)"; manifest_rc=$?
  printf '%s\n' "$MANIFEST_RESULT" | sed 's/^/     /'
  if [ "$manifest_rc" -eq 0 ]; then
    note_ok "manifest describes existing fixture files"
  else
    note_fail "corpus manifest is malformed, empty, or refers to missing fixtures"
  fi
fi

# ---------------------------------------------------------------- 6. the public-mirror guarantee
echo
echo "-- public-mirror guarantee (what must NOT be here)"
LEAKY="$(find . -path ./.git -prune -o -type f -name '*.pdf' -print 2>/dev/null | grep -c 'raw/private' || true)"
if [ "$LEAKY" -eq 0 ]; then
  note_ok "no pdf under corpus/raw/private is tracked (correct for a public clone)"
else
  note_fail "$LEAKY pdf(s) under corpus/raw/private are present in this checkout — if this is the PUBLIC repo, that is a leak"
fi

# ---------------------------------------------------------------- verdict
echo
if [ "$FAILURES" -eq 0 ]; then
  echo "== VERIFY-CLONE PASS =="
  echo "The clone builds and tests. The real-world archive is absent by design; see README section 0."
  exit 0
else
  echo "== VERIFY-CLONE FAIL: $FAILURES check(s) failed =="
  exit 1
fi