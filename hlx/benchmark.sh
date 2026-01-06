#!/bin/bash
# HLX Compiler Performance Benchmark
# Measures compilation times for the three-stage bootstrap

set -e

echo "=== HLX COMPILER BENCHMARK ==="
echo "Testing on: $(uname -s) $(uname -m)"
echo "Date: $(date)"
echo ""

RUST_COMPILER="./target/release/hlx"
SOURCE="hlx_compiler/bootstrap/compiler.hlxc"

# Build Rust compiler first (not timed - setup)
echo "Building Rust bootstrap compiler..."
cargo build --release --bin hlx > /dev/null 2>&1
echo "✓ Rust compiler ready"
echo ""

# Clean previous artifacts
rm -f stage1.lcc stage2.lcb stage2.lcc stage3.lcb stage3.lcc /tmp/stage1_wrapped.hlxa

# ============================================
# STAGE 1: Rust → HLX Compiler (Stage 1)
# ============================================
echo "--- Stage 1: Rust Compiling Self-Hosted Compiler ---"
echo "program hlx_compiler {" > /tmp/stage1_wrapped.hlxa
cat "$SOURCE" >> /tmp/stage1_wrapped.hlxa
echo "}" >> /tmp/stage1_wrapped.hlxa

START=$(date +%s%N)
$RUST_COMPILER compile /tmp/stage1_wrapped.hlxa -o stage1.lcc > /dev/null 2>&1
END=$(date +%s%N)

STAGE1_TIME=$(awk "BEGIN {printf \"%.3f\", ($END - $START) / 1000000000}")
STAGE1_SIZE=$(wc -c < stage1.lcc)
STAGE1_HASH=$($RUST_COMPILER inspect stage1.lcc 2>/dev/null | grep "Hash:" | awk '{print $2}')

echo "  Time: ${STAGE1_TIME}s"
echo "  Size: ${STAGE1_SIZE} bytes"
echo "  Hash: ${STAGE1_HASH}"
echo ""

# ============================================
# STAGE 2: Stage 1 → HLX Compiler (Stage 2)
# ============================================
echo "--- Stage 2: Self-Hosted Compiler (First Self-Compilation) ---"

START=$(date +%s%N)
$RUST_COMPILER run stage1.lcc --main-input "$SOURCE" -o stage2.lcb > /dev/null 2>&1
$RUST_COMPILER build-crate stage2.lcb -o stage2.lcc > /dev/null 2>&1
END=$(date +%s%N)

STAGE2_TIME=$(awk "BEGIN {printf \"%.3f\", ($END - $START) / 1000000000}")
STAGE2_SIZE=$(wc -c < stage2.lcc)
STAGE2_HASH=$($RUST_COMPILER inspect stage2.lcc 2>/dev/null | grep "Hash:" | awk '{print $2}')

echo "  Time: ${STAGE2_TIME}s"
echo "  Size: ${STAGE2_SIZE} bytes"
echo "  Hash: ${STAGE2_HASH}"
echo ""

# ============================================
# STAGE 3: Stage 2 → HLX Compiler (Stage 3)
# ============================================
echo "--- Stage 3: Self-Hosted Compiler (Second Self-Compilation) ---"

START=$(date +%s%N)
$RUST_COMPILER run stage2.lcc --main-input "$SOURCE" -o stage3.lcb > /dev/null 2>&1
$RUST_COMPILER build-crate stage3.lcb -o stage3.lcc > /dev/null 2>&1
END=$(date +%s%N)

STAGE3_TIME=$(awk "BEGIN {printf \"%.3f\", ($END - $START) / 1000000000}")
STAGE3_SIZE=$(wc -c < stage3.lcc)
STAGE3_HASH=$($RUST_COMPILER inspect stage3.lcc 2>/dev/null | grep "Hash:" | awk '{print $2}')

echo "  Time: ${STAGE3_TIME}s"
echo "  Size: ${STAGE3_SIZE} bytes"
echo "  Hash: ${STAGE3_HASH}"
echo ""

# ============================================
# VERIFICATION
# ============================================
echo "--- Verification ---"

if [ "$STAGE2_HASH" == "$STAGE3_HASH" ]; then
    echo "✓ Determinism: Stage 2 == Stage 3 (bytewise identical)"
else
    echo "✗ FAILED: Stage 2 ≠ Stage 3"
    exit 1
fi

# Full SHA256 verification
STAGE2_SHA256=$(sha256sum stage2.lcc | awk '{print $1}')
STAGE3_SHA256=$(sha256sum stage3.lcc | awk '{print $1}')

echo "  Stage 2 SHA256: ${STAGE2_SHA256}"
echo "  Stage 3 SHA256: ${STAGE3_SHA256}"

if [ "$STAGE2_SHA256" == "$STAGE3_SHA256" ]; then
    echo "✓ Cryptographic verification passed"
else
    echo "✗ SHA256 mismatch!"
    exit 1
fi

echo ""

# ============================================
# PERFORMANCE ANALYSIS
# ============================================
echo "=== PERFORMANCE SUMMARY ==="
echo ""

# VM overhead (Stage 2 vs Stage 1)
VM_OVERHEAD=$(awk "BEGIN {printf \"%.2f\", ($STAGE2_TIME / $STAGE1_TIME)}")
echo "Compilation Times:"
echo "  Stage 1 (Rust):        ${STAGE1_TIME}s"
echo "  Stage 2 (Self-hosted): ${STAGE2_TIME}s"
echo "  Stage 3 (Self-hosted): ${STAGE3_TIME}s"
echo ""
echo "VM Overhead: ${VM_OVERHEAD}x (self-hosted vs native Rust)"
echo ""

# Compile speed (source lines per second)
SOURCE_LINES=$(wc -l < "$SOURCE")
STAGE1_LPS=$(awk "BEGIN {printf \"%.0f\", $SOURCE_LINES / $STAGE1_TIME}")
STAGE2_LPS=$(awk "BEGIN {printf \"%.0f\", $SOURCE_LINES / $STAGE2_TIME}")

echo "Throughput:"
echo "  Source: ${SOURCE_LINES} lines"
echo "  Stage 1: ${STAGE1_LPS} lines/sec"
echo "  Stage 2: ${STAGE2_LPS} lines/sec"
echo ""

# Size comparison
SIZE_REDUCTION=$(awk "BEGIN {printf \"%.1f\", 100 * (1 - $STAGE2_SIZE / $STAGE1_SIZE)}")
echo "Binary Size:"
echo "  Stage 1: ${STAGE1_SIZE} bytes"
echo "  Stage 2: ${STAGE2_SIZE} bytes (${SIZE_REDUCTION}% smaller)"
echo ""

echo "=== BENCHMARK COMPLETE ==="
echo ""
echo "Reproducible Hash: ${STAGE2_SHA256}"
