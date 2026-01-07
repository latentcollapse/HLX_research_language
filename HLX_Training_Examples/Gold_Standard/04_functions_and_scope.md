# Lesson 04: Functions, Scope, and Side-Effect Safety

## Concept
HLX functions enforce **referential transparency** as much as possible through its value semantics.
When you pass an object to a function, you are passing the *data*, not the *capability to mutate* the caller's data.

## The Rules
1.  **Arguments are Pass-by-Value**: If a function modifies an argument, it is modifying a *local copy*. The change is lost when the function returns (unless returned explicitly).
2.  **Shadowing**: `let x = ...` inside a block hides the outer `x`. This is safe and prevents accidental overwrites.
3.  **Return Values**: To propagate changes, functions must return new values/objects.

## Fine-Tuning Data (JSON)

```json
[
  {
    "instruction": "Explain why this HLX code prints 10, not 20.",
    "input": "fn change(a) { a = 20; }\nlet x = 10;\nchange(x);\nprint(x);",
    "output": "HLX uses pass-by-value. The function 'change' receives a copy of 'x'. Modifying 'a' inside the function only affects the local copy. The outer 'x' remains 10."
  },
  {
    "instruction": "Write an HLX function that takes a point {x, y} and returns a new point with x increased by 1.",
    "output": "fn move_right(p) {\n  let new_p = p;\n  new_p.x = new_p.x + 1;\n  return new_p;\n}"
  },
  {
    "instruction": "Demonstrate variable shadowing.",
    "output": "let a = 1;\n{\n  let a = 2; // Shadows outer 'a'\n  print(a); // Prints 2\n}\nprint(a); // Prints 1"
  }
]
```

## Why This Matters
For AI, tracking **Side Effects** is the hardest part of reasoning about code.
"Does calling `process_data(data)` clear the database?"
In many languages, you don't know without reading the function code.
In HLX, if `process_data` doesn't return anything or talk to an external system explicitly, you know `data` is safe.
This makes "Chain of Thought" reasoning much more robust.

```