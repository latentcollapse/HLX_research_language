# HIL — HLX Inference Layer

HIL is "how Bit reasons." It's a first-class HLX standard library providing
inference, pattern recognition, and learning primitives.

## Relationship to APE

APE governs HLX from the outside — "the physics of what Bit cannot do."
HIL operates from the inside — it's what Bit imports to think.

```
HLX Runtime
  ├── APE embedded (governance, conscience predicates, formal proofs)
  │
  └── HLX program (Bit)
        └── use hil::infer;
            use hil::pattern;
            use hil::learn;
```

APE and HIL never bleed into each other.

## Modules

- `infer.hlx` — Inference primitives (confidence scoring, hypothesis testing, reasoning chains)
- `pattern.hlx` — Pattern recognition and matching (observation → pattern extraction)
- `learn.hlx` — Learning and memory consolidation (pattern storage, retrieval, forgetting)

## Status

These are stub modules. They will become importable once Phase 3 (Module System) lands.
The Rust-side implementations backing these functions already exist in `hlx-runtime/src/`:

- `tensor.rs` — Tensor operations (backing `hil::infer`)
- `lora_adapter.rs` — LoRA weight adaptation (backing `hil::learn`)
- `memory_pool.rs` — Observation/pattern storage (backing `hil::pattern`, `hil::learn`)
- `bond.rs` — LLM bonding protocol (backing `hil::infer`)
- `forgetting_guard.rs` — Memory retention policies (backing `hil::learn`)

Once the module system exists, these `.hlx` files will wrap the Rust builtins
with HLX-native syntax: `use hil::infer;` instead of calling raw builtins.
