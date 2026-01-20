# Contributing to HLX

We welcome contributions to HLX! Whether you're fixing bugs, adding features, or improving documentation, here's how to get started.

## Code of Conduct

Be respectful, constructive, and focus on the technical merits of ideas. We're building a language for AI and safety-critical systems — high standards and rigorous discussion help us achieve that.

## Getting Started

### Prerequisites

```bash
rustc --version  # 1.70+
cargo --version
git clone https://github.com/latentcollapse/hlx-compiler.git
cd hlx-compiler/hlx
cargo build --release
```

### Running Tests

```bash
# Full test suite
cargo test

# Run only unit tests
cargo test --lib

# Run integration tests
cargo test --test '*'

# Run with logging
RUST_LOG=1 cargo test

# Specific test
cargo test test_agent_state_hash
```

### Building & Running

```bash
# Build the compiler
cargo build --release

# Run an HLX program
./target/release/hlx run program.hlx

# Compile to bytecode
./target/release/hlx compile program.hlx -o program.lcc
```

## Project Structure

```
hlx/
├── hlx_compiler/          # Main compiler (parser, type checker, lowering)
│   ├── src/
│   │   ├── ast.rs         # Abstract Syntax Tree definitions
│   │   ├── hlxa.rs        # ASCII parser (main syntax)
│   │   ├── runic.rs       # Runic syntax parser
│   │   ├── lower.rs       # Lowering to LC-B bytecode + substrate inference
│   │   ├── substrate.rs   # Substrate detection and configuration
│   │   ├── substrate_inference.rs  # AST analysis for parallelization
│   │   └── ...
│   └── Cargo.toml
├── hlx_runtime/           # Runtime executor (CPU/GPU)
│   ├── src/
│   │   ├── lib.rs         # Thread-local state management
│   │   ├── executor.rs    # Instruction execution
│   │   ├── speculation.rs # HLX-Scale parallel coordinator
│   │   ├── vulkan.rs      # Vulkan (GPU) backend
│   │   └── ...
│   └── Cargo.toml
├── hlx_core/              # Core types and instruction set
│   ├── src/
│   │   ├── instruction.rs # LC-B bytecode instruction definitions
│   │   ├── value.rs       # Value type (Int, Float, String, etc.)
│   │   └── ...
│   └── Cargo.toml
├── HLX-SCALE.md           # Full HLX-Scale documentation
├── HLX-SCALE-QUICKSTART.md # 2-minute quick start
└── ...
```

## Key Components

### Compiler Pipeline

1. **Parser** (`hlxa.rs`, `runic.rs`) → AST
2. **Type Checker** → Infer/verify types
3. **Lowering** (`lower.rs`) → LC-B bytecode
4. **Substrate Inference** (`substrate_inference.rs`) → Detect parallelism opportunities
5. **Output** → Bytecode file (`.lcc`)

### Runtime Pipeline

1. **Load bytecode** → Deserialize LC-B instructions
2. **Execute** (`executor.rs`) → Dispatch instructions
3. **Detect @scale** → Check for speculation metadata
4. **Route to coordinator** → `SpeculationCoordinator` if parallelizable
5. **Barriers** → Sync agents, verify consensus, fallback if needed

## Contributing Areas

### 1. HLX-Scale Development

**Current Phase:** Phase 1B (Multi-agent execution with barrier sync)

**Key Files:**
- `hlx_compiler/src/substrate_inference.rs` - AST analysis for parallelization
- `hlx_runtime/src/speculation.rs` - Agent coordination and barrier verification
- `hlx_compiler/src/lower.rs` - Integration with compiler pipeline

**Phase 2 Opportunities:**
- [ ] Multi-function speculation (remove main-only restriction)
- [ ] Substrate-aware execution (CPU/GPU/QPU routing)
- [ ] Dynamic agent count tuning
- [ ] Performance benchmarking suite
- [ ] Graceful agent failure handling

**Phase 3+ Vision:**
- [ ] Quantum substrate integration
- [ ] Distributed speculation across machines
- [ ] Adaptive speculation based on workload

**Testing Guidelines for HLX-S:**
```rust
#[test]
fn test_my_feature() {
    // 1. Create test input
    let config = SpeculationConfig::default();

    // 2. Run speculation
    let coordinator = SpeculationCoordinator::new(config);
    let result = coordinator.execute_speculative(&test_program)?;

    // 3. Verify consensus
    assert_eq!(result, expected_value);
}
```

### 2. Compiler Improvements

**Opportunities:**
- Optimization passes that preserve determinism (A1 axiom)
- Better error messages with source location hints
- New substrate inference heuristics
- Performance profiling and analysis tools

**Axiom Preservation:** Any optimization must preserve:
- **A1 (Determinism):** Same input → Same output, always
- **A2 (Reversibility):** State is snapshottable and recoverable
- **A3 (Bijection):** Results are bijectively representable
- **A4 (Universal Value):** All agents work with same Value type

### 3. Runtime & Executor

**Opportunities:**
- GPU backend testing on diverse hardware (NVIDIA, AMD, Intel, Apple)
- GPU backend optimization (Vulkan performance)
- New GPU operations (image filters, tensor ops)
- CPU optimizations
- Cross-platform compatibility testing

### 4. Language Features

**Contracts** (Alpha):
- New validation rules beyond `["not_empty", "valid_email"]`
- Contract synthesis improvements
- Integration with formal verification tools

**LSTX** (Latent Space) (Experimental):
- Latent space operation implementations
- Vector database integration
- Semantic search primitives

### 5. Code Generation (HLX CodeGen)

**Current:** Aerospace (DO-178C) production-ready

**Opportunities:**
- Medical domain (IEC 62304) - Coming Q1 2026
- Automotive domain (ISO 26262) - Coming Q2 2026
- Additional safety-critical domains

### 6. Documentation & Examples

**Opportunities:**
- Real-world examples using HLX
- Benchmarks on diverse AI workloads
- Integration examples with ML frameworks (PyTorch, JAX)
- Safety-critical system case studies
- Formal verification examples

## Submitting Changes

### Before You Start

1. **Check existing issues** — Is someone already working on this?
2. **Open an issue** — Describe what you want to change and why
3. **Wait for feedback** — Let's discuss the approach first
4. **For HLX-S changes:** Coordinate with the phase roadmap

### Making Changes

1. **Create a branch:**
   ```bash
   git checkout -b feature/my-feature
   ```

2. **Write code and tests:**
   ```bash
   # Ensure tests pass
   cargo test

   # Check formatting
   cargo fmt

   # Run clippy
   cargo clippy
   ```

3. **Document your changes:**
   - Update relevant `.md` files if changing behavior
   - Add comments for non-obvious logic
   - Update API docs (doc comments) for public types/functions

4. **Commit with clear messages:**
   ```bash
   git commit -m "feature: Add multi-function speculation

   - Enables @scale on functions other than main()
   - Implements call graph analysis for safety
   - Adds tests for nested speculation

   Closes #123"
   ```

5. **Push and open a PR:**
   ```bash
   git push origin feature/my-feature
   ```

### PR Checklist

- [ ] Tests pass (`cargo test`)
- [ ] Code is formatted (`cargo fmt`)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] Documentation is updated
- [ ] Commit messages are clear
- [ ] For HLX-S: Axioms are preserved (A1-A4)
- [ ] For compiler: Determinism is guaranteed

## Development Workflow

### Understanding the Axioms

HLX's core guarantee is: **Same input → Same output, everywhere.**

This is preserved through four axioms:

- **A1 (Determinism):** Code execution is fully deterministic
- **A2 (Reversibility):** State can be snapshots and rolled back
- **A3 (Bijection):** Results map bijectively to inputs
- **A4 (Universal Value):** All agents/threads use same Value representation

When making changes, ask: *"Does this preserve all axioms?"*

### Debugging HLX-S Code

```bash
# Enable detailed HLX-Scale logging
RUST_LOG=1 cargo run --bin hlx -- run program.lcc

# Check for specific barrier behavior
RUST_LOG=1 cargo test test_multi_barrier 2>&1 | grep BARRIER

# Verify determinism preservation
./verify_hlx_scale.sh
```

### Performance Profiling

```bash
# Generate flamegraph (requires flamegraph installed)
cargo flamegraph

# Profile barrier overhead
RUST_LOG=1 cargo test test_barrier_overhead -- --nocapture
```

## Common Mistakes to Avoid

1. **Breaking determinism**: Any change that makes code non-deterministic breaks the core promise. Even small things (floating-point operations, hash-based data structures, timing) can break this.

2. **Ignoring axioms**: Before submitting, verify your change preserves A1-A4.

3. **Unoptimized parallelism**: @scale has overhead. Small computations are slower in parallel. Always profile.

4. **Unclear error messages**: Users will hit your error messages. Make them helpful.

5. **Missing tests**: If you fix a bug, add a test that would have caught it.

## Questions?

- **Issues**: [GitHub Issues](https://github.com/latentcollapse/hlx-compiler/issues) - Bug reports, features, questions
- **Discussions**: [GitHub Discussions](https://github.com/latentcollapse/hlx-compiler/discussions) - Ideas, design questions
- **HLX-Scale specific**: See [HLX-SCALE.md](hlx/HLX-SCALE.md) and [HLX-SCALE-QUICKSTART.md](hlx/HLX-SCALE-QUICKSTART.md)

---

## License

All contributions are under Apache License 2.0. By submitting a PR, you agree to license your code under this license.

**Thank you for contributing to HLX!**
