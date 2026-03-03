# HLX Technical Debt Ledger

**Purpose:** Track deferred work, shortcuts, and "we'll do it later" decisions. Review monthly.

**Legend:**
- 🔴 **Critical:** Blocks production, address immediately
- 🟡 **Warning:** Technical debt accruing interest, schedule soon
- 🟢 **Acceptable:** Conscious trade-off, can defer
- ⚪ **Icebox:** Nice to have, no timeline

---

## Active Debt

### 🔴 Critical (Fix in next 48 hours)

*None — all critical items resolved!* 🎉

### 🟡 In Progress (Handoff to Opus)

*None — all items completed!* 🎉

### ✅ Recently Fixed (Last 24h)

| ID | Date | Description | Resolution | Owner |
|----|------|-------------|------------|-------|
| DEBT-023 | 2026-03-02 | Compound assignment (`+=`, `-=`, `*=`, `/=`, `%=`) silently no-ops | Fixed: Lexer had no compound tokens. `i += 1` tokenized as `i + = 1`, parsed as `Assign{target:(i+nil), value:1}` — silently compiled to no-op. Added `PlusEq/MinusEq/StarEq/SlashEq/PercentEq` tokens to lexer and `CompoundAssign` parsing in `parse_statement`. | Opus |
| DEBT-024 | 2026-03-02 | `parse_primary` default case returns `nil` silently | Fixed: Changed `_ => Ok(Expression::nil())` to return a proper `ParseError`. Was masking compound-assign bug and any future unknown token at expression start. | Opus |
| DEBT-016 | 2026-03-02 | Higher-order functions / lambdas | Fixed: `Value::Function(String)` variant, `CallDyn` opcode (56), `\|params\| body` and `\|\| body` syntax. Lambdas compile as `__lambda_N__` functions. `map`, `fold`, `filter` all work. | Opus |
| DEBT-025 | 2026-03-02 | Array literal lowering byte mismatch | Fixed: `Push` opcode was emitted with 1 byte but VM reads 2. Changed to `Const(empty_array)` + `Push(dst, elem_reg)` per element. | Opus |
| DEBT-020 | 2026-03-02 | Dict literal with string keys fails | Fixed: Reorder field assignment lowering — evaluate value BEFORE loading field name constant. Temp regs 200+ not saved across calls; field_reg was clobbered by `embed()` call. | Opus |
| DEBT-003 | 2026-03-02 | Short-circuit evaluation unverified | Fixed: `&&`/`||` now use JumpIfNot/JumpIf to skip RHS when LHS determines result. `m < top_k && matches[m] >= 0` now safe. | Opus |
| DEBT-002 | 2026-03-02 | Source line mapping | Fixed: Parser propagates token spans to statements. Lowerer emits (pc, line) into line_table. VM enriches errors with get_line(). Errors now show "at line N". | Opus |
| DEBT-012 | 2026-03-02 | Nested latent array assignment broken | Fixed: `arr[idx].field = x` now emits LatentGet→Get→Set→Set→LatentSet read-modify-write sequence. `learn()` and `patterns[i].field` assignments work. | Opus |
| DEBT-001 | 2026-03-02 | `current_time` builtin missing | Added alias to `clock_ms`. `repl_step("Hello Bit")` works! | Kilo |
| DEBT-021 | 2026-03-02 | `shell()` builtin for Bitsy | Bitsy can execute bash commands! `exec_shell("ls -la")` | Kilo |
| DEBT-PAID-001 | 2026-03-02 | `__top_level__` merge bug | Fixed: All module-level lets now merge | 🎉 |
| DEBT-PAID-002 | 2026-03-02 | Register collision (params/locals) | Fixed: `next_var_reg` properly advanced | 🎉 |
| DEBT-PAID-003 | 2026-03-02 | Index assignment was no-op | Fixed: `arr[idx] = val` now works | 🎉 |
| DEBT-PAID-004 | 2026-03-02 | Field assignment unimplemented | Fixed: `obj.field = val` now works | 🎉 |
| DEBT-PAID-005 | 2026-03-02 | saved_register_count too small | Fixed: Increased 20 → 150 | 🎉 |
| DEBT-PAID-006 | 2026-03-02 | Auto-init for arrays/strings | Fixed: Proper defaults emitted | 🎉 |

### 🗑️ DELETED (No longer needed)

| File | Size | Reason |
|------|------|--------|
| `bitsy_learning_core.py` | 19KB | Replaced by HLX bridge |
| `bitsy_pattern_extractor.py` | 14KB | Replaced by HLX bridge |

**Python AI layer: GONE.** 🐍💀

**Remaining Python:**
- `hlx_bridge.py` - Thin wrapper calling `hlx-run`
- `bitsy_tui.py` - Terminal UI using bridge

**Bitsy now runs entirely in HLX.** 🧸🚀

### ✅ Recently Fixed (Last 24h)

| ID | Date | Description | Resolution | Owner |
|----|------|-------------|------------|-------|
| DEBT-001 | 2026-03-02 | `current_time` builtin missing | Added alias to clock_ms | Kilo |

### 🟡 Warning (Schedule within 2 weeks)

| ID | Date | Description | Location | Deferred Because | Resolution Criteria | Owner |
|----|------|-------------|----------|------------------|---------------------|-------|
| DEBT-004 | 2026-03-02 | No garbage collection | `vm.rs` | Copy-on-write sufficient for now | No memory leaks in long runs | TBD |
| DEBT-005 | 2026-03-02 | Windows/Mac untested | All platforms | Linux priority | CI passes on all three | TBD |
| DEBT-006 | 2026-03-02 | Scale migration unimplemented | `hlx-runtime` | Not needed for Bitsy v0.1 | migrate keyword works | TBD |
| DEBT-007 | 2026-02-28 | `concat()` builtin has register collision | `lowerer.rs` | Workaround: use `+` operator | concat() works with local vars | TBD |

### 🟢 Acceptable (Conscious trade-offs)

| ID | Date | Description | Location | Why It's Okay | When to Revisit | Owner |
|----|------|-------------|----------|---------------|-----------------|-------|
| DEBT-008 | 2026-03-01 | No generics for user types | `struct<T>` | Arrays work, user types rare | When users ask for it | TBD |
| DEBT-011 | 2026-02-28 | `as f64` cast not implemented | Parser | `i64_to_f64()` works | Add syntax sugar later | TBD |
| ~~DEBT-022~~ | ~~2026-03-02~~ | ~~For loop has step limit issue~~ | ~~`for x in arr`~~ | **FIXED** | Uses reserved registers 240-243 for loop state | Kilo |

### ✅ Recently Fixed (Today)

| ID | Date | Description | Resolution | Owner |
|----|------|-------------|------------|-------|
| DEBT-008 | 2026-03-02 | Method call syntax | `obj.method()` now works! Lowerer transforms to `method(obj, args)` | Kilo |
| DEBT-009 | 2026-03-02 | For loops | `for x in arr` implemented and working | Kilo/Claude |
| DEBT-017 | 2026-03-02 | Doc comments | `///` comments now parsed (stored but not yet attached to items) | Kilo |

### ⚪ Icebox (No timeline)

| ID | Date | Description | Notes |
|----|------|-------------|-------|
| DEBT-013 | 2026-03-02 | GPU acceleration for tensors | Would be cool, not needed |
| DEBT-014 | 2026-03-02 | Streaming bond responses | Current sync model works |
| DEBT-015 | 2026-03-02 | Regex support in strings | Can add when needed |
| ~~DEBT-016~~ | ~~2026-03-02~~ | ~~Higher-order functions~~ | **FIXED** — Lambdas + `CallDyn` opcode. map/filter/fold work as HLX functions. | Opus |
| DEBT-017 | 2026-03-02 | Doc comments | /// documentation |
| DEBT-018 | 2026-03-02 | Package manager | Import from registry |
| DEBT-019 | 2026-03-02 | LSP server | IDE support |

---

## Recently Paid Off (Celebration Section!)

| ID | Date Paid | Description | Resolution | Celebrated |
|----|-----------|-------------|------------|------------|
| DEBT-PAID-001 | 2026-03-02 | `__top_level__` merge bug | Fixed: All module-level lets now merge | 🎉 |
| DEBT-PAID-002 | 2026-03-02 | Register collision (params/locals) | Fixed: `next_var_reg` properly advanced | 🎉 |
| DEBT-PAID-003 | 2026-03-02 | Index assignment was no-op | Fixed: `arr[idx] = val` now works | 🎉 |
| DEBT-PAID-004 | 2026-03-02 | Field assignment unimplemented | Fixed: `obj.field = val` now works | 🎉 |
| DEBT-PAID-005 | 2026-03-02 | saved_register_count too small | Fixed: Increased 20 → 150 | 🎉 |
| DEBT-PAID-006 | 2026-03-02 | Auto-init for arrays/strings | Fixed: Proper defaults emitted | 🎉 |
| DEBT-PAID-007 | 2026-03-01 | `LatentSet` byte order | Fixed: Corrected (name_idx, src) order | 🎉 |

---

## Debt Metrics

**Current Status:**
- 🔴 Critical: **0 items** 🎉
- 🟡 Warning: 4 items
- 🟢 Acceptable: 3 items
- ⚪ Icebox: 5 items
- **Total Active:** 12 items
- **Paid Off:** **16 items (57% completion rate!)** 🚀

**Session 2 (2026-03-02):**
- Debt paid: 4 items (DEBT-023, DEBT-024, DEBT-016, DEBT-025)
- DEBT-016 (higher-order functions / lambdas) — out of icebox, DONE
- Root cause of loop bug identified and fixed: compound assignment lexer gap

**Session 1 (2026-03-02):**
- Debt paid: 4 items (DEBT-020, DEBT-003, DEBT-002, DEBT-012)
- All critical/in-progress items resolved
- **HLX runtime is solid**

**Velocity:**
- Debt added this week: 8 items (including 3 new DEBT-023/024/025)
- Debt paid this week: **16 items**
- **Net:** -8 items (still CRUSHING it!)

---

## Monthly Review Template

**Date:** ___________

### What we paid off:
- [ ] DEBT-___: _______________
- [ ] DEBT-___: _______________

### What we deferred:
- [ ] DEBT-___ (promoted/demoted): _______________

### New debt added:
- [ ] DEBT-___: _______________

### Critical debt aging:
| ID | Age (days) | Risk Level | Action |
|----|------------|------------|--------|
| | | | |

### Decision log:
- **Kept as debt:** _______________
- **Paid off:** _______________
- **Accepted permanently:** _______________

---

## Decision Log

### 2026-03-02: concat() register collision accepted as debt
**Context:** `concat()` builtin fails when result register overlaps with argument registers. Root cause is lowerer emitting wrong source registers for Call opcode.

**Decision:** Accept as debt, use `+` operator workaround in bit.hlx.

**Rationale:** Fixing requires significant lowerer refactor. `+` operator works fine and is more readable. Will fix if users complain.

**Owner:** TBD
**Revisit:** Post v1.0

---

### 2026-03-02: Nested latent array assignment accepted as debt
**Context:** `patterns[idx].field = x` doesn't work for module-level latent arrays.

**Decision:** Accept as debt.

**Rationale:** Not reached with `pattern_count = 0`. Will fix when pattern storage actually used.

**Owner:** TBD
**Revisit:** When implementing learn/merge patterns

---

### 2026-03-01: Garbage collection deferred
**Context:** No GC, uses copy-on-write for arrays/strings.

**Decision:** Accept as acceptable debt.

**Rationale:** Copy-on-write sufficient for current use. Long-running processes may leak, but Bitsy cycles are bounded.

**Owner:** TBD
**Revisit:** If memory issues observed

---

## How to Add New Debt

1. **Assign next ID:** Check latest ID, increment
2. **Categorize:** 🔴 🟡 🟢 ⚪ based on urgency
3. **Fill all fields:** Date, description, location, reason
4. **Update metrics:** Adjust counts
5. **Commit message:** Use `[DEBT-XXX]` prefix

---

*Ledger created: March 2, 2026*
*Last updated: March 2, 2026*
*Next review: March 9, 2026*

*"Debt is not bad. Unmanaged debt is bad." - Ancient Software Proverb*
