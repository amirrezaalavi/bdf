#!/usr/bin/env bash
set -euo pipefail

# Wrapper script for triage_pdf_sample.py
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

python3 "${SCRIPT_DIR}/triage_pdf_sample.py" "$@"
