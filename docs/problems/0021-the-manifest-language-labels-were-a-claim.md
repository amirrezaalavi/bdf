# 0021 — manifest language labels require evidence and review

**Date:** 2026-10-04 · **Status:** owner-adjudicated corrections applied; other labels remain unresolved · **Scope:** `corpus/manifest.json`, `scripts/audit_languages.py`

## What was measured

`python3 scripts/audit_languages.py --binary target/debug/pdfrtl` uses positive script evidence in emitted text and compares it with the manifest's language field. It counts script families; it does not identify Persian versus Arabic or English versus another Latin-script language by itself.

## Owner adjudication

The owner identified the reviewed English-only records as English and the displayed RFP excerpts as Persian content with English terms. The extracted text was reported as seeming right. The corresponding local manifest labels were corrected using that human decision, not by converting a script count into a language claim. Remaining mixed or undetermined labels are unchanged pending positive evidence and human review.

Per-record identifiers, private measurement tables, titles, hashes, raw reports and extracted text remain owner-local and are not included in tracked/public artifacts.

## Two defects in the audit rule itself, found by testing it

Both were caught by `tests/test_audit_languages.py` (18 cases), not by reading the code:

1. **Absence alone was treated as a mismatch.** The first version called any file whose claimed
   script was absent a mismatch, so a 95%-Persian report with a large English appendix read as
   mislabelled — and a 9-character English string did too. The rule now requires the
   alternative script to be **dominant** before calling a mismatch. Absence alone yields
   `undetermined`, which is the honest answer and also what
   `references/real-world-pdf-interrogation.md` requires.
2. **A multi-script label confirmed on one script.** `a multi-script label` asserts both Hebrew and English;
   the first version reported `confirmed` when it found only the Latin half. That would let an
   English-only file score as a Hebrew success. The rule now requires **every** script a label
   asserts to be present before confirming.

A third problem was in the test, not the rule: `"سلام دنیا " * 40` produces 160 Arabic-script
characters, which is **below** the 200 threshold, so a case meant to exercise `confirmed` was
really exercising `undetermined` and passing for the wrong reason. Fixtures are now sized from
the script's own threshold.

## What remains deliberately unresolved

- Only owner-adjudicated labels were corrected. Remaining `mixed` and `undetermined` cases stay unresolved: absence is not evidence, and a refusal cannot establish a document's language.
- Private measurements, paths, titles, hashes, raw reports and extracted text are owner-local and excluded from public publication.
- `albdf` is not an oracle. It was used as a differential comparator only, never to decide language identity or reading order.

## The rule that follows

A label in the manifest is a **claim about a file**, and it is audited like one. Two rules:

1. **A number in a reader-facing document is produced by a script**, never typed, and the
   script must derive its categories from measured evidence rather than from a label
   (`docs/problems/0016`).
2. **Absence of a script is never evidence of a language.** Decide a language only from
   positive evidence, and when nothing is positively present say `undetermined` — a refused
   file tells you nothing about what script it holds.
