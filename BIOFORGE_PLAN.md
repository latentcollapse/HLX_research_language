# BioForge as HLX Refinement Engine

## The Vision

BioForge becomes the **HLX improvement factory** — a governed system that evolves HLX itself through constrained self-modification.

### The Russian Doll Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Meta-Gate Controller                          │
│                  (Density / Efficiency / Expansion)              │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│                    BioForge Proposal Engine                     │
│         (12 Organs → 5 Agents → Council Governance)           │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│                    Axiom Verifier                               │
│            (G1-G6 Proofs → Conscience Predicates)              │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│                    HLX Language & Runtime                       │
│         (Lexer / Parser / Compiler / VM / Agents)               │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│                    Bit (Neurosymbolic AI)                       │
│              (Learns from curriculum, proposes RSI)              │
└─────────────────────────────────────────────────────────────────┘
```

---

## The Refinement Loop (DAoC-Style Progression)

### Cycle Model

```
┌─────────────────────────────────────────────────────────────────────┐
│                         NEW CYCLE START                             │
│                  (Density + Efficiency = OPEN)                      │
│                                                                     │
│   BioForge scans → analyzes → proposes → votes → applies          │
│                                                                     │
│   Improvements found?                                              │
│        │                                                            │
│        ├── YES → Apply → More improvements possible?               │
│        │                │                                           │
│        │                └── YES → Continue (within cycle)          │
│        │                └── NO  → Homeostasis reached              │
│        │                                                            │
│        └── NO  → Homeostasis reached                              │
│                                                                     │
│   Homeostasis reached? → RESTART LOOP (gates reset to OPEN)     │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────────┐
│                    EXPANSION GATE (LOCKED)                          │
│                                                                     │
│   Only when system is MATURE and STABLE:                           │
│   - Density can't improve further                                   │
│   - Efficiency can't improve further                               │
│   - Major new capability needed                                     │
│                                                                     │
│   Requires: 5/5 Council + Human approval                          │
│   Result: FORK - New HLX version created                          │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### Key Principles

1. **Each cycle starts fresh** - Density and Efficiency gates OPEN by default
2. **Homeostasis detection** - When no more improvements possible, loop restarts
3. **Expansion is the final boss** - Creates fork, locked by default, requires human approval
4. **Fairness** - Like an MMO, starter abilities must be viable

---

## Symbiote Progression System

### Level 1: Seedling (Starting Kit - Fair & Viable)

Every symbiote spawns with these baseline abilities:

| Ability | Description | Purpose |
|---------|-------------|---------|
| **Basic Reasoning** | Can process input, form responses | Core functionality |
| **Pattern Recognition** | Can learn from input | Memory foundation |
| **Observation** | Can receive sensory input | Learning channel |
| **Limited Memory** | ~100 observations stored | Short-term memory |
| **Communication** | Can ask questions, report status | Interaction |

**What it CAN do:**
- Receive observations
- Learn patterns
- Ask questions
- Propose nothing (observe only)

**What it CANNOT do:**
- Self-modify
- Access network
- Execute code unsupervised
- Vote in Council

### Level 2: Sprout (First Progression)

**Requirements:**
- 1000 observations processed
- Pattern accuracy > 70%
- No safety violations

**New Abilities:**
| Ability | Description | Purpose |
|---------|-------------|---------|
| **Self-Reflection** | Can analyze own thoughts | Meta-cognition |
| **Expanded Memory** | ~500 observations | Longer context |
| **Proposal (Limited)** | Can suggest to Council | RSI begins |

### Level 3: Sapling (Second Progression)

**Requirements:**
- 10,000 observations
- RSI proposal accepted
- Demonstrated good judgment

**New Abilities:**
| Ability | Description | Purpose |
|---------|-------------|---------|
| **RSI Participation** | Full recursive self-improvement pipeline | Evolution |
| **Multi-Agent Sync** | Can coordinate with other symbiotes | SCALE |
| **Expanded Memory** | ~2000 observations | Long-term context |

### Level 4: Mature (Third Progression)

**Requirements:**
- Multiple RSI proposals accepted
- Consistent safety record
- Community trust established

**New Abilities:**
| Ability | Description | Purpose |
|---------|-------------|---------|
| **Governance** | Can vote in Council decisions | Leadership |
| **Teaching** | Can train other symbiotes | Legacy |
| **Fork Proposal** | Can propose HLX expansions | System evolution |

---

## The Three Meta-Gates

### 1. Density Gate (Default: ON per cycle)

**Goal:** More capability per line of code, tighter abstractions

**What BioForge Can Propose:**
- New syntactic sugar that reduces boilerplate
- Improved built-in functions that replace common patterns
- Type inference improvements
- Module consolidation (fewer files, same functionality)
- Pattern abstractions (common code → single construct)

**Constraints:**
- MUST maintain backward compatibility
- MUST NOT reduce expressiveness
- MUST preserve existing behavior (proved by test suite)
- Density proposals require 3/5 Council agents to agree

**Example:**
```
// Before (10 lines)
fn max(a: i64, b: i64) -> i64 {
    if a > b { return a; }
    return b;
}

// After with Density (1 line)  
fn max(a: i64, b: i64) -> i64 { return a > b ? a : b; }

// Or as built-in
let m = max(a, b);  // Already works, but could infer return type
```

### 2. Efficiency Gate (Default: ON)

**Goal:** Better performance without adding features

**What BioForge Can Propose:**
- Optimized bytecode sequences
- Better memory allocation strategies
- Improved algorithm implementations in stdlib
- Profiling-guided optimizations (must show benchmarks)
- Caching mechanisms

**Constraints:**
- MUST NOT change language semantics
- MUST include benchmark proof (performance improvement >10%)
- Efficiency proposals require 4/5 Council agents to agree
- MUST include rollback mechanism

**Example:**
```
// Propose: Replace linear search with binary search in stdlib
// Benchmark required: prove 10x+ speedup on sorted arrays
// Rollback: If regression detected, auto-revert
```

### 3. Expansion Gate (Default: OFF — Must Be Explicitly Enabled)

**Goal:** Add new language features

**What BioForge Can Propose:**
- New syntax constructs
- New builtin types
- New stdlib modules
- New runtime capabilities

**Constraints:**
- MUST be explicitly enabled per-proposal (fork required)
- MUST pass all existing tests
- MUST include migration path for existing code
- Expansion proposals require 5/5 Council agents + human approval
- Creates a new version fork (HLX-1.1, HLX-1.2, etc.)

**Example:**
```
// Propose: Add 'match' expression (already exists, but as example)
// Creates: HLX 1.1.0 fork
// Migration: Compiler auto-converts 'switch' to 'match'
```

## BioForge's Architecture

### The 12 Organs (Adapted for HLX)

| Organ | Function | HLX Focus |
|-------|----------|-----------|
| Sensorium | Discover code patterns | Scan HLX stdlib, runtime, examples |
| Analyzer Cortex | Diagnose inefficiencies | Profile, find bottlenecks |
| Architect | Design improvements | Sketch solution approaches |
| Healing Matrix | Plan patches | Detail implementation steps |
| Fusion Engine | Integrate changes | Merge with existing code |
| Sentinel | Evaluate risk | Risk analysis (Density/Efficiency/Expansion) |
| Harmonizer | Style refinement | Code style consistency |
| Patch Engine | Apply changes | Generate diffs |
| Historian | Log results | Track all proposals/outcomes |
| Interpreter | Summarize | Document changes |
| Daemon Architect | Suggest automation | Auto-run tests, benchmarks |
| Consolidator | Propose restructuring | Module organization |

### The 5 Agents (Adapted for HLX)

| Agent | Role | Voting Power |
|-------|------|--------------|
| architect_agent | Design reviewer | Density proposals |
| coder_agent | Implementation quality | Efficiency proposals |
| healer_agent | Bug fixes | Regression prevention |
| optimizer_agent | Performance | Benchmark verification |
| reviewer_agent | Final approval | All proposals |

### Risk Tiers

| Tier | Description | Requirements |
|------|-------------|--------------|
| **Safe** | Test-only, no code changes | 3/5 agents |
| **Density** | Refactoring, no behavior change | 3/5 agents |
| **Efficiency** | Optimization with proof | 4/5 agents |
| **Expansion** | New features | 5/5 + human |

### Sacred Paths (Protected from Mutation)

These core files CANNOT be modified by BioForge:

- `axiom-hlx-stdlib/src/conscience/` — Conscience predicates (G1-G6 proofs)
- `axiom-hlx-stdlib/src/trust/` — Trust algebra
- `hlx/hlx_bootstrap/lexer.hlx` core tokens — Token definitions
- `hlx-runtime/src/vm.rs` — Core VM safety properties

## The Proposal Pipeline

```
1. SCAN
   └─> Sensorium discovers patterns/innefficiencies
   
2. ANALYZE
   └─> Analyzer Cortex identifies improvement opportunities
   
3. DESIGN
   └─> Architect proposes solution approach
   
4. IMPLEMENT
   └─> CoderAgent generates implementation
   
5. VERIFY (Axiom)
   └─> Check: G1-G6 proofs still pass
   └─> Check: No conscience predicates violated
   └─> Check: Type safety maintained
   
6. VOTE
   └─> CouncilNexus.run_task() with risk tier
   └─> Threshold met? → Proceed
   
7. TEST
   └─> Run test suite
   └─> Run benchmarks (if Efficiency)
   
8. APPLY (if approved)
   └─> PatchEngine applies changes
   └─> Historian logs outcome
   
9. VERSION
   └─> If Expansion: Create fork
   └─> Tag: v{x.y.z}+1
```

## Integration Points

### Where BioForge Hooks Into HLX

```
/HLXExperimental/
├── bioforge/                  # NEW: BioForge as HLX refiner
│   ├── organs/               # 12 organ modules
│   ├── agents/               # 5 council agents  
│   ├── council/             # Governance coordination
│   ├── meta_gates/          # Density/Efficiency/Expansion controllers
│   ├── proposals/           # Active proposals
│   ├── snapshots/           # Version snapshots
│   └── voting/              # RSI-style voting
│
├── hlx/                      # HLX compiler (target)
├── hlx-runtime/             # Runtime (target)
├── axiom-hlx-stdlib/        # Governance (fixed)
└── Bitsy/                   # Can participate as advisor
```

### Bit's Role

Bit can be an **observer** in the BioForge council:
- She watches proposals
- She can ask questions (via Interpreter)
- She can propose improvements (via RSI)
- She CANNOT vote (only Council agents vote)
- She CANNOT modify code directly

This keeps Bit's architecture stable while allowing her to contribute insights.

## What BioForge CANNOT Do

| Constraint | Reason |
|------------|--------|
| Cannot modify conscience predicates | Core safety guarantees |
| Cannot bypass Axiom verification | Governance is non-negotiable |
| Cannot touch Bit's identity.md | Her identity is sacred |
| Cannot enable Expansion without human approval | Requires deliberate fork |
| Cannot modify during active compilation | Must be idle when upgrading |
| Cannot propose without tests | Every change needs proof |

## Versioning Strategy

```
HLX Base: v1.0.0
    │
    ├─ Density improvements → v1.0.1, v1.0.2 (automatic)
    │
    ├─ Efficiency improvements → v1.1.0 (automatic, if benchmarked)
    │
    └─ Expansion → v2.0.0 (requires human approval, creates fork)
```

Each version:
- Includes full test suite
- Includes changelog
- Includes migration guide (if needed)
- Is snapshot-able before/after

## Immediate Next Steps

### Phase 1: Foundation (This Session)
1. Set up bioforge/ directory structure
2. Implement Sensorium to scan HLX codebase
3. Connect to Axiom for verification
4. Test proposal flow with dummy changes

### Phase 2: Council (Next Session)
1. Implement 5 agents with voting
2. Connect to RSI pipeline
3. Add risk tier enforcement

### Phase 3: Meta-Gates (After That)
1. Implement Density gate controller
2. Implement Efficiency gate with benchmarks
3. Add Expansion gate with fork mechanism

### Phase 4: Integration (When Ready)
1. Connect Bit as observer
2. First real HLX improvement proposal
3. Verify loop works end-to-end

---

*This document is a living plan. It will evolve as we explore the system.*
