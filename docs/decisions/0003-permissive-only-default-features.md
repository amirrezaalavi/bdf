# ADR 0003 — Shipped default features are permissive-licensed only

* Status: accepted
* Date: 2026-09-27
* Deciders: Yolka

## Context

We chose AGPL-3.0-or-later for our own code **plus** a commercial licence (ADR 0001).
The intuitive conclusion is "since we are AGPL, we may use GPL/AGPL/LGPL dependencies".
That conclusion is wrong, and the mistake is expensive to discover late.

## Decision

Every dependency in a **shipped default feature** must be permissively licensed:
MIT, MIT-0, Apache-2.0 (incl. LLVM-exception), BSD-2/3-Clause, ISC, Unicode-3.0,
Unicode-DFS-2016, Zlib, MPL-2.0 (file-level), CC0-1.0, Unlicense, BSL-1.0, OpenSSL, FTL.
GPL-2.0/3.0, AGPL-3.0, LGPL-2.1/3.0, SSPL and BUSL are denied in shipped code.

Enforced by `deny.toml` + `cargo deny check` in CI and in `scripts/wsl-build.sh`.

## Rationale

Selling a commercial licence means granting customers the right to use **the whole work**
under terms of our choosing. We cannot grant that right over code we do not own. Linking
GPL/AGPL/LGPL code into the core makes the core legally un-sublicensable — the commercial
tier collapses, no matter what our own licence says. Ownership, not our own licence, is
the binding constraint.

## Consequences

* AGPL/GPL tools (`pdftotext`, `mutool`, `gs`, Poppler) are **verification oracles** only:
  used in CI, never linked, never bundled, never in default features. `docs/DEPS.md`
  records each as `oracle`.
* MPL-2.0 (e.g. a future veraPDF integration) is acceptable only as file-level: we may not
  modify and relicense those files, and we must keep their notices.
* Adding a dependency is a two-line ritual: a `docs/DEPS.md` row plus a green
  `cargo deny check`, in the same commit.
* If Q-001 lands as "hosted SaaS only", the AGPL-or-permissive calculus for *our* code may
  be revisited — but this ADR stays: ownership is still required for any commercial grant.

## Follow-ups

* P4: PAdES/PDF-A validation path must be checked against this rule (MPL branch of
  veraPDF only).
