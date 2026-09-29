#!/usr/bin/env python3
"""Triage a folder of PDFs: what is each file, and can its text be recovered?

Purpose: decide what belongs in the corpus, and *why*. For every file it reports the
producer, page count, and — most importantly — the RTL text machinery actually present
(Type0 fonts, ToUnicode CMaps, /ActualText, /ReversedChars), plus which scripts the file
can carry (detected from the Unicode targets in those structures, not from rendering).

This is triage, not extraction: it never claims what the text *is*, only what the file is
made of. `pdfinfo` is used for pages/producer metadata (an oracle tool, never shipped).

Usage:
    python tools/triage_pdfs.py [DIR]  # defaults to corpus/raw/private/desktop-pdfs
    python tools/triage_pdfs.py DIR --json reports/triage.json
    python tools/triage_pdfs.py DIR --markdown reports/triage.md
    python tools/triage_pdfs.py DIR --max-mb 5      # only files up to 5 MB
"""
from __future__ import annotations

import argparse
import json
import pathlib
import re
import subprocess
import sys
import zlib

STREAM_RE = re.compile(rb"stream\r?\n(.*?)\r?\nendstream", re.DOTALL)
ACTUAL_TEXT_RE = re.compile(rb"/ActualText\s*(<[0-9A-Fa-f\s]*>|\((?:[^()\\]|\\.)*\))")
BFCHAR_RE = re.compile(rb"<([0-9A-Fa-f]{2,8})>\s*<([0-9A-Fa-f]{2,8})>")
BFRANGE_RE = re.compile(rb"<([0-9A-Fa-f]{2,8})>\s*<([0-9A-Fa-f]{2,8})>\s*<([0-9A-Fa-f]{2,8})>")

SCRIPT_RANGES = {
    "ar": [(0x0600, 0x06FF), (0x0750, 0x077F), (0xFB50, 0xFDFF), (0xFE70, 0xFEFF)],
    "he": [(0x0590, 0x05FF), (0xFB1D, 0xFB4F)],
    "latin": [(0x0020, 0x007E), (0x00A0, 0x024F)],
}


def which_scripts(codepoints: set[int]) -> list[str]:
    found = []
    for script, ranges in SCRIPT_RANGES.items():
        if any(any(lo <= cp <= hi for lo, hi in ranges) for cp in codepoints):
            found.append(script)
    return found


def inflate(blob: bytes) -> bytes:
    try:
        return zlib.decompress(blob)
    except zlib.error:
        return b""


def codepoints_from_hex(hex_digits: bytes) -> set[int]:
    """Decode a UTF-16BE payload (as used in ToUnicode targets and /ActualText)."""
    try:
        raw = bytes.fromhex(hex_digits.decode("ascii"))
    except ValueError:
        return set()
    if raw.startswith(b"\xfe\xff"):
        raw = raw[2:]
        text = raw.decode("utf-16-be", "ignore")
    else:
        text = raw.decode("utf-8", "ignore") or raw.decode("latin-1", "ignore")
    return {ord(ch) for ch in text}


def pdfinfo(path: pathlib.Path) -> dict:
    try:
        out = subprocess.run(["pdfinfo", str(path)], capture_output=True, text=True, timeout=60)
    except (OSError, subprocess.SubprocessError):
        return {}
    info: dict[str, object] = {}
    for line in out.stdout.splitlines():
        if ":" not in line:
            continue
        key, _, value = line.partition(":")
        key, value = key.strip().lower(), value.strip()
        if key in ("pages",):
            try:
                info["pages"] = int(value)
            except ValueError:
                pass
        elif key in ("producer", "creator", "encrypted", "page size", "file size"):
            info[key.replace(" ", "_")] = value
    return info


def probe_pdftotext(path: pathlib.Path) -> list[str]:
    """Which scripts appear in an oracle extraction of the first pages.

    Used only to *classify* a file (what language is this?), never as evidence about ordering
    or correctness — poppler gets RTL order and /ActualText wrong, but it does report which
    Unicode blocks a text layer claims to contain, which is exactly the question here.
    """
    try:
        out = subprocess.run(["pdftotext", "-f", "1", "-l", "3", str(path), "-"],
                             capture_output=True, timeout=120)
    except (OSError, subprocess.SubprocessError):
        return []
    text = out.stdout.decode("utf-8", "ignore")
    return which_scripts({ord(ch) for ch in text})


# Runs of >=3 consecutive RTL codepoints stored as UTF-16BE. A 3-in-a-row match inside
# compressed data is vanishingly unlikely, so this finds Persian/Hebrew strings in /Info and
# XMP metadata (and in any uncompressed string) without a false-positive storm.
UTF16BE_ARABIC_RUN = re.compile(rb"(?:\x06[\x00-\xff]|[\xfb-\xfd][\x50-\xff]|\xfe[\x70-\xff]){4,}")
UTF16BE_HEBREW_RUN = re.compile(rb"(?:\x05[\x90-\xff]|\xfb[\x1d-\x4f]){5,}")


def probe_metadata_scripts(data: bytes) -> list[str]:
    """Scripts visible in UTF-16BE strings — metadata, not the text layer.

    A *positive* signal only. Absence proves nothing, because the reliable places to look for
    RTL text (ToUnicode, /ActualText) are compressed and frequently missing altogether.
    """
    found: set[int] = set()
    if UTF16BE_ARABIC_RUN.search(data):
        found.add(0x0600)
    if UTF16BE_HEBREW_RUN.search(data):
        found.add(0x0590)
    return which_scripts(found)


def triage(path: pathlib.Path) -> dict:
    data = path.read_bytes()
    result: dict = {
        "path": str(path),
        "bytes": len(data),
        "pages": None,
        "producer": None,
        "encrypted": False,
    }
    result.update({k: v for k, v in pdfinfo(path).items() if k in ("pages", "producer")} or {})
    if result["pages"] is None:
        # pdfinfo (poppler-utils) may be absent; fall back to the page tree count so the
        # corpus never records a fixture without a page count it could have known.
        sizes = [int(m) for m in re.findall(rb"/Type\s*/Pages[^>]*?/Count\s+(\d+)", data)]
        result["pages"] = max(sizes) if sizes else None
        result["pages_from"] = "page-tree-count" if sizes else "unknown"
    if "encrypted" in pdfinfo(path):
        result["encrypted"] = str(pdfinfo(path)["encrypted"]).lower().startswith("yes")

    counts = {
        "actual_text": len(ACTUAL_TEXT_RE.findall(data)),
        "reversed_chars": data.count(b"/ReversedChars"),
        "type0_fonts": data.count(b"/Type0"),
        "identity_h": data.count(b"/Identity-H"),
        "tounicode_refs": data.count(b"/ToUnicode"),
        "cid_to_gid": data.count(b"/CIDToGIDMap"),
        "jbig2": data.count(b"/JBIG2Decode"),
        "jpx": data.count(b"/JPXDecode"),
        "dct": data.count(b"/DCTDecode"),
        "image_xobject": data.count(b"/Image"),
    }

    codepoints: set[int] = set()
    text_ops = 0
    actual_text_in_streams = 0
    for blob in STREAM_RE.findall(data):
        content = inflate(blob)
        if not content:
            continue
        text_ops += content.count(b" Tj") + content.count(b" TJ")
        for match in ACTUAL_TEXT_RE.finditer(content):
            token = match.group(1)
            if token.startswith(b"<"):
                codepoints |= codepoints_from_hex(re.sub(rb"\s", b"", token[1:-1]))
        if b"beginbfchar" in content or b"beginbfrange" in content:
            for _, target in BFCHAR_RE.findall(content):
                codepoints |= codepoints_from_hex(target)
            for _, _, target in BFRANGE_RE.findall(content):
                codepoints |= codepoints_from_hex(target)
        if b"/ActualText" in content:
            actual_text_in_streams += content.count(b"/ActualText")
        # Simple-font producers carry no ToUnicode at all; their script evidence lives in
        # glyph names such as /uni0645 or /afii57409 (PDF 32000-1 §9.6.5.2 Differences).
        for name in re.findall(rb"/(uni[0-9A-Fa-f]{4}|u[0-9A-Fa-f]{4,6}|afii[0-9]{5})", content):
            if name.startswith(b"uni") or name.startswith(b"u"):
                digits = name[3:] if name.startswith(b"uni") else name[1:]
                try:
                    codepoints.add(int(digits, 16))
                except ValueError:
                    pass
            elif name.startswith(b"afii"):
                # Adobe glyph-list names: afii57409 range is Hebrew/Arabic presentation.
                try:
                    index = int(name[4:])
                except ValueError:
                    continue
                if 57344 <= index <= 57800:
                    codepoints.add(0x0600)
                elif 57000 <= index < 57344:
                    codepoints.add(0x0590)

    # /ActualText only exists compressed inside content streams; the raw-byte count is
    # almost always 0 and is kept only as a curiosity.
    counts["actual_text_raw"] = counts.pop("actual_text")
    counts["actual_text"] = actual_text_in_streams

    scripts = which_scripts(codepoints)
    pdftotext_scripts = probe_pdftotext(path)
    has_font_resources = counts["type0_fonts"] > 0 or counts["tounicode_refs"] > 0
    verdict = []
    if text_ops == 0 and not has_font_resources:
        verdict.append("no text layer (scanned/image-only?)")
    if counts["actual_text"] or counts.get("actual_text_in_streams"):
        verdict.append("has /ActualText")
    if counts["reversed_chars"]:
        verdict.append("has /ReversedChars (producer stores visual order)")
    if has_font_resources and not counts["actual_text"]:
        verdict.append("no /ActualText: recovery depends on ToUnicode")
    if result["encrypted"]:
        verdict.append("encrypted")

    result.update(
        {
            "counts": counts,
            "text_ops": text_ops,
            "scripts": scripts,
            "metadata_scripts": probe_metadata_scripts(data),
            "pdftotext_scripts": pdftotext_scripts,
            "has_text_layer": text_ops > 0,
            "verdict": "; ".join(verdict) or "plain text, no marked content",
        }
    )
    return result


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("directory", nargs="?", default="corpus/raw/private/desktop-pdfs",
                        help="PDF folder (default: corpus/raw/private/desktop-pdfs)")
    parser.add_argument("--json")
    parser.add_argument("--markdown")
    parser.add_argument("--max-mb", type=float, default=None)
    parser.add_argument("--recursive", action="store_true")
    args = parser.parse_args()

    root = pathlib.Path(args.directory)
    pattern = "**/*.pdf" if args.recursive else "*.pdf"
    files = sorted(p for p in root.glob(pattern) if p.is_file())
    if args.max_mb is not None:
        files = [p for p in files if p.stat().st_size <= args.max_mb * 1048576]

    rows = []
    for path in files:
        try:
            row = triage(path)
        except Exception as exc:  # noqa: BLE001 - triage must survive a broken file
            row = {"path": str(path), "error": f"{type(exc).__name__}: {exc}"}
        rows.append(row)
        print(
            f"{(row.get('bytes') or 0) / 1048576:7.2f} MB  "
            f"p={row.get('pages') or '?':>4}  "
            f"{'/'.join(row.get('scripts') or ['-']):<12} "
            f"AT={row.get('counts', {}).get('actual_text', 0):<4} "
            f"RC={row.get('counts', {}).get('reversed_chars', 0):<3} "
            f"T0={row.get('counts', {}).get('type0_fonts', 0):<3} "
            f"| {pathlib.Path(row.get('path', '')).name}  :: {row.get('verdict', row.get('error', ''))[:70]}"
        )

    if args.json:
        out = pathlib.Path(args.json)
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(json.dumps({"count": len(rows), "files": rows}, indent=2, ensure_ascii=False) + "\n",
                       encoding="utf-8")
        print(f"\nwrote {out}")

    if args.markdown:
        out = pathlib.Path(args.markdown)
        out.parent.mkdir(parents=True, exist_ok=True)
        lines = [
            "# PDF triage",
            "",
            "| file | MB | pages | scripts | /ActualText | /ReversedChars | Type0 | text layer | verdict |",
            "|---|---|---|---|---|---|---|---|---|",
        ]
        for row in rows:
            counts = row.get("counts", {})
            lines.append(
                f"| `{pathlib.Path(row.get('path','')).name}` | {(row.get('bytes') or 0)/1048576:.1f} "
                f"| {row.get('pages') or '?'} | {'/'.join(row.get('scripts') or ['-'])} "
                f"| {counts.get('actual_text', 0)} | {counts.get('reversed_chars', 0)} "
                f"| {counts.get('type0_fonts', 0)} | {'yes' if row.get('has_text_layer') else 'no'} "
                f"| {row.get('verdict', row.get('error',''))} |"
            )
        out.write_text("\n".join(lines) + "\n", encoding="utf-8")
        print(f"wrote {out}")

    print(f"\ntriaged {len(rows)} file(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
