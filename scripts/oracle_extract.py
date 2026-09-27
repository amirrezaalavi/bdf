#!/usr/bin/env python3
"""Show what each available third-party extractor returns for a PDF, and classify it
against a known-correct string, so pdfrtl never mistakes one of them for ground truth.

WHY this exists: for RTL PDFs the extractors do not merely differ, they disagree about the
ORDER of the text (visual vs logical), about whether ZWNJ survives, and -- for the
`pdftotext` binary on PATH here -- about whether the RTL characters survive at all, because
without `-enc UTF-8` its output charset is environment-dependent and silently drops every
unmappable character. A library that picks one of them as "the reference" inherits all of
those bugs. This tool makes the disagreement measurable:

  * three required extractors: PDFium (via pypdfium2), `pdftotext -raw`, `pdftotext`
    default (layout) mode -- the latter two exactly as specified;
  * two supplementary runs of the same two modes with `-enc UTF-8`, because without them
    the default-mode result is a locale artifact, not an RTL result;
  * for each: the extracted text PLUS its first 40 codepoints as U+XXXX, so ordering is
    visible as data instead of trusting a rendering;
  * a classification per extractor against --expect (the authority string, e.g. taken from
    corpus/generated/SOURCES.md):
        ABSENT (i)     the expected sample is not in the output in any order
        VISUAL (ii)    the expected sample is present but in reversed (visual) order
        LOGICAL (iii)  the expected sample is present in logical order
    Files with no authority string (real-world PDFs) are only compared RELATIVE to PDFium
    and are labelled as such -- PDFium is not promoted to truth by this script.

Usage (run from the repo root, repo's venv):
    ./.venv/Scripts/python.exe scripts/oracle_extract.py FILE.pdf --expect '\u0633\u0644\u0627\u0645 \u062f\u0646\u06cc\u0627'
    ./.venv/Scripts/python.exe scripts/oracle_extract.py F1.pdf F2.pdf --expect 'X' --expect '' \\
        --pages 2 --report reports/oracle-extraction.md

--expect accepts a Python-style escaped string (\\uXXXX, \\UXXXXXXXX, \\xXX, \\n, \\t, \\\\);
pass exactly one --expect per input file in the same order ('' = no authority for that file).
"""
from __future__ import annotations

import argparse
import datetime
import pathlib
import re
import shutil
import subprocess
import sys
import unicodedata
from collections import Counter

import pypdfium2 as pdfium

REPO_ROOT = pathlib.Path(__file__).resolve().parent.parent

# Script ranges, same ones tools/pdf_probe.py classifies with.
RTL_RANGES = (
    (0x0600, 0x06FF), (0x0750, 0x077F), (0xFB50, 0xFDFF), (0xFE70, 0xFEFF),  # Arabic
    (0x0590, 0x05FF), (0xFB1D, 0xFB4F),                                      # Hebrew
)
LAM_ALEF = "\u0644\u0627"     # the ligature that makes naive reversal fail
SENTINEL = "\uE000"           # stands in for LAM_ALEF during ligature-aware compares
ZWNJ = "\u200C"
MAX_CODEPOINTS = 40           # the hex dump mandated by the spec

_ESCAPE_RE = re.compile(
    r"\\(u[0-9a-fA-F]{4}|U[0-9a-fA-F]{8}|x[0-9a-fA-F]{2}|[nrtbfv0\\'\"a])")
_SIMPLE = {"n": "\n", "r": "\r", "t": "\t", "b": "\b", "f": "\f", "v": "\v",
           "0": "\0", "\\": "\\", "'": "'", '"': '"', "a": "\a"}


def unescape_expect(raw: str) -> str:
    """Decode a Python-style escaped expectation string; pass plain text through."""
    def repl(match: re.Match[str]) -> str:
        token = match.group(1)
        if token[0] in _SIMPLE:
            return _SIMPLE[token[0]]
        if token[0] == "u":
            return chr(int(token[1:], 16))
        if token[0] == "U":
            return chr(int(token[1:], 16))
        return chr(int(token[1:], 16))  # \xXX
    return _ESCAPE_RE.sub(repl, raw)


def escape_text(text: str, limit: int) -> str:
    """Human-auditable one-line form: control/format chars as \\uXXXX/\\n, letters literal."""
    out: list[str] = []
    for ch in text:
        cp = ord(ch)
        cat = unicodedata.category(ch)
        if ch == "\n":
            out.append("\\n")
        elif ch == "\r":
            out.append("\\r")
        elif ch == "\t":
            out.append("\\t")
        elif cp < 0x20 or 0x7F <= cp <= 0x9F or cat in ("Cc", "Cf", "Cs", "Co"):
            out.append("\\u%04X" % cp)
        else:
            out.append(ch)
    joined = "".join(out)
    if limit and len(joined) > limit:
        return joined[:limit] + "... [truncated: %d of %d chars shown; comparisons below " \
               "used the full text]" % (limit, len(joined))
    return joined


def codepoint_dump(text: str, limit: int = MAX_CODEPOINTS) -> str:
    return " ".join("U+%04X" % ord(c) for c in text[:limit])


def is_rtl(ch: str) -> bool:
    cp = ord(ch)
    return any(lo <= cp <= hi for lo, hi in RTL_RANGES)


def rtl_only(text: str) -> str:
    """RTL-script characters only: whitespace, Latin, punctuation and bidi marks dropped.

    Classification asks about the RTL *sample*; neutral characters (spaces, the em dash,
    '/', bidi isolates) must not be able to decide the verdict.
    """
    return "".join(ch for ch in text if is_rtl(ch))


def strip_noise(text: str) -> str:
    """Drop whitespace and format/control characters (bidi marks, ZWNJ) from both sides."""
    return "".join(ch for ch in text
                   if not ch.isspace() and unicodedata.category(ch) not in ("Cf", "Cc"))


def mask_lam_alef(text: str) -> str:
    return text.replace(LAM_ALEF, SENTINEL)


def stats(text: str) -> dict:
    rtl = [ch for ch in text if is_rtl(ch)]
    format_marks = Counter(ch for ch in text if unicodedata.category(ch) == "Cf"
                           and ch not in (ZWNJ, "\u200D"))
    presentation = sum(1 for ch in text
                       if 0xFB50 <= ord(ch) <= 0xFDFF or 0xFE70 <= ord(ch) <= 0xFEFF)
    return {
        "chars": len(text),
        "rtl": len(rtl),
        "rtl_distinct": len(set(rtl)),
        "zwnj": text.count(ZWNJ),
        "format_marks": sum(format_marks.values()),
        "format_mark_detail": ", ".join("U+%04X x%d" % (ord(k), v)
                                        for k, v in sorted(format_marks.items())),
        "presentation_forms": presentation,
    }


def classify(expected: str, candidate: str) -> dict:
    """Classify `candidate` against `expected`: LOGICAL (iii) / VISUAL (ii) / ABSENT (i)."""
    exp_rtl, cand_rtl = rtl_only(expected), rtl_only(candidate)
    result: dict = {"zwnj_expected": expected.count(ZWNJ),
                    "zwnj_output": candidate.count(ZWNJ)}

    if not exp_rtl:
        # No RTL in the expectation (the en-control case): compare the whole string,
        # ignoring whitespace and bidi marks, so RLE/PDF wrappers cannot fake a match.
        exp, cand = strip_noise(expected), strip_noise(candidate)
        if exp and exp in cand:
            result.update(verdict="LOGICAL", scheme="(iii)", method="substring (LTR)")
            return result
        if exp and exp[::-1] in cand:
            result.update(verdict="VISUAL", scheme="(ii)", method="reversed substring (LTR)")
            return result
        detail = "expected string not found"
        if exp and cand:
            limit = min(len(exp), len(cand))
            for i in range(limit):
                if exp[i] != cand[i]:
                    detail = (f"first divergence at index {i}: expected U+{ord(exp[i]):04X} "
                              f"got U+{ord(cand[i]):04X}")
                    break
            else:
                detail = f"prefix matches for {limit} chars, then length differs " \
                         f"(expected {len(exp)}, got {len(cand)})"
        elif not cand:
            detail = "output is empty"
        result.update(verdict="ABSENT", scheme="(i)", method="substring (LTR)", detail=detail)
        return result

    if exp_rtl in cand_rtl:
        result.update(verdict="LOGICAL", scheme="(iii)", method="RTL substring, logical order")
        return result
    if exp_rtl[::-1] in cand_rtl:
        result.update(verdict="VISUAL", scheme="(ii)",
                      method="RTL substring, naive reversal")
        return result
    if mask_lam_alef(exp_rtl)[::-1] in mask_lam_alef(cand_rtl):
        # The lam-alef ligature is one glyph, so a correct visual reversal keeps it whole
        # where the logical string has two codepoints. Compare with it masked out.
        result.update(verdict="VISUAL", scheme="(ii)",
                      method="RTL substring, reversal with lam-alef ligature masked")
        return result

    missing = Counter(exp_rtl) - Counter(cand_rtl)
    if not missing:
        detail = (f"all {len(exp_rtl)} expected RTL characters are present, but in neither "
                  f"logical nor visual order (scrambled)")
    else:
        detail = "missing RTL characters: " + ", ".join(
            "U+%04X x%d" % (ord(k), v) for k, v in sorted(missing.items()))
    if not cand_rtl:
        detail = "output contains 0 RTL-script characters"
    result.update(verdict="ABSENT", scheme="(i)", method="RTL substring, both orders",
                  detail=detail)
    return result


def compare_relative(reference: str, candidate: str) -> str:
    """Relative order vs PDFium. NOT ground truth -- used only where no authority exists."""
    ref, cand = rtl_only(reference), rtl_only(candidate)
    if not ref:
        return "no RTL text in the PDFium output to compare against"
    if not cand:
        return "output has 0 RTL characters (PDFium has %d)" % len(ref)
    if cand == ref:
        return "same RTL order as PDFium"
    if cand == ref[::-1]:
        return "exact reverse of PDFium's RTL order"
    if mask_lam_alef(cand) == mask_lam_alef(ref):
        return "same RTL order as PDFium (ignoring lam-alef ligature clustering)"
    if mask_lam_alef(cand) == mask_lam_alef(ref)[::-1]:
        return "reverse of PDFium's RTL order (ignoring lam-alef ligature clustering)"
    missing = Counter(ref) - Counter(cand)
    if not missing:
        return "same RTL characters as PDFium but in a third order (neither order matches)"
    return "differs from PDFium; missing: " + ", ".join(
        "U+%04X x%d" % (ord(k), v) for k, v in sorted(missing.items()))


# --------------------------------------------------------------------------- extractors

def pdftotext_argv(mode_utf8: str, raw: bool, pdf: pathlib.Path, page: int) -> list[str]:
    """mode_utf8 is 'utf8' or 'locale'; raw selects -raw vs default (layout) mode."""
    argv = ["pdftotext"]
    if raw:
        argv.append("-raw")
    if mode_utf8 == "utf8":
        argv += ["-enc", "UTF-8"]
    argv += ["-f", str(page), "-l", str(page), pdf.as_posix(), "-"]
    return argv


def extract_pdftotext(argv: list[str]) -> tuple[str, dict]:
    try:
        proc = subprocess.run(argv, capture_output=True, timeout=120)
    except FileNotFoundError:
        raise
    except subprocess.TimeoutExpired as exc:
        raise RuntimeError(f"timed out after {exc.timeout}s: {' '.join(argv)}") from exc
    raw_bytes = proc.stdout
    note = {"returncode": proc.returncode,
            "stderr": proc.stderr.decode("utf-8", "replace").strip(),
            "argv": " ".join(argv)}
    try:
        text = raw_bytes.decode("utf-8")
        note["stdout_encoding"] = "utf-8"
    except UnicodeDecodeError:
        # Without -enc the binary picks a locale charset and drops what it cannot map.
        text = raw_bytes.decode("cp1252", "replace")
        note["stdout_encoding"] = "NOT utf-8 (decoded as cp1252 for display only)"
        note["stdout_bytes"] = len(raw_bytes)
    return text, note


def extract_pdfium(doc, page_no: int) -> tuple[str, dict]:
    page = doc[page_no - 1]
    textpage = page.get_textpage()
    return textpage.get_text_range(), {"argv": "pypdfium2 get_text_range(page %d)" % page_no}


def build_extractors() -> list[dict]:
    """The three extractors required by the spec plus the two UTF-8 supplements."""
    return [
        {"id": "pdfium", "label": "PDFium via pypdfium2", "kind": "pdfium"},
        {"id": "pdftotext-raw", "label": "pdftotext -raw (as specified, locale charset)",
         "kind": "pdftotext", "raw": True, "enc": "locale"},
        {"id": "pdftotext-default", "label": "pdftotext default/layout mode (as specified, locale charset)",
         "kind": "pdftotext", "raw": False, "enc": "locale"},
        {"id": "pdftotext-raw-utf8", "label": "pdftotext -raw -enc UTF-8 (supplementary)",
         "kind": "pdftotext", "raw": True, "enc": "utf8"},
        {"id": "pdftotext-default-utf8", "label": "pdftotext -enc UTF-8 default/layout mode (supplementary)",
         "kind": "pdftotext", "raw": False, "enc": "utf8"},
    ]


# --------------------------------------------------------------------------- reporting

def environment_block(pdftotext_path: str | None, pdftotext_banner: str) -> list[str]:
    import locale
    import platform
    lines = [
        "",
        "## Environment (recorded so the numbers can be reproduced)",
        "",
        f"- python: {platform.python_version()} ({sys.executable})",
        f"- pypdfium2: {getattr(pdfium, 'V_PYPDFIUM2', 'unknown')} "
        f"(PDFium {getattr(pdfium, 'V_PDFIUM', '?')})",
        f"- pdftotext path: {pdftotext_path or 'NOT FOUND'}",
        "- pdftotext banner (verbatim):",
        "",
        "```",
        pdftotext_banner.rstrip("\n") or "(no banner captured)",
        "```",
        f"- output charset env: LANG={_env('LANG')!r} LC_ALL={_env('LC_ALL')!r} "
        f"LANGUAGE={_env('LANGUAGE')!r}",
        "- The banner says Glyph & Cog / the binary self-identifies as Xpdf, not poppler; "
        "the ids `pdftotext-*` below mean this binary.",
        "",
    ]
    return lines


def _env(name: str) -> str:
    import os
    return os.environ.get(name, "")


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Print what each available extractor returns for a PDF and classify "
                    "each result against a known-correct string.")
    parser.add_argument("files", nargs="+", help="input PDF path(s)")
    parser.add_argument("--expect", action="append", default=[], metavar="STR",
                        help="known-correct string, Python-style escapes "
                             r"(e.g. '\u0633\u0644\u0627\u0645'); pass one per file, "
                             "'' = none for that file")
    parser.add_argument("--pages", type=int, default=2,
                        help="number of leading pages to extract per file (default: 2)")
    parser.add_argument("--report", metavar="PATH", default=None,
                        help="also write a markdown report to PATH")
    parser.add_argument("--max-chars", type=int, default=1500,
                        help="truncate displayed text beyond N chars (0 = no limit; "
                             "comparisons always use the full text; default: 1500)")
    args = parser.parse_args()

    files = [pathlib.Path(f) for f in args.files]
    for path in files:
        if not path.is_file():
            print(f"ERROR: no such file: {path}", file=sys.stderr)
            return 2
    if args.pages < 1:
        print("ERROR: --pages must be >= 1", file=sys.stderr)
        return 2
    if args.expect and len(args.expect) not in (0, len(files)):
        print(f"ERROR: got {len(args.expect)} --expect value(s) for {len(files)} file(s); "
              f"pass exactly one per file (use '' for no expectation)", file=sys.stderr)
        return 2

    expects = args.expect if args.expect else [""] * len(files)

    pdftotext_path = shutil.which("pdftotext")
    banner = ""
    if pdftotext_path:
        try:
            proc = subprocess.run([pdftotext_path, "-v"], capture_output=True, timeout=60)
            banner = (proc.stdout + proc.stderr).decode("utf-8", "replace")
        except (OSError, subprocess.TimeoutExpired) as exc:
            banner = f"(could not run pdftotext -v: {exc})"
    else:
        print("ERROR: pdftotext not found on PATH; the comparison would be incomplete",
              file=sys.stderr)
        return 2

    extractors = build_extractors()
    collected: list[dict] = []
    stdout_lines: list[str] = []

    def emit(line: str = "") -> None:
        stdout_lines.append(line)
        print(line)

    for path, expect_raw in zip(files, expects):
        expected = unescape_expect(expect_raw) if expect_raw else None
        emit(f"=== {path.as_posix()} ===")
        if expected:
            emit(f"    expectation (--expect {expect_raw!r}): {escape_text(expected, 0)}")
            emit(f"    expectation codepoints: {codepoint_dump(expected, 60)}")
        else:
            emit("    expectation: none (no authority string for this file; results below "
                 "are compared only RELATIVE to PDFium)")
        try:
            doc = pdfium.PdfDocument(str(path))
        except Exception as exc:  # noqa: BLE001
            emit(f"    ERROR: PDFium cannot open it: {type(exc).__name__}: {exc}")
            collected.append({"file": str(path), "error": str(exc)})
            continue
        page_count = len(doc)
        page_numbers = [p for p in range(1, args.pages + 1) if p <= page_count]
        if not page_numbers:
            emit(f"    ERROR: no pages (file reports {page_count})")
            doc.close()
            collected.append({"file": str(path), "error": "no pages"})
            continue
        if page_count < args.pages:
            emit(f"    note: file has {page_count} page(s); requested first {args.pages}")

        entry = {"file": path.as_posix(), "page_count": page_count,
                 "pages_extracted": page_numbers, "expect_raw": expect_raw,
                 "expect": expected, "results": []}

        for ext in extractors:
            emit(f"\n-- extractor: {ext['id']}  [{ext['label']}]")
            per_page = []
            for page_no in page_numbers:
                try:
                    if ext["kind"] == "pdfium":
                        text, note = extract_pdfium(doc, page_no)
                    else:
                        argv = pdftotext_argv(ext["enc"], ext["raw"], path, page_no)
                        note_argv = " ".join(argv)
                        text, note = extract_pdftotext(argv)
                        note["argv"] = note_argv
                    error = None
                except Exception as exc:  # noqa: BLE001 - loud: never look like empty output
                    text, note, error = "", {}, f"{type(exc).__name__}: {exc}"
                st = stats(text)
                rec = {"page": page_no, "text": text, "stats": st, "note": note,
                       "error": error}
                per_page.append(rec)
                emit(f"  page {page_no} command: {note.get('argv', '?')}")
                if error:
                    emit(f"  page {page_no} ERROR: {error}")
                    continue
                if note.get("stderr"):
                    emit(f"  page {page_no} stderr: {note['stderr'][:300]}")
                emit(f"  page {page_no} text: {escape_text(text, args.max_chars)}")
                emit(f"  page {page_no} first {MAX_CODEPOINTS} codepoints: "
                     f"{codepoint_dump(text)}")
                emit(f"  page {page_no} stats: chars={st['chars']} rtl={st['rtl']} "
                     f"zwnj={st['zwnj']} format_marks={st['format_marks']}"
                     + (f" ({st['format_mark_detail']})" if st['format_mark_detail'] else "")
                     + f" presentation_forms={st['presentation_forms']} "
                       f"stdout={note.get('stdout_encoding', 'n/a')}")
                if page_no == 1:
                    if expected is not None:
                        verdict = classify(expected, text)
                        rec["classification"] = verdict
                        line = (f"  page 1 vs expectation: {verdict['verdict']} "
                                f"{verdict['scheme']}  method={verdict['method']}")
                        if verdict.get("detail"):
                            line += f"  detail={verdict['detail']}"
                        if verdict.get("zwnj_expected") is not None:
                            line += (f"  zwnj: expected={verdict['zwnj_expected']} "
                                     f"output={verdict['zwnj_output']}")
                        emit(line)
                    else:
                        rec["relative"] = None  # filled in after PDFium page 1 is known
                        emit("  page 1 vs expectation: n/a (no authority string)")
            entry["results"].append({"extractor": ext, "pages": per_page})
        doc.close()

        # Relative comparison for files with no authority string.
        if expected is None:
            pdfium_pages = next((r for r in entry["results"]
                                 if r["extractor"]["id"] == "pdfium"), None)
            reference = pdfium_pages["pages"][0]["text"] if pdfium_pages else ""
            emit("\n  -- relative comparison (PDFium is a peer, NOT ground truth) --")
            for r in entry["results"]:
                cand = r["pages"][0]["text"] if r["pages"] else ""
                rel = compare_relative(reference, cand)
                r["pages"][0]["relative"] = rel
                emit(f"  {r['extractor']['id']}: {rel}")
        collected.append(entry)
        emit("")

    if args.report:
        write_report(pathlib.Path(args.report), collected, extractors,
                     pdftotext_path, banner, args)
        print(f"wrote {args.report}")

    return 0


# --------------------------------------------------------------------------- markdown

def md_escape(text: str, limit: int) -> str:
    """Markdown-safe inline code for text that may contain backticks/quotes."""
    escaped = escape_text(text, limit)
    return escaped.replace("`", "'")


def write_report(report_path: pathlib.Path, collected: list[dict], extractors: list[dict],
                 pdftotext_path: str | None, banner: str, args) -> None:
    now = datetime.datetime.now().isoformat(timespec="seconds")
    lines: list[str] = []
    add = lines.append

    add("# Oracle extraction: what each third-party extractor actually returns for RTL PDFs")
    add("")
    add(f"Generated by `scripts/oracle_extract.py` on {now} "
        f"(command line: `--pages {args.pages}`).")
    add("Every string, codepoint and verdict below was produced by that run; nothing in "
        "this file is transcribed by hand.")
    add("")
    add("## How to read this")
    add("")
    add("- The **authority** for the generated fixtures is `corpus/generated/SOURCES.md`, "
        "not any extractor. Each file's expectation is printed above its evidence, both as "
        "a Python-style escape and as `U+XXXX` codepoints.")
    add("- Classification compares only **RTL-script characters** of the expectation "
        "against the output, so whitespace, bidi marks and Latin text cannot decide a "
        "verdict on their own:")
    add("  - **LOGICAL (iii)** -- the expected sample is present in logical order;")
    add("  - **VISUAL (ii)** -- present, but reversed (what a screen shows, and what "
        "`-raw` tends to return);")
    add("  - **ABSENT (i)** -- present in neither order (including 'characters all there "
        "but scrambled').")
    add("- `pdftotext -raw` / `pdftotext` default mode are the two modes the spec names; "
        "without `-enc UTF-8` they run in the binary's default charset, so they are "
        "reported twice: as specified, and with `-enc UTF-8` for a charset-independent "
        "result. The difference between those two runs is the point of this report.")
    add("- Files with **no authority string** (the real-world PDFs) get "
        "`n/a` plus a *relative* comparison against PDFium. That comparison shows the two "
        "tools disagreeing; it does not say which one is right.")
    add("")
    lines += environment_block(pdftotext_path, banner)

    # ------------------------------------------------------------- summary table
    add("## Summary (page 1 classification)")
    add("")
    header = "| fixture | " + " | ".join(e["id"] for e in extractors) + " |"
    add(header)
    add("|" + "---|" * (len(extractors) + 1))
    for entry in collected:
        if entry.get("error"):
            name = pathlib.Path(entry["file"]).name
            add("| `" + name + "` | " + " | ".join(
                f"ERROR: {entry['error']}" for _ in extractors) + " |")
            continue
        cells = []
        for r in entry["results"]:
            rec = r["pages"][0] if r["pages"] else {}
            if rec.get("error"):
                cells.append("ERROR")
            elif "classification" in rec:
                v = rec["classification"]
                cells.append(f"**{v['verdict']}** {v['scheme']}")
            elif rec.get("relative"):
                cells.append(f"n/a (relative: {rec['relative']})")
            else:
                cells.append("n/a")
        add(f"| `{pathlib.Path(entry['file']).name}` | " + " | ".join(cells) + " |")
    add("")

    # ------------------------------------------------------------- per-file evidence
    add("## Per-file evidence (text + first %d codepoints, verbatim from the run)" %
        MAX_CODEPOINTS)
    for entry in collected:
        add("")
        add(f"### `{entry['file']}`")
        add("")
        if entry.get("error"):
            add(f"ERROR: {entry['error']}")
            continue
        add(f"- pages: {entry['page_count']} (extracted: {entry['pages_extracted']})")
        if entry["expect"]:
            add(f"- expectation: `{md_escape(unescape_expect(entry['expect_raw']), 0)}` = "
                f"`{md_escape(entry['expect'], 0)}`")
            add(f"- expectation codepoints: `{codepoint_dump(entry['expect'], 60)}`")
        else:
            add("- expectation: none (no authority string; see the relative comparison)")
        add("")
        for r in entry["results"]:
            ext = r["extractor"]
            add(f"**`{ext['id']}`** -- {ext['label']}")
            add("")
            for rec in r["pages"]:
                add(f"- page {rec['page']}: `{rec['note'].get('argv', '?')}`")
                if rec.get("error"):
                    add(f"  - ERROR: {rec['error']}")
                    continue
                st = rec["stats"]
                add(f"  - text: `{md_escape(rec['text'], args.max_chars)}`")
                add(f"  - first {MAX_CODEPOINTS} codepoints: "
                    f"`{codepoint_dump(rec['text'])}`")
                add(f"  - stats: chars={st['chars']}, rtl={st['rtl']}, "
                    f"zwnj={st['zwnj']}, format_marks={st['format_marks']}"
                    + (f" ({st['format_mark_detail']})" if st['format_mark_detail'] else "")
                    + f", presentation_forms={st['presentation_forms']}, "
                      f"stdout={rec['note'].get('stdout_encoding', 'n/a')}")
                if rec["note"].get("stderr"):
                    add(f"  - stderr: {rec['note']['stderr'][:300]}")
                if "classification" in rec:
                    v = rec["classification"]
                    line = (f"  - **{v['verdict']} {v['scheme']}** -- {v['method']}; "
                            f"zwnj expected={v['zwnj_expected']} output={v['zwnj_output']}")
                    if v.get("detail"):
                        line += f"; {v['detail']}"
                    add(line)
                elif rec.get("relative"):
                    add(f"  - relative vs PDFium (NOT ground truth): {rec['relative']}")
            add("")

    # ------------------------------------------------------------- observations
    add("## Cross-cutting observations (computed from the rows above)")
    add("")
    with_expect = [e for e in collected if e.get("expect")]
    add(f"- Files with an authority string: {len(with_expect)} of {len(collected)} "
        f"({', '.join('`' + pathlib.Path(e['file']).name + '`' for e in with_expect) or 'none'}).")
    add("")
    add("### Verdict tally per extractor (files with an authority string, page 1)")
    add("")
    add("| extractor | LOGICAL (iii) | VISUAL (ii) | ABSENT (i) |")
    add("|---|---|---|---|")
    for ext in extractors:
        tally = Counter()
        for entry in with_expect:
            r = next((r for r in entry["results"]
                      if r["extractor"]["id"] == ext["id"]), None)
            rec = r["pages"][0] if r and r["pages"] else {}
            tally[rec.get("classification", {}).get("verdict", "ERROR")] += 1
        add(f"| `{ext['id']}` | {tally['LOGICAL']} | {tally['VISUAL']} | "
            f"{tally['ABSENT'] + tally['ERROR']} |")
    add("")
    add("### RTL characters surviving on page 1 (0 = the extractor dropped the script)")
    add("")
    add("| fixture | " + " | ".join(e["id"] for e in extractors) + " |")
    add("|" + "---|" * (len(extractors) + 1))
    for entry in collected:
        if entry.get("error"):
            continue
        cells = []
        for r in entry["results"]:
            rec = r["pages"][0] if r["pages"] else {}
            cells.append(str(rec.get("stats", {}).get("rtl", "ERR")))
        add(f"| `{pathlib.Path(entry['file']).name}` | " + " | ".join(cells) + " |")
    add("")
    zwnj_files = [e for e in with_expect if unescape_expect(e["expect_raw"]).count(ZWNJ)]
    if zwnj_files:
        add("### ZWNJ (U+200C) fidelity -- the codepoint visual review can never see")
        add("")
        add("| fixture | expected | " + " | ".join(e["id"] for e in extractors) + " |")
        add("|" + "---|" * (len(extractors) + 2))
        for entry in zwnj_files:
            want = unescape_expect(entry["expect_raw"]).count(ZWNJ)
            cells = []
            for r in entry["results"]:
                rec = r["pages"][0] if r["pages"] else {}
                got = rec.get("stats", {}).get("zwnj", 0)
                cells.append(str(got) + ("" if got == want else " (DROPPED)"))
            add(f"| `{pathlib.Path(entry['file']).name}` | {want} | "
                + " | ".join(cells) + " |")
        add("")
    add("---")
    add("Reproduce: `./.venv/Scripts/python.exe scripts/oracle_extract.py "
        + " ".join(e["file"] for e in collected)
        + " --expect ... --report " + str(report_path).replace("\\", "/") + "`")
    add("")

    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text("\n".join(lines), encoding="utf-8")


if __name__ == "__main__":
    raise SystemExit(main())
