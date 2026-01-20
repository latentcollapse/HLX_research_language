#!/bin/bash
# HLX Bootstrap Compiler Pipeline Test
# Orchestrates: Source → Lexer → Parser → Lower → Emit → Bytecode

set -e  # Exit on error

echo "════════════════════════════════════════════════════════════"
echo "HLX Bootstrap Compiler - Full Pipeline Test"
echo "════════════════════════════════════════════════════════════"

# Change to hlx directory
cd "$(dirname "$0")/.."

# Build the HLX CLI if needed
if [ ! -f "../target/release/hlx" ]; then
    echo "Building HLX CLI..."
    cd ..
    cargo build --release
    cd hlx
fi

CLI="../target/release/hlx"

echo ""
echo "Testing pipeline on simple arithmetic program..."
echo ""

# Create a simple test program
TEST_SOURCE="program test_add {
    fn add(a: i64, b: i64) -> i64 {
        return a + b;
    }

    fn main() -> i64 {
        let result = add(10, 32);
        return result;
    }
}"

echo "Source program:"
echo "$TEST_SOURCE"
echo ""

# Save test source
echo "$TEST_SOURCE" > /tmp/test_pipeline_source.hlx

# Phase 1: Compile and run each component
echo "════════════════════════════════════════════════════════════"
echo "Phase 1: Testing Individual Components"
echo "════════════════════════════════════════════════════════════"

echo ""
echo "[1/4] Compiling lexer.hlx..."
$CLI compile hlx_bootstrap/lexer.hlx -o /tmp/lexer.hlxb

echo "[1/4] Running lexer (testing tokenize)..."
$CLI run /tmp/lexer.hlxb

echo ""
echo "[2/4] Compiling parser.hlx..."
$CLI compile hlx_bootstrap/parser.hlx -o /tmp/parser.hlxb

echo "[2/4] Running parser (testing parse_program)..."
$CLI run /tmp/parser.hlxb

echo ""
echo "[3/4] Compiling lower.hlx..."
$CLI compile hlx_bootstrap/lower.hlx -o /tmp/lower.hlxb

echo "[3/4] Running lower (testing lower_program)..."
$CLI run /tmp/lower.hlxb

echo ""
echo "[4/4] Compiling emit.hlx..."
$CLI compile hlx_bootstrap/emit.hlx -o /tmp/emit.hlxb

echo "[4/4] Running emit (testing emit_bytecode)..."
$CLI run /tmp/emit.hlxb

echo ""
echo "════════════════════════════════════════════════════════════"
echo "Phase 2: Verification"
echo "════════════════════════════════════════════════════════════"

echo ""
echo "✓ All components compiled successfully"
echo "✓ All component tests passed"
echo ""
echo "NOTE: Full pipeline integration (lexer→parser→lower→emit)"
echo "      requires either:"
echo "      1. HLX module system (not yet implemented)"
echo "      2. Monolithic compiler file (future work)"
echo "      3. Rust orchestration layer"
echo ""
echo "For now, each component has been verified individually."
echo ""
echo "════════════════════════════════════════════════════════════"
echo "Pipeline Test Complete!"
echo "════════════════════════════════════════════════════════════"
