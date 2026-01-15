# HLX Compiler Performance Benchmarks

**System:** Linux (Arch)
**Date:** 2026-01-06
**Compiler Version:** v1.0 (Ouroboros)
**Source:** `hlx_compiler/bootstrap/compiler.hlxc` (926 lines)

## Compilation Performance

| Stage | Compiler Host | Time (ms) | Speedup vs Host | Notes |
|-------|--------------|-----------|-----------------|-------|
| **Stage 1** | Native Rust (`hlx`) | **16ms** | 1x (Baseline) | Blazing fast. |
| **Stage 2** | HLX VM (`stage1.lcc`) | **535ms** | 0.03x | ~33x overhead for interpreted VM. |
| **Stage 3** | HLX VM (`stage2.lcc`) | **491ms** | 0.03x | Self-hosting verification. |

**Throughput (Self-Hosted):** ~1,729 lines/sec on VM.

## Determinism Verification

- **Stage 2 Hash:** `98ce9ac411b488b4ecc32f35a35e7995c68d1ca5910f3aec368af213f8184e03`
- **Stage 3 Hash:** `98ce9ac411b488b4ecc32f35a35e7995c68d1ca5910f3aec368af213f8184e03`
- **Result:** **Bitwise Identical.**

## Profiling (Flame Graphs)

We have verified the performance characteristics using `perf` and flame graphs.

- **Stage 1 (Rust):** dominated by `native_tokenize` and parsing.
- **Stage 2/3 (Self-Hosted):** dominated by `execute_instruction` (the VM interpreter loop). Specifically `Index` and `Store` operations on immutable data structures (`im::OrdMap`) are the hotspots, as expected.

## Conclusion

The 33x overhead for a purely interpreted, self-hosted compiler running on immutable data structures is an excellent starting point. It is significantly faster than typical Python/Ruby bootstraps (often 50-100x slowdown).

Future work on a JIT backend (LLVM/Cranelift) targets a 2-5x overhead.

---

## Benchmarking with `hlx bench`

The HLX CLI provides a built-in benchmarking command for performance analysis with statistical metrics and optional flamegraph generation.

### Basic Usage

```bash
# Benchmark a compiled crate
hlx bench demo_scale.lcc -n 10

# Benchmark with JSON output (for CI/CD)
hlx bench demo_scale.lcc -n 100 --json

# Enable HLX-Scale speculation
hlx bench demo_scale.lcc -n 10 --hlx_s
```

### Flamegraph Generation

Generate CPU flamegraphs with detailed labeling:

```bash
# Generate flamegraph with default 1000 Hz sampling
hlx bench demo_scale.lcc -n 10 --flamegraph

# Custom sampling frequency
hlx bench demo_scale.lcc -n 10 --flamegraph --frequency 5000
```

**Linux Setup Required:**

Flamegraph generation requires perf event access. On Linux, set:

```bash
sudo sysctl -w kernel.perf_event_paranoid=-1
```

**Flamegraph Output:**

Flamegraphs are saved to `perf_data/` with detailed naming:
- Format: `flamegraph-{test}-{mode}{scale_info}_n{iterations}_mean{ms}_{timestamp}.svg`
- Example: `flamegraph-demo_scale-serial_n10_mean1154ms_20260115_143052.svg`

Titles include comprehensive metadata:
- Test name and execution mode (serial/speculation)
- Iteration count and mean execution time
- @scale pragma info (agent count, barrier count) if present

### Statistics Output

The benchmark command provides comprehensive statistics:
- **Mean:** Average execution time
- **Median:** Middle value of sorted execution times
- **Min/Max:** Fastest and slowest runs
- **StdDev:** Standard deviation (measure of consistency)
- **Total:** Total time across all iterations

Per-run timing is displayed for ≤20 iterations.

### Tracing and Logging

Enable structured logging to observe HLX-Scale speculation behavior:

```bash
# View info-level logs
RUST_LOG=info hlx bench demo_scale.lcc -n 5

# Debug-level for agent/barrier details
RUST_LOG=debug hlx bench demo_scale.lcc -n 5 --hlx_s
```

**Log Categories:**
- `HLX-SCALE:` Main speculation coordinator messages
- `HLX-SCALE/AGENT:` Individual agent lifecycle events
- `HLX-SCALE/BARRIER:` Barrier synchronization and consensus verification