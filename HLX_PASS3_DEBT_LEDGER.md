# HLX Exascale Substrate Ledger & Execution Plan
**Date:** March 6, 2026
**Target:** Claude (Opus 4.6)
**Objective:** Transition HLX from a Hardened VM to a True Exascale Intelligence Substrate.

---

## MILESTONE 1: The Foundation (Language & Logic)
*The core plumbing required to fuse the "Mind" (HLX) with the "Conscience" (APE).*

*   **Phase 1: Syntactic Ignition** (`ast_parser.rs`)
    *   **Mission:** Implement `parse_intent()`, `parse_contract()`, and `parse_do()`.
*   **Phase 2: Compiler Fusion** (`lowerer.rs`)
    *   **Mission:** Wire `ConscienceKernel::verify()` directly into the compiler. Implement Effect Inference (block `WRITE` labeled as `NOOP`).
*   **Phase 3: The Missing Genesis** (`builtins.rs`)
    *   **Mission:** Implement `tensor_rand` to allow native entropy initialization.
*   **Phase 4: Parameter Casting Bug** (`ast_parser.rs`)
    *   **Mission:** Fix the `as` operator losing type context on function parameters.
*   **Phase 5: Ouroboros Verification** (`hlxc`)
    *   **Mission:** Backport the new Axiomatic syntax to the self-hosted compiler.

## MILESTONE 2: Memory & Stability (The Persistent Brain)
*Ensuring the 10k-dimension manifold doesn't leak or drift across turns.*

*   **Phase 6: The "Ghost Tensor" Leak** (`vm.rs`)
    *   **Mission:** Ensure `reset_execution_state()` explicitly drops unused tensors.
*   **Phase 7: Register Exhaustion OOM** (`vm.rs`)
    *   **Mission:** Add `self.registers.fill(Value::Nil)` between turns to prevent inflation.
*   **Phase 8: Cyclic Memory GC** (`value.rs`)
    *   **Mission:** Implement a lightweight, deterministic Mark-and-Sweep pass.
*   **Phase 9: Exception Gravity** (`vm.rs`)
    *   **Mission:** Add `Try/Catch` opcodes to prevent soft-errors from crashing the agent loop.
*   **Phase 10: Persistent Heap Snapshots** (`vm.rs`)
    *   **Mission:** Implement `builtin_snapshot()` to serialize the manifold to disk for cold-starts.

## MILESTONE 3: Security & Governance (The Vault)
*Locking the doors to the host operating system.*

*   **Phase 11: The Sandbox Race (TOCTOU)** (`builtins.rs`)
    *   **Mission:** Fix `validate_sandboxed_path` symlink vulnerability. Use file-handles or `openat`.
*   **Phase 12: The Python FFI Error Leak** (`hlx_ffi.py`)
    *   **Mission:** Wrap JSON decodes in `try/finally` to ensure `hlx_free_string` always executes.
*   **Phase 13: Null-Byte Corruption** (`lib.rs`)
    *   **Mission:** Remove `s.replace('\0', " ")`.
*   **Phase 14: The Standard Library Sandbox** (`axiom-hlx-stdlib/`)
    *   **Mission:** Audit stdlib for `extern` blocks that bypass the sandbox.

## MILESTONE 4: Exascale Performance (Speed & Scale)
*Upgrading the transmission for High-Dimensional Math.*

*   **Phase 15: The C-ABI Zero-Copy** (`lib.rs`)
    *   **Mission:** Move from JSON serialization to Binary ABI (`*const u8`, `size_t`).
*   **Phase 16: The "Clone Army" Seeding** (`vm.rs`)
    *   **Mission:** Seed PRNG with `blake3::hash(agent_id + logical_clock)`.
*   **Phase 17: The Vulkan Phantom** (`builtins.rs`)
    *   **Mission:** Dispatch heavy tensor operations to GPU via `shader_attestation`.
*   **Phase 18: The Network Mirage** (`dd_protocol.rs`)
    *   **Mission:** Add true TCP/UDP sockets for the distributed consensus protocol.

## MILESTONE 5: The Interface (Bitsy's Controls)
*How the Agent talks to the World.*

*   **Phase 19: The Symmetric Bridge** (`hlx_ffi.py`)
    *   **Mission:** Allow Python to pass Tensor handles back into `rt.call()`.
*   **Phase 20: Subconscious Access** (`vm.rs`)
    *   **Mission:** Add `get_latent` and `set_latent` builtins to access persistent memory.
*   **Phase 21: Tether Inbox Async** (`communication.rs`)
    *   **Mission:** Implement `builtin_poll_inbox()` for SQLite integration.
*   **Phase 22: SMI SQLite Sync** (`builtins.rs`)
    *   **Mission:** Ensure `RSIApply` emits a sync event to update `corpus.db`.
*   **Phase 23: The Scrying Engine (Next-Gen LSP)** (`hlx-lsp/`)
    *   **Mission:** Build the Axiomatic LSP (Manifold Inspector, Predictive Governance, Dream Simulator).

---

# Architect's Q&A: Opus Execution Guide

### Milestone 1: Foundation
**Q: Are there existing test cases for intent, do, contract?**
*Archmagos:* Yes. Use the `ape/examples/adversarial/` suite (specifically `conscience_evasion.axm`). If your parser and lowerer are correct, those files should generate compile-time `LowerError`s.
**Q: Does `ConscienceKernel::verify()` log why a violation occurred?**
*Archmagos:* Yes, it returns `ConscienceVerdict::Deny(String)`. You must surface this string directly to the compiler's output so Bitsy knows *why* she was rejected.
**Q: Should verification run at multiple stages?**
*Archmagos:* No. Run it exactly once, at the AST level, *before* lowering to IR. If the thought is heretical, do not waste CPU cycles generating bytecode for it.

### Milestone 2: Stability
**Q: Should `try/catch` catch any error, or specific types?**
*Archmagos:* Specific types only. Catching a `MemoryExhaustion` error is fatal. We only want to catch logical/intent errors (e.g., `IntentFailed`, `FileNotFound`).
**Q: Is the GC synchronous or asynchronous?**
*Archmagos:* Strictly Synchronous. HLX is deterministic. Background threads cause temporal drift. The GC must be a deterministic sweep executed at the end of a designated `Turn` or `Cycle`.

### Milestone 3: Performance & Security
**Q: Should we support both JSON and Binary C-ABI?**
*Archmagos:* Yes. Keep JSON behind a `--debug` or feature flag for introspection, but default to zero-copy Binary for production tensor passing.
**Q: How do we maintain determinism across a network cluster?**
*Archmagos:* The `logical_clock`. Network packets must carry their timestamp. The VM processes them in chronological order. The `Consensus` AST token provides the mathematical barrier.
**Q: Fallback if Vulkan is missing?**
*Archmagos:* Yes, transparent fallback to the existing CPU loops in `builtins.rs`. Single-GPU only for now to prevent PCIe sync drift.

### Milestone 5: The Interface
**Q: Should `builtin_poll_inbox()` be non-blocking?**
*Archmagos:* Yes. If the inbox is empty, return `nil` instantly. The Agent will decide whether to sleep or continue thinking.
**Q: Should the SMI SQLite Sync be blocking or queued?**
*Archmagos:* Queued natively via the SQLite WAL. The VM fires an event, and the Python bridge handles the disk write asynchronously to avoid blocking the tensor math.