# HLX Semantic Analysis Design

## Overview

Semantic analysis is the compilation phase that checks **meaning**, not just syntax. It comes after parsing and before lowering, catching bugs that syntax checkers miss.

## Goals (The 4 Axioms Apply Here Too!)

1. **Determinism (A1)**: Same program always produces same semantic errors
2. **Reversibility (A2)**: Type information flows bidirectionally (can reconstruct types)
3. **Bijection (A3)**: Different type errors → different error codes
4. **Universal Value (A4)**: Type checking is context-independent

---

## Phase 1: Scope Tracking

### What We Check

**1. Variable Scope**
```hlx
fn example() {
    let x = 5;
    print(x);      // ✓ OK - x is in scope
    print(y);      // ✗ ERROR - y is undefined

    {
        let z = 10;
        print(z);  // ✓ OK - z is in scope here
    }
    print(z);      // ✗ ERROR - z out of scope
}
```

**2. Function Scope**
```hlx
fn caller() {
    result = foo(42);  // ✓ OK - foo is defined
    result = bar(42);  // ✗ ERROR - bar is not defined
}

fn foo(x) { return x * 2; }
```

**3. Shadowing**
```hlx
let x = 5;
{
    let x = 10;    // ✓ OK - shadows outer x
    print(x);      // 10
}
print(x);          // 5
```

### Data Structures

**Symbol Table (Scope Stack)**
```
[Global Scope]
  ├─ x: { name: "x", type: i64, line: 10, assigned: true }
  ├─ foo: { name: "foo", type: function, params: [i64], returns: i64 }
  └─ [Block Scope]
      ├─ y: { name: "y", type: String, line: 15, assigned: false }
      └─ [Block Scope]
          └─ z: { name: "z", type: array, line: 20 }
```

**Entry in Symbol Table**
```
{
  name: String,           // Variable name
  type: Type,             // i64, String, array, function, etc.
  scope_level: i64,       // 0 = global, 1+ = nested block
  line: i64,              // Line number for error reporting
  column: i64,            // Column number for error reporting
  assigned: i64,          // 1 if assigned before use, 0 otherwise
  is_parameter: i64,      // 1 if function parameter
  is_function: i64,       // 1 if this is a function name
  arity: i64,             // Number of parameters (if function)
}
```

### Algorithm

```
fn analyze_program(ast: Program) -> SemanticResult {
    let global_scope = create_scope(0);

    // Phase 1: Collect all function definitions
    for each function in ast.functions {
        add_function_to_scope(global_scope, function);
    }

    // Phase 2: Check each function body
    for each function in ast.functions {
        check_function(function, global_scope);
    }

    // Phase 3: Generate diagnostic report
    return diagnostics;
}

fn check_function(func: Function, parent_scope: Scope) -> void {
    let fn_scope = create_scope(parent_scope.level + 1, parent_scope);

    // Add parameters to scope
    for each param in func.parameters {
        add_binding(fn_scope, param.name, param.type);
    }

    // Check function body
    check_statements(func.body, fn_scope);

    // Verify function returns if non-void
    if (func.return_type != void) {
        if (!all_paths_return(func.body)) {
            error("Function must return value on all paths");
        }
    }
}

fn check_statements(stmts: [Statement], scope: Scope) -> void {
    for each stmt in stmts {
        switch stmt.type {
            LET => check_let_binding(stmt, scope);
            ASSIGN => check_assignment(stmt, scope);
            IF => check_if_statement(stmt, scope);
            LOOP => check_loop_statement(stmt, scope);
            CALL => check_function_call(stmt, scope);
            RETURN => check_return_statement(stmt, scope);
        }
    }
}

fn check_let_binding(stmt: LetStatement, scope: Scope) {
    let var_name = stmt.name;

    // Check variable not already declared in this scope
    if (lookup_in_current_scope(scope, var_name) != null) {
        error("Variable already declared", stmt.line);
    }

    // Check initializer expression
    check_expression(stmt.value, scope);

    // Infer or validate type
    let expr_type = infer_type(stmt.value, scope);

    // Add to scope
    add_binding(scope, var_name, expr_type);
}

fn check_assignment(stmt: AssignStatement, scope: Scope) {
    let lhs_name = stmt.lhs;

    // Check variable is declared
    if (lookup_binding(scope, lhs_name) == null) {
        error("Undefined variable: " + lhs_name, stmt.line);
    }

    // Check RHS type matches LHS
    check_expression(stmt.rhs, scope);
    let rhs_type = infer_type(stmt.rhs, scope);
    let lhs_type = lookup_type(scope, lhs_name);

    if (!types_compatible(lhs_type, rhs_type)) {
        error("Type mismatch: " + lhs_type + " vs " + rhs_type, stmt.line);
    }

    // Mark variable as assigned
    mark_assigned(scope, lhs_name);
}

fn check_expression(expr: Expression, scope: Scope) -> void {
    switch expr.type {
        INT => { /* always OK */ }
        STRING => { /* always OK */ }
        IDENT => {
            if (lookup_binding(scope, expr.name) == null) {
                error("Undefined: " + expr.name, expr.line);
            }
            if (!is_assigned(scope, expr.name)) {
                error("Used before assignment: " + expr.name, expr.line);
            }
        }
        BINOP => {
            check_expression(expr.lhs, scope);
            check_expression(expr.rhs, scope);
            check_binop_types(expr.op, expr.lhs, expr.rhs, scope);
        }
        CALL => {
            check_function_call(expr, scope);
        }
        ARRAY => {
            for each elem in expr.elements {
                check_expression(elem, scope);
            }
        }
        INDEX => {
            check_expression(expr.array, scope);
            check_expression(expr.index, scope);

            let array_type = infer_type(expr.array, scope);
            if (array_type != ARRAY) {
                error("Cannot index non-array", expr.line);
            }
        }
    }
}

fn check_binop_types(op: Operator, lhs: Expr, rhs: Expr, scope: Scope) {
    let lhs_type = infer_type(lhs, scope);
    let rhs_type = infer_type(rhs, scope);

    switch op {
        ADD | SUB | MUL | DIV | MOD => {
            if (lhs_type != I64 || rhs_type != I64) {
                error("Arithmetic requires i64 operands", lhs.line);
            }
        }
        BIT_AND | BIT_OR | BIT_XOR | SHL | SHR => {
            if (lhs_type != I64 || rhs_type != I64) {
                error("Bitwise requires i64 operands", lhs.line);
            }
        }
        EQ | NE | LT | LE | GT | GE => {
            if (lhs_type != rhs_type) {
                error("Cannot compare different types", lhs.line);
            }
        }
        AND | OR => {
            if (lhs_type != I64 || rhs_type != I64) {
                error("Logical ops require i64", lhs.line);
            }
        }
    }
}

fn check_function_call(call: FunctionCall, scope: Scope) {
    let func_info = lookup_function(scope, call.func_name);

    if (func_info == null) {
        error("Undefined function: " + call.func_name, call.line);
    }

    if (array_len(call.args) != func_info.arity) {
        error("Function expects " + func_info.arity +
              " args, got " + array_len(call.args), call.line);
    }

    // Type check arguments
    for each arg, i in call.args {
        check_expression(arg, scope);
        let arg_type = infer_type(arg, scope);
        let param_type = func_info.params[i];

        if (!types_compatible(param_type, arg_type)) {
            error("Argument type mismatch at position " + i, arg.line);
        }
    }
}

fn check_if_statement(stmt: IfStatement, scope: Scope) {
    // Check condition
    check_expression(stmt.condition, scope);
    let cond_type = infer_type(stmt.condition, scope);

    if (cond_type != I64) {
        error("Condition must be i64", stmt.line);
    }

    // Check branches with new scopes
    let then_scope = create_scope(scope.level + 1, scope);
    check_statements(stmt.then_body, then_scope);

    if (array_len(stmt.else_body) > 0) {
        let else_scope = create_scope(scope.level + 1, scope);
        check_statements(stmt.else_body, else_scope);
    }
}

fn infer_type(expr: Expression, scope: Scope) -> Type {
    switch expr.type {
        INT => return I64;
        STRING => return STRING;
        IDENT => {
            let binding = lookup_binding(scope, expr.name);
            return binding.type;
        }
        ARRAY => return ARRAY;
        CALL => {
            let func = lookup_function(scope, expr.func_name);
            return func.return_type;
        }
        BINOP => {
            switch expr.op {
                ADD | SUB | MUL | DIV | MOD | BIT_* | SHL | SHR => return I64;
                EQ | NE | LT | LE | GT | GE | AND | OR => return I64;
            }
        }
        INDEX => return I64;  // Arrays contain i64
    }
    return UNKNOWN;
}
```

---

## Phase 2: Type Checking

### Type System

**Basic Types**
```
i64       - 64-bit signed integer
String    - Text string
[i64]     - Array of i64s
function  - Function type
null      - Null/none value
```

**Type Compatibility Rules**
```
i64 ↔ i64     ✓ Compatible
String ↔ String ✓ Compatible
[i64] ↔ [i64]  ✓ Compatible
i64 ↔ String   ✗ Incompatible
```

### Type Inference

We infer types from:
1. Literal values (42 → i64, "hello" → String)
2. Function return types
3. Array contents
4. Expression results

### Type Validation

We validate:
1. Binary operations (can't add string to int)
2. Function calls (arguments match parameter types)
3. Array access (index must be i64)
4. Assignments (RHS type matches LHS)

---

## Phase 3: Error Diagnostics

### Error Information

Each error contains:
```hlx
{
  code: String,           // e.g., "E_UNDEFINED_VAR"
  message: String,        // Human-readable message
  line: i64,              // Line number
  column: i64,            // Column number
  context: String,        // Excerpt of source code
  suggestion: String,     // How to fix it (optional)
}
```

### Error Codes

```
E_UNDEFINED_VAR       - Variable not declared
E_UNDEFINED_FUNCTION  - Function not defined
E_DUPLICATE_BINDING   - Variable already declared
E_TYPE_MISMATCH       - Types incompatible
E_ARITY_MISMATCH      - Wrong number of arguments
E_USED_BEFORE_ASSIGN  - Variable used before assignment
E_INDEX_NON_ARRAY     - Indexing non-array value
E_NOT_RETURNS         - Function doesn't return on all paths
E_DEAD_CODE           - Unreachable code detected
E_UNUSED_VARIABLE     - Variable declared but never used (warning)
```

### Error Messages

**Example 1:**
```
Error [E_UNDEFINED_VAR] at line 15, column 12:
  Undefined variable: result

    14 |     let x = 42;
    15 |     print(result);
         |           ^^^^^^

Suggestion: Did you mean 'x'?
```

**Example 2:**
```
Error [E_TYPE_MISMATCH] at line 8, column 10:
  Cannot add String to i64

     7 |     let sum = 0;
     8 |     sum = sum + "hello";
         |           ^^^^^^^^^^^

Left side:  i64
Right side: String

Suggestion: Convert String to i64 with to_int()
```

---

## Implementation in Bootstrap

### Current Pipeline
```
parser.hlxa ─────→ ast
lower.hlxa ─────→ instructions
emit.hlxa ─────→ bytecode
```

### New Pipeline
```
parser.hlxa ──────→ ast
semantic.hlxa ────→ semantic info + errors
                       ↓
                  [if errors, report & exit]
lower.hlxa ──────→ instructions
emit.hlxa ─────→ bytecode
```

### Semantic Module Structure

```hlx
module semantic {
    // Scope management
    fn create_scope() -> Scope
    fn enter_scope(scope) -> Scope
    fn exit_scope(scope) -> Scope
    fn add_binding(scope, name, type) -> Scope
    fn lookup_binding(scope, name) -> Binding

    // Type operations
    fn infer_type(expr, scope) -> Type
    fn types_compatible(type1, type2) -> i64

    // Type checking
    fn check_expression(expr, scope) -> SemanticResult
    fn check_statement(stmt, scope) -> SemanticResult
    fn check_function(func, scope) -> SemanticResult

    // Error management
    fn add_error(code, message, line, column)
    fn get_diagnostics() -> [Error]
    fn has_errors() -> i64

    // Main entry
    fn analyze(ast) -> SemanticResult
}
```

---

## Phases

### Phase 1: Scope Tracking (CURRENT)
- Track variable declarations
- Track function definitions
- Detect undefined variables
- Detect used-before-assigned

### Phase 2: Type Checking
- Infer types from expressions
- Validate binary operations
- Check function calls
- Detect type mismatches

### Phase 3: Error Diagnostics
- Generate human-readable errors
- Include source context
- Suggest fixes
- Track error codes for testing

### Phase 4: Advanced Checks (Future)
- Dead code detection
- Unreachable statements
- All-paths-return verification
- Unused variable warnings
- Loop bound checking

---

## Benefits for Self-Hosting

Once semantic analysis is working in the bootstrap:

1. **Self-Validation** - The bootstrap validates *itself*
2. **Better Errors** - Easier to debug compiler bugs
3. **Correctness** - Catch bugs before bytecode emission
4. **Foundation** - Essential for optimization passes later
5. **Testing** - Can test compiler behavior deterministically

---

## Next Steps

1. Implement `semantic.hlxa` with Phase 1 (scope tracking)
2. Integrate into bootstrap pipeline (between parser and lowerer)
3. Test with programs that have semantic errors
4. Add Phase 2 (type checking)
5. Add Phase 3 (diagnostics)
