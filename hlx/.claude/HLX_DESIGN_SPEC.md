# HLX Language Design Specification

**Status:** Active Development | **Version:** 1.0-Vision | **Date:** 2026-01-19

---

## Executive Summary

**HLX** is a deterministic programming language designed to be equally natural for humans and LLMs. It combines:
- Procedural expressiveness (practical for humans)
- Contract-based values (natural for LLMs)
- Four axioms guaranteeing determinism, reversibility, bijection, and universal value
- Deterministic bytecode (LC-B format)
- Dual representation (HLX-A ASCII + HLX-R Runic) with perfect bijection

**Primary audiences:**
1. LLM agents (primary design target)
2. Human developers (secondary but important)

---

## Core Vision

### The Problem We Solve
AI systems need a language where:
- ✅ Output is always deterministic (same input = same output, always)
- ✅ Code is easy for LLMs to generate and understand
- ✅ It's hard to write incorrect code
- ✅ State is explicit (no hidden mutations)
- ✅ Representation is lossless in all transformations

### Why Not Rust?
RustD (Rust DSL) is our *reference implementation* and proof-of-concept. But Rust-like syntax:
- Is human-friendly but LLM-awkward
- Has implicit coercions that cause bugs
- Lacks contract semantics LLMs naturally understand
- Doesn't encode the bootstrap capsule concepts

---

## Language Components

### 1. HLX (The Programming Language)

**Type:** Procedural + Functional + Contract-based

**Core features:**
- Functions with parameters and returns
- Variables (bound to registers in bytecode)
- Contract-based values (embedded from bootstrap spec)
- Handles for explicit state management
- Bounded loops (prevents infinite recursion)
- Pattern matching (exhaustive, no silent defaults)
- Module system with imports/exports
- Attributes (#[no_mangle], #[entry], etc.)

**Example:**
```hlx
module process {
    fn analyze(data: {42:{@0:i64, @1:string}}) -> {14:{@0:i64}} {
        let value = data.@0;
        let text = data.@1;
        return {14:{@0:(value * 2)}};
    }
}
```

### 2. HLX-A (ASCII Representation)

**Purpose:** Human-readable form

**Syntax:**
- `program`, `module`, `fn`, `let`, `return`, `if`, `loop`, `break`, `continue`
- Type annotations with `:` (e.g., `x: i64`)
- Struct/enum definitions
- Standard operators (`+`, `-`, `*`, `/`, `&&`, `||`, `!`)
- Contract syntax: `{contract_id:{@field_id:value}}`
- Handle syntax: `⟁handle_name`

**Design goal:** Readable to both humans and LLMs

### 3. HLX-R (Runic Representation)

**Purpose:** Compressed, symbol-dense form for LLM transmission

**Symbols:** (evolved from bootstrap capsule)
- ⟠ program
- ◇ function
- ⊢ let
- ↩ return
- ❓ if
- ⟳ loop
- ⚳ collapse (value → handle)
- ⚯ mode switch
- ⟁ handle reference

**Design goal:** Highly compressible for network transfer, natural for LLM encoding

**Bijection:** Perfect lossless conversion A ↔ R

### 4. Embedded Data Format

**Name:** [TBD - "Contract Spec" or "HLX-D"]

**Purpose:** Formal specification of contract-based values that embed in HLX code

**Syntax:**
```
{contract_id:{@field_id:value, ...}}
```

**Example:**
```
{42:{@0:123, @1:"hello"}}  // Contract 42 with fields @0 and @1
{14:{@0:{42:{@0:456}}}}    // Nested contracts
```

**Design goal:** Natural structure for LLMs to generate and manipulate

---

## Four Axioms

Every valid HLX program must satisfy:

### A1: Determinism
- Same input → Same output (always)
- No randomness, no time-dependent behavior
- No floating-point surprises
- Bounded loops guarantee termination

### A2: Reversibility
- Every computation is traceable
- You can step backward through execution
- State at each step is inspectable
- Bijection A ↔ R means no information loss

### A3: Bijection
- HLX-A ↔ HLX-R perfect conversion
- Different programs → Different bytecode
- Same semantics → Same bytecode hash
- Value ↔ Handle mapping is 1:1

### A4: Universal Value
- All state is explicit
- No implicit conversions or coercions
- Contract fields are mandatory (exhaustive)
- Type checking is complete at compile time

---

## Compilation Pipeline

```
HLX-A Source
    ↓
[Parser] → AST
    ↓
[Semantic Analyzer] → Validated AST (checks A1-A4)
    ↓
[Lowerer] → Bytecode Instructions
    ↓
[Emitter] → LC-B Crate (deterministic bytecode)
    ↓
[Lifter] → HLX-A Source (reversible)
```

**Key feature:** Lift operation reconstructs source from bytecode with perfect fidelity

---

## Self-Hosting Status

✅ **HLX compiler is written in HLX** (25,801 lines across 7 modules)

- `lexer.hlx` - Tokenization (1,463 instructions)
- `parser.hlx` - AST construction (2,523 instructions)
- `lower.hlx` - Bytecode generation (1,848 instructions)
- `emit.hlx` - Binary emission
- `compiler.hlx` - Full pipeline (6,482 instructions)

**Current bootstrap:** Rust VM (RustD) executes HLX bytecode

**Future:** HLX compiler compiles to x86_64 native code, bootstraps itself

---

## Current Implementation Status

### ✅ Complete
- Parser for HLX-A (functions, let, return, if, loop, break, continue)
- Lexer for HLX-A
- Type system (i64, string, bool, arrays, pointers, custom types)
- Lowering to bytecode
- Module system with imports
- Const declarations and enum variants
- Struct literals
- Four axioms enforcement
- Self-hosting compiler
- Deterministic bytecode generation

### 🚧 In Progress
- Embedding contract syntax in HLX-A
- Handle manipulation (collapse/resolve)
- Mode switching (for swarm execution)
- HLX-R (Runic) representation

### ⏳ TODO
- Native code generation (x86_64)
- HLX-S (Swarm/Scale execution mode)
- Barrier execution with hash verification
- Complete bootstrap capsule → HLX-D spec conversion
- LSP enhancements for LLM-friendly features

---

## Design Principles

1. **LLM-First, Human-Friendly-Second**
   - Contracts and handles are natural for LLM generation
   - HLX-A syntax is still readable (not cryptic)
   - No hidden behavior

2. **Nothing Wasted**
   - Every decision is reversible
   - Every representation is recoverable
   - Bijection means no information loss

3. **Determinism is Non-Negotiable**
   - Fundamental property, not bolted-on
   - Enforced at language level
   - Verified by axioms

4. **Practical Over Pure**
   - Functions, loops, variables (procedural comfort)
   - BUT with contract values underneath
   - Both paradigms, unified

5. **Documentation is Code**
   - Design decisions are documented as code
   - Context persists between sessions
   - Breadcrumbs capture progress

---

## Relationship to RustD

**RustD** (Rust DSL) is a **reference implementation**:
- Proof that determinism works in practice
- Verefiable research artifact for jobs/publications
- LSP implementation inspiration
- Separate project: `/home/matt/rustd/`

**HLX** is the **production language**:
- Builds on lessons from RustD
- Independent evolution
- Target: AI/ML reproducibility, safety-critical systems
- May diverge from RustD as needed

---

## Success Metrics

✅ **Language is successful when:**
1. LLMs naturally generate correct HLX code
2. Humans can read and understand HLX code
3. Programs are deterministic by construction
4. Axioms are provably maintained
5. Bootstrap capsule is properly embedded
6. HLX compiler compiles itself without errors
7. Axiom Kernel boots successfully

---

## Document Version History

- **v1.0-Vision** (2026-01-19): Initial comprehensive spec after RustD/HLX separation
  - Clarified dual nature (programming language + data format)
  - Established axioms as core principle
  - Documented self-hosting status
  - Outlined embedding strategy for bootstrap capsule

---

**Maintained by:** Claude Code with user oversight | **Last Updated:** 2026-01-19
