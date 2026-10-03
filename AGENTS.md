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

0. **Never write a private filename into a tracked document.** The corpus files are customer
   documents; their names, hashes, embedded titles and derived counts are private too (see
   `docs/decisions/0004` and the deny-list in `scripts/publish-public.sh`). Refer to them by
   their manifest language label or as "one unlabelled file". Writing a real filename into a
   plan or a README looks harmless and is caught only by the publisher's deny-list at the very
   last step — measured twice on 2026-10-03: a plan table named a customer file, and the rule
   written to prevent it named the same file in its own text. Both were refused before
   publication. The guard worked every time; the lesson is that it was the only thing standing
   between a customer name and the public internet, and both mistakes were mine.
1. **TDD.** Failing test first, observed failing, then the minimal implementation.
   No test, no merge. Never weaken, delete or skip a test to get a green gate — if an
   expectation is wrong, fix the expectation to match verified reality and say why.
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
7. **No guessing — ask instead.** If an API signature is uncertain, pull the docs through
   context7 (`mcp__context7__query_docs`) or read the vendored source. If the *knowledge*
   is not obtainable locally (a producer's behaviour, a spec clause, how another engine
   decides something), write the question into **`docs/RESEARCH-QUESTIONS.md`** in the
   format that file prescribes. Never invent a name, and never fill a knowledge gap with a
   plausible-sounding answer: a fabricated fact here produces text nobody can read but
   everybody can ship.
8. **Human review for text output.** Any change that alters RTL text output needs a
   human-reviewed fixture diff in `docs/reviews/`. Agent self-assessment does not count.
   `scripts/render_pages.py` + vision review makes that cheap; the reviewer's eye is the
   only oracle for "does this read correctly".
9. **Commit trailers.** Every commit carries:
   ```
   Designed-by: Yolka <alavi2004@outlook.com>
   Implemented-by: <agent name/model>
   Reviewed-by: <human or agent + date>
   ```
   Rationale: the project is dual-licensed; ownership/authorship must be traceable.
   See `AUTHORSHIP.md`.

## Delegated lanes (subagents)

A lane is one agent given one bounded deliverable, working on its own branch while the
orchestrator integrates. Rules that each cost real time to learn:

* **Never work in the main checkout.** `git worktree add -b lane/<name> ../bdf-<name> master`,
  and set a **separate** `CARGO_TARGET_DIR="$HOME/target-bdf-<name>"` — a target directory
  belongs to exactly one tree and the gate refuses artifacts built elsewhere. Push your own
  branch; `main` belongs to the orchestrator.
* **Run the whole suite, never one target.** After any change to extraction or the order
  ladder, `bash scripts/wsl-build.sh` — the invariant control (`no_silent_reversal`) lives in
  its own target, so a single-target run reports green while the control is failing.
* **Verify the binary you are measuring.** An ad-hoc `cargo build` without `CARGO_TARGET_DIR`
  writes to the repo-local `target/` while the run uses the exported one; stamp a marker and
  check `strings "$BIN" | grep -c '<marker>'` before trusting any behavioural claim.
* **The core must not print.** Diagnostics are *returned*; the CLI owns stdout. The slop gate
  greps for prints, and a feature flag does not get around a grep.
* **No lane touches `corpus/raw/private/` or runs `scripts/publish-public.sh`.** Real customer
  documents stay local; publishing is the orchestrator's job.
* **Never pipe a build to `/dev/null` when its result is the thing you are about to measure.** A
  missing binary compares as "everything differs", not as "the build failed". Assert the artifact
  exists (`[ -x "$BIN" ] || exit 2`) before comparing two builds.
* **Add a target to the PINNED toolchain, not to `stable`.** `rustup target add x86_64-unknown-linux-musl`
  run outside the repo installs for the default toolchain, and the pinned build then fails with
  `can't find crate for 'core'` — the same drift `docs/problems/0003` records, in a new costume.
  Use `rustup target add --toolchain "$(sed -n 's/^channel *= *"\(.*\)"/\1/p' rust-toolchain.toml)-x86_64-unknown-linux-gnu"`.
* **A lane that cannot finish says so**, with the exact failing command and its output. An
  honest partial result is worth more than a green summary nobody can reproduce.
* **A lane's own summary is a claim, not evidence.** Raw output goes to `reports/` or
  `docs/reviews/`, and the orchestrator re-runs the check before integrating.

The 51 real-world PDFs are customer documents and are **not part of the public clone**. Never
assume `corpus/raw/private/desktop-pdfs/` exists or contains data on another machine. Do not add
those PDFs, their names, hashes, embedded titles, or derived `reports/` to a public commit. A new
contributor can build and run the committed synthetic fixtures without them; archive validation
numbers require the owner's private corpus. See `corpus/README.md` and the root README's clone
instructions. Do not invent a fetch path: the owner will arrange access to the private files when
needed.

## Environment: how to build and test

Build and test run in **WSL Ubuntu-26.04** (the Windows host has no Rust toolchain).

```bash
# from WSL
bash ~/playground/ai/bdf/scripts/wsl-build.sh
# = fmt --check, clippy -D warnings, cargo test --workspace --locked,
#   the CI slop greps (unwrap/expect in core, todo!/dbg!, prints in core),
#   gen-deps.py --check, cargo deny check
bash ~/playground/ai/bdf/scripts/wsl-build.sh --quick   # skip cargo-deny
```

The script derives the repo root from its own location, so the path above is a convenience, not a
requirement (`PDFRTL_ROOT` still overrides it). The Windows checkout
`C:/Users/netcon/playground/ai/pdfrtl` is a read-only mirror — never build in it.

Exact toolchain: **rustc/cargo 1.98.1**, pinned identically in `rust-toolchain.toml` and
`.github/workflows/ci.yml`. Never `channel = "stable"` — a floating channel on either side is
two toolchains waiting to disagree, and it already cost three red CI runs
(`docs/problems/0003-toolchain-drift-made-the-gate-lie.md`). Artifacts go to
`$CARGO_TARGET_DIR=$HOME/target-bdf` (ext4). **Sources live in WSL too**: the dev root is
`~/playground/ai/bdf`, and the Windows checkout `C:/Users/netcon/playground/ai/pdfrtl` is a
read-only mirror. Cargo over `/mnt/c` was the bottleneck (the same workspace builds in 14 s on
ext4), and a target directory belongs to exactly one tree — the gate refuses artifacts built
elsewhere rather than reusing them (exit 5).

**A gate may not pass vacuously.** If a check can skip itself because its input is missing, it
must first assert that the input exists, and it must report how much it examined
(`docs/problems/0006`). A control that cannot see what it is checking must refuse.

Single test: `cargo test -p pdfrtl-core --test docinfo`
CLI smoke: `cargo run -q -p pdfrtl-cli -- --json inspect corpus/raw/synthetic/minimal-ltr.pdf`

### Windows/MSYS traps that cost real time

* The shell is git-bash; `terminal` runs POSIX, not PowerShell.
* Native programs (`git`, `python`, `node`, `cargo.exe`) get **no MSYS path translation**:
  pass `C:/Users/...`, never `/c/Users/...`. The wrong form does not always error — it
  silently reads or creates `C:\c\Users\...` instead.
* `python3` on the Windows side is a stub that produces nothing; use `python` or the repo
  venv `./.venv/Scripts/python.exe` (pypdf, pypdfium2, Pillow).
* Feed multi-line scripts to WSL on **stdin**: `wsl.exe -d Ubuntu-26.04 -- bash -s <<'EOF'`.
* Never truncate a long-running publisher/CI pipeline with `head` — SIGPIPE kills it mid-run.
  Redirect to a file and `tail` the file.

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

`pdftotext` (poppler **or** Xpdf — always record which, with `pdftotext -v`), `qpdf`
(Apache-2.0), `mutool` (AGPL-3.0), `gs`, `tesseract` are **verification tools only**. They
must never be linked, vendored or bundled; `docs/DEPS.md` records each as `oracle`. PDFium
(via `pypdfium2`) is the pixel oracle — its text output drops ZWNJ and returns per-cluster
visual order, so it may never judge text identity. Measured matrix:
`docs/problems/0004-no-oracle-is-byte-faithful-for-rtl.md`.

## Publishing (the only path to the public mirror)

`scripts/publish-public.sh` is the control. It stages the tree without `corpus/raw/private`
and `reports/`, **strips the private manifest rows**, runs a deny-list built from the original
filenames and embedded document titles, runs the CI gate on the staged tree under public
conditions, then pushes a `snapshot/<ts>` branch and fast-forwards `main` only after CI passes
on that exact SHA. A bare `git add -A` push by an agent is forbidden; code-writing lanes push
their own branch instead.

The mirror is a **scratch collaboration remote**, not a product channel; its history is
disposable and the reset is deferred by decision —
`docs/decisions/0005-defer-the-public-repo-reset.md` (which also lists the hard trigger that
ends the deferral).

## Knowledge and handoff

* `docs/RESEARCH-QUESTIONS.md` — knowledge gaps that need an outside researcher. Follow the
  format; consume the answer and delete the question once it is answered.
* `docs/OPEN-QUESTIONS.md` — decisions only the owner can make (product, scope, licence).
* `scripts/db.py` (SQLite + FTS5) — searchable task/knowledge base:
  `python3 scripts/db.py search "lam-alef"`, `python3 scripts/db.py task-add …`,
  `python3 scripts/db.py task-done <id> --ref <sha>` (refuses without evidence).
* `docs/problems/` — our own failure log, one file per problem, root cause + fix. Read it
  before re-deriving anything; it is the cheapest documentation we own.
* `HANDOFF.md` — regenerated every milestone; must let another machine continue with
  zero chat history.
* Area guides: `crates/*/AGENT.md`, `corpus/AGENT.md`, `scripts/AGENT.md`.
