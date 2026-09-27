#!/usr/bin/env python3
"""Render page 1 of every review fixture and emit the human vision-review checklist.

WHY this exists: text extractors disagree about RTL order (reports/oracle-extraction.md),
so the only surface a human can check without trusting an extractor is the rendered page.
This script renders each fixture with scripts/render_pages.py, proves every PNG is
non-blank (the non-white pixel fraction is printed AND put in the table), copies the
expected sentence straight out of corpus/generated/SOURCES.md -- the authority, never
retyped -- and writes a checklist with a blank "verified by human: yes/no" column.

ZWNJ is written as the explicit escape \\u200C in the table, because an invisible
codepoint in a printed sentence is exactly what visual review cannot catch
(corpus/generated/SOURCES.md, rule 4).

Usage (run from the repo root, repo's venv):
    ./.venv/Scripts/python.exe scripts/build_vision_review.py
    ./.venv/Scripts/python.exe scripts/build_vision_review.py --out reports/vision-review-2026-09-27.md

Exit codes: 0 = every page rendered and non-blank; 1 = a render failed or was blank.
"""
from __future__ import annotations

import argparse
import datetime
import pathlib
import re
import sys

import render_pages  # scripts/render_pages.py -- the rendering must stay in one place

REPO_ROOT = pathlib.Path(__file__).resolve().parent.parent
SOURCES = REPO_ROOT / "corpus" / "generated" / "SOURCES.md"
DEFAULT_OUT = REPO_ROOT / "reports" / "vision-review-2026-09-27.md"

# (pdf path relative to repo root, SOURCES.md key or None if the file has no authority line)
FIXTURES: list[tuple[str, str | None]] = [
    ("corpus/raw/generated/chrome/fa-plain.pdf", "fa-plain"),
    ("corpus/raw/generated/chrome/fa-zwnj-word.pdf", "fa-zwnj-word"),
    ("corpus/raw/generated/chrome/fa-zwnj-lamalef.pdf", "fa-zwnj-lamalef"),
    ("corpus/raw/generated/chrome/mixed-fa-en.pdf", "mixed-fa-en"),
    ("corpus/raw/generated/chrome/en-control.pdf", "en-control"),
    ("corpus/raw/synthetic/actualtext-fa.pdf", None),
    ("corpus/raw/private/desktop-pdfs/hebrew-2.pdf", None),
    ("corpus/raw/private/desktop-pdfs/arabic-2.pdf", None),
]
NO_SENTENCE = {
    None: "(no line in SOURCES.md -- synthetic ActualText fixture; check the page reads "
          "as the two Persian lines of its source text)",
    "hebrew-2": "(no line in SOURCES.md -- real-world Hebrew PDF; check the page reads "
                "as coherent Hebrew)",
    "arabic-2": "(no line in SOURCES.md -- real-world Arabic PDF; check the page reads "
                "as coherent Arabic)",
}

ROW_RE = re.compile(r"^\|\s*`([a-z0-9-]+)`\s*\|\s*`([^`]+)`\s*\|")


def load_sources() -> dict[str, str]:
    """key -> exact sentence from corpus/generated/SOURCES.md (never retyped)."""
    if not SOURCES.is_file():
        raise FileNotFoundError(f"authority file missing: {SOURCES}")
    sentences: dict[str, str] = {}
    for line in SOURCES.read_text(encoding="utf-8").splitlines():
        match = ROW_RE.match(line)
        if match:
            sentences[match.group(1)] = match.group(2)
    if not sentences:
        raise ValueError(f"no table rows parsed from {SOURCES}")
    return sentences


def as_cell(sentence: str) -> str:
    """Keep ZWNJ visible as an explicit escape inside the markdown cell."""
    return sentence.replace("\u200C", "\\u200C")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--out", default=str(DEFAULT_OUT), help="markdown output path")
    parser.add_argument("--scale", type=float, default=3.0, help="render scale (default 3.0)")
    args = parser.parse_args()

    try:
        sources = load_sources()
    except (OSError, ValueError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 1

    rows: list[dict] = []
    exit_code = 0
    for rel_pdf, key in FIXTURES:
        pdf_path = REPO_ROOT / rel_pdf
        if not pdf_path.is_file():
            print(f"ERROR: missing fixture {rel_pdf}", file=sys.stderr)
            exit_code = 1
            rows.append({"pdf": rel_pdf, "error": "fixture file missing"})
            continue
        results = render_pages.render(pdf_path, REPO_ROOT / "reports" / "render", [1],
                                      args.scale, render_pages.DEFAULT_BLANK_WHITE,
                                      render_pages.DEFAULT_MIN_INK)
        if not results:
            print(f"ERROR: nothing rendered for {rel_pdf}", file=sys.stderr)
            exit_code = 1
            rows.append({"pdf": rel_pdf, "error": "render produced nothing"})
            continue
        rec = results[0]
        status = "BLANK" if rec["blank"] else "non-blank"
        print(f"page 1 -> {rec['path']}  {rec['width']}x{rec['height']} px  "
              f"non-white={rec['non_white_fraction']:.6f} "
              f"({rec['non_white_fraction'] * 100.0:.3f}%)  ink={rec['ink_pixels']}px  "
              f"{status}")
        if rec["blank"]:
            exit_code = 1
            print(f"FAIL: {rel_pdf} page 1 rendered blank "
                  f"({rec['white_percent']:.3f}% white)", file=sys.stderr)
        if key is not None and key not in sources:
            print(f"ERROR: {key} has no row in {SOURCES}", file=sys.stderr)
            exit_code = 1
        rows.append({"pdf": rel_pdf, "key": key, "rec": rec})

    # ------------------------------------------------------------------ markdown
    lines: list[str] = []
    add = lines.append
    add("# Vision review checklist -- fixture pages (2026-09-27)")
    add("")
    add("Page 1 of every review fixture, rendered with PDFium "
        f"(`scripts/render_pages.py --pages 1 --scale {args.scale:g}`), plus the sentence "
        "each page is *supposed* to print.")
    add("")
    add("- **The expected sentences are copied verbatim from "
        "`corpus/generated/SOURCES.md`** (the project's ground truth for text); this file "
        "is generated by `scripts/build_vision_review.py`, nothing is retyped.")
    add("- **ZWNJ is written as the explicit escape `\\u200C`**, because a ZWNJ is "
        "invisible in a rendered sentence -- `corpus/generated/SOURCES.md` rule 4: visual "
        "review can never catch it, only a codepoint comparison can. The codepoint-level "
        "check lives in `reports/oracle-extraction.md`.")
    add("- **Every PNG below is verified non-blank** by the same run: the table shows the "
        "measured non-white pixel fraction (any channel != 255). `render_pages.py` exits "
        "non-zero on a blank render. These fixtures are one line of text on an A4 page, so "
        "a CORRECT render measures 99.6%-99.9% white -- above the 99.5% limit -- which is "
        "why the blank check combines that limit with an ink-pixel floor (both numbers "
        "appear per page below; the script docstring records the measured values).")
    add("- How to verify: open the PNG, read the page, compare it with the expected "
        "sentence, then write `yes` (matches) or `no` (does not match) in the last column.")
    add("")
    add("| # | Fixture (page 1) | Rendered PNG | Pixels | Non-white fraction | "
        "Expected text (SOURCES.md, ZWNJ as `\\u200C`) | Verified by human: yes/no |")
    add("|---|---|---|---|---|---|---|")
    for index, row in enumerate(rows, start=1):
        name = pathlib.Path(row["pdf"]).name
        if row.get("error"):
            add(f"| {index} | `{name}` | ERROR: {row['error']} | | | | |")
            continue
        rec = row["rec"]
        key = row["key"]
        if key is not None:
            expected = f"`{as_cell(sources[key])}`"
        else:
            stem = name[:-4] if name.endswith(".pdf") else name
            expected = NO_SENTENCE.get(stem, NO_SENTENCE[None])
        blank_flag = " (BLANK!)" if rec["blank"] else ""
        add(f"| {index} | `{name}` | `{rec['path']}` | {rec['width']}x{rec['height']} | "
            f"{rec['non_white_fraction']:.6f} "
            f"({rec['non_white_fraction'] * 100.0:.3f}%){blank_flag} | {expected} | |")
    add("")
    add("## Evidence per page (printed by the run that wrote this file)")
    add("")
    add("```")
    for row in rows:
        if row.get("error"):
            add(f"{row['pdf']}: ERROR {row['error']}")
            continue
        rec = row["rec"]
        add(f"{row['pdf']} page 1 -> {rec['path']}  {rec['width']}x{rec['height']} px  "
            f"non-white={rec['non_white_fraction']:.6f}  ink={rec['ink_pixels']}px  "
            f"white={rec['white_percent']:.3f}%  "
            f"blank_check={'BLANK' if rec['blank'] else 'ok'}")
    add("```")
    add("")
    add("## Reviewer notes")
    add("")
    add("- [ ] All PNGs above are readable (not blank, no missing glyphs).")
    add("- [ ] Each rendered page matches its expected sentence character for character, "
        "including `\\u200C` where shown -- the ZWNJ is invisible in the image, so this "
        "must be checked with a codepoint tool, not the eye "
        "(see `reports/oracle-extraction.md`).")
    add("- [ ] `reports/oracle-extraction.md` agrees with what I read on these pages.")
    add("")
    add(f"Generated {datetime.datetime.now().isoformat(timespec='seconds')} by "
        "`scripts/build_vision_review.py`.")
    add("")

    out_path = pathlib.Path(args.out)
    if not out_path.is_absolute():
        out_path = REPO_ROOT / out_path
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text("\n".join(lines), encoding="utf-8")
    print(f"wrote {out_path.relative_to(REPO_ROOT).as_posix()}")
    if exit_code:
        print("FAIL: one or more pages did not pass the non-blank check", file=sys.stderr)
    return exit_code


if __name__ == "__main__":
    raise SystemExit(main())
