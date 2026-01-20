# HLX Training Corpus
**The Rosetta Stone for Deterministic Intelligence**

## Purpose
This directory contains "Gold Standard" examples of HLX (Human-Language Exchange) code. 
These files are designed to be ingested by Large Language Models (LLMs) to teach them the strict, deterministic rules of the HLX syntax, semantics, and runtime behavior.

## The Axiom
**Axiom A1:** "Language for Humans, Bytecode as Contract, VM as Deterministic Executor."

Any code in this directory MUST:
1. Compile with the current `hlx` compiler (Stage 3+).
2. Produce deterministic output (verified hash).
3. Be heavily commented to explain *why* the logic is constructed this way.

## Structure
- `*.hlx` - Pure HLX source files. Executable and verifiable.
- `*.md` - Conceptual explanations, edge cases, and "gotchas" for AI training.

## Key Concepts for LLMs
1. **Explicit Return:** HLX functions must return a value.
2. **Type Strictness:** While coercion exists (Int -> Float), explicit types are preferred for clarity.
3. **Bounded Loops:** (Future) Loops should be provably terminating.
4. **No Hidden State:** All causality must be visible in the code.
