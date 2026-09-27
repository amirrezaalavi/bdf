#!/usr/bin/env python3
"""Emit a deterministic PDF whose RTL text lives in /ActualText marked content.

Why this fixture matters: it is the *best case* for extraction. The visual glyphs are
dummy Latin (a Type1 font cannot show Persian), but ISO 32000-1 §14.9.4 says /ActualText
carries the exact replacement text a consumer should use. So:

  * a correct extractor must return the Persian strings, not the dots;
  * an extractor that only reads glyph->Unicode mapping and reverses runs is exposed;
  * it exercises the lam-alef (U+0644 U+0627) and ZWNJ (U+200C) cases end to end.

Independent oracle check (expected): `pdftotext` honours /ActualText, so it should print
the same Persian text. If it does not, the fixture is wrong, not the oracle.

Usage:
    python tools/gen_actualtext_fixture.py [output.pdf]

Default output: corpus/raw/synthetic/actualtext-fa.pdf
"""
from __future__ import annotations

import pathlib
import sys

# Sentence 1: lam-alef initial cluster (س ل ا م) — the ligature that degrades in ToUnicode.
# Sentence 2: ZWNJ between mi and ravam — semantically meaningful in Persian, must survive
#             extraction verbatim even though search normalisation strips it.
SENTENCES = ["سلام دنیا", "می\u200cروم"]

PLACEHOLDER = "."  # Helvetica renders Latin; real glyphs are irrelevant to this fixture


def utf16be_hex(text: str) -> bytes:
    """PDF hex string: UTF-16BE bytes preceded by the BOM (PDF 32000-1 §7.9.2.2)."""
    payload = b"\xfe\xff" + text.encode("utf-16-be")
    return payload.hex().upper().encode("ascii")


def content_stream() -> bytes:
    parts = ["BT /F1 12 Tf 20 80 Td"]
    for index, sentence in enumerate(SENTENCES):
        if index:
            parts.append("0 -24 Td")
        parts.append(
            "/Span << /ActualText <" + utf16be_hex(sentence).decode("ascii") + "> >> BDC "
            + "(" + PLACEHOLDER * len(sentence) + ") Tj EMC"
        )
    parts.append("ET")
    return "\n".join(parts).encode("ascii")


def build() -> bytes:
    content = content_stream()
    objects: list[bytes] = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 120] "
        b"/Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>",
        b"<< /Length "
        + str(len(content)).encode("ascii")
        + b" >>\nstream\n"
        + content
        + b"\nendstream",
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
        b"<< /Producer (pdfrtl-gen) /Creator (pdfrtl-gen) "
        b"/Title (pdfrtl actualtext fixture) >>",
    ]

    out = bytearray(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n")
    offsets: list[int] = []
    for number, body in enumerate(objects, start=1):
        offsets.append(len(out))
        out += f"{number} 0 obj\n".encode("ascii") + body + b"\nendobj\n"

    xref_at = len(out)
    size = len(objects) + 1
    out += f"xref\n0 {size}\n".encode("ascii") + b"0000000000 65535 f \n"
    for offset in offsets:
        out += f"{offset:010d} 00000 n \n".encode("ascii")
    out += (
        f"trailer\n<< /Size {size} /Root 1 0 R /Info 6 0 R >>\n"
        f"startxref\n{xref_at}\n%%EOF\n"
    ).encode("ascii")
    return bytes(out)


def main() -> int:
    dest = pathlib.Path(
        sys.argv[1] if len(sys.argv) > 1 else "corpus/raw/synthetic/actualtext-fa.pdf"
    )
    dest.parent.mkdir(parents=True, exist_ok=True)
    data = build()
    dest.write_bytes(data)
    print(f"wrote {dest} ({len(data)} bytes)")
    print("expected logical text (one line per run):")
    for sentence in SENTENCES:
        print(f"  {sentence}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
