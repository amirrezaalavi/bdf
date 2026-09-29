# 0006 — a control that cannot see its input passes vacuously (private manifest leaked)

**Found:** 2026-09-29, immediately after a successful publish, by reading the published file
back from the remote instead of trusting the publish log.
**Severity:** high — a confidentiality incident, not a code bug.
**Status:** closed forward; history still carries it (decision pending, see below).

## What happened

`scripts/publish-public.sh` is the control that keeps the local tree's real customer documents
(`corpus/raw/private/`, gitignored) out of the public mirror. It stages a copy of the working
tree, strips the private rows out of `corpus/manifest.json`, then runs a deny-list scan over
every staged file, and only then pushes.

The published `corpus/manifest.json` contained **61 rows, 51 of them private**: our slugs,
customer document **sha256 hashes**, page and font counts, producer strings, and each
document's **embedded title** — while `corpus/raw/private/` itself was correctly absent.

The publish log for that same run read:

```
== staging working tree (private corpus and reports excluded) ==
deny-list clean (70 private identities checked)          <- 0 files were actually read
```

## Root cause

One line, one missing path conversion:

```bash
STAGE="$MIRROR/.stage"            # MSYS path:  /c/Users/.../bdf-snapshot/.stage
STAGE_NATIVE="$STAGE"             # <- handed to NATIVE Windows Python unconverted
"$PY" - "$REPO_NATIVE" "$STAGE_NATIVE" <<'PY'
```

Windows Python resolves `/c/Users/...` as `C:\c\Users\...`, which does not exist. So:

```python
mp = stage / "corpus/manifest.json"
if mp.exists():        # False -> the strip SILENTLY DOES NOTHING
    ...
for path in stage.rglob("*"):   # 0 files -> hits == [] -> "clean"
```

Both controls degraded to no-ops **without failing**, because each was written to tolerate a
missing input: the strip skipped itself when the manifest "wasn't there", and the scan
reported clean when it had read nothing. `pathlib` is well-behaved here — `Path("/c/...")` is
simply a relative-looking path on Windows, so there is no exception to notice.

The stripping logic, the deny-list and the CI-gated publish were all correct. The control
never ran.

## Exposure

Metadata only: hashes, page/font counts, producer strings, our own slug filenames, and the
titles embedded in the PDFs (some documents identify their subject and language in the title).
No document content, no original filenames, no credentials. The leak is in the public
repository's git history at the affected commits; the current `main` is clean.

## Fix (in this repo, committed with 0006)

1. `STAGE_NATIVE` is converted with `pwd -W`, like `MIRROR_NATIVE`/`REPO_NATIVE` already were.
2. **A control that cannot see its input must refuse, not pass.** The block now asserts the
   staging directory exists, that `corpus/manifest.json` is present, that the strip left zero
   private rows and a non-empty manifest, and that the deny-list scan actually read a
   plausible number of files (`>= 20`). Any of those failing exits non-zero before a push.
3. The success line reports the work done, not a verdict alone:
   `deny-list clean (70 private identities checked across 104 files)` — the file count is the
   evidence that it looked.

## The generalisable rule

**A guard whose "nothing to check" path returns success is not a guard.** Whenever a check is
allowed to skip itself because its input is absent, something must separately prove the input
was *supposed* to be there — and a scan must report how much it read, so a zero-work pass is
visible in the log rather than indistinguishable from a real pass. The same disease as a CI
step that swallows a tool's exit code, and it is invisible in exactly the same way: every
signal says green.

Two habits caught this and are worth keeping: read back the **artifact** from the remote after
publishing (not the log), and make the local gate assert on the artifact's contents rather
than on the exit code of the tool that produced it.
