# HLX Phase 2 Roadmap: The Engine & The Ecosystem

**Status:** Phase 1 (Bootstrap) Complete. Control Plane (HLX-C) Active.
**Goal:** Achieve hardware-accelerated, deterministic execution on NVIDIA/AMD via Vulkan.

---

## 1. The Engine (Vulkan Backend)

The memory allocator is in place. Now we need to light the compute fires.

- [ ] **SPIR-V Generation (`spirv_gen.rs`):**
    - Implement a translator from LC-B `Instruction` to SPIR-V assembly (using `rspirv` or raw bytes).
    - Map `MatMul`, `Add`, `Gelu` to GLSL/SPIR-V compute shaders.
- [ ] **Compute Dispatch:**
    - Implement `cmd_dispatch` in `VulkanBackend`.
    - Integrate `BackendTuning` to select workgroup sizes dynamically.
- [ ] **Pipeline Management:**
    - Implement `PipelineCache` to avoid recompiling shaders every run.
    - Manage Descriptor Sets for binding tensors.

## 2. The Ecosystem (HLX-C Standard Library)

We need to build the "libc" of HLX.

- [ ] **`std.hlxc`:**
    - Basic math functions (`pow`, `exp`, `log`).
    - Tensor manipulation (`slice`, `concat`).
    - Randomness (`rand_uniform`, `rand_normal`) using the deterministic seed.
- [ ] **Kernel Library:**
    - Implement `Softmax`, `LayerNorm`, `Attention` in pure HLX-C (as reference) and optimized Kernels.

## 3. The Validation (Chaos Monkey)

- [ ] **Cross-Vendor Test Suite:**
    - A script that runs the same LC-B capsule on CPU, NVIDIA, and AMD (when available).
    - Verifies bit-exact output hashes.

## 4. The Vision (Helinux Kernel)

- [ ] **Scheduler Prototype:**
    - A simple Round-Robin scheduler written in HLX-C that manages a queue of dummy "processes" (capsules).
    - Demonstrates the usage of `max_iter` (DLB) to preempt tasks.

---

**Next Immediate Step:** Implement `spirv_gen` to bridge the gap between `Instruction` and the GPU.
