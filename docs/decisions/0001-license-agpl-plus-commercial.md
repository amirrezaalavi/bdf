# ADR 0001 — Licence: AGPL-3.0-or-later core, dual-licensed commercially

* Status: accepted
* Date: 2026-09-27
* Deciders: Yolka

## Context

The product plan is: a free, feature-complete GUI/CLI edition, and a paid tier built on
the same core (commercial licence + MCP/agent surface + enterprise features). The
project must remain "open source" in the free tier while being sellable.

## Decision

The core is licensed **AGPL-3.0-or-later**. Commercial licences are sold separately
(dual licensing, the iText/Qt model). The GUI and CLI are free under the same terms.
Paid tier = commercial core licence + MCP server + enterprise features (signing/LTV,
PDF/A, redaction, multi-tenant serving).

## Why not the alternatives

* **Permissive core (MIT/Apache-2.0) + paid MCP** — the MCP server is a thin adapter
  (~1k LOC over the CLI). Anyone could rebuild the paid tier from a permissive core in a
  week; the moat would be support only.
* **Source-available (BUSL/FSL)** — not OSI open source, so the free tier's "open
  source" claim becomes misleading, and enterprise legal review is harder.
* **AGPL only (no commercial option)** — blocks the paid tier entirely, since we would be
  bound by our own licence when shipping closed additions.

AGPL specifically (rather than GPL) because it closes the "host it and never publish"
loophole: §13 requires network users be offered the source, which is exactly the
protection a single-product company needs against a hosted clone.

## Consequences

* **We must own 100% of the copyright** to grant commercial licences. Hence: a CLA
  before any outside contribution, human-authored design artifacts, and commit trailers
  recording authorship (`AUTHORSHIP.md`).
* **Copyleft third-party code is still banned from shipped default features.** Our own
  AGPL does not let us sublicense someone else's GPL/AGPL/LGPL code. See ADR 0003.
* Anyone redistributing a modified free edition must publish their changes — acceptable,
  and arguably desirable for the ecosystem.

## Follow-ups

* `LICENSE-COMMERCIAL.md` states terms are TBD; a lawyer's text is required before the
  first paid sale.
* Revisit if Q-007 (ownership of the al-bdf RTL work) comes back unclear.
