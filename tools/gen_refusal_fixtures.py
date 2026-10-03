#!/usr/bin/env python3
"""Add refusal-path fixtures: files where rung 3 MUST decline.

Why this exists (docs/problems/0017): measured 2026-10-03, `unproven_lines` is 0 on all 14
tracked fixtures. The refusal path — the whole reason this project exists — has ZERO public
coverage. Every tracked fixture carries per-cluster `/ActualText` or `/ReversedChars`, so it is
decided at rung 1 or 2 and never reaches the rung that refuses.

What a withholding fixture needs, and why each part is load-bearing:

  * real glyph codes + a `/ToUnicode` CMap    -> the text DECODES. A file that cannot decode
                                                tests the wrong branch.
  * NO `/ActualText`                          -> removes rung 1. This is what makes it a test.
  * NO `/ReversedChars`                        -> removes rung 2.
  * every unit at ONE text origin             -> every unit TIES on paint_x. A tie is not a
                                                measurement (docs/problems/0005), so rung 3
                                                cannot decide and must fall through.
  * a `/Producer` on NO allow-list             -> no fingerprint to fall back to, so the honest
                                                answer is the refusal.

Crucially, the EXPECTED VALUE of these fixtures is a REFUSAL, not a string. That makes them the
cheapest kind of RTL fixture: asserting "the tool declines" needs no native-speaker oracle and no
answer key, which is why they can be published while customer documents cannot
(`docs/problems/0006`: a gate may not pass vacuously — so each fixture asserts it really does
withhold, before anything asserts that it must).

Contents are invented-but-common Persian words with no digits and no Latin, so visual order is
exactly the character reversal (UAX #9 L2) and no fixture smuggles a mixed-line problem into a
file whose stated purpose is the tie.
"""
from __future__ import annotations

import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
OUT = ROOT / "corpus" / "raw" / "synthetic"


def utf16be_hex(text: str) -> bytes:
    return (b"\xfe\xff" + text.encode("utf-16-be")).hex().upper().encode("ascii")


def tounicode_cmap(codepoints: list[str]) -> bytes:
    lines = [
        "/CIDInit /ProcSet findresource begin",
        "12 dict begin",
        "begincmap",
        "/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def",
        "/CMapName /Adobe-Identity-UCS def",
        "/CMapType 2 def",
        "1 begincodespacerange",
        "<0000> <FFFF>",
        "endcodespacerange",
        f"{len(codepoints)} beginbfchar",
    ]
    for index, ch in enumerate(codepoints, start=1):
        lines.append(f"<{index:04X}> <{ord(ch):04X}>")
    lines += [
        "endbfchar",
        "endcmap",
        "CMapName currentdict /CMap defineresource pop",
        "end",
        "end",
    ]
    return ("\n".join(lines) + "\n").encode("ascii")


# --- the refusal fixtures -----------------------------------------------------
#
# `logical` is what a correct extractor WOULD return. The generator stores its
# reversal (what a painter lays down), so the file is genuinely visual-order. The expected
# result is still a REFUSAL, because every unit ties and nothing in the file can settle it.
#
# Each entry gets its own file so one case cannot mask another, and so a future failure names the
# case. `producer` is deliberately a name that is NOT in PRODUCER_ALLOW_LIST.

REFUSAL_CASES: dict[str, dict] = {
    # Pure RTL, one unit per character, all at one origin. The simplest form of the tie.
    "refuse-tied-rtl": {
        "lines": ["سلام دنیا", "کتاب خوب"],
        "producer": "pdfrtl-gen",
        "title": "pdfrtl refusal fixture: tied RTL units, visual order",
    },
    # Two lines on ONE origin rather than one per line: more units tied together, so a
    # hypothetical fix that only handles single-line pages still fails here.
    "refuse-tied-multiline": {
        "lines": ["گزارش روز", "صفحه آخر"],
        "producer": "pdfrtl-gen",
        "title": "pdfrtl refusal fixture: two tied lines at one origin",
    },
    # A producer string that LOOKS like an allow-listed one but is not it. Guards against a
    # fix that matches on a prefix (`Skia/PDF m1`) instead of the exact measured family.
    "refuse-near-miss-producer": {
        "lines": ["متن آزمایشی"],
        "producer": "pdfrtl-gen (NOT an allow-listed producer)",
        "title": "pdfrtl refusal fixture: producer must not match",
    },
}


def code_table(lines: list[str]) -> dict[str, int]:
    """One code per distinct character, by first appearance in LOGICAL order.

    Shared by the content stream and the CMap — they must agree, or the fixture decodes to the
    wrong text (docs/problems/0003: a gate that cannot see its own input).
    """
    table: dict[str, int] = {}
    for line in lines:
        for ch in line:
            table.setdefault(ch, len(table) + 1)
    return table


def tied_content(lines: list[str], mirrored: bool) -> tuple[bytes, list[str]]:
    """Real codes, VISUAL order, every unit on ONE text origin.

    The pen is never advanced: there is no `/Widths` array and no font descriptor, so every unit
    keeps the origin the line matrix set. That is the tie under test.

    One `Tj` per character keeps each glyph a separate unit. A single multi-code `Tj` would be
    one unit, and one unit cannot tie with anything — it would decide trivially and the fixture
    would stop testing the branch.
    """
    codes = code_table(lines)
    ops = ["BT", "/F1 12 Tf"]
    # `a <= 0` mirrors the line: x then orders glyphs opposite to how they paint, which is the
    # ordinary RTL case and must not be mistaken for a missing measurement
    # (rtl-pdf-text-pipeline skill, "a mirrored text matrix is the ordinary RTL case").
    matrix = "-1 0 0 -1" if mirrored else "1 0 0 -1"
    ops.append(f"{matrix} 20 80 Tm")
    for line in lines:
        for ch in reversed(line):
            ops.append(f"<{codes[ch]:04X}> Tj")
    ops.append("ET")
    return ("\n".join(ops) + "\n").encode("ascii"), list(codes)


def build(objects: list[bytes]) -> bytes:
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
        f"trailer\n<< /Size {size} /Root 1 0 R /Info {size - 1} 0 R >>\n"
        f"startxref\n{xref_at}\n%%EOF\n"
    ).encode("ascii")
    return bytes(out)


def page_dict(media_h: int = 140) -> bytes:
    return (
        f"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 {media_h}] "
        f"/Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>"
    ).encode("ascii")


def stream_obj(content: bytes) -> bytes:
    return b"<< /Length " + str(len(content)).encode("ascii") + b" >>\nstream\n" + content + b"endstream"


def build_refusal(name: str, spec: dict) -> bytes:
    content, chars = tied_content(spec["lines"], mirrored=True)
    return build(
        [
            b"<< /Type /Catalog /Pages 2 0 R >>",
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            page_dict(),
            stream_obj(content),
            # No /Widths and no /DescendantFonts: the pen cannot advance, so every unit ties.
            b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /ToUnicode 6 0 R >>",
            stream_obj(tounicode_cmap(chars)),
            f"<< /Producer ({spec['producer']}) /Creator ({spec['producer']}) "
            f"/Title ({spec['title']}) >>".encode("latin-1"),
        ]
    )


FIXTURES = {
    name: (
        (lambda spec=spec, n=name: build_refusal(n, spec)),
        f"corpus/raw/synthetic/{name}.pdf",
    )
    for name, spec in REFUSAL_CASES.items()
}


def main() -> int:
    args = sys.argv[1:]
    if args and args[0] in ("-h", "--help"):
        print(__doc__)
        return 0
    names = args or list(FIXTURES)
    unknown = [n for n in names if n not in FIXTURES]
    if unknown:
        print(f"unknown fixture(s): {unknown}; have {list(FIXTURES)}", file=sys.stderr)
        return 2
    for name in names:
        factory, rel = FIXTURES[name]
        data = factory()
        dest = ROOT / rel
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_bytes(data)
        print(f"wrote {rel} ({len(data)} bytes)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())