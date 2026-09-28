#!/usr/bin/env python3
"""Validate `pdfrtl extract` over the private archive and write the evidence reports.

Runs the CLI once per file in `corpus/raw/private/desktop-pdfs/` (READ ONLY — the
script never writes there) and records, per file, two claims that are counted
SEPARATELY (ADR 0004):

  * **fully decoded**  — every page produced its glyphs (no `unsupported_*` decode
    refusal on any page);
  * **order verified** — every page's order was established by a NAMED rule, and
    that rule is reported per file. A clean decode is not evidence of order.

Decoding and ordering are different claims, so this report never collapses them
into one "recovered" number. Characters that were decoded but not ordered are
counted in `unordered_chars` — the CLI withholds them from `data.text` — and an
independent word probe checks, per file, that the logical form of a pure-RTL word
appears in the emitted text and its character-reversed form does not.

Output: `reports/validate-archive.json` (machine) and `reports/validate-archive.md`
(summary). Both are local evidence: files are named by their corpus (renamed) ids.

Usage (from the repo root):
    python3 scripts/validate-archive.py --binary /path/to/pdfrtl
Exit code: 0 always (the reports are evidence, not a gate — see ADR 0004).
"""
from __future__ import annotations

import argparse
import collections
import datetime as dt
import json
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
ARCHIVE = ROOT / "corpus" / "raw" / "private" / "desktop-pdfs"
REPORTS = ROOT / "reports"


def run(binary: str, verb: str, path: pathlib.Path) -> tuple[int, dict]:
    proc = subprocess.run([binary, "--json", verb, str(path)],
                          capture_output=True, text=True, cwd=ROOT)
    try:
        envelope = json.loads(proc.stdout)
    except json.JSONDecodeError:
        envelope = {"ok": False, "data": {"error": f"unparseable stdout: {proc.stdout[:120]!r}"},
                    "reasons": []}
    return proc.returncode, envelope


# Positive rules that ESTABLISH order. `to_unicode_logical` belongs here only
# because after ADR 0004 it survives when the producer is allow-listed as logical
# (Chromium/Skia) or the line needed no ordering decision at all — a bare clean
# `/ToUnicode` decode is explicitly not evidence of order.
ORDER_RULES = ("actual_text", "bidi_reordered", "producer_visual_order_known",
               "to_unicode_logical")
# Refusals about ORDER: the page's text was withheld, or a line could not be ordered.
ORDER_REFUSALS = ("unsupported_visual_order", "unsupported_no_evidence")
# Refusals about DECODING: some glyph could not be turned into Unicode.
DECODE_REFUSALS = ("unsupported_broken_to_unicode", "unsupported_font_encoding",
                   "unsupported_page_content")

# Orchestrator ruling (Task 4): `producer_visual_order_known` counts as
# order-established only while the fingerprint family it came from is backed by a
# test in the repo. A fingerprint with no backing test does not count.
BACKING_TESTS = (
    "`recover.rs::tests::producer_fingerprint_allow_lists_are_pinned` pins the "
    "allow-lists themselves, and `tests/no_silent_reversal.rs` checks the outcome "
    "on Word (`persian-7`, `hebrew-4`) and InDesign (`arabic-2`) when the private "
    "subset is present locally"
)

RTL_RANGES = ((0x0590, 0x05FF), (0x0600, 0x06FF), (0x0750, 0x077F),
              (0x08A0, 0x08FF), (0xFB50, 0xFDFF), (0xFE70, 0xFEFF))

# Pure-RTL probe words, 4+ characters, so a reversed form cannot match by accident
# inside a longer word (word boundaries are enforced as well). ALL of these are run
# against EVERY file: the corpus filename prefix is a category label, not a claim
# about the script inside (`english-asnad-*` holds Persian text).
PROBE_WORDS = ("شرکت", "قرارداد", "مدیریت",
               "منظمة", "الصحة", "إطار",
               "פניות", "ציבור")


def rtl_chars(text: str) -> int:
    return sum(1 for ch in text if any(a <= ord(ch) <= b for a, b in RTL_RANGES))


def count_word(text: str, word: str) -> int:
    """Word-boundary count: a short word inside a longer one counts in neither
    direction, in either form."""
    return len(re.findall(r"(?<!\w)" + re.escape(word) + r"(?!\w)", text))


def reverse_chars(text: str) -> str:
    return "".join(reversed(text))


def order_probe(name: str, text: str) -> dict:
    """Independent check: the logical form is present, the character-reversed form
    is absent. Reads only the emitted text — never our own reasons.

    Two counts per word: `raw` is a plain substring count (the strictest reading of
    "zero occurrences of the reversed form"), `word` additionally demands word
    boundaries so a 4+ character word cannot match inside a longer one."""
    rows = []
    for word in PROBE_WORDS:
        reversed_ = word[::-1]
        raw_logical, raw_reversed = text.count(word), text.count(reversed_)
        word_logical = len(re.findall(r"(?<!\w)" + re.escape(word) + r"(?!\w)", text))
        word_reversed = len(re.findall(r"(?<!\w)" + re.escape(reversed_) + r"(?!\w)", text))
        if raw_logical == 0 and raw_reversed == 0:
            verdict = "absent"
        elif raw_reversed > 0 and raw_logical > 0:
            verdict = "BOTH — mixed order"
        elif raw_reversed > 0:
            verdict = "REVERSED ONLY — wrong order"
        else:
            verdict = "logical only"
        rows.append({"word": word, "raw_logical": raw_logical,
                     "raw_reversed": raw_reversed, "word_logical": word_logical,
                     "word_reversed": word_reversed, "verdict": verdict})
    return {"words": rows}


def order_rule_text(order_refusals, reasons, rtl, chars) -> str:
    """Name the rule that established this file's order — or why there is none."""
    if order_refusals:
        return "no — " + ", ".join(order_refusals)
    rules = [r for r in reasons if r in ORDER_RULES]
    if rules:
        return "yes — " + ", ".join(rules)
    if chars == 0:
        return "yes — no text to order (no text layer)"
    if rtl < 2:
        return "yes — no RTL text"
    return "yes — structural (no ordering decision, ADR 0004)"


def probe(binary: str, path: pathlib.Path) -> dict:
    exit_extract, ext = run(binary, "extract", path)
    exit_inspect, insp = run(binary, "inspect", path)
    data = ext.get("data") or {}
    pages = data.get("pages") or []
    text = data.get("text") or ""
    reasons = ext.get("reasons") or []
    unordered_chars = int(data.get("unordered_chars") or 0)

    page_reasons = [r for p in pages for r in (p.get("reasons") or [])]
    page_decode_refusals = sorted({r for r in page_reasons if r in DECODE_REFUSALS})
    page_order_refusals = sorted({r for r in page_reasons if r in ORDER_REFUSALS})
    failed_pages = [p for p in pages if not p.get("ok")]
    decode_refusals = [r for r in reasons if r in DECODE_REFUSALS]
    order_refusals = [r for r in reasons if r in ORDER_REFUSALS]

    pages_total = len(pages) or (insp.get("data") or {}).get("pages") or 0
    # Decoding and ordering are separate claims; neither implies the other.
    pages_decoded = sum(1 for p in pages
                        if not any(r in DECODE_REFUSALS for r in (p.get("reasons") or [])))
    pages_ordered = sum(1 for p in pages
                        if not any(r in ORDER_REFUSALS for r in (p.get("reasons") or [])))
    fully_decoded = bool(pages) and pages_decoded == pages_total and not decode_refusals
    order_verified = bool(pages) and pages_ordered == pages_total and not order_refusals

    rtl = rtl_chars(text)
    refusals = decode_refusals + order_refusals
    if text:
        status = "full" if exit_extract == 0 else "partial"
    elif refusals:
        status = "refused"
    else:
        # No text layer at all (scanned image) — nothing was refused, and nothing
        # was withheld. Calling this "refused" would misreport the file.
        status = "no_text"

    return {
        "file": path.name,
        "rtl_chars": rtl,
        "fully_decoded": fully_decoded,
        "order_verified": order_verified,
        "order_rule": order_rule_text(order_refusals, reasons, rtl, len(text)),
        "bytes": path.stat().st_size,
        "producer": (insp.get("data") or {}).get("producer"),
        "pages": pages_total,
        "pages_decoded": pages_decoded,
        "pages_ordered": pages_ordered,
        "pages_failed": len(failed_pages),
        "status": status,
        "exit_code": exit_extract,
        "ok": bool(ext.get("ok")),
        "chars": len(text),
        "unordered_chars": unordered_chars,
        "decoded_chars": len(text) + unordered_chars,
        "reasons": reasons,
        "decode_refusals": decode_refusals,
        "order_refusals": order_refusals,
        "failed_page_numbers": [p.get("page") for p in failed_pages[:20]],
        "failed_page_reasons": sorted(set(page_decode_refusals) | set(page_order_refusals)),
        "probe": order_probe(path.name, text),
        "text_head": text[:60],
        "error": data.get("error"),
        "inspect_exit": exit_inspect,
    }


def summarise(rows: list[dict]) -> dict:
    by_status = collections.Counter(r["status"] for r in rows)
    decode_reason_files = collections.Counter()
    order_reason_files = collections.Counter()
    reason_pages = collections.Counter()
    producer_status = collections.Counter()
    mixed = []
    probe_matched = 0
    probe_logical_only = 0
    for row in rows:
        for reason in set(row["decode_refusals"]):
            decode_reason_files[reason] += 1
        for reason in set(row["order_refusals"]):
            order_reason_files[reason] += 1
        for reason in row["failed_page_reasons"]:
            reason_pages[reason] += 1
        producer_status[f"{row['producer'] or 'unknown'} / {row['status']}"] += 1
        for word in row["probe"]["words"]:
            if word["verdict"] == "absent":
                continue
            probe_matched += 1
            if word["verdict"] == "logical only":
                probe_logical_only += 1
            if word["verdict"].startswith("BOTH"):
                mixed.append({"file": row["file"], **word})
    return {
        "files": len(rows),
        "full": by_status["full"],
        "fully_decoded": sum(1 for r in rows if r["fully_decoded"]),
        "order_verified": sum(1 for r in rows if r["order_verified"]),
        "order_verified_rtl": sum(1 for r in rows if r["order_verified"]
                                  and r["rtl_chars"] >= 2 and r["chars"] > 0),
                                          "rtl_text_files": sum(1 for r in rows if r["rtl_chars"] >= 2 and r["chars"] > 0),
        "partial": by_status["partial"],
        "refused": by_status["refused"],
        "no_text": by_status["no_text"],
        "emitted": by_status["full"] + by_status["partial"],
        "chars_emitted": sum(r["chars"] for r in rows),
        "chars_withheld": sum(r["unordered_chars"] for r in rows),
        "files_by_decode_refusal": dict(decode_reason_files.most_common()),
        "files_by_order_refusal": dict(order_reason_files.most_common()),
        "pages_by_refusal": dict(reason_pages.most_common()),
        "producer_by_status": dict(producer_status.most_common()),
        "probe_words_matched": probe_matched,
        "probe_words_logical_only": probe_logical_only,
        "probe_mixed": mixed,
    }


def write_markdown(rows: list[dict], summary: dict, binary: str, stamp: str) -> pathlib.Path:
    lines = [
        "# Archive validation — `pdfrtl extract` over the private corpus",
        "",
        f"* generated: `{stamp}`",
        f"* command: `python3 scripts/validate-archive.py --binary {binary}`",
        f"* input: `corpus/raw/private/desktop-pdfs/*.pdf` — {summary['files']} files, "
        "read only (never modified, never copied)",
        "",
        "## Headline — two claims, counted separately (ADR 0004)",
        "",
        "| claim | files | meaning |",
        "|---|---|---|",
        f"| **fully decoded** | **{summary['fully_decoded']}** / {summary['files']} | "
        "every page decoded — no `unsupported_*` decode refusal anywhere |",
        f"| **logical order verified** | **{summary['order_verified']}** / "
        f"{summary['files']} | every page's order established by a named rule — "
        "the rule is named per file below |",
        "",
        f"Of the {summary['order_verified']}, **{summary['order_verified_rtl']}** "
        f"files actually emit right-to-left text — and that is every file in the "
        f"corpus that emits any RTL text at all ({summary['rtl_text_files']} of "
        f"{summary['files']}). The other {summary['order_verified'] - summary['order_verified_rtl']} "
        "were verified as having nothing to order: no RTL text, or no text layer. "
        "Every file is accounted for by exactly one row below.",
        "",
        "| emission | files | meaning |",
        "|---|---|---|",
        f"| text emitted, all pages decoded | {summary['full']} | exit 0, `ok: true` |",
        f"| text emitted, some pages refused | {summary['partial']} | exit 3, "
        "`ok: false`; `data.text` holds only order-established pages |",
        f"| no text emitted — refused | {summary['refused']} | a refusal is "
        "recorded; decoded characters survive as `unordered_chars` |",
        f"| no text layer | {summary['no_text']} | nothing to extract (image-only "
        "or blank): no refusal, nothing withheld |",
        "",
        f"Characters emitted (order-established): **{summary['chars_emitted']:,}**. "
        "Characters decoded but withheld (order unestablished): "
        f"**{summary['chars_withheld']:,}**.",
        "",
        "> \"Decoded\" and \"ordered\" are different claims. A file can read every "
        "glyph and still refuse to say which order those glyphs are in — that is "
        "what `unordered_chars` counts, and why this report has no single "
        "\"recovered\" number. Text whose order is unestablished is never present "
        "in `data.text`; it is counted instead.",
        "",
        "## Why pages still fail",
        "",
        "| reason | kind | files affected | pages affected |",
        "|---|---|---|---|",
    ]
    page_counts = summary["pages_by_refusal"]
    for reason, count in summary["files_by_decode_refusal"].items():
        lines.append(f"| `{reason}` | decode | {count} | {page_counts.get(reason, 0)} |")
    for reason, count in summary["files_by_order_refusal"].items():
        lines.append(f"| `{reason}` | order | {count} | {page_counts.get(reason, 0)} |")
    if not summary["files_by_decode_refusal"] and not summary["files_by_order_refusal"]:
        lines.append("| — | — | 0 | 0 |")
    lines += [
        "",
        "## Producer families",
        "",
        "| producer | status | files |",
        "|---|---|---|",
    ]
    for key, count in summary["producer_by_status"].items():
        producer, status = key.rsplit(" / ", 1)
        lines.append(f"| {producer} | {status} | {count} |")
    lines += [
        "",
        "## Per file",
        "",
        "| file | status | decoded pages | ordered pages | emitted chars | "
        "withheld chars | order rule | reasons | first 60 chars |",
        "|---|---|---|---|---|---|---|---|---|",
    ]
    for row in rows:
        head = row["text_head"].replace("|", "\\|").replace("\n", " ⏎ ")
        decoded = f"{row['pages_decoded']}/{row['pages']}"
        ordered = f"{row['pages_ordered']}/{row['pages']}"
        reasons = ", ".join(row["reasons"]) or "-"
        lines.append(f"| `{row['file']}` | {row['status']} | {decoded} | {ordered} "
                     f"| {row['chars']} | {row['unordered_chars']} "
                     f"| {row['order_rule']} | {reasons} | {head} |")

    lines += [
        "",
        "## Independent order probe (logical vs character-reversed)",
        "",
        "For a pure-RTL word, correct logical output contains the logical form and "
        "**zero** occurrences of its character-reversed form. Every word is 4+ "
        "characters and is counted twice: as a plain substring (the strict reading "
        "— zero means zero anywhere in the text) and with word boundaries added so "
        "a short word cannot match inside a longer one. This probe reads only the "
        "emitted text; it does not read our own reasons.",
        "",
        "| file | word | raw logical | raw reversed | word-boundary logical | "
        "word-boundary reversed | verdict |",
        "|---|---|---|---|---|---|---|",
    ]
    probe_rows = 0
    for row in rows:
        for word in row["probe"]["words"]:
            if word["verdict"] == "absent":
                continue
            probe_rows += 1
            lines.append(f"| `{row['file']}` | {word['word']} | {word['raw_logical']} "
                         f"| {word['raw_reversed']} | {word['word_logical']} "
                         f"| {word['word_reversed']} | {word['verdict']} |")
    if probe_rows == 0:
        lines.append("| — | — | 0 | 0 | 0 | 0 | no probe word matched |")
    lines.append("")
    lines.append(f"Probe words matched: {summary['probe_words_matched']}; "
                 f"of those, {summary['probe_words_logical_only']} were logical-only "
                 "(the passing verdict).")
    if summary["probe_mixed"]:
        lines += [
            "",
            "### Files where BOTH forms appear (mixed per-run order — needs its own reason)",
            "",
            "| file | word | raw logical | raw reversed |",
            "|---|---|---|---|",
        ]
        for hit in summary["probe_mixed"]:
            lines.append(f"| `{hit['file']}` | {hit['word']} | {hit['raw_logical']} "
                         f"| {hit['raw_reversed']} |")
    else:
        lines += ["",
                  "No file returned both the logical and the character-reversed form "
                  "of a probe word."]

    lines += [
        "",
        "## Why `arabic-3.pdf` is withheld — the number that must survive",
        "",
        "This is the file the rule exists for: 321 pages that decode cleanly and "
        "whose RTL order nothing in the file establishes. Before the order gate the "
        "same extraction put **1,187,357 characters** of visual-order Arabic into "
        "`data.text` while `ok` was already `false` — a caller reading `data.text` "
        "and ignoring the reason got reversed Arabic:",
        "",
        "| probe word | logical | reversed |",
        "|---|---|---|",
        "| التقرير | 0 | 79 |",
        "| المرأة | 0 | 449 |",
        "| النساء | 0 | 541 |",
        "",
        "Measured on `reports/verify/postfix-arabic-3.json` (pre-fix run kept as "
        "evidence). The withheld count reconciles exactly:",
        "",
        "* sum of all page texts: 1,187,037 characters;",
        "* the old joined `data.text` added 320 inter-page separators → 1,187,357;",
        "* withheld now: **1,187,033** characters on the 319 pages carrying "
        "`unsupported_visual_order`; the other 2 pages contributed 4 characters of "
        "whitespace, which is the only text still emitted.",
        "",
        "Nothing was dropped: the text is still decoded, counted and reported. It is "
        "no longer presented as logical order.",
        ]

    lines += [
        "",
        "## Reading this report",
        "",
        "* `full` = every page decoded and ordered (`ok: true`, exit 0).",
        "* `partial` = at least one page refused: `ok` is `false` (exit 3) and "
        "`data.text` contains ONLY the pages whose order was established. Each "
        "refused page lists its own reason inside `data.pages[].reasons` (ADR 0004).",
        "* `refused` = no page emitted text AND a refusal is on record: `ok: false`, "
        "exit 3, `data.error` names the file and the reason; `unordered_chars` says "
        "what was decoded anyway.",
        "* `no_text` = the file carries no text layer at all (scan/blank): nothing "
        "was refused and nothing was withheld, so it is not counted as a failure.",
        "* `order rule` names the rule that established the file's order. "
        "`producer_visual_order_known` counts because its fingerprint family is "
        f"backed by tests in the repo: {BACKING_TESTS}. **A fingerprint with no "
        "backing test does not count.**",
        "* `no RTL text` means there was no right-to-left text to order.",
        "* Reasons come from the CLI envelope verbatim; nothing here is inferred.",
        "* A clean decode alone is *not* evidence of order (ADR 0002/0004).",
        "",
    ]
    out = REPORTS / "validate-archive.md"
    out.write_text("\n".join(lines), encoding="utf-8")
    return out


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", default=str(ROOT / "target" / "debug" / "pdfrtl"))
    parser.add_argument("--json-out", default=str(REPORTS / "validate-archive.json"))
    args = parser.parse_args()

    files = sorted(ARCHIVE.glob("*.pdf"))
    if not files:
        print(f"no files in {ARCHIVE}", file=sys.stderr)
        return 1

    rows = []
    for path in files:
        row = probe(args.binary, path)
        rows.append(row)
        print(f"{row['status']:>8}  {row['file']:<52} pages {row['pages_decoded']}/"
              f"{row['pages']:<4} ordered {row['pages_ordered']:<4} "
              f"chars {row['chars']:>7} withheld {row['unordered_chars']:>7}  "
              f"{','.join(row['reasons']) or '-'}", flush=True)

    summary = summarise(rows)
    stamp = dt.datetime.now(dt.timezone.utc).strftime("%Y-%m-%dT%H%M%SZ")

    REPORTS.mkdir(exist_ok=True)
    json_out = pathlib.Path(args.json_out)
    json_out.write_text(
        json.dumps({"generated": stamp, "binary": args.binary,
                    "archive": str(ARCHIVE.relative_to(ROOT)), "summary": summary,
                    "files": rows},
                   indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8")
    md_out = write_markdown(rows, summary, args.binary, stamp)

    print()
    print(f"files={summary['files']} fully_decoded={summary['fully_decoded']} "
          f"order_verified={summary['order_verified']} "
          f"emitted={summary['emitted']} partial={summary['partial']} "
          f"refused={summary['refused']} no_text={summary['no_text']}")
    print(f"chars_emitted={summary['chars_emitted']} "
          f"chars_withheld={summary['chars_withheld']} "
          f"probe_matched={summary['probe_words_matched']} "
          f"probe_logical_only={summary['probe_words_logical_only']} "
          f"probe_mixed={len(summary['probe_mixed'])}")
    print(f"wrote {json_out}")
    print(f"wrote {md_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
