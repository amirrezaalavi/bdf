#!/usr/bin/env python3
"""Diff two `validate-archive.json` reports: what a change to the extractor actually did.

Prints only numeric fields and named rule strings — never a document's text. That is
deliberate: this output is meant to be read in a terminal, pasted into a commit message and
cited in a problem doc, so it must be safe to look at without exposing the corpus.

Aggregate movement is not enough on its own. A change can move thousands of characters
without altering a single file's verdict, or move none while flipping a whole family from
refused to verified, so the two are reported separately and every verdict change is named.
"""
import argparse
import json
import pathlib

RULE_KEYS = ("order_rule", "decode_rule", "status", "verdict", "category")
MAX_MOVES = 40


def load(path: str) -> dict:
    data = json.loads(pathlib.Path(path).read_text(encoding="utf-8"))
    rows = data["files"] if isinstance(data, dict) and "files" in data else data
    return {row["file"]: row for row in rows}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("before")
    parser.add_argument("after")
    args = parser.parse_args()
    before, after = load(args.before), load(args.after)
    print(f"files: before={len(before)} after={len(after)}")
    for name in sorted(set(before) - set(after)):
        print(f"  only in before: {name}")
    for name in sorted(set(after) - set(before)):
        print(f"  only in after : {name}")

    totals_before: dict[str, int] = {}
    totals_after: dict[str, int] = {}
    verdict_changes = []
    moves: dict[str, list[str]] = {}
    for name in sorted(set(before) & set(after)):
        old, new = before[name], after[name]
        for key, value in old.items():
            if isinstance(value, int) and not isinstance(value, bool):
                other = new.get(key) if isinstance(new.get(key), int) else 0
                totals_before[key] = totals_before.get(key, 0) + value
                totals_after[key] = totals_after.get(key, 0) + other
                if value != other:
                    moves.setdefault(name, []).append(f"{key} {value}->{other}")
        for key in RULE_KEYS:
            if (key in old or key in new) and old.get(key) != new.get(key):
                verdict_changes.append((name, key, old.get(key), new.get(key)))

    print("\nnumeric totals — only the fields that moved:")
    for key in sorted(totals_before):
        delta = totals_after.get(key, 0) - totals_before[key]
        if delta:
            print(f"  {key:26} {totals_before[key]:>10} -> {totals_after.get(key, 0):>10}  ({delta:+})")

    print(f"\nverdict changes: {len(verdict_changes)}")
    for name, key, old, new in verdict_changes:
        print(f"  {name[:44]:44} {key}: {old} -> {new}")

    print(f"\nfiles with a numeric move: {len(moves)}")
    for name in sorted(moves)[:MAX_MOVES]:
        print(f"  {name[:44]:44} {'; '.join(moves[name])[:110]}")
    if len(moves) > MAX_MOVES:
        print(f"  ... and {len(moves) - MAX_MOVES} more")


if __name__ == "__main__":
    main()
