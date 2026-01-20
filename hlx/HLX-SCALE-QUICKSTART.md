# HLX-Scale Quick Start (2 Minutes)

**HLX-Scale** enables parallel execution of deterministic code with automatic hash verification. 8 agents (threads) run your code in parallel, verify they all got the same result, and return it.

## The Simplest Example

```hlx
program parallel_hello {

@scale(size=8)
fn main() -> Int {
    let result = 100 + 200;
    barrier("done");  // All 8 agents must agree
    return result;
}

}
```

Run it:
```bash
cargo run --bin hlx -- run hello_parallel.hlx
```

Output:
```
[HLX-SCALE] Starting speculation with 8 agents
[HLX-SCALE][BARRIER] 'done': All 8 agents agree (hash: 7a3d2e1...)
[HLX-SCALE][CONSENSUS] All 8 agents agree
300
```

## What Just Happened?

1. **@scale(size=8)** → Fork 8 parallel agents
2. **Each agent** → Runs `100 + 200` independently
3. **barrier("done")** → Wait for all agents to reach this point
4. **Hash verification** → Compute BLAKE3 hash of state, all agents must match
5. **Return result** → If all agree, return the computed value

## Real Example: Expensive Computation

```hlx
program compute {

@scale(size=8)
fn main() -> Int {
    let step1 = fib(30);
    barrier("after_fib");         // Checkpoint: verify all computed same value

    let step2 = step1 * 2;
    barrier("after_multiply");    // Another checkpoint

    return step2;
}

fn fib(n: Int) -> Int {
    if n <= 1 { return 1; }
    return fib(n - 1) + fib(n - 2);
}

}
```

Each of the 8 agents computes `fib(30)` independently, then all verify they got the same answer before continuing.

## When to Use @scale

✅ **Good candidates:**
- Pure computations (no I/O side effects)
- Deterministic algorithms
- Expensive operations you want to verify in parallel

❌ **Bad candidates:**
- Small operations (overhead > benefit)
- Non-deterministic code (random, time, I/O)
- Code with side effects

## Phase 1B Limitations

- ❌ **Only `main()` can have @scale** (multi-function coming Phase 2)
- ❌ **Max 1024 agents** (configurable safety limit)
- ✅ **Determinism guaranteed** (A1 axiom preserved)
- ✅ **Automatic serial fallback** (if agents disagree)

## Debugging

### See what's happening:
```bash
RUST_LOG=1 cargo run --bin hlx -- run hello_parallel.hlx
```

Shows agent fork events, barrier syncs, and hash verification.

### Verify determinism:
```bash
./verify_hlx_scale.sh
```

Compares serial vs @scale execution to ensure identical results.

## Next Steps

- **[Full Documentation →](HLX-SCALE.md)** - Deep dive into barriers, cost model, roadmap
- **[Phase 1A Details →](HLX_SCALE_PHASE1A_COMPLETE.md)** - Architecture and implementation
- **[Demo Program →](demo_main_scale.hlx)** - Working example with 2 barriers

---

**Phase 1B Status:** ✅ Complete
**Axioms Preserved:** A1 ✅ A2 ✅ A3 ✅ A4 ✅
**Tests:** 11/11 passing
