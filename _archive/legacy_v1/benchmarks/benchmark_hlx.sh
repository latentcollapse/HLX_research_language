#!/bin/bash
# HLX Vulkan Benchmark Suite
# Quick performance comparison tests

set -e

echo "╔══════════════════════════════════════════════════════╗"
echo "║     HLX vs CUDA Benchmark Suite                      ║"
echo "╚══════════════════════════════════════════════════════╝"
echo ""

# Create results directory
mkdir -p benchmarks/results
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
RESULTS_DIR="benchmarks/results/$TIMESTAMP"
mkdir -p "$RESULTS_DIR"

echo "Results will be saved to: $RESULTS_DIR"
echo ""

# Test 1: Training throughput (20 epochs)
echo "=== Test 1: Training Throughput (20 epochs) ==="
echo "Running HLX Vulkan training..."
./target/release/train_transformer_full 2>&1 | tee "$RESULTS_DIR/hlx_training.log" | grep -E "Epoch|tok/s|Best loss"

echo ""
echo "=== Benchmark Complete! ==="
echo ""
echo "Results saved to: $RESULTS_DIR"
echo ""
echo "Summary files:"
ls -lh "$RESULTS_DIR"
