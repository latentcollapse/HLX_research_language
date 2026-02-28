# HLX Development Plan
## Road to Writing Symbolic AI

**Author:** Matt (architect) + Claude Opus 4.6 (senior dev)
**Date:** Feb 28 2026
**Repo:** https://github.com/latentcollapse/hlx-compiler (`experimental` branch for dev, merge to `main` on milestones)

---

## Overview

HLX has solid bones. The VM is complete (all 52 opcodes implemented, no stubs), the RSI Rust machinery is working, the parser handles all keywords. What's missing is the **wiring** — keywords that parse but emit no-ops, gates that exist in Rust but can't be touched from HLX code, a module system that silently drops imports, and a developer toolchain that makes working in HLX something other than archaeology.

This plan completes HLX in five milestone phases, each building on the last, structured so that each phase delivers something demonstrably runnable.

**Goal at completion:** A Bit agent written in HLX that can propose a modification to itself, pass it through the governance gates, apply it, verify it improved, and run that loop in a way you can observe in real time — with LSP, debugger, and imports making the whole thing writable without pain.

---

## Phase 1 — Foundation: Source Positions, Builtins, and the Variable Limit
**Estimated effort: Small. Kilo should complete this in one session.**

Nothing else can be properly debugged without source positions. This phase is unglamorous but it unblocks everything that follows.

### 1.1 — Tokenizer line/column tracking

**File:** `hlx-runtime/src/ast_parser.rs` (tokenizer section)

**Problem:** `ParseError` has `line` and `col` fields but the tokenizer always emits `line: 0, col: 0`. Every error reports the wrong location.

**Fix:** Add a `line: usize` and `col: usize` counter to the tokenizer's character loop. Increment `line` on `'\n'`, reset `col` to 0. Increment `col` on every other character. When creating any token, attach the current `(line, col)` as its source position.

```
tokenize() currently:
  for each char in source {
      match char { ... produce Token }
  }

After fix:
  let mut line = 1; let mut col = 1;
  for each char {
      if char == '\n' { line += 1; col = 1; } else { col += 1; }
      match char { ... produce Token::WithSpan { token, line, col } }
  }
```

The `Token` enum should carry `(line, col)` as an attached `SourceSpan`. All `ParseError` construction sites in `parse_*` functions should use the current token's span instead of `(0, 0)`.

**Test:** A parse error on line 5 of a test file should report `line: 5`.

### 1.2 — AST node span population

**File:** `hlx-runtime/src/ast_parser.rs` (all `parse_*` functions)

**Problem:** `SourceSpan` struct exists in every AST node but is always `SourceSpan::unknown()`.

**Fix:** After the tokenizer carries `(line, col)` per token, every `parse_*` function should record `span_start` from the current token before parsing, `span_end` after parsing, and pass `SourceSpan { start_line, start_col, end_line, end_col }` into the resulting AST node constructor.

Priority order: `parse_fn`, `parse_let`, `parse_if`, `parse_loop`, `parse_agent`, `parse_expression`. Lower-priority: block-level nodes.

**Test:** A parsed function's `SourceSpan` should have `start_line` equal to the line of the `fn` keyword.

### 1.3 — Raise the variable limit

**File:** `hlx-runtime/src/lowerer.rs`

**Problem:** The lowerer has a hard cap of 19 named variables per scope (`MAX_VARS = 19` or similar). Any non-trivial HLX program hits this. Bit.hlx itself is close to the ceiling.

**Fix:** Raise the limit. The VM has 256 registers. Reserve registers 0–199 for local variables, 200–230 for temporaries, 231–255 for function argument passing (currently uses 150+). A limit of 200 named variables per scope is reasonable. Update the constant and any index arithmetic.

**Test:** A function with 25 local variables should compile and run without a `LowerError`.

### 1.4 — Missing builtins

**File:** `hlx-runtime/src/builtins.rs` and `hlx-runtime/src/vm.rs` (builtin dispatch)

**Problem:** The builtin set is missing basic math, array ops, map ops, and type conversion needed for symbolic reasoning programs.

**Add the following builtins:**

*Math:*
- `abs(x: i64|f64) -> i64|f64`
- `floor(x: f64) -> i64`
- `ceil(x: f64) -> i64`
- `round(x: f64) -> i64`
- `min(a, b) -> same`
- `max(a, b) -> same`
- `pow(base: f64, exp: f64) -> f64`
- `rand() -> f64` (0.0–1.0, use `rand` crate)
- `rand_range(lo: i64, hi: i64) -> i64`

*Type conversion:*
- `f64_to_i64(x: f64) -> i64`
- `i64_to_f64(x: i64) -> f64`
- `parse_i64(s: String) -> i64`
- `parse_f64(s: String) -> f64`
- `type_of(x: any) -> String` (returns `"i64"`, `"f64"`, `"string"`, `"bool"`, `"list"`, `"map"`, `"nil"`)

*String:*
- `str_split(s: String, delim: String) -> List<String>`
- `str_trim(s: String) -> String`
- `str_replace(s: String, from: String, to: String) -> String`
- `str_to_upper(s: String) -> String`
- `str_to_lower(s: String) -> String`
- `str_starts_with(s: String, prefix: String) -> bool`
- `str_ends_with(s: String, suffix: String) -> bool`
- `str_index_of(s: String, sub: String) -> i64` (-1 if not found)

*Array:*
- `array_slice(arr: List, start: i64, end: i64) -> List`
- `array_concat(a: List, b: List) -> List`
- `array_contains(arr: List, val: any) -> bool`
- `array_pop(arr: List) -> any` (removes and returns last element)
- `array_reverse(arr: List) -> List`
- `array_sort(arr: List) -> List` (sort by natural ordering)

*Map:*
- `map_get(m: Map, key: String) -> any`
- `map_set(m: Map, key: String, val: any) -> Map` (returns updated map)
- `map_keys(m: Map) -> List<String>`
- `map_values(m: Map) -> List`
- `map_contains(m: Map, key: String) -> bool`
- `map_remove(m: Map, key: String) -> Map`

*I/O:*
- `read_file(path: String) -> String`
- `write_file(path: String, content: String) -> bool`
- `clock_ms() -> i64` (milliseconds since epoch)

**Test:** Each builtin should have at least one test in a `.hlx` test file.

### ✅ Phase 1 Milestone
- Parse errors show correct line numbers
- AST nodes carry correct source spans
- Programs with 50+ local variables compile
- All builtins above callable from HLX
- The fix for the `BinaryOp::Pow` Nop in the lowerer (wire it to the new `pow` builtin)

---

## Phase 2 — Keywords: gate, modify, and govern wiring
**Estimated effort: Medium. Two to three sessions for Kilo.**

The unique thing about HLX is formal constraint — the conscience predicates and gates. Right now they parse and silently do nothing. This phase makes them real.

### 2.1 — Fix the GovernCheck operand mismatch

**Files:** `hlx-runtime/src/lowerer.rs`, `hlx-runtime/src/vm.rs`

**Problem:** The VM's `GovernCheck` handler reads 3 operands: `result_reg`, `effect_type_byte`, `desc_string_idx`. The lowerer emits `GovernCheck` with only 1 operand: `result_reg`. This causes a decode error at runtime for any agent using `govern`.

**Fix:** Audit every `GovernCheck` emit site in the lowerer. Update to emit all 3 operands. The `effect_type_byte` should map from the parsed `EffectClass` enum. The `desc_string_idx` can be a blank string index initially.

**Test:** An agent with `govern { effect: Analyze; trust: 0.8; }` should execute without decode error.

### 2.2 — Wire conscience predicates from source to Governance system

**Files:** `hlx-runtime/src/lowerer.rs`, `hlx-runtime/src/governance.rs`

**Problem:** `govern { conscience: [no_harm, path_safety]; }` parses into a `Vec<ConsciencePredicate>` in the AST but the lowerer ignores it. The parsed list is never connected to the Rust `Governance` predicate registration.

**Fix:** In the agent lowering path, after emitting `GovernRegister`, iterate the `govern.conscience` list. For each `ConsciencePredicate`, emit a `GovernRegister` variant that passes the predicate name. The `Governance` system should map predicate name strings to the built-in predicate functions it already has (`confidence_halt`, `rate_limit`, `self_modify_safeguard`, `severity_cap`, `reversibility`).

Mapping:
- `no_harm` → `severity_cap` predicate
- `path_safety` → `reversibility` predicate
- `rate_limit` → `rate_limit` predicate
- `confidence_halt` → `confidence_halt` predicate
- `self_modify_safeguard` → `self_modify_safeguard` predicate

**Test:** An agent with `conscience: [confidence_halt]` should have that predicate active in the governance registry at runtime.

### 2.3 — Make `modify self { ... }` emit gate enforcement bytecode

**Files:** `hlx-runtime/src/lowerer.rs`

**Problem:** `modify self { gate proof; ... body ... }` parses into a `ModifyDef` with a `gates: Vec<Gate>` list, but the lowerer iterates the gates and discards them (`let _ = gate_idx`). The modify body executes without any gate check.

**Fix:**

Before emitting the modify body:
1. For each gate in `modify.gates`:
   - `Gate::Proof { .. }` → emit `GovernCheck` with `EffectClass::Modify` + a confidence check
   - `Gate::Consensus { threshold }` → emit `ConsensusCreate` + `ConsensusVote` + `ConsensusResult` check, `JumpIfNot` to skip body on failure
   - `Gate::Human` → emit a `MemoryGet` for an auth token key + `GovernCheck` with Human gate type
   - `Gate::SafetyCheck` → emit `GovernCheck` with all registered conscience predicates

2. After each gate check, emit a `JumpIfNot skip_label` so the body is bypassed if the gate rejects.

3. Emit the RSI proposal: `RSIPropose` opcode with `ModificationType::ParameterUpdate` by default (until Phase 4 adds richer types).

4. After the body executes: emit `RSIVote` (self-vote approve), `RSIValidate`, `RSIApply`.

5. Add a rollback label: if any step returns failure, emit `RSIRollback`.

**Test:** A `modify self { gate proof; let x: i64 = 1; }` block should:
- Pass gate check if governance confidence is above threshold
- Fail gate check and skip the body if below threshold
- Emit an RSI proposal with type ParameterUpdate

### 2.4 — `for` loop lowering

**File:** `hlx-runtime/src/lowerer.rs`

**Problem:** `StmtKind::For` emits a single `Nop`. For loops are needed for iterating over arrays, maps, and training data.

**Fix:** Lower `for item in collection { body }` as:
```
  i_reg = 0
  len_reg = call len(collection)
loop_start:
  cmp_reg = i_reg >= len_reg
  JumpIf cmp_reg, loop_end
  item_reg = call get_at(collection, i_reg)
  [body lowered with item_reg bound to loop variable]
  i_reg = i_reg + 1
  Jump loop_start
loop_end:
```

This requires the iterator variable to be registered in the scope like a `let` binding for the duration of the loop body.

**Test:** `for x in arr { println(x); }` should print each element.

### 2.5 — `match` expression lowering

**File:** `hlx-runtime/src/lowerer.rs`

**Problem:** `ExprKind::Match` emits `Nop`. Match is heavily used in Bit's pattern recognition.

**Fix:** Lower `match val { arm1 => expr1, arm2 => expr2, _ => default }` as a chain of `Eq` comparisons with `JumpIf` branches. Each arm emits: compare subject to pattern literal → if equal, emit branch body → jump to end. Wildcard `_` arm is the fallthrough.

**Test:** A match on string values should dispatch to the correct arm.

---

## Phase 3 — Module System
**Estimated effort: Large. This is the most complex phase. Plan for 3–4 sessions.**

No real HLX program can be written in a single file. Bit.hlx is already 709 lines and we want it to grow. The module system is what turns HLX from a toy into a real language.

### 3.1 — Module resolution design

HLX module resolution follows these rules:
1. `import "path/to/file.hlx"` — relative to the importing file's directory
2. `import stdlib::math` — built-in standard library (later; skip for now)
3. A module is a compiled `Bytecode` object with a **symbol table** of exported names → function offsets

The runtime needs:
- A `ModuleCache`: `HashMap<PathBuf, CompiledModule>` — files are compiled once and cached
- `CompiledModule`: `{ bytecode: Bytecode, exports: HashMap<String, FunctionEntry> }`
- The VM needs a `modules: HashMap<String, CompiledModule>` field

### 3.2 — Multi-file compilation in hlx-run

**File:** `hlx-run/src/main.rs`

**Change:** When compiling a file, after parsing, scan for `Item::Import` nodes. For each import:
1. Resolve the path relative to the current file's directory
2. If not in cache, recursively compile that file first
3. Add the resulting `CompiledModule` to the `ModuleCache`
4. Patch the current bytecode's constant pool to include the imported function addresses

This is a depth-first recursive compilation. Cycles should be detected and produce an error.

### 3.3 — Export table

**File:** `hlx-runtime/src/lowerer.rs`

**Change:** When lowering, track which functions were declared with `export fn`. After lowering, produce an `ExportTable: HashMap<String, u32>` mapping function names to their start instruction offsets in the bytecode.

The `Bytecode` struct should gain an `exports: HashMap<String, u32>` field.

### 3.4 — Import resolution in the lowerer

**File:** `hlx-runtime/src/lowerer.rs`

**Change:** When `Item::Import(path)` is encountered during lowering, instead of `Ok(())`:
1. Look up the path in the `ModuleCache`
2. Register the imported module's exported functions as callable names in the current scope's function table
3. When a call to an imported function is encountered, emit a cross-module `Call` that references the correct bytecode offset (possibly via a function address constant)

For the initial implementation, **inline all imported bytecode into the caller's bytecode** with offset adjustment. This is simpler than dynamic linking and sufficient for all current use cases. Namespacing: imported names are available unqualified (matching how Bit.hlx expects to call helpers).

### 3.5 — Standard library bootstrap

**Directory:** `hlx/stdlib/` (new)

Create these as `.hlx` files importable via `import stdlib::<name>`:
- `stdlib/math.hlx` — wrappers around the math builtins with named constants (`PI`, `E`)
- `stdlib/string.hlx` — higher-order string operations built on the string builtins
- `stdlib/list.hlx` — `map`, `filter`, `reduce` implemented as HLX functions
- `stdlib/io.hlx` — file read/write wrappers

**Test:** `import "stdlib/math.hlx"` followed by calling a stdlib function should work end-to-end.

### ✅ Phase 3 Milestone
- `bit.hlx` split into multiple files with imports works
- Circular import produces a clear error
- Exported functions from one file callable from another
- A basic stdlib exists and is importable

---

## Phase 4 — Closing the RSI Loop
**Estimated effort: Medium-Large. This is the heart of HLX's value proposition.**

The Rust RSI machinery (voting, gating, rollback) is complete and tested. Phase 4 wires it to HLX-level constructs so a running agent can propose and apply modifications to itself under formal governance.

### 4.1 — RSIPropose opcode: support all ModificationTypes

**File:** `hlx-runtime/src/vm.rs` (RSIPropose handler)

**Problem:** The VM's `RSIPropose` handler only supports `ModificationType` 0 (ParameterUpdate) and 1 (CycleConfigChange). Types 2–8 return an error.

**Fix:** Implement all 9 types:
- `0` ParameterUpdate — update a numeric parameter by name
- `1` CycleConfigChange — change cycle count or H-level
- `2` BehaviorAdd — add a new behavior function (name + bytecode blob)
- `3` BehaviorRemove — remove a named behavior
- `4` ThresholdChange — update a confidence/trust threshold
- `5` WeightMatrixUpdate — update a latent weight array
- `6` RuleAdd — add a new governance predicate (by name + serialized rule)
- `7` RuleRemove — remove a predicate by name
- `8` RuleUpdate — update an existing predicate

For types 2–8, the `RSIPropose` opcode operands should include a string register for the target name and a value register for the proposed new value.

### 4.2 — `modify self` → full RSI proposal flow

**File:** `hlx-runtime/src/lowerer.rs`

Building on Phase 2.3 (gate checks), now complete the RSI proposal flow inside `modify self` blocks:

The lowerer should inspect the body of a `modify self` block and infer the `ModificationType` from what it contains:
- `let x = new_value` on a parameter → `ParameterUpdate`
- `cycle H(N)` change → `CycleConfigChange`
- A new function definition → `BehaviorAdd`
- A `govern` block change → `RuleUpdate`

The emitted bytecode for a `modify self` block should follow this sequence:
```
1. Gate checks (Phase 2.3)
2. RSIPropose(mod_type, target_name_reg, new_value_reg)
3. RSIVote(proposal_id_reg, approve=true)     ← self-votes to approve
4. GovernCheck(confidence >= threshold)
5. RSIValidate(proposal_id_reg)
6. JumpIfNot rollback_label
7. RSIApply(proposal_id_reg)
8. Jump end_label
rollback_label:
9. RSIRollback(proposal_id_reg)
end_label:
```

### 4.3 — Homeostasis and Promotion readable from HLX

**Files:** `hlx-runtime/src/vm.rs`, `hlx-runtime/src/lowerer.rs`, `hlx-runtime/src/bytecode.rs`

Add new builtins (or opcodes) to expose gate state to running HLX programs:

```hlx
// From HLX code:
let pressure = homeostasis_pressure();         // returns f64 0.0–1.0
let level = promotion_level();                 // returns String: "Seedling", "Sprout", etc.
let can_modify = can_modify_self();            // returns bool (checks all gates)
let history = rsi_history();                   // returns List of past proposal results
```

Implement as builtins dispatched in `vm.rs`. The builtins read from the VM's `rsi_pipeline` and `homeostasis_gate` fields.

### 4.4 — Promotion criteria fulfillment

**File:** `hlx-runtime/src/promotion.rs`

**Problem:** The `PromotionGate` starts all agents at `Seedling`, which blocks most RSI modification types. The criteria for promotion exist in code but there's no automatic evaluation.

**Fix:** After every successful `RSIApply`, call `PromotionGate::check_criteria()`. If criteria are met, automatically advance the level. The check should verify:
- Seedling → Sprout: at least 3 successful ParameterUpdate proposals with positive fitness delta
- Sprout → Sapling: at least 1 CycleConfigChange + stable homeostasis pressure < 0.5
- Sapling → Mature: BehaviorAdd successfully applied and evaluated
- Mature → ForkReady: Full RSI cycle (Propose→Vote→Validate→Apply→Evaluate) completed with positive net fitness

Emit a log message when promotion occurs: `"[PROMOTION] Bit advanced to Sprout"`.

### 4.5 — Fitness evaluation hooks

For the RSI loop to be governed correctly, there must be a way to measure whether a modification improved the agent. Add a `fitness` construct to HLX:

```hlx
govern {
    effect: Modify;
    conscience: [self_modify_safeguard, confidence_halt];
    trust: 0.8;
    fitness: fn(before: dict, after: dict) -> f64 {
        // user-defined fitness function
        return after["accuracy"] - before["accuracy"];
    };
}
```

The lowerer should parse the optional `fitness:` clause in `govern` and register it as a named function `__fitness__<agent_name>`. After `RSIApply`, the VM should automatically call this function with pre/post snapshots and store the result in the RSI proposal's fitness field.

**This is the core feedback loop:** modify → gate → apply → measure → inform next modification.

### ✅ Phase 4 Milestone
- `modify self { gate proof; ... }` causes an actual RSI proposal and passes/fails the governance gate
- `homeostasis_pressure()` and `promotion_level()` return live values from a running agent
- An agent that successfully modifies itself 3 times promotes from Seedling to Sprout
- A fitness function attached to `govern` is called after every `RSIApply`
- The full loop is demonstrable: run `bit.hlx`, observe it propose a modification, watch it pass the gate and get applied

---

## Phase 5 — Developer Toolchain: LSP, Debugger, and bond keyword
**Estimated effort: Large. Build in parallel with Phase 4 where possible.**

### 5.1 — LSP Server

**New crate:** `hlx-lsp/` (add to workspace)

HLX LSP implements the [Language Server Protocol](https://microsoft.github.io/language-server-protocol/) over stdin/stdout using the `tower-lsp` crate.

**Required capabilities:**

*Diagnostics (publishDiagnostics):*
- Parse errors with correct line/col (requires Phase 1.1)
- Unknown variable references (lowerer can emit these as diagnostics instead of silent Nil)
- Unknown function calls
- Type mismatches where inferable

*Hover (textDocument/hover):*
- Hovering over a variable shows its type (if determinable from `let` annotation)
- Hovering over a function shows its signature
- Hovering over a builtin shows its signature + brief description

*Go to definition (textDocument/definition):*
- Jump from function call site to function definition
- Jump from imported name to the source file + line

*Completion (textDocument/completion):*
- In-scope variable names
- Function names (current file + imported)
- HLX keywords
- Agent latent variable names inside agent bodies
- Builtin function names

*Document symbols (textDocument/documentSymbol):*
- Outline view: list all `fn`, `recursive agent`, `module`, `struct` definitions with their line numbers

**Architecture:**
```
hlx-lsp/src/main.rs         — stdin/stdout LSP server binary
hlx-lsp/src/backend.rs      — tower-lsp Backend trait implementation
hlx-lsp/src/analysis.rs     — thin wrapper over AstParser + symbol table builder
hlx-lsp/src/symbols.rs      — symbol table: name → definition location + type
```

The LSP should re-parse the document on every `textDocument/didChange` notification and rebuild the symbol table. For performance, the symbol table build should be incremental (only re-parse changed regions) but a full re-parse on every change is acceptable for initial implementation.

**VS Code extension:** Add `hlx-lsp` binary invocation to `editors/vscode/package.json` as a language server. The extension already has `.hlx` syntax highlighting. Add:
```json
"activationEvents": ["onLanguage:hlx"],
"contributes": {
    "languages": [{ "id": "hlx", "extensions": [".hlx"] }]
}
```

**Test:** Opening `bit.hlx` in VS Code with the extension active should show:
- Red underlines on intentional parse errors
- Hover on a function showing its signature
- Completion popup in function call position

### 5.2 — Debugger protocol

**New module:** `hlx-runtime/src/debugger.rs`

The debugger is implemented as a **DAP (Debug Adapter Protocol)** server. DAP is the same protocol VS Code uses for all debuggers — implementing it means VS Code debug integration comes free.

**VM changes for debugability:**

Add to `Vm`:
```rust
pub breakpoints: HashSet<u32>,           // instruction offsets with breakpoints
pub debug_mode: bool,
pub step_mode: StepMode,                 // Run | StepOver | StepIn | StepOut
pub debug_tx: Option<Sender<DebugEvent>>,
```

`DebugEvent` enum:
```rust
enum DebugEvent {
    Stopped { reason: StopReason, pc: u32, line: u32, col: u32 },
    VariableState { registers: Vec<(String, Value)>, latents: Vec<(String, Value)> },
    RSIProposal { proposal_id: u64, mod_type: String, target: String },
    GateCheck { gate: String, passed: bool },
    Promotion { from: String, to: String },
}
```

When `debug_mode` is true, the VM pauses before each instruction if:
- `step_mode == StepOver` (always pause), OR
- The current `pc` is in `breakpoints`

On pause, send a `Stopped` event and wait for a resume command on a channel.

**DAP server (`hlx-lsp/src/dap.rs`):**
- `initialize` → return capabilities (supports setBreakpoints, stackTrace, variables, continue, next, stepIn, stepOut)
- `launch` → start VM in debug mode with the given `.hlx` file
- `setBreakpoints` → convert line numbers to instruction offsets via the bytecode's source map
- `continue` → resume VM
- `next` / `stepIn` → set step mode
- `variables` → return current register names + values + latent state
- `stackTrace` → return call stack frames with file + line

**The source map:** The bytecode needs a `source_map: Vec<(u32, u32, u32)>` — `(instruction_offset, line, col)`. The lowerer populates this using the AST spans from Phase 1.2. This is what maps breakpoints from editor line numbers to VM instruction offsets.

**Test:** Setting a breakpoint on line N of a `.hlx` file should pause execution at that line with correct variable state visible in the VS Code variables panel.

### 5.3 — The `bond` HLX keyword

**Problem:** `hlx-bond` works as a standalone binary with a REPL. HLX programs cannot invoke the LLM. The `bond` keyword is planned but not implemented.

**Design:**

```hlx
// In an agent body:
let response: String = bond("What is the entropy of this pattern?", context_dict);
```

`bond(prompt, context)` is a blocking call that:
1. Sends `prompt` + serialized `context` to the `hlx-bond` process via Unix socket / named pipe
2. Waits for a response string
3. Returns the response as an HLX `String` value

**Implementation plan:**

*`hlx-bond` changes:*
- Add an `--server` mode: instead of a REPL, listen on a Unix socket (`/tmp/hlx-bond.sock`)
- Deserialize incoming `{ prompt: String, context: Map }` JSON requests
- Run inference with the context injected into the system prompt
- Return `{ response: String }` JSON

*`hlx-runtime` changes:*
- Add `bond` as a builtin: `bond(prompt_reg, context_reg) -> result_reg`
- The builtin connects to the socket (lazy-connect on first call), sends the request, blocks on response
- Connection state stored in `Vm` as `bond_socket: Option<UnixStream>`

*Parser change:*
- `bond(...)` can be parsed as a function call to the `bond` builtin — no new keyword needed, it's just a builtin function

**Alternative (simpler) implementation:** Have `bond(prompt, context)` invoke `hlx-bond` as a subprocess per call with `--prompt` and `--context-json` flags. Slower but avoids the socket infrastructure. Implement the socket version after the subprocess version works.

**Test:** An HLX agent that calls `bond("hello", {})` should return a string response from Qwen3.

### 5.4 — Move LSP/Debugger from BioForge to hlx-runtime

Kilo placed the LSP and debugger stubs in `bioforge/`. They belong in the core toolchain.

**Action:**
1. Audit `bioforge/` for any LSP/debugger code
2. Extract it to `hlx-lsp/` and `hlx-runtime/src/debugger.rs`
3. Update `bioforge/` to import from `hlx-lsp` rather than defining its own

### ✅ Phase 5 Milestone
- `hlx-lsp` binary exists and VS Code shows diagnostics on `.hlx` files
- Breakpoints work in VS Code: execution pauses, variables panel shows agent state
- `bond("prompt", {})` in HLX returns a Qwen3 response
- BioForge uses the same LSP, not its own copy

---

## Cross-Cutting: The Observation Interface

Matt explicitly needs to **watch Bit evolve in real time** — see the RSI pressure, gate state, proposal queue, and promotion level as they change.

This is separate from the debugger (which is for development) — this is for **runtime observation of a live agent**.

### Observation stream (`hlx-run --observe`)

Add a `--observe` flag to `hlx-run` that emits a stream of JSON events to stdout (or a file) during execution:

```json
{ "event": "cycle_begin", "agent": "bit", "level": "H3", "ts": 1234567890 }
{ "event": "latent_update", "agent": "bit", "name": "z_high", "value": 0.73, "ts": 1234567890 }
{ "event": "rsi_propose", "agent": "bit", "id": "abc123", "type": "ParameterUpdate", "target": "confidence_threshold", "proposed": 0.85, "ts": 1234567890 }
{ "event": "gate_check", "gate": "proof", "passed": true, "confidence": 0.91, "ts": 1234567890 }
{ "event": "rsi_apply", "id": "abc123", "fitness_delta": +0.03, "ts": 1234567890 }
{ "event": "promotion", "from": "Seedling", "to": "Sprout", "ts": 1234567890 }
{ "event": "homeostasis", "density_pressure": 0.42, "efficiency_pressure": 0.18, "expansion_score": 0.61, "ts": 1234567890 }
```

This event stream can be consumed by:
- A simple terminal visualizer (`hlx-observe` TUI tool — future work)
- BioForge's audit system (it already writes audit JSONL files)
- Bitsy's Place frontend

Implementation: a `EventBus` in the VM — an `Option<Sender<VmEvent>>`. When `--observe` is set, the main thread spawns a logger that receives events and writes JSONL to a file or stdout.

---

## Testing Strategy

Each phase should have a corresponding `.hlx` test file committed to `hlx-runtime/tests/hlx/`:

- `tests/hlx/phase1_builtins.hlx` — exercises every new builtin
- `tests/hlx/phase2_gates.hlx` — an agent that checks gates and logs results
- `tests/hlx/phase3_modules.hlx` + `tests/hlx/phase3_helper.hlx` — cross-file import test
- `tests/hlx/phase4_rsi_loop.hlx` — full modify-gate-apply-evaluate loop
- `tests/hlx/phase5_bond.hlx` — bond call returning a string (mocked if LLM not available)

The Rust test suite (`hlx-runtime/tests/`) should gain integration tests that run these `.hlx` files and assert on their output.

---

## File Change Summary

| File | Phase | Change |
|---|---|---|
| `hlx-runtime/src/ast_parser.rs` | 1, 2 | Line/col tracking, span population |
| `hlx-runtime/src/lowerer.rs` | 1, 2, 3, 4 | Var limit, for/match, gate enforcement, RSI opcodes, imports |
| `hlx-runtime/src/builtins.rs` | 1 | 30+ new builtins |
| `hlx-runtime/src/vm.rs` | 2, 4 | GovernCheck fix, RSIPropose all types, bond builtin, homeostasis/promotion builtins, EventBus |
| `hlx-runtime/src/bytecode.rs` | 4 | source_map field, exports field |
| `hlx-runtime/src/rsi.rs` | 4 | All ModificationTypes, fitness field |
| `hlx-runtime/src/promotion.rs` | 4 | Auto-promotion after RSIApply |
| `hlx-runtime/src/debugger.rs` | 5 | New: DAP protocol handler |
| `hlx-run/src/main.rs` | 3, 5 | Multi-file compilation, --observe flag |
| `hlx-lsp/` | 5 | New crate: tower-lsp server + VS Code extension update |
| `hlx-bond/src/main.rs` | 5 | --server mode with Unix socket |
| `hlx/stdlib/` | 3 | New: stdlib .hlx files |

---

## What Kilo Should Work On First

**Start with Phase 1.** It's all unblocking work and none of it requires design decisions — it's just implementation. Specifically in this order:

1. `1.1` Tokenizer line tracking — 20 lines, unblocks all error messages
2. `1.3` Variable limit raise — 5 lines, unblocks Bit.hlx from hitting the ceiling
3. `1.4` Builtins — repetitive but straightforward, do 5–10 at a time
4. `2.1` GovernCheck operand fix — small, unblocks all govern/gate testing
5. `2.4` For loop lowering — needed for most of Phase 4's test programs
6. Then tackle Phase 2 and 3 in parallel (module system is independent of gate wiring)

---

*This plan was written against a full codebase audit of commit `bef5c90` on `main` (Feb 28 2026). As Kilo makes changes, update the checkboxes above and note any deviations from the plan in a `PLAN_NOTES.md` file in the repo root.*
