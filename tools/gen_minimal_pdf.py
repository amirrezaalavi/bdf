#!/usr/bin/env python3
"""Emit a minimal, valid, deterministic 1-page PDF. No third-party deps, no timestamps.

Why hand-rolled instead of a library: this fixture must be byte-identical on every
machine and every run (determinism rule), so it cannot depend on a PDF library's
version-specific output.

Usage:
    python tools/gen_minimal_pdf.py [output.pdf]

Default output: corpus/raw/synthetic/minimal-ltr.pdf
Verify with an independent parser, e.g.:
    pdftotext corpus/raw/synthetic/minimal-ltr.pdf -     # must print: Hello pdfrtl
"""
from __future__ import annotations

import pathlib
import sys

TEXT = "Hello pdfrtl"
PRODUCER = "pdfrtl-gen"


def build() -> bytes:
    content = f"BT /F1 12 Tf 20 40 Td ({TEXT}) Tj ET".encode("ascii")

    # Object 1..6. /Info is object 6 and is emitted *before* the xref table so the
    # trailer reference is valid for strict parsers.
    objects: list[bytes] = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 80] "
        b"/Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>",
        b"<< /Length "
        + str(len(content)).encode("ascii")
        + b" >>\nstream\n"
        + content
        + b"\nendstream",
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
        b"<< /Producer ("
        + PRODUCER.encode("ascii")
        + b") /Creator ("
        + PRODUCER.encode("ascii")
        + b") >>",
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
        sys.argv[1] if len(sys.argv) > 1 else "corpus/raw/synthetic/minimal-ltr.pdf"
    )
    dest.parent.mkdir(parents=True, exist_ok=True)
    data = build()
    dest.write_bytes(data)
    print(f"wrote {dest} ({len(data)} bytes)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
