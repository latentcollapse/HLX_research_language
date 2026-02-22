# HLX → Klyntar Roadmap
**Date:** 2026-02-22
**Context:** Post-v0.1.2 audit completion, preparing for neurosymbolic integration

---

## Vision

**HLX** = The explicit, verbose, manual symbolic AI runtime. Full control, no magic.

**Klyntar** = The "pip install and go" neurosymbolic framework. Feed it weights, symbiote bonds, ready to engage.

**The Bond:** HLX symbiote + text-only LLM = complete neurosymbolic AI where HLX handles multimodal/symbolic, LLM handles language.

---

## What We're NOT Changing

The core HLX architecture is **elegant and correct**:

- **Four axioms:** Determinism, Boundedness, Auditability, Zero hidden state
- **Inference layer:** Flow → Guard → Shield → Fortress modes
- **RSI pipeline:** Three-gate model (proof → consensus → human)
- **SCALE coordination:** Barriers, consensus, agent pools
- **Governance:** Effect predicates, rate limiting, severity caps
- **Tensor system:** Shape-checked, allocation-tracked, bounded

These are the foundation. We add, we don't replace.

---

## Backend Status (ACCURATE AS OF v0.1.2)

### Vulkan Backend: `backends/vulkan/mod.rs` (~3400 lines)

**Already implemented:**
- Full GPU compute with SPIR-V shaders
- Shader attestation with SHA256 verification
- Tensor operations: add, gemm, activation, softmax, layernorm, cross_entropy, elementwise, reduction, conv2d, pooling, batchnorm, dropout, transpose
- **Image processing shaders (ALREADY EXIST!):**
  - gaussian_blur
  - sobel (edge detection)
  - grayscale
  - threshold
  - brightness
  - contrast
  - invert_colors
  - sharpen

**What we need to add:**
- Image load/save (PNG, JPEG) - the processing exists, but not the decode/encode
- Audio load/save
- Expose these shaders through builtins

### LLVM Backend: `backends/llvm/mod.rs` (~2300 lines)`

**Already implemented:**
- JIT/AOT compilation via inkwell (LLVM Rust bindings)
- Control flow graph analysis
- Native machine code generation
- Function compilation and optimization

**What we need to add:**
- Expose JIT path for hot bytecode
- Profile-guided optimization hints

---

## What We're Adding

### 1. Image Load/Save (Week 1)

**Current state:** Vulkan has image PROCESSING shaders, but no image DECODE/ENCODE

**Implementation:**
```
image.png → [decode CPU] → Tensor { shape: [C, H, W], data: [...] }
Tensor → [Vulkan processing] → Tensor (processed)
Tensor → [encode CPU] → image.png
```

**Files to modify:**
- `hlx-core/src/tensor.rs` - Add `from_image_bytes()`, `to_image_bytes()`
- `hlx-runtime/src/builtins.rs` - Add `image_load`, `image_save`, `image_process`

**Dependencies:**
- `image` crate for PNG/JPEG decode/encode

**Vulkan integration:**
- Shaders already exist, just need to wire them to builtins

### 2. Audio Load/Save (Week 2)

**Goal:** Standard handshake between HLX symbiote and LLM weights

**Why:** Klyntar needs a reproducible way to "bond" the symbiote to a model

**Protocol phases:**
```
1. HELLO    - Symbiote announces capabilities, LLM responds with model card
2. SYNC     - Exchange initial state (memory, latent variables)
3. BOND     - Confirm bond, establish communication channels
4. READY    - System is live, ready for CLI interaction
```

**Data structures:**
```rust
struct BondRequest {
    symbiote_version: String,
    capabilities: Vec<Capability>,
    initial_memory: HashMap<String, Value>,
}

struct BondResponse {
    model_name: String,
    model_version: String,
    context_window: usize,
    accepted: bool,
}
```

**Files to add:**
- `bond.rs` - Protocol implementation
- `symbiote.rs` - Symbiote state management

---

### 3. Memory Bridge

**Goal:** HLX state ↔ LLM context window

**Why:** The LLM needs to "see" what HLX knows, and HLX needs to incorporate LLM outputs

**Bridge format:**
```
HLX State → [serialize] → LLM context string (text/markdown/JSON)
LLM output → [parse] → HLX Value / AgentMemory update
```

**Implementation:**
- `to_context()` method on `AgentMemory`, `Value`, `Tensor`
- `from_llm_output()` parser for structured LLM responses
- Context window management (summarization, pruning)

**Files to modify:**
- `rsi.rs` - Add `to_context()` on `AgentMemory`
- `value.rs` - Add `to_context()` on `Value`
- `tensor.rs` - Add `to_context()` (tensor summary for LLM)

**Files to add:**
- `bridge.rs` - LLM context serialization/deserialization

---

### 4. Output Formatting

**Goal:** HLX outputs in LLM-consumable formats

**Why:** The neurosymbolic loop requires HLX → LLM → HLX communication

**Formats:**
- **Structured:** JSON, EDN (for programmatic parsing)
- **Natural:** Markdown, plain text (for LLM context injection)
- **Hybrid:** Markdown with embedded JSON blocks

**Implementation:**
- `OutputFormat` enum
- `format_for_llm()` method on `Value`, `Tensor`, `AgentMemory`
- Configurable verbosity (flow mode = brief, fortress mode = full)

---

## LLVM Leverage

Current status: LLVM backend exists but underutilized

**Opportunities:**
1. **JIT compilation** of HLX bytecode to native code
2. **Optimization passes** on tensor operations
3. **SIMD vectorization** for tensor math
4. **Cross-compilation** for different targets (embedded, server, etc.)

**Action items:**
- Audit current LLVM integration
- Add JIT compilation path for hot bytecode
- Explore LLVM's tensor operation patterns

---

## Vulkan Leverage

Current status: Vulkan backend exists but underutilized

**Opportunities:**
1. **GPU tensor operations** (matmul, convolution, etc.)
2. **Image preprocessing** on GPU (resize, normalize, augment)
3. **Parallel agent execution** via compute shaders
4. **Memory sharing** with LLM inference engines

**Action items:**
- Audit current Vulkan integration
- Add GPU-accelerated tensor ops
- Add image preprocessing shaders
- Profile CPU vs GPU for key operations

---

## Implementation Phases

### Phase 1: Image I/O (Week 1)
- [ ] Add `image` crate dependency to hlx-core or hlx-runtime
- [ ] Implement `Tensor::from_image_bytes()` (PNG, JPEG decode)
- [ ] Implement `Tensor::to_image_bytes()` (PNG, JPEG encode)
- [ ] Add `image_load(path)` builtin - loads image → Tensor
- [ ] Add `image_save(tensor, path)` builtin - saves Tensor → image
- [ ] Wire Vulkan image shaders to builtins (blur, sobel, etc.)
- [ ] Add tests for image ↔ tensor roundtrip
- [ ] Document image tensor format: **CHW** (channels, height, width)

### Phase 2: Audio I/O (Week 2)
- [ ] Add `hound` or `symphonia` dependency
- [ ] Implement `Tensor::from_audio()`
- [ ] Implement `Tensor::to_audio()`
- [ ] Add `audio_load` / `audio_save` builtins
- [ ] Add tests for audio ↔ tensor roundtrip

### Phase 3: Bond Protocol (Week 4)
- [ ] Design full protocol spec
- [ ] Implement `BondRequest` / `BondResponse`
- [ ] Add `bond.rs` module
- [ ] Add `symbiote.rs` module
- [ ] Add integration tests for bond handshake
- [ ] Document protocol for Klyntar consumption

### Phase 4: Memory Bridge (Week 5-6)
- [ ] Implement `to_context()` on core types
- [ ] Implement `from_llm_output()` parser
- [ ] Add `bridge.rs` module
- [ ] Design context window management strategy
- [ ] Add tests for bridge roundtrip
- [ ] Document bridge format for LLM integration

### Phase 5: LLVM JIT Integration (Week 7-8)
- [ ] Audit LLVM backend capabilities
- [ ] Expose JIT compilation path for hot bytecode
- [ ] Add profile-guided optimization hints
- [ ] Benchmark JIT vs interpreter
- [ ] Document performance characteristics

---

## Current Architecture Reference

```
hlx/
├── hlx-core/           # Core types (Value, Tensor, etc.)
├── hlx-runtime/        # VM, Compiler, RSI, SCALE, Governance
├── backends/
│   ├── vulkan/         # GPU compute (~3400 lines)
│   │   ├── mod.rs
│   │   └── shaders/    # SPIR-V compute shaders
│   │       ├── pointwise_add.spv
│   │       ├── gemm.spv
│   │       ├── conv2d.spv
│   │       ├── gaussian_blur.spv   # Image processing
│   │       ├── sobel.spv
│   │       ├── grayscale.spv
│   │       └── ... (16 more)
│   └── llvm/           # Native compilation (~2300 lines)
│       └── mod.rs
└── redteaming and auditing data/
    ├── 2026-02-21_Opus_comprehensive_audit.md
    └── 2026-02-22_HLX_Klyntar_Roadmap.md (this file)
```

---

## Testing Strategy

For each phase, we need:

1. **Unit tests** - Pure function behavior
2. **Integration tests** - Cross-module behavior
3. **Roundtrip tests** - Encode → decode → verify
4. **Adversarial tests** - Malformed input, edge cases
5. **Performance tests** - Latency, memory, throughput

Current test count: 74
Target: 150+ by end of Phase 5

---

## Klyntar Integration Notes

Klyntar will consume HLX as a library. Key interfaces:

```python
# Python pseudo-code for Klyntar API
from klyntar import Symbiote

# Bond to weights
symbiote = Symbiote.bond("path/to/model.weights")

# Engage in CLI
symbiote.engage()

# The symbiote now:
# - Receives multimodal input (via HLX tensor pipelines)
# - Processes symbolically (via HLX VM)
# - Generates text (via bonded LLM)
# - Returns output (via HLX formatting)
```

HLX must expose:
- `bond()` entry point
- `process_input()` for multimodal
- `get_context()` for LLM injection
- `receive_output()` for LLM responses

---

## Non-Goals (For Now)

These are explicitly out of scope:

1. **Training** - HLX is inference-only. No gradient computation.
2. **GPU memory management** - Vulkan handles this, we don't reinvent
3. **LLM inference** - That's the bonded model's job, not HLX
4. **Distributed execution** - Single-node for now
5. **Real-time constraints** - Best-effort, not hard real-time

---

## Success Criteria

Phase 1-5 complete when:

- [ ] HLX can load and process images
- [ ] HLX can load and process audio
- [ ] HLX can bond to an LLM via protocol
- [ ] HLX can inject context to LLM
- [ ] HLX can receive LLM output
- [ ] Vulkan path is faster than CPU for tensor ops
- [ ] LLVM JIT path works for bytecode
- [ ] 150+ tests pass
- [ ] Klyntar can import HLX and bond to weights

---

## Open Questions

1. **Image format:** HWC (height, width, channels) or CHW (channels, height, width)?
   - PyTorch uses CHW, TensorFlow uses HWC
   - Recommendation: CHW for consistency with ML ecosystem

2. **Audio sample format:** f32 normalized? i16 raw? Configurable?
   - Recommendation: f32 normalized internally, configurable on load

3. **Bond protocol transport:** stdin/stdout? TCP socket? Unix socket?
   - Recommendation: stdin/stdout for CLI, TCP for server mode

4. **Context window management:** FIFO? Summarization? Importance scoring?
   - Recommendation: FIFO with configurable size, summarization as future work

---

## References

- HLX v0.1.2 audit: `2026-02-21_Opus_comprehensive_audit.md`
- HLX runtime: `hlx-runtime/src/`
- HLX core: `hlx-core/src/`
- Vulkan backend: `backends/vulkan/mod.rs` (~3400 lines, fully functional)
- LLVM backend: `backends/llvm/mod.rs` (~2300 lines, JIT/AOT ready)
- Vulkan shaders: `backends/vulkan/shaders/*.spv` (16+ compute shaders)

---

*Document created by GLM5 with Matt. HLX stays elegant. We're just adding connectors. The backends are already FAR more advanced than expected - we just need to wire them up.*
