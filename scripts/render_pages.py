#!/usr/bin/env python3
"""Render PDF pages to PNG with PDFium so a human (or a vision model) can read what the
page is SUPPOSED to say.

WHY this exists: every text extractor available here reorders or drops RTL text in its own
way (see reports/oracle-extraction.md), so pixels are the only review surface that does
not depend on an extractor's opinion. PDFium is used because pypdfium2 is permissively
licensed (BSD-3/Apache-2.0), bundles its own renderer (no system install) and is the
engine Chrome itself uses.

The script refuses to be silently useless: it measures the non-white pixel fraction of
every rendered page and fails on a blank render, because a blank PNG would let a reviewer
"confirm" fixtures that say nothing, which is worse than a failing test.

BLANK CHECK (a page is blank only if BOTH conditions hold):
  * the page is >= --blank-white white (default 0.995 = the 99.5%-white rule), AND
  * the page carries fewer than --min-ink pixels (default 256).
The second condition is required, not decoration: these fixtures put one line of text in
the top ~10% of an A4 page, so a CORRECT render measures 99.75%-99.93% white (measured
2026-09-27: fa-zwnj-lamalef 99.755% white, fa-plain 99.925% white). The 99.5% rule on its
own would reject every real fixture and would pass only a uniformly empty page. A genuinely
failed render has 0 ink pixels and still fails. Both numbers (white %, ink px) are printed
for every page so the reviewer can judge.

Usage (run from the repo root, repo's venv):
    ./.venv/Scripts/python.exe scripts/render_pages.py corpus/raw/generated/chrome/fa-plain.pdf
    ./.venv/Scripts/python.exe scripts/render_pages.py FILE.pdf --pages 1 --scale 3 \
        --out-dir reports/render --json reports/render/fa-plain.json

Output layout: <out-dir>/<pdf-stem>/page-<N>.png   (N is the 1-based page number)
Exit codes: 0 = every requested page rendered and non-blank;
            1 = a rendered page was blank, or no page could be rendered;
            2 = bad arguments / unreadable input.
"""
from __future__ import annotations

import argparse
import json
import pathlib
import sys

import pypdfium2 as pdfium
from PIL import Image, ImageChops

REPO_ROOT = pathlib.Path(__file__).resolve().parent.parent
DEFAULT_OUT_DIR = "reports/render"
DEFAULT_BLANK_WHITE = 0.995   # the 99.5%-white rule from the spec
DEFAULT_MIN_INK = 256         # pixels; a failed render has 0


def parse_pages(spec: str, page_count: int) -> list[int]:
    """Parse a 1-based page spec like '1', '1-2', '3,5-7' into sorted unique page numbers."""
    pages: set[int] = set()
    for chunk in spec.split(","):
        chunk = chunk.strip()
        if not chunk:
            continue
        if "-" in chunk:
            lo_s, _, hi_s = chunk.partition("-")
            try:
                lo, hi = int(lo_s), int(hi_s)
            except ValueError as exc:
                raise argparse.ArgumentTypeError(f"bad page range {chunk!r}") from exc
            if lo < 1 or hi < lo:
                raise argparse.ArgumentTypeError(f"bad page range {chunk!r} (need 1 <= lo <= hi)")
            pages.update(range(lo, hi + 1))
        else:
            try:
                n = int(chunk)
            except ValueError as exc:
                raise argparse.ArgumentTypeError(f"bad page number {chunk!r}") from exc
            if n < 1:
                raise argparse.ArgumentTypeError(f"bad page number {chunk!r} (must be >= 1)")
            pages.add(n)
    if not pages:
        raise argparse.ArgumentTypeError(f"no pages in {spec!r}")
    kept = sorted(p for p in pages if p <= page_count)
    for dropped in sorted(p for p in pages if p > page_count):
        print(f"WARN page {dropped} requested but the file has only {page_count} page(s) "
              f"-- skipped", file=sys.stderr)
    return kept


def ink_stats(image: Image.Image) -> tuple[int, tuple[int, int, int, int] | None]:
    """Count pixels that are not exactly (255, 255, 255) and return their bounding box.

    Anti-aliased strokes, rules and tints all count as ink: anything that is not pure
    white proves the renderer actually drew something.
    """
    rgb = image.convert("RGB")
    white = Image.new("RGB", rgb.size, (255, 255, 255))
    diff = ImageChops.difference(rgb, white)
    r, g, b = diff.split()
    ink = ImageChops.lighter(ImageChops.lighter(r, g), b)
    histogram = ink.histogram()
    total = rgb.size[0] * rgb.size[1]
    ink_px = total - histogram[0]   # histogram()[0] == pixels identical to white
    bbox = ink.getbbox() if ink_px else None
    return ink_px, bbox


def render(pdf_path: pathlib.Path, out_dir: pathlib.Path, pages: list[int], scale: float,
           blank_white: float, min_ink: int) -> list[dict]:
    """Render `pages` of `pdf_path`; returns one record per rendered page."""
    results: list[dict] = []
    doc = pdfium.PdfDocument(str(pdf_path))
    try:
        for number in pages:
            page = doc[number - 1]
            image = page.render(scale=scale).to_pil()
            target_dir = out_dir / pdf_path.stem
            target_dir.mkdir(parents=True, exist_ok=True)
            target = target_dir / f"page-{number}.png"
            image.save(target, format="PNG")
            ink_px, bbox = ink_stats(image)
            total = image.size[0] * image.size[1]
            non_white = ink_px / total if total else 0.0
            white_fraction = 1.0 - non_white
            blank = white_fraction >= blank_white and ink_px < min_ink
            results.append({
                "page": number,
                "path": rel_to_repo(str(target)),
                "width": image.size[0],
                "height": image.size[1],
                "non_white_fraction": round(non_white, 6),
                "white_percent": round(white_fraction * 100.0, 4),
                "ink_pixels": ink_px,
                "ink_bbox": list(bbox) if bbox else None,
                "blank": blank,
                "white_rule_fired": white_fraction >= blank_white,
            })
    finally:
        doc.close()
    return results


def rel_to_repo(path_str: str) -> str:
    """Repo-relative POSIX path (always forward slashes, so reports are host-neutral)."""
    path = pathlib.Path(path_str)
    try:
        return path.resolve().relative_to(REPO_ROOT).as_posix()
    except ValueError:
        return path.as_posix()


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Render PDF pages to PNG with PDFium and verify they are not blank.")
    parser.add_argument("pdf", help="input PDF path")
    parser.add_argument("--out-dir", default=DEFAULT_OUT_DIR,
                        help=f"output directory (default: {DEFAULT_OUT_DIR}); "
                             "files land in <out-dir>/<stem>/page-N.png")
    parser.add_argument("--pages", default="1-2",
                        help="1-based pages to render, e.g. '1', '1-2', '3,5-7' (default: 1-2)")
    parser.add_argument("--scale", type=float, default=3.0,
                        help="render scale, 1.0 = 72 dpi (default: 3.0)")
    parser.add_argument("--json", metavar="PATH", default=None,
                        help="also write the results as JSON to PATH")
    parser.add_argument("--blank-white", type=float, default=DEFAULT_BLANK_WHITE,
                        help="white fraction that starts the blank check "
                             f"(default: {DEFAULT_BLANK_WHITE})")
    parser.add_argument("--min-ink", type=int, default=DEFAULT_MIN_INK,
                        help="ink-pixel floor that keeps a sparse-but-real page from being "
                             f"called blank (default: {DEFAULT_MIN_INK})")
    args = parser.parse_args()

    pdf_path = pathlib.Path(args.pdf)
    if not pdf_path.is_file():
        print(f"ERROR: no such file: {pdf_path}", file=sys.stderr)
        return 2
    if args.scale <= 0:
        print("ERROR: --scale must be > 0", file=sys.stderr)
        return 2
    if not 0.0 < args.blank_white <= 1.0:
        print("ERROR: --blank-white must be in (0, 1]", file=sys.stderr)
        return 2
    if args.min_ink < 0:
        print("ERROR: --min-ink must be >= 0", file=sys.stderr)
        return 2

    try:
        doc = pdfium.PdfDocument(str(pdf_path))
    except Exception as exc:  # noqa: BLE001 - report, never swallow
        print(f"ERROR: cannot open {pdf_path}: {type(exc).__name__}: {exc}", file=sys.stderr)
        return 2
    page_count = len(doc)
    doc.close()

    try:
        pages = parse_pages(args.pages, page_count)
    except argparse.ArgumentTypeError as exc:
        print(f"ERROR: --pages: {exc}", file=sys.stderr)
        return 2
    if not pages:
        print(f"ERROR: no requested page is within the {page_count} page(s) of {pdf_path}",
              file=sys.stderr)
        return 1

    try:
        results = render(pdf_path, pathlib.Path(args.out_dir), pages, args.scale,
                         args.blank_white, args.min_ink)
    except Exception as exc:  # noqa: BLE001 - a render failure must be loud, never silent
        print(f"ERROR: render failed for {pdf_path}: {type(exc).__name__}: {exc}",
              file=sys.stderr)
        return 1
    if not results:
        print("ERROR: nothing was rendered", file=sys.stderr)
        return 1

    exit_code = 0
    for rec in results:
        status = "BLANK" if rec["blank"] else "ok"
        print(f"page {rec['page']} -> {rec['path']}  "
              f"{rec['width']}x{rec['height']} px  "
              f"non-white={rec['non_white_fraction']:.6f} "
              f"({rec['non_white_fraction'] * 100.0:.3f}%)  "
              f"ink={rec['ink_pixels']}px  bbox={rec['ink_bbox']}  {status}")
        if rec["white_rule_fired"] and not rec["blank"]:
            print(f"note: page {rec['page']} is {rec['white_percent']:.3f}% white "
                  f"(>= {args.blank_white * 100:.1f}% rule) but carries "
                  f"{rec['ink_pixels']} ink px (floor {args.min_ink}) -> content present")
        if rec["blank"]:
            exit_code = 1
            print(f"FAIL: page {rec['page']} is {rec['white_percent']:.3f}% white with only "
                  f"{rec['ink_pixels']} ink px -- blank render, the review lane would be "
                  f"worthless", file=sys.stderr)

    if args.json:
        json_path = pathlib.Path(args.json)
        json_path.parent.mkdir(parents=True, exist_ok=True)
        json_path.write_text(json.dumps({
            "input": str(pdf_path),
            "scale": args.scale,
            "pages_requested": args.pages,
            "page_count": page_count,
            "blank_white": args.blank_white,
            "min_ink": args.min_ink,
            "rendered": results,
            "ok": exit_code == 0,
        }, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
        print(f"wrote {json_path}")

    return exit_code


if __name__ == "__main__":
    raise SystemExit(main())
