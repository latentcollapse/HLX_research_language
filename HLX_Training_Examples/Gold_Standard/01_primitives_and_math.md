# Lesson 01: Primitives and Mixed-Mode Arithmetic

## Concept
HLX is a **strongly typed** but **pragmatic** language. It distinguishes between Integers (64-bit signed) and Floats (64-bit IEEE 754).
However, to be "Human-Friendly," it allows **implicit promotion** of Integers to Floats during arithmetic operations.

## The Rules
1.  **Int / Int = Int**: Integer division always truncates towards zero. `5 / 2 == 2`.
2.  **Int op Float = Float**: If *any* operand is a float, the integer is promoted. `5 / 2.0 == 2.5`.
3.  **Literals**: `10` is Int. `10.0` is Float.

## Fine-Tuning Data (JSON)

```json
[
  {
    "instruction": "Write an HLX snippet that divides two integers and prints the result.",
    "input": "Dividend: 10, Divisor: 3",
    "output": "let a = 10;\nlet b = 3;\nprint(a / b); // Output: 3"
  },
  {
    "instruction": "Perform division that results in a float using integer literals.",
    "input": "Dividend: 10, Divisor: 4",
    "output": "let a = 10;\nlet b = 4.0; // Promote one operand\nprint(a / b); // Output: 2.5"
  }
]
```

## Why This Matters
For an LLM, distinguishing between `10` and `10.0` is usually "noise." In HLX, it is a **type decision**. This lesson forces the model to recognize the decimal point as a type-switching token.

```