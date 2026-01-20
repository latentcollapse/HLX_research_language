# Bounded Recursion Robustness Status

## ✅ Phase 7 Complete: Core Implementation Hardened

### What's Now Working
1. **FuncDef max_depth Storage** - Carries recursion depth through bytecode
2. **Reversibility Fixed** - Lifting reconstructs #[max_depth(N)] attributes
3. **Runtime Enforcement** - Depth limits properly checked during execution
4. **Comprehensive Testing** - 6/6 hardened tests passing

### Axioms Preserved
- ✅ **A1 (Determinism)**: Same recursion patterns always enforce same limits
- ✅ **A2 (Reversibility)**: FIXED - max_depth now round-trips through compilation
- ✅ **A3 (Bijection)**: Different max_depth values produce different bytecode
- ✅ **A4 (Universal Value)**: Depth limits independent of context

---

## 🔄 Phase 8: Bootstrap Integration (Next Priority)

### What Still Needs Implementation

#### 8.1 Parser Support (hlx_bootstrap/parser.hlx)
**Status**: NOT IMPLEMENTED

Needs to parse `#[max_depth(N)]` attribute syntax:
```hlx
#[max_depth(50)]
fn fibonacci(n: i64) -> i64 { ... }
```

Changes needed:
- [ ] Add token recognition for `[`, `]` in attribute position
- [ ] Parse attribute into list of strings
- [ ] Store attributes in Block AST node
- [ ] Validate max_depth is numeric

Estimated: 30-50 lines in parser.hlx

#### 8.2 Semantic Validation (hlx_bootstrap/semantic_complete.hlx)
**Status**: NOT IMPLEMENTED

Needs to validate max_depth attributes:
```hlx
// Validate max_depth is positive integer
// Warn if max_depth seems too small for recursion depth
// Check for inconsistent mutual recursion limits
```

Changes needed:
- [ ] Add max_depth validation function
- [ ] Extract and parse max_depth from attributes
- [ ] Validate range (1-2^32)
- [ ] Report validation errors

Estimated: 40-60 lines in semantic_complete.hlx

#### 8.3 Lowering (hlx_bootstrap/lower.hlx)
**Status**: NOT IMPLEMENTED

Currently uses Rust lowering. Bootstrap version needs:
```hlx
// Extract max_depth from block attributes
// Use extract_max_depth() helper
// Emit it in FuncDef instructions
```

Changes needed:
- [ ] Add extract_max_depth() helper function
- [ ] Track function_depths HashMap
- [ ] Apply max_depth when lowering Call instructions
- [ ] Set max_depth in FuncDef lowering

Estimated: 50-80 lines in lower.hlx

---

## 🧪 Validation Requirements

### Edge Cases to Handle

#### Case 1: max_depth = 0
```hlx
#[max_depth(0)]
fn never_recurses(n) { return never_recurses(n - 1); }
// Should error immediately on first recursive call
```

#### Case 2: max_depth = 1
```hlx
#[max_depth(1)]
fn single_call(n) {
  if n <= 0 { return 0; }
  return single_call(n - 1);  // ERROR: exceeds depth 1
}
```

#### Case 3: Mutual Recursion Conflict
```hlx
#[max_depth(100)]
fn is_even(n) { return is_odd(n - 1); }

#[max_depth(10)]
fn is_odd(n) { return is_even(n - 1); }
// Calling is_even with depth 100 but is_odd limits to 10
// This is OK - each function has its own limit
```

#### Case 4: Invalid Attribute Values
```hlx
#[max_depth(-5)]        // ERROR: negative
fn bad1() { ... }

#[max_depth(abc)]       // ERROR: not a number
fn bad2() { ... }

#[max_depth()]          // ERROR: missing value
fn bad3() { ... }
```

### Validation Tests Needed

```hlx
// File: test_bootstrap_recursion_validation.hlx

#[max_depth(0)]
fn zero_depth(n) { return zero_depth(n - 1); }

#[max_depth(1)]
fn unit_depth(n) { return unit_depth(n - 1); }

#[max_depth(50)]
fn mutual_a(n) { return mutual_b(n - 1); }

#[max_depth(50)]
fn mutual_b(n) { return mutual_a(n - 1); }

fn main() {
  // Test zero_depth with 0 calls - should work
  zero_depth(0);  // OK

  // Test unit_depth with 1 level
  unit_depth(1);  // ERROR: exceeds max_depth(1)

  // Test mutual recursion
  mutual_a(25);   // OK (25 < 50)

  return 0;
}
```

---

## 📊 Testing Coverage

### Current Tests ✅
- `test_recursion_depth.hlx` - Basic recursion patterns
- `test_recursion_exceed.hlx` - Runtime depth enforcement
- `test_recursion_hardened.hlx` - Comprehensive 6-test suite

### Needed Tests ❌
- **Bootstrap Parser**: Attribute parsing for `#[max_depth(N)]`
- **Bootstrap Semantic**: Validation of max_depth values
- **Bootstrap Lowering**: max_depth extraction and emission
- **Edge Cases**: max_depth=0, max_depth=1, invalid values
- **Round-trip**: Bootstrap → bytecode → lift back to source

---

## 🎯 Implementation Order

**Phase 8a** (Optional but recommended):
1. Add max_depth validation to semantic analyzer
2. Update bootstrap parser to handle attributes
3. Create validation error tests

**Phase 8b** (Enables self-hosting):
4. Implement max_depth in bootstrap lowering
5. Test bootstrap compiler round-trip compilation
6. Verify bytecode matches Rust compiler

**Phase 8c** (Polish):
7. Add comprehensive error messages
8. Document max_depth best practices
9. Create migration guide for users

---

## 🔐 Robustness Checklist

- [x] FuncDef carries max_depth through bytecode
- [x] Lifting reconstructs max_depth attributes
- [x] Runtime enforces depth limits
- [x] Reversibility preserved (Axiom 2)
- [ ] Bootstrap parser handles attributes
- [ ] Bootstrap semantic validates max_depth
- [ ] Bootstrap lowering emits max_depth
- [ ] Edge cases documented and handled
- [ ] Error messages are clear
- [ ] Bootstrap → Rust compiler output matches

---

## 📝 Notes

- Default max_depth = 1000 (matches typical stack depths)
- Mutual recursion each function has independent limit
- Tail recursion still counts toward depth (maintains determinism)
- No special-casing for any recursion pattern
- Error format: `"Recursion depth for function 'X' exceeded (max: Y)"`

---

## 🚀 Self-Hosting Readiness

**Current Status**: Ready for Phase 8
- Rust compiler ✅ Fully implements bounded recursion
- Bytecode format ✅ Stores and preserves max_depth
- Runtime ✅ Enforces limits correctly

**Blocker for Self-Hosting**: Bootstrap modules need Phase 8 implementation
- Without bootstrap support, can't compile HLX programs with recursion
- Phase 8 enables true self-hosting with recursion support
