#!/usr/bin/env python3
"""Render the fixed sentence set through real producers to create corpus fixtures.

Why these fixtures matter more than any hand-written PDF: producers disagree about how
RTL text is stored (visual vs logical order, `/ActualText` or not, one glyph per `Tj` or
not). The same sentence through different producers is how we find that out — and Chrome
is the first producer because it is installed here, is scriptable headless, and is a known
"visual order + per-glyph ActualText" offender.

Usage:
    python tools/make_producer_fixtures.py                    # chrome, fa+en+mixed
    python tools/make_producer_fixtures.py --sets fa,en,ar,he,mixed
    python tools/make_producer_fixtures.py --chrome "C:/Program Files/Google/Chrome/Application/chrome.exe"

Output: corpus/raw/generated/chrome/<id>.pdf  (+ the HTML used, for reproducibility)
"""
from __future__ import annotations

import argparse
import html
import pathlib
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parent.parent
OUT_DIR = ROOT / "corpus" / "raw" / "generated" / "chrome"

# Ground truth lives in corpus/generated/SOURCES.md — keep these two in sync, and never
# "fix" a sentence here to make an extraction test pass.
SENTENCES: dict[str, tuple[str, str, str]] = {
    # key: (language, direction, text)
    "fa-zwnj-lamalef": ("fa", "rtl", "نیم\u200cفاصله و لا اله الا الله — ۱۴۰۳/۰۵/۱۲"),
    "fa-zwnj-word": ("fa", "rtl", "می\u200cروم"),
    "fa-plain": ("fa", "rtl", "سلام دنیا"),
    "en-control": ("en", "ltr", "pdfrtl v0.1 — ISO 32000-1"),
    "mixed-fa-en": ("fa", "rtl", "گزارش فنی pdfrtl v0.1 — ISO 32000-1 §9.7.4.3 — 42%"),
    "ar-harakat": ("ar", "rtl", "اللغة العربية مُشكَّلةٌ بالحركات — ٢٠٢٤"),
    "he-niqqud": ("he", "rtl", "שָׁלוֹם עוֹלָם — 2024"),
}

DEFAULT_SETS = ["fa-zwnj-lamalef", "fa-zwnj-word", "fa-plain", "en-control", "mixed-fa-en"]

DEFAULT_CHROME_CANDIDATES = [
    r"C:\Program Files\Google\Chrome\Application\chrome.exe",
    r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
    r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
]

# Tahoma/Segoe UI cover Arabic script and Persian digits on a stock Windows install.
FONT_STACK = "Tahoma, 'Segoe UI', 'Times New Roman', sans-serif"


def build_html(keys: list[str]) -> str:
    blocks = []
    for key in keys:
        language, direction, text = SENTENCES[key]
        blocks.append(
            f'  <section lang="{language}" dir="{direction}">\n'
            f'    <div class="body">{html.escape(text)}</div>\n'
            f'    <div class="meta">{key} · {language} · {direction}</div>\n'
            f"  </section>"
        )
    return (
        "<!doctype html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n"
        f"<title>pdfrtl producer fixture</title>\n<style>\n"
        f"  @page {{ size: A4; margin: 18mm; }}\n"
        f"  body {{ font-family: {FONT_STACK}; }}\n"
        f"  section {{ margin-bottom: 14mm; }}\n"
        f"  .body {{ font-size: 18pt; line-height: 1.9; }}\n"
        f"  .meta {{ font-size: 8pt; color: #666; font-family: monospace; direction: ltr; }}\n"
        f"</style>\n</head>\n<body>\n" + "\n".join(blocks) + "\n</body>\n</html>\n"
    )


def find_chrome(explicit: str | None) -> str:
    if explicit:
        return explicit
    for candidate in DEFAULT_CHROME_CANDIDATES:
        if pathlib.Path(candidate).exists():
            return candidate
    raise SystemExit("no Chrome/Edge found; pass --chrome <path>")


def render(chrome: str, html_path: pathlib.Path, pdf_path: pathlib.Path, profile: pathlib.Path) -> None:
    cmd = [
        chrome,
        "--headless=new",
        "--disable-gpu",
        "--no-pdf-header-footer",
        "--virtual-time-budget=3000",  # let local fonts settle before printing
        f"--user-data-dir={profile}",
        f"--print-to-pdf={pdf_path}",
        html_path.as_uri(),
    ]
    proc = subprocess.run(cmd, capture_output=True, text=True)
    if proc.returncode != 0 or not pdf_path.exists():
        print(proc.stdout[-2000:], file=sys.stderr)
        print(proc.stderr[-2000:], file=sys.stderr)
        raise SystemExit(f"chrome failed (exit {proc.returncode})")
    if pdf_path.stat().st_size < 500:
        raise SystemExit(f"{pdf_path} looks empty ({pdf_path.stat().st_size} bytes)")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--sets", default=",".join(DEFAULT_SETS),
                        help="comma-separated keys from corpus/generated/SOURCES.md")
    parser.add_argument("--chrome", default=None)
    args = parser.parse_args()

    keys = [k.strip() for k in args.sets.split(",") if k.strip()]
    unknown = [k for k in keys if k not in SENTENCES]
    if unknown:
        raise SystemExit(f"unknown sentence keys: {unknown}; known: {sorted(SENTENCES)}")

    chrome = find_chrome(args.chrome)
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    html_path = OUT_DIR / "source.html"

    with tempfile.TemporaryDirectory(prefix="pdfrtl-chrome-") as profile_dir:
        profile = pathlib.Path(profile_dir)
        rendered = 0
        for key in keys:
            # One page per sentence: the HTML must contain ONLY that sentence, otherwise
            # every fixture is a copy of the whole set (caught by identical file sizes).
            page_html = OUT_DIR / f"source-{key}.html"
            page_html.write_text(build_html([key]), encoding="utf-8")
            pdf_path = OUT_DIR / f"{key}.pdf"
            render(chrome, page_html, pdf_path, profile)
            rendered += 1
            print(f"rendered {pdf_path.relative_to(ROOT)} ({pdf_path.stat().st_size} bytes)")

    # One combined fixture keeps the whole set in a single file for end-to-end runs.
    combined = OUT_DIR / "all-sets.pdf"
    html_path.write_text(build_html(keys), encoding="utf-8")
    with tempfile.TemporaryDirectory(prefix="pdfrtl-chrome-") as profile_dir:
        render(chrome, html_path, combined, pathlib.Path(profile_dir))
    print(f"rendered {combined.relative_to(ROOT)} ({combined.stat().st_size} bytes)")

    # Guard against the class of bug this tool just had: distinct sentences must produce
    # distinct files, otherwise we are testing the same page five times.
    sizes = {p.name: p.stat().st_size for p in OUT_DIR.glob("*.pdf") if p.name != "all-sets.pdf"}
    if len(set(sizes.values())) == 1 and len(sizes) > 1:
        raise SystemExit(f"all fixtures are byte-identical in size — generation is broken: {sizes}")
    print(f"producer used: {chrome}")
    print(f"sentences rendered: {rendered + 1} page-file(s) from {len(keys)} sentence(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
