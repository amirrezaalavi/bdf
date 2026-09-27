#!/usr/bin/env python3
"""Rename corpus files to a language-tagged, ASCII, test-friendly convention.

Why: Persian filenames cannot be slugified into manifest ids, are painful to type in a
shell, and different tools normalise them differently (NFC/NFD/ZWNJ), so they fight every
test harness. The convention this enforces mirrors the files that were already named that
way by hand (`arabic-1.pdf`, `hebrew-3.pdf`, `he-eng-ar.pdf`):

    <language>-<n>.pdf            name carried no usable ASCII information
    <language>-<ascii-slug>.pdf   name was descriptive ASCII; meaning is preserved

Language comes from the file's *content* where the content can say anything — the Unicode
targets in `/ActualText` and `ToUnicode` — and falls back to the filename script only for
files with no text layer at all. The evidence used is recorded per file, so a wrong guess is
visible instead of silent.

Dry run by default. `--apply` renames and writes a reversible map to reports/rename-map.json.

Usage:
    python tools/rename-corpus.py corpus/raw/private/desktop-pdfs          # show the plan
    python tools/rename-corpus.py corpus/raw/private/desktop-pdfs --apply  # do it
"""
from __future__ import annotations

import argparse
import json
import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from triage_pdfs import triage  # noqa: E402  (same-directory tool, reused not duplicated)

ARABIC = re.compile(r"[\u0600-\u06FF\u0750-\u077F]")
HEBREW = re.compile(r"[\u0590-\u05FF]")

# Labels follow the hand-made files already in the corpus (arabic-1.pdf, hebrew-1.pdf).
LABELS = {"fa": "persian", "en": "english", "ar": "arabic", "he": "hebrew"}

# Files already named in this convention are left alone, which is what makes re-running safe.
CONFORMING = re.compile(r"^(persian|english|arabic|hebrew|mixed|unknown|he-eng-ar)", re.IGNORECASE)


def slugify(stem: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", stem.lower()).strip("-")


def classify(path: pathlib.Path, facts: dict) -> tuple[str, str]:
    """Return (label, evidence). Only *positive* RTL evidence decides a label.

    Hard-won rule: "no Arabic codepoints found" is not evidence that a file is English. The
    reliable places to find Persian (ToUnicode, /ActualText) are compressed and often absent,
    and the one oracle available (poppler) drops Arabic-script text outright — so the first
    draft of this function labelled a Persian contract "english". The ladder below therefore
    requires a positive signal to call a file Persian/Hebrew, and falls back to the filename
    before it will call anything English.
    """
    content = set(facts.get("scripts") or [])
    meta = set(facts.get("metadata_scripts") or [])
    rtl = {s for s in (content | meta) if s in ("ar", "he")}
    if rtl:
        # No "-english" suffix even when Latin text is present: nearly every Persian document
        # contains Latin (numbers, acronyms, emails), so the suffix told the reader nothing.
        # The script mix still travels in the manifest's `scripts` field.
        if rtl == {"ar"}:
            return LABELS["fa"], "content" if "ar" in content else "metadata"
        if rtl == {"he"}:
            return LABELS["he"], "content" if "he" in content else "metadata"
        return "persian-hebrew", "metadata"

    stem = path.stem
    if ARABIC.search(stem):
        return LABELS["fa"], "filename-script"
    if HEBREW.search(stem):
        return LABELS["he"], "filename-script"
    # A time-tested filename convention in this estate: `-fa` / `fa-` marks Persian content.
    if re.search(r"(^|[-_])fa([-_]|$)", stem, re.IGNORECASE):
        return LABELS["fa"], "filename-marker"
    if re.search(r"(^|[-_])en([-_]|$)", stem, re.IGNORECASE):
        return LABELS["en"], "filename-marker"
    if facts.get("has_text_layer"):
        return LABELS["en"], "assumed(latin text only)"
    return "unknown", "undetermined(no text layer)"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("directory")
    parser.add_argument("--apply", action="store_true", help="actually rename (default: dry run)")
    parser.add_argument("--map-out", default="reports/rename-map.json")
    args = parser.parse_args()

    root = pathlib.Path(args.directory)
    files = sorted(p for p in root.glob("*.pdf") if p.is_file())
    plan: list[dict] = []
    used: set[str] = {p.name for p in files if CONFORMING.match(p.stem)}

    for path in files:
        if CONFORMING.match(path.stem):
            plan.append({"old": path.name, "new": path.name, "label": "-", "evidence": "already conforming",
                         "pages": None, "skipped": True})
            continue
        try:
            facts = triage(path)
        except Exception as exc:  # noqa: BLE001 - a broken file must not stop the pass
            facts = {"error": f"{type(exc).__name__}: {exc}"}
        label, evidence = classify(path, facts)
        slug = slugify(path.stem) if path.stem.isascii() else ""
        plan.append({
            "old": path.name, "new": None, "label": label, "evidence": evidence,
            "numbered": not (path.stem.isascii() and len(slug) >= 4),
            "slug": slug, "pages": facts.get("pages"),
            "text_layer": facts.get("has_text_layer"), "scripts": facts.get("scripts"),
            "skipped": False,
        })

    # Number per language rather than by position in the folder listing: `persian-48.pdf` is an
    # accident of sort order and tells the reader nothing.
    counters: dict[str, int] = {}
    for row in plan:
        if row.get("skipped"):
            continue
        if row["numbered"]:
            counters[row["label"]] = counters.get(row["label"], 0) + 1
            base = f"{row['label']}-{counters[row['label']]}"
        else:
            base = f"{row['label']}-{row['slug']}"
        candidate = f"{base}.pdf"
        bump = 2
        while candidate in used and candidate != row["old"]:
            candidate = f"{base}-{bump}.pdf"
            bump += 1
        used.add(candidate)
        row["new"] = candidate

    for row in plan:
        if row.get("skipped"):
            continue
        mark = "=" if row["old"] == row["new"] else "->"
        print(f"{mark:>2} {row['old'][:52]:<54} {row['new'][:46]:<48} "
              f"[{row['label']}/{row['evidence']}] p={row['pages']}")

    renamed = [r for r in plan if not r.get("skipped") and r["old"] != r["new"]]
    print(f"\n{len(plan)} file(s): {len(renamed)} to rename, "
          f"{sum(1 for r in plan if r.get('skipped'))} already conforming")

    if not args.apply:
        print("dry run — nothing changed. Re-run with --apply to rename.")
        return 0

    for row in renamed:
        (root / row["old"]).rename(root / row["new"])
    out = pathlib.Path(args.map_out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps({"directory": str(root), "moves": renamed}, indent=2, ensure_ascii=False) + "\n",
                   encoding="utf-8")
    print(f"renamed {len(renamed)} file(s); reversible map written to {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
