#!/usr/bin/env python3
"""Test the language-label audit's DECISION RULE, not its file I/O.

The rule is the part that can silently invert a headline number, so it is tested here directly
with strings instead of through 51 PDF extractions. Extracting real files proves the plumbing;
only these cases decide whether a mislabelled file is reported or missed.

The asymmetry under test (scripts/audit_languages.py):
    confirmed    — the label's script is POSITIVELY present and dominant
    mismatch     — the claimed script is absent AND another script is positively present
    undetermined — nothing is positively present; absence alone never becomes "English"
    mixed        — claimed script present but not dominant; a reader decides

Run:  python3 tests/test_audit_languages.py
"""
from __future__ import annotations

import importlib.util
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location(
    "audit_languages", ROOT / "scripts" / "audit_languages.py"
)
mod = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mod)


def repeat(word: str, target: int | None = None) -> str:
    """Build text that clears PRESENCE_THRESHOLD BY MEASUREMENT, not by eye.

    The first version of this file repeated a 4-character Persian word 40 times, assumed it
    produced "enough" characters, and the case silently fell below the threshold — so a test
    meant to exercise `confirmed` was really exercising `undetermined` and passed for the wrong
    reason. Sizes are computed from the script's own threshold now.
    """
    need = target if target is not None else mod.PRESENCE_THRESHOLD + 50
    per_run = sum(1 for ch in word if ch.isalpha())
    if per_run == 0:
        return (word + " ") * need
    return " ".join([word] * (-(-need // per_run)))  # ceil division


def classify(label: str, text: str) -> str:
    """Call the script's own rule rather than restating it.

    A test that keeps its own copy of the logic tests the copy: the day the two diverge the
    test keeps passing while the audit does the wrong thing. `audit_file` needs a PDF on disk,
    so the rule is reached by substituting the only part that needs one — the extraction.
    """
    original = mod.extract
    mod.extract = lambda binary, path: (0, {"data": {"text": text, "pages": []},
                                            "reasons": []})
    try:
        row = {"id": "t", "language": label, "script": None}
        return mod.audit_file("unused-binary", row, pathlib.Path("/dev/null"))["outcome"]
    finally:
        mod.extract = original


# Each case names the failure it prevents. A case with no name is a number, not a test.
CASES: list[tuple[str, str, str, str]] = [
    # --- the finding that motivated this script -------------------------------
    ("fa", repeat("This is an English report"), "mismatch",
     "a file labelled fa with only Latin letters must be a mismatch, not relabelled English"),
    ("fa", repeat("سلام دنیا"), "confirmed",
     "repeated Persian must clear the presence threshold and confirm the label"),
    # --- absence must NOT become 'English' -----------------------------------
    ("fa", "", "undetermined",
     "a refused file emits nothing; relabelling it by elimination is the defect itself"),
    ("fa", "12345 --- ... ??", "undetermined",
     "digits and punctuation carry no script evidence, so the label stays undetermined"),
    ("fa", "nothing decodable", "undetermined",
     "a short Latin string below the threshold is not evidence of anything"),
    ("und", repeat("This is plainly English"), "undetermined",
     "'und' asserts nothing, so it can never be reported as a mismatch"),
    # --- the focus language must be able to fail ------------------------------
    ("he", repeat("This is English text only"), "mismatch",
     "a Hebrew label over an English document must be caught, not just Persian"),
    ("he", repeat("שלום עולם"), "confirmed", "genuine Hebrew confirms its own label"),
    # --- dominance, not mere presence ----------------------------------------
    ("fa", repeat("سلام دنیا", 900) + " " + repeat("the quick brown fox"), "confirmed",
     "Persian dominant over a Latin minority is confirmed, not called mislabelled"),
    ("he-eng", repeat("hello world"), "mismatch",
     "he-eng asserts Hebrew too; pure English under that label is a mismatch"),
    ("he-eng", repeat("שלום") + " " + repeat("hello"), "confirmed",
     "both asserted scripts present is confirmed"),
    ("ar", repeat("مرحبا بالعالم"), "confirmed", "Arabic confirms its own label"),
    ("en", repeat("An English engineering report"), "confirmed", "English confirms"),
    # --- the shared-vocabulary traps -----------------------------------------
    ("persian", repeat("This is an English report"), "mismatch",
     "'persian' and 'fa' mean the same thing and must fail identically"),
    ("persian", repeat("سلام دنیا"), "confirmed",
     "'persian' must be accepted as Persian, not rejected as a schema gap"),
    # --- threshold is a real boundary, not a magic number --------------------
    ("fa", "س" * (mod.PRESENCE_THRESHOLD + 1), "confirmed",
     "just above the presence threshold confirms"),
    ("fa", "س" * (mod.PRESENCE_THRESHOLD - 1) + " " + repeat("English text", 900), "mismatch",
     "just below the threshold is not presence, and dominant Latin decides the mismatch"),
    # --- a majority direction the label does not assert ----------------------
    ("fa", repeat("the quick brown fox") + " " + repeat("سلام"), "mixed",
     "Latin-dominant under an fa label is neither a clean mismatch nor a confirmation"),
]


def main() -> int:
    failures = []
    for label, text, want, why in CASES:
        got = classify(label, text)
        if got != want:
            failures.append((label, want, got, why))

    print(f"audit_languages decision rule: {len(CASES) - len(failures)}/{len(CASES)} cases pass")
    for label, want, got, why in failures:
        print(f"  FAIL label={label!r}: got {got!r}, want {want!r} — {why}")

    # Every case must actually reach the outcome it claims to test: a fixture that clears no
    # threshold is indistinguishable from one that does, which is how the sizing bug above
    # stayed invisible.
    thin = [label for label, text, want, _ in CASES
            if want in ("confirmed", "mismatch") and not any(ch.isalpha() for ch in text)]
    for label in thin:
        print(f"  FAIL fixture carries no letters at all: label={label!r}")

    # The invariant the whole script exists to protect, as an assertion rather than left to the
    # case list: no RTL label is ever decided from absence alone.
    for label in ("fa", "persian", "ar", "he"):
        for probe in ("", "12345 ---", "a" * 5):
            assert classify(label, probe) == "undetermined", (label, probe)
    print("invariant held: no RTL label is ever decided from absence alone")
    return 1 if (failures or thin) else 0


if __name__ == "__main__":
    sys.exit(main())
