#!/usr/bin/env python3
"""Classify the STORED order of RTL text in a PDF, with two independent implementations.

Research instrument only — never shipped, and deliberately not part of the extraction path.
It exists so that an allow-list entry, a bug report or a refusal can be justified by a
measurement instead of by a producer's name (docs/decisions/0006, docs/research/).

Method, per page:

  stream order   `pdftotext -raw -enc UTF-8`  — poppler's raw mode is documented to keep
                                              content-stream order, i.e. the order the
                                              producer wrote the glyphs in.
  painted order  PDFium `FPDFText_GetCharBox` — per-character user-space boxes from a second
                                              implementation, sorted by x within each line.

Both are reduced to their RTL characters, because the inverse of UAX #9 rule L2 is a plain
reversal only for a pure-RTL run; mixed lines are reported as undecided rather than guessed.

  stream == painted           -> the file stores PAINTED-VISUAL order
  stream == reverse(painted)  -> the file stores LOGICAL order

The report contains only lengths, verdicts and indices. No document text is written or
printed: this runs over private customer files (see the corpus policy in corpus/README.md).
"""

import argparse
import json
import pathlib
import re
import subprocess
import sys

RTL = re.compile(
    "[\u0590-\u05FF\u0600-\u06FF\u0750-\u077F\u08A0-\u08FF\uFB1D-\uFDFF\uFE70-\uFEFF]"
)
LINE_TOLERANCE = 2.0
MARGIN = 0.05


def bigram_score(a: str, b: str) -> float:
    """Fraction of `a`'s adjacent character pairs that also appear, in order, in `b`.

    Length-tolerant on purpose. The two oracles disagree about which characters survive —
    PDFium drops ZWNJ, poppler keeps it, combining marks are handled differently — so strict
    equality reports `mismatch` on files that agree perfectly about the ORDER, which is the
    only thing being asked here.
    """
    if len(a) < 2 or len(b) < 2:
        return 0.0
    from collections import Counter

    pairs_a = Counter(zip(a, a[1:]))
    pairs_b = Counter(zip(b, b[1:]))
    return sum((pairs_a & pairs_b).values()) / (len(a) - 1)


def rtl_only(text: str) -> str:
    return "".join(ch for ch in text if RTL.match(ch))


def poppler_page(path: pathlib.Path, mode: str, page: int) -> str:
    """`-raw` is content-stream order; `-layout` is poppler's own reordering."""
    proc = subprocess.run(
        ["pdftotext", f"-{mode}", "-enc", "UTF-8", "-f", str(page), "-l", str(page), str(path), "-"],
        capture_output=True,
        text=True,
        timeout=180,
    )
    return proc.stdout


def pdfium_painted_page(path: pathlib.Path, page: int) -> str:
    """RTL characters of one page, in the order they are PAINTED (sorted by x within a line)."""
    import pypdfium2 as pdfium

    doc = pdfium.PdfDocument(str(path))
    try:
        # Keep the page object alive: dropping it invalidates the textpage and the next
        # call fails with "Failed to load page".
        page_obj = doc[page - 1]
        textpage = page_obj.get_textpage()
        count = textpage.count_chars()
        placed = []
        for index in range(count):
            ch = textpage.get_text_range(index, 1)
            if not ch or not RTL.match(ch):
                continue
            try:
                left, bottom, _right, _top = textpage.get_charbox(index)
            except Exception:
                continue
            placed.append((bottom, left, ch))
    finally:
        doc.close()

    if not placed:
        return ""

    placed.sort(key=lambda item: (-round(item[0], 1), item[1]))
    lines, current, current_y = [], [], None
    for bottom, left, ch in placed:
        if current_y is None or abs(bottom - current_y) <= LINE_TOLERANCE:
            current.append((left, ch))
            if current_y is None:
                current_y = bottom
        else:
            lines.append(current)
            current = [(left, ch)]
            current_y = bottom
    if current:
        lines.append(current)
    return "".join(
        ch for line in lines for _left, ch in sorted(line, key=lambda item: item[0])
    )


def classify_page(path: pathlib.Path, page: int) -> dict:
    stream = rtl_only(poppler_page(path, "raw", page))
    painted = pdfium_painted_page(path, page)
    record = {
        "page": page,
        "stream_rtl": len(stream),
        "painted_rtl": len(painted),
    }
    if len(stream) < 8 or len(painted) < 8:
        record["verdict"] = "no_rtl"
        return record
    forward = bigram_score(stream, painted)
    backward = bigram_score(stream, painted[::-1])
    record["bigram_stream_vs_painted"] = round(forward, 4)
    record["bigram_stream_vs_reversed"] = round(backward, 4)
    margin = forward - backward
    # The margin is part of the evidence: a decisive verdict must differ by a real gap,
    # not by rounding noise on a pair of scores that are both near zero.
    record["margin"] = round(margin, 4)
    if margin > MARGIN:
        record["verdict"] = "visual"
    elif margin < -MARGIN:
        record["verdict"] = "logical"
    else:
        record["verdict"] = "undecided"
    return record


def pdfium_page_count(path: pathlib.Path) -> int:
    import pypdfium2 as pdfium

    doc = pdfium.PdfDocument(str(path))
    try:
        return len(doc)
    finally:
        doc.close()


def classify_file(path: pathlib.Path, pages: int) -> dict:
    # A one-page file asked for page 3 is not an error in the file. Clamp to what exists and
    # report both numbers, so "sampled" can never be misread as "all pages".
    available = pdfium_page_count(path)
    sampled = max(1, min(pages, available))
    per_page = [classify_page(path, page) for page in range(1, sampled + 1)]
    tally: dict[str, int] = {}
    for page in per_page:
        tally[page["verdict"]] = tally.get(page["verdict"], 0) + 1
    decided = [v for v in ("visual", "logical") if tally.get(v)]
    overall = decided[0] if len(decided) == 1 else ("mixed" if decided else "undecided")
    return {
        "file": path.name,
        "pages_available": available,
        "pages_sampled": sampled,
        "tally": tally,
        "overall": overall,
        "pages": per_page,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("files", nargs="+")
    parser.add_argument("--pages", type=int, default=3)
    parser.add_argument("--json-out")
    args = parser.parse_args()

    results = []
    for name in args.files:
        path = pathlib.Path(name)
        if not path.exists():
            print(f"{path.name}: MISSING")
            continue
        try:
            result = classify_file(path, args.pages)
        except Exception as error:  # a probe that cannot see its input says so
            print(f"{path.name}: ERROR {type(error).__name__}: {error}")
            continue
        results.append(result)
        print(f"{result['file']:38} {result['overall']:10} {json.dumps(result['tally'])}")

    if args.json_out:
        pathlib.Path(args.json_out).write_text(
            json.dumps(results, ensure_ascii=False, indent=2), encoding="utf-8"
        )
        print(f"\nwrote {args.json_out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
