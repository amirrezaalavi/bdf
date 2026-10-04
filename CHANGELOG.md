# Changelog

All notable changes to `pdfrtl`. This project is pre-1.0: the extraction contract can still change,
and the numbers below are the measured state at each release.

## [Unreleased]

### Changed

- Corrected owner-adjudicated language labels in the local private manifest and updated public acceptance documentation to keep private measurements and identities out of the mirror.
- The owner reviewed the displayed Persian RFP extraction text and said it seemed right. This is not a rendered-page comparison fixture; stronger reading-order claims remain open.
- `docs/ACCEPTANCE.md`, `CANARY.md` and `README.md` now state that private archive results are owner-local and excluded from public claims. The v0.1.0 measurements below remain historical as-of-release figures.

## [0.1.0] — 2026-10-03

First release: a canary. The contract is stable; the coverage is not complete.

### Added

- **Static Linux binaries** for `x86_64` and `aarch64` (musl, no runtime required), built by
  `scripts/build-release.sh` and verified before publication — static-ness, architecture and a
  smoke test are all checked, so a broken artifact fails the build rather than shipping.
- **`--include-unproven`** — the withheld text of lines whose order could not be established,
  behind an opt-in flag. The default envelope reports the **counts** only, because that text is
  frequently reversed and an LLM or MCP caller reads the whole JSON object without asking for a
  field by name.
- **Public fixtures for the refusal path** (`tools/gen_refusal_fixtures.py`). Measured before this
  release: `unproven_lines` was 0 on all 14 tracked fixtures — every one carried `/ActualText` or
  `/ReversedChars` and was decided before reaching the rung that refuses. Three new fixtures give
  that branch public coverage; their expected value is a *refusal*, which is why they can be
  published while customer documents cannot.
- **`scripts/by_language.py`** — acceptance measured per language, with Persian first. Every
  headline number in the documentation is produced by it rather than typed, and it exits non-zero
  rather than printing zeros when the private corpus is absent.
- `docs/ACCEPTANCE.md` — the measured acceptance table and what each number means.

### Changed

- **`bidi_verified` is now `bidi_consistent`.** A breaking change to the serialized reason code.
  Two different logical strings can paint identically (measured, `docs/problems/0015`), so a
  forward match is consistency, not proof. The old name claimed more than the code establishes.
- Reason codes and the `unproven` field documented in `docs/CLI.md` and `CANARY.md`.
- `CANARY.md` and `README.md` now report acceptance **per language and per file**. The previous
  headline ("withheld characters exceed emitted characters") was an artefact of one 321-page file
  contributing 93.3% of the total.

### Measured state at this release

Private-corpus language and extraction metrics have been removed from public documentation. Their historical snapshot remains owner-local; the public release is supported by the committed synthetic fixtures and CI checks.

### Known limitations

- **Reading order is not recovered on every input.** The blocker is lines whose painted form neither existing hypothesis reproduces (`docs/problems/0018`).
- **No human has verified the output.** Every number here proves characters were *ordered*, not
  that a Persian speaker read them and found them correct (`AGENTS.md` rule 8; Q-013).
- **No reproducible build is claimed.** Output identity between the musl and glibc builds was
  verified across the public fixtures; rebuilding the committed binary byte-for-byte is not
  verified and no artefact for it exists.

### Licence

All rights reserved. Not AGPL, not MIT — the licence is still to be decided, and this release is
an invitation to test rather than a grant. See the README.

[0.1.0]: https://github.com/amirrezaalavi/bdf/releases/tag/v0.1.0