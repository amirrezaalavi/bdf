#!/usr/bin/env python3
"""Real-object probe for a PDF: what the file actually contains, per page.

Why this exists next to `triage_pdfs.py`: byte-level regexes are fast and dependency-free, but
they lie the moment a file is encrypted or its streams are compressed with something other
than Flate — which is exactly the state the Arabic/Hebrew samples arrived in (`ToUnicode`
counts of 97811 on a 1.7 MB file, zero text operators). A real parser answers the questions
that matter:

  * is the file encrypted (and can it be opened with an empty password)?
  * how many pages, and does each page actually carry text operators?
  * which fonts, and do they carry a `ToUnicode` map at all — without one, RTL text is
    unrecoverable and the file is an OCR candidate, not an extraction candidate;
  * what do the document's own metadata strings say (usually the honest answer to "what
    language is this?").

The text itself is NOT trusted here: pypdf orders RTL text by its own rules, so this probe
reports which *scripts* appear and how much text there is — never the text as truth.

Runs in the repo's .venv (pypdf is a dev/analysis dependency; it is not shipped).

Usage:
    .venv/Scripts/python tools/pdf_probe.py FILE [FILE ...]
    .venv/Scripts/python tools/pdf_probe.py --dir corpus/raw/private/desktop-pdfs
"""
from __future__ import annotations

import argparse
import pathlib
import re
import sys

from pypdf import PdfReader

SCRIPTS = {
    "ar": [(0x0600, 0x06FF), (0x0750, 0x077F), (0xFB50, 0xFDFF), (0xFE70, 0xFEFF)],
    "he": [(0x0590, 0x05FF), (0xFB1D, 0xFB4F)],
    "latin": [(0x0020, 0x007E), (0x00A0, 0x024F)],
}


def scripts_of(text: str) -> list[str]:
    cps = {ord(ch) for ch in text}
    return [name for name, ranges in SCRIPTS.items()
            if any(any(lo <= cp <= hi for lo, hi in ranges) for cp in cps)]


def probe(path: pathlib.Path) -> dict:
    facts: dict = {"path": str(path), "bytes": path.stat().st_size}
    try:
        reader = PdfReader(str(path))
    except Exception as exc:  # noqa: BLE001
        return {**facts, "error": f"{type(exc).__name__}: {exc}"}

    facts["encrypted"] = bool(reader.is_encrypted)
    if reader.is_encrypted:
        try:
            facts["decrypts_with_empty_password"] = bool(reader.decrypt(""))
        except Exception:  # noqa: BLE001
            facts["decrypts_with_empty_password"] = False
    try:
        facts["pages"] = len(reader.pages)
    except Exception as exc:  # noqa: BLE001
        facts["pages"] = None
        facts["pages_error"] = f"{type(exc).__name__}: {exc}"

    meta = {}
    try:
        for key, value in (reader.metadata or {}).items():
            if value is None:
                continue
            text = str(value)
            meta[key.lstrip("/").lower()] = {
                "value": text[:80],
                "scripts": scripts_of(text),
            }
    except Exception as exc:  # noqa: BLE001
        meta["error"] = f"{type(exc).__name__}: {exc}"
    facts["metadata"] = meta

    fonts: dict[str, dict] = {}
    page_scripts: list[list[str]] = []
    text_chars = 0
    op_counts: list[int] = []
    for index, page in enumerate(reader.pages):
        try:
            raw = page.get_contents()
            data = raw.get_data() if raw is not None else b""
        except Exception:  # noqa: BLE001
            data = b""
        op_counts.append(data.count(b" Tj") + data.count(b" TJ") + data.count(b"Tj\n"))
        try:
            text = page.extract_text() or ""
        except Exception:  # noqa: BLE001
            text = ""
        text_chars += len(text.strip())
        page_scripts.append(scripts_of(text))
        try:
            resources = page.get("/Resources") or {}
            font_dict = resources.get("/Font") or {}
            for name in list(font_dict.keys()):
                font = font_dict[name].get_object()
                key = f"{name}:{font.get('/Subtype')}"
                entry = fonts.setdefault(key, {"pages": 0, "tounicode": False, "encoding": str(font.get("/Encoding"))[:24]})
                entry["pages"] += 1
                if font.get("/ToUnicode") is not None:
                    entry["tounicode"] = True
                descendants = font.get("/DescendantFonts")
                if descendants:
                    try:
                        descendant = descendants[0].get_object()
                        if descendant.get("/W"):
                            entry["w_array"] = True
                        if descendant.get("/CIDToGIDMap") is not None:
                            entry["cid_to_gid_map"] = True
                    except Exception:  # noqa: BLE001
                        pass
        except Exception:  # noqa: BLE001
            pass

    facts["fonts"] = fonts
    facts["tounicode_fonts"] = sum(1 for f in fonts.values() if f["tounicode"])
    facts["fonts_total"] = len(fonts)
    facts["text_ops"] = sum(op_counts)
    facts["text_chars"] = text_chars
    seen: list[str] = []
    for group in page_scripts:
        for script in group:
            if script not in seen:
                seen.append(script)
    facts["page_text_scripts"] = seen
    return facts


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("files", nargs="*")
    parser.add_argument("--dir")
    parser.add_argument("--json")
    args = parser.parse_args()

    targets = [pathlib.Path(p) for p in args.files]
    if args.dir:
        targets += sorted(p for p in pathlib.Path(args.dir).glob("*.pdf") if p.is_file())

    results = []
    for path in targets:
        facts = probe(path)
        results.append(facts)
        if "error" in facts:
            print(f"{path.name:<46} ERROR {facts['error'][:60]}")
            continue
        enc = "ENCRYPTED" + ("(empty-pw ok)" if facts.get("decrypts_with_empty_password") else "(no key)")
        print(f"{path.name:<46} {facts['bytes']/1048576:6.1f}MB pg={facts['pages']!s:>4} "
              f"ops={facts['text_ops']:>6} chars={facts['text_chars']:>6} "
              f"fonts={facts['fonts_total']}(toUni {facts['tounicode_fonts']}) "
              f"scripts={'/'.join(facts['page_text_scripts']) or '-'} {enc if facts['encrypted'] else ''}")
        named = {k: v for k, v in facts["metadata"].items()
                 if isinstance(v, dict) and "scripts" in v and v["scripts"]}
        for key, value in named.items():
            print(f"      /{key}: {value['value']!r} [{ '/'.join(value['scripts']) }]")

    if args.json:
        out = pathlib.Path(args.json)
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(__import__("json").dumps({"files": results}, indent=2, ensure_ascii=False) + "\n",
                       encoding="utf-8")
        print(f"\nwrote {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
