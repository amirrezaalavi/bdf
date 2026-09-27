# pdfrtl — container packaging.
#
# Three targets, so a future agent on any machine can (a) run the test suite,
# (b) get a throwaway CLI to poke at files, (c) produce the shipped artifact.
#
#   docker build --target test -t pdfrtl:test .      && docker run --rm pdfrtl:test
#   docker build --target cli  -t pdfrtl:cli  .      && docker run --rm pdfrtl:cli --help
#   docker build --target dist -t pdfrtl:dist .
#
# Notes
#  * The core is pure Rust with permissive-only dependencies, so there is no C/C++
#    toolchain, no PDFium and no system PDF library in the build — by design
#    (docs/decisions/0003). PDFium arrives at P2 as an *optional* feature; this file
#    gets a `--features pdfium` build stage then.
#  * Oracle tools (qpdf, poppler) are installed in the `test` stage only, because the
#    corpus harness needs an independent parser to disagree with. They are never copied
#    into `dist`.
#  * Rust version is pinned to the same one as CI (1.97.1); keep in sync with
#    rust-toolchain.toml and Cargo.toml's rust-version.

FROM rust:1.97-bookworm AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock rust-toolchain.toml deny.toml ./
COPY crates ./crates
COPY tools ./tools
COPY scripts ./scripts
# `--locked` is mandatory: a build that silently resolves different versions is not
# reproducible, and the corpus hashes depend on reproducible behaviour.
RUN cargo build --release --locked -p pdfrtl-cli

# --- test: the full suite plus the oracle tools the harness shells out to ------------
FROM rust:1.97-bookworm AS test
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      qpdf poppler-utils ghostscript python3 ca-certificates \
 && rm -rf /var/lib/apt/lists/*
WORKDIR /src
COPY . .
CMD ["bash", "-lc", "cargo test --workspace --locked && python3 scripts/run-corpus.py --binary target/debug/pdfrtl"]

# --- cli: a small runtime image for humans and agents to try the binary --------------
FROM debian:bookworm-slim AS cli
RUN apt-get update \
 && apt-get install -y --no-install-recommends qpdf poppler-utils \
 && rm -rf /var/lib/apt/lists/*
COPY --from=build /src/target/release/pdfrtl /usr/local/bin/pdfrtl
COPY corpus /corpus
WORKDIR /corpus
ENTRYPOINT ["pdfrtl"]
CMD ["--json", "inspect", "/corpus/raw/synthetic/minimal-ltr.pdf"]

# --- dist: the shipped artifact. Static musl build (P2) lands here -------------------
FROM scratch AS dist
COPY --from=build /src/target/release/pdfrtl /pdfrtl
ENTRYPOINT ["/pdfrtl"]
