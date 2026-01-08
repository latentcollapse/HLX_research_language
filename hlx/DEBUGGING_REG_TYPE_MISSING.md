# Debugging "Reg type missing" Errors

## What is "Reg type missing"?

**Error Location:** `hlx_backend_llvm/src/lib.rs` - in `load_reg()` function

**Cause:** The LLVM backend tries to load a register that has never been stored to, so it has no type information.

**Example:**
```rust
// LLVM backend
fn load_reg(&self, reg: Register) -> Result<(BasicValueEnum<'ctx>, ValueType)> {
    let reg_type = self.reg_types.get(&reg)
        .ok_or_else(|| anyhow!("Reg type missing for r{}", reg.0))?;  // ❌ ERROR HERE
    // ...
}
```

---

## Root Causes

### 1. **User Code: Uninitialized Variables** ✅ FIXED (Phase 1.1)

**Example HLX Code:**
```hlx
fn main() {
    let x;
    print(x);  // Uses x before initialization
}
```

**Solution:** Dataflow analysis in LSP catches this before compilation.

**Status:** ✅ Should be caught by LSP now

---

### 2. **Compiler Bug: Bad IR Generation** ⚠️ POSSIBLE

**Example Scenario:**
```hlx
fn main() {
    return 0;  // Valid code
}
```

**But compiler generates:**
```
Load r0    // ❌ r0 was never stored!
Return r0
```

**Solution:** Fix the compiler to initialize registers or emit correct IR.

**Status:** ⚠️ Needs investigation

---

### 3. **LLVM Backend: Missing Register Initialization** ⚠️ POSSIBLE

**Problem:** Some code paths in the backend don't initialize `reg_types` for all registers.

**Example:**
```rust
// In compile_instruction()
Instruction::Call { out, func, args } => {
    // ... call logic
    // ❌ Forgot to store return value type to reg_types!
}
```

**Solution:** Audit all instructions that write to registers, ensure `reg_types` is updated.

**Status:** ⚠️ Needs audit

---

## Debugging Steps for Gemini

### Step 1: Reproduce the Error

Run your code that triggers "Reg type missing":

```bash
cd /path/to/your/hlx/code
hlx compile your_file.hlxa
```

**Capture:**
- The exact error message
- The register number (e.g., "Reg type missing for r5")
- The source code that triggers it

---

### Step 2: Check LSP Diagnostics

Open the same file in your editor with HLX LSP running.

**Expected:** LSP should show a red error if it's an uninitialized variable.

**If LSP shows no error:** The compiler is generating bad IR from valid code.

---

### Step 3: Add Debug Logging

Modify `hlx_backend_llvm/src/lib.rs` to log register operations:

```rust
fn store_reg(&mut self, reg: Register, val: BasicValueEnum<'ctx>, typ: ValueType) -> Result<()> {
    eprintln!("DEBUG: store_reg r{} = {:?}", reg.0, typ);  // ADD THIS
    self.reg_types.insert(reg, typ);
    // ...
}

fn load_reg(&self, reg: Register) -> Result<(BasicValueEnum<'ctx>, ValueType)> {
    eprintln!("DEBUG: load_reg r{}", reg.0);  // ADD THIS
    let reg_type = self.reg_types.get(&reg)
        .ok_or_else(|| anyhow!("Reg type missing for r{}", reg.0))?;
    // ...
}
```

Recompile and run:
```bash
cargo build --release
./target/release/hlx compile your_file.hlxa
```

**Look for:**
- Loads without corresponding stores
- Pattern of register usage

---

### Step 4: Inspect Generated IR

Add logging to see what IR the compiler emits:

In `hlx_compiler` (wherever IR is generated), add:
```rust
eprintln!("IR: {:?}", instruction);
```

**Look for:**
- Instructions that use registers before they're defined
- Missing `Constant` or `Store` instructions

---

## Quick Fixes

### Fix 1: Default Register Initialization (Backend Safety Net)

**In `hlx_backend_llvm/src/lib.rs`, at function entry:**

```rust
fn compile_function(&mut self, ...) {
    // Initialize all registers to Int(0) by default
    for i in 0..256 {
        self.reg_types.insert(Register(i), ValueType::Int);
    }

    // ... rest of function compilation
}
```

**Pros:** Prevents crashes, always safe
**Cons:** Masks compiler bugs, might hide real issues

---

### Fix 2: Better Error Messages

**In `load_reg()`:**

```rust
fn load_reg(&self, reg: Register) -> Result<(BasicValueEnum<'ctx>, ValueType)> {
    let reg_type = self.reg_types.get(&reg).ok_or_else(|| {
        // Show all known registers for debugging
        let known_regs: Vec<_> = self.reg_types.keys().map(|r| r.0).collect();
        anyhow!(
            "Reg type missing for r{}.\n\
            Known registers: {:?}\n\
            This usually means the compiler generated IR that uses this register before storing to it.\n\
            Check the IR generation for the current function.",
            reg.0, known_regs
        )
    })?;
    // ...
}
```

---

## For Claude to Investigate

### Audit All `store_reg` Call Sites

Search `hlx_backend_llvm/src/lib.rs` for all instructions that write to `out` register:

```rust
// Find patterns like:
Instruction::SomeOp { out, ... } => {
    // ... compute result
    self.store_reg(*out, result, result_type)?;  // ✅ Good
}

// OR missing store:
Instruction::SomeOp { out, ... } => {
    // ... compute result
    // ❌ FORGOT TO STORE!
}
```

**Instructions to check:**
- Add, Sub, Mul, Div, Neg
- Lt, Gt, Eq
- Call (return value)
- Constant
- Load (from memory)

---

## Test Cases

### Test 1: Simple Uninitialized Use (Should be caught by LSP)
```hlx
program test {
    fn main() {
        let x;
        print(x);  // LSP should error here
        return 0;
    }
}
```

### Test 2: Conditional Initialization (Should be caught by LSP)
```hlx
program test {
    fn main() {
        let x;
        if (true) {
            x = 5;
        }
        print(x);  // LSP should warn: "maybe uninitialized"
        return 0;
    }
}
```

### Test 3: Valid Code (Should NOT error)
```hlx
program test {
    fn main() {
        let x = 5;
        print(x);
        return 0;
    }
}
```

### Test 4: Function Return (Potential compiler bug)
```hlx
program test {
    fn get_value() {
        return 42;
    }

    fn main() {
        let x = get_value();
        print(x);
        return 0;
    }
}
```

---

## Status

**Dataflow Analysis (Phase 1.1):** ✅ Complete - Should catch user code issues
**Backend Audit:** ⏳ Needed - Check for missing `store_reg` calls
**Compiler IR Generation:** ⏳ Needed - Check for bad IR emission

**Next Action:** Run tests with Gemini's actual failing code to identify which category the bug falls into.
