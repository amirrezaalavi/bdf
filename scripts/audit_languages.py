#!/usr/bin/env python3
"""Audit every manifest language label against POSITIVE script evidence.

Why this exists (docs/problems/0021): a headline number in a reader-facing document must be
produced by a script, and a label in the manifest is a CLAIM about a file, not evidence of one.
Measured 2026-10-04: several rows labelled `fa` carry zero Arabic-script characters on any page,
so every per-language acceptance number derived from the manifest scores English documents as
RTL successes.

The decision rule is deliberately asymmetric, and the asymmetry is the whole point:

  * A label is CONFIRMED only when the script it claims is POSITIVELY PRESENT in extracted text.
  * A label is MISMATCHED only when the claimed script is absent AND a DIFFERENT script is
    positively present in quantity. Absence alone never upgrades to "English": the layers that
    carry the text also carry the script evidence, and they are usually the layers that failed,
    so a file that withheld everything would otherwise be relabelled by elimination.
  * A label is UNDETERMINED when no script is positively present — refused, no text layer, or
    genuinely scriptless. Those rows are reported, never rewritten.

The script therefore proposes nothing and writes no manifest. It prints the evidence table; a
relabel is a human decision applied to the manifest, recorded in an ADR.

Usage:
    python3 scripts/audit_languages.py --binary target/debug/pdfrtl [--json]

Exit codes:
    0  audit ran (MISMATCH rows may or may not exist — see the table)
    2  the private corpus is absent, so nothing can be audited. Refusing beats printing zeros
       (docs/ACCEPTANCE.md §5): an empty table on a public clone invites the next reader to
       quote zeros as measurements.
"""
from __future__ import annotations

import argparse
import collections
import json
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
ARCHIVE = ROOT / "corpus" / "raw" / "private" / "desktop-pdfs"
REPORTS = ROOT / "reports"

# Unicode blocks per script. Deliberately block-based rather than "not ASCII": a Persian
# document with a Latin product code still carries thousands of Arabic-script characters, and
# an English one carries zero — the margin is enormous either way, so a threshold on
# presence-with-volume is safer than any attempt at per-character classification.
SCRIPT_RANGES: dict[str, tuple[tuple[int, int], ...]] = {
    "Arab": ((0x0600, 0x06FF), (0x0750, 0x077F), (0x08A0, 0x08FF),
             (0xFB50, 0xFDFF), (0xFE70, 0xFEFF)),
    "Hebr": ((0x0590, 0x05FF),),
    "Latn": ((0x0041, 0x005A), (0x0061, 0x007A), (0x00C0, 0x024F), (0x1E00, 0x1EFF)),
    "Cyrl": ((0x0400, 0x04FF),),
    "Zyyy": (),  # not a script: "undetermined"/common. Present so the key is documented.
}

# Which script a language label is entitled to assert. This is a normalisation table, not a
# judgement: `persian` and `fa` both mean Arabic script, and both appear in the manifest.
LABEL_SCRIPT: dict[str, frozenset[str]] = {
    "fa": frozenset({"Arab"}),
    "persian": frozenset({"Arab"}),
    "ar": frozenset({"Arab"}),
    "he": frozenset({"Hebr"}),
    "he-eng": frozenset({"Hebr", "Latn"}),
    "en": frozenset({"Latn"}),
    "english": frozenset({"Latn"}),
    "und": frozenset(),  # asserts nothing, so it can never mismatch
}

# A script counts as POSITIVELY PRESENT above this many characters. Set from measurement, not
# taste: mixed RTL/LTR documents in this corpus sit in the tens of thousands, while incidental
# script bleed (one vendor name, one page footer) sits in the tens. Two orders of magnitude
# apart, so any threshold in between is stable; the exact value is not load-bearing.
PRESENCE_THRESHOLD = 200

# What share of the file's letters must the claimed script hold before we call a label
# confirmed? A file that is 95% Persian and 5% English is Persian. One that is 60/40 is
# reported as MIXED rather than silently assigned to either.
DOMINANT_SHARE = 0.60

OUTCOME_CONFIRMED = "confirmed"
OUTCOME_MISMATCH = "mismatch"
OUTCOME_MIXED = "mixed"
OUTCOME_UNDETERMINED = "undetermined"


def count_scripts(text: str) -> dict[str, int]:
    counts = {name: 0 for name in SCRIPT_RANGES}
    for ch in text:
        cp = ord(ch)
        for name, ranges in SCRIPT_RANGES.items():
            if any(lo <= cp <= hi for lo, hi in ranges):
                counts[name] += 1
                break
    return counts


def extract(binary: str, path: pathlib.Path) -> tuple[int, dict]:
    proc = subprocess.run([binary, "--json", "extract", str(path)],
                          capture_output=True, text=True, cwd=ROOT)
    try:
        return proc.returncode, json.loads(proc.stdout)
    except json.JSONDecodeError:
        return proc.returncode, {"data": {}, "reasons": [], "ok": False}


def audit_file(binary: str, row: dict, path: pathlib.Path) -> dict:
    _, envelope = extract(binary, path)
    data = envelope.get("data") or {}
    text = data.get("text") or ""
    counts = count_scripts(text)
    letters = sum(counts.values())

    label = (row.get("language") or "und").lower()
    entitled = LABEL_SCRIPT.get(label)
    if entitled is None:
        # An unknown label is a manifest-schema problem, not a file problem.
        outcome = "unknown-label"
    elif not entitled:
        outcome = OUTCOME_UNDETERMINED  # `und` asserts nothing
    elif letters == 0:
        outcome = OUTCOME_UNDETERMINED  # nothing to be evidence about
    else:
        present = {s for s, n in counts.items() if n >= PRESENCE_THRESHOLD}
        missing = entitled - present          # scripts the label asserts but the file lacks
        if missing:
            # The label claims a script the file does not carry. Whether that is a `mismatch` or
            # only `mixed` depends on how decisively the file is something else, so absence alone
            # is never enough: a 95%-Persian report with a large English appendix must not read as
            # mislabelled, and a 9-character English string must not either.
            #   measured defect 1: without the dominance check, `fa` over Persian-dominant text
            #     with an English appendix was reported `mismatch`.
            #   measured defect 2: `he-eng` over pure English was reported `confirmed`, because
            #     finding the Latin half satisfied the label — which would let an English-only
            #     file score as a Hebrew success. Finding one of several asserted scripts is not
            #     confirmation of a label that asserts all of them.
            if not present:
                outcome = OUTCOME_UNDETERMINED
            else:
                share = sum(counts[s] for s in present) / letters
                outcome = OUTCOME_MISMATCH if share >= DOMINANT_SHARE else OUTCOME_MIXED
        elif present - entitled:
            # Every asserted script is present, but so is a script the label does not claim.
            share = sum(counts[s] for s in entitled) / letters
            outcome = OUTCOME_CONFIRMED if share >= DOMINANT_SHARE else OUTCOME_MIXED
        else:
            share = sum(counts[s] for s in entitled) / letters
            outcome = OUTCOME_CONFIRMED if share >= DOMINANT_SHARE else OUTCOME_MIXED

    return {
        "id": row.get("id"),
        "file": path.name,
        "label": label,
        "label_script_field": row.get("script"),
        "outcome": outcome,
        "letters": letters,
        "chars": len(text),
        "unordered_chars": int(data.get("unordered_chars") or 0),
        "pages": len(data.get("pages") or []),
        "counts": {k: v for k, v in counts.items() if v},
        "shares": {k: round(v / letters, 4) for k, v in counts.items() if v} if letters else {},
        "reasons": envelope.get("reasons") or [],
    }


def summarise(rows: list[dict]) -> dict:
    by_outcome = collections.Counter(r["outcome"] for r in rows)
    by_label = collections.Counter(r["label"] for r in rows)
    # Mislabelling inflates the focus language, so count it in the direction that hurts:
    # rows that CLAIM the focus language but do not carry its script.
    claims_focus = [r for r in rows if r["label"] in ("fa", "persian")]
    false_focus = [r for r in claims_focus if r["outcome"] == OUTCOME_MISMATCH]
    real_focus = [
        r for r in rows
        if r["outcome"] in (OUTCOME_CONFIRMED, OUTCOME_MIXED)
        and counts_has(r, "Arab")
    ]
    return {
        "files": len(rows),
        "by_outcome": dict(by_outcome),
        "by_label": dict(by_label),
        "focus_claimed": len(claims_focus),
        "focus_false_claimed": len(false_focus),
        "focus_false_claimed_ids": [r["id"] for r in false_focus],
        "focus_true": len(real_focus),
        "labels_with_no_script_mapping": sorted(
            {r["label"] for r in rows if r["outcome"] == "unknown-label"}
        ),
        "script_field_vocabulary": sorted(
            {r["label_script_field"] for r in rows if r["label_script_field"]}
        ),
    }


def counts_has(row: dict, script: str) -> bool:
    return row["counts"].get(script, 0) >= PRESENCE_THRESHOLD


def render(rows: list[dict], summary: dict) -> str:
    out = [
        "# Language-label audit — manifest claim vs measured script evidence",
        "",
        "Produced by `python3 scripts/audit_languages.py`. No number here is typed by hand.",
        "",
        f"* files audited: **{summary['files']}**",
        f"* presence threshold: {PRESENCE_THRESHOLD} characters of a script",
        f"* dominance threshold: {DOMINANT_SHARE:.0%} of letters",
        "",
        "## Outcome",
        "",
        "| outcome | files | meaning |",
        "|---|---|---|",
        f"| `{OUTCOME_CONFIRMED}` | {summary['by_outcome'].get(OUTCOME_CONFIRMED, 0)} "
        "| the label's script is present and dominant |",
        f"| `{OUTCOME_MIXED}` | {summary['by_outcome'].get(OUTCOME_MIXED, 0)} "
        "| the label's script is present but not dominant — needs a human call |",
        f"| `{OUTCOME_MISMATCH}` | {summary['by_outcome'].get(OUTCOME_MISMATCH, 0)} "
        "| the claimed script is absent and another script is present |",
        f"| `{OUTCOME_UNDETERMINED}` | {summary['by_outcome'].get(OUTCOME_UNDETERMINED, 0)} "
        "| no script positively present — refused, no text layer, or scriptless |",
        "",
        "## The focus language, counted in the direction that hurts",
        "",
        f"* rows claiming Persian (`fa`/`persian`): **{summary['focus_claimed']}**",
        f"* of those, carrying no Arabic script at all: **{summary['focus_false_claimed']}**",
        f"* rows that actually carry Arabic script: **{summary['focus_true']}**",
        "",
    ]
    if summary["focus_false_claimed_ids"]:
        out += ["Misclaimed ids: " + ", ".join(f"`{i}`" for i in summary["focus_false_claimed_ids"]), ""]
    out += [
        "## Manifest vocabulary",
        "",
        f"* language values in use: {', '.join(f'`{k}`' for k in summary['by_label'])}",
        f"* script values in use: {', '.join(f'`{k}`' for k in summary['script_field_vocabulary'])}",
    ]
    if summary["labels_with_no_script_mapping"]:
        out.append(
            "* labels with no script mapping (schema gap): "
            + ", ".join(f"`{k}`" for k in summary["labels_with_no_script_mapping"])
        )
    out += [
        "",
        "## Per file",
        "",
        "| id | label | outcome | letters | Arab | Hebr | Latn | reasons |",
        "|---|---|---|---|---|---|---|---|",
    ]
    for r in sorted(rows, key=lambda x: (x["outcome"], x["id"] or "")):
        counts = r["counts"]
        out.append(
            f"| `{r['id']}` | `{r['label']}` | **{r['outcome']}** | {r['letters']:,} "
            f"| {counts.get('Arab', 0):,} | {counts.get('Hebr', 0):,} "
            f"| {counts.get('Latn', 0):,} | {','.join(r['reasons']) or '-'} |"
        )
    out += [
        "",
        "## Reading this report",
        "",
        "* `mismatch` is evidence of a wrong label, not a licence to guess a replacement: the "
        "replacement must itself be positive evidence, which is why the table carries the "
        "counts rather than a proposed new label.",
        "* `undetermined` is never relabelled. A file that refused to emit text tells us "
        "nothing about which script it holds.",
        "* `mixed` needs a reader, not a threshold.",
        "",
    ]
    return "\n".join(out)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--binary", default=str(ROOT / "target" / "debug" / "pdfrtl"))
    ap.add_argument("--json", action="store_true")
    args = ap.parse_args()

    files = sorted(ARCHIVE.glob("*.pdf"))
    if not files:
        print(
            f"no private PDFs under {ARCHIVE.relative_to(ROOT)} — nothing to audit.\n"
            "This corpus is owner-local by design. On a public clone this script must REFUSE "
            "(exit 2) rather than print an empty table, because an empty table reads as "
            "'no mislabels found' when it means 'nothing measured'.",
            file=sys.stderr,
        )
        return 2

    manifest_path = ROOT / "corpus/manifest.json"
    if not manifest_path.exists():
        print(f"manifest absent at {manifest_path.relative_to(ROOT)}", file=sys.stderr)
        return 2
    manifest = json.loads(manifest_path.read_text())
    by_path = {r["path"]: r for r in manifest["fixtures"]}

    rows = []
    for path in files:
        rel = str(path.relative_to(ROOT))
        row = by_path.get(rel) or by_path.get(f"corpus/raw/private/desktop-pdfs/{path.name}")
        if row is None:
            # A manifest gap, reported rather than skipped silently (corpus/README.md rule 1).
            row = {"id": f"<no manifest row: {path.name}>", "language": "und",
                   "script": None}
        rows.append(audit_file(args.binary, row, path))

    summary = summarise(rows)
    if args.json:
        print(json.dumps({"summary": summary, "files": rows}, indent=2,
                         ensure_ascii=False, sort_keys=True))
        return 0

    text = render(rows, summary)
    REPORTS.mkdir(exist_ok=True)
    (REPORTS / "audit-languages.md").write_text(text, encoding="utf-8")
    (REPORTS / "audit-languages.json").write_text(
        json.dumps({"summary": summary, "files": rows}, indent=2,
                   ensure_ascii=False, sort_keys=True) + "\n", encoding="utf-8")
    print(text)
    print(f"wrote {REPORTS / 'audit-languages.md'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
