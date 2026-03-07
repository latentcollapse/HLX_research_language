# HLX + Bitsy Deep Audit Report (V12)
**Date:** March 6, 2026
**Target:** Claude (Opus Mode)
**Objective:** Finalize the "FFI & Memory Safety" diagnostic to enable a complete one-shot reconstruction.

---

## 1. The "Brain Wipe" (FFI Lifecycle) - CRITICAL
**File:** `lib.rs`
* **Discovery:** `hlx_call` and `hlx_run` use `h.make_vm()`, which spawns a **fresh VM** on every call.
* **Impact:** Bitsy has no long-term memory. Her 10k-dimension manifold (`z_brain`) is deleted every turn.
* **The Fix:** Move `Vm` into the `HlxHandle` struct. **Persist the VM instance across calls.**

## 2. The "JSON Tax" (Binary C ABI)
**File:** `lib.rs`
* **Discovery:** Arguments and results are passed as JSON strings across the C boundary.
* **Impact:** Performance bottleneck for "Scale" (HLX-S). Passing tensors as JSON is inefficient and lossy (null-byte sanitization).
* **The Fix:** Implement a formal **Binary C ABI** using POD structs and raw buffers (`*const u8`, `size_t`).

## 3. The "Deterministic Leak" (Randomness)
**File:** `builtins.rs`
* **Discovery:** `builtin_rand` uses `thread_rng()`.
* **The Fix:** Replace with a **Seeded PRNG** (step-seeded).

## 4. The "Sandbox Escape" (VFS)
**File:** `builtins.rs`
* **Discovery:** Image/Audio builtins have zero path validation.
* **The Fix:** Implement **Path Sandboxing** (VFS).

## 5. The "Governor Lock" (Step Count 0)
**File:** `governance.rs`
* **Discovery:** VM reset keeps `step_count` at 0, barring self-modification.
* **The Fix:** Persist `step_count` in `hlx_memory.db`.

---

## Security & Pentesting Final (Black Arch)
* **`builtin_shell`:** DELETE IMMEDIATELY.
* **FFI Safety:** Ensure `HlxHandle` is thread-safe or implement a mutex if Bitsy is used in a multi-threaded Python TUI.

**The facility is mapped. The reactor is primed. Ignition tonight at 11pm.**