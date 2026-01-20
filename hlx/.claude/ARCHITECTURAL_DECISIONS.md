# HLX Architectural Decisions & Rationale

**Status:** Living Document | **Date:** 2026-01-19

This document captures the "why" behind key HLX design choices.

---

## Decision 1: RustD as Separate Project

**Decision:** Move Rust implementation to `/home/matt/rustd/` as standalone project

**Rationale:**
- **Problem it solved:** Claude was reflexively fixing Rust code instead of building HLX
- **The insight:** Rust code is a *reference implementation*, not the actual product
- **Prevents:** Scope creep into Rust maintenance instead of HLX development
- **Enables:** Clear mental model (RustD = proof-of-concept, HLX = production)
- **Additional value:** RustD becomes a portfolio piece for Rust/Determinism jobs

**Decision date:** 2026-01-19 | **Status:** ✅ Implemented

---

## Decision 2: Language, Not Just Format

**Decision:** HLX is a *programming language* (procedural + contract-based), not just a data format

**Rationale:**
- **Problem it solved:** Original bootstrap capsule was LLM-teaching format; needed to evolve into executable language
- **The insight:** Can embed contract spec as data types within procedural language
- **Serves both:** Humans get familiar procedural syntax; LLMs get natural contract semantics
- **Prevents:** Losing the value of the bootstrap capsule (which becomes HLX-D)
- **Enables:** Single unified system instead of separate language + format

**Example evolution:**
```
Bootstrap Capsule (v1)
  → HLX-D (embedded data format in v1.0)
    → HLX Language (procedural wrapper + contract values)
```

**Decision date:** 2026-01-19 | **Status:** ⏳ In Progress

---

## Decision 3: Dual Representation (A/R)

**Decision:** Maintain HLX-A (ASCII) and HLX-R (Runic) with perfect bijection

**Rationale:**
- **Problem it solved:** Need both human-readable AND LLM-efficient forms
- **The insight:** Perfect bijection means zero information loss in translation
- **Serves both:** Humans read/write HLX-A; LLMs communicate via HLX-R
- **Prevents:** Divergence between representations
- **Enables:** LLM-to-LLM code transmission at minimum bandwidth

**Current status:** HLX-A implemented, HLX-R in progress

**Decision date:** 2026-01-19 | **Status:** 🚧 In Progress

---

## Decision 4: Procedural Base Language

**Decision:** HLX uses procedural syntax (fn, let, return, if, loop) not purely functional

**Rationale:**
- **Problem it solved:** Pure functional would be elegant but hard for LLMs to generate naturally
- **The insight:** Procedural is familiar to LLMs (trained on Python, JavaScript, etc.)
- **Serves both:** Humans comfortable with functions; LLMs generate procedural code easily
- **Prevents:** Creating a language so different that LLMs struggle
- **Enables:** Gradual adoption (looks like familiar languages)

**Under the hood:** Values are still immutable contract-based; procedural is syntax, not semantics

**Decision date:** 2026-01-19 | **Status:** ✅ Implemented

---

## Decision 5: Embedded Contract Syntax

**Decision:** Contract values `{42:{@0:123}}` are first-class HLX syntax, not something external

**Rationale:**
- **Problem it solved:** How to keep bootstrap capsule alive in a procedural language?
- **The insight:** Embed it as data literal syntax, just like JSON in JavaScript
- **Serves LLMs:** Contract structure is explicit and natural
- **Prevents:** Losing the LLM-friendly structure of the capsule
- **Enables:** Programs can manipulate contracts directly

**Example:**
```hlx
let data = {42:{@0:123, @1:"test"}};  // Contract literal in HLX code
let result = collapse(data);           // Explicit value → handle transition
```

**Decision date:** 2026-01-19 | **Status:** ⏳ In Progress (parser support exists, needs lowering)

---

## Decision 6: Four Axioms as Language Guarantees

**Decision:** Four axioms are checked by the compiler, not just documented

**Rationale:**
- **Problem it solved:** How to ensure determinism isn't just aspirational?
- **The insight:** Make axioms **enforced** at compile time, not runtime
- **Serves safety:** Programs that don't satisfy axioms fail to compile
- **Prevents:** Silent violations of determinism guarantees
- **Enables:** Mathematical proofs about program behavior

**How enforced:**
- A1 (Determinism): No randomness, bounded loops, no time-dependent ops
- A2 (Reversibility): Lift operation must recover source perfectly
- A3 (Bijection): Type system prevents non-bijective mappings
- A4 (Universal Value): No implicit conversions, exhaustive patterns

**Decision date:** 2026-01-19 | **Status:** 🚧 Partially Implemented

---

## Decision 7: Self-Hosting from Start

**Decision:** Build HLX compiler in HLX, compile with RustD bootstrap

**Rationale:**
- **Problem it solved:** Chicken-and-egg: need a compiler to compile HLX
- **The insight:** Write compiler in HLX, bootstrap with Rust implementation
- **Serves dogfooding:** HLX compiler is itself written in HLX (proof it works)
- **Prevents:** Lock-in to Rust ecosystem
- **Enables:** HLX can evolve independently

**Status:** ✅ HLX compiler (25,801 LOC in HLX) self-compiles

**Decision date:** Earlier sessions | **Status:** ✅ Verified 2026-01-19

---

## Decision 8: Separate Bootstrap Capsule → HLX-D

**Decision:** Evolve bootstrap capsule into formal "HLX-D" (data layer) specification

**Rationale:**
- **Problem it solved:** Bootstrap capsule is a teaching artifact; needs formal status
- **The insight:** Make it the *official data specification* that HLX implements
- **Serves clarity:** Separates "how to teach LLMs" from "programming language spec"
- **Prevents:** Confusion about what the capsule actually is
- **Enables:** Versioning and formal specification

**Naming options:**
- HLX-D (Data)
- LCS (Latent Contract Spec)
- [TBD by user]

**Decision date:** 2026-01-19 | **Status:** ⏳ Pending (needs naming)

---

## Decision 9: No Implicit Type Coercion

**Decision:** All type conversions must be explicit (no silent i64 → string, etc.)

**Rationale:**
- **Problem it solved:** Implicit coercions are a common bug source in other languages
- **The insight:** Make LLMs choose conversions explicitly (fewer bugs)
- **Serves axiom A4:** Universal Value (no hidden state)
- **Prevents:** Surprising type conversion bugs
- **Enables:** Proof that all behavior is as written

**Example (not allowed):**
```hlx
let x = 123;
let s = x;  // ❌ ERROR: implicit i64 → ??? conversion
```

**Example (correct):**
```hlx
let x = 123;
let s = to_string(x);  // ✅ Explicit conversion
```

**Decision date:** 2026-01-19 | **Status:** ⏳ In Progress (needs enforcement)

---

## Decision 10: Pass-by-Value Arrays

**Decision:** Arrays are pass-by-value (modifications don't affect original)

**Rationale:**
- **Problem it solved:** How to maintain determinism without data races?
- **The insight:** Pass-by-value prevents hidden shared state
- **Serves axiom A1:** Determinism (no surprises from aliasing)
- **Serves axiom A4:** Universal Value (state is explicit)
- **Serves HLX-S:** Swarm parallelization requires no shared mutable state
- **Prevents:** Data races in distributed execution

**Consequence:** Insertion sort works; quicksort doesn't (without redesign)

**Decision date:** Earlier sessions | **Status:** ✅ Verified

---

## Decision 11: Bounded Loops Only

**Decision:** `loop(condition, max_iterations)` not `while` or `for`

**Rationale:**
- **Problem it solved:** How to guarantee termination?
- **The insight:** Explicit bound forces thinking about iteration limits
- **Serves axiom A1:** Determinism (no infinite loops)
- **Prevents:** Accidental infinite recursion
- **Enables:** Verification (compiler can reason about termination)

**Design choice:** Entry field tracks loop re-entry for correct `continue` behavior

**Decision date:** Earlier sessions | **Status:** ✅ Verified (continue bug fixed)

---

## Decision 12: Pointer Types for Kernel Development

**Decision:** Support `*const T` and `*mut T` syntax for bare-metal code

**Rationale:**
- **Problem it solved:** Axiom Kernel needs hardware register access (0xB8000 for VGA)
- **The insight:** Pointers aren't unsafe if determinism is enforced
- **Serves axiom A1:** Deterministic pointer arithmetic
- **Prevents:** Segfaults without losing expressiveness
- **Enables:** Writing OS kernels in HLX

**Status:** ✅ Parsing works; lowering to bytecode in progress

**Decision date:** 2026-01-19 | **Status:** ⏳ In Progress

---

## Decision 13: LSP Inspired by RustD

**Decision:** Use RustD's LSP implementation as inspiration, not copy

**Rationale:**
- **Problem it solved:** HLX LSP should be LLM-friendly, not just IDE-friendly
- **The insight:** RustD proved what works; build on those ideas
- **Serves both:** Humans get IDE support; LLMs get semantics
- **Prevents:** Starting from scratch
- **Enables:** Faster development

**Features to adapt:**
- Type inference (but simpler for HLX)
- Error recovery (preserve partial info)
- Hover documentation (natural for LLMs)
- Suggestions (weighted for LLM naturalness)

**Decision date:** 2026-01-19 | **Status:** 🚧 Design phase

---

## Decision 14: Session Context Persistence

**Decision:** Document architectural decisions, session breadcrumbs, design spec in `.claude/`

**Rationale:**
- **Problem it solved:** Claude losing context between sessions, making repeated mistakes
- **The insight:** Documented context prevents ramp-up time and repeated errors
- **Serves continuity:** Every session starts with full architectural understanding
- **Prevents:** Drift in vision or repeated "why did we do this?" questions
- **Enables:** Compound progress (each session builds on previous, no backtracking)

**System:**
- `HLX_DESIGN_SPEC.md` - What HLX is
- `ARCHITECTURAL_DECISIONS.md` - Why each decision (this file)
- `SESSION_BREADCRUMBS/session_YYYY-MM-DD.md` - Daily progress

**Decision date:** 2026-01-19 | **Status:** ✅ Implemented

---

## Decisions Still Open

### Should HLX have OOP features?
- **Status:** Undecided
- **Trade-off:** Classes add complexity; contracts may be sufficient
- **TODO:** Compare contract-based polymorphism vs. traditional OOP

### What should "HLX-D" be named?
- **Options:** HLX-D, LCS, Contract-Spec, DataLayer
- **Status:** Needs user decision
- **TODO:** Naming decision with user

### Should HLX compile to x86_64 or stay on RustD VM?
- **Status:** Undecided
- **Trade-off:** Native = more control, determinism harder; VM = easier proof
- **TODO:** Evaluate for Axiom Kernel requirements

### How should HLX-S (Swarm) work?
- **Status:** Design phase
- **Trade-off:** Distributed execution vs. complexity
- **TODO:** Reference original capsule for mode-switching

---

## Document Maintenance

**Owner:** User + Claude Code
**Review frequency:** Start of each session
**Update protocol:** Add new decision when made; rationale from discussion

**When Claude Code makes a decision:**
1. Capture in this document
2. Explain the trade-off
3. Link to relevant code
4. Mark status (Design/In Progress/Complete)

---

**Last Updated:** 2026-01-19 | **Next Review:** Start of next session
