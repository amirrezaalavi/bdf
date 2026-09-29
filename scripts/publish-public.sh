#!/usr/bin/env bash
# Publish the current working tree to the public mirror repo (github.com/amirrezaalavi/bdf).
#
# Why this script exists: the local working copy holds real customer documents (corpus/raw/private)
# and reports that name them. That material must never reach a public remote, and "remember to
# exclude it" is not a control. This script is the control: it stages a copy, strips the private
# fixture rows out of the manifest, runs an explicit deny-list check against the ORIGINAL filenames,
# and refuses to publish if anything matches.
#
# Usage:  bash scripts/publish-public.sh ["commit message"] [--skip-preflight]
# Env:    MIRROR=<path to the public clone>   (default ~/playground/ai/bdf-snapshot)
#         WSL_DISTRO=<wsl distro>             (default Ubuntu-26.04)
# Exit:   2 not a mirror clone · 3 CI did not pass · 4 pre-flight gate failed
#
# Pre-flight (the hole that let a red snapshot reach CI, docs/problems/0003-era): after
# the staged tree is built and the deny-list passes, the CI gate is run INSIDE THE STAGED
# TREE — which has no corpus/raw/private — with the exact commands .github/workflows/ci.yml
# runs. That is a PUBLIC-CONDITIONS gate: it fails the same way the public mirror's CI
# would, before a single byte leaves this machine. A failure aborts here, naming the
# failing command; main and the remote are untouched.
#
# `--skip-preflight` opts out of that gate. Emergency use only (e.g. the WSL build
# environment itself is broken and a publish must go out): the snapshot branch is still
# pushed, CI still has to pass before main moves, but you have deliberately given up the
# local proof. Say so in the commit message when you use it.
#
# The mirror clone holds the public repo's git history; only its working tree is rebuilt here.
set -euo pipefail

SKIP_PREFLIGHT=0
_ARGS=()
for _arg in "$@"; do
  case "$_arg" in
    --skip-preflight) SKIP_PREFLIGHT=1 ;;
    *) _ARGS+=("$_arg") ;;
  esac
done
set -- ${_ARGS[@]+"${_ARGS[@]}"}

REPO="$(cd "$(dirname "$0")/.." && pwd)"
MIRROR="${MIRROR:-$HOME/playground/ai/bdf-snapshot}"
# native path for the Windows Python interpreter (MSYS paths are NOT translated for native programs)
MIRROR_NATIVE="$(cd "$MIRROR" && pwd -W 2>/dev/null || printf '%s' "$MIRROR")"
REPO_NATIVE="$(cd "$REPO" && pwd -W 2>/dev/null || printf '%s' "$REPO")"

if [ ! -d "$MIRROR/.git" ]; then
  echo "ERROR: $MIRROR is not a clone of the public repo. Clone it first:" >&2
  echo "  git clone https://github.com/amirrezaalavi/bdf.git $MIRROR" >&2
  exit 2
fi

PY="$REPO/.venv/Scripts/python.exe"
[ -x "$PY" ] || PY="$(command -v python || command -v python3)"

STAGE="$MIRROR/.stage"
rm -rf "$STAGE"; mkdir -p "$STAGE"
echo "== staging working tree (private corpus and reports excluded) =="
tar -cf - -C "$REPO" \
    --exclude='./.git' --exclude='./target' --exclude='./target-*' --exclude='./.venv' \
    --exclude='./corpus/raw/private' --exclude='./reports' \
    --exclude='*/__pycache__' --exclude='*.pyc' --exclude='*.bak-*' . \
  | tar -xf - -C "$STAGE"

STAGE_NATIVE="$STAGE"
"$PY" - "$REPO_NATIVE" "$STAGE_NATIVE" <<'PY'
import json, pathlib, re, sys
repo, stage = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])

# 1. private fixtures describe files that are not published: drop their manifest rows
mp = stage / "corpus/manifest.json"
if mp.exists():
    m = json.loads(mp.read_text(encoding="utf-8"))
    before = len(m["fixtures"])
    m["fixtures"] = [r for r in m["fixtures"] if not r["path"].startswith("corpus/raw/private/")]
    mp.write_text(json.dumps(m, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"manifest: {before} -> {len(m['fixtures'])} public rows")

# 2. deny-list = the real private identities, so the check cannot be talked around
deny = set()
rmp = repo / "reports/rename-map.json"
if rmp.exists():
    for move in json.loads(rmp.read_text(encoding="utf-8")).get("moves", []):
        for cand in (move.get("old", ""), pathlib.Path(move.get("old", "")).stem):
            if len(cand) >= 8 and not pathlib.Path(cand).stem.isdigit():
                deny.add(cand)
lmp = repo / "corpus/manifest.json"
if lmp.exists():
    for row in json.loads(lmp.read_text(encoding="utf-8"))["fixtures"]:
        for title in re.findall(r"title='([^']*)'", row.get("notes", "")):
            if len(title.strip()) >= 8:
                deny.add(title.strip())

hits = []
for path in stage.rglob("*"):
    if not path.is_file() or ".git" in path.parts:
        continue
    try:
        text = path.read_text(encoding="utf-8", errors="ignore")
    except OSError:
        continue
    hits += [(path.relative_to(stage).as_posix(), needle[:70]) for needle in deny if needle in text]

if hits:
    print("REFUSING TO PUBLISH - private material found:")
    for path, needle in hits[:20]:
        print(f"   {path}  <-  {needle!r}")
    sys.exit(1)
print(f"deny-list clean ({len(deny)} private identities checked)")
PY

echo "== pre-flight: CI gate on the staged snapshot =="
# The staged tree has no corpus/raw/private, so this is exactly what the public mirror
# runs: the CI commands, under PUBLIC conditions, on the bytes that are about to be
# pushed. Running it here is what stops a red snapshot from reaching CI at all.
WSL_DISTRO="${WSL_DISTRO:-Ubuntu-26.04}"
# MSYS/Windows path -> WSL path; wsl.exe does not translate its arguments.
to_wsl_path() {
  printf '%s' "$1" | sed -e 's#\\#/#g' -e 's#^/\([A-Za-z]\)/#/mnt/\1/#' -e 's#^\([A-Za-z]\):/#/mnt/\1/#'
}
if [ "$SKIP_PREFLIGHT" -eq 1 ]; then
  echo "== pre-flight SKIPPED by --skip-preflight: publishing WITHOUT the local public-conditions gate =="
else
  STAGE_WSL="$(to_wsl_path "$STAGE")"
  PRE_LOG="$(mktemp)"
  # Reuses the gate script (same commands as .github/workflows/ci.yml) against the
  # staged tree via PDFRTL_ROOT; --quick keeps it to fmt + clippy + test + slop + deps.
  # wsl-build.sh gives a foreign tree its OWN target dir on purpose: this checkout's
  # artifacts bake in this checkout's CARGO_MANIFEST_DIR, so reusing them would make the
  # control read corpus/raw/private again and the gate would no longer be public.
  if wsl -d "$WSL_DISTRO" -e bash -c \
      "PDFRTL_ROOT='$STAGE_WSL' bash '$STAGE_WSL/scripts/wsl-build.sh' --quick" \
      2>&1 | tee "$PRE_LOG"; then
    echo "== pre-flight green: the staged snapshot passes the CI gate under public conditions =="
    rm -f "$PRE_LOG"
  else
    step="$(grep -E '^-- ' "$PRE_LOG" | tail -1 | sed -e 's/^-- //' -e 's/ --$//')"
    {
      echo "PREFLIGHT FAILED - the staged snapshot is NOT publishable, nothing was pushed."
      echo "  failing command: ${step:-<unknown; full log below>}"
      echo "  This was a PUBLIC-CONDITIONS gate: it ran on the staged tree, which has NO"
      echo "  corpus/raw/private, using the exact commands from .github/workflows/ci.yml."
      echo "  main and the remote are untouched. Log tail:"
      tail -n 30 "$PRE_LOG"
    } >&2
    rm -f "$PRE_LOG"
    exit 4
  fi
fi

echo "== replacing mirror working tree =="
find "$MIRROR" -mindepth 1 -maxdepth 1 ! -name '.git' ! -name '.stage' -exec rm -rf {} +
cp -a "$STAGE/." "$MIRROR/"
rm -rf "$STAGE"

cd "$MIRROR"
git add -A
if git diff --cached --quiet; then
  echo "== public mirror already up to date, nothing to publish =="
  exit 0
fi
git commit -q -m "${1:-chore: sync public mirror}"
git log --oneline -1
git config credential.helper "store --file=$HOME/.git-credentials-yolka"
SHA=$(git rev-parse HEAD)

# main only ever receives commits whose CI is green: snapshot branch -> CI -> main.
# On 2026-09-27 three commits reached main with red CI while every local gate reported
# green (docs/problems/0003). This ordering makes that impossible rather than unlikely.
BRANCH="snapshot/$(date -u +%Y%m%dT%H%M%SZ)"
echo "== pushing $BRANCH (main stays untouched until CI passes) =="
git push -q origin "HEAD:refs/heads/$BRANCH" 2>&1 | sed 's#//[^@/]*@#//[REDACTED]@#g' | tail -2
if ! bash "$(dirname "$0")/ci-wait.sh" "$SHA"; then
  echo "== CI did not pass for $SHA: main is untouched, the snapshot stayed on $BRANCH =="
  exit 3
fi
echo "== CI green: fast-forwarding main =="
git push -q origin "HEAD:refs/heads/main" 2>&1 | sed 's#//[^@/]*@#//[REDACTED]@#g' | tail -2
git push -q origin --delete "$BRANCH" 2>/dev/null || true
echo "== published: origin/main == $SHA =="
