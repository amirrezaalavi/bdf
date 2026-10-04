# Acceptance, measured per language

**Date:** 2026-10-04 · **Source:** `scripts/by_language.py` over `reports/validate-archive.json` · **Corpus:** 51 owner-local private PDFs. Manifest labels for nine records were corrected after owner review; raw identities and per-document report rows remain private.

Run it yourself from an owner-local checkout with the corpus present:

```bash
python3 scripts/by_language.py            # aggregate table
python3 scripts/by_language.py --json     # machine-readable output
```

Every number here is **produced by that script**, never typed. If the private corpus or report is absent, the script exits non-zero rather than printing zeros. These aggregates cannot be reproduced from the public clone.

---

## 1. Private acceptance data is not public

The language audit and per-language extraction outcomes are derived from private customer PDFs. They remain owner-local and are deliberately excluded from tracked documents and the public mirror. Owner-reviewed language-label corrections are reflected only in the local private manifest; public publication strips private manifest rows.

To recompute results locally, run `scripts/validate-archive.sh` and `python3 scripts/by_language.py` in a checkout where the owner has arranged access to the corpus. Do not copy the raw output, per-record rows, private paths, titles, hashes, or extracted text into public artifacts.

## 2. Ordering evidence and refusal gap

The private audit records page-level order decisions, but its result rows remain owner-local. Add public construction-grounded success and refusal fixtures before changing order recovery. Preserve proven-only `data.text`; do not weaken evidence rules to improve private aggregate counts.

## 3. Public evidence

The public clone contains redistributable synthetic fixtures. Run `bash scripts/verify-clone.sh` and `bash scripts/wsl-build.sh` for reproducible public checks. A private-corpus result cannot replace those tests or be reproduced by public contributors.

## 4. Owner review and limits

The owner reviewed the displayed Persian RFP extraction text and said it seemed right. That feedback does not constitute a rendered-page comparison fixture or prove general reading-order correctness. Remaining mixed/undetermined labels require further positive evidence and human adjudication; no inference is made from missing output.
