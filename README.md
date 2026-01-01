# HLX Compiler (Rust Bootstrap)

**The Native Tongue of Synthetic Intelligence.**

HLX is a deterministic, bijective, and Turing-complete language family designed to compile directly to **LC-B (Latent Capsule Binary)** and execute on **SPIR-V (Vulkan)** hardware. It eliminates the "black box" of the CUDA stack, offering bit-perfect reproducibility across NVIDIA, AMD, and Intel GPUs.

> **Status:** Phase 1 Complete (Rust Bootstrap). The compiler, runtime, and HLX-C parser are functional. GPU backend is in progress.

---

## 🏗 Architecture

The repository is organized as a Rust workspace:

*   **`hlx_core`**: The foundational types, `Instruction` set, and `Capsule` format (with BLAKE3 hashing).
*   **`hlx_compiler`**: The compiler frontend.
    *   **HLX-C**: C-like Turing-complete control language (`if`, `loop`, `fn`).
    *   **HLXL**: Linear data description language.
    *   **Lowering**: Translates AST to flat LC-B instructions.
*   **`hlx_runtime`**: The execution engine.
    *   **Executor**: Deterministic VM with **Deterministic Loop Bounds (DLB)** safety.
    *   **BackendTuning**: Hardware abstraction for vendor-specific optimizations.
*   **`hlx_cli`**: Command-line tools and test runners.

## 🚀 Getting Started

### Prerequisites
*   Rust (latest stable)
*   Vulkan SDK (for GPU backend)

### Build
```bash
cargo build --release
```

### Run Tests
Verify the core logic, determinism, and parser:
```bash
cargo test
```

### Run the HLX-C Demo (Fibonacci)
Compile and execute a Turing-complete program (Fibonacci sequence) on the deterministic runtime:
```bash
cargo run -p hlx_cli --bin test_hlxc_run
```

---

## 📜 The Tri-Track Protocol

1.  **HLXL (Data Plane):** Immutable configuration and tensor shapes.
2.  **HLX-C (Control Plane):** Logic, loops, and kernels. Safe, bounded execution.
3.  **LC-R (Visual Plane):** Graph-based visualization of logic.

All compile bijectively to **LC-B**.

## 📄 Documentation

*   [HLX-C Specification](docs/HLXC_SPECIFICATION.md)
*   [Research Paper: HLX Native Tongue](https://github.com/latentcollapse/hlx-research/blob/main/papers/HLX_NATIVE_TONGUE_ARXIV.md) (Link to be updated)

---

*Note: The legacy Python prototype has been moved to `_archive/legacy_v1`.*
