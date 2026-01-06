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