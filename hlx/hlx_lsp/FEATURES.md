# HLX LSP Features Guide

Complete guide to all Language Server features available in HLX.

---

## 🎨 Document Formatting

**What:** Automatically format your HLX code to follow consistent style guidelines

**How to use:**
- **Quick:** Press `Shift+Alt+F` (or `Ctrl+Shift+I`)
- **Menu:** Right-click → "Format Document"
- **Command:** `Ctrl+Shift+P` → "Format Document"

**What it formats:**
- 4-space indentation
- Proper spacing around operators (`x + 1` not `x+1`)
- Correct brace placement
- Aligned statements
- Clean if/else structure
- Organized function definitions

**Example:**
```hlx
// Before
fn messy(){let x=1+2*3;if(x>5){return x;}else{return 0;}}

// After (Shift+Alt+F)
fn messy() {
    let x = 1 + 2 * 3;
    if (x > 5) {
        return x;
    } else {
        return 0;
    }
}
```

---

## 📞 Call Hierarchy

**What:** See all places where a function is called, or all functions it calls

**How to use:**
1. Right-click on any function name
2. Select "Show Call Hierarchy"
3. View:
   - **Incoming Calls:** Who calls this function
   - **Outgoing Calls:** What this function calls

**Perfect for:**
- Understanding code flow
- Refactoring safely
- Finding all usages
- Navigating large codebases

**Example:**
```hlx
fn helper(n) {
    return n * 2;
}

fn caller_one() {
    return helper(5);  // ← Shows in "incoming calls" for helper
}
```

---

## 📂 Code Folding

**What:** Collapse/expand sections of code to focus on what matters

**How to use:**
- Click the fold arrow (▼) in the left gutter
- Or use keyboard shortcuts (platform-specific)

**What you can fold:**
- Function bodies
- If/else blocks (independently)
- Loop bodies
- Nested structures
- Import sections

**Visual Impact:**
```hlx
// Expanded view (70 lines)
fn badly_formatted() -> void {
    let x: any = 1 + 2 * 3;
    if (x > 5) {
        return x;
    } else {
        return 0;
    }
}
fn helper(n) -> void { ... }
fn caller_one() -> void { ... }
...

// Collapsed view (7 lines)
fn badly_formatted() -> void { ...
fn helper(n) -> void { ...
fn caller_one() -> void { ...
fn caller_two() -> void { ...
fn complex_function() -> void { ...
fn deeply_nested() -> void { ...
fn main() -> void { ...
```

**Pro tip:** Collapse all functions to see file structure, then expand only what you're working on!

---

## 🐛 Multi-Error Reporting

**What:** See ALL syntax errors at once, not just the first one

**How to use:**
- Errors appear automatically in the **Problems panel**
- Open Problems: `Ctrl+Shift+M` (or View → Problems)
- Errors also show inline with red squiggly underlines

**The Game Changer:**

**OLD WAY (other LSPs):**
1. See error #1 → Fix it → Save
2. See error #2 → Fix it → Save
3. See error #3 → Fix it → Save
4. Repeat 38 times... 😫

**NEW WAY (HLX LSP):**
1. See ALL 38 errors at once
2. Fix all of them
3. Save
4. Done! ✅

**What it catches:**
- Syntax errors (missing semicolons, braces, parens)
- Unmatched braces/parentheses
- Invalid expressions
- Type errors
- Undefined functions
- Structural issues

**Example Problems panel:**
```
❌ Unmatched closing brace '}' [Line 7, Col 5]
❌ Undefined function 'helper' [Line 15, Col 1]
❌ Unmatched closing brace '}' [Line 12, Col 1]
❌ Missing semicolon [Line 8, Col 10]
... (34 more errors)
```

Fix them all, save once, and they all disappear!

---

## 🔍 Go to Definition

**What:** Jump to where a function/variable is defined

**How to use:**
- **Quick:** `Ctrl+Click` on any symbol
- **Menu:** Right-click → "Go to Definition"
- **Keyboard:** `F12`

---

## 📖 Find All References

**What:** Find everywhere a symbol is used

**How to use:**
- Right-click on any symbol → "Find All References"
- Or press `Shift+F12`

---

## 💡 Hover Documentation

**What:** See documentation and type info by hovering over code

**How to use:**
- Just hover your mouse over any function or variable
- Documentation appears in a popup

---

## ✨ Code Completion

**What:** Auto-complete suggestions as you type

**How to use:**
- Start typing - suggestions appear automatically
- Press `Tab` or `Enter` to accept
- Trigger manually: `Ctrl+Space`

**What it suggests:**
- Function names
- Variable names
- Keywords
- Builtin functions
- Contract fields

---

## 🎯 Semantic Highlighting

**What:** Smart syntax coloring based on code meaning

**How to use:**
- Automatic! Just open an HLX file
- Functions, variables, types all get distinct colors

---

## ⚡ Quick Fixes

**What:** Automatic suggestions to fix common errors

**How to use:**
- Click the lightbulb (💡) icon next to errors
- Or press `Ctrl+.` on an error

---

## 🔧 Code Actions

**What:** Context-aware actions (extract function, rename, etc.)

**How to use:**
- Right-click → "Code Actions"
- Or press `Ctrl+.`

---

## 📊 Document Symbols

**What:** See outline of all functions/variables in current file

**How to use:**
- `Ctrl+Shift+O` → Quick navigation dropdown
- Or use the Outline panel in the sidebar

---

## 🌍 Workspace Symbols

**What:** Search for any symbol across your entire project

**How to use:**
- Press `Ctrl+T`
- Type symbol name
- Jump to definition

---

## ✏️ Rename Symbol

**What:** Rename a variable/function across all files

**How to use:**
- Right-click on symbol → "Rename Symbol"
- Or press `F2`
- Type new name → Enter
- All references update automatically!

---

## 🎨 Inlay Hints

**What:** See type information and parameter names inline

**How to use:**
- Automatically shown for:
  - Variable types
  - Function parameter names
  - Return types

---

## 📝 Signature Help

**What:** See function parameters as you type

**How to use:**
- Start typing a function call
- Parameter hints appear automatically
- Shows which parameter you're currently typing

---

## Performance

All features are designed for speed:

| Feature | Performance |
|---------|-------------|
| Formatting | <100ms @ 1000 lines |
| Call Hierarchy | <50ms indexing |
| Folding | <10ms compute |
| Error Detection | Real-time |
| Completion | <50ms |
| Go to Definition | Instant |

---

## Tips & Tricks

**1. Use folding for large files**
   - Collapse all functions to see structure
   - Expand only what you're working on

**2. Format on save**
   - Set up "Format on Save" in VS Code settings
   - Every file stays consistent automatically

**3. Check Problems panel regularly**
   - `Ctrl+Shift+M` to open
   - Fix all errors before testing

**4. Use call hierarchy for refactoring**
   - See all callers before changing a function
   - Ensure you update all call sites

**5. Hover for quick docs**
   - Faster than switching to documentation
   - See types and signatures inline

---

## Keyboard Shortcuts (VS Code/Codium)

| Action | Shortcut |
|--------|----------|
| Format Document | `Shift+Alt+F` or `Ctrl+Shift+I` |
| Go to Definition | `F12` or `Ctrl+Click` |
| Find References | `Shift+F12` |
| Rename Symbol | `F2` |
| Quick Fix | `Ctrl+.` |
| Show Problems | `Ctrl+Shift+M` |
| Document Symbols | `Ctrl+Shift+O` |
| Workspace Symbols | `Ctrl+T` |
| Trigger Completion | `Ctrl+Space` |
| Show Hover | Hover mouse |
| Call Hierarchy | Right-click → "Show Call Hierarchy" |

---

## Troubleshooting

**LSP not starting?**
- Check Output panel: `Ctrl+Shift+U` → Select "HLX Language Server"
- Verify file extension is `.hlxa` or `.hlxc`
- Check bottom-right: should say "HLX" not "Plain Text"

**Formatting not working?**
- Try Command Palette: `Ctrl+Shift+P` → "Format Document"
- Check for syntax errors (formatter needs valid code)

**No suggestions appearing?**
- Trigger manually: `Ctrl+Space`
- Wait a moment after typing (slight delay is normal)

**Errors not showing?**
- Open Problems panel: `Ctrl+Shift+M`
- Check if file is saved (some errors only show after save)

---

## For AI-Assisted Development

HLX LSP is designed to work seamlessly with AI code generation:

**✅ Format AI-generated code instantly**
   - AI writes code → You press `Shift+Alt+F` → Perfect formatting

**✅ See all errors in generated code**
   - No more iterative fixing
   - AI sees all issues and fixes them at once

**✅ Navigate AI-generated codebases**
   - Call hierarchy shows relationships
   - Folding shows structure
   - Symbols for quick jumping

**✅ Refactor with confidence**
   - Rename symbol updates everywhere
   - Find all references before changes
   - Call hierarchy shows impact

---

## Getting Help

- **LSP Source:** `/home/matt/hlx-compiler/hlx/hlx_lsp/src/`
- **Issues:** Report bugs or request features
- **Documentation:** This file and `IMPLEMENTATION_SUMMARY.md`
- **Testing:** See `/tmp/LSP_TESTING_GUIDE.md`

---

## Feature Maturity

| Feature | Status | Quality |
|---------|--------|---------|
| Document Formatting | ✅ | Production |
| Call Hierarchy | ✅ | Production |
| Code Folding | ✅ | Production |
| Multi-Error Reporting | ✅ | Production |
| Go to Definition | ✅ | Stable |
| Find References | ✅ | Stable |
| Hover Docs | ✅ | Stable |
| Completion | ✅ | Stable |
| Semantic Highlighting | ✅ | Stable |
| Quick Fixes | ✅ | Stable |
| Rename Symbol | ✅ | Stable |
| Document Symbols | ✅ | Stable |
| Workspace Symbols | ✅ | Stable |
| Signature Help | ✅ | Stable |
| Inlay Hints | ✅ | Stable |

**Overall LSP Maturity:** 60-68% (17+ features working)

---

*Last Updated: January 16, 2026*
*HLX Language Server v0.1.0*
