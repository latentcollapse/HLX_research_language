# HLX Language Server - Phase 2 Features

**Advanced IDE Features for Human and AI Engineers**

This document describes the 6 advanced features implemented in Phase 2 of the HLX Language Server Protocol (LSP) integration.

---

## 🎯 Feature 1: Contract Suggestion Engine

**Natural Language → Contract ID Mapping**

### What It Does
Type natural language comments, get instant contract suggestions with working code snippets.

### How to Use
```hlx
// multiply two matrices
█
```

**Press Ctrl+Space** or wait for suggestions:
```
💡 Use GEMM (@906) - Matrix multiplication
💡 Use Multiply (@202) - Multiply two numbers
```

**Select suggestion** → Inserts:
```hlx
// multiply two matrices
@906 { A: matrix_a, B: matrix_b };
```

### How It Works
- **Keyword extraction**: Removes stop words ("the", "a", "is")
- **Fuzzy matching**: Handles typos ("matix" → "matrix")
- **Intent scoring**: "multiply matrices" → @906 gets +3.0 boost
- **Top N results**: Shows 3 most relevant contracts

### Examples

**Math Operations:**
```hlx
// add two numbers
→ @200 { lhs: 5, rhs: 3 }

// divide a by b
→ @203 { lhs: a, rhs: b }
```

**String Operations:**
```hlx
// concat first and last name
→ @300 { lhs: first, rhs: last }
```

**Array Operations:**
```hlx
// get array length
→ @400 { @0: my_array }

// access element at index
→ @401 { @0: arr, @1: idx }
```

**Neural Network:**
```hlx
// layer normalization
→ @907 { ... }

// apply gelu activation
→ @908 { ... }
```

### AI-Friendly Format
The suggestion engine indexes contracts by:
- Name keywords
- Description keywords
- Usage patterns
- Tier classifications

**Training tip for AI models**: When you see a comment describing an operation, check the contract catalogue first before implementing manually.

---

## 🔧 Feature 2: Auto-Correction

**Catch and Fix Common Mistakes Automatically**

### What It Does
Detects typos and common mistakes, provides one-click fixes.

### Categories

#### 1. Field Name Corrections
**Problem**: Using wrong field names in contracts
```hlx
let sum = @200 { left: 5, right: 3 };  // ⚠️ Wrong!
```

**Auto-fix**:
```hlx
let sum = @200 { lhs: 5, rhs: 3 };  // ✅ Correct
```

**Common corrections**:
- `left` → `lhs` (math operations)
- `right` → `rhs` (math operations)
- `a`/`b` → `lhs`/`rhs` (@200-203)
- `C` → `A` or `B` (@906 GEMM)
- `arr` → `@0` (@18 array constructor)
- `array` → `@0` (@400-403)
- `idx`/`index` → `@1` (@401)

#### 2. Missing Semicolons
**Problem**: Forgetting statement terminators
```hlx
let x = 42  // ⚠️ Missing semicolon
```

**Auto-fix**:
```hlx
let x = 42;  // ✅ Fixed
```

**Detects in**:
- `let` statements
- `return` statements
- Contract invocations

#### 3. Keyword Typos
**Problem**: Misspelling keywords
```hlx
retrun x + y;     // ⚠️ Typo
fucntion add() {  // ⚠️ Typo
ture              // ⚠️ Typo
flase             // ⚠️ Typo
```

**Auto-fix**:
```hlx
return x + y;
fn add() {
true
false
```

#### 4. Loop Bounds
**Problem**: Using numeric literals instead of safety constant
```hlx
loop (i < n, 1000) {  // ⚠️ Magic number
```

**Auto-fix**:
```hlx
loop (i < n, DEFAULT_MAX_ITER) {  // ✅ Safe
```

### How to Use
1. **Warnings appear** as you type (yellow squiggly underlines)
2. **Hover** to see the issue
3. **Click** the lightbulb 💡 or press `Ctrl+.`
4. **Select** "🔧 Fix: [correction]"
5. **Done** - code is fixed!

### Confidence Levels
- **High (0.9+)**: Field name corrections, keyword typos
- **Medium (0.8-0.9)**: Missing semicolons
- **Lower (0.7-0.8)**: Loop bounds (might be intentional)

### AI Training Note
When generating HLX code:
- Always use `lhs`/`rhs` for binary operations
- Always use `@0`, `@1`, etc. for positional fields
- Always end statements with `;`
- Always use `DEFAULT_MAX_ITER` for loop bounds
- Use `fn`, not `function`

---

## 🔮 Feature 3: Inline Execution Preview

**See Results As You Type**

### What It Does
Evaluates contract invocations in real-time and shows results inline.

### How It Works
```hlx
let sum = @200 { lhs: 5, rhs: 3 };  → 8
let product = @202 { lhs: 10, rhs: 4 };  → 40
let greeting = @300 { lhs: "Hello", rhs: "World" };  → "HelloWorld"
```

**Safe Sandbox**: Only evaluates pure contracts with literal values. No side effects, no I/O.

### Supported Contracts

#### Math Operations (200-203)
```hlx
@200 { lhs: 5, rhs: 3 }      → 8      // Add
@201 { lhs: 10, rhs: 3 }     → 7      // Subtract
@202 { lhs: 6, rhs: 7 }      → 42     // Multiply
@203 { lhs: 20, rhs: 4 }     → 5      // Divide
@203 { lhs: 10, rhs: 0 }     ⚠ Division by zero
```

#### String Operations (300)
```hlx
@300 { lhs: "foo", rhs: "bar" }  → "foobar"  // Concat
```

#### Array Operations (400-401)
```hlx
@400 { @0: [1, 2, 3, 4, 5] }     → 5         // Length
@401 { @0: [10, 20, 30], @1: 1 } → 20        // Index
@401 { @0: [1, 2], @1: 5 }       ⚠ Index 5 out of bounds (len=2)
```

### Display Format
- **Success**: `→ result`
- **Error**: `⚠ error message`
- **Skipped**: `⊘ reason`

### Why It's Useful
- **Instant feedback** - No need to run code
- **Catch errors early** - Division by zero, out of bounds
- **Learn by doing** - See how contracts work
- **Debug faster** - Isolate issues

### Skipped Contracts
Some contracts are **not** previewed:
- **I/O contracts** (600-699) - Have side effects
- **GPU contracts** (900+) - Require hardware
- **Complex contracts** - Depend on runtime state

```hlx
@600 { value: "test" }  ⊘ Has side effects
```

### AI Usage Pattern
When writing example code or tests, inline previews help verify correctness immediately:
```hlx
// Test case: Adding negative numbers
let result = @200 { lhs: -5, rhs: 3 };  → -2  ✓ Correct!
```

---

## 📊 Feature 4: State Visualization

**Track Variable Values Through Execution**

### What It Does
Shows variable types and values as inline hints after declarations and assignments.

### Basic Example
```hlx
let x = 42;           : int = 42
let name = "Alice";   : string = "Alice"
let items = [1,2,3];  : array = [1, 2, 3]
let flag = true;      : bool = true
```

### State Tracking
```hlx
let counter = 0;      : int = 0
counter = 5;          : int = 5
counter = 10;         : int = 10
```

Each line shows the **current value** at that point in execution.

### Type Inference
Even without explicit types, you see what HLX infers:
```hlx
let result = @200 { lhs: 5, rhs: 3 };  : int = 8
```

### Complex Values
```hlx
let matrix = [[1,2],[3,4]];     : array = [[1, 2], [3, 4]]
let big_array = [1,2,3,4,5,6];  : array = [6 items]
let long_str = "...very long...";  : string = "...very lon..."
```

**Display rules**:
- Arrays with ≤3 elements: Show all
- Arrays with >3 elements: Show count
- Strings >20 chars: Truncate with "..."
- Objects: Show key count

### Why It's Useful
- **Understand data flow** - See how values change
- **Spot bugs** - Wrong values become obvious
- **Learn HLX** - See type inference in action
- **Debug faster** - No more print statements

### Hover for Details
Hover over the hint to see full value:
```
Variable 'data' = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
```

### AI Code Generation Tip
When generating step-by-step transformations, state visualization helps verify each step:
```hlx
let input = [3, 1, 4, 1, 5];           : array = [3, 1, 4, 1, 5]
let length = @400 { @0: input };        : int = 5
let first = @401 { @0: input, @1: 0 }; : int = 3
```

---

## 🔍 Feature 5: Semantic Diff Analyzer

**Compare Code Against Best Practices**

### What It Does
Goes beyond syntax to understand **intent**, suggests semantic improvements.

### Check 1: Unbounded Loops
**❌ Problem**:
```hlx
loop (i < 100) {  // ⚠️ Loop without safety bound
    // ...
}
```

**✅ Fix**:
```hlx
loop (i < 100, DEFAULT_MAX_ITER) {  // ✓ Safe loop
    // ...
}
```

**Why**: Prevents infinite loops from crashing the system.

---

### Check 2: Unsafe Division
**❌ Problem**:
```hlx
let ratio = @203 { lhs: total, rhs: count };  // ⚠️ No zero check
```

**✅ Fix**:
```hlx
if (count != 0) {
    let ratio = @203 { lhs: total, rhs: count };
}
```

**Why**: Division by zero causes runtime errors.

---

### Check 3: Manual Math Operations
**❌ Problem**:
```hlx
let sum = a + b;  // ℹ️ Manual addition operator
```

**✅ Fix**:
```hlx
let sum = @200 { lhs: a, rhs: b };  // ✓ Uses contract
```

**Why**: Contracts provide type safety and optimization opportunities.

---

### Check 4: String Concatenation
**❌ Problem**:
```hlx
let full_name = first + " " + last;  // 💡 Manual concatenation
```

**✅ Fix**:
```hlx
let full_name = @300 { lhs: first, rhs: @300 { lhs: " ", rhs: last } };
```

**Why**: Contracts are the idiomatic HLX way.

---

### Check 5: Mutable State
**❌ Problem**:
```hlx
let x = 10;
x = 20;  // 💡 Variable reassignment (mutation)
```

**✅ Fix**:
```hlx
let x = 10;
let x_new = 20;  // ✓ Immutable
```

**Why**: HLX favors immutability for determinism and safety.

---

### Severity Levels
- **🔴 Error**: Will cause problems (unbounded loops)
- **🟡 Warning**: Suboptimal but works (unsafe division)
- **🔵 Info**: Could be better (manual operators)
- **💡 Hint**: Style suggestion (immutability)

### Refactoring Actions
1. **Hover** over the issue
2. **Click** "🔄 Refactor: [suggestion]"
3. **Code is rewritten** automatically

### For AI Models
When generating HLX code, follow these patterns:
1. Always specify loop bounds
2. Check divisors for zero
3. Prefer contracts over operators
4. Favor immutability
5. Use idiomatic HLX constructs

---

## ⚖️ Feature 6: Constrained Grammar

**Parser-Level Structural Validation**

### What It Does
Enforces HLX grammar rules, prevents invalid constructs before compilation.

### Rule 1: Balanced Braces
**❌ Invalid**:
```hlx
fn test() {
    let x = 42;
}}  // 🔴 Unmatched closing brace '}'
```

**✅ Valid**:
```hlx
fn test() {
    let x = 42;
}
```

---

### Rule 2: Semicolon Placement
**❌ Invalid**:
```hlx
let x = 42  // 🔴 Statement must end with semicolon
```

**✅ Valid**:
```hlx
let x = 42;
```

**Exception**: Block statements don't need semicolons:
```hlx
fn test() {  // No semicolon needed
if (x > 0) {  // No semicolon needed
```

---

### Rule 3: Function Signatures
**❌ Invalid**:
```hlx
fn calculate_sum {  // 🔴 Missing parentheses for parameters
fn test()           // 🔴 Missing opening brace '{'
```

**✅ Valid**:
```hlx
fn calculate_sum(a, b) {
fn test() {
```

**Required format**: `fn name(params) {`

---

### Rule 4: Contract Syntax
**❌ Invalid**:
```hlx
let x = @ { value: 42 };     // 🔴 '@' must be followed by contract ID
let y = @ABC { value: 10 };  // 🔴 Contract ID must be digits
```

**✅ Valid**:
```hlx
let x = @200 { lhs: 5, rhs: 3 };
```

**Format**: `@<digits> { fields }`

---

### Rule 5: Variable Declarations
**❌ Invalid**:
```hlx
let x;  // 🔴 Variable declaration must include assignment
```

**✅ Valid**:
```hlx
let x = 42;
```

**Format**: `let name = value;`

---

### Rule 6: Loop Syntax
**❌ Invalid**:
```hlx
loop {                        // 🔴 Missing condition and bound
loop (i < 10) {               // 🔴 Missing bound
loop i < 10, 1000 {           // 🔴 Missing parentheses
```

**✅ Valid**:
```hlx
loop (i < 10, DEFAULT_MAX_ITER) {
```

**Format**: `loop (condition, bound) {`

---

### Strict Mode (Optional)

In strict mode, additional constructs are forbidden:

**❌ Forbidden**:
```hlx
while (condition) {  // 🔴 Use 'loop' instead of 'while'
for (i = 0; ...) {   // 🔴 Use 'loop' instead of 'for'
var x = 10;          // 🔴 Use 'let' instead of 'var'
goto label;          // 🔴 'goto' is not allowed in HLX
```

**Why**: HLX enforces structured programming with `loop` as the universal iteration construct.

---

### Grammar Error Messages

All errors include:
- **Clear message**: What's wrong
- **Fix suggestion**: How to fix it
- **One-click fix**: Apply automatically

Example:
```
🔴 Loop must specify both condition and bound separated by comma
💡 Add ', DEFAULT_MAX_ITER' before closing parenthesis
```

---

## 🎓 Learning Path for AI Models

### 1. Basic Syntax (Grammar + Auto-Correction)
```hlx
// All statements end with semicolons
let x = 42;
let y = @200 { lhs: x, rhs: 10 };

// Functions use fn keyword
fn add(a, b) {
    return @200 { lhs: a, rhs: b };
}

// Loops always have bounds
loop (i < 100, DEFAULT_MAX_ITER) {
    // ...
}
```

### 2. Contract Discovery (Suggestion Engine)
```hlx
// Math: @200-299
// Strings: @300-399
// Arrays: @400-499
// I/O: @600-699
// GPU: @900+

// Use comments for discovery:
// matrix multiply
// → Suggests @906
```

### 3. Verification (Inline Preview + State Viz)
```hlx
// Test your understanding:
let test = @200 { lhs: 5, rhs: 3 };  → 8  ✓

// Track state:
let x = 0;      : int = 0
x = 5;          : int = 5
```

### 4. Best Practices (Semantic Diff)
```hlx
// ✓ Always use contracts
let sum = @200 { lhs: a, rhs: b };

// ✓ Check for edge cases
if (divisor != 0) {
    let ratio = @203 { lhs: dividend, rhs: divisor };
}

// ✓ Prefer immutability
let x = 10;
let x_updated = x + 5;  // New variable
```

---

## 🚀 IDE Integration

### VS Code
All features work automatically with the HLX LSP extension:
- Install: `code --install-extension hlx.hlx-language-server`
- Reload: `Ctrl+Shift+P` → "Reload Window"

### Neovim
```lua
require('lspconfig').hlx_lsp.setup{}
```

### Emacs
```elisp
(use-package lsp-mode
  :hook (hlx-mode . lsp))
```

---

## 📈 Performance

All features run **in real-time** with minimal latency:
- **Diagnostics**: <10ms per file
- **Completions**: <5ms
- **Inlay hints**: <20ms
- **Code actions**: <5ms

Optimized for:
- Large files (10,000+ lines)
- Frequent edits
- Low memory usage

---

## 🐛 Troubleshooting

### Suggestions Not Appearing
1. Check contract catalogue is loaded:
   - Set `HLX_CONTRACT_CATALOGUE` env var
   - Or place `CONTRACT_CATALOGUE.json` in project root

### Inline Previews Not Showing
1. Enable inlay hints in your IDE:
   - VS Code: `"editor.inlayHints.enabled": true`
   - Neovim: `:lua vim.lsp.inlay_hint(0, true)`

### Auto-Corrections Too Aggressive
1. Disable specific rules in `.hlxrc`:
   ```json
   {
     "lsp": {
       "autoCorrect": {
         "loopBounds": false
       }
     }
   }
   ```

---

## 🎯 Next Steps

See **Phase 3 Features** for upcoming enhancements:
- Refactoring Suite (rename, extract function)
- Smart Navigation (go to definition, find references)
- Performance Lens (execution cost estimates)
- Interactive Contract Explorer

---

**Documentation Version**: 1.0.0
**LSP Version**: Phase 2 Complete
**Last Updated**: 2026-01-08
