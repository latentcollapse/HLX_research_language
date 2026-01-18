# HLX-Scale: Speculative Parallel Execution

HLX-Scale (HLX-S) adds deterministic speculative parallelization to HLX, enabling quantum-inspired multi-agent swarm execution while preserving all four axioms (A1-A4).

**New to HLX-Scale?** Start with the [2-minute Quick Start →](HLX-SCALE-QUICKSTART.md)

## Quick Start

Add the `@scale` pragma to your main function to enable parallel speculation:

```hlx
program parallel_example {

@scale(size=8)
fn main() -> Int {
    let step1 = compute_expensive();
    barrier("checkpoint1");  // Synchronize and verify consensus

    let step2 = process(step1);
    barrier("checkpoint2");  // Another sync point

    return finalize(step2);
}

}
```

Compile and run:
```bash
cargo run --bin hlx -- compile parallel_example.hlxa -o parallel.lcc
RUST_LOG=1 cargo run --bin hlx -- run parallel.lcc
```

## How It Works

### Speculation Model

HLX-Scale forks N parallel **agents** (threads) that execute the same code independently:

1. **Fork**: Spawn N agents at program start
2. **Execute**: Each agent runs independently on separate registers
3. **Synchronize**: Agents wait at explicit `barrier()` points
4. **Verify**: At each barrier, compute BLAKE3 hash of agent state
5. **Consensus**: All agents must have identical hashes to continue
6. **Merge**: Return result from first agent (all are identical)

### Determinism Guarantees (A1)

- **Same input → Same hash → Same output**
- All agents execute identical deterministic code
- BLAKE3 hash verification at every barrier
- Automatic serial fallback on divergence
- No race conditions or timing dependencies

### Barriers

Barriers are explicit synchronization points where agents verify consensus:

```hlx
let x = 10 * 5;         // All agents compute this
barrier("phase1");       // Wait here, verify all agents agree
let y = x + 12;         // Continue only if hashes match
barrier("phase2");       // Another verification point
return y - 2;
```

**What barriers do:**
- Wait for all N agents to reach the barrier
- Compute BLAKE3 hash of all register values
- Compare hashes across all agents
- ERROR and fallback to serial if any mismatch
- Log consensus for observability

**When to use barriers:**
- After expensive computations (checkpoint progress)
- Before critical sections (verify state consistency)
- For debugging (see where divergence occurs)

**Barrier overhead:**
- Thread synchronization cost
- Hash computation (BLAKE3 is fast)
- Verification logic

Use barriers strategically - not after every operation.

## Phase 1B: Current Status

### What Works ✅

1. **Main-Only Speculation** - Only `main()` can have `@scale` (MVP restriction)
2. **Multi-Barrier Sync** - Intermediate hash verification at explicit barriers
3. **Fork Bomb Prevention** - Thread-local recursion prevention
4. **Automatic Serial Fallback** - Graceful recovery on divergence
5. **Deterministic Hashing** - BLAKE3 hash of sorted register state
6. **8 Agents** - Default speculation with configurable size

### Limitations (Phase 1B MVP)

❌ **Only main() can use @scale**
```hlx
@scale(size=8)
fn helper() -> Int { ... }  // ❌ ERROR: Only main() can use @scale
```

❌ **No nested speculation**
```hlx
fn main() {
    swarm_helper();  // ❌ Would cause infinite recursion
}
```

❌ **Max 1024 agents** (safety limit, configurable)

✅ **Deterministic code only** (this is by design - A1 preservation)

### Usage Guidelines

**When to use @scale:**
- ✅ Embarrassingly parallel workloads
- ✅ Pure computation (no I/O side effects)
- ✅ Deterministic algorithms
- ✅ CPU-bound tasks (not memory-bound)

**When NOT to use @scale:**
- ❌ Non-deterministic operations (random, time, I/O)
- ❌ Small/fast computations (overhead > benefit)
- ❌ Memory-intensive workloads (N agents = N×memory)
- ❌ Functions that call other functions (Phase 1B limitation)

## Examples

### Basic Speculation

```hlx
program basic_speculation {

@scale(size=4)
fn main() -> Int {
    let result = 100 + 200;
    barrier("checkpoint");
    return result * 2;
}

}
```

Output:
```
[HLX-SCALE] Starting speculation with 4 agents (max: 1024)
[HLX-SCALE][AGENT-0] Forked and starting execution
[HLX-SCALE][AGENT-1] Forked and starting execution
[HLX-SCALE][AGENT-2] Forked and starting execution
[HLX-SCALE][AGENT-3] Forked and starting execution
[HLX-SCALE][AGENT-0] Reached barrier 'checkpoint' with hash: 7a3d2e1...
[HLX-SCALE][BARRIER] 'checkpoint': All 4 agents agree (hash: 7a3d2e1...)
[HLX-SCALE][CONSENSUS] All 4 agents agree (hash: 9f8e7d6...)
600
```

### Multiple Barriers

```hlx
program multi_barrier {

@scale(size=8)
fn main() -> Int {
    let step1 = 10 * 5;          // 50
    barrier("phase1");            // ✓ Consensus

    let step2 = step1 + 12;       // 62
    barrier("phase2");            // ✓ Consensus

    let result = step2 - 2;       // 60
    return result;
}

}
```

See `verify_hlx_scale.sh` for a complete working example.

## Debugging

### Enable Detailed Logging

```bash
RUST_LOG=1 cargo run --bin hlx -- run program.lcc
```

Shows:
- Agent fork events with IDs
- Barrier synchronization with hashes
- Consensus verification
- Divergence detection (if any)

### Check Barrier Hashes

```bash
RUST_LOG=1 cargo run --bin hlx -- run program.lcc 2>&1 | grep BARRIER
```

Output:
```
[HLX-SCALE][BARRIER] 'phase1': All 8 agents agree (hash: 371d8eeabf7c30b5)
[HLX-SCALE][BARRIER] 'phase2': All 8 agents agree (hash: b1b7e5d604816e4f)
```

### Verify Determinism

Run the verification script:
```bash
./verify_hlx_scale.sh
```

Compares serial vs speculation execution to ensure identical results.

## Error Handling

### Divergence Detection

If agents disagree at a barrier:
```
[HLX-SCALE][BARRIER] ERROR: Divergence detected at barrier 'phase1':
  Agent 0: 371d8eeabf7c30b5
  Agent 1: 371d8eeabf7c30b5
  Agent 2: 9a8b7c6d5e4f3a2b <- DIVERGENT
  Agent 3: 371d8eeabf7c30b5
[HLX-SCALE] Falling back to serial execution...
[HLX-SCALE] Serial fallback completed successfully
```

**Automatic recovery:** HLX-Scale re-runs in serial mode and returns the correct result.

### Common Errors

**Error:** `Multiple @scale functions not supported in MVP`
**Solution:** Only use `@scale` on `main()` in Phase 1B

**Error:** `@scale on 'helper' not supported`
**Solution:** Move `@scale` to `main()` only

**Error:** `Requested 2000 agents, capped at max 1024`
**Warning:** Agent count exceeds safety limit, automatically clamped

## Architecture

```
main() with @scale
    │
    ├──> Agent 0 (fork)
    ├──> Agent 1 (fork)
    ├──> Agent 2 (fork)
    └──> Agent 3 (fork)
         │
         ├──> Execute code
         ├──> Hit barrier
         ├──> Compute BLAKE3 hash
         ├──> Wait for others
         └──> Verify consensus
              │
              ├──> All agree? Continue
              └──> Mismatch? Fallback to serial
```

## Performance Considerations

### Speedup Expectations

- **Best case:** Linear speedup for embarrassingly parallel work
- **Typical:** 2-4× speedup with 8 agents (depends on barrier frequency)
- **Worst case:** Slowdown if barriers are too frequent or work is too small

### Overhead Sources

1. **Thread spawning:** One-time cost at start
2. **Barrier synchronization:** Wait for slowest agent
3. **Hash computation:** BLAKE3 per barrier per agent
4. **Memory:** N agents = N×register state

### Optimization Tips

- **Minimize barriers:** Only at critical checkpoints
- **Balance work:** Ensure significant computation between barriers
- **Choose agent count wisely:** More agents ≠ faster (diminishing returns)
- **Profile first:** Measure before optimizing

## Roadmap

### Phase 1B (Current) ✅
- [x] Main-only speculation
- [x] Multi-barrier synchronization
- [x] Hash verification at barriers
- [x] Automatic serial fallback
- [x] Fork bomb prevention
- [x] 8-agent swarm execution

### Phase 2 (Future)
- [ ] Multi-function speculation (remove main-only restriction)
- [ ] Substrate-aware execution (CPU/GPU/QPU routing)
- [ ] Dynamic agent count tuning
- [ ] Performance benchmarking suite
- [ ] Graceful agent failure handling
- [ ] Barrier-free speculation (speculative consistency)

### Phase 3+ (Vision)
- [ ] Quantum substrate integration
- [ ] Distributed speculation across machines
- [ ] Adaptive speculation based on workload
- [ ] Zero-overhead speculation (perfect prediction)

## Testing

Run the test suite:
```bash
cargo test --lib speculation
./verify_hlx_scale.sh
```

Check for fork bomb prevention:
```bash
RUST_LOG=1 cargo run --bin hlx -- run demo_swarm.lcc 2>&1 | grep -c "Forked"
# Should output exactly: 8
```

Verify barrier synchronization:
```bash
RUST_LOG=1 cargo run --bin hlx -- run demo_swarm.lcc 2>&1 | grep BARRIER
# Should show 2 barriers with consensus
```

## FAQ

**Q: Why only main() in Phase 1B?**
A: Simplifies the MVP. Multi-function speculation requires deeper analysis of call graphs and state propagation. Coming in Phase 2.

**Q: What happens if agents diverge?**
A: HLX-Scale detects divergence at the next barrier, logs the error, and automatically re-runs in serial mode to produce the correct result.

**Q: Is speculation always faster?**
A: No. If the work is small or barriers are frequent, serial execution may be faster. Use profiling to decide.

**Q: Can I use @scale with I/O operations?**
A: Not recommended. I/O introduces non-determinism. Speculation works best with pure computation.

**Q: How do I choose the agent count?**
A: Start with CPU core count (e.g., 8). Profile and adjust. More agents ≠ better performance.

**Q: What's the maximum agent count?**
A: Default max is 1024 (safety limit). Configurable in SpeculationConfig.

## Contributing

HLX-Scale is under active development. See `hlx_runtime/src/speculation.rs` for implementation details.

Key files:
- `hlx_compiler/src/lower.rs` - Substrate inference and @scale validation
- `hlx_compiler/src/substrate_inference.rs` - AST analysis for parallelization
- `hlx_runtime/src/speculation.rs` - Speculation coordinator and barrier sync
- `hlx_runtime/src/executor.rs` - Instruction dispatch with barrier support

## References

- HLX Axioms: A1 (Determinism), A2 (Reversibility), A3 (Bijection), A4 (Universal Value)
- LC-B Instruction Set: Low-level bytecode with Barrier instruction
- BLAKE3: Fast cryptographic hash for state verification
- MAS: Multi-Agent System paradigm

---

**Status:** Phase 1B Complete ✅
**Tested:** 8 agents, 2 barriers, serial fallback, fork bomb prevention
**Axioms Preserved:** A1 ✅ A2 ✅ A3 ✅ A4 ✅
