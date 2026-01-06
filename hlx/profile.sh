#!/bin/bash
set -e

# HLX Profiling Script
# Generates flame graphs for the bootstrap process.
# Requirements: perf, flamegraph (cargo install flamegraph)

echo "=== HLX COMPILER PROFILING ==="

if ! command -v perf &> /dev/null; then
    echo "Error: 'perf' is not installed. Install it (e.g., sudo pacman -S perf linux-tools)"
    exit 1
fi

if ! command -v flamegraph &> /dev/null; then
    echo "Warning: 'flamegraph' (cargo) not found. Recording raw perf data only."
    echo "Run: cargo install flamegraph"
fi

# Build release
echo "Building release binary..."
cargo build --release

# 1. Profile Rust Bootstrap (Stage 1)
echo "Profiling Stage 1 (Rust -> HLX)..."
# We wrap in 'perf record'
perf record -g -o perf-stage1.data -- ./target/release/hlx compile hlx_compiler/bootstrap/compiler.hlxc -o stage1.lcc
echo "Stage 1 data captured."

# 2. Profile Self-Hosted (Stage 2)
echo "Profiling Stage 2 (HLX -> HLX)..."
perf record -g -o perf-stage2.data -- ./target/release/hlx run stage1.lcc --output stage2.lcb
echo "Stage 2 data captured."

# Generate SVGs if possible
if command -v flamegraph &> /dev/null; then
    echo "Generating Flame Graphs..."
    # Note: flamegraph tool usually wraps execution, but we can convert perf data
    # We'll use the perf script | stackcollapse | flamegraph pipeline if available,
    # or just tell the user to use 'hotspot' or 'firefox profiler' on the data files.
    echo "Perf data files saved: perf-stage1.data, perf-stage2.data"
    echo "Open them with 'hotspot' or upload to https://profiler.firefox.com/"
else
    echo "Perf data saved. Install flamegraph tools to visualize."
fi

echo "Done."