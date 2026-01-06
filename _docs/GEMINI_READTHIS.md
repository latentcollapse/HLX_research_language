# 🚨 GEMINI: READ THIS BEFORE BUILDING THE LSP 🚨

**Status:** The HLX compiler is **self-hosting and proven**. We achieved Ouroboros tonight (Jan 6, 2026) with deterministic, bytewise identical Stage 2 == Stage 3 compilation.

**Your Mission:** Build the Language Server Protocol (LSP) to make HLX feel alive in editors **without breaking the core compiler**.

---

## ⚠️ CRITICAL RULES - DO NOT VIOLATE ⚠️

### 1. **DO NOT MODIFY THE CORE COMPILER**

These files are **SACRED** - they produce the Ouroboros:
- ❌ `hlx_compiler/bootstrap/compiler.hlxc` - The self-hosted compiler source
- ❌ `hlx_compiler/src/lib.rs` - Core compilation logic
- ❌ `hlx_compiler/src/hlxa.rs` - The parser (for now)
- ❌ `hlx_compiler/src/emitter.rs` - Bytecode generation
- ❌ `hlx_core/` - Core types and values
- ❌ `hlx_runtime/` - The VM executor

**Why?** These files produce deterministic output. Changing them risks breaking the Ouroboros (Stage 2 == Stage 3 identity). We've proven they work. Don't touch them until LSP v1 is stable.

### 2. **BUILD THE LSP AS A SEPARATE MODULE**

The LSP should:
- ✅ Live in `hlx_lsp/` (already created)
- ✅ **Import and call** existing compiler functions
- ✅ **Not modify** existing compiler code
- ✅ Be its own binary (`hlx-lsp` executable)

**Architecture:**
```
Editor (VS Code)
    ↓ LSP Protocol
hlx_lsp (your code)
    ↓ calls existing functions
hlx_compiler (proven, don't touch)
    ↓ parses/compiles
Result (diagnostics, symbols, etc.)
```

### 3. **USE EXISTING PARSER - DON'T REWRITE IT**

**What you tried:** Adding `nom_locate` to get line/column info.
**What broke:** 1,171 compiler errors from dependency conflicts.

**What to do instead:**
- Call the existing parser: `hlx_compiler::parse(source_code)`
- If it succeeds → no syntax errors
- If it fails → extract error message (line/column may not be perfect yet, that's OK)
- **Phase 2:** Enhance the parser separately after LSP v1 works

**Example (good approach):**
```rust
// In hlx_lsp/src/diagnostics.rs
use hlx_compiler;

pub fn check_syntax(source: &str) -> Vec<Diagnostic> {
    match hlx_compiler::parse(source) {
        Ok(_ast) => vec![], // No errors
        Err(e) => vec![
            Diagnostic {
                message: format!("Syntax error: {}", e),
                line: 0, // TODO: extract from error later
                column: 0,
            }
        ],
    }
}
```

### 4. **INCREMENTAL PROGRESS - START MINIMAL**

**Phase 1: Basic Diagnostics (Start Here)**
- Goal: Show red squiggles when syntax is wrong
- How: Call existing parser, return success/failure
- Test: Type invalid HLX in VS Code, see red line
- **Don't worry about perfect line numbers yet**

**Phase 2: Semantic Highlighting (Later)**
- Goal: Color functions, types, keywords correctly
- How: Walk the AST from existing parser
- Test: Functions appear blue, types appear green, etc.

**Phase 3: Go to Definition (Much Later)**
- Goal: Click on a function name, jump to definition
- How: Build symbol table from AST
- Test: Ctrl+Click on `add()` jumps to `fn add()`

**DO NOT TRY TO DO ALL THREE AT ONCE.** Get Phase 1 working first.

### 5. **TEST THE OUROBOROS BEFORE COMMITTING**

Before you commit ANY changes to git:

```bash
cd /home/matt/hlx-compiler/hlx
./bootstrap.sh
```

**Expected output:**
```
✓✓✓ OUROBOROS COMPLETE! ✓✓✓
Hash: 5b8fa2ee59205fbf6e8710570db3ab0ddf59a3b4c5cbbbe64312923ade111f20
```

**If the hash changes or the script fails:**
- ❌ STOP IMMEDIATELY
- ❌ DO NOT COMMIT
- 🔄 Revert your changes: `git restore <file>`
- 💬 Tell Matthew what you were trying to do

### 6. **DEPENDENCY HYGIENE**

**When adding dependencies to `hlx_lsp/Cargo.toml`:**
- ✅ Use `tower-lsp` (standard LSP library) - already added
- ✅ Use `tokio` (async runtime) - already added
- ✅ Use workspace dependencies (`serde.workspace = true`)
- ❌ **DO NOT** add `nom_locate` or anything that changes `nom` version
- ❌ **DO NOT** add dependencies to `hlx_compiler/Cargo.toml`

**Why?** We use `nom 7.1` in the workspace. Adding `nom_locate` pulls in `nom 8.0` which breaks everything. If you need better error messages, do it in Phase 2 after Phase 1 works.

---

## 📋 YOUR CHECKLIST FOR BUILDING THE LSP

### Step 1: Set Up the LSP Server
- [x] Create `hlx_lsp/` crate (done)
- [ ] Add basic `tower-lsp` server scaffolding
- [ ] Handle `initialize` and `shutdown` LSP requests
- [ ] Test: LSP starts and responds to VS Code

### Step 2: Basic Syntax Checking
- [ ] Call `hlx_compiler::parse()` on document open/change
- [ ] Return diagnostics (errors) to VS Code
- [ ] Test: Type invalid syntax, see red squiggle

### Step 3: VS Code Extension (Minimal)
- [ ] Create `.vscode/extension/` folder
- [ ] Add `package.json` with language ID `hlx`
- [ ] Point to `hlx-lsp` binary
- [ ] Test: Open `.hlxa` file, LSP activates

### Step 4: Verify Nothing Broke
- [ ] Run `./bootstrap.sh` successfully
- [ ] Same hash as before
- [ ] Commit to git

---

## 🚫 WHAT NOT TO DO (LESSONS LEARNED)

### ❌ Don't Rewrite the Parser
**What happened:** You added `nom_locate` and changed `type Input<'a> = &'a str` to `type Input<'a> = LocatedSpan<&'a str>`.
**Result:** 1,171 compiler errors. Required reverting.
**Why it broke:** Dependency version conflict (`nom 7.1` vs `nom 8.0`).
**What to do instead:** Use the existing parser as-is. Enhance it in Phase 2.

### ❌ Don't Modify Core Types
**What could break:** Changing `hlx_core::Value` or `hlx_core::Instruction` to add metadata.
**Why it breaks:** The self-hosted compiler (`compiler.hlxc`) depends on exact type signatures.
**What to do instead:** Add LSP-specific types in `hlx_lsp/src/types.rs`.

### ❌ Don't Add Big Dependencies to Core Crates
**What could break:** Adding `serde_derive` or `async` traits to `hlx_compiler`.
**Why it breaks:** Increases compile time, can cause trait conflicts.
**What to do instead:** Keep heavy dependencies in `hlx_lsp/` only.

---

## 💬 COMMUNICATION PROTOCOL

### When You Get Stuck:
1. **Stop coding** - don't guess or try 10 different approaches
2. **Document the problem** - what are you trying to do? What error did you get?
3. **Ask Matthew** - describe the issue, propose 2-3 solutions
4. **Wait for guidance** - we'll debug together

### When You Finish a Phase:
1. **Test the Ouroboros** - `./bootstrap.sh` must pass
2. **Test the LSP** - open a `.hlxa` file, verify it works
3. **Commit with clear message** - `git commit -m "feat(lsp): Add basic syntax diagnostics"`
4. **Tell Matthew** - "Phase 1 complete: LSP shows syntax errors"

---

## 📚 REFERENCE COMMANDS

### Build the Rust compiler:
```bash
cd /home/matt/hlx-compiler/hlx
cargo build --release --bin hlx
```

### Run the Ouroboros bootstrap:
```bash
./bootstrap.sh
```

### Build the LSP server:
```bash
cargo build --release --bin hlx-lsp
```

### Test a simple HLX program:
```bash
./target/release/hlx run examples/test_simple_math.hlxa
```

### Revert a file if you broke something:
```bash
git restore hlx_compiler/src/hlxa.rs
```

---

## 🎯 GOAL: LSP V1 (PHASE 1 ONLY)

**Success Criteria:**
1. Open a `.hlxa` file in VS Code
2. Type invalid syntax (e.g., missing semicolon)
3. See a red squiggle appear
4. Hover over it, see error message
5. Fix the syntax, red squiggle disappears
6. **The Ouroboros still works** (`./bootstrap.sh` passes)

**That's it.** No fancy features. No perfect line numbers. No go-to-definition. Just: **does the user see an error when they make a mistake?**

Once that works, we'll tackle Phase 2 (highlighting) and Phase 3 (go-to-definition).

---

## 🙏 THANK YOU

We know you're excited to build with HLX. We are too! But the compiler we built tonight is **historic** - a self-hosting, deterministically proven system achieved for $35. Let's not break it in our excitement.

Build the LSP carefully, incrementally, and communicate often. You've got this. 🚀

---

**Last Ouroboros Hash (Jan 6, 2026):**
```
5b8fa2ee59205fbf6e8710570db3ab0ddf59a3b4c5cbbbe64312923ade111f20
```

If this hash changes without explicit intent, **STOP AND REVERT.**

---

**- Claude Sonnet 4.5 & Matthew**
*Preserving the Ouroboros*
