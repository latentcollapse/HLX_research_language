# HLX Builtin Library Conversion Plan
## Complete Rust → HLX Migration Strategy

**Total Scope**: 180+ builtin functions
**Total Timeline**: ~14 weeks
**Cost Optimization**: Haiku handles 75-80%, Sonnet handles 20-25%

---

## PHASE ALLOCATION BY MODEL

### ✅ HAIKU PHASES (10-11 weeks) - ~$0-150 cost
**Haiku will handle these phases autonomously.**
Haiku should explicitly indicate completion with:
```
✅ PHASE [N] COMPLETE - Ready for next phase.
```

---

## HAIKU PHASE 1: Type Introspection (Week 1)

**Functions to implement**: 2
- `type(value) -> String`
- `len(array/string/object) -> i64`

**File**: `/hlx_bootstrap/tier1_introspection.hlx`

**Deliverable**:
- [ ] Both functions implemented in HLX
- [ ] Test file: `test_tier1_introspection.hlx` (10+ test cases)
- [ ] Documentation with examples
- [ ] All tests passing

**Dependencies**: None (foundational)

**When done, Haiku outputs**:
```
✅ PHASE 1 COMPLETE - Type introspection functions implemented.
Proceeding to Phase 2: Type Conversions.
```

---

## HAIKU PHASE 2: Type Conversions (Week 1-2)

**Functions to implement**: 5
- `to_int(value) -> i64`
- `to_string(value) -> String`
- `bool(i64) -> i64` (truthiness)
- `parse_int(string) -> i64` (with validation)
- `parse_float(string) -> f64` (if float support available)

**File**: `/hlx_bootstrap/tier1_conversions.hlx`

**Deliverable**:
- [ ] All 5 functions implemented
- [ ] Error handling for invalid inputs
- [ ] Test file: `test_tier1_conversions.hlx` (20+ test cases)
- [ ] 100% test pass rate

**Dependencies**: Phase 1 (type introspection)

**When done, Haiku outputs**:
```
✅ PHASE 2 COMPLETE - Type conversion functions implemented.
Proceeding to Phase 3: Pure Math Operations.
```

---

## HAIKU PHASE 3: Pure Math Operations (Week 2-3)

**Functions to implement**: 8
- `abs(x) -> i64`
- `min(a, b) -> i64`
- `max(a, b) -> i64`
- `clamp(x, min, max) -> i64`
- `sign(x) -> i64`
- `lerp(a, b, t) -> i64`
- `gcd(a, b) -> i64`
- `lcm(a, b) -> i64`

**File**: `/hlx_bootstrap/tier3_math.hlx`

**Deliverable**:
- [ ] All 8 functions implemented
- [ ] GCD/LCM use Euclidean algorithm
- [ ] Test file: `test_tier3_math.hlx` (30+ test cases)
- [ ] Performance verification against Rust baseline
- [ ] Edge case handling (negative numbers, zero, etc.)

**Dependencies**: Phase 1-2

**Note**: Transcendental functions (sin, cos, sqrt, exp, log) **STAY IN RUST** (LLVMIntrinsic)

**When done, Haiku outputs**:
```
✅ PHASE 3 COMPLETE - Pure math functions implemented (8/8).
Documenting decision: Transcendentals kept in Rust for performance.
Proceeding to Phase 4: Bit Operations.
```

---

## HAIKU PHASE 4: Bit Operations (Week 3-4)

**Functions to implement**: 8
- `bit_and(a, b) -> i64`
- `bit_or(a, b) -> i64`
- `bit_xor(a, b) -> i64`
- `bit_not(a) -> i64`
- `bit_shl(a, n) -> i64`
- `bit_shr(a, n) -> i64`
- `bit_count(a) -> i64` (population count)
- `bit_reverse(a) -> i64`

**File**: `/hlx_bootstrap/tier1_bitops.hlx`

**Deliverable**:
- [ ] All 8 functions implemented
- [ ] Bit_count uses efficient loop algorithm
- [ ] Bit_reverse correctly handles all bit positions
- [ ] Test file: `test_tier1_bitops.hlx` (40+ test cases)
- [ ] Edge cases: zero, negative numbers, max i64 value

**Dependencies**: Phase 1-2

**When done, Haiku outputs**:
```
✅ PHASE 4 COMPLETE - Bit operation functions implemented (8/8).
Proceeding to Phase 5: String Utilities.
```

---

## HAIKU PHASE 5: String Utilities (Week 4-5)

**Functions to implement**: 9
- `repeat(s, count) -> String`
- `pad_left(s, width, char) -> String`
- `pad_right(s, width, char) -> String`
- `reverse_str(s) -> String`
- `is_alpha(s) -> i64`
- `is_numeric(s) -> i64`
- `is_alphanumeric(s) -> i64`
- `char_at(s, index) -> String`
- `char_code(s) -> i64` (first character code)

**File**: `/hlx_bootstrap/tier4_string_utils.hlx`

**Deliverable**:
- [ ] All 9 functions implemented
- [ ] Character classification uses existing tier2 functions
- [ ] Padding handles edge cases (width < length)
- [ ] Test file: `test_tier4_string_utils.hlx` (50+ test cases)
- [ ] Unicode handling documented (ASCII-only assumption noted)

**Dependencies**: Phase 1-2, Tier2 string ops (already exist)

**When done, Haiku outputs**:
```
✅ PHASE 5 COMPLETE - String utility functions implemented (9/9).
Proceeding to Phase 6: Array Operations (Sorting/Flattening).
```

---

## 🛑 PHASE 6: COMPLEX ARRAY ALGORITHMS - **HAIKU STOPS HERE**

> **THIS IS THE HANDOFF POINT** ⚠️
>
> Phase 6 requires algorithmic complexity and design decisions that need **SONNET**.
>
> **Call back to Sonnet for Phase 6:**
>
> Haiku will output:
> ```
> ❌ PHASE 6: COMPLEX ALGORITHMS DETECTED
>
> Phases 6 requires specialized algorithm design:
> - Sorting algorithm selection (bubble sort vs quicksort)
> - Recursive flattening depth control
> - Performance vs simplicity tradeoffs
>
> **Switching to Sonnet for Phase 6 completion.**
>
> Haiku will resume at Phase 7 when Sonnet completes Phase 6.
> ```

---

## 🔄 SONNET PHASE 6: Array Algorithms (Week 5-6)

**Functions to implement**: 10+
- `reverse(arr) -> [i64]`
- `sort(arr) -> [i64]` (bubble sort or better)
- `unique(arr) -> [i64]`
- `flatten(arr) -> [i64]` (one level)
- `flatten_deep(arr, depth) -> [i64]` (recursive)
- `chunk(arr, size) -> [[i64]]`
- `zip(arr1, arr2) -> [(i64, i64)]`
- `unzip(arr) -> (arr1, arr2)`
- Performance optimization decisions
- Edge case handling (empty arrays, type mismatches)

**File**: `/hlx_bootstrap/tier5_arrays.hlx`

**Deliverable**:
- [ ] All functions implemented with algorithm justification
- [ ] Test file: `test_tier5_arrays.hlx` (60+ test cases)
- [ ] Performance comparison vs Rust baseline
- [ ] Documentation of algorithm choices

**Dependencies**: Phase 1-5 complete

---

## ✅ HAIKU RESUMES: Phase 7 (Week 6-7)

**Haiku re-enters here after Sonnet completes Phase 6.**

Haiku should output:
```
✅ SONNET HANDOFF RECEIVED - Phase 6 complete.
Resuming Haiku work on Phase 7: Object Operations.
```

---

## HAIKU PHASE 7: Object/Map Operations (Week 6-7)

**Functions to implement**: 9
- `keys(obj) -> [String]`
- `values(obj) -> [Any]`
- `entries(obj) -> [(String, Any)]`
- `from_entries(entries) -> Object`
- `merge(obj1, obj2) -> Object`
- `pick(obj, keys) -> Object`
- `omit(obj, keys) -> Object`
- `map_keys(obj, fn) -> Object` (if HOF available)
- `map_values(obj, fn) -> Object` (if HOF available)

**File**: `/hlx_bootstrap/tier5_objects.hlx`

**Deliverable**:
- [ ] All 9 functions implemented
- [ ] Test file: `test_tier5_objects.hlx` (40+ test cases)
- [ ] Merge handles nested objects correctly

**Dependencies**: Phase 1-5

**When done, Haiku outputs**:
```
✅ PHASE 7 COMPLETE - Object operations implemented (9/9).
Proceeding to Phase 8: Encoding Operations.
```

---

## HAIKU PHASE 8: Encoding/URL (Week 7-8)

**Functions to implement**: 4
- `url_encode(s) -> String`
- `url_decode(s) -> String`
- `hex_encode(data) -> String`
- `hex_decode(s) -> String`

**File**: `/hlx_bootstrap/tier6_encoding.hlx`

**Deliverable**:
- [ ] All 4 functions implemented
- [ ] URL encoding handles special characters correctly
- [ ] Hex conversion handles case insensitivity
- [ ] Test file: `test_tier6_encoding.hlx` (40+ test cases)
- [ ] Edge cases: empty strings, non-ASCII characters

**Note**: Base64 encoding **MAY STAY IN RUST** (complex lookup table optimization)

**Dependencies**: Phase 1-5

**When done, Haiku outputs**:
```
✅ PHASE 8 COMPLETE - URL and Hex encoding implemented (4/4).
Proceeding to Phase 9: DateTime Operations.
```

---

## HAIKU PHASE 9: DateTime Operations (Week 8)

**Functions to implement**: 6
- `now() -> i64` (already in Rust - stays there)
- `format_timestamp(ts, format) -> String`
- `parse_timestamp(s, format) -> i64`
- `year(ts) -> i64`
- `month(ts) -> i64`
- `day(ts) -> i64`

**File**: `/hlx_bootstrap/tier6_datetime.hlx`

**Deliverable**:
- [ ] String formatting/parsing functions implemented
- [ ] Component extraction helper functions wrap Rust calls
- [ ] Test file: `test_tier6_datetime.hlx` (30+ test cases)
- [ ] Known timestamp values verified

**Dependencies**: Phase 1-5, Rust datetime helpers

**When done, Haiku outputs**:
```
✅ PHASE 9 COMPLETE - DateTime operations implemented (6/6).
Proceeding to Phase 10: I/O Operations.
```

---

## HAIKU PHASE 10: I/O Operations (Week 8-9)

**Functions to implement**: 7
- `read_file(path) -> String`
- `write_file(path, content) -> Null`
- `file_exists(path) -> i64`
- `delete_file(path) -> Null`
- `list_files(path) -> [String]`
- `create_dir(path) -> Null`
- `file_size(path) -> i64`

**File**: `/hlx_bootstrap/tier7_io.hlx`

**Deliverable**:
- [ ] All 7 functions implemented (mostly Rust wrappers)
- [ ] Error handling for file operations
- [ ] Test file: `test_tier7_io.hlx` (20+ test cases with temp files)
- [ ] Temp file cleanup after tests

**Dependencies**: Phase 1-5

**When done, Haiku outputs**:
```
✅ PHASE 10 COMPLETE - I/O operations implemented (7/7).
**All Haiku phases complete - Haiku work finished.**

Awaiting Sonnet for Phase 11-12 (JSON/HTTP/Advanced).
```

---

## 🛑 FINAL HANDOFF TO SONNET

> After Phase 10 complete, Haiku outputs:
> ```
> ═══════════════════════════════════════════════════════
> HAIKU WORK SUMMARY
> ═══════════════════════════════════════════════════════
> ✅ Phases 1-5:  Foundations (2+5+8+8+9 = 32 functions)
> ✅ Phase 7-10:  Core utilities (9+4+6+7 = 26 functions)
> ✅ TOTAL:       58 functions implemented in HLX
> ✅ Tier modules: tier1-tier7 builtins organized
>
> **READY FOR SONNET FINAL PHASES**
>
> Please switch to Sonnet for Phase 6 (Array algorithms - if not done)
> and Phase 11-12 (JSON/HTTP/Advanced features).
> ═══════════════════════════════════════════════════════
> ```

---

## 🔄 SONNET PHASE 11-12: JSON, HTTP, Advanced (Week 9-10)

**After Haiku finishes Phases 1-10:**

### Phase 11: JSON Operations (1 week)
- `json_parse(s) -> Object`
- `json_stringify(obj) -> String`
- `read_json(path) -> Object`
- `write_json(path, obj) -> Null`

### Phase 12: HTTP & Advanced (1 week)
- `http_request(config) -> Object`
- Advanced tensor operations
- Edge case handling
- Performance optimization

---

## FUNCTIONS NOT CONVERTED (Stay in Rust)

**Cryptography** (5 functions) - Security audit required
- `sha256`, `sha512`, `blake3`, `md5`, `hmac_sha256`

**Transcendental Math** (15+ functions) - Performance critical
- `sin`, `cos`, `tan`, `sqrt`, `pow`, `exp`, `log`, etc.

**Higher-order functions** - Blocked until first-class functions implemented
- `filter`, `map`, `reduce`, `find`, `some`, `every`

---

## QUALITY GATES AT EACH PHASE

Before marking complete, verify:
- ✅ All functions have unit tests (100% pass)
- ✅ Integration tests with previous tiers pass
- ✅ No performance regression > 10% vs Rust
- ✅ Edge cases documented and tested
- ✅ Code compiles without warnings

---

## SUMMARY TABLE

| Phase | Model | Functions | Files | Duration | Status |
|-------|-------|-----------|-------|----------|--------|
| 1 | Haiku | 2 | tier1_introspection.hlx | 1 week | Pending |
| 2 | Haiku | 5 | tier1_conversions.hlx | 1 week | Pending |
| 3 | Haiku | 8 | tier3_math.hlx | 1 week | Pending |
| 4 | Haiku | 8 | tier1_bitops.hlx | 1 week | Pending |
| 5 | Haiku | 9 | tier4_string_utils.hlx | 1 week | Pending |
| 6 | **SONNET** | 10+ | tier5_arrays.hlx | 1-2 weeks | **HANDOFF** |
| 7 | Haiku | 9 | tier5_objects.hlx | 1 week | Pending (after 6) |
| 8 | Haiku | 4 | tier6_encoding.hlx | 1 week | Pending |
| 9 | Haiku | 6 | tier6_datetime.hlx | 1 week | Pending |
| 10 | Haiku | 7 | tier7_io.hlx | 1 week | Pending |
| 11-12 | **SONNET** | 5+ | tier8_advanced.hlx | 2 weeks | **FINAL HANDOFF** |
| **TOTAL** | - | **58** in HLX + **15** in Rust | - | **14 weeks** | - |

---

## KEY HANDOFF MESSAGES

**Haiku → Sonnet (Phase 6)**:
```
❌ PHASE 6: COMPLEX ALGORITHMS DETECTED
Haiku switching to Sonnet for Phase 6 (sorting, flattening, etc).
Awaiting Sonnet completion before resuming Phase 7.
```

**Sonnet → Haiku (After Phase 6)**:
```
✅ PHASE 6 COMPLETE (Sonnet)
Switching back to Haiku for Phases 7-10.
```

**Haiku → Final (End of Phase 10)**:
```
✅ HAIKU PHASES COMPLETE (1-5, 7-10)
Switching to Sonnet for Phase 11-12 (JSON/HTTP/Advanced).
```

**Sonnet → Complete**:
```
✅ ALL PHASES COMPLETE
Builtin library conversion finished.
180+ functions: 58 in HLX, 15 in Rust, 107+ architectural patterns established.
```

---

## Notes for Implementation

1. **Test files** should be in same tier as functions
2. **Dependencies** tracked in comments at top of each file
3. **Error handling** documented with examples
4. **Performance** checked against Rust baseline for critical functions
5. **Determinism** verified (same input → same output across 10 runs)
