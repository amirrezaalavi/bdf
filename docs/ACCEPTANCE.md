# Acceptance, measured per language

**Date:** 2026-10-03 · **Source:** `scripts/by_language.py` over `reports/validate-archive.json`
· **Corpus:** 51 private files, owner's own documents.

Run it yourself:

```bash
python3 scripts/by_language.py            # the table below
python3 scripts/by_language.py --json     # for tooling
```

Every number here is **produced by that script**, never typed. That is the rule from
`docs/problems/0016`: a table that merely sums correctly is not a measurement. If the corpus is
absent the script exits 2 and says so rather than printing zeros.

---

## 1. Persian is the focus — so Persian is the headline

25 of 51 files. **13 emit every decoded character, 8 are refused entirely, 1 is partial, 3 have no
text layer at all.**

| outcome | files | |
|---|---|---|
| fully emitted | 13 / 25 | every decoded character returned in order |
| partial | 1 / 25 | some text withheld |
| refused entirely | 8 / 25 | decoded, order unestablished, reason named |
| no text layer | 3 / 25 | scanned images — OCR territory, out of scope |

**The median Persian file emits 100% of its decoded text.**

### The 8 Persian refusals are one problem, not eight

Measured, every one of them reports `unsupported_visual_order` and nothing else blocks them:

| file | withheld chars | pages |
|---|---|---|
| `persian-report-noc-revision` | 42,107 | 29 |
| `persian-1` | 8,475 | 13 |
| `persian-hld-7-summary-fa` | 6,333 | 3 |
| `persian-2` | 2,076 | 2 |
| `persian-3` | 1,981 | 2 |
| `persian-panel1` | 1,629 | 1 |
| `persian-4` | 1,488 | 1 |
| `persian-6` | 759 | 1 |

One root cause, one ladder rung (rung 3, the geometry comparison), one fix. **8 files is the
entire remaining Persian scope** — not 51, and not one pathological file.

## 2. What actually decides Persian text

The rung that produced the output, per file:

| rung | files | note |
|---|---|---|
| `to_unicode_logical` | 9 | `/ToUnicode` decoded to base letters, stored order already logical |
| `producer_visual_order_known` | 2 | allow-listed producer, inverted per the fingerprint |
| both | 1 | |
| no RTL text / no text layer | 4 | out of scope |
| **rung 3 exhausted → refuse** | **9** | **the remaining work** |

So **two rungs carry 100% of working Persian text today**, and the third never fires
successfully on a Persian file in this corpus. That is where effort belongs: extend rung 3's
hypothesis space (the critique's P-7), and do it after public fixtures exist (P-5), because
`docs/problems/0009`, `0014` and `0015` all say a comparison must not change before the fixtures
that would catch the regression are in place.

## 3. All languages, for context

| language | files | full | partial | refused | no text | median emitted |
|---|---|---|---|---|---|---|
| persian | 25 | 13 | 1 | 8 | 3 | 100% |
| english | 10 | 9 | 1 | 0 | 0 | 100% |
| unknown | 7 | 2 | 0 | 2 | 3 | 50% |
| arabic | 4 | 3 | 1 | 0 | 0 | 100% |
| hebrew | 4 | 4 | 0 | 0 | 0 | 100% |
| he-eng | 1 | 0 | 1 | 0 | 0 | 5% |
| **total** | **51** | **31** | **4** | **10** | **6** | |

Hebrew is 4/4 and needs nothing. English is 9/10 and is not the focus. The `unknown` bucket is
files whose language was never established from positive evidence — see
`references/real-world-pdf-interrogation.md`; a filename is not evidence.

## 4. Corrections to earlier numbers

Earlier documents in this repository said "12 refused", then "14", then quoted character totals as
a headline. Both were wrong and the reasons differ:

* **12 vs 14** — two different definitions of the same set. 14 was "files that do not emit
  everything"; the measured split is 8 refused entirely + 4 partial = 12 files with a
  non-empty `unproven` field, and 10 files that emit nothing. The table above is disjoint and sums
  to 51.
* **Character totals** — `1,271,919` withheld against `1,038,880` emitted reads as "the tool
  refuses more than it answers". It is an artefact of one 321-page scanned volume contributing
  93.3% of all withheld characters. **Per-language and per-file counts are the honest unit**, and
  that is what this document uses. A single file is not a reference for general work.

## 5. Known gaps in this measurement

* **Not reproducible outside the owner's machine.** The corpus is private customer documents.
  `scripts/validate-archive.sh` should emit an aggregate-only report that a second holder of the
  same corpus can diff (the critique's P-9). Until then, treat these numbers as owner-local.
* **No human has read the output.** Every count here says characters were ordered, not that a
  person read them and found them correct. `AGENTS.md` rule 8 requires that read and it has not
  happened (Q-006).
* **No public fixture exercises the refusal path.** All 14 tracked fixtures emit fully
  (`docs/problems/0017`), so rung 3 has zero coverage outside the private archive.