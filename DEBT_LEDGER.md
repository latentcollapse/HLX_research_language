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
| DEBT-035 | 2026-03-03 | Phase 3 Module System (`use` keyword) | Fixed: Implemented `use` keyword for canonical stdlib imports. Added recursive module resolution with qualified name mapping (`hil::infer::reason`). | Gemini |
| DEBT-036 | 2026-03-03 | Error Diagnostics & Context | Fixed: Enhanced `ParseError` and `LowerError` with mandatory span info and "Expected/Got" formatting. | Gemini |
| DEBT-023 | 2026-03-02 | Compound assignment (`+=`, `-=`, etc.) | Fixed: Added compound tokens and parsing. | Opus |
| DEBT-016 | 2026-03-02 | Higher-order functions / lambdas | Fixed: `Value::Function` variant, `CallDyn` opcode. | Opus |
| DEBT-025 | 2026-03-02 | Array literal lowering byte mismatch | Fixed: `Push` opcode emission corrected. | Opus |
| DEBT-020 | 2026-03-02 | Dict literal with string keys fails | Fixed: Reorder field assignment lowering. | Opus |

### 🟡 Warning (Schedule within 2 weeks)

| ID | Date | Description | Location | Deferred Because | Resolution Criteria | Owner |
|----|------|-------------|----------|------------------|---------------------|-------|
| DEBT-004 | 2026-03-02 | No garbage collection | `vm.rs` | Copy-on-write sufficient for now | No memory leaks in long runs | TBD |
| DEBT-005 | 2026-03-02 | Windows/Mac untested | All platforms | Linux priority | CI passes on all three | TBD |
| DEBT-006 | 2026-03-02 | Scale migration unimplemented | `hlx-runtime` | Not needed for Bitsy v0.1 | migrate keyword works | TBD |
| DEBT-007 | 2026-02-28 | `concat()` builtin has register collision | `lowerer.rs` | Workaround: use `+` operator | concat() works with local vars | TBD |

---

## Debt Metrics

**Current Status:**
- 🔴 Critical: **0 items** 🎉
- 🟡 Warning: 4 items
- 🟢 Acceptable: 2 items
- ⚪ Icebox: 5 items
- **Total Active:** 11 items
- **Paid Off:** **18 items (62% completion rate!)** 🚀

---

*Last updated: March 3, 2026*
