# HLX-C (Helinux Compute) Specification

**Version:** 0.1.0 (Draft)
**Status:** Experimental
**Target:** LC-B (Latent Capsule Binary)

---

## 1. Philosophy

HLX-C is the **Control Plane** language for the Helix ecosystem. While HLXL (Linear) describes *data* (tensors, shapes, pipelines), HLX-C describes *logic* (decisions, iteration, OS kernels).

It is designed to be **Turing-complete yet Deterministic**. This is achieved by enforcing strict bounds on all control flow structures at compile time.

## 2. Syntax

HLX-C borrows heavily from Rust and C, but simplifies the semantics to match the LC-B instruction set.

### 2.1 Variables & Types

Variables are immutable by default (SSA form).

```rust
// Basic types
let a: i32 = 42;
let b: f32 = 3.14;
let c: bool = true;

// Tensors (Shapes are generic/inferred)
let t1: Tensor = load("data.bin");
```

### 2.2 Functions

Functions are the primary unit of code organization. They compile to `Instruction::FuncDef` in LC-B.

```rust
fn calculate_loss(logits: Tensor, targets: Tensor) -> Tensor {
    let loss = cross_entropy(logits, targets);
    return loss;
}
```

### 2.3 Conditional Logic (If/Else)

Conditional execution is fully supported. The condition must evaluate to a scalar `bool` or `i32` (0=false).

```rust
fn activation(x: Tensor, use_gelu: bool) -> Tensor {
    // 'result' is assigned the value of the block
    let result = if use_gelu {
        gelu(x)
    } else {
        relu(x)
    };
    return result;
}
```

### 2.4 Bounded Loops (The Safety mechanism)

**CRITICAL:** Infinite loops are syntactically illegal. Every loop MUST declare a compile-time constant `max_iter`. This is the **Deterministic Loop Bound (DLB)**.

Syntax: `loop (condition, max_iter) { body }`

```rust
fn matmul_tiled(a: Tensor, b: Tensor) -> Tensor {
    let mut acc = zeros(a.shape);
    let mut i = 0;
    
    // Loop runs while i < 16, but HALTS HARD at 16 iterations 
    // even if the logic is buggy.
    loop (i < 16, 16) {
        let tile = load_tile(a, i);
        acc = acc + tile;
        i = i + 1;
    }
    
    return acc;
}
```

### 2.5 Hardware Abstraction (Kernels)

HLX-C can invoke optimized kernels using the `kernel` keyword, which maps to backend-specific SPIR-V tuning.

```rust
fn efficient_op(x: Tensor) -> Tensor {
    // Dispatches to "nvidia_kernel" or "amd_kernel" based on runtime HAL
    kernel("matrix_ops", x)
}
```

## 3. Grammar (EBNF Draft)

```ebnf
program     ::= function*
function    ::= "fn" identifier "(" params? ")" "->" type "{" block "}"
params      ::= param ("," param)*
param       ::= identifier ":" type
block       ::= statement*
statement   ::= let_stmt | return_stmt | expr_stmt | if_stmt | loop_stmt
let_stmt    ::= "let" identifier (":" type)? "=" expression ";"
return_stmt ::= "return" expression ";"
if_stmt     ::= "if" expression "{" block "}" ("else" "{" block "}")?
loop_stmt   ::= "loop" "(" expression "," integer_literal ")" "{" block "}"
expression  ::= literal | identifier | binary_op | call_expr
```

## 4. Compilation to LC-B

| HLX-C Construct | LC-B Opcode |
| :--- | :--- |
| `fn name(...)` | `FuncDef { id, instructions }` |
| `if cond { A } else { B }` | `If { cond, then: A, else: B }` |
| `loop (cond, 100) { ... }` | `Loop { cond, body, max_iter: 100 }` |
| `call(x)` | `Call { func, args }` |

## 5. The "Panic" State

If a loop exceeds its `max_iter`, the runtime enters a **Deterministic Panic**. It does not crash randomly; it returns a specific `E_LOOP_BOUND` error hash. This allows the OS kernel (Helinux) to kill the process safely without destabilizing the system.
