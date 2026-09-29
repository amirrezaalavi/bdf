#!/usr/bin/env python3
"""Emit deterministic PDFs for the order ladder's OTHER rungs (W1.7c, W1.7d).

The three `actualtext-*.pdf` fixtures (tools/gen_actualtext_fixture.py) prove rung 1:
the text lives in /ActualText, one marked sequence per line, so no order decision is
ever made. Two rungs were therefore unproven on the public corpus:

  * rung 2 — `/ReversedChars` + real glyph codes. `reversed-fa.pdf` below has NO
    /ActualText at all: the characters come from a ToUnicode CMap on real codes, and
    the only order evidence in the file is the marker. Removing the marker from this
    file must change the answer — that is the inversion axis (docs/problems/0005).
  * the producer fingerprint (ADR 0004, the last rung). `family-*.pdf` below are one
    fixture PER ALLOW-LIST FAMILY in `pdfrtl_core::text::recover::PRODUCER_ALLOW_LIST`.
    Each carries that family's `/Producer` string and stores its text in that family's
    measured convention; `crates/pdfrtl-core/tests/producer_allowlist.rs` enumerates
    the allow-list and fails when a family has no fixture, or a fixture is missing.

Honesty rules these fixtures keep:
  * `producer_tool` says `simulated` — the /Producer string is the real tool's, the
    bytes are OURS. Nothing here claims Microsoft/Adobe/Skia wrote the file; the
    private archive is what measured the real producers (Q-R3).
  * one axis per fixture (corpus/AGENT.md rule 5): reversed-fa varies only the order
    signal; the family fixtures vary only which /Producer convention is declared.
  * determinism: no timestamps, no random ids — two runs are byte-identical
    (corpus/AGENT.md rule 4), which `sha256sum` verifies.

Usage:
    python tools/gen_order_fixtures.py                 # all four
    python tools/gen_order_fixtures.py reversed-fa      # just one
"""
from __future__ import annotations

import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
OUT = ROOT / "corpus" / "raw" / "synthetic"

# --- rung 2: /ReversedChars + real glyph codes -------------------------------
#
# Logical text (what a correct extractor must return), one line per /ReversedChars
# run. The generator writes the VISUAL order (reverse of logical — pure RTL, no
# Latin, no digits, so visual == character-reversed), one code per glyph.
REVERSED_FA_LINES = ["سلام دنیا", "گزارش سالانه"]

# --- the producer allow-list, one fixture per family -------------------------
#
# One line per fixture entry: the string exactly as the file STORES it, one
# /ActualText unit per character (a unit is then always its own logical text — the
# way a real producer writes clusters: logical inside a cluster, the cluster ORDER
# is what the convention is about). Every fixture also carries one line the UAX #9
# rung cannot settle on its own, so the fingerprint rung itself stays exercised.
#
def rtl_painted(logical: str) -> str:
    """The sequence a renderer paints for a pure-RTL line: reversed (UAX #9 L2).

    Deriving the stored text from the LOGICAL expectation instead of typing it is
    deliberate — a hand-typed mirror is one typo away from a fixture that asserts
    the wrong word (docs/problems/0003: an input the control cannot really see).
    """
    return logical[::-1]


def rtl_then_ltr_painted(logical: str) -> str:
    """Painted order of `"<rtl words> <latin>"`: the LTR run keeps its reading order
    and moves to the left end (level 2), the RTL part is painted reversed."""
    rtl, ltr = logical.rsplit(" ", 1)
    return f"{ltr} {rtl[::-1]}"


# Each line: (stored text, mirrored_text_matrix). One unit per character, so a unit
# is always its own logical text — the way a real producer writes clusters (logical
# inside a cluster; the cluster ORDER is what the convention is about).
#
# word/InDesign line 1: pure RTL at one origin -> painted == stored -> rung 3
#   inverts it, from the file's own positions (no fingerprint involved).
# word/InDesign line 2: the text matrix mirrors the line (`a <= 0`), so x does not
#   order the glyphs the way they paint and rung 3 refuses to read it. The line
#   falls through to the producer fingerprint — which is what these fixtures are
#   here to exercise: every allow-list entry needs a fixture that runs it.
FAMILY_WORD_VISUAL_LINES = [
    (rtl_painted("گزارش سالانه"), False),
    (rtl_then_ltr_painted("دنیا pdfrtl"), True),
]
FAMILY_INDESIGN_VISUAL_LINES = [
    (rtl_painted("مرحبا بالعالم"), False),
    (rtl_then_ltr_painted("منظومة pdfrtl"), True),
]
# Skia: stored LOGICAL, one origin — the painting fits neither UAX #9 reading, so
# rung 3 declines on its own and the fingerprint's LOGICAL branch must keep the text
# byte-exact. This is the branch that stops us from inverting Chromium files.
FAMILY_CHROME_LOGICAL_LINES = [
    ("گزارش pdfrtl", False),
]


def utf16be_hex(text: str) -> bytes:
    """PDF hex string: UTF-16BE bytes preceded by the BOM (ISO 32000-1 §7.9.2.2)."""
    return (b"\xfe\xff" + text.encode("utf-16-be")).hex().upper().encode("ascii")


def tounicode_cmap(codepoints: list[str]) -> bytes:
    """A one-code-per-character ToUnicode CMap, entries in first-appearance order."""
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


def code_table() -> dict[str, int]:
    """One code per distinct character, by first appearance in LOGICAL order.

    Shared by the content stream and the CMap — they must agree or the fixture
    decodes to the wrong text (docs/problems/0003: a gate that cannot see its input).
    """
    table: dict[str, int] = {}
    for line in REVERSED_FA_LINES:
        for ch in line:
            table.setdefault(ch, len(table) + 1)
    return table


def reversed_fa_content() -> tuple[bytes, list[str]]:
    """`/ReversedChars` around real codes: glyph order in the stream is VISUAL."""
    codes = code_table()
    ops = ["BT", "/F1 12 Tf", "20 80 Td"]
    for line_index, line in enumerate(REVERSED_FA_LINES):
        if line_index:
            ops.append("0 -24 Td")
        ops.append("/ReversedChars BMC")
        # One hex STRING of all the codes: `<0001> <0002> Tj` would be two operands
        # (Tj takes one string), and the second would be silently never shown.
        shown = "".join(f"{codes[ch]:04X}" for ch in reversed(line))
        ops.append(f"<{shown}> Tj")
        ops.append("EMC")
    ops.append("ET")
    return ("\n".join(ops) + "\n").encode("ascii"), list(codes)


def actualtext_content(lines: list[tuple[str, bool]]) -> bytes:
    """One `/ActualText` sequence per unit, one text matrix per line.

    Several units per line on ONE origin: the painted order is then the stream order,
    which is exactly the evidence the ladder's last two rungs argue over. Spaces are
    their own units so the extracted text keeps its word boundaries (the
    no-silent-reversal control counts words at boundaries, never substrings).
    """
    ops = ["BT", "/F1 12 Tf"]
    for index, (line, mirrored) in enumerate(lines):
        # `a <= 0` mirrors the line: x then orders the glyphs opposite to the way
        # they paint, and rung 3 declines to read positions it cannot trust.
        matrix = "-1 0 0 -1" if mirrored else "1 0 0 -1"
        ops.append(f"{matrix} 20 {80 - 24 * index} Tm")
        # One unit per character (spaces included): a unit is then trivially its own
        # logical text, and the char-level and unit-level inversions agree.
        for unit in line:
            ops.append(
                "/Span <</ActualText <"
                + utf16be_hex(unit).decode("ascii")
                + "> >> BDC (.) Tj EMC"
            )
    ops.append("ET")
    return ("\n".join(ops) + "\n").encode("ascii")


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


def page_dict() -> bytes:
    return (
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 140] "
        b"/Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>"
    )


def stream_obj(content: bytes) -> bytes:
    return b"<< /Length " + str(len(content)).encode("ascii") + b" >>\nstream\n" + content + b"endstream"


def info_obj(producer: str, title: str) -> bytes:
    return f"<< /Producer ({producer}) /Creator ({producer}) /Title ({title}) >>".encode("latin-1")


def build_reversed_fa() -> bytes:
    content, _ = reversed_fa_content()
    return build(
        [
            b"<< /Type /Catalog /Pages 2 0 R >>",
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            page_dict(),
            stream_obj(content),
            b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica "
            b"/ToUnicode 6 0 R >>",
            stream_obj(tounicode_cmap(list(code_table()))),
            info_obj("pdfrtl-gen", "pdfrtl ReversedChars fixture"),
        ]
    )


def build_family(name: str, producer: str, lines: list[tuple[str, bool]]) -> bytes:
    return build(
        [
            b"<< /Type /Catalog /Pages 2 0 R >>",
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            page_dict(),
            stream_obj(actualtext_content(lines)),
            b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
            info_obj(producer, f"pdfrtl {name} fixture"),
        ]
    )


FIXTURES = {
    "reversed-fa": (build_reversed_fa, "corpus/raw/synthetic/reversed-fa.pdf"),
    "family-word-visual": (
        lambda: build_family(
            "family-word-visual", "Microsoft Word 2019", FAMILY_WORD_VISUAL_LINES
        ),
        "corpus/raw/synthetic/family-word-visual.pdf",
    ),
    "family-indesign-visual": (
        lambda: build_family(
            "family-indesign-visual", "Adobe InDesign 19.4", FAMILY_INDESIGN_VISUAL_LINES
        ),
        "corpus/raw/synthetic/family-indesign-visual.pdf",
    ),
    "family-chrome-logical": (
        lambda: build_family(
            "family-chrome-logical", "Skia/PDF m154", FAMILY_CHROME_LOGICAL_LINES
        ),
        "corpus/raw/synthetic/family-chrome-logical.pdf",
    ),
}


def main() -> int:
    args = sys.argv[1:]
    if args and args[0] in ("-h", "--help"):
        print(__doc__)
        return 0
    names = args or list(FIXTURES)
    unknown = [name for name in names if name not in FIXTURES]
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
