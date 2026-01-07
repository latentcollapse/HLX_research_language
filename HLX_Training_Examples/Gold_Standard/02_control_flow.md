# Lesson 02: Control Flow, Stack, and Recursion

## Concept
HLX manages control flow through explicit branching (`if`/`else`) and deterministic loops. 
Unlike C or Python, HLX loops are designed with verification in mind. 
Recursion is allowed but constrained by the physical stack size (1024 frames by default).

## The Rules
1.  **Blocks are Scopes**: Variables defined inside `{ ... }` die at the closing `}`.
2.  **Explicit Returns**: Functions do not implicitly return the last expression. You must use `return`.
3.  **Loop Syntax**: `loop(condition, max_iterations)` (Note: max_iterations is currently a hint, but conceptually enforces termination).

## Fine-Tuning Data (JSON)

```json
[
  {
    "instruction": "Create a recursive function for factorial in HLX.",
    "constraint": "Include a base case for n <= 1.",
    "output": "fn factorial(n) {\n  if n <= 1 { return 1; }\n  return n * factorial(n - 1);\n}"
  },
  {
    "instruction": "Write a loop that counts from 0 to 9.",
    "output": "let i = 0;\nloop(i < 10, 100) {\n  print(i);\n  i = i + 1;\n}"
  }
]
```

## Edge Cases
- **Stack Overflow**: Infinite recursion `fn a() { a(); }` will crash the VM deterministically.
- **Shadowing**: Declaring `let x = 5` inside an `if` block shadows an outer `x`, but does not mutate it.

```