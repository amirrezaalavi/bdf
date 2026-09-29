#!/usr/bin/env python3
"""Enrich manifest rows with facts from the real-object probe (`tools/pdf_probe.py`).

The byte-level triage that produced the existing rows lies about the things that matter most:
on `arabic-1.pdf` it reported zero text operators and 97,811 `/ToUnicode` references, where a
real parser finds 23,115 operators across 91 pages and six properly mapped fonts. So the
manifest's *structure* facts are refreshed from the probe, and the triage numbers stay behind
as the weaker evidence they are.

Language policy, in priority order:

1. **A human-named file wins.** `arabic-*.pdf`, `hebrew-*.pdf` and `he-eng-ar.pdf` were named by
   the person who knows what they contain; a probe that disagrees is recorded as a
   discrepancy, not used to overrule the name.
2. Otherwise the probe's evidence decides: scripts in the extracted page text and in the
   document's own metadata strings.
3. Anything the probe cannot determine keeps whatever the row already had.

Usage:
    python scripts/manifest-apply-probe.py --probe reports/probe-new-rtl.json
"""
from __future__ import annotations

import argparse
import json
import pathlib
import re

ROOT = pathlib.Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "corpus" / "manifest.json"

HUMAN_NAMED = re.compile(r"^(arabic|hebrew|persian|english)-\d+\.pdf$|^he-eng-ar\.pdf$", re.IGNORECASE)
NAME_LANGUAGE = [
    (re.compile(r"^arabic", re.I), ("ar", "rtl")),
    (re.compile(r"^hebrew", re.I), ("he", "rtl")),
    (re.compile(r"^persian", re.I), ("fa", "rtl")),
    (re.compile(r"^english", re.I), ("en", "ltr")),
    (re.compile(r"^he-eng-ar", re.I), ("he", "rtl")),
]
SCRIPT_LANGUAGE = {"ar": ("fa", "rtl"), "he": ("he", "rtl"), "latin": ("en", "ltr")}

# What script the filename claims. Note "arabic" and "persian" both mean the Arabic script —
# the script cannot distinguish them, only a reader can, so it is never a conflict.
NAME_SCRIPT = [
    (re.compile(r"^arabic", re.I), "ar"),
    (re.compile(r"^persian", re.I), "ar"),
    (re.compile(r"^hebrew", re.I), "he"),
    (re.compile(r"^english", re.I), "latin"),
    (re.compile(r"^he-eng-ar", re.I), "he"),
]


def basename(path: str) -> str:
    """pypdf echoes back the platform's path separator; the manifest stores POSIX paths."""
    return path.replace("\\", "/").rsplit("/", 1)[-1]


def language_from_scripts(scripts: list[str]) -> tuple[str, str] | None:
    for script in ("ar", "he"):
        if script in scripts:
            return SCRIPT_LANGUAGE[script]
    if "latin" in scripts:
        return SCRIPT_LANGUAGE["latin"]
    return None


def meta_value(entry: dict, key: str) -> str:
    value = entry.get("metadata", {}).get(key)
    return value.get("value", "") if isinstance(value, dict) else ""


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--probe", required=True)
    args = parser.parse_args()

    probe = json.loads((ROOT / args.probe).read_text(encoding="utf-8"))
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    by_name = {basename(r["path"]): r for r in manifest["fixtures"]}

    updated = missing = 0
    for entry in probe["files"]:
        name = basename(entry["path"])
        row = by_name.get(name)
        if row is None or "error" in entry:
            print(f"no row for {name}" if row is None else f"probe error for {name}")
            missing += 1
            continue

        scripts = list(entry.get("page_text_scripts") or [])
        for key, value in entry.get("metadata", {}).items():
            if isinstance(value, dict):
                for script in value.get("scripts") or []:
                    if script not in scripts:
                        scripts.append(script)

        discrepancy = ""
        if HUMAN_NAMED.match(name):
            for pattern, (language, direction) in NAME_LANGUAGE:
                if pattern.match(name):
                    row["language"], row["direction"] = language, direction
                    break
            probe_scripts = set(scripts)
            for pattern, script in NAME_SCRIPT:
                if not pattern.match(name):
                    continue
                if script == "he" and "he" not in probe_scripts:
                    discrepancy = (f" DISCREPANCY: filename says Hebrew, probe found "
                                   f"{'/'.join(scripts) or 'no text'} — human reading required.")
                elif script == "ar" and "ar" not in probe_scripts:
                    discrepancy = (f" DISCREPANCY: filename says Arabic script, probe found "
                                   f"{'/'.join(scripts) or 'no text'} — human reading required.")
                elif script == "latin" and (probe_scripts & {"ar", "he"}):
                    discrepancy = (f" DISCREPANCY: filename says English, probe found RTL text "
                                   f"({'/'.join(scripts)}) — human reading required.")
                break
        else:
            probe_language = language_from_scripts(scripts)
            if probe_language:
                row["language"], row["direction"] = probe_language

        row["pages"] = entry.get("pages") or row.get("pages")
        row["producer"] = meta_value(entry, "producer") or row.get("producer")
        row["producer_tool"] = meta_value(entry, "creator") or row.get("producer_tool")
        row["status"] = "probed"
        # Rebuild from the byte-triage base instead of appending: appending made the notes grow
        # on every run and left a stale discrepancy from a previous pass looking current.
        base_notes = row["notes"].split(" Verified by pypdf:")[0]
        row["notes"] = (
            f"Real-world archive file, provenance unresolved (licence unknown — see "
            f"docs/OPEN-QUESTIONS.md Q-005). Verified by pypdf: pages={entry.get('pages')}, "
            f"fonts={entry.get('fonts_total')} (with /ToUnicode={entry.get('tounicode_fonts')}), "
            f"text_ops(first pages)={entry.get('text_ops')}, text_chars={entry.get('text_chars')}, "
            f"encrypted={entry.get('encrypted')}, scripts={'/'.join(scripts) or 'none'}, "
            f"title={meta_value(entry, 'title')!r}. "
            f"Byte-triage numbers (unreliable on these files, kept for the record): {base_notes}"
            f"{discrepancy}"
        )
        updated += 1

    MANIFEST.write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"enriched {updated} row(s), {missing} skipped")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
