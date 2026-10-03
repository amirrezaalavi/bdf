#!/usr/bin/env python3
"""Derive the per-language acceptance table from measured output. No hand-typed numbers.

Usage:  python3 scripts/by_language.py [--json]

Prints, per language: file counts by outcome, the median per-file emitted fraction, and which
RUNG of the order ladder actually decided each file. Writes nothing; the docs quote this output.

Rule (docs/problems/0016): a headline number in a reader-facing document is produced by a script,
not typed. "The total adds up" is not a check on a breakdown.
"""
from __future__ import annotations

import argparse
import collections
import json
import pathlib
import statistics
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent


def load_manifest() -> dict[str, dict]:
    """path -> row. The manifest is the only source of a file's language."""
    data = json.loads((ROOT / "corpus/manifest.json").read_text())
    return {r["path"]: r for r in data["fixtures"]}


def language_of(row: dict, manifest: dict[str, dict]) -> str:
    """Manifest first; fall back to the conventional fixture prefix.

    A filename is not evidence of a language, so the prefix fallback exists only for report rows
    whose manifest entry is absent (docs/problems/0015 naming rules).
    """
    entry = manifest.get(row["file"])
    if entry:
        return entry.get("language") or "unknown"
    base = pathlib.Path(row["file"]).name
    for prefix in ("persian", "arabic", "hebrew", "english", "he-eng"):
        if base.startswith(prefix):
            return prefix
    return "unknown"


def outcome(row: dict) -> str:
    """Disjoint classification. The four sum to the file count (problems/0016)."""
    emitted = row.get("chars", 0)
    withheld = row.get("unordered_chars", 0)
    if emitted + withheld == 0:
        return "no text layer"
    if withheld == 0:
        return "fully emitted"
    if emitted == 0:
        return "refused entirely"
    return "partial"


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--json", action="store_true", help="machine-readable output")
    args = ap.parse_args()

    report_path = ROOT / "reports/validate-archive.json"
    if not report_path.exists():
        print(
            "reports/validate-archive.json absent.\n"
            "Run the archive validation on the machine that holds the private corpus:\n"
            "    bash scripts/validate-archive.sh\n"
            "Its numbers are owner-local; a public clone cannot reproduce them.",
            file=sys.stderr,
        )
        return 2

    manifest = load_manifest()
    files = json.loads(report_path.read_text())["files"]

    buckets: dict[str, list[tuple[dict, str]]] = collections.defaultdict(list)
    for row in files:
        buckets[language_of(row, manifest)].append((row, outcome(row)))

    result = {}
    for lang in sorted(buckets):
        entries = buckets[lang]
        counts = collections.Counter(o for _, o in entries)
        fractions = [
            r["chars"] / (r["chars"] + r["unordered_chars"])
            for r, _ in entries
            if r["chars"] + r["unordered_chars"] > 0
        ]
        rules = collections.Counter(
            (r.get("order_rule") or "none").split(",")[0] for r, _ in entries
        )
        result[lang] = {
            "files": len(entries),
            "fully_emitted": counts["fully emitted"],
            "partial": counts["partial"],
            "refused_entirely": counts["refused entirely"],
            "no_text_layer": counts["no text layer"],
            "median_emitted_fraction": round(statistics.median(fractions), 4) if fractions else None,
            "top_deciding_rule": rules.most_common(1)[0][0] if rules else None,
        }

    if args.json:
        print(json.dumps(result, indent=2, sort_keys=True))
        return 0

    print("Extraction outcome by language (measured; see docs/ACCEPTANCE.md)\n")
    header = f"{'language':<10}{'files':>6}{'full':>6}{'part':>6}{'refuse':>8}{'no text':>9}{'median':>9}"
    print(header)
    print("-" * len(header))
    for lang, v in result.items():
        median = (
            f"{v['median_emitted_fraction']:.0%}"
            if v["median_emitted_fraction"] is not None
            else "—"
        )
        print(
            f"{lang:<10}{v['files']:>6}{v['fully_emitted']:>6}{v['partial']:>6}"
            f"{v['refused_entirely']:>8}{v['no_text_layer']:>9}{median:>9}"
        )

    print("\nWhich rung decided the text (Persian is the focus):\n")
    for lang in sorted(result):
        entries = buckets[lang]
        rules = collections.Counter(r.get("order_rule") or "none" for r, _ in entries)
        print(f"  {lang}:")
        for rule, n in rules.most_common():
            print(f"      {rule:<58} {n:>3}")
    print("\nTotal files:", sum(v["files"] for v in result.values()))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())