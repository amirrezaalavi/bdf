#!/usr/bin/env python3
"""Debug aid: dump readable text operators out of a PDF, including marked content.

This is NOT a parser and must never be used as ground truth — it inflates streams and
regex-scans them, which is exactly what you want while investigating "what did this
producer actually write?" and exactly what you must not ship in the product.

What it prints per stream:
  * text-showing operators with their operands (`Tj`, `TJ`, `'`, `"`),
  * marked-content operators (`BDC`/`BMC`/`EMC`) with their property lists, so you can see
    `/ActualText`, `/ReversedChars`, `/Lang`,
  * a hex preview of the fonts and a warning if a `ToUnicode` CMap is absent.

Usage:
    python tools/dump_streams.py file.pdf            # summary
    python tools/dump_streams.py file.pdf --raw      # full inflated stream text
    python tools/dump_streams.py file.pdf --ops      # only text/marked-content lines
"""
from __future__ import annotations

import argparse
import pathlib
import re
import sys
import zlib

STREAM_RE = re.compile(rb"stream\r?\n(.*?)\r?\nendstream", re.DOTALL)
OPS = (b" Tj", b" TJ", b" '", b' "', b"BDC", b"BMC", b"EMC", b"BT", b"ET", b"Tf", b"Td", b"TD")
# /ActualText followed by a hex string <…> or a literal string (…)
ACTUAL_TEXT_RE = re.compile(rb"/ActualText\s*(<[0-9A-Fa-f\s]*>|\((?:[^()\\]|\\.)*\))")


def decode_pdf_string(token: bytes) -> str:
    """Decode a PDF string operand: hex string or literal string (PDF 32000-1 §7.9.2)."""
    if token.startswith(b"<"):
        raw = bytes.fromhex(re.sub(rb"\s", b"", token[1:-1]).decode("ascii"))
    else:
        body = token[1:-1]
        body = re.sub(rb"\\([nrtbf()\\])", lambda m: {
            b"n": b"\n", b"r": b"\r", b"t": b"\t", b"b": b"\b", b"f": b"\f",
        }.get(m.group(1), m.group(1)), body)
        raw = body
    if raw.startswith(b"\xfe\xff"):
        return raw[2:].decode("utf-16-be", "replace")
    if raw.startswith(b"\xff\xfe"):
        return raw[2:].decode("utf-16-le", "replace")
    # Without a BOM the spec says text strings use PDFDocEncoding; UTF-8 is the practical
    # guess producers make. Try UTF-8, fall back to latin-1 losslessly.
    try:
        return raw.decode("utf-8")
    except UnicodeDecodeError:
        return raw.decode("latin-1")



def inflate(blob: bytes) -> bytes:
    try:
        return zlib.decompress(blob)
    except zlib.error:
        return blob


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("pdf")
    parser.add_argument("--raw", action="store_true")
    parser.add_argument("--ops", action="store_true")
    parser.add_argument("--actual-text", action="store_true",
                        help="decode every /ActualText value in document order, then stop")
    args = parser.parse_args()

    data = pathlib.Path(args.pdf).read_bytes()
    print(f"file: {args.pdf} ({len(data)} bytes)")

    # Object-level facts that matter for RTL text handling.
    for needle, label in [
        (b"/ActualText", "ActualText occurrences"),
        (b"/ReversedChars", "ReversedChars occurrences"),
        (b"/ToUnicode", "ToUnicode references"),
        (b"/Type0", "Type0 (composite) fonts"),
        (b"/Identity-H", "Identity-H encodings"),
        (b"/Subtype /TrueType", "TrueType fonts"),
        (b"/Subtype /Type1", "Type1 fonts"),
    ]:
        count = data.count(needle)
        print(f"  {label:28} {count}")

    streams = STREAM_RE.findall(data)
    print(f"  {'streams found':28} {len(streams)}")

    for index, blob in enumerate(streams):
        text = inflate(blob)

        if args.actual_text:
            values = [decode_pdf_string(m.group(1)) for m in ACTUAL_TEXT_RE.finditer(text)]
            if values:
                print(f"\n--- stream #{index}: {len(values)} /ActualText value(s) ---")
                for position, value in enumerate(values[:120]):
                    print(f"   [{position:>3}] {value!r}")
                if len(values) > 120:
                    print(f"   … {len(values) - 120} more")
                joined = "".join(values)
                print(f"   concatenated ({len(values)} values): {joined!r}")
            continue

        printable_ratio = sum(32 <= b < 127 or b in (9, 10, 13) for b in text[:4000]) / max(1, len(text[:4000]))
        if printable_ratio < 0.5:
            continue  # binary (font file, image)

        lines = text.decode("latin-1").splitlines()
        interesting = [ln for ln in lines if any(op.decode("latin-1") in ln for op in OPS)]
        has_actual_text = "/ActualText" in text.decode("latin-1", "replace")

        print(f"\n--- stream #{index} ({len(blob)} bytes raw, {len(text)} inflated) "
              f"actual_text={has_actual_text} ---")
        if args.raw:
            print(text.decode("latin-1"))
        elif args.ops or not interesting:
            for line in (interesting or lines)[:40]:
                print("   ", line[:300])
            if len(interesting) > 40:
                print(f"    … {len(interesting) - 40} more operator lines")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
