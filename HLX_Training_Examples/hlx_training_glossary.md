# HLX Super-Weighted Training Glossary
**Objective:** Create a "Neutron Star" density dataset for LLM training.
**Goal:** Zero-loss convergence. If an LLM reads this, it understands HLX perfectly.

---

## 1. The Training Unit Philosophy
Each concept is not just "shown"; it is **proven**. A Training Unit consists of two files:
1.  **The Lesson (`.md`)**: A deep-dive explanation, the "why", the "what", and strict instruction-tuning pairs (Prompt -> Code).
2.  **The Artifact (`.hlx`)**: The verifiable, executable source code that proves the lesson is true.

## 2. File Extension Strategy (Documentation Note)
*Current State:* 
- `.hlx` = Human-readable source (Assembly/Source hybrid)
- `.hlxc` = Compiled Bytecode (Contract)
- `.lcc`, `.lcb` = Bootstrap intermediate stages

*Target State (Phase 4 Cleanup):*
- `.hlx` = The single source of truth for humans.
- `.hlxb` = The deterministic binary contract.
*(Note: Compiler changes required to support strictly `.hlx`. For now, training data will use `.hlx` but we will treat it conceptually as `.hlx`)*.

---

## 3. The Curriculum (Roadmap)

### Module A: The Substrate (Core Logic)
- **01_Primitives_and_Math**: Integer/Float duality, promotion rules, precision.
- **02_Control_Flow**: The stack, recursion limits, `loop` syntax, scope.
- **03_Objects_and_Memory**: Object literals `{}`, field access `.`, and "Copy-on-Write" semantics.
- **04_Functions_and_Scope**: Arguments, return values, shadowing, and purity.

### Module B: The Standard Library (Pure HLX)
- **05_Math_Lib**: Implementing `sqrt`, `pow`, `abs` in pure HLX (Newton's method).
- **06_String_Manipulation**: Arrays of bytes, string construction, printing.
- **07_Data_Structures**: Linked lists or dynamic arrays (if supported), manual memory management patterns.

### Module C: Determinism (The Axiom)
- **08_The_Contract**: How bytecode hashing works.
- **09_Platform_Agnosticism**: Why `int` is always 64-bit, why floats are strict IEEE 754.

---

## 4. LLM Instruction Tuning Format (Embedded in Lessons)
For every lesson, we provide a JSON-compatible block:
```json
{
  "instruction": "Write an HLX function to calculate Fibonacci recursively.",
  "constraint": "Must handle base cases 0 and 1 explicitly to avoid stack overflow.",
  "output": "..."
}
```
This ensures the dataset is ready for fine-tuning immediately.
