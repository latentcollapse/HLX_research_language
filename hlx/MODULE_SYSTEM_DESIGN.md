# HLX Module System Design

**Status**: Design Phase
**Philosophy**: Foundationalist - Build it right, not fast
**Goal**: Enable clean separation and composition of HLX programs

---

## Overview

The HLX module system enables:
- **Separation of concerns**: Split large programs into focused modules
- **Code reuse**: Import functions from other files
- **Namespace hygiene**: Explicit exports prevent name pollution
- **Compile-time linking**: All resolution happens at compile time (no runtime overhead)

---

## Syntax

### Export Declaration

Functions can be marked for export using the `export` keyword:

```hlx
// math.hlxa
program math_lib {
    export fn add(a: i64, b: i64) -> i64 {
        return a + b;
    }

    export fn multiply(a: i64, b: i64) -> i64 {
        return a * b;
    }

    // Private function (not exported)
    fn internal_helper(x: i64) -> i64 {
        return x * 2;
    }
}
```

### Import Declaration

Import specific functions from other modules:

```hlx
// main.hlxa
import { add, multiply } from "./math.hlxa";

program main {
    fn main() -> i64 {
        let sum = add(10, 20);
        let product = multiply(5, 6);
        return sum + product;
    }
}
```

**Syntax Rules:**
- Import statements must appear at the top of the file, before the `program` declaration
- Import paths are relative to the current file
- Import paths must use `.hlxa` extension
- Imported names must be explicitly listed (no wildcard imports for v1)

---

## Semantics

### Module Resolution

1. **Path Resolution**:
   - Relative paths: `"./foo.hlxa"` → same directory
   - Parent paths: `"../bar.hlxa"` → parent directory
   - No absolute paths (for portability)

2. **File Loading**:
   - Compiler reads and parses imported files
   - Circular imports are detected and rejected
   - Each file is compiled once (caching)

3. **Name Binding**:
   - Imported names are bound in the importing module's scope
   - Name conflicts are compile-time errors
   - No automatic namespacing (explicit imports only)

### Export Visibility

Only functions marked with `export` are visible to other modules:

```hlx
program utils {
    export fn public_api() -> i64 { ... }  // ✓ Can be imported

    fn private_impl() -> i64 { ... }       // ✗ Cannot be imported
}
```

### Compilation Model

**Single-file compilation (current)**:
```
source.hlxa → parse → lower → emit → binary.hlxb
```

**Multi-file compilation (with modules)**:
```
main.hlxa → parse → discover imports
    ├→ foo.hlxa → parse → extract exports
    └→ bar.hlxa → parse → extract exports
         → link all → lower → emit → binary.hlxb
```

**Key properties:**
- All module resolution happens at compile time
- Imported functions are inlined into the main program
- No runtime module loader needed
- Binary output is self-contained

---

## AST Changes

### Import Node

```rust
// In AST
enum Item {
    Import {
        names: Vec<String>,      // ["add", "multiply"]
        path: String,            // "./math.hlxa"
    },
    Statement(Statement),
}
```

### Export Flag

```rust
// In AST
enum Statement {
    Block {
        name: String,
        params: Vec<String>,
        body: Vec<Statement>,
        is_export: bool,         // NEW: export flag
    },
    // ... other statements
}
```

---

## Bytecode Changes

**No changes needed!**

The module system is purely a compile-time feature. After linking:
- All imported functions are resolved to direct function calls
- No module metadata in bytecode
- No runtime overhead

This preserves all four axioms:
- **A1 (Determinism)**: Module resolution is deterministic
- **A2 (Reversibility)**: Can lift bytecode back to source (imports are in original source)
- **A3 (Bijection)**: Different imports → different linked code
- **A4 (Universal Value)**: Module resolution is context-independent

---

## Implementation Plan

### Phase 1: Lexer (hlx_compiler/src/lexer.rs)

Add three new keywords:
- `import` (TOK_IMPORT)
- `export` (TOK_EXPORT)
- `from` (TOK_FROM)

**Changes needed:**
- Add keyword detection in lexer
- Assign token kinds (extend existing number sequence)

---

### Phase 2: Parser (hlx_compiler/src/parser.rs)

Add parsing for:

1. **Import statements**:
   ```
   import_stmt := "import" "{" ident_list "}" "from" string_literal ";"
   ident_list := ident ("," ident)*
   ```

2. **Export functions**:
   ```
   export_fn := "export" "fn" ident "(" params ")" "->" type block
   ```

**Changes needed:**
- Add `parse_import()` function
- Modify `parse_statement()` to detect `export` keyword
- Add `Import` variant to `Item` enum
- Add `is_export` field to function blocks

---

### Phase 3: Module Resolution (hlx_compiler/src/module.rs - NEW FILE)

Create new module resolver:

```rust
struct ModuleResolver {
    modules: HashMap<PathBuf, Module>,
    dependency_graph: Vec<(PathBuf, PathBuf)>,
}

struct Module {
    path: PathBuf,
    ast: Program,
    exports: HashMap<String, FunctionDef>,
}

impl ModuleResolver {
    fn resolve(entry_point: &Path) -> Result<LinkedProgram>;
    fn load_module(path: &Path) -> Result<Module>;
    fn check_circular_deps() -> Result<()>;
    fn link_imports(program: &mut Program) -> Result<()>;
}
```

**Algorithm:**
1. Parse entry point file
2. Extract import declarations
3. Recursively load imported modules
4. Build dependency graph
5. Check for circular dependencies
6. Link imported names to function definitions
7. Merge all functions into single program AST
8. Proceed with existing lowering

---

### Phase 4: Compiler Integration (hlx_compiler/src/compiler.rs)

Modify compilation pipeline:

**Before:**
```rust
pub fn compile(source: &str) -> Result<Vec<u8>> {
    let ast = parse(source)?;
    let bytecode = lower(ast)?;
    Ok(emit(bytecode))
}
```

**After:**
```rust
pub fn compile_file(path: &Path) -> Result<Vec<u8>> {
    let linked_ast = ModuleResolver::resolve(path)?;  // NEW
    let bytecode = lower(linked_ast)?;
    Ok(emit(bytecode))
}
```

---

## Example: Bootstrap Compiler with Modules

### Before (monolithic):
```
compiler.hlxa (2500 lines)
  - tokenize() function
  - parse_program() function
  - lower_program() function
  - emit_bytecode() function
  - compile() function
```

### After (modular):

**lexer.hlxa**:
```hlx
program hlx_lexer {
    export fn tokenize(source: String) -> [i64] {
        // 400 lines of lexer logic
    }
}
```

**parser.hlxa**:
```hlx
program hlx_parser {
    export fn parse_program(tokens: [i64]) -> [i64] {
        // 800 lines of parser logic
    }
}
```

**lower.hlxa**:
```hlx
program hlx_lowerer {
    export fn lower_program(ast: [i64]) -> [i64] {
        // 800 lines of lowering logic
    }
}
```

**emit.hlxa**:
```hlx
program hlx_emitter {
    export fn emit_bytecode(instructions: [i64]) -> [i64] {
        // 300 lines of emission logic
    }
}
```

**compiler.hlxa** (main):
```hlx
import { tokenize } from "./lexer.hlxa";
import { parse_program } from "./parser.hlxa";
import { lower_program } from "./lower.hlxa";
import { emit_bytecode } from "./emit.hlxa";

program bootstrap_compiler {
    fn compile(source: String) -> [i64] {
        let tokens = tokenize(source);
        let ast = parse_program(tokens);
        let bytecode = lower_program(ast);
        return emit_bytecode(bytecode);
    }

    fn main() -> i64 {
        // Test self-compilation
    }
}
```

---

## Error Handling

### Compile-time Errors

1. **Import not found**:
   ```
   Error: Cannot find module './foo.hlxa'
     --> main.hlxa:1:24
      |
    1 | import { bar } from "./foo.hlxa";
      |                        ^^^^^^^^^^
   ```

2. **Exported function not found**:
   ```
   Error: Function 'bar' not exported by './foo.hlxa'
     --> main.hlxa:1:10
      |
    1 | import { bar } from "./foo.hlxa";
      |          ^^^
   ```

3. **Circular dependency**:
   ```
   Error: Circular module dependency detected
     --> a.hlxa imports b.hlxa
         b.hlxa imports c.hlxa
         c.hlxa imports a.hlxa
   ```

4. **Name conflict**:
   ```
   Error: Function 'foo' imported from multiple modules
     --> main.hlxa:2:10
      |
    1 | import { foo } from "./a.hlxa";
    2 | import { foo } from "./b.hlxa";
      |          ^^^
   ```

---

## Testing Strategy

### Test 1: Simple Import/Export
```hlx
// utils.hlxa
program utils {
    export fn double(x: i64) -> i64 {
        return x * 2;
    }
}

// main.hlxa
import { double } from "./utils.hlxa";
program main {
    fn main() -> i64 {
        return double(21); // Should return 42
    }
}
```

### Test 2: Multiple Imports
```hlx
import { add } from "./math.hlxa";
import { print_result } from "./io.hlxa";
```

### Test 3: Circular Dependency Detection
```hlx
// a.hlxa imports b.hlxa
// b.hlxa imports a.hlxa
// Should fail with error
```

### Test 4: Bootstrap Compiler
```hlx
// The ultimate test: 4-file modular bootstrap compiler
import { tokenize } from "./lexer.hlxa";
import { parse_program } from "./parser.hlxa";
import { lower_program } from "./lower.hlxa";
import { emit_bytecode } from "./emit.hlxa";
```

---

## Future Enhancements (Not in v1)

1. **Wildcard imports**: `import * from "./foo.hlxa";`
2. **Aliasing**: `import { foo as bar } from "./baz.hlxa";`
3. **Re-exports**: `export { foo } from "./bar.hlxa";`
4. **Type exports**: `export type Point = [i64, i64];`
5. **Nested paths**: `import { foo } from "./lib/utils.hlxa";`
6. **Package system**: `import { foo } from "stdlib/math";`

---

## Rationale: Why This Design?

### Explicit over implicit
- No wildcard imports: Forces clear dependencies
- No automatic module discovery: Explicit import paths

### Compile-time over runtime
- All linking at compile time: No runtime module loader
- Self-contained binaries: No external dependencies

### Simple over complex
- No module namespaces (yet): Just direct function imports
- No version management (yet): Simple file-based modules
- No macro system (yet): Functions only

### Foundation first
- Get the core right: Import/export semantics
- Extensible: Easy to add features later
- Test with real code: Bootstrap compiler is perfect test case

---

## Success Criteria

Module system is complete when:
1. ✓ Can compile multi-file HLX programs
2. ✓ Bootstrap compiler works as 4 separate files
3. ✓ Self-compilation succeeds with modular compiler
4. ✓ Circular dependencies are detected
5. ✓ Error messages are clear and actionable

---

## Next Steps

1. Implement lexer changes (add keywords)
2. Implement parser changes (parse import/export)
3. Implement module resolver
4. Integrate into compiler pipeline
5. Test with bootstrap compiler
6. Achieve self-compilation! 🎉
