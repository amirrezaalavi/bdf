# Commercial licence — terms TBD

`pdfrtl` is dual-licensed:

* **AGPL-3.0-or-later** (see `LICENSE`) for the free edition — including the CLI and any
  GUI built on the core.
* **A commercial licence** for organisations that want to embed the core in a closed
  product, or operate the paid tier, without the obligations of the AGPL.

**Status: TBD.** No commercial licence is granted by this file. A lawyer's text with
actual terms (scope, fees, term, support, liability, trademark) is required before the
first paid sale — tracked as an open item in `docs/OPEN-QUESTIONS.md` and pending on
ADR 0001.

## What the commercial licence will need to cover

| Surface | Why it needs the commercial licence |
|---|---|
| Embedding `pdfrtl-core` in a closed-source application | AGPL §5/§6 would otherwise require releasing the whole combined work |
| Running the MCP/agent server as a network service | AGPL §13 obliges you to offer the source to network users |
| Enterprise features (PAdES/LTV signing, PDF/A + PDF/UA validation, redaction, multi-tenant serving) | Closed-source additions to the core |
| Support, SLAs, indemnification, private builds | Not things a free licence can provide |

## Why we can sell it

The AGPL obligations apply to *our* code; the commercial licence is a grant from us. That
only works if we own every line we ship — which is why:

* shipped dependencies are permissive-only (`docs/decisions/0003-…`), and
* authorship is recorded per commit and per fixture (`AUTHORSHIP.md`).

## Inbound contributions

Not accepted yet: a Contributor Licence Agreement (CLA) is required first, otherwise a
single outside patch can block commercial licensing forever. External PRs should be
closed with a link to this section until the CLA lands.
