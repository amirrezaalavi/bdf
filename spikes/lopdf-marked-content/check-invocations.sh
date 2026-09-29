#!/usr/bin/env bash
# Cross-invocation reproducibility check for this spike.
#
# Runs run.sh twice, sequentially, in this one shell, and compares the sha256 of
# out/report.txt BETWEEN invocations. Every input is verified to exist, be non-empty and
# carry the sections FINDINGS.md cites BEFORE any hash is compared; otherwise the check
# prints REFUSED and exits non-zero. A control that cannot read its input must not pass.
#
#   wsl.exe -d Ubuntu-26.04 -- bash /mnt/c/Users/netcon/playground/ai/pdfrtl/spikes/lopdf-marked-content/check-invocations.sh
set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
cd "$HERE" || { echo "REFUSED: cannot cd to $HERE"; exit 2; }

get_sha() { # $1 = label, $2 = file to write the bare hash into
  local f=out/report.txt
  if [ ! -f "$f" ]; then echo "REFUSED($1): $f does not exist"; return 1; fi
  local sz marks h
  sz=$(wc -c < "$f" | tr -d ' ')
  if [ -z "$sz" ] || [ "$sz" -eq 0 ]; then echo "REFUSED($1): $f is empty"; return 1; fi
  marks=$(grep -c -E "^== E1:|^===== ORACLE: qpdf --check" "$f" || true)
  if [ "$marks" -lt 2 ]; then
    echo "REFUSED($1): only $marks of the 2 expected sections found in $f - report not read correctly"
    return 1
  fi
  h=$(sha256sum "$f" | awk '{print $1}')
  if [ -z "$h" ]; then echo "REFUSED($1): sha256sum returned nothing"; return 1; fi
  echo "report[$1] size=${sz}B expected_sections=${marks}/2 sha256=${h}"
  printf '%s\n' "$h" > "$2"
  return 0
}

echo "=== invocation A ==="
bash run.sh > /tmp/invA.log 2>&1; echo "run.sh exit=$?"
get_sha A /tmp/shaA || { echo "RESULT: REFUSED (invocation A input unreadable)"; exit 1; }

sleep 1

echo "=== invocation B ==="
bash run.sh > /tmp/invB.log 2>&1; echo "run.sh exit=$?"
get_sha B /tmp/shaB || { echo "RESULT: REFUSED (invocation B input unreadable)"; exit 1; }

A=$(cat /tmp/shaA 2>/dev/null || true)
B=$(cat /tmp/shaB 2>/dev/null || true)
echo "A=${A:-(empty)} B=${B:-(empty)}"

if [ -n "$A" ] && [ -n "$B" ] && [ "$A" = "$B" ]; then
  echo "RESULT: STABLE ACROSS INVOCATIONS (both hashes non-empty and equal)"
else
  echo "RESULT: UNSTABLE OR UNVERIFIED (A non-empty=$([ -n "$A" ] && echo yes || echo no); B non-empty=$([ -n "$B" ] && echo yes || echo no); equal=$([ "$A" = "$B" ] && echo yes || echo no))"
  exit 1
fi

echo
echo "=== run.sh's own inner determinism gate (two runs inside one invocation) ==="
awk '/ORACLE: determinism/{f=1} f&&/IDENTICAL|DIFFERS/{print; c++} c==1{exit}' out/report.txt
echo "grep -c \"IDENTICAL across two runs\": $(grep -c 'IDENTICAL across two runs' out/report.txt)"
