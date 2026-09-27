#!/usr/bin/env bash
# Wait for the CI checks of one commit to finish, then say whether they passed.
#
# Why: pushing successfully says nothing about whether the code passed. On
# 2026-09-27 three commits landed on main with red CI while every report said
# "gate green" (see docs/problems/0003). This is the script that closes that gap:
# publish-public.sh pushes a snapshot branch, waits here, and only then fast-forwards
# main.
#
# Usage: bash scripts/ci-wait.sh <sha> [max_seconds]
# Exit:  0 all checks succeeded · 4 at least one check failed · 5 timed out
set -uo pipefail

SHA="${1:?usage: ci-wait.sh <sha> [max_seconds]}"
MAX="${2:-900}"
REPO="amirrezaalavi/bdf"

CRED=$(grep -m1 'github.com' "$HOME/.git-credentials-yolka" 2>/dev/null || true)
TOKEN=$(printf '%s' "$CRED" | sed -n 's#.*://[^:]*:\([^@]*\)@.*#\1#p')
if [ -z "$TOKEN" ]; then
  echo "ci-wait: no GitHub credential in ~/.git-credentials-yolka" >&2
  exit 2
fi
PY=$(command -v python3 || command -v python)

deadline=$(( $(date +%s) + MAX ))
while [ "$(date +%s)" -lt "$deadline" ]; do
  line=$(curl -sS -H "Authorization: token $TOKEN" \
      "https://api.github.com/repos/$REPO/commits/$SHA/check-runs" 2>/dev/null \
    | "$PY" -c '
import sys, json
runs = json.load(sys.stdin).get("check_runs", [])
done = [r for r in runs if r["status"] == "completed"]
if not runs:
    print("WAIT|no check runs registered yet")
elif len(done) < len(runs):
    print("WAIT|%d/%d checks done" % (len(done), len(runs)))
else:
    bad = [r["name"] for r in done if r["conclusion"] != "success"]
    detail = " ".join("%s=%s" % (r["name"], r["conclusion"]) for r in done)
    print(("FAIL|" if bad else "PASS|") + detail)
' 2>/dev/null)

  case "${line%%|*}" in
    PASS) echo "ci-wait: PASS  (${line#*|})"; exit 0 ;;
    FAIL) echo "ci-wait: FAIL  (${line#*|})"; exit 4 ;;
    *)    echo "ci-wait: waiting — ${line#*|}" ;;
  esac
  sleep 20
done
echo "ci-wait: no conclusion for $SHA within ${MAX}s — inspect GitHub Actions" >&2
exit 5
