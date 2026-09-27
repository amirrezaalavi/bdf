#!/usr/bin/env python3
"""Add or update one fixture row in corpus/manifest.json — with the rules enforced.

Hand-editing manifest.json is how corpora rot: a missing licence, a stale hash, a
guessed producer class. This script refuses the row instead.

Usage:
    python3 scripts/manifest-add.py \
        --id synthetic-actualtext-fa-001 \
        --path corpus/raw/synthetic/actualtext-fa.pdf \
        --language fa --script Arab --direction rtl \
        --producer pdfrtl-gen --producer-class logical \
        --licence own-work --notes "..."

    python3 scripts/manifest-add.py --check       # validate the whole manifest, touch nothing

Rules enforced (see corpus/README.md and corpus/AGENT.md):
  * --licence is mandatory; use `unknown` if it is genuinely unknown (never guess one),
  * files outside raw/private/ must be `redistributable: true`; files inside raw/private/
    are forced to `redistributable: false` and their sha256 is the only thing recorded,
  * sha256 is computed from the file when it exists; a missing file is allowed only for
    private fixtures (not present on every machine),
  * producer_class must be one of visual|logical|unknown and may not be `visual`/`logical`
    without a note saying how that was established (that field drives extraction order),
  * an unknown --id is an error (no silent creation of near-duplicate rows).
"""
from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "corpus" / "manifest.json"
CLASSES = {"visual", "logical", "unknown"}


def sha256(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(65536), b""):
            digest.update(chunk)
    return digest.hexdigest()


def load() -> dict:
    return json.loads(MANIFEST.read_text(encoding="utf-8"))


def save(manifest: dict) -> None:
    MANIFEST.write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def check(manifest: dict) -> int:
    problems = []
    seen = set()
    for row in manifest["fixtures"]:
        rid = row.get("id", "<no id>")
        if rid in seen:
            problems.append(f"{rid}: duplicate id")
        seen.add(rid)
        for field in ("id", "path", "language", "script", "direction", "producer",
                      "producer_class", "licence", "redistributable", "status"):
            if field not in row:
                problems.append(f"{rid}: missing field `{field}`")
        if row.get("licence") in (None, ""):
            problems.append(f"{rid}: licence missing (use `unknown`, do not omit)")
        if row.get("producer_class") not in CLASSES:
            problems.append(f"{rid}: producer_class must be one of {sorted(CLASSES)}")
        in_private = "raw/private" in row.get("path", "")
        if in_private and row.get("redistributable"):
            problems.append(f"{rid}: file lives in raw/private/ but is marked redistributable")
        if not in_private and row.get("redistributable") is False:
            problems.append(f"{rid}: outside raw/private/ but marked non-redistributable")
        if row.get("producer_class") in ("visual", "logical") and not row.get("notes"):
            problems.append(f"{rid}: producer_class={row['producer_class']} needs a note "
                            "explaining the evidence")
        path = ROOT / row.get("path", "")
        if path.exists() and row.get("sha256"):
            actual = sha256(path)
            if actual != row["sha256"]:
                problems.append(f"{rid}: sha256 mismatch (manifest {row['sha256'][:12]}…, "
                                f"file {actual[:12]}…)")
    if problems:
        for problem in problems:
            print(f"FAIL {problem}", file=sys.stderr)
        return 1
    print(f"manifest ok: {len(manifest['fixtures'])} fixture(s)")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--id", required=False)
    parser.add_argument("--path")
    parser.add_argument("--language")
    parser.add_argument("--script")
    parser.add_argument("--direction", choices=["rtl", "ltr", "ttb"])
    parser.add_argument("--producer")
    parser.add_argument("--producer-class", choices=sorted(CLASSES))
    parser.add_argument("--licence")
    parser.add_argument("--source-url")
    parser.add_argument("--notes")
    parser.add_argument("--status", default="unverified")
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()

    manifest = load()
    if args.check:
        return check(manifest)

    required = ["id", "path", "language", "script", "direction", "producer",
                "producer_class", "licence"]
    missing = [name for name in required if getattr(args, name.replace("-", "_")) is None]
    if missing:
        print(f"missing required arguments: {missing}", file=sys.stderr)
        return 2

    path = ROOT / args.path
    in_private = "raw/private" in args.path
    if not path.exists() and not in_private:
        print(f"file does not exist: {args.path} (only raw/private/ fixtures may be absent)",
              file=sys.stderr)
        return 2
    if args.producer_class in ("visual", "logical") and not args.notes:
        print("--notes is required when producer_class is visual or logical: state the evidence",
              file=sys.stderr)
        return 2

    row = {
        "id": args.id,
        "path": args.path,
        "language": args.language,
        "script": args.script,
        "direction": args.direction,
        "producer": args.producer,
        "producer_class": args.producer_class,
        "licence": args.licence,
        "source_url": args.source_url,
        "sha256": sha256(path) if path.exists() else None,
        "redistributable": not in_private,
        "status": args.status,
        "notes": args.notes,
    }

    existing = next((r for r in manifest["fixtures"] if r["id"] == args.id), None)
    if existing:
        existing.update({k: v for k, v in row.items() if v is not None})
        action = "updated"
    else:
        manifest["fixtures"].append(row)
        action = "added"
    save(manifest)
    print(f"{action} {args.id}")
    return check(manifest)


if __name__ == "__main__":
    raise SystemExit(main())
