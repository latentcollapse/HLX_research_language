# HLX Compiler and LSP Server Docker Image
# Multi-stage build for optimal image size

# ==============================================================================
# Stage 1: Builder - Compile Rust binaries
# ==============================================================================
FROM rust:1.75-bookworm AS builder

# Install system dependencies required for building
RUN apt-get update && apt-get install -y \
    build-essential \
    cmake \
    pkg-config \
    libssl-dev \
    llvm-18-dev \
    libclang-18-dev \
    clang-18 \
    libsdl2-dev \
    libvulkan-dev \
    && rm -rf /var/lib/apt/lists/*

# Set LLVM environment variables for inkwell
ENV LLVM_SYS_180_PREFIX=/usr/lib/llvm-18

# Create app directory
WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./

# Copy all crate directories
COPY hlx_core ./hlx_core
COPY hlx_compiler ./hlx_compiler
COPY hlx_runtime ./hlx_runtime
COPY hlx_cli ./hlx_cli
COPY hlx_backend_llvm ./hlx_backend_llvm
COPY hlx_lsp ./hlx_lsp

# Build release binaries with optimizations
RUN cargo build --release --bin hlx --bin hlx_lsp

# Verify binaries were built
RUN ls -lh /app/target/release/hlx /app/target/release/hlx_lsp

# ==============================================================================
# Stage 2: Runtime - Minimal image with only necessary dependencies
# ==============================================================================
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libsdl2-2.0-0 \
    libvulkan1 \
    llvm-18-runtime \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user for security
RUN useradd -m -u 1000 -s /bin/bash hlx

# Create directories
RUN mkdir -p /app/bin /app/examples /home/hlx/.hlx

# Copy binaries from builder
COPY --from=builder /app/target/release/hlx /app/bin/hlx
COPY --from=builder /app/target/release/hlx_lsp /app/bin/hlx_lsp

# Copy example files (optional)
COPY --chown=hlx:hlx examples /app/examples

# Set ownership
RUN chown -R hlx:hlx /app /home/hlx

# Switch to non-root user
USER hlx
WORKDIR /home/hlx

# Add binaries to PATH
ENV PATH="/app/bin:${PATH}"

# Default command: show version
CMD ["hlx", "--version"]

# ==============================================================================
# Labels
# ==============================================================================
LABEL maintainer="HLX Contributors"
LABEL description="HLX Compiler and Language Server"
LABEL version="0.1.0"
LABEL org.opencontainers.image.source="https://github.com/latentcollapse/hlx-compiler"
LABEL org.opencontainers.image.licenses="Apache-2.0"
