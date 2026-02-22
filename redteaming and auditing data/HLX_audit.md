# HLX Runtime Audit Report

**Auditor:** Claude Opus 4.6 (at Matt's request)
**Date:** 2025-02-21
**Scope:** `hlx-runtime/src/` — all 12 Rust source files (~5,095 LOC)
**Purpose:** Identify bugs, safety issues, and architectural concerns before next iteration

---

## Audit Summary

| Severity | Count | Description |
|----------|-------|-------------|
| CRITICAL | 6 | Will panic or produce wrong results on valid/malformed input |
| HIGH | 7 | Memory safety, race conditions, bounds violations |
| MEDIUM | 8 | Logic bugs, inconsistencies, silent failures |
| LOW | 5 | Stubs, documentation gaps, minor risks |
| **Total** | **26** | |

**Overall assessment:** The architecture is sound — the four-pillar design (determinism, boundedness, auditability, zero hidden state) maps cleanly to the implementation. The critical issues are concentrated in two areas: **unchecked indexing** (vm.rs, bytecode.rs, compiler.rs) and **integer overflow** (tensor.rs, compiler.rs). These are fixable without architectural changes.

---

## CRITICAL Issues

### C1. Unchecked Register Indexing in VM
**File:** `vm.rs` lines 196-233
**Impact:** Panic on out-of-bounds register index from bytecode

The arithmetic opcodes index `self.registers[a]` and `self.registers[b]` directly without bounds checks. If bytecode specifies a register index >= 256, the VM panics instead of returning a RuntimeError.

```rust
// Line 200
let result = self.binary_add(&self.registers[a], &self.registers[b])?;
// Lines 206, 214, 223, 229: same pattern for Sub, Mul, Div, Mod
```

**Fix:** Use `self.get_register(a)` which already exists and handles bounds safely, or add bounds validation before indexing:
```rust
if a >= self.registers.len() || b >= self.registers.len() {
    return Err(RuntimeError::new("Register index out of bounds", pc));
}
```

---

### C2. Integer Overflow in Tensor Shape Calculation
**File:** `tensor.rs` lines 62, 210, 246
**Impact:** Silent wraparound producing wrong-sized tensors; potential enormous allocation

The shape-to-element-count calculation uses unchecked `.product()` on `usize`. A shape like `[65536, 65536, 65536]` would overflow silently.

```rust
// Line 62
let total: usize = shape.iter().product();
```

Additionally, the `size()` method at line 210 uses `self.data.iter().product::<f64>() as usize` — this computes the **product of the data values**, not the element count. This is almost certainly a bug (should use `self.data.len()` or `self.shape.iter().product()`).

**Fix:**
```rust
let total = shape.iter().try_fold(1usize, |acc, &dim| acc.checked_mul(dim))
    .ok_or_else(|| RuntimeError::new("Tensor shape overflow", 0))?;
```
And fix `size()` to return `self.shape.iter().product::<usize>()` or `self.data.len()`.

---

### C3. Register Overflow in Compiler
**File:** `compiler.rs` lines 416-419, 456-459
**Impact:** `u8` overflow wraps register index to 0, corrupting compilation

`next_var_reg` and `next_tmp_reg` are `u8` (max 255). Functions with > 255 variables silently wrap around, producing bytecode that overwrites register 0.

```rust
// Line 416-419: no overflow protection
let reg = self.next_var_reg;
self.next_var_reg += 1;  // wraps at 255
```

**Fix:** Check before increment:
```rust
if self.next_var_reg >= 255 {
    return Err(CompileError {
        message: "Too many variables: register limit exceeded (max 255)".into(),
        line: self.current_line,
    });
}
```

---

### C4. Bytecode Read Panics
**File:** `bytecode.rs` lines 262, 271, 280
**Impact:** `.unwrap()` panics on slice-to-array conversion

```rust
// Line 262
let bytes: [u8; 2] = self.code[*pc..*pc + 2].try_into().unwrap();
// Line 271: [u8; 4] variant
// Line 280: [u8; 8] variant
```

While bounds are checked before the slice operation, `try_into().unwrap()` is still a hard panic path. In a safety-critical runtime, every `unwrap()` is a defect.

**Fix:** Replace with `map_err`:
```rust
let bytes: [u8; 2] = self.code[*pc..*pc + 2].try_into()
    .map_err(|_| RuntimeError::new("Bytecode read: invalid byte slice", *pc))?;
```

---

### C5. RSI Serialization Panic
**File:** `rsi.rs` line 268
**Impact:** `.expect()` panics if serialization fails

```rust
bincode::serialize(&snapshot).expect("Serialization failed")
```

If any value in the snapshot contains unsupported types (which is possible since `Value` has complex variants), this panics instead of returning a proper error.

**Fix:**
```rust
bincode::serialize(&snapshot)
    .map_err(|e| RuntimeError::new(format!("RSI serialization failed: {}", e), 0))?
```

---

### C6. Negative Index Underflow in `substring` Builtin
**File:** `builtins.rs` lines 17-31
**Impact:** Panic on negative `start` or `len` arguments

The `start` and `len` values are cast from `i64` to `usize` without checking for negative values. A negative i64 becomes a massive usize, causing a panic on slice access.

```rust
let start = args[1].as_i64()... as usize;  // negative -> huge positive
let len = args[2].as_i64()... as usize;
let end = (start + len).min(s.len());
Ok(Value::String(s[start..end].to_string()))  // panic if start > s.len()
```

Additionally, `start` is not clamped to `s.len()`, so even a valid positive value that exceeds string length will panic.

**Fix:**
```rust
let start_i = args[1].as_i64()?;
let len_i = args[2].as_i64()?;
if start_i < 0 || len_i < 0 {
    return Err(RuntimeError::new("substring: negative index", 0));
}
let start = (start_i as usize).min(s.len());
let end = (start + len_i as usize).min(s.len());
```

---

## HIGH Issues

### H1. Function Call Register Boundaries Are Hard-Coded
**File:** `vm.rs` lines 1192-1197
**Impact:** Stack corruption if arg_count exceeds register bounds

```rust
let saved_regs: Vec<Value> = self.registers[..20].to_vec();
let arg_base = 150;
for i in 0..arg_count.min(param_count) {
    self.registers[i + 1] = self.registers[arg_base + i].clone();
}
```

The magic numbers 20 and 150 are not validated. If bytecode specifies `arg_base + arg_count > 256`, this panics.

**Fix:** Validate `arg_base + arg_count <= self.registers.len()` before the loop.

---

### H2. Global Tensor Allocation Never Freed
**File:** `tensor.rs` lines 102, 163-187
**Impact:** Memory tracking drifts from reality; limit reached prematurely

`GLOBAL_TENSOR_ALLOCATION` is incremented on creation but never decremented when tensors are dropped. Tensor `Clone` also doesn't update tracking.

**Fix:** Implement `Drop` for `Tensor`:
```rust
impl Drop for Tensor {
    fn drop(&mut self) {
        GLOBAL_TENSOR_ALLOCATION.fetch_sub(self.data.len(), Ordering::Relaxed);
    }
}
```
And add tracking to Clone.

---

### H3. Barrier Double-Arrival Race Condition
**File:** `scale.rs` lines 122-124
**Impact:** Race condition in multi-threaded barrier synchronization

The check for duplicate arrival and the insert are not atomic:

```rust
if self.arrived.contains(&agent_id) {
    return Ok(self.state == BarrierState::Released);
}
self.arrived.push(agent_id);  // another thread could insert between check and push
```

**Fix:** Use a `HashSet` for O(1) dedup, and ensure the check+insert is under a single lock scope (which it appears to be via `&mut self`, but callers must ensure this).

---

### H4. Governance: Denied Effects Still Modify State
**File:** `governance.rs` (GovernanceContext methods)
**Impact:** Side effects persist even when governance denies the action

When a governance check fails, the effect has already been recorded in the context's history. This means rate-limiting counts denied attempts, which can cause cascading denials.

**Fix:** Only record the effect in history after the governance check passes:
```rust
// Check first
let result = self.check_governance(&effect)?;
// Record only on success
if result.allowed {
    self.history.push(effect);
}
```

---

### H5. `strcmp` Implementation Bug
**File:** `builtins.rs` line 50
**Impact:** Wrong return values for string comparison

```rust
Ok(Value::I64(a.cmp(b) as i64 - 1))
```

`Ordering` discriminant values are `Less=255(-1)`, `Equal=0`, `Greater=1` on most platforms, but this is **not guaranteed by Rust**. The spec says `strcmp` returns -1/0/1, but casting the `Ordering` enum to i64 and subtracting 1 is undefined behavior in terms of ABI stability.

**Fix:** Use an explicit match:
```rust
Ok(Value::I64(match a.cmp(b) {
    std::cmp::Ordering::Less => -1,
    std::cmp::Ordering::Equal => 0,
    std::cmp::Ordering::Greater => 1,
}))
```

---

### H6. `char` Builtin: Unchecked i64-to-u8 Cast
**File:** `builtins.rs` line 66
**Impact:** Truncation produces wrong character for values > 255

```rust
let code = args[0].as_i64()? as u8;  // 256 becomes 0, 300 becomes 44, etc.
```

**Fix:**
```rust
let code_i = args[0].as_i64()?;
if code_i < 0 || code_i > 127 {  // or 255 for extended ASCII
    return Err(RuntimeError::new("char: code out of ASCII range", 0));
}
let code = code_i as u8;
```

---

### H7. Compiler Tokenizer: Fragile Bounds Checks
**File:** `compiler.rs` lines 80-215
**Impact:** Potential panic on malformed source near end of input

Direct `chars[pos]` access at line 80 is safe due to the outer while loop, but subsequent `chars[pos + 1]` accesses at lines 87, 107, 113 etc. rely on separate bounds checks that don't consistently precede the access.

**Fix:** Extract a helper `fn peek(&self, offset: usize) -> Option<char>` to make bounds checking systematic.

---

## MEDIUM Issues

### M1. Silent Skip on Invalid Latent Names
**File:** `vm.rs` lines 549-578
**Impact:** `LatentGet`/`LatentSet` silently do nothing if string index is invalid

```rust
if let Some(name) = bytecode.strings.get(name_idx) { ... }
// else: silently continues — should be an error
```

**Fix:** Return `RuntimeError` for invalid string indices.

---

### M2. Agent Memory Leak on Dissolve
**File:** `vm.rs` lines 88, 1113-1117
**Impact:** `agent_memories` HashMap entries persist after agent dissolution

Spawned agents add entries to `agent_memories` but `AgentDissolve` doesn't clean them up.

**Fix:** Add `self.agent_memories.remove(&agent_id)` in the AgentDissolve handler.

---

### M3. No Operator Precedence in Compiler
**File:** `compiler.rs` (expression parsing)
**Impact:** `1 + 2 * 3` evaluates as `(1 + 2) * 3 = 9` instead of `1 + (2 * 3) = 7`

The compiler parses expressions left-to-right without precedence rules. This is a significant semantic deviation from mathematical convention and most languages.

**Fix:** Implement Pratt parsing or precedence climbing for expressions.

---

### M4. `patch_function_calls` Is a No-Op Stub
**File:** `compiler.rs` lines 844-846
**Impact:** Forward function references may not resolve correctly

```rust
fn patch_function_calls(&mut self) -> Result<(), CompileError> {
    Ok(())
}
```

If functions are referenced before definition, calls won't be patched to the correct addresses.

**Fix:** Implement forward reference patching or enforce declaration-before-use.

---

### M5. Consensus Division-by-Zero Risk
**File:** `scale.rs` lines 219-226
**Impact:** Division by zero if `votes.len() == 0`

```rust
let total = self.votes.len();
let agreement = winning_count as f64 / total as f64;
```

While guarded by `is_complete()`, the guard checks `expected >= votes.len()` which doesn't prevent `total == 0`.

**Fix:** Add explicit `if total == 0 { return Err(...) }` guard.

---

### M6. Shader Bytes Mutable After Attestation
**File:** `shader_attestation.rs`
**Impact:** Registered shaders can be modified after verification, invalidating attestation

The `ShaderInfo` struct stores shader bytes as `Vec<u8>`. After registration and attestation, the bytes can be mutated through a mutable reference, silently invalidating the stored hash.

**Fix:** Store bytes as `Arc<[u8]>` or a frozen wrapper that prevents mutation.

---

### M7. Asymmetric Serialize/Deserialize for Complex Constants
**File:** `bytecode.rs` lines 531-534
**Impact:** Bytecode containing Array/Map/Tensor constants can be serialized but not deserialized

Serialization handles all Value variants, but deserialization returns an error for types 6-9 (Array, Map, Tensor, Bytes). This breaks the round-trip guarantee needed for BLAKE3 integrity verification.

**Fix:** Implement matching deserialization for all serializable types.

---

### M8. Compiler Jump Patch Unsafe
**File:** `compiler.rs` lines 327-329, 700-702, 733-735
**Impact:** Panic if jump target offset exceeds bytecode length

```rust
code[skip_jump..skip_jump + 4].copy_from_slice(&(end_pc as u32).to_le_bytes());
```

If `skip_jump + 4 > code.len()`, this panics.

**Fix:** Bounds-check before patching or reserve space in advance.

---

## LOW Issues

### L1. Unbounded Agent ID Counter
**File:** `agent.rs` lines 120-129 — `next_id` is `u64`, wraps after 2^64 spawns. Theoretical but violates the "no hidden state" axiom.

### L2. Tensor Slice Stride Calculation
**File:** `tensor.rs` line 289 — `self.shape[dim + 1..].iter().product()` returns 1 for empty slice, producing silently wrong strides for the last dimension.

### L3. Governance Rate Limit Window Undocumented
**File:** `governance.rs` lines 323-345 — Rate limiting counts all history entries for the same effect type, but it's unclear if this is per-cycle, per-agent, or global. Needs documentation.

### L4. Incomplete `fn` Pointer Serialization in Governance
**File:** `governance.rs` — Predicates use `fn` pointers, which are not serializable with serde/bincode. This prevents governance config from being saved/restored across sessions.

### L5. Hardcoded `SpawnRateLimit` Constant
**File:** `vm.rs` — The spawn rate limit is compiled into the binary. For a configurable runtime, this should be part of `RuntimeConfig`.

---

## Architectural Observations

### What's Good

1. **BTreeMap over HashMap** — Consistent throughout. This is correct for determinism.
2. **BLAKE3 integrity on bytecode** — The checksum in the bytecode header is the right foundation for auditability.
3. **Three-gate RSI model** — Proof gate -> consensus gate -> human gate is well-structured. The implementation in `rsi.rs` is the most mature module.
4. **Barrier/consensus in SCALE** — The core synchronization model is clean. Hash-based consensus (all agents must produce identical state hashes) is the correct approach.
5. **Value type system** — Clean enum with appropriate variants. `Void` vs `Nil` distinction is well-considered.
6. **Global tensor allocation tracking** — Right idea for boundedness, just needs the Drop impl.

### What Needs Work

1. **Error handling discipline** — 11 `unwrap()`/`expect()` calls across the codebase need to become `?` or `map_err()`. A safety-critical runtime should have zero panic paths.
2. **The compiler is the weakest module** — No operator precedence, no forward reference patching, fragile tokenizer. This is fine for bootstrap but needs hardening before self-hosting.
3. **Thread safety story is incomplete** — Barriers use `&mut self` (exclusive access) but the SCALE model implies concurrent agents. The synchronization primitives (Mutex, channels) aren't visible in the barrier code itself.
4. **Serialization round-trip gap** — Complex constants can be serialized but not deserialized. This breaks the reversibility axiom.
5. **No test coverage for error paths** — The existing tests (18 in rsi.rs, 4 in vm.rs, etc.) cover happy paths. Need adversarial tests: malformed bytecode, register overflow, negative indices.

---

## Recommended Priority Order

1. **Eliminate all `unwrap()`/`expect()` calls** — Replace with proper error propagation. This is the single highest-impact change. (~2 hours)
2. **Fix tensor `size()` method** — It computes the product of data values instead of element count. This is a functional bug. (~5 minutes)
3. **Add bounds checking to VM register access** — Use `get_register()` consistently. (~30 minutes)
4. **Implement `Drop` for `Tensor`** — Fix the allocation tracking leak. (~15 minutes)
5. **Fix `substring` negative index handling** — Add bounds checks. (~10 minutes)
6. **Fix `strcmp` to use explicit match** — Don't rely on enum discriminant values. (~5 minutes)
7. **Add compiler register overflow check** — Prevent u8 wraparound. (~10 minutes)
8. **Implement operator precedence** — Pratt parser or precedence climbing. (~2 hours)
9. **Complete bytecode deserialization** — Match all serializable types. (~1 hour)
10. **Add adversarial test suite** — Malformed bytecode, boundary values, error paths. (~3 hours)

---

## Stats

| File | Lines | Tests | Panics (`unwrap`/`expect`) |
|------|-------|-------|---------------------------|
| vm.rs | 1,449 | 4 | 2 |
| compiler.rs | 1,102 | 0 | 1 |
| rsi.rs | 830 | 18 | 1 |
| tensor.rs | 689 | 0 | 3 |
| bytecode.rs | 666 | 0 | 3 |
| scale.rs | 550 | 0 | 0 |
| governance.rs | 518 | 0 | 0 |
| agent.rs | 232 | 0 | 0 |
| shader_attestation.rs | 225 | 0 | 0 |
| value.rs | 154 | 0 | 0 |
| builtins.rs | 129 | 0 | 1 |
| lib.rs | 46 | 0 | 0 |
| **Total** | **5,590** | **22** | **11** |

---

*Audit conducted by Claude Opus 4.6 on behalf of Matt. GLM5 — the architecture is solid. The issues above are implementation-level, not design-level. The four axioms hold; they just need the code to enforce them without panic paths.*
