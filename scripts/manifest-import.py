#!/usr/bin/env python3
"""Turn a triage report into manifest rows — for an archive of files we do not own.

Why this exists: the corpus will keep receiving folders of real-world PDFs (a desktop
dump, a colleague's samples, a customer's documents). Hand-writing 40 rows is how you get
a manifest with guessed licences and wrong producers — the exact rot `manifest-add.py`
refuses. This importer fills only what triage *measured*, and leaves the rest explicitly
unknown.

It is deliberately conservative:
  * `licence` is `unknown` — an importer cannot know it, and guessing is the one thing
    that must never happen here;
  * `producer_class` stays `unknown` unless the file carries `/ReversedChars` (which is
    itself evidence that the producer stores visual order);
  * `redistributable` is false whenever the destination is under `corpus/raw/private/`,
    which is also gitignored — the files stay local, their hashes travel;
  * language is `fa` when Arabic-script codepoints were found, `en` for Latin-only, and
    `und` (undetermined) otherwise. Persian and Arabic are indistinguishable by codepoint
    alone, so `fa` here means "Arabic script, probably Persian given the filename" and the
    report says so.

Usage:
    python3 scripts/manifest-import.py --triage reports/triage-desktop-pdfs.json \
        --dest corpus/raw/private/desktop-pdfs --prefix owned-desktop
"""
from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "corpus" / "manifest.json"


def sha256(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(65536), b""):
            digest.update(chunk)
    return digest.hexdigest()


def slugify(name: str) -> str:
    slug = re.sub(r"[^a-z0-9]+", "-", name.lower()).strip("-")
    return (slug or "file")[:48]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--triage", required=True)
    parser.add_argument("--dest", required=True, help="directory the files were copied to (repo-relative)")
    parser.add_argument("--prefix", required=True, help="id prefix, e.g. owned-desktop")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    triage = json.loads((ROOT / args.triage).read_text(encoding="utf-8"))
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    dest = ROOT / args.dest
    private = "raw/private" in args.dest

    added = updated = missing = 0
    for entry in triage["files"]:
        source = pathlib.Path(entry["path"])
        target = dest / source.name
        if not target.exists():
            print(f"MISSING (not copied): {source.name}", file=sys.stderr)
            missing += 1
            continue

        counts = entry.get("counts", {})
        scripts = entry.get("scripts") or []
        if "ar" in scripts:
            language, direction = "fa", "rtl"
        elif "he" in scripts:
            language, direction = "he", "rtl"
        elif "latin" in scripts:
            language, direction = "en", "ltr"
        else:
            language, direction = "und", "unknown"

        digest = sha256(target)
        rel_path = f"{args.dest}/{source.name}"
        # identity is the path, not the slug: re-importing the same folder must update the same
        # rows. Deriving the id from the filename instead made every re-import mint a second row
        # for names that slugify to "" (all-Persian filenames) or to a slug already taken.
        existing = next((r for r in manifest["fixtures"] if r.get("path") == rel_path), None)
        if existing:
            fixture_id = existing["id"]
        else:
            base = f"{args.prefix}-{slugify(source.stem) or 'unnamed'}"
            taken = {r["id"] for r in manifest["fixtures"]}
            fixture_id = base if base not in taken else f"{base}-{digest[:8]}"

        row = {
            "id": fixture_id,
            "path": rel_path,
            "language": language,
            "script": "Arab" if language == "fa" else "Latn" if language == "en" else "Zyyy",
            "direction": direction,
            "producer": entry.get("producer") or "unknown",
            "producer_tool": "unknown",
            "producer_class": "visual" if counts.get("reversed_chars") else "unknown",
            "licence": "unknown",
            "source_url": None,
            "sha256": digest,
            "redistributable": not private,
            "status": "triaged",
            "notes": (
                f"Real-world archive file, provenance unresolved (licence unknown — see "
                f"docs/OPEN-QUESTIONS.md Q-005). Triage: pages={entry.get('pages')}, "
                f"scripts={scripts or 'none detected'}, text_ops={entry.get('text_ops')}, "
                f"ActualText={counts.get('actual_text', 0)}, "
                f"ReversedChars={counts.get('reversed_chars', 0)}, "
                f"Type0={counts.get('type0_fonts', 0)}, "
                f"ToUnicode={counts.get('tounicode_refs', 0)}, "
                f"CIDToGIDMap={counts.get('cid_to_gid', 0)}, "
                f"scanned={not entry.get('has_text_layer')}, "
                f"verdict={entry.get('verdict', '')}"
            ),
        }
        if existing:
            existing.update(row)
            updated += 1
        else:
            manifest["fixtures"].append(row)
            added += 1

    if args.dry_run:
        print(f"dry run: would add {added}, update {updated}, missing {missing}")
        return 0

    MANIFEST.write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"added {added}, updated {updated}, missing {missing} (total {len(manifest['fixtures'])})")
    print("now run: python3 scripts/manifest-add.py --check")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
