# HLX Roadmap

This document outlines HLX's development phases and current status. Our philosophy: **under-promise, over-deliver.**

## Phase Summary

| Phase | Status | Timeline | Focus |
|-------|--------|----------|-------|
| **Phase 1** | ✅ Complete | 2025 | Core language, deterministic execution, LSP |
| **Phase 1B** | ✅ Complete | 2025 | HLX-Scale (parallel execution), CodeGen |
| **Phase 2** | 🔜 Next | 2026 | Multi-function speculation, adaptive tuning, medical/automotive CodeGen |
| **Phase 3+** | 🔮 Future | 2026-2027 | Quantum integration, distributed scaling, advanced features |

---

## Phase 1: Foundation (✅ Complete)

### Core Language & Compiler
- ✅ Parser (ASCII + Runic syntax)
- ✅ Type inference and checking
- ✅ Deterministic compilation pipeline
- ✅ Self-hosting compiler
- ✅ 128/128 tests passing on all platforms

### Runtime
- ✅ CPU executor (LLVM backend)
- ✅ GPU runtime (Vulkan)
- ✅ Determinism guarantees (A1 axiom)
- ✅ Automatic CPU/GPU fallback
- ✅ 10-100x GPU speedup for image/tensor ops

### Developer Tooling
- ✅ Language Server Protocol (95%+ feature parity)
- ✅ VS Code extension
- ✅ CI/CD (Linux, macOS, Windows)
- ✅ FFI bindings (C, Python, Node.js, Rust, Java, Ada/SPARK)

### Operations
- ✅ Image processing (8 GPU-accelerated ops)
- ✅ Tensor operations (creation, manipulation, reductions)
- ✅ File I/O (JSON, CSV, images, raw files)
- ✅ Math operations (full suite)

### Status
**Production Ready** — The language is stable, compiler is self-hosting, IDE rivals Rust/Python in feature completeness.

---

## Phase 1B: Parallel Execution & Enterprise Code Gen (✅ Complete)

### HLX-Scale (Speculative Parallelization)

**What's Implemented:**
- ✅ **@scale pragma** - Enable parallel speculation on main()
- ✅ **Multi-agent execution** - Fork N independent agents (default 8)
- ✅ **Barrier synchronization** - Explicit sync points with hash verification
- ✅ **BLAKE3 hash verification** - All agents must agree at each barrier
- ✅ **Automatic serial fallback** - If agents diverge, re-run serially for correctness
- ✅ **Fork bomb prevention** - Thread-local state prevents infinite recursion
- ✅ **Axiom preservation** - A1 (determinism), A2 (partial), A3, A4 all preserved
- ✅ **Comprehensive testing** - 11 test cases, all passing
- ✅ **Demo programs** - Working examples with 2 barriers
- ✅ **Verification script** - Compare serial vs @scale results

**Limitations (by design for MVP):**
- ❌ Only main() can use @scale (multi-function coming Phase 2)
- ❌ Max 1024 agents (safety limit, configurable)
- ❌ No nested speculation (prevents fork bombs)

**Documentation:**
- ✅ [HLX-SCALE.md](hlx/HLX-SCALE.md) - Full technical documentation
- ✅ [HLX-SCALE-QUICKSTART.md](hlx/HLX-SCALE-QUICKSTART.md) - 2-minute introduction
- ✅ [HLX_SCALE_PHASE1A_COMPLETE.md](hlx/HLX_SCALE_PHASE1A_COMPLETE.md) - Implementation details

### HLX CodeGen (Enterprise)

**Aerospace (DO-178C) - Production Ready:**
- ✅ Generate 557+ lines of certified-ready code in 3 minutes
- ✅ Triple Modular Redundancy (TMR)
- ✅ Safety analysis documentation
- ✅ Test procedures
- ✅ Formal verification ready

**Roadmap:**
- 🔜 Medical (IEC 62304) - Q1 2026
- 🔜 Automotive (ISO 26262) - Q2 2026

### Status
**Production Ready** — HLX-Scale is fully implemented with comprehensive tests and documentation. Multi-agent execution, barrier synchronization, and determinism guarantees are all verified.

---

## Phase 2: Multi-Function Parallelism & Adaptive Tuning (🔜 Next)

### HLX-Scale Enhancements

#### Multi-Function Speculation
- [ ] Remove main-only restriction
- [ ] Call graph analysis for safety
- [ ] Speculate across function boundaries
- [ ] Nested speculation handling
- [ ] Tests: 20+ integration tests

**Timeline:** Q1 2026
**Complexity:** Medium (requires call graph analysis, state propagation)

#### Substrate-Aware Execution
- [ ] CPU/GPU substrate routing based on operation type
- [ ] Heuristic: large tensor ops → GPU, small ops → CPU
- [ ] GPU speculation (multiple GPU threads)
- [ ] Mixed CPU/GPU execution
- [ ] Hardware-specific optimizations

**Timeline:** Q2 2026
**Complexity:** High (requires GPU thread management, memory pooling)

#### Dynamic Agent Count Tuning
- [ ] Profile workload characteristics
- [ ] Adaptive agent count selection
- [ ] Auto-disable speculation for small ops (overhead > benefit)
- [ ] Performance benchmarking suite
- [ ] Cost model refinement

**Timeline:** Q2 2026
**Complexity:** Medium (requires profiling infrastructure, cost model)

#### Graceful Failure Handling
- [ ] Agent timeout detection
- [ ] Partial success recovery (N-1 agents succeed)
- [ ] Retry with reduced agent count
- [ ] Detailed error diagnostics

**Timeline:** Q1 2026
**Complexity:** Low (enhancement to existing fallback)

### Contracts (Expand Alpha)

- [ ] New validation rules (date formats, regex, custom validators)
- [ ] Contract composition (combine multiple rules)
- [ ] Custom contract types
- [ ] Integration with formal verification tools

**Timeline:** Q1-Q2 2026
**Complexity:** Medium

### LSTX (Latent Space) - Expand Experimental

- [ ] Vector database integration (ChromaDB, Pinecone, Weaviate)
- [ ] Semantic search primitives
- [ ] Embedding operations (@lstx for vector queries)
- [ ] RAG (Retrieval-Augmented Generation) support

**Timeline:** Q2-Q3 2026
**Complexity:** High (requires external integrations)

### CodeGen - Additional Domains

- [ ] Medical (IEC 62304 DAL-A)
- [ ] Automotive (ISO 26262 ASIL-B)
- [ ] Railway (EN 50128)

**Timeline:** Q1-Q3 2026
**Complexity:** High per domain

### Status
**Under Planning** — Ready to begin Phase 2 work. Will maintain Phase 1B stability while developing Phase 2 features on separate branches.

---

## Phase 3+: Quantum & Advanced Features (🔮 Future)

### Quantum Substrate Integration

- [ ] Quantum hardware detection (IBM Qiskit, IonQ, Rigetti)
- [ ] QPU (Quantum Processing Unit) substrate
- [ ] Quantum-classical hybrid execution
- [ ] Distributed barrier protocol for quantum agents
- [ ] Quantum error correction awareness

**Timeline:** Q3 2026 - Q2 2027
**Complexity:** Very High (requires quantum hardware expertise, new axioms)

### Distributed Speculation Across Machines

- [ ] Network barrier protocol (distributed consensus)
- [ ] Multi-machine agent coordination
- [ ] 10k+ agent swarm execution
- [ ] Network failure recovery
- [ ] Latency-aware barrier optimization

**Timeline:** Q4 2026 - Q3 2027
**Complexity:** Very High (distributed systems challenges)

### Advanced Optimizations

- [ ] Speculative consistency (barrier-free speculation with eventual consistency)
- [ ] Zero-overhead speculation (perfect workload prediction)
- [ ] ML-based agent count prediction
- [ ] Compiler optimizations that enable speculation

**Timeline:** Ongoing
**Complexity:** Very High (requires ML infrastructure, advanced analysis)

---

## Feature Status by Area

### Language Core

| Feature | Phase | Status | Notes |
|---------|-------|--------|-------|
| Parser (ASCII/Runic) | 1 | ✅ Complete | Both syntaxes fully supported |
| Type system | 1 | ✅ Complete | Full inference, no null pointers |
| Control flow | 1 | ✅ Complete | if/else, loops, functions |
| Pattern matching | 2 | 🔜 Planned | Match expressions coming |
| Generics | 3 | 🔮 Future | Generic functions/types |

### Runtime & Execution

| Feature | Phase | Status | Notes |
|---------|-------|--------|-------|
| CPU execution | 1 | ✅ Complete | LLVM backend, deterministic |
| GPU execution (Vulkan) | 1 | ✅ Complete | All major vendors supported |
| HLX-Scale (main-only) | 1B | ✅ Complete | 8 agents, 2+ barriers |
| Multi-function @scale | 2 | 🔜 Planned | Q1 2026 |
| Quantum execution | 3 | 🔮 Future | Hardware integration |
| Distributed execution | 3 | 🔮 Future | 10k+ agent swarms |

### Standard Library

| Feature | Phase | Status | Notes |
|---------|-------|--------|-------|
| Tensor operations | 1 | ✅ Complete | Create, reshape, reduce |
| Image processing | 1 | ✅ Complete | 8 GPU-accelerated ops |
| File I/O | 1 | ✅ Complete | JSON, CSV, images, raw |
| Math functions | 1 | ✅ Complete | Full set |
| String operations | 1 | ✅ Complete | Parse, format, search |
| Contracts | 2 | 🔜 Alpha → Beta | Expanding validation rules |
| LSTX (latent space) | 2 | 🔜 Experimental | Vector ops coming |
| Random number generation | 2 | 🔜 Planned | Deterministic RNG |

### Developer Tools

| Feature | Phase | Status | Notes |
|---------|-------|--------|-------|
| LSP | 1 | ✅ Complete | 95%+ Rust/Python parity |
| VS Code extension | 1 | ✅ Complete | Full integration |
| Debugger | 2 | 🔜 Planned | Step-through, breakpoints |
| Profiler | 2 | 🔜 Planned | Flame graphs, cost analysis |
| Formatter | 1 | ✅ Complete | Code style enforcement |
| Linter | 1 | ✅ Complete | Basic style checks |
| FFI bindings | 1 | ✅ Complete | C, Python, Node, Rust, Java, Ada/SPARK |

### Enterprise Features

| Feature | Phase | Status | Notes |
|---------|-------|--------|-------|
| CodeGen (Aerospace) | 1B | ✅ Complete | DO-178C DAL-A, production ready |
| CodeGen (Medical) | 2 | 🔜 Planned | IEC 62304, Q1 2026 |
| CodeGen (Automotive) | 2 | 🔜 Planned | ISO 26262, Q2 2026 |
| Formal verification | 3 | 🔮 Future | Rocq/Coq integration |
| Safety analysis | 1B | ✅ Complete | In CodeGen output |

---

## Axiom Preservation Status

HLX is built on four axioms (A1-A4) that define its correctness guarantees.

### A1: Determinism (Same input → Same output)

| Component | Status | Notes |
|-----------|--------|-------|
| Compiler | ✅ Complete | AST-hash based decisions |
| CPU runtime | ✅ Complete | LLVM generates deterministic code |
| GPU runtime (Vulkan) | ✅ Complete | Deterministic floating-point |
| HLX-Scale (Phase 1B) | ✅ Complete | BLAKE3 hash verification |
| Multi-function @scale | 🔜 Planned | Will maintain A1 |
| Quantum execution | 🔮 Future | New axiom extensions needed |

### A2: Reversibility (State snapshots, recovery)

| Component | Status | Notes |
|-----------|--------|-------|
| Architecture | ✅ Complete | Supports snapshots |
| Error recovery | ✅ Partial | Serial fallback implemented |
| Explicit rollback | 🔜 Phase 2 | Detailed mechanism needed |
| Distributed rollback | 🔮 Phase 3 | Network consistency needed |

### A3: Bijection (Results map bijectively to inputs)

| Component | Status | Notes |
|-----------|--------|-------|
| Value type | ✅ Complete | Canonical representation |
| Serialization | ✅ Complete | Lossless encoding |
| HLX-Scale | ✅ Complete | Agent states isolated |

### A4: Universal Value (All agents use same Value type)

| Component | Status | Notes |
|-----------|--------|-------|
| Runtime | ✅ Complete | Single Value enum type |
| CPU/GPU | ✅ Complete | Same representation |
| HLX-Scale | ✅ Complete | All agents same Value |
| Distributed (future) | 🔮 Planned | Network encoding needed |

---

## Current Priorities

### Q4 2025 (Now)
1. ✅ Phase 1B completion (HLX-Scale + aerospace CodeGen)
2. ✅ Documentation and discoverability
3. ✅ Real-world testing and feedback

### Q1 2026
1. 🔜 Multi-function speculation (Phase 2 foundation)
2. 🔜 Medical CodeGen (IEC 62304)
3. 🔜 Performance benchmarking suite
4. 🔜 LSP enhancements (debugging, profiling)

### Q2 2026
1. 🔜 Substrate-aware execution
2. 🔜 Automotive CodeGen (ISO 26262)
3. 🔜 Cost model refinement
4. 🔜 Contract expansion (Beta)

### Q3 2026 - Q1 2027
1. 🔜 LSTX integration (vector databases)
2. 🔜 Quantum exploration (Phase 3 foundation)
3. 🔜 Advanced optimizations

---

## How to Track Progress

- **GitHub Issues** - Feature tracking and bug reports
- **GitHub Projects** - Organized by phase/area
- **This file** - Updated at end of each quarter
- **GitHub Discussions** - Community input and feedback

## Philosophy

We follow these principles:

1. **Ship working code** - No vaporware. Everything in this roadmap is either complete or has a clear implementation plan.
2. **Under-promise, over-deliver** - If we ship early, that's a win. Features slip left on the roadmap, not right.
3. **Maintain stability** - Phase 1B is locked. Phase 2+ work happens on branches until ready.
4. **Preserve axioms** - Every change must maintain A1-A4 guarantees.
5. **Listen to users** - Feedback can shift priorities. This roadmap is a guide, not law.

---

## Contributing

Want to help? See [CONTRIBUTING.md](CONTRIBUTING.md) for details on how to contribute to any phase.

---

**Last updated:** January 2026
**Next update:** April 2026 (end of Q1)

Questions? Open an issue or discussion on GitHub.
