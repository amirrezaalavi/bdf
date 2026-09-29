#!/usr/bin/env python3
"""Emit deterministic PDFs whose RTL text lives in /ActualText marked content.

Why these fixtures matter: they are the *best case* for extraction. The visual glyphs are
dummy Latin (a Type1 font cannot show Persian/Arabic/Hebrew), but ISO 32000-1 §14.9.4 says
/ActualText carries the exact replacement text a consumer should use. So:

  * a correct extractor must return the RTL strings, not the dots;
  * an extractor that only reads glyph->Unicode mapping and reverses runs is exposed;
  * the fa fixture exercises the lam-alef (U+0644 U+0627) and ZWNJ (U+200C) end to end.

Why three languages: `crates/pdfrtl-core/tests/no_silent_reversal.rs` refuses to be
vacuous — it must run real fa/ar/he cases. The Arabic and Hebrew private archive files
are gitignored, so without public ar/he fixtures that control can never be satisfied on
the published mirror. actualtext-ar.pdf and actualtext-he.pdf are that public coverage.

Independent oracle check (expected): `pdftotext` honours /ActualText, so it should print
the same RTL text. If it does not, the fixture is wrong, not the oracle.

Usage:
    python tools/gen_actualtext_fixture.py [lang] [output.pdf]

    lang    one of fa/ar/he, default fa (so the historical no-argument invocation still
            regenerates corpus/raw/synthetic/actualtext-fa.pdf)
    output  default corpus/raw/synthetic/actualtext-<lang>.pdf

Determinism: no timestamps, no random ids — two runs are byte-identical
(corpus/AGENT.md rule 4), which `sha256sum` verifies.
"""
from __future__ import annotations

import pathlib
import sys

# One entry per language: one line of text per /ActualText run, written in logical order.
#
# fa sentence 1: lam-alef initial cluster (س ل ا م) — the ligature that degrades in
#                ToUnicode.
# fa sentence 2: ZWNJ between mi and ravam — semantically meaningful in Persian, must
#                survive extraction verbatim even though search normalisation strips it.
# ar sentence 1: السلام — lam-alef inside the word; ar sentence 2: a plain greeting.
# he             niqqud-free words of FOUR letters or more: the no-silent-reversal control
#                rejects three-letter words because a boundary match can still be a
#                coincidence (דוח matches inside חודש).
#
# Words asserted by the no-silent-reversal control are pure RTL (no Latin, no digits, no
# ZWNJ, no niqqud) and at least four letters: fa دنیا · ar السلام, مرحبا ·
# he שלום, עולם, תודה.
FIXTURES: dict[str, list[str]] = {
    "fa": ["سلام دنیا", "می\u200cروم"],
    "ar": ["السلام عليكم", "مرحبا بالعالم"],
    "he": ["שלום עולם", "תודה רבה על העזרה"],
}
DEFAULT_LANG = "fa"

PLACEHOLDER = "."  # Helvetica renders Latin; real glyphs are irrelevant to this fixture


def utf16be_hex(text: str) -> bytes:
    """PDF hex string: UTF-16BE bytes preceded by the BOM (PDF 32000-1 §7.9.2.2)."""
    payload = b"\xfe\xff" + text.encode("utf-16-be")
    return payload.hex().upper().encode("ascii")


def content_stream(sentences: list[str]) -> bytes:
    parts = ["BT /F1 12 Tf 20 80 Td"]
    for index, sentence in enumerate(sentences):
        if index:
            parts.append("0 -24 Td")
        parts.append(
            "/Span << /ActualText <" + utf16be_hex(sentence).decode("ascii") + "> >> BDC "
            + "(" + PLACEHOLDER * len(sentence) + ") Tj EMC"
        )
    parts.append("ET")
    return "\n".join(parts).encode("ascii")


def build(lang: str) -> bytes:
    content = content_stream(FIXTURES[lang])
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
    args = sys.argv[1:]
    if args and args[0] in ("-h", "--help"):
        print(__doc__)
        return 0

    # First positional argument: a language key (fa/ar/he) or — as before — an output
    # path; a second one is always the output path.
    lang = DEFAULT_LANG
    out = None
    if args and args[0] in FIXTURES:
        lang = args.pop(0)
    if args:
        out = args.pop(0)
    if args or (out is not None and not out.endswith(".pdf")):
        print(
            f"usage: {sys.argv[0]} [lang] [output.pdf]  (lang: {', '.join(FIXTURES)})",
            file=sys.stderr,
        )
        return 2

    dest = pathlib.Path(out) if out else pathlib.Path(
        f"corpus/raw/synthetic/actualtext-{lang}.pdf"
    )
    dest.parent.mkdir(parents=True, exist_ok=True)
    data = build(lang)
    dest.write_bytes(data)
    print(f"wrote {dest} ({len(data)} bytes, lang={lang})")
    print("expected logical text (one line per run):")
    for sentence in FIXTURES[lang]:
        print(f"  {sentence}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
