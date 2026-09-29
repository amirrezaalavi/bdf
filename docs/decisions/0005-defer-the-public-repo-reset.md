# 0005 — defer the public-repo reset until the first stable release

**Status:** accepted (2026-09-29)
**Deciders:** owner (Yolka); recorded by the orchestrator.

## Context

`github.com/amirrezaalavi/bdf` is a **scratch collaboration remote**: it exists so that coding
lanes have somewhere to push branches and so CI judges a published tree. It is not a product
channel — no reviewers, no users, no releases.

Four commits in its history (`8ab88aa`, `0cce48d`, `a6a36ac`, `05d45a0`) carry a manifest that
was published unstripped: 51 rows describing real customer documents — our slug filenames,
sha256 hashes, page/font counts, producer strings and each document's embedded title. No
document content and no credentials. Root cause and fix: `docs/problems/0006`. The current
`main` is clean and the control that leaked now refuses to pass when it cannot see its input.

Removing those commits means rewriting published history or recreating the repository.

## Decision

**Do not rewrite history now.** The owner's call: with no public reviewers and the repository
due to be recreated anyway, the cost of a history rewrite is not worth paying twice. The reset
happens once, deliberately, when the software reaches a stable version — together with the
other things that were always going to change at that point: licensing, git history, naming.

## Consequences

- Those four commits stay fetchable by SHA in the meantime. Acceptable **only** because the
  repository has no external audience, no forks, and will be recreated. This trade-off expires
  the moment that stops being true.
- **Hard trigger:** if an external reviewer, a shared URL, a release, or any third party is
  about to touch this repo, the reset happens **first**. The deferral is about not paying for a
  rewrite on a scratch remote, not about tolerating the exposure.
- The local repository keeps the real history; only the mirror's history is disposable.

### Reset checklist (do all of it in one pass)

1. Delete and recreate the repository, then push a **squashed clean tree** — not the old
   history. Re-verify the published `corpus/manifest.json` contains only redistributable rows.
2. Re-decide the licence posture (AGPL-3.0-or-later + commercial is the current intent) and
   the project name; the repo is currently called `bdf`, the product is `pdfrtl`.
3. Re-run the publisher's strip and deny-list against the fresh mirror and confirm the log
   reports a plausible file count, per `docs/problems/0006`.
4. Re-check every tracked file for customer-identifying strings before the first push.
5. Rewrite `AGENTS.md`/`README.md` refs to the old URL if the remote changes.
