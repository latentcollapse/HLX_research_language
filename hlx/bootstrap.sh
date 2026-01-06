#!/bin/bash
# HLX Ouroboros Bootstrap Script
# Phase 4: Self-hosting the HLX compiler

set -e

echo "=== HLX OUROBOROS BOOTSTRAP ==="
echo "Phase 4: Self-hosting the HLX compiler written in HLX"
echo ""

# Build the Rust bootstrap compiler
echo "Step 1: Building Rust bootstrap compiler..."
cargo build --release --bin hlx > /dev/null 2>&1
RUST_COMPILER="./target/release/hlx"
echo "✓ Rust compiler ready"

# Stage 1: Rust -> HLX (A)
echo ""
echo "Step 2: Compiling Self-Hosted Compiler (Stage 1)..."
SOURCE="hlx_compiler/bootstrap/compiler.hlxc"
STAGE1_SRC="/tmp/stage1_wrapped.hlxa"
STAGE1_BIN="stage1.lcc"

# Wrap the flat compiler file in a program block for the Rust compiler
echo "program hlx_compiler {" > "$STAGE1_SRC"
cat "$SOURCE" >> "$STAGE1_SRC"
echo "}" >> "$STAGE1_SRC"

$RUST_COMPILER compile "$STAGE1_SRC" -o "$STAGE1_BIN"
echo "✓ Stage 1 built: $STAGE1_BIN"

# Stage 2: Stage 1 -> HLX (B)
echo ""
echo "Step 3: Self-Compilation (Stage 2) - THE OUROBOROS! 🐍"
STAGE2_IR="stage2.lcb"
STAGE2_BIN="stage2.lcc"

echo "  (Running Stage 1 to compile its own source...)"
$RUST_COMPILER run "$STAGE1_BIN" --main-input "$SOURCE" -o "$STAGE2_IR"
$RUST_COMPILER build-crate "$STAGE2_IR" -o "$STAGE2_BIN"
echo "✓ Stage 2 built: $STAGE2_BIN"

# Stage 3: Stage 2 -> HLX (C)
echo ""
echo "Step 4: Verifying Reproducibility (Stage 3)..."
STAGE3_IR="stage3.lcb"
STAGE3_BIN="stage3.lcc"

echo "  (Running Stage 2 to compile its own source...)"
$RUST_COMPILER run "$STAGE2_BIN" --main-input "$SOURCE" -o "$STAGE3_IR"
$RUST_COMPILER build-crate "$STAGE3_IR" -o "$STAGE3_BIN"
echo "✓ Stage 3 built: $STAGE3_BIN"

# Final Hash Comparison
echo ""
echo "Step 5: Checking Determinism..."
HASH2=$($RUST_COMPILER inspect "$STAGE2_BIN" | grep "Hash:" | awk '{print $2}')
HASH3=$($RUST_COMPILER inspect "$STAGE3_BIN" | grep "Hash:" | awk '{print $2}')

if [ "$HASH2" == "$HASH3" ]; then
    echo "✓✓✓ OUROBOROS COMPLETE! ✓✓✓"
    echo "Hash: $HASH2"
    echo ""
    echo "The HLX compiler is now fully self-hosted."
else
    echo "❌ HASH MISMATCH"
    echo "Stage 2: $HASH2"
    echo "Stage 3: $HASH3"
    exit 1
fi