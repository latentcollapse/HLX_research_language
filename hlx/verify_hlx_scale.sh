#!/bin/bash
# HLX-Scale Verification Script
# Compares serial vs speculation execution to verify determinism (A1)

set -e

echo "=== HLX-Scale Verification ==="
echo

# Compile both versions
echo "1. Compiling serial version..."
cargo run --quiet --bin hlx -- compile demo_serial.hlxa -o demo_serial.lcc 2>&1 | grep "Compiled"

echo "2. Compiling @swarm version..."
cargo run --quiet --bin hlx -- compile demo_main_swarm.hlxa -o demo_swarm.lcc 2>&1 | grep "Compiled"

echo
echo "3. Running serial execution..."
SERIAL_RESULT=$(cargo run --quiet --bin hlx -- run demo_serial.lcc 2>&1 | tail -1)
echo "   Result: $SERIAL_RESULT"

echo
echo "4. Running @swarm(size=8) execution..."
RUST_LOG=1 cargo run --quiet --bin hlx -- run demo_swarm.lcc 2>&1 > /tmp/swarm_output.txt
SWARM_RESULT=$(tail -1 /tmp/swarm_output.txt)
echo "   Result: $SWARM_RESULT"

echo
echo "5. Checking consensus..."
CONSENSUS=$(grep "\[CONSENSUS\]" /tmp/swarm_output.txt | tail -1)
echo "   $CONSENSUS"

echo
echo "6. Verification:"
if [ "$SERIAL_RESULT" = "$SWARM_RESULT" ]; then
    echo "   ✅ Serial and speculation results MATCH"
    echo "   ✅ A1 (Determinism) PRESERVED"
else
    echo "   ❌ Results MISMATCH!"
    echo "   Serial: $SERIAL_RESULT"
    echo "   Swarm:  $SWARM_RESULT"
    exit 1
fi

echo
echo "7. Swarm details:"
grep "\[HLX-SCALE\] Starting speculation" /tmp/swarm_output.txt | head -1
AGENT_COUNT=$(grep "\[HLX-SCALE\]\[AGENT-" /tmp/swarm_output.txt | grep "Forked" | wc -l)
echo "   Agents forked: $AGENT_COUNT"

HASH=$(grep "State hash:" /tmp/swarm_output.txt | head -1 | awk '{print $NF}')
echo "   Common hash: $HASH"

echo
echo "=== Verification Complete ==="
echo "✅ HLX-Scale Phase 1B: WORKING"
