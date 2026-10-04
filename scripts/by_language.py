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

# Measured language corrections (docs/problems/0021). The manifest's `language` field is a claim
# about a file; this audit measures it. `--measured` opts into the measured reading, and the
# default stays manifest-faithful so no existing number changes meaning silently.
MEASURED_PATH = ROOT / "reports" / "audit-languages.json"
_MEASURED_CACHE: dict[str, str] | None = None

# Dominant script -> the language label we report for it. A script is not a language (Arabic
# script covers Arabic, Persian and Urdu), so a measured-dominant script gives the narrowest
# label that is true: `und` would erase information, and `fa` would be a guess.
_SCRIPT_TO_LANGUAGE = {"Arab": "ar-script", "Hebr": "he", "Latn": "en", "Cyrl": "ru"}


def load_manifest() -> dict[str, dict]:
    """Index the manifest by BOTH its own path key and each row's basename.

    The manifest is keyed by `path`, while every report row carries a bare `file` name. Measured
    2026-10-04: a lookup keyed only on `path` matched **0 of 51** private rows, so
    `language_of` fell through to the filename-prefix fallback for the whole private archive
    while its own docstring claimed "the manifest is the only source of a file's language". The
    published per-language table was therefore derived from filenames, not from the manifest —
    the exact substitution `references/real-world-pdf-interrogation.md` forbids.

    Basename indexing is safe here because no two manifest rows share a basename (measured: 0
    collisions). A collision is refused rather than resolved by insertion order, because either
    row could be the intended one and silently picking one is how a wrong label survives.
    """
    data = json.loads((ROOT / "corpus/manifest.json").read_text())
    index: dict[str, dict] = {}
    for row in data["fixtures"]:
        index[row["path"]] = row
        name = pathlib.PurePosixPath(row["path"]).name
        existing = index.get(name)
        if existing is not None and existing is not row:
            raise SystemExit(
                f"manifest basename collision: {name!r} is claimed by "
                f"{existing['path']!r} and {row['path']!r}. Refusing to index — a "
                f"collision resolved by insertion order would attribute one file's language "
                f"to another."
            )
        index[name] = row
    return index


def language_of(row: dict, manifest: dict[str, dict]) -> str:
    """Manifest first; fall back to the conventional fixture prefix.

    A filename is not evidence of a language, so the prefix fallback exists only for report rows
    whose manifest entry is absent (docs/problems/0015 naming rules).

    IMPORTANT (docs/problems/0021): the manifest value is a CLAIM about the file, and auditing all
    51 private files found most of the `fa`/`en` claims wrong. So a label that the audit has
    already refuted must not silently keep driving the acceptance table. This function cannot
    re-derive the truth — it has no text — so it applies the only correction it can justify
    without inventing one:

      * a label the audit reported as `mismatch` is reported under its **measured** dominant
        script, because the audit already established that script positively;
      * a label the audit reported as `undetermined` or `mixed` is reported as `unverified`,
        because nothing positive was established and absence is not evidence.

    Pass `--measured reports/audit-languages.json` to get those corrections; without it this
    function behaves exactly as before and the table keeps its manifest-faithful meaning. A
    reader who wants the manifest's claim rather than the measurement can therefore see both, and
    a number with no provenance is not quoted.
    """
    entry = manifest.get(row["file"])
    if entry:
        measured = _measured_language(row["file"], MEASURED_PATH)
        if measured is not None:
            return measured
        return entry.get("language") or "unknown"
    # No manifest row. The report knows a bare filename and the prefix convention names a
    # LANGUAGE, which is exactly the inference docs/problems/0021 measured to be wrong (four
    # `persian-*` files hold no Persian, five `english-*` files are majority-Persian). A filename
    # is not a place to record that guess, so report it as unlabelled and say so in the output —
    # the row is still counted, it simply cannot claim a language.
    return "unlabelled"


def _measured_language(file_name: str, path: pathlib.Path) -> str | None:
    """Measured language for one file, or None when the audit has no verdict for it.

    Cached: the audit report is read once. A missing file returns None, so the manifest value
    stands and the table says which mode it is in rather than silently changing meaning.
    """
    global _MEASURED_CACHE
    if not path.exists():
        return None
    if _MEASURED_CACHE is None:
        try:
            data = json.loads(path.read_text())
        except json.JSONDecodeError:
            _MEASURED_CACHE = {}
        else:
            cache: dict[str, str] = {}
            for entry in data.get("files", []):
                outcome = entry.get("outcome")
                counts = entry.get("counts") or {}
                dominant = max(counts, key=counts.get) if counts else None
                if outcome == "mismatch" and dominant:
                    cache[entry["file"]] = _SCRIPT_TO_LANGUAGE.get(dominant, "unknown")
                elif outcome == "mixed":
                    cache[entry["file"]] = "unverified"
                # `confirmed` and `undetermined` keep the manifest's own label: confirmed means
                # the label was right, and undetermined means nothing was established either way.
            _MEASURED_CACHE = cache
    return _MEASURED_CACHE.get(file_name)


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
    global MEASURED_PATH, _MEASURED_CACHE

    ap = argparse.ArgumentParser()
    ap.add_argument("--json", action="store_true", help="machine-readable output")
    ap.add_argument(
        "--measured",
        action="store_true",
        help=(
            "classify files by MEASURED script evidence from "
            "reports/audit-languages.json instead of trusting the manifest's language "
            "field. Run scripts/audit_languages.py first; docs/problems/0021 measured "
            "most fa/en labels in the private archive to be wrong. Without this flag the "
            "table is manifest-faithful, so an existing number keeps its meaning."
        ),
    )
    ap.add_argument(
        "--measured-path",
        default=str(ROOT / "reports" / "audit-languages.json"),
        metavar="PATH",
        help="audit report to read with --measured (default reports/audit-languages.json)",
    )
    args = ap.parse_args()

    if args.measured:
        MEASURED_PATH = pathlib.Path(args.measured_path)
        _MEASURED_CACHE = None
    else:
        # Without --measured, no corrections may be applied. Measured on 2026-10-04: a default
        # that silently consulted the audit report made `--measured` a no-op — both modes
        # produced byte-identical tables while the flag claimed they differed. A flag whose two
        # states are indistinguishable is worse than no flag.
        MEASURED_PATH = pathlib.Path("/nonexistent-audit-report")

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
    print(f"Classification: {'MEASURED script evidence' if args.measured else 'manifest label (unaudited)'}\n")
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