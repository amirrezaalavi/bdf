#!/usr/bin/env python3
"""Generate `crates/pdfrtl-core/src/text/tables.rs` — the checked-in Unicode tables
for simple 8-bit PDF font encodings (ADR 0004's "compact checked-in table").

Why a generated file instead of a crate dependency: the project forbids new runtime
dependencies for this (licensing + footprint, ADR 0003), and the data we need is
tiny and stable. The provenance of every byte is recorded in the generated header:

  * Adobe Glyph List (glyph name -> Unicode):
        https://raw.githubusercontent.com/adobe-type-tools/agl-aglfn/
        4036a9ca80a62f64f9de4f7321a9a045ad0ecfd6/glyphlist.txt
    pinned commit + sha256 asserted below (the file is Adobe's, BSD-3-Clause).
  * /WinAnsiEncoding : Python's `cp1252` codec (identical byte map to ISO 32000-1
    Table 128; the five ISO-undefined positions 0x81 0x8D 0x8F 0x90 0x9D are
    exactly the ones cp1252 refuses).
  * /MacRomanEncoding: Python's `mac_roman` codec (Mac OS Roman), with 0xDB pinned
    to the spec's `currency` instead of Apple's later euro (see below).
  * /StandardEncoding: Apache PDFBox `encoding/StandardEncoding.java` (tag 3.0.5)
    resolved through the Adobe glyph list — PDFBox and pdf.js agree on every code
    that table defines; pypdf additionally fills positions the spec leaves undefined
    and is therefore only a reported cross-check, never the source.
  * /PDFDocEncoding: pypdf 6.x `_codecs/pdfdoc.py` (PDF Reference 1.7 Table D.2).

Cross-checks before emitting (any mismatch aborts):
  * WinAnsi and MacRoman are compared code-for-code against Apache PDFBox's
    independently transcribed `encoding/*.java` name tables (tag 3.0.5), resolved
    through the same Adobe glyph list;
  * Standard is built from that PDFBox table and diffed against pypdf's
    transcription (differences reported);
  * a handful of well-known positions are asserted with literal expectations.

Usage (from the repo root, with the repo's .venv active — it provides pypdf):
    .venv/Scripts/python tools/gen-encoding-tables.py
    python3 tools/gen-encoding-tables.py --glyphlist /path/to/glyphlist.txt
"""
from __future__ import annotations

import argparse
import codecs
import hashlib
import pathlib
import re
import sys
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parent.parent
OUT = ROOT / "crates" / "pdfrtl-core" / "src" / "text" / "tables.rs"

GLYPHLIST_URL = (
    "https://raw.githubusercontent.com/adobe-type-tools/agl-aglfn/"
    "4036a9ca80a62f64f9de4f7321a9a045ad0ecfd6/glyphlist.txt"
)
GLYPHLIST_SHA256 = "a3b2f61ced9f3644cc0d4ecde5c59df34ca286c689d9484a43a710a81c466789"
PDFBOX_REF = "3.0.5"
PDFBOX_URL = ("https://raw.githubusercontent.com/apache/pdfbox/"
              f"{PDFBOX_REF}/pdfbox/src/main/java/org/apache/pdfbox/pdmodel/font/encoding/")
PDFBOX_TABLES = ("WinAnsiEncoding", "MacRomanEncoding", "StandardEncoding")

UNDEF = 0xFFFF


# --- data sources -----------------------------------------------------------

def load_glyphlist(path: pathlib.Path | None) -> dict[str, str]:
    if path is None:
        raw = urllib.request.urlopen(GLYPHLIST_URL, timeout=60).read()
    else:
        raw = path.read_bytes()
    digest = hashlib.sha256(raw).hexdigest()
    if digest != GLYPHLIST_SHA256:
        raise SystemExit(f"glyphlist sha256 mismatch: {digest} != {GLYPHLIST_SHA256}")
    table: dict[str, str] = {}
    for line in raw.decode("utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        name, sep, value = line.partition(";")
        if not sep or not value:
            continue
        text = utf16be_hex_to_str(value)
        if text is None:
            print(f"  ! dropped {name};{value} (not decodable UTF-16)", file=sys.stderr)
            continue
        table.setdefault(name, text)
    if len(table) < 4000:
        raise SystemExit(f"glyphlist looks truncated: {len(table)} entries")
    return table


def utf16be_hex_to_str(value: str) -> str | None:
    # Adobe's glyphlist separates UTF-16 units with spaces ("05E8 05B0").
    units: list[int] = []
    for token in value.split():
        if len(token) != 4:
            return None
        units.append(int(token, 16))
    if not units:
        return None
    out: list[str] = []
    i = 0
    while i < len(units):
        unit = units[i]
        if 0xD800 <= unit <= 0xDBFF and i + 1 < len(units) and 0xDC00 <= units[i + 1] <= 0xDFFF:
            cp = 0x10000 + ((unit - 0xD800) << 10) + (units[i + 1] - 0xDC00)
            i += 2
        elif 0xD800 <= unit <= 0xDFFF:
            return None
        else:
            cp = unit
            i += 1
        if cp == 0:
            return None
        out.append(chr(cp))
    return "".join(out)


def scalar(code: int | None) -> int:
    """Map a code point to the table slot: control characters are not text."""
    if code is None:
        return UNDEF
    if 0x00 <= code <= 0x1F or 0x7F <= code <= 0x9F:
        return UNDEF
    if 0xD800 <= code <= 0xDFFF or code > 0x10FFFF:
        return UNDEF
    return code


def base_from_codec(name: str) -> list[int]:
    out = []
    for byte in range(256):
        try:
            text = bytes([byte]).decode(name)
        except UnicodeDecodeError:
            out.append(UNDEF)
            continue
        out.append(scalar(ord(text)) if len(text) == 1 else UNDEF)
    return out


def base_from_pypdf(encoding_list: list[str]) -> list[int]:
    if len(encoding_list) != 256:
        raise SystemExit(f"expected 256 entries, got {len(encoding_list)}")
    out = []
    for text in encoding_list:
        out.append(scalar(ord(text)) if len(text) == 1 else UNDEF)
    return out


def base_from_pdfbox(names: dict[int, str], glyphlist: dict[str, str]) -> list[int]:
    """Build a byte table from PDFBox's code -> glyph-name table through the AGL."""
    out = [UNDEF] * 256
    for code in range(256):
        name = names.get(code)
        if name is None:
            continue
        text = glyphlist.get(name)
        if text is None:
            raise SystemExit(f"PDFBox name {name!r} is not in the Adobe glyph list")
        out[code] = scalar(ord(text)) if len(text) == 1 else UNDEF
    return out


def secondary_check(label: str, mine: list[int], other: list[int],
                    other_name: str) -> None:
    """Informational comparison against a second implementation: differences are
    reported, never fatal (the primary source is documented in the file header)."""
    diffs = [(code, mine[code], other[code]) for code in range(256)
             if mine[code] != other[code]]
    print(f"  ~ {label}: {len(diffs)} codes differ from {other_name}")
    for code, a, b in diffs[:6]:
        print(f"      0x{code:02X} ours=U+{a:04X} {other_name}=U+{b:04X}", file=sys.stderr)


def load_pdfbox_tables(directory: pathlib.Path | None) -> dict[str, dict[int, str]]:
    """Apache PDFBox `encoding/*.java`: `{octal, "glyphname"}` pairs -> code -> name."""
    tables: dict[str, dict[int, str]] = {}
    for label in PDFBOX_TABLES:
        if directory is None:
            raw = urllib.request.urlopen(f"{PDFBOX_URL}{label}.java", timeout=60).read()
        else:
            raw = (directory / f"{label}.java").read_bytes()
        pairs = re.findall(r'\{(\d+),\s*"([^"]+)"\}', raw.decode("utf-8"))
        table: dict[int, str] = {}
        for octal, name in pairs:
            table.setdefault(int(octal, 8), name)
        if len(table) < 100:
            raise SystemExit(f"PDFBox {label} parsed to {len(table)} entries — format drift?")
        tables[label] = table
    return tables


# --- cross-checks -----------------------------------------------------------

def cross_check(label: str, mine: list[int], names: dict[int, str],
                glyphlist: dict[str, str]) -> None:
    """Every code PDFBox defines must resolve (via the Adobe glyph list) to exactly
    the scalar our table holds; codes neither source defines must be `UNDEF`."""
    checked = 0
    notes: list[str] = []
    for code in range(256):
        name = names.get(code)
        if name is None:
            if mine[code] != UNDEF:
                notes.append(f"0x{code:02X} defined by us (U+{mine[code]:04X}), "
                             f"absent from PDFBox")
            continue
        expected = glyphlist.get(name)
        if expected is None:
            notes.append(f"0x{code:02X} PDFBox name {name!r} not in the Adobe glyph list")
            continue
        if len(expected) != 1:
            notes.append(f"0x{code:02X} {name} resolves to {len(expected)} chars "
                         f"(we hold U+{mine[code]:04X})")
            continue
        want = scalar(ord(expected))
        if want != mine[code]:
            raise SystemExit(
                f"{label} mismatch at 0x{code:02X}: PDFBox {name} -> U+{want:04X}, "
                f"we produced U+{mine[code]:04X}"
            )
        checked += 1
    print(f"  ok {label}: {checked} codes agree with PDFBox {PDFBOX_REF}"
          + (f"; {len(notes)} noted" if notes else ""))
    for note in notes[:12]:
        print(f"    ~ {note}", file=sys.stderr)


def assert_positions(label: str, table: list[int], expected: dict[int, int]) -> None:
    for code, want in expected.items():
        got = table[code]
        if got != want:
            raise SystemExit(f"{label}[0x{code:02X}]: expected U+{want:04X}, got U+{got:04X}")
    print(f"  ok {label}: {len(expected)} hand-checked positions")


# --- emission ---------------------------------------------------------------

def emit(winansi: list[int], macroman: list[int], standard: list[int], pdfdoc: list[int],
         glyphlist: dict[str, str]) -> str:
    lines: list[str] = []
    add = lines.append
    add("//! GENERATED FILE — do not edit by hand.")
    add("//!")
    add("//! Simple-font Unicode tables for 8-bit PDF encodings, produced by")
    add("//! `tools/gen-encoding-tables.py` (run it to refresh):")
    add("//!")
    add("//! * `WINANSI`     — ISO 32000-1 Table 128 (= the `cp1252` byte map);")
    add("//! * `MACROMAN`     — Mac OS Roman (= the `mac_roman` byte map);")
    add("//! * `STANDARD`     — PostScript `StandardEncoding` (PDF Reference 1.7 App. D);")
    add(f"//! * `PDFDOC`      — `PDFDocEncoding` (PDF Reference 1.7 Table D.2);")
    add("//! * `AGL`         — Adobe Glyph List, glyph name -> Unicode text, sorted by name.")
    add("//!")
    add(f"//! Glyph list: adobe-type-tools/agl-aglfn @ 4036a9c, sha256 {GLYPHLIST_SHA256}.")
    add("//! Slot value `UNDEF` means \"this code has no Unicode mapping\": we refuse to")
    add("//! decode it rather than invent a character (ADR 0004).")
    add("")
    add("/// No Unicode mapping for this byte code (C0/C1 control or undefined slot).")
    add("pub(crate) const UNDEF: u16 = 0xFFFF;")
    add("")
    for const, table in (("WINANSI", winansi), ("MACROMAN", macroman),
                         ("STANDARD", standard), ("PDFDOC", pdfdoc)):
        add(f"/// Byte code -> Unicode scalar (`UNDEF` where the encoding defines nothing).")
        add(f"#[rustfmt::skip]")
        add(f"pub(crate) static {const}: [u16; 256] = [")
        for offset in range(0, 256, 8):
            chunk = ", ".join(f"0x{value:04X}" for value in table[offset:offset + 8])
            add(f"    {chunk},")
        add("];")
        add("")
    add("/// Adobe Glyph List: glyph name -> Unicode text, sorted by name for binary search.")
    add("#[rustfmt::skip]")
    add("pub(crate) static AGL: &[(&str, &str)] = &[")
    for name in sorted(glyphlist, key=lambda s: s.encode("utf-8")):
        text = glyphlist[name]
        add(f"    ({rust_str(name)}, {rust_str(text)}),")
    add("];")
    add("")
    return "\n".join(lines)


def rust_str(text: str) -> str:
    # Escape anything Unicode classifies as INVISIBLE, by category rather than by a
    # hand-written list. Two reasons, both learned the hard way:
    #   * rustc rejects literals that silently change text direction
    #     (`text_direction_codepoint_in_literal`);
    #   * clippy denies invisible characters in literals (`clippy::invisible_characters`),
    #     and U+00AD SOFT HYPHEN - a format character, category Cf - slipped through the
    #     previous hardcoded bidi range list and broke the build on the generated file.
    # Categories: Cc control, Cf format, Zl/Zp separators, Zs separators other than the
    # plain ASCII space. Combining marks (Mn) stay literal: they are visible modifiers,
    # and glyph names legitimately contain them (e.g. "shaddaarabic").
    import unicodedata

    out = ['"']
    for ch in text:
        cp = ord(ch)
        if ch in ('"', "\\"):
            out.append("\\" + ch)
            continue
        cat = unicodedata.category(ch)
        if cat in ("Cc", "Cf", "Zl", "Zp") or (cat == "Zs" and ch != " "):
            out.append("\\u{%04X}" % cp)
        else:
            out.append(ch)
    out.append('"')
    return "".join(out)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--glyphlist", type=pathlib.Path, default=None)
    parser.add_argument("--pdfbox", type=pathlib.Path, default=None,
                        help="directory holding PDFBox encoding/*.java (else fetched)")
    parser.add_argument("--out", type=pathlib.Path, default=OUT)
    args = parser.parse_args()

    try:
        import pypdf  # noqa: F401
        from pypdf._codecs.pdfdoc import _pdfdoc_encoding
        from pypdf._codecs.std import _std_encoding
    except ImportError:
        raise SystemExit("pypdf is required (run with the repo's .venv python)")

    print("loading sources…")
    glyphlist = load_glyphlist(args.glyphlist)
    print(f"  ok glyph list: {len(glyphlist)} names")

    winansi = base_from_codec("cp1252")
    macroman = base_from_codec("mac_roman")
    # The PDF spec's MacRomanEncoding keeps the classic Mac OS Roman slot at 0xDB
    # (currency sign); Apple's modern `mac_roman` codec re-pointed it at the euro.
    # PDFBox and pdf.js both say `currency`, so the spec wins over the codec.
    macroman[0xDB] = 0x00A4
    pdfdoc = base_from_pypdf(_pdfdoc_encoding)

    print("cross-checking against Apache PDFBox encoding tables…")
    pdfbox = load_pdfbox_tables(args.pdfbox)
    standard = base_from_pdfbox(pdfbox["StandardEncoding"], glyphlist)
    cross_check("WinAnsiEncoding", winansi, pdfbox["WinAnsiEncoding"], glyphlist)
    cross_check("MacRomanEncoding", macroman, pdfbox["MacRomanEncoding"], glyphlist)
    # StandardEncoding is *built* from PDFBox (two independent sources — PDFBox and
    # pdf.js — agree on every code it defines); pypdf fills positions the spec leaves
    # undefined, so it is only reported, never required to agree.
    secondary_check("StandardEncoding", standard, base_from_pypdf(_std_encoding),
                    "pypdf")

    print("hand-checked positions…")
    assert_positions("WINANSI", winansi, {
        0x20: 0x0020,   # space
        0x27: 0x0027,   # quotesingle (NOT U+2019 — that is StandardEncoding)
        0x41: 0x0041,   # A
        0x7F: UNDEF,    # DEL is not text
        0x80: 0x20AC,   # euro
        0x81: UNDEF,    # ISO-undefined
        0x92: 0x2019,   # rquote
        0x9D: UNDEF,    # ISO-undefined
        0xA0: 0x00A0,   # nbspace
        0xC9: 0x00C9,   # Eacute
        0xFF: 0x00FF,   # ydieresis
    })
    assert_positions("MACROMAN", macroman, {
        0x20: 0x0020,
        0x41: 0x0041,
        0x7F: UNDEF,
        0xA5: 0x2022,   # bullet (the Euro really lives at 0xDB in classic Mac Roman)
        0xCA: 0x00A0,   # nbspace
        0xDB: 0x00A4,   # currency — spec value, not the codec's modern euro
        0xF0: 0xF8FF,   # Apple logo (private use — the encoding really maps it)
    })
    assert_positions("STANDARD", standard, {
        0x20: 0x0020,
        0x27: 0x2019,   # quoteright
        0x60: 0x2018,   # quoteleft
        0x80: UNDEF,    # 0200-0237 are undefined in StandardEncoding
        0xA4: 0x2044,   # fraction
        0xB7: 0x2022,   # bullet
        0xC1: 0x0060,   # grave
    })
    assert_positions("PDFDOC", pdfdoc, {
        0x20: 0x0020,
        0x18: 0x02D8,   # breve
        0xAD: UNDEF,
        0xE9: 0x00E9,
    })

    text = emit(winansi, macroman, standard, pdfdoc, glyphlist)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(text, encoding="utf-8")
    print(f"wrote {args.out} ({len(text)} bytes, {text.count(chr(10))} lines)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
