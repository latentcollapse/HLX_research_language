# HLX Debug Briefing: The "Phantom State" Bug

**Context:** We are in Phase 4 (Self-Hosting). The Rust bootstrap compiler (`stage0`) successfully compiles the self-hosted compiler (`stage1`). The `stage1` compiler successfully executes on the VM.

**The Bug:**
During the "Ouroboros" step (Stage 1 compiling Stage 2), the process crashes with a specific validation error:

```text
Error: Execution failed
Caused by:
    E_VALIDATION_FAIL: Key 'pos' not found in object with keys: ["_global_state", "_lt", "_name", "_params", "_t", "_tokens"]
```

**Analysis:**
1.  **Location:** The error occurs in the `compile` function of `hlx_compiler/bootstrap/compiler.hlxc`.
2.  **Symptom:** The `_global_state` object, which is supposed to hold the compiler state (`pos`, `tokens`, `z_bc`, etc.), appears to be replaced by an object containing the *local variable names* of the `compile` function (`_lt`, `_name`, etc.).
3.  **Implication:** This suggests a deep semantic issue where the VM or Lowering pass is confusing a "scope object" or "stack frame map" with the `_global_state` variable itself.
    *   It is *not* a parser error (we fixed `Nom(Tag)`).
    *   It is *not* a simple logic bug (we painstakingly verified `_global_state` reconstruction).
    *   It *is* a runtime value corruption where `_global_state` points to the wrong entity.

**Goal:**
Identify why `_global_state` (register) holds the function's local scope keys instead of its assigned value.

**Hypotheses:**
*   **Lowering Bug (`lower.rs`):** Is the `Ident` lookup resolving to a register that implicitly stores the stack frame?
*   **Runtime Bug (`executor.rs`):** Is `Instruction::Index` or `Call` accidentally exposing internal stack frame maps?
*   **Shadowing:** Is `_global_state` being shadowed by an implicit scope variable?

**Files Provided (`ouroboros_debug_pack.zip`):**
1.  `compiler.hlxc`: The source code triggering the issue.
2.  `lower.rs`: The compiler logic converting AST to IR.
3.  `executor.rs`: The VM executing the code.
4.  `instruction.rs`: The IR definition.
5.  `bootstrap.sh`: The reproduction script.

**To Reproduce:**
Run `./bootstrap.sh`. It will build the Rust compiler, compile Stage 1, and then crash during Stage 2 execution.
