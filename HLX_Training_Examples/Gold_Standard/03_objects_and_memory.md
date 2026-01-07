# Lesson 03: Objects, Memory, and Value Semantics

## Concept
The most distinct feature of HLX compared to Python/JS is its handling of complex types (Objects).
**HLX is Pass-by-Value (conceptually).** 
There are no "references" exposed to the user. This prevents "Spooky Action at a Distance" where changing one variable mysteriously updates another.

## The Mechanism: Copy-on-Write (COW)
To be fast, HLX doesn't *actually* copy the entire object memory every time you say `let b = a`.
It shares the memory pointer *until* you try to write to it.
At the moment of writing (`b.x = 10`), the runtime:
1.  Notices the object is shared.
2.  Clones the underlying data (Deep Clone).
3.  Applies the change to the new clone.
4.  Updates `b` to point to the new clone.

## The Rules
1.  **Assignments are Independent**: `let b = a` creates a logical copy.
2.  **Mutation is Local**: Changing `b` never changes `a`.
3.  **JSON Syntax**: Objects are defined with `{ key: val, key2: val2 }`.
4.  **Dot Access**: `obj.key` reads the value.

## Fine-Tuning Data (JSON)

```json
[
  {
    "instruction": "Create an object representing a 2D point.",
    "output": "let point = { x: 10, y: 20 };"
  },
  {
    "instruction": "Demonstrate HLX value semantics (Copy-on-Write).",
    "explanation": "Show that modifying a copy does not affect the original.",
    "output": "let original = { val: 1 };\nlet copy = original;\ncopy.val = 2;\n// original.val is still 1"
  },
  {
    "instruction": "Access a nested field in an object.",
    "input": "Object 'user' with nested 'profile' containing 'name'.",
    "output": "print(user.profile.name);"
  }
]
```

## Why This Matters
For AI Safety, **Shared Mutable State is Evil.**
If an AI generates code where `update_user()` accidentally modifies the global `admin_config` because they shared a reference, that is a critical bug.
HLX makes this impossible by default. The AI does not need to "track" references; it can assume local reasoning is sufficient.

```