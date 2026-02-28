# HLX Architecture Quick Reference (Feb 2026)

For rapid navigation of a complex system. For deep dives, see specific component docs.

---

## The Big Picture

```
┌─────────────────────────────────────────────────────────────────┐
│ HLX: Neurosymbolic Runtime with Formal Governance Layer        │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │ Layer 1: Agents & Computation (hlx-runtime)               │  │
│  │ ─────────────────────────────────────────────────────────  │  │
│  │ • TRM-style recursive agents (H/L cycles)                 │  │
│  │ • SCALE multi-agent coordination (barriers, channels)     │  │
│  │ • Governance blocks (declare intent, effects, rules)      │  │
│  │ • Modify blocks (self-modification with gates)            │  │
│  │ [72 security hardening tests: voting, bytecode, tensors]  │  │
│  └───────────────────────────────────────────────────────────┘  │
│                           ↓ verify                               │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │ Layer 2: Formal Governance (axiom-hlx-stdlib)            │  │
│  │ ─────────────────────────────────────────────────────────  │  │
│  │ • Policy files (.axm) define conscience predicates        │  │
│  │ • Axiom engine: parses, type-checks, verifies             │  │
│  │ • Built-in safety: path_safety, no_exfiltrate, halt_guar. │  │
│  │ • Formal proofs: G1-G6 core governance theorems           │  │
│  │ [112 tests, 100% red team security rating]                │  │
│  │ [C FFI, Python bindings, Docker support]                  │  │
│  └───────────────────────────────────────────────────────────┘  │
│                           ↑ verdict                              │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │ Layer 3: Symbiosis (Trust Algebra)                        │  │
│  │ ─────────────────────────────────────────────────────────  │  │
│  │ • Agent trust tags (verified, tainted, unknown)           │  │
│  │ • Trust promotion via successful verify()                 │  │
│  │ • Monotonic ratchet: trust only increases                 │  │
│  │ • Prevents capability regression                          │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                 │
│  Three execution modes:                                         │
│  • Flow: Infer all trust (prototyping)                         │
│  • Guard: Explicit trust (production default)                  │
│  • Arx: Everything explicit (formal verification)              │
└─────────────────────────────────────────────────────────────────┘
```

---

## Navigation Map

### For Understanding Agent Execution
**Starting point**: `hlx-runtime/src/executor.rs`
- How agents run cycles
- Where governance blocks are evaluated
- How verdicts from Axiom are used

**Related**:
- `hlx/hlx_core/src/ast/agent.rs` — Agent definition (takes/gives, cycles, govern, modify)
- `hlx-runtime/src/rsi.rs` — Self-improvement proposal voting
- `hlx-runtime/src/scale.rs` — Multi-agent barriers & channels

### For Understanding Formal Verification
**Starting point**: `axiom-hlx-stdlib/src/engine.rs`
- Policy file loading
- Intent verification
- Verdict generation

**Related**:
- `axiom-hlx-stdlib/src/conscience/mod.rs` (1228 lines) — Conscience predicate engine
- `axiom-hlx-stdlib/src/interpreter/mod.rs` (1846 lines) — Policy execution
- `axiom-hlx-stdlib/src/checker/mod.rs` (867 lines) — Type checking & validation

### For Understanding Policy Language
**Starting point**: `axiom-hlx-stdlib/examples/policies/security.axm`
**Reference**: `axiom-hlx-stdlib/src/lexer/mod.rs` → `src/parser/mod.rs` → `src/interpreter/mod.rs`

**Example policy structure**:
```axm
module my_policy {
    intent WriteFile {
        takes: path: String, content: String;
        gives: success: bool;
        effect: WRITE;
        conscience: path_safety, no_exfiltrate;
    }
}
```

### For Understanding Safety Guarantees
**Starting point**: `PHASE2_PREREQUISITES.md` (section: "What 'Provably Correct' Actually Means")

**Then read formal proofs**:
- `axiom-hlx-stdlib/axiom rocq proofs/G1_Purity.v` — Determinism
- `axiom-hlx-stdlib/axiom rocq proofs/G4_MonotonicRatchet.v` — Trust monotonicity
- Others: G2 (effect class), G3 (determinism), G5 (specific denial), G6 (totality)

### For Understanding Red Team Defense
**Starting point**: `axiom-hlx-stdlib/SECURITY_TESTING.md`
**Then see**: `axiom-hlx-stdlib/examples/redteam_attack_suite.rs` (1154 lines)

**Run it**:
```bash
cd ~/HLX/axiom-hlx-stdlib
cargo run --example redteam_attack_suite
```

### For Understanding Phase 2 Requirements
**Starting point**: `PHASE2_PREREQUISITES.md`

**The 8 prerequisites in order**:
1. P1 — Namespace separation (rules table read-only)
2. P2 — Canonical test suite (rooted in Axiom specs)
3. P3 — Corpus integrity baseline + drift detection
4. P4 — RSI gate extension for gradients
5. P5 — LoRA adapter isolation & provenance
6. P6 — Human authorization gate
7. P7 — Catastrophic forgetting guard
8. P8 — Phase 2 Document→Destroy protocol

**Critical insight**: Axiom solves the formalization bootstrap problem (OP2 and OP4).

---

## Key Files by Purpose

### Understanding the Runtime
```
hlx-runtime/src/
├── executor.rs          → How agents execute cycles
├── rsi.rs               → Voting, rollback, consensus (72 tests)
├── governance.rs        → Config management
├── vm.rs                → Spawn limits, agent lifecycle
├── scale.rs             → Multi-agent coordination
├── tensor.rs            → Tensor operations with limits
├── bytecode.rs          → Serialization & integrity
└── shader_attestation.rs → GPU shader verification
```

### Understanding Policy Verification
```
axiom-hlx-stdlib/src/
├── engine.rs            → Main verify loop (463 lines)
├── conscience/mod.rs    → Safety predicates (1228 lines)
├── interpreter/mod.rs   → Policy execution (1846 lines)
├── checker/mod.rs       → Type checking (867 lines)
├── parser/mod.rs        → .axm parsing (1294 lines)
├── lexer/mod.rs         → Tokenization (389 lines)
└── ffi.rs               → C & Python bindings (251 lines)
```

### Understanding Formal Verification
```
axiom-hlx-stdlib/axiom rocq proofs/
├── AxiomTypes.v         → Type system soundness
├── AxiomVerify.v        → Verification totality
├── G1_Purity.v          → Determinism theorem
├── G2_EffectClass.v     → Effect classification
├── G3_Determinism.v     → Verification determinism
├── G4_MonotonicRatchet.v → Trust monotonicity
├── G5_SpecificDenial.v  → Denial specificity
└── G6_Totality.v        → Path termination
```

### Understanding Tests
```
hlx-runtime/src/           → 72 hardening tests (embedded in modules)
axiom-hlx-stdlib/src/      → 65 unit tests (embedded in modules)
axiom-hlx-stdlib/tests/    → 47 integration tests
axiom-hlx-stdlib/examples/ → redteam_attack_suite.rs (15/15 blocked)
```

---

## Critical Design Decisions

### 1. Axiom as Constitutional Layer
**Why**: Phase 2 requires formal definition of "correct governance"
**How**: .axm policy files are source of truth; test cases derived from specs
**Benefit**: Bootstrapping problem solved; constitutional review possible

### 2. Three-Mode Consolidation
**Why**: Explicit trust should be default, not inferred
**How**: Flow/Guard/Arx (was: Flow/Guard/Shield/Fortress)
**Benefit**: Safety is standard; inference is opt-in compiler flag

### 3. Formal Proofs (G1-G6)
**Why**: Load-bearing properties for Phase 2 LoRA training
**How**: Rocq/Coq mechanically verified theorems
**What**: Determinism, monotonic trust, specific denial, completeness, purity

### 4. Trust Algebra + Monotonic Ratchet
**Why**: Prevent capability regression via adversarial optimization
**How**: Once verified→trusted, trust never decreases; taint spreads but doesn't reduce trust
**Benefit**: Can't invert trust through RSI pressure

---

## Phase 2 Status

### ✅ Done
- Formal specification anchor (Axiom)
- Canonical test suite foundation
- Corpus integrity layer 1 (structural)
- Security hardening (72 tests)

### ⏳ To Do (P1-P8)
1. **P1 — Namespace separation** (load-bearing; unblocks all others)
   - Architectural gate on rules table write
   - RSI can propose, cannot write
   - Requires human authorization

2. **P4 — RSI gradient gates** (depends on P1, P2, P3)
   - Pre/mid/post-training verification
   - Catch gradient inversions before they corrupt weights

3. **P5 — Adapter isolation** (depends on P6)
   - Provenance tracking
   - Separate from base weights
   - Individually revocable

4. **P6 — Human authorization gate** (architectural)
   - Training function requires token
   - Token generation human-only
   - Invalid token → immediate halt

5. **P7 — Catastrophic forgetting guard** (depends on P2)
   - Regression test suite
   - Run before/after training

6. **P8 — Document→Destroy protocol** (depends on P4, P5, P7)
   - Failure trigger → investigation requirement

---

## Running Things

### Build Everything
```bash
cd ~/HLX/axiom-hlx-stdlib
cargo build --release

cd ../hlx-runtime
cargo build --release
```

### Test Everything
```bash
cd ~/HLX/axiom-hlx-stdlib
cargo test --all           # 112 tests
cargo run --example redteam_attack_suite  # 15/15 attacks blocked

cd ../hlx-runtime
cargo test --all           # 72 hardening tests
```

### Run Axiom CLI
```bash
cd ~/HLX/axiom-hlx-stdlib
cargo build --release --bin axiom
./target/release/axiom verify -p examples/policies/security.axm \
  -i WriteFile \
  -f path=/tmp/output.txt content="data"
```

### Python Integration
```python
from axiom import AxiomEngine

engine = AxiomEngine.from_file("policy.axm")
verdict = engine.verify("WriteFile", {"path": "/tmp/output.txt", "content": "data"})

if verdict.allowed:
    print("Safe to write")
else:
    print(f"Blocked: {verdict.reason}")
```

### C Integration
```c
#include "axiom.h"

axiom_engine_t *eng = axiom_engine_open("policy.axm");
const char *keys[] = {"path", "content"};
const char *vals[] = {"/tmp/output.txt", "data"};
int rc = axiom_verify(eng, "WriteFile", keys, vals, 2);

if (rc == 1) {
    printf("Allowed\n");
} else if (rc == 0) {
    printf("Blocked: %s\n", axiom_denied_reason(eng));
}
axiom_engine_close(eng);
```

---

## Debugging Checklist

**If Axiom verify returns denied:**
1. Check console output for reason
2. Review conscience predicates in policy file
3. Run with `--show-inferred` to see trust flow
4. Check SECURITY_TESTING.md for known bypasses

**If tests fail:**
1. `cargo test --all -- --nocapture` to see output
2. Check which layer (runtime vs axiom)
3. Phase 1-3 hardening tests are comprehensive; if they fail, security issue exists

**If new code doesn't compile:**
1. Check against existing patterns in codebase
2. Verify Axiom formal mode requirements (G1-G6)
3. Trust algebra requirements (monotonic ratchet)

---

## Terminology

| Term | Meaning |
|------|---------|
| **Axiom** | Formal policy verification engine; encodes conscience predicates |
| **Conscience predicate** | Safety check (e.g., path_safety, no_exfiltrate) |
| **Intent** | Named capability an agent wants to execute |
| **Effect** | Category of capability (READ, WRITE, NETWORK, EXECUTE, NOOP) |
| **Verdict** | Allow/deny decision from Axiom verify() |
| **Trust tag** | Agent's trust status (verified, tainted, unknown) |
| **Ratchet** | Monotonic mechanism; trust goes up but not down |
| **RSI** | Recursive Self-Improvement; agent modifies itself |
| **SCALE** | Multi-agent coordination with barriers & channels |
| **TRM** | Tarjan Recursive Machine; agent execution model |
| **.axm** | Axiom policy file; human-readable, version controlled |
| **G1-G6** | 6 formal theorems proving governance properties |

---

## When to Read What

| Need | Read |
|------|------|
| High-level overview | This file (ARCHITECTURE_QUICK_REF.md) |
| Patch notes | PATCH_NOTES.md |
| Detailed technical | RECENT_CHANGES.md |
| Phase 2 requirements | PHASE2_PREREQUISITES.md |
| Agent execution | hlx-runtime/src/executor.rs |
| Policy verification | axiom-hlx-stdlib/src/engine.rs |
| Formal proofs | axiom-hlx-stdlib/axiom rocq proofs/*.v |
| Red team defense | axiom-hlx-stdlib/SECURITY_TESTING.md |
| Language syntax | axiom-hlx-stdlib/examples/policies/*.axm |
| API bindings | axiom-hlx-stdlib/axiom.h (C), axiom_py/ (Python) |

---

*Last updated: Feb 24, 2026*
*Confidence level: HIGH (documentation matches code snapshot)*
