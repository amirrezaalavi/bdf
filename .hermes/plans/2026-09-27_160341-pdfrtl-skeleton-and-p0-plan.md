# pdfrtl — skeleton + P0 build plan (Option A: AGPL core + commercial licence)

Plan file: `.hermes/plans/2026-09-27_160341-pdfrtl-skeleton-and-p0-plan.md`
Written for: an implementer (human or agent) with **zero context** on this codebase.
Repo root assumed by every path below: `C:/Users/netcon/playground/ai/pdfrtl` (Windows) = `/mnt/c/Users/netcon/playground/ai/pdfrtl` (WSL).

---

## 1. Goal (one sentence)

Stand up the `pdfrtl` Rust workspace and the machinery around it (corpus, oracles, licence gate, knowledge DB, handoff docs) so that P0 — `pdfrtl inspect|extract --json` plus a producer×script fixture corpus — is implemented, tested and reproducible by agents on two machines by the end of this plan.

## 2. Current context / assumptions

**Verified environment (probed 2026-09-27, read-only):**

| Tool | Windows host | Note |
|---|---|---|
| rustc / cargo / rustup | **MISSING** | must be installed; WSL is the recommended target |
| WSL | **Ubuntu-26.04, STATE=Stopped, VERSION=2** | primary build/test environment |
| docker | **MISSING** | Dockerfile is still written (for CI + other machines), not run here |
| cmake / ninja / clang | MISSING | not needed: no C++ in this project |
| qpdf / mutool / gs | MISSING | needed as CI oracles → install in WSL via apt |
| `pdftotext` | **present, poppler 4.00** | usable local oracle (GPL → **never shipped**) |
| node / npm | v26.7.0 / 11.19.0 | available if the MCP server is ever done in TS |
| python | 3.14.7 as `python` | `python3` does **not** exist on Windows; scripts must handle both |
| git | 2.53.0.windows.3 | fine |
| disk | 118 GB free on C: | fine |

**Decisions already taken (do not re-litigate):**
- Core language: **Rust**. Headless. No Qt, no GUI in the core.
- Licence: **Option A — core under AGPL-3.0-or-later, dual-licensed commercially**; GUI + CLI free, paid tier = commercial core licence + MCP + enterprise features.
- Engines are **bundled, not written**: lopdf (parse/edit), krilla + pdf-writer (write + marked content), harfrust (shaping), unicode-bidi (UAX #9), PDFium (render oracle, optional feature), RustCrypto (PAdES later).
- We must own 100% of the core's copyright for dual licensing to work.

**Consequence of Option A that governs every dependency choice (non-obvious, read twice):**
AGPL for *our* code does **not** mean we may take on AGPL/GPL dependencies. A dual licence requires the owner to grant customers a commercial licence to **the whole work** — and we cannot relicense third-party copyleft code we do not own. Therefore:
> **Every dependency in a shipped default feature must be permissive (MIT / Apache-2.0 / BSD / ISC / Unicode-3.0 / Zlib / FTL / MPL-2.0-file-level).**
> AGPL/GPL tools (poppler `pdftotext`, mutool, gs, verapdf-GPL-branch) are **CI oracles only** — never linked, never bundled, never in the default build.

**Human-authored artifacts are a legal asset, not bureaucracy** (US Copyright Office: purely AI-generated material is not protected; prompts alone are insufficient control). So: design docs, ADRs, the corpus, and review sign-offs are authored/reviewed by the human, recorded per commit (§5 task 0.23, §12 risk 1).

## 3. Architecture / proposed approach (2-3 sentences)

A cargo workspace with three crates — `pdfrtl-core` (document model, RTL text pipeline, extract, generate, edit), `pdfrtl-cli` (`pdfrtl <verb> --json`, the real product surface), `pdfrtl-mcp` (thin adapter over the CLI contract) — where every extraction either returns **logical order** or fails with an explicit **`Reason`** code, and correctness is proven by a committed producer×script fixture corpus checked against independent oracles (PDFium pixels, `pdftotext`/`mutool` extraction, `qpdf --check`). Build/test runs in **WSL Ubuntu-26.04** with `CARGO_TARGET_DIR` on ext4 while sources stay on the Windows filesystem, so the repo remains visible to the Windows-side agent tools.

## 4. Rules of engagement (binding for every task below)

1. **TDD.** Every task: write the failing test → run it → see it fail → implement minimally → run it → see it pass → commit. No test, no merge.
2. **One logical change per commit**, Conventional Commits (`feat:`, `fix:`, `test:`, `docs:`, `chore:`), never commit with a red build.
3. **File ownership.** In Phase 4 no two lanes touch the same file. The integrator (Yolka) is the only one who merges and runs the full gate.
4. **Determinism.** No timestamps, no random ids, no locale-dependent output in documents or in `--json`. Corpus hashes must be stable.
5. **Zero unverifiable claims.** Agents report artifact paths and raw command output, never summaries like "tests pass".
6. **Licence red line.** A new dependency requires a `docs/DEPS.md` row (name, version, licence, evidence URL, why) **in the same commit**. `cargo deny check` must stay green.
7. **No new API guessing.** If an API is uncertain, pull it from context7 (`mcp__context7__get_prompt`/`query_docs`) or read the vendored source; never invent a function name.
8. **Human review gate.** Anything that changes RTL text output requires a human-reviewed fixture diff (`docs/reviews/`). Agent self-assessment does not count.

---

## 5. Step-by-step tasks

### PHASE 0 — Workspace skeleton (all of it verifiable in one sitting)

**0.1 Create the directory skeleton**
```bash
cd /mnt/c/Users/netcon/playground/ai/pdfrtl
mkdir -p crates/pdfrtl-core/src crates/pdfrtl-core/tests \
         crates/pdfrtl-cli/src crates/pdfrtl-cli/tests \
         crates/pdfrtl-mcp/src \
         corpus/raw/synthetic corpus/raw/generated corpus/raw/private \
         corpus/expected tools scripts docs/decisions docs/reviews docs/research \
         .github/workflows .hermes/plans
```
Verify: `find . -type d | sort | head -30` lists every path above with no error.

**0.2 `.gitattributes` (LF everywhere; stop CRLF churn before it starts)**
```gitattributes
* text=auto eol=lf
*.rs text eol=lf
*.pdf -text
*.ttf binary
*.png binary
```
Verify: `git check-attr eol -- crates/pdfrtl-core/src/lib.rs` → `crates/pdfrtl-core/src/lib.rs: eol: lf`.

**0.3 `.gitignore`**
```gitignore
/target
/target-*
corpus/raw/private/
corpus/tmp/
reports/tmp/
*.sqlite3
*.db
.env
.DS_Store
```
Verify: `git check-ignore -v corpus/raw/private/x.pdf` → prints the matching line.

**0.4 Install the toolchain in WSL and pin it (this is where the exact version comes from — do not guess it)**

Run **inside WSL** (`wsl.exe -d Ubuntu-26.04`):
```bash
sudo apt-get update
sudo apt-get install -y build-essential pkg-config git curl ca-certificates \
  qpdf ghostscript poppler-utils tesseract-ocr tesseract-ocr-fas tesseract-ocr-ara tesseract-ocr-heb
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --component rustfmt,clippy
. "$HOME/.cargo/env"
rustup target add x86_64-unknown-linux-musl
rustc -V && cargo -V
```
Expected: `rustc 1.9x.x (…)` and `cargo 1.9x.x`. **Record those exact strings** — task 0.5 writes them into the repo.

Notes: `qpdf`/`ghostscript`/`poppler-utils`/`tesseract` are **oracles/optional features only** (GPL/AGPL) — see §4.6. If apt is blocked, mirror per the user's Aliyun mirror convention and re-run.

**0.5 `rust-toolchain.toml`** (paste the version from 0.4 into `channel`)
```toml
[toolchain]
channel = "1.9x.x"          # ← exact `rustc -V` from task 0.4
components = ["rustfmt", "clippy"]
targets = ["x86_64-unknown-linux-musl"]
profile = "minimal"
```
Verify: `rustup show active-toolchain` → prints the pinned channel + `(overridden by '…/rust-toolchain.toml')`.

**0.6 Workspace `Cargo.toml`**
```toml
[workspace]
resolver = "2"
members = ["crates/pdfrtl-core", "crates/pdfrtl-cli", "crates/pdfrtl-mcp"]

[workspace.package]
version = "0.0.1"
edition = "2021"
license = "AGPL-3.0-or-later"
publish = false

[workspace.dependencies]
anyhow = "1"
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
lopdf = "0.45"                     # verified 0.45.0, MIT, Send+Sync, rayon
unicode-normalization = "0.1"      # MIT/Apache-2.0
unicode-bidi = "=0.3.18"           # MIT/Apache-2.0, pinned (slow release cadence)
# P2 spikes add, each pinned exactly and after a DEPS.md row:
# krilla = "=0.8.2", pdf-writer = "=0.15.0", harfrust = "=0.13.3", fontdb, skrifa, subsetter
```
Verify: `cargo metadata --no-deps --format-version 1 | python3 -c "import json,sys;print([p['name'] for p in json.load(sys.stdin)['packages']])"` → `['pdfrtl-cli', 'pdfrtl-core', 'pdfrtl-mcp']` (crates must exist first — do 0.7–0.9 before this).

**0.7 `crates/pdfrtl-core/Cargo.toml`**
```toml
[package]
name = "pdfrtl-core"
version.workspace = true
edition.workspace = true
license.workspace = true
publish.workspace = true

[dependencies]
anyhow.workspace = true
serde.workspace = true
lopdf.workspace = true
unicode-normalization.workspace = true
unicode-bidi.workspace = true

[dev-dependencies]
serde_json.workspace = true
```

**0.8 `crates/pdfrtl-cli/Cargo.toml`**
```toml
[package]
name = "pdfrtl-cli"
version.workspace = true
edition.workspace = true
license.workspace = true
publish.workspace = true

[[bin]]
name = "pdfrtl"
path = "src/main.rs"

[dependencies]
anyhow.workspace = true
clap.workspace = true
serde.workspace = true
serde_json.workspace = true
pdfrtl-core = { path = "../pdfrtl-core" }
```

**0.9 `crates/pdfrtl-mcp/Cargo.toml`** (placeholder crate so the workspace is complete; implementation is P2)
```toml
[package]
name = "pdfrtl-mcp"
version.workspace = true
edition.workspace = true
license.workspace = true
publish.workspace = true

[dependencies]
pdfrtl-core = { path = "../pdfrtl-core" }
```

**0.10 The invariant as a type — `crates/pdfrtl-core/src/reasons.rs`** (write the test first)
Test file `crates/pdfrtl-core/tests/reasons.rs`:
```rust
use pdfrtl_core::Reason;

#[test]
fn reasons_serialize_to_stable_snake_case() {
    let json = serde_json::to_string(&Reason::UnsupportedNoEvidence).unwrap();
    assert_eq!(json, "\"unsupported_no_evidence\"");
    assert_eq!(
        serde_json::to_string(&Reason::ActualText).unwrap(),
        "\"actual_text\""
    );
}
```
Run: `cargo test -p pdfrtl-core --test reasons` → **expect a compile error** (`Reason` not found) — that is the RED step.
Then implementation `crates/pdfrtl-core/src/reasons.rs`:
```rust
//! Why a run came out in the order it did — or why we refuse to guess.
//! This type is the product invariant: logical order, or an explicit reason.
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    /// /ActualText marked content carried the logical text verbatim.
    ActualText,
    /// ToUnicode mapped cleanly to base letters and run order was already logical.
    ToUnicodeLogical,
    /// ToUnicode mapped cleanly; run order was reconstructed with UAX #9 levels.
    BidiReordered,
    /// Producer is allow-listed as storing visual order (recorded per producer, never guessed).
    ProducerVisualOrderKnown,
    /// No reliable recovery path — refuse rather than emit reversed text.
    UnsupportedNoEvidence,
    /// ToUnicode missing or mapping to C0 control characters.
    UnsupportedBrokenToUnicode,
}
```
Re-run: `cargo test -p pdfrtl-core --test reasons` → `test result: ok. 1 passed`.

**0.11 `crates/pdfrtl-core/src/docinfo.rs`** — file-level facts, no text decoding yet.
Test `crates/pdfrtl-core/tests/docinfo.rs` (fixture created in 0.13):
```rust
use std::path::Path;
use pdfrtl_core::inspect;

#[test]
fn inspect_reports_pages_and_producer() {
    let info = inspect(Path::new("../../corpus/raw/synthetic/minimal-ltr.pdf")).unwrap();
    assert_eq!(info.pages, 1);
    assert_eq!(info.producer.as_deref(), Some("pdfrtl-gen"));
}
```
Run: `cargo test -p pdfrtl-core --test docinfo` → fails to compile (RED).
Implementation (**API warning**: verify every lopdf call against context7 in task 2.1 before committing; `Document::load`, `get_pages()`, `trailer` are the intended entry points — if the real signatures differ, fix the code, not the test):
```rust
//! Level-1 inspection: what the file *is*, before any text handling.
use anyhow::{Context, Result};
use lopdf::Document;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Serialize)]
pub struct DocInfo {
    pub path: String,
    pub pages: u32,
    pub producer: Option<String>,
    pub creator: Option<String>,
    pub pdf_version: Option<String>,
    pub encrypted: bool,
    /// P1 fills this; P0 reports None = "not analysed yet", never false.
    pub has_actual_text: Option<bool>,
}

pub fn inspect(path: &Path) -> Result<DocInfo> {
    let doc = Document::load(path).with_context(|| format!("loading {}", path.display()))?;
    let info_dict = doc
        .trailer
        .get(b"Info")
        .ok()
        .and_then(|o| o.as_reference().ok())
        .and_then(|id| doc.get_object(id).ok())
        .and_then(|o| o.as_dict().ok());

    let get = |key: &[u8]| -> Option<String> {
        info_dict
            .and_then(|d| d.get(key).ok())
            .and_then(|o| o.as_str().ok())
            .map(|b| String::from_utf8_lossy(b).into_owned())
    };

    Ok(DocInfo {
        path: path.display().to_string(),
        pages: doc.get_pages().len() as u32,
        producer: get(b"Producer"),
        creator: get(b"Creator"),
        pdf_version: doc.version.clone(),
        encrypted: doc.is_encrypted(),
        has_actual_text: None,
    })
}
```
Re-run → `test result: ok. 1 passed`.
Commit: `feat(core): inspect() reports level-1 document facts`.

**0.12 `crates/pdfrtl-core/src/lib.rs`**
```rust
//! pdfrtl core: document model, RTL text pipeline, extraction, generation.
//! Invariant: every text result is logical order, or carries an explicit `Reason`.
pub mod docinfo;
pub mod reasons;

pub use docinfo::{inspect, DocInfo};
pub use reasons::Reason;
```

**0.13 Minimal synthetic fixture generator — `tools/gen_minimal_pdf.py`** (committed, deterministic, no binary blob in git)
```python
#!/usr/bin/env python3
"""Emit a minimal, valid, deterministic 1-page PDF. No deps, no timestamps."""
import sys, pathlib

OUT = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "corpus/raw/synthetic/minimal-ltr.pdf")
TEXT = b"Hello pdfrtl"

objects = [
    b"<< /Type /Catalog /Pages 2 0 R >>",
    b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
    b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 80] "
    b"/Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>",
    None,  # content stream, filled below
    b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
]
stream = b"BT /F1 12 Tf 20 40 Td (" + TEXT + b") Tj ET"
objects[3] = (
    b"<< /Length " + str(len(stream)).encode() + b" >>\nstream\n" + stream + b"\nendstream"
    if False else
    b"<< /Length " + str(len(stream)).encode() + b" >>\nstream\n" + stream + b"\nendstream"
)

out = bytearray(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n")
offsets = []
for i, body in enumerate(objects, start=1):
    offsets.append(len(out))
    out += f"{i} 0 obj\n".encode() + body + b"\nendobj\n"
xref = len(out)
out += f"xref\n0 {len(objects)+1}\n".encode() + b"0000000000 65535 f \n"
for off in offsets:
    out += f"{off:010d} 00000 n \n".encode()
out += (
    f"trailer\n<< /Size {len(objects)+1} /Root 1 0 R "
    f"/Info {len(objects)+1} 0 R >>\nstartxref\n{xref}\n%%EOF\n"
).encode()
# /Info object appended last so the trailer reference is valid
info_off = len(out)
out += (
    f"{len(objects)+1} 0 obj\n<< /Producer (pdfrtl-gen) /Creator (pdfrtl-gen) >>\nendobj\n".encode()
)
OUT.parent.mkdir(parents=True, exist_ok=True)
OUT.write_bytes(bytes(out))
print(f"wrote {OUT} ({len(out)} bytes)")
```
Run:
```bash
python tools/gen_minimal_pdf.py                     # Windows: `python`; WSL: `python3`
pdftotext corpus/raw/synthetic/minimal-ltr.pdf -    # oracle check
```
Expected (oracle): stdout contains `Hello pdfrtl`. Then `cargo test -p pdfrtl-core --test docinfo` → `test result: ok. 1 passed`.
Note for the implementer: if `pdftotext` prints nothing, the xref offsets are wrong — fix the generator, **not** the test; the file must pass a real parser.

**0.14 `crates/pdfrtl-cli/src/main.rs`** — the CLI contract, JSON-first.
Test `crates/pdfrtl-cli/tests/cli_contract.rs`:
```rust
use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_pdfrtl"))
}

#[test]
fn json_envelope_shape() {
    let out = bin()
        .args(["--json", "inspect", "../../corpus/raw/synthetic/minimal-ltr.pdf"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["ok"], serde_json::json!(true));
    assert_eq!(v["data"]["pages"], serde_json::json!(1));
    assert!(v["data"]["producer"].is_string());
}

#[test]
fn missing_file_is_exit_4_with_reason() {
    let out = bin()
        .args(["--json", "inspect", "does-not-exist.pdf"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(4));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["ok"], serde_json::json!(false));
}
```
Run: `cargo test -p pdfrtl-cli` → compile error / RED.
Implementation:
```rust
//! pdfrtl CLI. Contract: every verb prints one JSON envelope on stdout.
//! Exit codes: 0 ok · 2 usage · 3 unsupported (with a `Reason`) · 4 io/parse error.
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;

const EXIT_UNSUPPORTED: u8 = 3;
const EXIT_IO: u8 = 4;

#[derive(Parser)]
#[command(name = "pdfrtl", version, about = "RTL-first PDF toolkit for agents")]
struct Cli {
    /// Emit the JSON envelope on stdout (default is human text)
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Report document facts (pages, producer, encryption, marked content)
    Inspect { file: PathBuf },
}

fn envelope<T: serde::Serialize>(ok: bool, data: T, reasons: &[pdfrtl_core::Reason]) -> String {
    serde_json::json!({ "ok": ok, "data": data, "reasons": reasons }).to_string()
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Inspect { file } => match pdfrtl_core::inspect(&file) {
            Ok(info) => {
                println!("{}", envelope(true, &info, &[]));
                ExitCode::SUCCESS
            }
            Err(e) => {
                let payload = serde_json::json!({ "error": e.to_string() });
                println!("{}", envelope(false, payload, &[]));
                ExitCode::from(EXIT_IO)
            }
        },
    }
}
```
Re-run: `cargo test -p pdfrtl-cli` → `2 passed`.
Smoke: `cargo run -q -p pdfrtl-cli -- --json inspect corpus/raw/synthetic/minimal-ltr.pdf` → one-line JSON with `"ok":true`.

**0.15 `docs/CLI.md`** — write the contract down (exit codes 0/2/3/4, envelope shape, `--json` on every verb, `--schema` planned for P1, provenance fields planned: `page`, `object_id`, `char_range`, `reason`). Verify: `grep -c '^' docs/CLI.md` ≥ 25.

**0.16 `docs/decisions/0001-license-agpl-plus-commercial.md`** (ADR, human-authored) — records Option A, the free/paid split, and **why copyleft deps are still banned** (dual licensing requires owning the whole work). Verification: file exists and states the rule in one sentence.

**0.17 `docs/decisions/0002-logical-order-invariant.md`** — the invariant + the reason-code table; states that no blanket reversal may ever be merged.

**0.18 `docs/decisions/0003-permissive-only-default-features.md`** — the dependency rule, with the CI enforcement pointer (`deny.toml`).

**0.19 `deny.toml` + the licence gate**
```toml
[advisories]
yanked = "deny"
[licenses]
confidence-threshold = 0.9
allow = ["MIT", "Apache-2.0", "Apache-2.0 WITH LLVM-exception", "BSD-2-Clause", "BSD-3-Clause",
         "ISC", "Unicode-3.0", "Zlib", "MPL-2.0", "CC0-1.0", "Unlicense", "BSL-1.0", "OpenSSL"]
[bans]
multiple-versions = "warn"
wildcards = "deny"
[sources]
unknown-registry = "deny"
unknown-git = "deny"
```
Install + run (WSL): `cargo install cargo-deny --locked && cargo deny check` → expected `licenses ok`, `bans ok` (no GPL/AGPL in the tree).

**0.20 `docs/DEPS.md` + `THIRD-PARTY-NOTICES.md`** — one row per dependency: name · version · licence · evidence URL · why · shipped-or-oracle. Seed with lopdf/stub, unicode-normalization, unicode-bidi, clap, serde, anyhow — and the oracle-only rows (poppler `pdftotext` GPL-2.0+, qpdf Apache-2.0, mutool AGPL-3.0, tesseract Apache-2.0). Verify: `grep -c '|' docs/DEPS.md` ≥ 20.

**0.21 `AGENTS.md` (repo root)** — the binding contract for future agents: the eight rules from §4, the file-ownership rule for concurrent lanes, the "logical order or a `Reason`" invariant, "no dependency without a DEPS.md row", "no GUI in core", the exact test commands, and the WSL/`CARGO_TARGET_DIR` setup from task 0.24. Copy the *shape* of `../yolka/tmp/albdf-recon/albdf/AGENTS.md` (proven), not its content.

**0.22 `scripts/db.py` — the knowledge DB (decision: SQLite + FTS5, not a vector store)**

Rationale: zero services, single file, full-text search, works offline on every machine, and the exact pattern already proven in `al-bdf-engine` (`db/albdf.db` + `scripts/db.py`). A vector DB / RAG stack is rejected: overkill for ≤200 documents, adds a runtime, and the user rejects tooling that cannot be understood at a glance.

Schema (`scripts/db.py init` creates `db/pdfrtl.db`):
```sql
CREATE TABLE IF NOT EXISTS tasks(
  id INTEGER PRIMARY KEY, kind TEXT NOT NULL, title TEXT NOT NULL, body TEXT,
  status TEXT NOT NULL DEFAULT 'open', phase TEXT, owner TEXT, ref TEXT,
  created_at TEXT NOT NULL, closed_at TEXT);
CREATE TABLE IF NOT EXISTS knowledge(
  id INTEGER PRIMARY KEY, kind TEXT NOT NULL,       -- finding | lesson | pitfall | decision | spec
  title TEXT NOT NULL, body TEXT NOT NULL, source TEXT, created_at TEXT NOT NULL);
CREATE VIRTUAL TABLE IF NOT EXISTS knowledge_fts USING fts5(title, body, content='knowledge', content_rowid='id');
```
Commands to implement: `init`, `task-add --kind --title --body --phase`, `task-done <id> --ref <sha>` (refuses without a ref), `search "<query>"` (FTS5), `status`, `note --kind --title --body`. Verify:
```bash
python3 scripts/db.py init && python3 scripts/db.py note --kind pitfall \
  --title "CIDToGIDMap must be 65536 entries" --body "ISO 32000-1 9.7.4.3; short array paints Latin glyphs" \
  && python3 scripts/db.py search "CIDToGIDMap"
```
Expected: one row printed containing the title. (`python` on Windows, `python3` in WSL — the script must not care.)

**0.23 `AUTHORSHIP.md` + commit trailer convention** — one page: who designs (human), who implements (agents), why that matters for copyright, and the trailer every agent commit must carry:
```
Designed-by: Yolka <alavi2004@outlook.com>
Implemented-by: <agent name/model>
Reviewed-by: <human or agent + date>
```
Verify: `git log -1 --format=%B | grep '^Designed-by:'`.

**0.24 WSL build config — `..wsl.env` → `scripts/wsl-build.sh`** (keep sources on Windows, artifacts on ext4)
```bash
#!/usr/bin/env bash
# Run the full gate from WSL against the Windows checkout.
set -euo pipefail
export CARGO_TARGET_DIR="$HOME/target-pdfrtl"
cd /mnt/c/Users/netcon/playground/ai/pdfrtl
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check
```
Verify (WSL): `bash scripts/wsl-build.sh` → ends with `test result: ok` lines and `licenses ok`. Expected duration on first run: several minutes (dependency download); afterwards seconds.

**0.25 First commit**
```bash
git init -q 2>/dev/null || true
git add -A
git -c user.name="Yolka" -c user.email="alavi2004@outlook.com" commit -m "chore: workspace skeleton, JSON CLI contract, licence gate, knowledge DB

Designed-by: Yolka <alavi2004@outlook.com>
Implemented-by: agent
Reviewed-by: Yolka"
```
Verify: `git log --oneline` shows one commit; `git status --short` is clean.

---

### PHASE 1 — The question loop (do this BEFORE writing product code)

**1.1 Create `docs/OPEN-QUESTIONS.md`** with a strict, machine-parseable format so the loop terminates:
```markdown
# Open questions

Status legend: `open` (blocks work) · `answered` · `parked` (does not block).
Rule: a question may only be `open` if some task in the plan cannot start without it.

## Q-001 — <title>
- **status:** open
- **blocks:** task 4.x / phase P2
- **question:** <one sentence, answerable>
- **why it matters:** <what changes based on the answer>
- **answer:** _(filled by the upstream agent)_
```
**1.2 Seed it with the questions already known** (verbatim list, each a separate entry): Q-001 deployment model of the paid tier (self-hosted commercial binary vs hosted SaaS — decides whether AGPL §13 shapes the MCP design); Q-002 is extraction or generation the v1 headline (which one the demo must nail); Q-003 exact script/language scope (fa + ar + he + en only? Urdu, Kurdish, Syriac? vertical scripts?); Q-004 is a GUI in scope for this repo or does it consume the CLI/C ABI; Q-005 corpus redistribution rights (may fixtures containing textbook pages ship in a public repo, or generators+hashes only); Q-006 who is the native-speaker reviewer for Persian/Arabic output, and is oracle-only review acceptable for P0; Q-007 can we reuse `al-bdf-engine` code (who owns the RTL commits) — if yes, under which licence; Q-008 project name + public repo org; Q-009 is P0 a customer-facing demo or internal; Q-010 target packaging (single static binary per OS, or a container image too).
**1.3 Loop rule:** after each answer batch, re-read the list, mark `answered`, and only then re-plan. **Exit condition:** zero `open` entries. Record the exit in `docs/reviews/`.
**1.4** For every answered question, write the decision as an ADR (`docs/decisions/NNNN-<slug>.md`) — answers that live only in chat are lost.

---

### PHASE 2 — Context and tools we must pull before implementing

**2.1–2.6 Pull version-exact API docs via context7** (this machine's context7 MCP is verified working). One call per library, saved as `docs/research/NNN-<lib>.md` with the version string in the filename:
| Task | Library | What to extract |
|---|---|---|
| 2.1 | `lopdf` | `Document::load`, `get_pages`, `trailer`/`get_object`/`as_dict`, **content-stream decoding and whether marked content (BDC/EMC) survives a re-encode** |
| 2.2 | `krilla` | font subsetting, CID `ToUnicode`, `/ActualText`, `/ReversedChars`, PDF/A + PDF/UA validation API |
| 2.3 | `pdf-writer` | `PropertyList::actual_text`, marked-content begin/end, low-level emit |
| 2.4 | `harfrust` | buffer setup, cluster levels, absolute cluster offsets, glyph advances/marks, script/language detection |
| 2.5 | `unicode-bidi` | `BidiInfo`, paragraph levels, run extraction, mirroring |
| 2.6 | MCP server options in Rust | is there a maintained Rust MCP SDK? If yes → `pdfrtl-mcp` stays Rust; if not → thin TypeScript server over the CLI (node 26 is present) |

**2.7 Oracle inventory in WSL** — record exact versions into `docs/research/oracles.md`:
```bash
qpdf --version; pdftotext -v 2>&1 | head -1; gs --version; mutool -v 2>&1 | head -1
```
(PDFium static binaries from `bblanchon/pdfium-binaries` get pinned + sha256-recorded here when P1 needs pixel goldens.)

**2.8 Network doctrine for agents (write it in `AGENTS.md`)** — direct egress to many research hosts is blocked from this network; the working pattern is `curl --socks5-hostname 127.0.0.1:10808 …`, and Firecrawl is available through the same route (key in the profile `.env`). Subagents that need the web get this recipe in their context, verbatim — last batch of agents lost time to missing web access.

---

### PHASE 3 — Test corpus: books, papers, and producer variants

**3.1 Corpus layout + manifest.** `corpus/manifest.json` entries:
```json
{
  "id": "fa-wikisource-gulistan-001",
  "path": "corpus/raw/public/fa-gulistan.pdf",
  "language": "fa", "script": "Arab", "direction": "rtl",
  "producer": "unknown", "producer_class": "unknown",
  "licence": "public-domain", "source_url": "https://…", "sha256": "…",
  "redistributable": true,
  "notes": "lithograph scan with text layer; expect ToUnicode degradation"
}
```
**Rule:** `redistributable: false` ⇒ the file lives in `corpus/raw/private/` (gitignored) and only the manifest row + sha256 is committed.

**3.2–3.5 Acquire real-world files, four languages.** Rules: public domain / CC / own copies only; record `source_url` + `sha256` in the same commit as the manifest row. Candidate sources and search queries to run (verify jurisdiction before publishing anything):
- **Persian (fa):** `کتاب درسی ریاضی پایه پنجم pdf` · `کتاب فارسی پنجم دبستان pdf` (official textbooks: `chap.sch.ir`) · `گلستان سعدی pdf` · `کلیله و دمنه pdf` · `مثنوی معنوی pdf` · Wikisource fa, Internet Archive, ganjoor exports (text→PDF).
- **Arabic (ar):** `كتاب اللغة العربية للصف الخامس pdf` · `كليلة ودمنة pdf` · `ألف ليلة وليلة pdf` · `مقدمة ابن خلدون pdf` · Wikisource ar, shamela.ws, `مكتبة نور` (verify licence per file).
- **Hebrew (he):** `ספר לימוד חשבון כיתה ה pdf` · Sefaria exports (Tanakh, Mishnah — CC0) · Ben-Yehuda Project texts · Wikisource he.
- **English (en, LTR control):** Project Gutenberg classics (e.g. *Pride and Prejudice*, *Moby-Dick*) + 2–3 `arXiv` papers with figures/tables (math layout stress).
- **Mixed-direction:** any Persian/Arabic/Hebrew doc with embedded Latin (formulas, URLs, code) — the hardest and most valuable class.

**3.6–3.9 Producer variants (the axis that actually finds bugs).** Take one fixed sentence set per language (Persian with ZWNJ + lam-alef + Persian digits; Arabic with harakat + lam-alef; Hebrew with niqqud; mixed fa+en) and render it through: **Word, LibreOffice, Chrome/Edge print-to-PDF, XeLaTeX, ReportLab, mPDF, wkhtmltopdf, InDesign (if available)**. Store under `corpus/raw/generated/<producer>/<script>.pdf`, one manifest row each. These are the files that expose visual-vs-logical storage per producer. Everything generated is redistributable.
**3.10 `corpus/expected/<id>.json`** — the expected logical text per fixture, **human-authored/verified** (this is the ground truth; record `verified_by` + date).

**Deliverable of Phase 3 for the user:** a table of *titles to pull*, one per language, with the exact query, so the human can fetch anything the agents cannot (logins, scanners). Keep it in `docs/CORPUS-ACQUISITION.md`.

---

### PHASE 4 — P0 implementation in four concurrent lanes

**Frozen interfaces first (task 4.0, integrator-authored, committed before lanes start):**
- `pdfrtl_core::{DocInfo, Reason, inspect}`, `pdfrtl_core::extract::Extract` (P1 stub).
- CLI envelope `{"ok":bool,"data":…,"reasons":[Reason…]}` + exit codes 0/2/3/4.
- `corpus/expected/<id>.json` schema from 3.10.
- Branch naming `lane/<n>-<slug>`, one git worktree per lane (see the `parallel-git-workspaces` skill), integrator merges serially.

| Lane | Owns (exclusive) | Deliverable | Agent | Acceptance |
|---|---|---|---|---|
| **W1** RTL text core | `crates/pdfrtl-core/src/text/**`, `crates/pdfrtl-core/tests/text_*` | normalizer (6 steps) + bidi runs + shaper adapter + emission policy; ports the albdf *design* | `pdfrtl-core-rtl` | unit tests per step; lam-alef, ZWNJ, digit-folding, mixed-bidi fixtures pass; every output carries a `Reason` |
| **W2** CLI + JSON contract | `crates/pdfrtl-cli/**`, `docs/CLI.md` | `inspect`, `extract --json`, `--schema`, exit-code contract, JSONL streaming | `pdfrtl-cli` | CLI tests green; golden JSON for 3 fixtures; no stdout noise with `--json` |
| **W3** corpus + oracles + harness | `corpus/**`, `tools/**`, `tests/oracle/**` | fixture corpus, generator scripts, oracle runner (pdftotext/qpdf/PDFium), `reports/` writer | `pdfrtl-corpus` | `bash scripts/run-corpus.sh` produces a per-fixture pass/fail report with reason codes |
| **W4** infra + knowledge | `.github/workflows/**`, `Dockerfile`, `scripts/**`, `docs/**` | CI (fmt/clippy/test/deny + oracle lane), Dockerfile (test + static musl binary), `scripts/db.py`, `HANDOFF.md` | `pdfrtl-infra` | CI green on a scratch branch; `docker build` documented as runnable on a machine with Docker; handoff regenerates |

**4.1** Integrator writes `docs/plans/P0-lanes.md` with the ownership table above and the merge order (W4 infra → W1 text → W3 corpus → W2 CLI), because W2's tests need W1's types and W3's fixtures.

**4.2** Dispatch lanes as background subagents **with `context` containing**: the repo path, the file-ownership table, the 8 rules from §4, the exact test commands, the context7 doc paths from Phase 2, and the network doctrine (2.8). One lane per subagent; the integrator merges.

**4.3** After each lane returns: **verify, do not trust** — run `scripts/wsl-build.sh` on the lane branch, read the diff, check the claimed tests exist and fail when reverted (`git stash` the implementation, run the test, expect failure).

**4.4** Merge order as in 4.1; one merge commit per lane with the trailer block from 0.23.

**4.5** P0 exit report → `reports/<date>-p0.md`: per-fixture result, reason codes, oracle agreement (PDFium pixels vs `pdftotext` vs ours), and an explicit list of **producers we cannot yet handle** (that list is the honest product statement).

---

### PHASE 5 — The QA / de-slop loop (after every lane and every phase)

**5.1** A dedicated review agent reads the merged diff (not the summary) and applies this checklist, writing `docs/reviews/<date>-<lane>.md`:
- dead code, unused deps, `unwrap()/expect()` in library paths, `unsafe` without justification, `println!` outside the CLI envelope, TODOs without a DB task id;
- duplication (two normalizers, two envelope builders), speculative abstraction (traits with one impl and no consumer);
- API-shape drift (a `Reason` variant added without a fixture);
- licence/dependency drift (new crate without a DEPS.md row);
- determinism violations (timestamps, hashmaps in output ordering).
**5.2** Fix pass: strip, then `cargo clippy --workspace --all-targets -- -D warnings` + full test run + `cargo deny check` must be green.
**5.3** The anti-slop gate is enforced in CI (`.github/workflows/slop.yml`): grep-based bans for `unwrap()` outside tests, `todo!()`, `dbg!`, `println!` in `pdfrtl-core`, and a dependency-count ratchet (count may only go down without an ADR).
**5.4** Anything re-learned goes into `scripts/db.py note --kind pitfall` **and** into the `rtl-pdf-text-pipeline` skill, so the next machine starts smarter.

---

### PHASE 6 — Knowledge, handoff, packaging

**6.1** `docs/AGENT.md` guides per area (`crates/pdfrtl-core/AGENT.md`, `crates/pdfrtl-cli/AGENT.md`, `corpus/AGENT.md`, `scripts/AGENT.md`) — the pattern proven in `al-bdf-engine`: what this area is, how to run its tests, the top three traps, and where the truth lives.
**6.2** `HANDOFF.md` (regenerated each milestone by `scripts/handoff.sh`): current phase, what changed, exact commands to reproduce, open questions, known-broken producers, next three tasks. Must be sufficient for another machine to continue with zero chat history.
**6.3** `Dockerfile` — two stages, no C++ anywhere:
```dockerfile
FROM rust:1-bookworm AS build
WORKDIR /src
COPY . .
RUN cargo build --release --locked -p pdfrtl-cli

FROM debian:bookworm-slim AS test
RUN apt-get update && apt-get install -y --no-install-recommends qpdf poppler-utils && rm -rf /var/lib/apt/lists/*
COPY --from=build /src/target/release/pdfrtl /usr/local/bin/pdfrtl
COPY corpus /corpus
CMD ["bash","-lc","pdfrtl --json inspect /corpus/raw/synthetic/minimal-ltr.pdf && cargo --version || true"]

FROM scratch AS dist
COPY --from=build /src/target/release/pdfrtl /pdfrtl
ENTRYPOINT ["/pdfrtl"]
```
(Static `musl` variant added in P2 when PDFium is optional-but-linked.) Verify on a Docker-capable host: `docker build --target test -t pdfrtl:test .` → image builds; `docker run --rm pdfrtl:test` prints one JSON envelope.
**6.4** `.github/workflows/ci.yml` — jobs: `fmt`, `clippy`, `test` (ubuntu-latest, + windows-latest for P1), `deny`, `slop`, `oracle` (installs qpdf/poppler, runs the corpus, uploads `reports/`), `pure-rust` (all heavy features off — must always be green).
**6.5** Cross-machine continuity: repo + `db/pdfrtl.db` + `HANDOFF.md` are the source of truth; profiles/skills travel separately with the existing `hermes-cross-machine-sync` / `hermes-profile-import` skills. Record that in `HANDOFF.md`.
**6.6** Register the `pdfrtl` project in the user's identity repo (`yolka-wiz/yolka`) pointer list at each milestone — same pattern as the other projects.

---

### PHASE 7 — Milestone verification (repeat for every major upgrade)

**7.1** Run the full corpus: `bash scripts/run-corpus.sh --report reports/<date>-<phase>.md`.
**7.2** Cross-check against an independent implementation for every fixture: `pdftotext -raw` (poppler, oracle) and, once PDFium is pinned, a pixel golden. Disagreements are **not** automatically our bug — record which side is right and why.
**7.3** Manual human spot-check: render 3 fixtures (one per script) to PNG and eyeball them; copy-paste from Chrome and compare to the expected logical text. This is the check that catches "tests green, pixels garbage".
**7.4** Update `HANDOFF.md`, `docs/PROBLEMS.md` (our own failure log), `db/pdfrtl.db`, and the version line in `README.md`; commit as `chore(release): <phase> verification report`.

---

### PHASE P1–P4 outline (plan detail gets written per phase, not now)

| Phase | Scope | Gate |
|---|---|---|
| P1 | `extract --json` with positions/reading order; producer-aware logical-order recovery; bidi search; provenance ids; JSONL | expected logical text matches for every supported producer; unsupported ⇒ `Reason`, never guess |
| P2 | generation (shaping → bidi → krilla/pdf-writer with `ToUnicode` + `/ActualText` + `/ReversedChars`); PDFium golden lane; static musl dist | pixel golden + copy-paste round-trip + `qpdf --check` |
| P3 | content edit with marked-content preservation, page ops, redaction with proof; C ABI + PyO3 + napi-rs | edit→re-extract invariants; bindings published to a private index |
| P4 | PAdES/LTV, forms + RTL appearance streams, PDF/A-2b + PDF/UA-1 via veraPDF (MPL branch) | signature validates in Adobe + `pdfsig`; veraPDF passes |

---

## 6. Tests / validation

- **Per task:** the failing test is written and *observed failing* before implementation (Phase 0 tasks 0.10–0.14 show the exact RED/GREEN commands).
- **Per lane:** `bash scripts/wsl-build.sh` (fmt + clippy `-D warnings` + tests + `cargo deny`) — expected final lines `test result: ok.` and `licenses ok`.
- **Per phase:** the corpus report (`reports/<date>-<phase>.md`) with per-fixture pass/fail + reason codes, plus at least one human-verified pixel/copy-paste check per script.
- **Negative tests are mandatory**: a fixture whose order we *cannot* recover must produce `unsupported_no_evidence` and exit code 3 — proving we refuse rather than guess.
- **Anti-fake-results rule:** a task is done only with the command output pasted into the review file; "tests pass" without output is not evidence.

## 7. Risks, tradeoffs, open questions

| # | Risk | Severity | Mitigation / decision needed |
|---|---|---|---|
| 1 | **Dual licensing needs enforceable copyright**, and agents write the code | Critical | human-authored ADRs/corpus/reviews + commit trailers (0.23); if that turns out to be unacceptable, fall back to Option B (permissive core, paid services) *before* publishing — the crate split keeps it cheap |
| 2 | **`lopdf` may not preserve marked content** through a rewrite (albdf had to patch its own editor for this) | High | spike in lane W1 week 1 (task 2.1 question); if broken → extend `lopdf` upstream or keep marked content in our own side-car structure |
| 3 | **No Rust toolchain on this Windows host** | High | WSL Ubuntu-26.04 is the build environment (0.4, 0.24); Windows-native rustup MSVC is a later fallback |
| 4 | **`/mnt/c` filesystem slowness** for cargo | Medium | `CARGO_TARGET_DIR=$HOME/target-pdfrtl` (0.24); if builds still drag, move the canonical checkout to ext4 and mirror to Windows |
| 5 | **Corpus licensing**: textbooks are copyrighted | High | public-domain/CC first, `redistributable:false` ⇒ gitignored + hash-only manifest; never publish scanned textbooks |
| 6 | **No native-speaker reviewer** for Arabic/Persian output | High | Q-006: either recruit a reviewer or accept oracle-only review for P0 and label it in `reports/` |
| 7 | **Agent web access is flaky** on this network (last batch lost time) | Medium | network doctrine (2.8) passed to every lane's context |
| 8 | **Concurrent lanes collide** in git | Medium | exclusive file ownership (4.1), one worktree per lane, serial integrator merges |
| 9 | **Scope creep** into OCR/VLM/chunking/PDF-A before P0 ships | Medium | YAGNI: explicitly out of P0/P1; each new surface needs a named consumer and an ADR |
| 10 | **krilla/hayro single-maintainer SPOF** | Medium | vendor the pinned revision at P2; keep our own emitter as a tested fallback |

**Open questions live in `docs/OPEN-QUESTIONS.md` (Phase 1) — the loop continues until none are `open`.** The ten seeded questions above are the ones that block P0/P1 decisions today.

## 8. What "done" looks like for this plan

`cargo test --workspace` green in WSL, `cargo deny check` clean, a committed corpus with ≥12 fixtures across 4 languages and ≥3 producers, `reports/<date>-p0.md` showing per-fixture results **including at least one honest `unsupported_no_evidence`**, `HANDOFF.md` reproducible on a second machine, and `docs/OPEN-QUESTIONS.md` with zero `open` items.
