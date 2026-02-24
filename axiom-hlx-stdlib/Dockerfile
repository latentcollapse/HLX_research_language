## Axiom Language — Deterministic, effect-typed, agent-oriented
##
## Build:   docker build -t axiom .
## Run:     docker run --rm -v $(pwd)/examples:/work axiom /work/hello.axm
## REPL:    docker run --rm -it axiom --help

# ── Stage 1: Build ──────────────────────────────────────────────
FROM rust:1.77-slim AS builder

WORKDIR /build
COPY Cargo.toml Cargo.lock* ./
COPY src/ src/
COPY stdlib/ stdlib/

# Build release binary
RUN cargo build --release && \
    strip /build/target/release/axiom-lang

# ── Stage 2: Runtime ────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Copy binary
COPY --from=builder /build/target/release/axiom-lang /usr/local/bin/axiom

# Copy stdlib for runtime imports
COPY --from=builder /build/stdlib /usr/local/lib/axiom/stdlib

# Copy examples for quick start
COPY examples/ /usr/local/share/axiom/examples/

# Non-root user for safety
RUN useradd -m -s /bin/bash axiom
USER axiom
WORKDIR /home/axiom

ENTRYPOINT ["axiom"]
CMD ["--help"]
