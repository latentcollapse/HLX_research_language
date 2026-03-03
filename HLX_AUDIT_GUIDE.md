# HLX Language Audit & Development Guide

**Purpose:** Systematic checklist for auditing HLX implementations and tracking gaps to production readiness.

---

## Part 1: Standard Language Components (Every Language Needs These)

### 1.1 Lexical Analysis (Tokenizer)
- [x] **Identifiers** (alphanumeric + underscore, not starting with digit)
- [x] **Keywords** (fn, let, if, else, return, loop, etc.)
- [x] **Literals**
  - [x] Integers (i64)
  - [x] Floats (f64)
  - [x] Strings (double-quoted, escape sequences: \n, \t, \r, \\, \", \0)
  - [x] Raw strings (r"..." - no escape processing)
  - [x] Multi-line strings ("""...""")
  - [x] Booleans (true/false)
- [x] **Operators** (+, -, *, /, %, ==, !=, <, >, <=, >=, &&, ||, !)
- [x] **Delimiters** (parentheses, braces, brackets, semicolons, commas)
- [x] **Comments** (// line, /* block */)
- [x] **Doc comments** (/// for documentation)

**HLX Status:** COMPLETE! Full lexer with escape sequences, raw strings, multi-line strings, and doc comments.

### 1.2 Syntax Analysis (Parser)
- [x] **Expressions**
  - [x] Binary operations (arithmetic, comparison, logical)
  - [x] Unary operations (negation, logical not)
  - [x] Function calls
  - [x] Array literals
  - [x] Array indexing (arr[idx])
  - [x] Field access (obj.field)
  - [x] Method calls (obj.method()) - FIXED by Kilo
  - [x] Ternary operator (cond ? then : else) - FIXED by Kilo
  - [ ] Match/switch expressions (switch/case exists, match not lowered)
- [x] **Statements**
  - [x] Variable declarations (let)
  - [x] Assignments (=)
  - [x] Compound assignments (+=, -=, *=, /=, %=) - FIXED 2026-03-02 (lexer had no compound tokens)
  - [x] If/else (expression form, returns value)
  - [x] Return
  - [x] Expression statements
  - [x] Loop (while-style with max iterations)
  - [x] For loops (for x in iterable) - NOW ADDED
  - [x] Break/continue - NOW WORKING
  - [ ] Block expressions (evaluate to value)
- [x] **Top-level Constructs**
  - [x] Function definitions (fn)
  - [x] Module-level variables (let at module scope)
  - [x] Export keyword
  - [x] Modules with imports
  - [x] Recursive agents
  - [ ] Struct/record definitions
  - [ ] Enums
  - [ ] Traits/interfaces
  - [ ] Generics

**HLX Status:** Core constructs solid. Missing method calls, for loops, break/continue, structs, enums.

### 1.3 Semantic Analysis
- [x] **Name Resolution**
  - [x] Scopes (block, function, module)
  - [x] Variable lookup (local → module → builtin)
  - [x] Function resolution
  - [x] Import resolution
  - [ ] Shadowing rules (currently allowed, may want warnings)
- [x] **Type Checking** (partial - inference + basic checking)
  - [x] Type annotations on variables
  - [x] Type annotations on function params/returns
  - [x] Array element types
  - [x] Dictionary/map types
  - [ ] Generic type checking
  - [ ] Type coercion rules (f64 + i64 currently?)
  - [ ] Exhaustive match checking
- [ ] **Control Flow Analysis**
  - [ ] Unreachable code detection
  - [ ] Return path checking (all branches return?)
  - [ ] Unused variable warnings
  - [ ] Unused import warnings

**HLX Status:** Basic type system working. Missing control flow analysis.

### 1.4 Code Generation (Lowering)
- [x] **Bytecode Generation**
  - [x] Opcode selection
  - [x] Register allocation (locals at 20+, params at 1-N)
  - [x] Constant pool management
  - [x] String pool management
  - [x] Jump target resolution
  - [x] Forward reference patching (function calls)
- [x] **Variable Lowering**
  - [x] Local variables (registers)
  - [x] Module-level variables (latent states)
  - [x] Function parameters
  - [x] Lambdas/closures - FIXED 2026-03-02 (|params| body, || body, Value::Function, CallDyn opcode)
- [x] **Expression Lowering**
  - [x] Binary ops
  - [x] Unary ops
  - [x] Function calls (direct and builtin)
  - [x] Array indexing (read)
  - [x] Array indexing (write) - NOW FIXED
  - [x] Field access (read)
  - [x] Field access (write) - NOW FIXED
  - [x] Short-circuit evaluation (&&, ||) - NOW FIXED (lazy evaluation)
  - [x] Ternary conditional (cond ? then : else) - NOW ADDED
  - [x] Source line mapping - NOW FIXED (errors show line numbers)
  - [x] Array indexing (read)
  - [x] Array indexing (write) - NOW FIXED
  - [x] Field access (read)
  - [x] Field access (write) - NOW FIXED
  - [x] Short-circuit evaluation (&&, ||) - NOW FIXED (lazy evaluation)
  - [x] Source line mapping - NOW FIXED (errors show line numbers)

**HLX Status:** Major fixes just landed (index/field assignment, register allocation, short-circuit eval, line numbers).

### 1.5 Runtime (VM)
- [x] **Execution Loop**
  - [x] Fetch-decode-execute
  - [x] Program counter management
  - [x] Step limit (safety)
  - [x] Max steps enforcement
- [x] **Memory Model**
  - [x] Register file (256 registers)
  - [x] Call stack
  - [x] Latent state storage (HashMap)
  - [x] Agent pool
  - [x] Memory management (copy-on-write for arrays/strings)
  - [ ] Garbage collection (currently copies, may leak on cycles?)
- [x] **Function Calling**
  - [x] Direct calls
  - [x] Builtin/native calls
  - [x] Register saving/restoring
  - [ ] Tail call optimization
  - [ ] Variable argument functions
- [x] **Error Handling**
  - [x] Runtime error messages
  - [x] Line number mapping (bytecode ↔ source) - NOW FIXED
  - [x] Stack traces (function call chain in error messages) - FIXED by Opus
  - [ ] Panic recovery

**HLX Status:** VM solid. Missing stack traces, line numbers, and error recovery.

### 1.6 Standard Library
- [x] **String Operations**
  - [x] concat, strlen, substring
  - [x] str_starts_with, str_contains
  - [x] i64_to_str, f64_to_str
  - [x] str_split, str_trim, str_replace
  - [x] str_char_at
  - [ ] Regex support
- [x] **Array Operations**
  - [x] len, push, pop
  - [x] get_at, set_at
  - [x] array_slice, array_concat
  - [x] sort - NOW ADDED
  - [x] map/filter/reduce (higher-order functions) - FIXED 2026-03-02 (lambdas enable these as HLX functions)
- [x] **Math Operations**
  - [x] Basic arithmetic
  - [x] abs, floor, ceil, round
  - [x] min, max, pow, sqrt
  - [x] rand, rand_range
  - [x] Trig functions (sin, cos, tan) - NOW ADDED
- [x] **I/O Operations**
  - [x] print, println
  - [x] File read/write (read_file, write_file, file_read, file_write)
  - [ ] Network I/O
- [x] **Time Operations**
  - [x] current_time (alias for clock_ms)
  - [x] sleep(ms) - NOW ADDED
- [x] **Development**
  - [x] assert(condition, message) - NOW ADDED
  - [x] shell(cmd) - NOW ADDED

**HLX Status:** Good coverage for core use case. Missing higher-order functions, file I/O.

---

## Part 2: HLX-Specific Components (Unique Requirements)

### 2.1 Agent System
- [x] **Agent Lifecycle**
  - [x] Agent spawning
  - [x] Latent variable declaration per agent
  - [x] Cycle execution (H, L, R scales)
  - [x] Cycle counting/limits
  - [ ] Agent termination/cleanup
- [x] **Governance Integration**
  - [x] Governance policy loading
  - [x] Effect classification (read, modify, execute)
  - [x] Conscience predicates (no_harm, path_safety, etc.)
  - [ ] Dynamic governance updates
- [ ] **Scale Operations**
  - [x] Scale declarations
  - [ ] Scale migration
  - [ ] Cross-scale communication

**HLX Status:** Core agent system working. Scale features partially implemented.

### 2.2 Tensor Operations
- [x] **Tensor Types**
  - [x] Fixed-size tensors (Tensor[N])
  - [x] zeros() builtin
  - [x] tensor_add, tensor_blend, tensor_norm
  - [ ] Dynamic tensors
  - [ ] GPU acceleration
- [x] **Tensor Storage**
  - [x] Dense storage
  - [ ] Sparse storage

**HLX Status:** Basic tensor ops for embeddings. Not a full tensor library.

### 2.3 Memory System
- [x] **HIL Memory**
  - [x] mem_store builtin
  - [x] mem_query builtin
  - [x] SQLite persistence
  - [ ] Vector similarity search (uses mem_query currently)
- [x] **Pattern System**
  - [x] Pattern struct
  - [x] Pattern matching (find_matching_patterns)
  - [ ] Pattern eviction strategies

**HLX Status:** Core memory system functional.

### 2.4 Bond Protocol
- [x] **Protocol Handshake**
  - [x] HELLO phase
  - [x] SYNC phase
  - [x] BOND phase
  - [x] READY phase
- [x] **Integration**
  - [x] Native bond() builtin
  - [x] LLM endpoint configuration
  - [ ] Streaming responses
  - [ ] Retry logic

**HLX Status:** Bond protocol implemented but stubbed without actual LLM.

---

## Part 3: Current Audit Results

### What's Working (Solid)
- ✅ Core language (expressions, statements, functions)
- ✅ Type system (basic annotations and inference)
- ✅ Module system (imports, exports)
- ✅ Bytecode compilation and execution
- ✅ Register allocation (NOW FIXED)
- ✅ Latent variable system (module-level state)
- ✅ Agent system (cycles, governance)
- ✅ Built-in functions (comprehensive coverage)
- ✅ Memory persistence (SQLite)

### What Has Issues (Recently Fixed)
- ✅ **Register collision** - Fixed by proper param/local separation
- ✅ **Array assignment** - Fixed (was a no-op)
- ✅ **Field assignment** - Fixed for both local and latent
- ✅ **Saved registers** - Fixed (increased to 150)
- ✅ **Auto-initialization** - Fixed for arrays, strings, dicts

### What's Missing (Gaps to Fill)
- ✅ **Source line numbers** - FIXED! Errors now show "at line N"
- ✅ **current_time builtin** - FIXED! Added as clock_ms alias
- ✅ **shell builtin** - FIXED! Bitsy can execute bash commands
- ✅ **Stack traces** - FIXED! Function call chain in error messages
- ✅ **Closures/lambdas** - FIXED! `|x| x * 2`, `|| expr`, CallDyn opcode
- ✅ **Compound assignment** - FIXED! `+=`, `-=`, `*=`, `/=`, `%=` (was silent no-op)
- ✅ **Method syntax** - FIXED! `obj.method()` transforms to `method(obj, args)`
- ✅ **For loops** - FIXED! `for x in arr { }` with reserved registers 240-243
- ✅ **Ternary operator** - FIXED! `cond ? then : else`
- ✅ **File I/O** - FIXED! `file_read`, `file_write` builtins
- ❌ **Debug info** - No way to inspect variables at runtime (--debug traces opcodes but not named vars)
- ❌ **Error recovery** - No try/catch or Result type
- ❌ **Generics** - Arrays are generic but user types aren't
- ❌ **Better parse errors** - No `^^^` pointer to error location

### HLX-Unique Gaps
- ❌ **Scale migration** - Declared but not fully implemented
- ❌ **Dynamic governance** - Policy updates at runtime
- ❌ **Cross-agent messaging** - Agents operate in isolation
- ❌ **Vector search** - mem_query does string matching, not vector similarity

---

## Part 4: Roadmap to Production

### Phase 1: Critical Fixes (Before Any Deployment)
**Goal:** Prevent debugging nightmares in production

1. **Add source line mapping to bytecode**
   - Store line number per instruction
   - Include in error messages
   - Effort: 1-2 days

2. **Add current_time builtin**
   - Needed for bit.hlx observe path
   - Effort: 2 hours

3. **Add comprehensive error context**
   - Function name in errors
   - Parameter values (optional, debug mode)
   - Call stack (last 5 frames)
   - Effort: 1-2 days

4. **Fix short-circuit evaluation**
   - Verify && and || don't evaluate both sides
   - Effort: 4 hours

### Phase 2: Developer Experience (Before External Users)
**Goal:** Make HLX pleasant to use

1. **Add --debug flag to hlx-run**
   - Print each executed instruction
   - Show register states
   - Show latent state changes
   - Effort: 1 day

2. **Add assert() builtin**
   - Assert with message
   - Halt on failure in debug mode
   - Effort: 2 hours

3. **Add type_of() improvements**
   - Return structured type info
   - Include generic parameters
   - Effort: 4 hours

4. **Better parse errors**
   - Point to error location with ^^^
   - Suggest fixes
   - Effort: 2-3 days

### Phase 3: Production Robustness (Before Serious Use)
**Goal:** Handle edge cases gracefully

1. **Add memory limits**
   - Max array size
   - Max string size
   - Max recursion depth
   - Effort: 1 day

2. **Add timeout handling**
   - Wall-clock timeout (not just step count)
   - Interruptible execution
   - Effort: 1-2 days

3. **Add signal handling**
   - Ctrl+C graceful shutdown
   - Cleanup on exit
   - Effort: 1 day

4. **Add logging framework**
   - Log levels (ERROR, WARN, INFO, DEBUG)
   - Structured logging
   - Effort: 1-2 days

### Phase 4: Platform Support (Windows/Mac)
**Goal:** Run on all major platforms

1. **Windows Support**
   - [ ] Test file paths (backslash vs forward slash)
   - [ ] Test SQLite paths
   - [ ] ANSI color codes in terminal
   - [ ] Signal handling differences
   - Effort: 2-3 days

2. **macOS Support**
   - [ ] Test on Apple Silicon (ARM64)
   - [ ] Check SQLite compatibility
   - [ ] Test terminal colors
   - Effort: 1-2 days

3. **Build System**
   - [ ] Cargo.toml platform dependencies
   - [ ] CI/CD for all platforms
   - [ ] Release binaries
   - Effort: 2-3 days

### Phase 5: HLX-Specific Features (For Bitsy Evolution)
**Goal:** Make Bitsy truly powerful

1. **Vector similarity search**
   - Add vector index to SQLite
   - Use cosine similarity in mem_query
   - Effort: 3-5 days

2. **Scale migration**
   - Implement migrate keyword
   - State transfer between scales
   - Effort: 1 week

3. **Dynamic governance**
   - Hot-reload policy.axm
   - Runtime governance updates
   - Effort: 3-4 days

4. **Cross-agent messaging**
   - Message passing between agents
   - Mailbox system
   - Effort: 1 week

---

## Part 5: Audit Checklist (Use This for Future Features)

When adding any new feature to HLX, check:

### Parser
- [ ] Can it be parsed unambiguously?
- [ ] Does it conflict with existing syntax?
- [ ] Are error messages clear when parsing fails?

### Lowerer
- [ ] Are all registers properly allocated?
- [ ] Do parameters and locals not collide?
- [ ] Is the destination register separate from sources?
- [ ] Are constants added to the constant pool correctly?
- [ ] Are strings added to the string pool correctly?

### VM
- [ ] Does the opcode handler read the correct number of bytes?
- [ ] Are bounds checked on all memory accesses?
- [ ] Are error messages actionable?
- [ ] Does it handle Nil gracefully?
- [ ] Does it work with both local and latent variables?

### Integration
- [ ] Does it work in --func mode?
- [ ] Does it work when called from another function?
- [ ] Does it preserve state correctly?
- [ ] Does it work with the TUI/bridge?

### Testing
- [ ] Unit tests for the feature
- [ ] Integration tests with bit.hlx
- [ ] Edge case tests (empty, nil, max values)
- [ ] Error case tests (wrong types, out of bounds)

---

## Summary

**HLX is currently at:** Late beta — MVP milestone reached

**Can use for:** Development, internal testing, Bitsy v0.1, writing real programs

**Don't use for:** Production systems, external users, mission-critical tasks

**Phase 1 (Critical Fixes):** ✅ COMPLETE — line numbers, stack traces, short-circuit eval, builtins
**Phase 2 (Dev Experience):** ✅ COMPLETE — --debug flag, assert(), file I/O, shell(), memory limits, timeout
**Phase 3 (Robustness):** ⏳ In progress — memory limits done, signal handling + logging pending → v0.5
**Phase 4 (Platform):** ⏳ Not started — Windows/Mac testing → v1.0
**Phase 5 (HLX-Specific):** ⏳ Not started — vector search, scale migration, dynamic governance, cross-agent messaging → v2.0

**Language feature completeness:** ~85% of standard language features implemented
**Remaining gaps:** match lowering, generics, error recovery, better parse errors, named variable inspection in debug

---

*Document created: March 2, 2026*
*Last updated: March 2, 2026 — after Session 2 (Opus: lambdas, compound assignment fix)*
*Authors: Kilo, Opus, Matt*
