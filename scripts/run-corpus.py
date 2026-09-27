#!/usr/bin/env python3
"""Corpus harness: run every fixture through the CLI and write a per-fixture report.

Why Python and not bash+jq: the report needs real JSON parsing of the CLI envelope, and
`jq` is not guaranteed on any machine we run on (it is absent on the Windows host today).
Python 3 is guaranteed on both sides (`python` on Windows, `python3` in WSL) and this
script uses only the standard library.

Usage (from repo root):
    python3 scripts/run-corpus.py                       # use target/debug/pdfrtl
    python3 scripts/run-corpus.py --binary /path/to/pdfrtl
    python3 scripts/run-corpus.py --only fa- --only ar- # subset by id prefix
    python3 scripts/run-corpus.py --update-hashes       # fill sha256 for synthetic/generated
Exit code: 0 if every fixture behaved as recorded, 1 otherwise (so CI can gate on it).

What it checks at P0 (inspect level):
  * the file exists and its sha256 matches the manifest (synthetic/generated only),
  * `inspect --json` exits 0 and its envelope parses,
  * pages > 0,
  * the producer the manifest claims matches what the file actually says (when recorded),
plus, once `expected/<id>.json` carries a `text` field (P1), extraction equality.
A fixture that is expected to be unsupported must exit 3 with an `unsupported_*` reason —
that counts as a PASS, because refusing is the correct behaviour.
"""
from __future__ import annotations

import argparse
import dataclasses
import datetime as dt
import hashlib
import json
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "corpus" / "manifest.json"
EXPECTED = ROOT / "corpus" / "expected"
REPORTS = ROOT / "reports"
HASH_SOURCES = ("synthetic", "generated")


@dataclasses.dataclass
class Result:
    fixture_id: str
    path: str
    status: str  # pass | fail | skip
    detail: str
    reason: str = ""
    exit_code: int | None = None


def sha256(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(65536), b""):
            digest.update(chunk)
    return digest.hexdigest()


def load_manifest() -> dict:
    if not MANIFEST.exists():
        return {"fixtures": []}
    return json.loads(MANIFEST.read_text(encoding="utf-8"))


def save_manifest(manifest: dict) -> None:
    MANIFEST.write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def update_hashes(manifest: dict) -> int:
    changed = 0
    for row in manifest["fixtures"]:
        path = ROOT / row["path"]
        if not path.exists():
            continue
        if not any(part in HASH_SOURCES for part in path.parts):
            continue  # only deterministic, self-generated files get auto-hashes
        digest = sha256(path)
        if row.get("sha256") != digest:
            row["sha256"] = digest
            changed += 1
    if changed:
        save_manifest(manifest)
    print(f"hashes updated: {changed}")
    return 0


def run_fixture(binary: str, row: dict, expected: dict | None) -> Result:
    fixture_id = row["id"]
    path = ROOT / row["path"]

    if not path.exists():
        if row.get("redistributable", True):
            return Result(fixture_id, row["path"], "fail", "file missing but marked redistributable")
        return Result(fixture_id, row["path"], "skip", "private fixture not present on this machine")

    if row.get("sha256") and any(part in HASH_SOURCES for part in pathlib.Path(row["path"]).parts):
        actual = sha256(path)
        if actual != row["sha256"]:
            return Result(
                fixture_id, row["path"], "fail",
                f"sha256 mismatch: manifest {row['sha256'][:12]}… != file {actual[:12]}…",
            )

    proc = subprocess.run(
        [binary, "--json", "inspect", str(path)],
        capture_output=True, text=True, cwd=ROOT,
    )
    try:
        envelope = json.loads(proc.stdout)
    except json.JSONDecodeError:
        return Result(fixture_id, row["path"], "fail",
                      f"stdout was not a JSON envelope: {proc.stdout[:120]!r}",
                      exit_code=proc.returncode)

    reasons = envelope.get("reasons", [])
    reason = ",".join(reasons)
    expect_unsupported = bool((expected or {}).get("expect_unsupported"))

    if expect_unsupported:
        if proc.returncode == 3 and any(r.startswith("unsupported_") for r in reasons):
            return Result(fixture_id, row["path"], "pass",
                          "refused to guess, as required", reason, proc.returncode)
        return Result(fixture_id, row["path"], "fail",
                      f"expected unsupported/exit 3, got exit {proc.returncode} reasons={reasons}",
                      reason, proc.returncode)

    if proc.returncode != 0 or not envelope.get("ok"):
        return Result(fixture_id, row["path"], "fail",
                      f"inspect failed: {envelope.get('data', {}).get('error', '')}",
                      reason, proc.returncode)

    pages = envelope.get("data", {}).get("pages")
    if not pages:
        return Result(fixture_id, row["path"], "fail", "pages == 0", reason, proc.returncode)

    claimed = row.get("producer")
    actual = envelope["data"].get("producer")
    if claimed and claimed not in ("unknown",) and actual != claimed:
        return Result(fixture_id, row["path"], "fail",
                      f"producer mismatch: manifest {claimed!r} vs file {actual!r}",
                      reason, proc.returncode)

    if expected and expected.get("text") is not None:
        # Extraction equality. Guarded by `verified_by`: expected text is ground truth only
        # once a human has verified it (corpus/AGENT.md rule 5). Until then the report says
        # so out loud instead of silently asserting an agent's guess.
        if not expected.get("verified_by"):
            return Result(fixture_id, row["path"], "skip",
                          "expected text present but unverified by a human — not asserted",
                          reason, proc.returncode)
        proc_ex = subprocess.run([binary, "--json", "extract", str(path)],
                                 capture_output=True, text=True, cwd=ROOT)
        if proc_ex.returncode != 0:
            return Result(fixture_id, row["path"], "fail",
                          f"extract failed (exit {proc_ex.returncode})", reason, proc_ex.returncode)
        got = json.loads(proc_ex.stdout)["data"].get("text", "")
        if got != expected["text"]:
            return Result(fixture_id, row["path"], "fail",
                          "extracted text != expected text", reason, proc_ex.returncode)
        return Result(fixture_id, row["path"], "pass", "extraction matches expected text",
                      reason, proc_ex.returncode)

    return Result(fixture_id, row["path"], "pass", f"inspect ok ({pages} page(s))",
                  reason, proc.returncode)


def write_report(results: list[Result], binary: str) -> pathlib.Path:
    REPORTS.mkdir(exist_ok=True)
    stamp = dt.datetime.now(dt.timezone.utc).strftime("%Y-%m-%dT%H%M%SZ")
    out = REPORTS / f"{stamp}-corpus.md"
    counts = {status: sum(1 for r in results if r.status == status)
              for status in ("pass", "fail", "skip")}
    lines = [
        f"# Corpus report — {stamp}",
        "",
        f"binary: `{binary}`",
        f"fixtures: {len(results)} · pass {counts['pass']} · fail {counts['fail']} · skip {counts['skip']}",
        "",
        "| fixture | status | exit | reason | detail |",
        "|---|---|---|---|---|",
    ]
    for r in results:
        lines.append(f"| `{r.fixture_id}` | {r.status} | {r.exit_code if r.exit_code is not None else '-'} "
                     f"| {r.reason or '-'} | {r.detail} |")
    lines += [
        "",
        "Legend: `skip` = private fixture not present locally (still counted, never silent).",
        "A fixture expected to be unsupported passes only when it returns an "
        "`unsupported_*` reason with exit 3 — refusing to guess is a correct outcome.",
        "",
        "Verification tier: P0 = inspect-level (`pages`, `producer`, sha256) for every fixture; "
        "extraction equality starts at P1 and appears as `extraction matches expected text`.",
        "",
    ]
    out.write_text("\n".join(lines), encoding="utf-8")
    return out


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", default=str(ROOT / "target" / "debug" / "pdfrtl"))
    parser.add_argument("--only", action="append", default=[])
    parser.add_argument("--update-hashes", action="store_true")
    args = parser.parse_args()

    manifest = load_manifest()
    if args.update_hashes:
        return update_hashes(manifest)

    results = []
    for row in manifest["fixtures"]:
        if args.only and not any(row["id"].startswith(prefix) for prefix in args.only):
            continue
        expected_path = EXPECTED / f"{row['id']}.json"
        expected = json.loads(expected_path.read_text(encoding="utf-8")) if expected_path.exists() else None
        results.append(run_fixture(args.binary, row, expected))

    if not results:
        print("no fixtures selected — is corpus/manifest.json populated?")
        return 1

    report = write_report(results, args.binary)
    failed = [r for r in results if r.status == "fail"]
    for r in results:
        print(f"{r.status:>4}  {r.fixture_id:<40} {r.reason or '-'}  {r.detail}")
    print(f"report: {report}")
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
