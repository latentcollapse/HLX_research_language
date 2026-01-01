# HLX Compiler - Rust Implementation

## Build Status

**Complete implementation** of the HLX compiler architecture in Rust.
This implements LC-B as the Universal IR - the "LLVM of deterministic ML."

## Architecture

```
HLXL (ASCII) ──┐
               ├─→ Compiler (Rust) ─→ LC-B Capsule ─→ Runtime (Rust) ─→ Result
HLX (Runic) ───┘
```

## Crate Structure

```
hlx-compiler/
├── Cargo.toml                 # Workspace root
├── hlx_core/                  # Core types and IR
│   ├── src/
│   │   ├── lib.rs            # Module exports
│   │   ├── value.rs          # 7 fundamental types + Contract + Handle
│   │   ├── instruction.rs    # 50+ IR instructions
│   │   ├── capsule.rs        # Integrity-wrapped instruction sequences
│   │   ├── lcb.rs            # LC-B wire format codec
│   │   └── error.rs          # Deterministic error types
│   └── Cargo.toml
├── hlx_compiler/              # Frontend (parser + emitter)
│   ├── src/
│   │   ├── lib.rs            # Module exports
│   │   ├── ast.rs            # Shared AST for HLXL and HLX
│   │   ├── parser.rs         # Parser trait
│   │   ├── emitter.rs        # Emitter trait
│   │   ├── hlxl.rs           # HLXL (ASCII) parser/emitter
│   │   ├── runic.rs          # HLX (Runic) parser/emitter + transliteration
│   │   └── lower.rs          # AST → Instructions → Capsule
│   └── Cargo.toml
├── hlx_runtime/               # Execution engine
│   ├── src/
│   │   ├── lib.rs            # Runtime entry points
│   │   ├── config.rs         # Runtime configuration
│   │   ├── backend.rs        # Backend trait abstraction
│   │   ├── executor.rs       # Instruction dispatch engine
│   │   ├── value_store.rs    # Content-addressed storage (CAS)
│   │   └── backends/
│   │       ├── mod.rs
│   │       └── cpu.rs        # CPU backend with ndarray
│   └── Cargo.toml
└── hlx_cli/                   # Command-line interface
    ├── src/
    │   └── main.rs           # CLI entry point
    └── Cargo.toml
```

## Key Components

### Value System (hlx_core/value.rs)
- 7 fundamental types: Null, Boolean, Integer, Float, String, Array, Object
- Contract: Type-tagged structures with schema validation
- Handle: Content-addressed references (&h_...)
- Determinism: Float NaN/Inf rejected, -0.0 normalized, keys sorted

### Instruction Set (hlx_core/instruction.rs)
- Scalar ops: Add, Sub, Mul, Div, Neg, Eq, Ne, Lt, Le, Gt, Ge, And, Or, Not
- Tensor ops: MatMul, MatMulBias, TensorCreate, Reshape, Transpose
- NN layers: LayerNorm, Softmax, Gelu, Relu, Attention
- Loss: CrossEntropy (with softmax probs output)
- Reductions: ReduceSum, ReduceMean, ReduceMax
- Embeddings: Embedding lookup + backward
- Optimizer: AdamUpdate with bias correction
- Latent Space: Collapse, Resolve, Snapshot

### Capsule System (hlx_core/capsule.rs)
- Version-tagged (CAPSULE_VERSION=1)
- BLAKE3 integrity hash
- Metadata: source_file, compiler_version, register_count
- Validation: Hash verification + register use-before-def

### LC-B Wire Format (hlx_core/lcb.rs)
- Magic byte: 0x7C ('|')
- Type tags: 0x00 (Null), 0x01 (False), 0x02 (True), 0x10 (Integer), etc.
- LEB128 for variable-length integers
- IEEE 754 big-endian for floats
- Deterministic: Same value → same bytes

### Glyph Mapping (hlx_compiler/runic.rs)
HLXL → HLX bidirectional transliteration:
- Structure: program→⟠, block→◇, let→⊢, return→↩
- Operators: +→⊕, -→⊖, *→⊗, /→⊘, ==→⩵, and→∧, or→∨
- LS ops: collapse→⚳, resolve→⚯, snapshot→⚶
- Types: null→Ⓝ, true→Ⓣ, false→Ⓕ

## CLI Usage

```bash
# Compile HLXL to capsule
hlx compile program.hlxl -o program.lcb

# Run a capsule
hlx run program.lcb

# Transliterate HLXL ↔ HLX
hlx translate --from hlxl --to hlx program.hlxl

# Inspect a capsule
hlx inspect program.lcb --json

# Run smoke tests
hlx test
```

## Determinism Guarantees

1. **A1 (Determinism)**: Same input → same LC-B output
2. **A2 (Reversibility)**: decode(encode(v)) == v
3. **A3 (Bijection)**: 1:1 correspondence between forms
4. **A4 (Universal Value)**: All types reduce to HLX-Lite subset

## To Build

Requires Rust 1.70+:

```bash
cd hlx-compiler
cargo build --release
cargo test
cargo run --bin hlx -- test
```

## Smoke Test

The implementation passes this core test:

```rust
// 🜃5 + 🜃3 = 8
let capsule = Capsule::new(vec![
    Instruction::Constant { out: 0, val: Value::Integer(5) },
    Instruction::Constant { out: 1, val: Value::Integer(3) },
    Instruction::Add { out: 2, lhs: 0, rhs: 1 },
    Instruction::Return { val: 2 },
]);

let result = execute(&capsule).unwrap();
assert_eq!(result, Value::Integer(8));
```

## Dependencies

- serde: Serialization
- bincode: Binary encoding
- blake3: Cryptographic hashing
- nom: Parser combinators
- ndarray: CPU tensor ops
- thiserror/anyhow: Error handling
- clap: CLI argument parsing

## Next Steps

1. Vulkan backend (hlx_runtime/src/backends/vulkan.rs)
2. SPIR-V code generation
3. Integration with existing GLSL shaders
4. Benchmarks (criterion)
5. Full runic parser (currently uses transliteration)
