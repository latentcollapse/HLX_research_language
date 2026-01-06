# HLX Compiler - V2 implementation

## Current Status: Phase 4 (Ouroboros)

We are in the final stages of self-hosting the HLX compiler. The Rust-based bootstrap compiler is feature-complete for the HLX-A (ASCII) dialect, and the "Iron" backend (LLVM JIT) is fully operational.

### Achievements
*   **Tier 0 (Axiom):** All 4 HLX Axioms (Determinism, Reversibility, Bijection, Universal Value) verified.
*   **Tier 1 (Iron):** LLVM-based JIT compilation with SDL2 graphics and high-performance tensor support.
*   **Tier 2 (Ouroboros):** Bootstrap compiler (`hlx`) can now compile the self-hosted compiler source (`compiler.hlxc`).

## Architecture

```text
HLX-A (ASCII)  ──┐
                 ├─→ Rust Compiler (hlx) ─→ LC-B Crate ──┐
HLX-R (Runic)  ──┘                                       │
                                                         ▼
Result (Value) ←─ LLVM JIT (Iron Backend) ←─ HLX Runtime (executor)
```

## Language Features (HLX-A)
- **Control Flow:** `if/else` (with optional parens), `loop(cond, max_iter)`, `break`, `continue`.
- **Data Structures:** Dynamic Arrays `[...]`, Object Literals `{"key": val}`, Field access `obj.field`.
- **Types:** Return type annotations `fn name(args) -> type`.
- **Escapes:** Proper string escape handling (`\n`, `\"`, `\t`).
- **Standard Library:** `math`, `vector`, `tensor`, `string`, `io`, `graphics`.

## CLI Usage

```bash
# Compile source to crate
hlx compile source.hlxa -o program.lcc

# Execute crate or source
hlx run program.lcc

# Self-hosting linkage (Stage 2)
hlx run stage1.lcc --output stage2.lcb
hlx build-crate stage2.lcb --output stage2.lcc

# Axiom Verification
cargo run --bin test_all_axioms
```

## Core Mandates
1. **Determinism:** Bit-identical results across all backends.
2. **Semantic Preservation:** LC-B IR retains high-level intent for LLM readability.
3. **Bijective Forms:** Lossless translation between Human (HLX-A) and AI (HLX-R) formats.

## Next Steps
1. Finalize the Ouroboros loop (bit-perfect reproducibility of the self-hosted compiler).
2. Implement the Helinux Kernel prototype in HLX.
3. Full integration of Vulkan compute backend.

```