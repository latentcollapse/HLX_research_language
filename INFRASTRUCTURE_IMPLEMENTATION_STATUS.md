# HLX Infrastructure Stabilization - Implementation Status

**Date Started**: 2026-01-19
**Current Focus**: Phase 1 - Contract Syntax & Types
**Critical Path**: Phases 1, 2, 4, 6

---

## ✅ COMPLETED PHASES

### Phase 1: Contract Syntax & Types (DONE)

#### 1.1 Lexer Extensions ✅
- Added `TOKEN_AT` (kind 60) for `@field_id` syntax
- Added keywords `collapse` (kind 61) and `resolve` (kind 62)
- File: `/home/matt/hlx-compiler/hlx/hlx_bootstrap/lexer.hlx`
- Changes:
  - Added `@` character handling in single-character token section
  - Extended `keyword_kind()` to recognize collapse/resolve
  - Updated token kind documentation

#### 1.2 Parser Extensions ✅
- Added AST node types: `EXPR_CONTRACT` (9), `EXPR_CONTRACT_FIELD` (25), `EXPR_FIELD_ACCESS` (26), `EXPR_COLLAPSE` (27), `EXPR_RESOLVE` (28)
- File: `/home/matt/hlx-compiler/hlx/hlx_bootstrap/parser.hlx`
- Changes:
  - Added token kind constants for `@`, `collapse`, `resolve`
  - Added helper functions: `make_expr_contract()`, `make_expr_contract_field()`, `make_expr_field_access()`, `make_expr_collapse()`, `make_expr_resolve()`
  - Extended `parse_primary_expr()` with:
    - Contract literal parsing: `{contract_id:{@field_id:value, ...}}`
    - Field access parsing: `.@field_id`
    - collapse/resolve function call parsing
  - Properly handles ambiguity between contract literals and code blocks via lookahead

#### 1.3 Semantic Analyzer Extensions ✅
- Added types: `TYPE_CONTRACT` (6), `TYPE_HANDLE` (7)
- File: `/home/matt/hlx-compiler/hlx/hlx_bootstrap/semantic_complete.hlx`
- Changes:
  - Extended `type_name()` to handle contract and handle types
  - Extended `infer_type()` with cases for EXPR_CONTRACT (9), EXPR_FIELD_ACCESS (26), EXPR_COLLAPSE (27), EXPR_RESOLVE (28)
  - Extended `analyze_expr()` with validation for contract and handle expressions
  - Analyzes field values recursively in contracts

#### 1.4 Lowerer Extensions ✅
- Added instruction types: `INST_CONTRACT_CREATE` (90), `INST_CONTRACT_GET` (91), `INST_CONTRACT_SET` (92), `INST_COLLAPSE` (93), `INST_RESOLVE` (94)
- File: `/home/matt/hlx-compiler/hlx/hlx_bootstrap/lower.hlx`
- Changes:
  - Added expression type constants for contracts
  - Extended `lower_expr()` switch statement with:
    - Case 9: CONTRACT literal lowering - allocates registers for fields, emits CONTRACT_CREATE
    - Case 26: FIELD_ACCESS lowering - emits CONTRACT_GET instruction
    - Case 27: COLLAPSE lowering - emits COLLAPSE instruction
    - Case 28: RESOLVE lowering - emits RESOLVE instruction

#### 1.5 Emitter Extensions ✅
- File: `/home/matt/hlx-compiler/hlx/hlx_bootstrap/emit.hlx`
- Changes:
  - Added instruction type constants for contracts
  - Extended `emit_instruction()` switch statement with:
    - Case 90: CONTRACT_CREATE - emits out register, contract ID, field count, field registers
    - Case 91: CONTRACT_GET - emits out register, object register, field ID
    - Case 92: CONTRACT_SET - emits object register, field ID, value register
    - Case 93: COLLAPSE - emits out register, value register
    - Case 94: RESOLVE - emits out register, handle register

#### 1.6 Testing (IN PROGRESS)
- Need to create test file: `/home/matt/hlx-compiler/hlx/tests/test_contracts.hlx`
- Test cases to implement:
  - Simple contract creation and field access
  - Nested contracts
  - Contracts in arrays
  - collapse/resolve operations
  - Type inference with contracts

---

## ⏳ PENDING PHASES

### Phase 2: Handle Operations (collapse/resolve) - PENDING

**Status**: Lexer/Parser support done in Phase 1; lowerer/emitter support done in Phase 1.4-1.5
**Remaining**:
1. Runtime support for handle table (collapse stores value, resolve retrieves)
2. Extended semantic analysis for handle types
3. Bijection verification (collapse/resolve are reversible)

**Impact**: Essential for Axiom A2 (Reversibility)

---

### Phase 3: HLX-R Runic Representation - PENDING

**Status**: Not started
**Goal**: Symbol-based representation for LLM efficiency
**Tasks**:
1. Create symbol mapping (⟠ = program, ◇ = fn, etc.)
2. Implement lexer_runic.hlx (Unicode symbol handling)
3. Implement emit_runic.hlx (AST → runic text)
4. Verify bijection: A ↔ R → same AST

**Impact**: Medium priority; needed for compression but not critical for bootstrap

---

### Phase 4: Native HLX Runtime - CRITICAL PATH ⚡

**Status**: Not started
**Goal**: Bytecode interpreter written in HLX, eliminates RustD dependency
**Scale**: ~1500 LOC, 40+ instruction handlers
**Tasks**:
1. Design VM architecture
   - State: [bytecode, pc, registers, call_stack, halted, return_value]
   - Use contract syntax for clean state modeling
2. Implement execution loop
   - Decode instruction from bytecode
   - Dispatch to handler
   - Update PC
3. Implement 40+ instruction handlers
   - Arithmetic: ADD, SUB, MUL, DIV, MOD
   - Comparison: EQ, NE, LT, LE, GT, GE
   - Logical: AND, OR
   - Bitwise: BIT_AND, BIT_OR, BIT_XOR, SHL, SHR
   - Control: IF, JUMP, LOOP, BREAK, CONTINUE
   - Functions: CALL, RETURN, FUNCDEF
   - Arrays: GET_ELEMENT, SET_ELEMENT
   - Constants: CONSTANT, MOVE
   - Contracts: CONTRACT_CREATE, CONTRACT_GET, CONTRACT_SET, COLLAPSE, RESOLVE
4. Implement syscall interface (file I/O)
5. Test with recursive programs, contracts, handles

**Critical for**: Bootstrap independence, enabling kernel boot

---

### Phase 5: Axiom Enforcement - PENDING

**Status**: Not started
**Tasks**:
1. A1 Determinism - Validate no banned operations (random, time, unbounded loops)
2. A2 Reversibility - Implement lifter (bytecode → AST), verify round-trip
3. A3 Bijection - Verify A ↔ R ↔ bytecode ↔ A perfect round-trip
4. A4 Universal Value - Validate no implicit coercions, all types explicit

**Impact**: Verification of language guarantees

---

### Phase 6: Axiom Kernel Boot - PENDING

**Status**: Axiom Kernel PoC exists, needs contract syntax updates
**Tasks**:
1. Update `/home/matt/hlx-compiler/axiom-kernel/boot.hlx` to use contract syntax
   - Define types as contracts instead of records
   - Use @field syntax for structure field access
2. Compile with native HLX toolchain (no RustD)
3. Test QEMU boot
4. Verify "HELINUX" output

**Success Criteria**:
- ✅ Compiler compiles itself
- ✅ Kernel compiles with contracts/handles
- ✅ VM executes kernel bytecode
- ✅ QEMU boots successfully
- ✅ All axioms verified

---

## ARCHITECTURE CHANGES NEEDED

### Runtime Support for Handles
Current: RustD handles file I/O via FFI
Needed: In-memory handle table in HLX VM
- HashMap<u64, Value> for storing collapsed values
- Generator for unique handle IDs
- Garbage collection or explicit cleanup

### Memory Management
Current: RustD manages memory
Needed: HLX runtime needs allocation strategy
- Register allocation already done in lowerer
- Stack frames for function calls
- No heap allocation needed for MVP

### Type System for Contracts
Current: Contracts are structural (contract_id + fields)
Enhancement: Add field schema validation
- Optional: Store field types in contract metadata
- For MVP: Assume all fields are i64

---

## KNOWN ISSUES & NOTES

1. **Parser ambiguity**: `{42:{@0:0}}` vs `{stmt1; stmt2}`
   - Solution: Lookahead for integer `:  pattern after `{`
   - ✅ Implemented

2. **Contract field ordering**: Must match definition order
   - Note: No named fields, only positional (@0, @1, etc.)
   - Fine for MVP; can enhance later

3. **Handle reversibility**: collapse/resolve are inverses
   - Requires: Handle table thread through VM state
   - Important for A2 validation

4. **Runic compression**: Unicode rendering may not work in all terminals
   - Fallback: Keep ASCII representation as primary
   - Enhancement: Add --emit-runic flag to compiler

5. **Axiom kernel**: Currently bare-metal x86_64 stub
   - Phase 4 VM enables testing in HLX directly
   - Phase 6 QEMU boot optional; can skip for now

---

## NEXT IMMEDIATE STEPS

### Phase 1 Completion (This Session)
1. ✅ Implement contract syntax in lexer/parser/semantic/lowerer/emitter
2. Create test file with contract examples
3. Compile test file with HLX compiler
4. Verify bytecode executes correctly in RustD

### Phase 4 FOCUS (Critical Path)
1. Analyze existing RustD executor as reference
2. Design VM state in HLX using contracts
3. Implement execution loop and instruction decoder
4. Implement core 20 instructions (arithmetic, comparison, control flow)
5. Add contract and handle instructions
6. Test with self-hosting compiler
7. Achieve: Compiler running on native HLX VM

### Phase 2 (Handle Runtime)
1. Add handle table to VM state
2. Implement collapse: allocate handle ID, store value
3. Implement resolve: lookup handle ID, return value
4. Test with handle creation/resolution

### Phase 6 (Boot Test)
1. Update kernel with contract syntax
2. Compile with native HLX toolchain
3. Run in QEMU emulator
4. Verify output

---

## TOTAL LOC IMPACT

**Phase 1**: ~500 LOC (lexer, parser, semantic, lowerer, emitter extensions)
**Phase 2**: ~200 LOC (handle runtime, semantic validation)
**Phase 3**: ~1000 LOC (lexer_runic, emit_runic, bijection tests)
**Phase 4**: ~1500 LOC (VM implementation, instruction handlers)
**Phase 5**: ~800 LOC (axiom validators, lifter)
**Phase 6**: ~100 LOC (kernel updates)

**Total**: ~4100 LOC across all phases

---

## SUCCESS METRICS

- [ ] Phase 1: Contract test compiles and runs
- [ ] Phase 2: Handle collapse/resolve works correctly
- [ ] Phase 3: Bijection tests pass for runic conversion
- [ ] Phase 4: Native VM executes simple programs without RustD
- [ ] Phase 5: All axiom validators detect violations correctly
- [ ] Phase 6: Axiom Kernel boots in QEMU

**ULTIMATE GOAL**: Axiom Kernel boots in actual HLX (not RustD), displaying "HELINUX"

---

Generated: 2026-01-19 by HLX Infrastructure Implementation Task
