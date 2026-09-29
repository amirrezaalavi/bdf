#!/usr/bin/env python3
"""Check the ORDER of our own output against an independent extractor.

The invariant is "logical order or an explicit refusal", and a page-level reason code is our
own claim about our own output. This cross-checks it with a second implementation: poppler's
`-layout -enc UTF-8` returns logical order (measured; it flips lam-alef pairs, so it is never
used to judge text IDENTITY — only order).

Method: keep only RTL codepoints on both sides and score ADJACENT CHARACTER PAIRS. Compare
our output against the reference and against its reversal; the invariant predicts a high
score forward and a low score backward. Both numbers are printed, never the text, so the
report can be read and cited without exposing a document.

A low forward score on a file we emitted is a real bug: we claimed logical order and shipped
something else. A file we refused is not a failure of this check — it is counted separately.
"""
import argparse
import json
import pathlib
import re
import subprocess
from collections import Counter

RTL = re.compile(
    "[\u0590-\u05FF\u0600-\u06FF\u0750-\u077F\u08A0-\u08FF\uFB1D-\uFDFF\uFE70-\uFEFF]"
)


def rtl_only(text: str) -> str:
    return "".join(ch for ch in text if RTL.match(ch))


def bigram_score(a: str, b: str) -> float:
    """Fraction of `a`'s adjacent pairs that also occur, in order, in `b`."""
    if len(a) < 2 or len(b) < 2:
        return 0.0
    pairs_a = Counter(zip(a, a[1:]))
    pairs_b = Counter(zip(b, b[1:]))
    return sum((pairs_a & pairs_b).values()) / (len(a) - 1)


def reference(path: pathlib.Path, pages: int) -> str:
    """Logical-order reference text for the first `pages` pages."""
    out = []
    for page in range(1, pages + 1):
        done = subprocess.run(
            [
                "pdftotext",
                "-layout",
                "-enc",
                "UTF-8",
                "-f",
                str(page),
                "-l",
                str(page),
                str(path),
                "-",
            ],
            capture_output=True,
            text=True,
            timeout=180,
        )
        if done.returncode != 0:
            break
        out.append(done.stdout)
    return "".join(out)


def our_output(binary: str, path: pathlib.Path) -> tuple[str, bool]:
    done = subprocess.run(
        [binary, "--json", "extract", str(path)],
        capture_output=True,
        text=True,
        timeout=600,
    )
    try:
        payload = json.loads(done.stdout)
    except json.JSONDecodeError:
        return "", False
    data = payload.get("data") or {}
    text = data.get("text") or payload.get("text") or ""
    return text, bool(payload.get("ok"))


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary")
    parser.add_argument("files", nargs="+")
    parser.add_argument("--pages", type=int, default=3)
    args = parser.parse_args()

    refused = []
    emitted = []
    for name in args.files:
        path = pathlib.Path(name)
        if not path.exists():
            print(f"{path.name[:44]:44} MISSING")
            continue
        text, ok = our_output(args.binary, path)
        ours = rtl_only(text)
        if not ok or len(ours) < 20:
            refused.append(path.name)
            print(f"{path.name[:44]:44} refused/empty  rtl_chars={len(ours)}")
            continue
        theirs = rtl_only(reference(path, args.pages))
        forward = bigram_score(ours, theirs)
        backward = bigram_score(ours, theirs[::-1])
        verdict = "LOGICAL" if forward > backward else "!!! NOT LOGICAL"
        emitted.append((path.name, forward, backward, verdict))
        print(
            f"{path.name[:44]:44} emitted  rtl={len(ours):>7}  "
            f"fwd={forward:.3f} rev={backward:.3f}  {verdict}"
        )

    print(f"\nemitted: {len(emitted)}   refused/empty: {len(refused)}")
    wrong = [row for row in emitted if row[3] != "LOGICAL"]
    print(f"emitted but NOT logical order: {len(wrong)}")
    for name, forward, backward, _ in wrong:
        print(f"  {name[:44]:44} fwd={forward:.3f} rev={backward:.3f}")


if __name__ == "__main__":
    main()
