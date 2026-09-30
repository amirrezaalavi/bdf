# 0012 — A publish that reports "up to date" while the remote lags

**Status:** fixed
**Date:** 2026-09-29
**Touches:** `scripts/publish-public.sh` (the only path to the public mirror)

## What happened

The publisher stages the public subset of the tree into a scratch clone
(`~/playground/ai/bdf-snapshot`), commits it, pushes a `snapshot/<ts>` branch, waits for CI on
that exact SHA and only then fast-forwards `main`.

A push failed — `! [remote rejected] HEAD -> snapshot/… (Internal Server Error)`, a transient
GitHub 5xx, not an auth problem and not a rejected update. The script exited non-zero, which is
correct, and `main` was untouched, which is also correct.

Then every later run printed:

```
== public mirror already up to date, nothing to publish ==
```

and exited **0** — while `origin/main` was still `7520335b` and the local clone held an
unpushed commit `3d1f721` with the content that was supposed to be published. Measured state:

```
## main...origin/main [ahead 1]
```

So the control reported success precisely when it had done nothing, and would have kept doing
so indefinitely: no later run could ever notice, because the check that decides "nothing to
publish" never looked at the remote.

## Why

```bash
cd "$MIRROR"
git add -A
if git diff --cached --quiet; then
  echo "== public mirror already up to date, nothing to publish =="
  exit 0
fi
```

The verdict compares the freshly staged tree against the **local clone's own HEAD**. After a
failed push, HEAD *is* the content that failed to publish, so the comparison is between the
new content and itself. The question actually being asked — "does the remote already have
this?" — is answered by a local object that may itself be the thing that broke.

The general shape: a step that can fail *between* "create the commit" and "push it" leaves the
local state looking exactly like success. Any later run that decides what to do by inspecting
that local state inherits the false conclusion.

## The rule

**A control that reports "nothing to do" must compare against the state it claims to have
synchronised — the remote — never against a local copy that may be the thing that failed.**

Corollary: **assume the previous run may have died anywhere.** Make the first action of a
publishing step "make the local state agree with the remote", so the rest of the logic starts
from a state that is true by construction rather than by hope.

## The fix

Before staging, sync the clone to the remote and reset it if it diverged. The placement matters:
the reset must happen **before** the working tree is replaced, or it discards the very content
being staged — resetting afterwards would have recreated the same silent no-op with extra
steps. Resetting loses nothing: the mirror's history is disposable
(`docs/decisions/0005`) and the tree is regenerated from `HEAD` on every run.

```bash
cd "$MIRROR"
git fetch -q origin main 2>/dev/null || true
if [ "$(git rev-parse HEAD)" != "$(git rev-parse origin/main)" ]; then
  echo "== mirror clone diverged from origin/main (… vs …): resetting to the published state"
  git reset --hard -q origin/main
fi
```

## Evidence

* RED (real, not constructed): the log line `== public mirror already up to date, nothing to
  publish ==`, exit 0, `origin/main` = `7520335b`, clone `[ahead 1]` at `3d1f721`.
* Repair: `git fetch` + `git reset --hard origin/main` in the clone, then the publisher ran
  normally. Published `main` = `80bcae98`, CI 5/5 green, and the change was read back **out of
  the remote** (`git show origin/main:tools/triage_pdfs.py` contains the new argument) rather
  than inferred from the log.
* GREEN: with the clone artificially pushed one commit ahead of `origin/main`, the fixed
  publisher detects the divergence, resets, publishes, and leaves the clone in sync.
