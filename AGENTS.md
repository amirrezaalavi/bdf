# AGENTS.md — binding contract for any agent working in this repo

Read this before touching anything. It is short on purpose; every rule here exists
because breaking it cost us time or correctness somewhere else.

## What this project is

`pdfrtl` is a headless, RTL-first PDF core (library + CLI + MCP) for Arabic, Persian
and Hebrew alongside English. Rust. No GUI, no Qt in the core. Engines are **bundled,
not written**: `lopdf` (parse/edit), later `krilla` + `pdf-writer` (write),
`harfrust` (shaping), `unicode-bidi` (UAX #9), PDFium (render oracle, optional).

## The one invariant (never weaken it)

> Extraction returns **logical order**, or it fails with an explicit `Reason`.
> Silent reversal is a bug. There is no third outcome.

Storage order in the wild is producer-dependent: LibreOffice/mPDF/iTextSharp/Chrome
store visual order, ReportLab/wkhtmltopdf/PDFreactor store logical order. A blanket
"reverse RTL runs" fixes one family and corrupts the other. Recover order from
`/ActualText` first, then `ToUnicode` + UAX #9 levels, then an allow-listed producer
fingerprint — otherwise return `unsupported_no_evidence` and exit 3.

## Rules

1. **TDD.** Failing test first, observed failing, then the minimal implementation.
   No test, no merge.
2. **One logical change per commit.** Conventional Commits. Never commit red.
3. **File ownership during concurrent work.** One agent per file; lanes are listed in
   `docs/plans/`. The integrator merges; agents do not merge each other.
4. **Determinism.** No timestamps, no random ids, no locale-dependent ordering in
   output. Fixtures must be byte-identical between runs (`sha256sum` twice).
5. **Evidence, not summaries.** A task is done when the raw command output exists in
   `docs/reviews/` or `reports/`. "Tests pass" without output is not evidence.
6. **Licence red line.** Every new dependency needs a `docs/DEPS.md` row (name,
   version, licence, evidence URL, why) **in the same commit**, and `cargo deny check`
   must stay green. Shipped default features are **permissive-only** — see
   `docs/decisions/0003-permissive-only-default-features.md` for why AGPL-for-us does
   not mean AGPL-dependencies-for-us.
7. **No API guessing.** If a signature is uncertain, pull the docs through context7
   (`mcp__context7__query_docs`) or read the vendored source. Never invent a name.
8. **Human review for text output.** Any change that alters RTL text output needs a
   human-reviewed fixture diff in `docs/reviews/`. Agent self-assessment does not count.
9. **Commit trailers.** Every commit carries:
   ```
   Designed-by: Yolka <alavi2004@outlook.com>
   Implemented-by: <agent name/model>
   Reviewed-by: <human or agent + date>
   ```
   Rationale: the project is dual-licensed; ownership/authorship must be traceable.
   See `AUTHORSHIP.md`.

## Environment: how to build and test

Build and test run in **WSL Ubuntu-26.04** (the Windows host has no Rust toolchain).

```bash
# from WSL
bash /mnt/c/Users/netcon/playground/ai/pdfrtl/scripts/wsl-build.sh
# = fmt --check, clippy -D warnings, cargo test --workspace, cargo deny check
bash /mnt/c/Users/netcon/playground/ai/pdfrtl/scripts/wsl-build.sh --quick   # skip cargo-deny
```

Exact toolchain: **rustc/cargo 1.97.1** (`stable`, pinned in `rust-toolchain.toml`;
CI pins 1.97.1 exactly). Artifacts go to `$CARGO_TARGET_DIR=$HOME/target-pdfrtl`
(ext4) because cargo over `/mnt/c` is slow; sources stay on the Windows checkout so
Windows-side tooling can see them.

Single test: `cargo test -p pdfrtl-core --test docinfo`
CLI smoke: `cargo run -q -p pdfrtl-cli -- --json inspect corpus/raw/synthetic/minimal-ltr.pdf`

## Network on this host (read this before debugging a fetch failure)

* Windows side has general egress. WSL resolves IPv4 only for some hosts, and WSL has
  **no IPv6 route** — that is why `/etc/gai.conf` carries a `precedence ::ffff:0:0/96 100`
  line. Do not "fix" it by removing that line.
* Cargo uses the **Aliyun sparse mirror** (`~/.cargo/config.toml`,
  `sparse+https://mirrors.aliyun.com/crates.io-index/`); direct crates.io CDN is
  unreliable from this network. Verified working.
* Blocked-by-DNS or blocked-by-route hosts (e.g. some CDNs, `api.firecrawl.dev`):
  the working pattern on the Windows side is a SOCKS5 proxy on `127.0.0.1:10808`:
  `curl --socks5-hostname 127.0.0.1:10808 <url>`.
* Subagents that need the web must receive these instructions in their context —
  otherwise they silently fail and report "no data found".

## Oracles (never shipped)

`pdftotext` (poppler, GPL-2.0+), `qpdf` (Apache-2.0), `mutool` (AGPL-3.0), `gs`,
`tesseract` are **verification tools only**. They must never be linked, vendored or
bundled. `docs/DEPS.md` records each one as `oracle`.

## Knowledge and handoff

* `scripts/db.py` (SQLite + FTS5) — searchable task/knowledge base:
  `python3 scripts/db.py search "lam-alef"`, `python3 scripts/db.py task-add …`,
  `python3 scripts/db.py task-done <id> --ref <sha>` (refuses without evidence).
* `docs/problems/` — our own failure log, one file per problem, root cause + fix.
* `HANDOFF.md` — regenerated every milestone; must let another machine continue with
  zero chat history.
* Area guides: `crates/*/AGENT.md`, `corpus/AGENT.md`, `scripts/AGENT.md`.
