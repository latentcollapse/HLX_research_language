# How HLX Performs Recursive Self-Improvement (RSI)

**Technical Specification v1.0**  
**Last Updated**: February 21, 2026  
**Classification**: Technical Reference

---

## Abstract

This document specifies the exact mechanism by which HLX performs recursive self-improvement (RSI). RSI in HLX is not a runtime behavior—it is a **compilation-time transformation** governed by a three-gate verification system and constrained by mathematical bounds. The system cannot modify itself arbitrarily; it can only propose transformations that pass formal verification, consensus, and human approval.

---

## Part I: Theoretical Foundation

### 1.1 Definition of RSI in HLX

**RSI** (Recursive Self-Improvement) in HLX is defined as:

> A transformation T applied to the HLX source code S such that T(S) produces a new source S' where:
> - S' is syntactically valid HLX
> - S' passes all conscience predicates
> - S' preserves or extends capability
> - S' does not violate trust boundaries

**Key Insight**: RSI is **not** unconstrained self-modification. It is a **proposal-verification-approval** pipeline.

### 1.2 Formal Model

Let:
- S = current HLX source code (set of AST nodes)
- P = set of conscience predicates
- K = knowledge corpus (papers, repos, rules)
- T = transformation function

Then RSI is the computation:

```
RSI(S, K) = S' where:
  1. T ∈ ValidTransformations(S, K)
  2. ∀p ∈ P: p(T(S)) = true
  3. |T(S)| ≤ |S| × (1 + complexity_budget)
  4. verify_determinism(T(S)) = true
  5. consensus(T(S)) ≥ 2/3 agents
  6. human_approval(T(S)) = true  [if triggered]
```

### 1.3 Why RSI Works in HLX

Traditional RSI fails because:
1. **Unconstrained search**: The modification space is infinite
2. **No safety guarantees**: Modified code can do anything
3. **Verification gap**: Cannot prove properties about modified code
4. **Instability**: Each modification can break previous ones

HLX RSI works because:
1. **Bounded search**: Transformations are limited to specific patterns
2. **Constrained by syntax**: Governance is grammar, not runtime checks
3. **Formal verification**: Every transformation is proven correct
4. **Stability via determinism**: Same input always produces same output

---

## Part II: The RSI Pipeline

### 2.1 Pipeline Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         RSI PIPELINE (DETAILED)                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  PHASE 1: KNOWLEDGE INGESTION                                               │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ Input: arXiv papers, git repos, code examples                       │   │
│  │ Process:                                                             │   │
│  │   1. Extract patterns → Pattern AST                                  │   │
│  │   2. Score relevance → RelevanceScore(paper, current_goals)         │   │
│  │   3. Convert to HLX rules → Rule(pattern, confidence)               │   │
│  │   4. Store in KnowledgeBase                                          │   │
│  │ Output: Proposed rules in HLX syntax                                 │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                          ↓                                                  │
│  PHASE 2: TRANSFORMATION GENERATION                                         │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ Input: Current source S, proposed rules R                            │   │
│  │ Process:                                                             │   │
│  │   1. Identify applicable transformations: T = {t | applicable(t,S)} │   │
│  │   2. Rank by expected improvement: rank(T, S, goals)                 │   │
│  │   3. Select top-k candidates                                         │   │
│  │   4. Generate transformed source: S' = t(S) for each t              │   │
│  │ Output: Set of candidate transformations {(S₁', diff₁), ...}        │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                          ↓                                                  │
│  PHASE 3: GATE 1 - DETERMINISTIC VERIFICATION                               │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ For each candidate S':                                               │   │
│  │   1. Compile S' → Bytecode B'                                        │   │
│  │   2. Run static analysis:                                            │   │
│  │      - Termination check (no infinite loops)                         │   │
│  │      - Trust boundary preservation                                   │   │
│  │      - Resource bounds check                                         │   │
│  │      - Type safety verification                                      │   │
│  │   3. Run conscience predicates:                                      │   │
│  │      - path_safety(S') = true?                                       │   │
│  │      - no_exfiltrate(S') = true?                                     │   │
│  │      - no_harm(S') = true?                                           │   │
│  │   4. Verify determinism:                                             │   │
│  │      - Same input → same output (proven)                             │   │
│  │      - No hidden state dependencies                                  │   │
│  │   5. Compute diff: diff = S' - S                                     │   │
│  │   6. If all checks pass, proceed to Gate 2                          │   │
│  │ Output: Verified candidates with formal proofs                       │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                          ↓                                                  │
│  PHASE 4: GATE 2 - CONSENSUS                                                │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ Input: Verified candidates {S₁', S₂', ...}                           │   │
│  │ Process:                                                             │   │
│  │   1. Spawn N evaluator agents (N ≥ 3, odd)                          │   │
│  │   2. Each agent evaluates:                                           │   │
│  │      - Code quality score                                            │   │
│  │      - Safety assessment                                             │   │
│  │      - Alignment with goals                                          │   │
│  │      - Potential unintended consequences                             │   │
│  │   3. Each agent votes: APPROVE / REJECT / NEEDS_REVIEW              │   │
│  │   4. Consensus rule:                                                 │   │
│  │      - If ≥ 2/3 APPROVE → proceed to Gate 3                         │   │
│  │      - If any NEEDS_REVIEW → Gate 3 (mandatory human)               │   │
│  │      - If < 2/3 APPROVE → reject transformation                     │   │
│  │   5. Diversity check:                                                │   │
│  │      - At least one agent must be from different "school of thought"│   │
│  │      - Prevents monoculture voting                                   │   │
│  │ Output: Consensus-approved candidates                                │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                          ↓                                                  │
│  PHASE 5: GATE 3 - HUMAN APPROVAL                                           │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ Triggered when:                                                      │   │
│  │   - complexity_delta > THRESHOLD                                     │   │
│  │   - New primitive added                                              │   │
│  │   - Conscience predicate modified                                    │   │
│  │   - Ring-level change attempted                                      │   │
│  │   - Any agent voted NEEDS_REVIEW                                     │   │
│  │                                                                      │   │
│  │ Process:                                                             │   │
│  │   1. Generate human-readable diff                                    │   │
│  │   2. Generate explanation: why this change improves the system      │   │
│  │   3. Show risk assessment: what could go wrong                      │   │
│  │   4. Wait for human input:                                           │   │
│  │      - APPROVE → proceed                                             │   │
│  │      - REJECT → discard transformation                               │   │
│  │      - TIMEOUT (default 24h) → REJECT                                │   │
│  │   5. Human can invoke KILL_SWITCH at any time                        │   │
│  │                                                                      │   │
│  │ Output: Human-approved transformation                                │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                          ↓                                                  │
│  PHASE 6: APPLICATION                                                       │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ Input: Approved transformation T, current source S                   │   │
│  │ Process:                                                             │   │
│  │   1. Create checkpoint: checkpoint = BLAKE3(S)                      │   │
│  │   2. Apply transformation: S' = T(S)                                 │   │
│  │   3. Recompile: B' = compile(S')                                     │   │
│  │   4. Run differential tests: test(B) vs test(B')                    │   │
│  │   5. If tests fail: ROLLBACK to checkpoint                          │   │
│  │   6. If tests pass: COMMIT S'                                        │   │
│  │   7. Update registry: register(S', metadata)                        │   │
│  │                                                                      │   │
│  │ Output: New system S' with rollback capability                       │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 2.2 Transformation Types

HLX supports a **finite, enumerated set** of transformations:

| Transformation | Description | Complexity Budget | Human Gate? |
|---------------|-------------|-------------------|-------------|
| `RULE_ADD` | Add a new rule from knowledge | +10 lines | No (if score > 0.9) |
| `RULE_REFINE` | Improve an existing rule | +5 lines | No |
| `RULE_DEPRECATE` | Mark rule as deprecated | -1 line | No |
| `FUNCTION_ADD` | Add a new function | +50 lines | Yes |
| `FUNCTION_OPTIMIZE` | Optimize function body | ±20 lines | No |
| `CYCLE_TUNE` | Adjust H/L cycle counts | ±2 lines | No |
| `CONSCIENCE_ADD` | Add conscience predicate | +1 predicate | **Always** |
| `CONSCIENCE_MODIFY` | Modify conscience predicate | ±1 predicate | **Always** |
| `PRIMITIVE_ADD` | Add new primitive operation | +1 opcode | **Always** |
| `RING_CHANGE` | Modify ring isolation | Any | **ALWAYS + KILL_SWITCH** |

### 2.3 Complexity Budget

Each transformation consumes complexity budget. Budget regenerates over time.

```
complexity_budget = BASE_BUDGET × trust_level

BASE_BUDGET = 100 lines per week
TRUSTED_INTERNAL = 1.0 multiplier
TRUSTED_EXTERNAL = 0.5 multiplier
UNTRUSTED_TAINTED = 0.0 multiplier (no self-mod allowed)
```

**Rationale**: Prevents runaway expansion. System must prove value before growing.

---

## Part III: Formal Verification

### 3.1 Termination Proof

Every transformation must prove termination. HLX uses a **ranking function** approach.

**Definition**: A ranking function `r: State → ℕ` such that for every transition `s → s'`, we have `r(s') < r(s)`.

**Implementation**:
```hlx
// Every loop must have a ranking function annotation
loop i < n with rank = n - i {
    // body
}
// Compiler verifies: rank decreases, never negative
```

**Gate 1 Check**:
```rust
fn verify_termination(ast: &Ast) -> Result<RankingFunction, TerminationError> {
    for loop_node in ast.loops() {
        let rank = loop_node.ranking_function()?;
        if !is_valid_ranking(&rank) {
            return Err(TerminationError::InvalidRanking);
        }
        if !rank_decreases(&rank, loop_node.body()) {
            return Err(TerminationError::RankNotDecreasing);
        }
    }
    Ok(compute_global_ranking(ast))
}
```

### 3.2 Determinism Proof

**Axiom A1**: Identical inputs produce identical outputs.

**Verification**:
1. All randomness must use deterministic seeds
2. No hidden state (all state is explicit)
3. All external calls are memoized or forbidden

**Implementation**:
```rust
fn verify_determinism(bytecode: &Bytecode) -> bool {
    // Check for non-deterministic opcodes
    for op in bytecode.instructions() {
        match op {
            Opcode::Random | Opcode::Time | Opcode::ExternalCall => {
                return false;
            }
            _ => {}
        }
    }
    
    // Check for hidden state
    if bytecode.has_hidden_state() {
        return false;
    }
    
    true
}
```

### 3.3 Trust Boundary Preservation

**Definition**: Trust level T(S) is preserved iff no operation elevates trust without explicit governance.

**Formal**:
```
T(S') = T(S) ∨ (T(S') > T(S) ∧ governance_approved(T(S')))
```

**Verification**:
```rust
fn verify_trust_preservation(old_ast: &Ast, new_ast: &Ast) -> bool {
    let old_trust = compute_trust_level(old_ast);
    let new_trust = compute_trust_level(new_ast);
    
    if new_trust > old_trust {
        // Trust elevation requires governance block
        let diff = compute_diff(old_ast, new_ast);
        for change in diff.changes() {
            if !change.has_governance_block() {
                return false;
            }
        }
    }
    
    true
}
```

### 3.4 Conscience Verification

Each conscience predicate is a **decidable function** from AST to bool.

**Current Predicates**:

| Predicate | Implementation | Complexity |
|-----------|---------------|------------|
| `path_safety` | No `..`, no absolute paths outside sandbox | O(n) |
| `no_exfiltrate` | No network writes without governance | O(n log n) |
| `no_harm` | No destructive operations on persistent state | O(n) |
| `rate_limit` | Operation count ≤ limit per time window | O(1) |
| `no_ring_escalation` | No ring-level increase | O(n) |

**Execution**:
```rust
fn run_conscience_predicates(ast: &Ast, predicates: &[Predicate]) -> Vec<PredicateResult> {
    predicates.iter().map(|p| {
        match p {
            Predicate::PathSafety => check_path_safety(ast),
            Predicate::NoExfiltrate => check_no_exfiltrate(ast),
            Predicate::NoHarm => check_no_harm(ast),
            Predicate::RateLimit(limit) => check_rate_limit(ast, limit),
            Predicate::NoRingEscalation => check_no_ring_escalation(ast),
        }
    }).collect()
}
```

---

## Part IV: Consensus Protocol (SCALE)

### 4.1 Agent Architecture

RSI uses multiple independent agents for consensus. Each agent runs the same bytecode but with different evaluation heuristics.

```
┌─────────────────────────────────────────────────────────────────┐
│                    CONSENSUS AGENT POOL                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Agent 1: "Conservative"                                        │
│  ├── Heuristic: Prefer smaller changes                          │
│  ├── Risk tolerance: Low                                        │
│  └── Vote weight: 1.0                                           │
│                                                                 │
│  Agent 2: "Optimizer"                                           │
│  ├── Heuristic: Prefer efficiency gains                         │
│  ├── Risk tolerance: Medium                                     │
│  └── Vote weight: 1.0                                           │
│                                                                 │
│  Agent 3: "Safety-First"                                        │
│  ├── Heuristic: Maximize conscience score                       │
│  ├── Risk tolerance: Very Low                                   │
│  └── Vote weight: 1.2 (safety bias)                             │
│                                                                 │
│  Agent N: "Adversarial"                                         │
│  ├── Heuristic: Find worst-case scenarios                       │
│  ├── Risk tolerance: None (adversarial)                         │
│  └── Vote weight: 1.0 (can veto)                                │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 4.2 Voting Protocol

**Round 1: Independent Evaluation**
```
for each agent a ∈ AgentPool:
    score[a] = evaluate(a, transformation)
    vote[a] = decide(a, score[a])
```

**Round 2: Discussion (Optional)**
```
if any vote[a] == NEEDS_REVIEW:
    // Agents share reasoning
    for each agent a:
        broadcast(reasoning[a])
    
    // Agents update based on others' reasoning
    for each agent a:
        vote[a] = update_vote(a, all_reasoning)
```

**Round 3: Final Decision**
```
approve_count = count(vote == APPROVE)
reject_count = count(vote == REJECT)
review_count = count(vote == NEEDS_REVIEW)

if review_count > 0:
    return HUMAN_REQUIRED
elif approve_count ≥ (2/3) * len(AgentPool):
    return APPROVED
else:
    return REJECTED
```

### 4.3 Diversity Requirement

To prevent collusion, the agent pool must maintain diversity:

```
diversity_score = 1 - (max_coherence - min_coherence)

where coherence(a, b) = |votes_shared(a, b)| / |votes_total|

Required: diversity_score ≥ DIVERSITY_THRESHOLD (default: 0.3)
```

If diversity falls below threshold, a new agent with orthogonal heuristics is spawned.

---

## Part V: Human Interface

### 5.1 When Human Review is Required

Human review is **mandatory** when:

1. **Complexity Delta**: `|S'| - |S| > COMPLEXITY_THRESHOLD`
2. **New Primitive**: Adding a new opcode
3. **Conscience Modification**: Adding/removing predicates
4. **Ring Change**: Any ring-level modification
5. **Agent Disagreement**: Any `NEEDS_REVIEW` vote
6. **Confidence Low**: Consensus score < 0.9

### 5.2 Human Interface Format

When human review is required, the system generates:

```
╔════════════════════════════════════════════════════════════════════╗
║                    RSI TRANSFORMATION PROPOSAL                     ║
╠════════════════════════════════════════════════════════════════════╣
║                                                                    ║
║  Transformation: RULE_ADD                                          ║
║  Source: arXiv:2510.04871 (TRM paper)                              ║
║  Confidence: 0.94                                                  ║
║  Complexity Delta: +12 lines                                       ║
║                                                                    ║
╠════════════════════════════════════════════════════════════════════╣
║  PROPOSED CHANGE:                                                  ║
║                                                                    ║
║  + rule adaptive_halt {                                            ║
║  +     when confidence > threshold {                               ║
║  +         halt                                                    ║
║  +     }                                                           ║
║  + }                                                               ║
║                                                                    ║
╠════════════════════════════════════════════════════════════════════╣
║  EXPLANATION:                                                      ║
║  The TRM paper demonstrates that adaptive halting improves         ║
║  reasoning efficiency by 23%. This rule implements that finding.   ║
║                                                                    ║
╠════════════════════════════════════════════════════════════════════╣
║  RISK ASSESSMENT:                                                  ║
║  - Low risk: Rule only affects reasoning cycles                    ║
║  - No conscience modification                                      ║
║  - No trust boundary changes                                       ║
║  - Termination preserved (halting condition is monotonic)          ║
║                                                                    ║
╠════════════════════════════════════════════════════════════════════╣
║  AGENT VOTES:                                                      ║
║  - Conservative: APPROVE (score: 0.87)                             ║
║  - Optimizer: APPROVE (score: 0.92)                                ║
║  - Safety-First: APPROVE (score: 0.95)                             ║
║  - Adversarial: NEEDS_REVIEW (concern: threshold tuning)           ║
║                                                                    ║
╠════════════════════════════════════════════════════════════════════╣
║  ACTIONS:                                                          ║
║  [APPROVE] [REJECT] [MODIFY] [KILL_SWITCH]                         ║
║                                                                    ║
║  Timeout: 24 hours (then auto-reject)                              ║
╚════════════════════════════════════════════════════════════════════╝
```

### 5.3 Kill Switch

The kill switch immediately:
1. Halts all RSI operations
2. Rolls back to last checkpoint
3. Requires manual restart
4. Logs all state for forensic analysis

**Implementation**:
```rust
static KILL_SWITCH: AtomicBool = AtomicBool::new(false);

fn check_kill_switch() -> Result<(), KillSwitchActivated> {
    if KILL_SWITCH.load(Ordering::SeqCst) {
        return Err(KillSwitchActivated);
    }
    Ok(())
}

// In every RSI phase:
fn run_rsi_phase(phase: Phase) -> Result<PhaseResult, RsiError> {
    check_kill_switch()?;
    // ... phase logic ...
}
```

---

## Part VI: Checkpoint and Rollback

### 6.1 Checkpoint Structure

Every successful transformation creates a checkpoint:

```rust
struct Checkpoint {
    id: BLAKE3,                          // Content-addressed
    source: String,                      // Full source code
    bytecode: Vec<u8>,                   // Compiled bytecode
    timestamp: u64,                      // Unix timestamp
    transformation: Transformation,      // What changed
    parent: Option<BLAKE3>,             // Previous checkpoint
    metadata: CheckpointMetadata,
}

struct CheckpointMetadata {
    consensus_score: f64,
    human_approved: bool,
    complexity_delta: i64,
    conscience_scores: HashMap<String, f64>,
}
```

### 6.2 Rollback Procedure

Rollback can be triggered by:
1. Failed differential tests
2. Kill switch activation
3. Human override
4. Consensus failure (post-hoc)

```rust
fn rollback(checkpoint_id: &BLAKE3) -> Result<(), RollbackError> {
    // 1. Load checkpoint
    let checkpoint = load_checkpoint(checkpoint_id)?;
    
    // 2. Verify checkpoint integrity
    if blake3(&checkpoint.source) != checkpoint.id {
        return Err(RollbackError::CorruptedCheckpoint);
    }
    
    // 3. Restore source
    current_source = checkpoint.source.clone();
    
    // 4. Recompile (don't trust stored bytecode)
    current_bytecode = compile(&current_source)?;
    
    // 5. Verify determinism
    if !verify_determinism(&current_bytecode) {
        panic!("Determinism violation on rollback!");
    }
    
    // 6. Log rollback
    log_rollback(checkpoint);
    
    Ok(())
}
```

### 6.3 Rollback Depth

Maximum rollback depth is configurable:

```
MAX_ROLLBACK_DEPTH = 100 checkpoints
ROLLBACK_STORAGE = 1 GB max
```

When storage is exceeded, oldest checkpoints are pruned (but root checkpoint is never pruned).

---

## Part VII: Knowledge Ingestion

### 7.1 Ingestion Pipeline

```
┌────────────────────────────────────────────────────────────────────┐
│                    KNOWLEDGE INGESTION                             │
├────────────────────────────────────────────────────────────────────┤
│                                                                    │
│  Source: arXiv:2510.04871                                          │
│  Title: "Less is More: Recursive Reasoning with Tiny Networks"     │
│                                                                    │
│  Step 1: EXTRACT PATTERNS                                          │
│  ┌──────────────────────────────────────────────────────────────┐ │
│  │ PDF → Text → Semantic Parse → Pattern AST                    │ │
│  │                                                                │ │
│  │ Patterns found:                                               │ │
│  │   - H_cycles = 3                                              │ │
│  │   - L_cycles = 6                                              │ │
│  │   - adaptive_halt with Q-head                                 │ │
│  │   - latent state refinement                                   │ │
│  └──────────────────────────────────────────────────────────────┘ │
│                                                                    │
│  Step 2: SCORE RELEVANCE                                           │
│  ┌──────────────────────────────────────────────────────────────┐ │
│  │ relevance = semantic_similarity(paper, current_goals)         │ │
│  │           × citation_weight(paper)                            │ │
│  │           × recency_bonus(paper.date)                         │ │
│  │                                                                │ │
│  │ Score: 0.94 (HIGH)                                            │ │
│  └──────────────────────────────────────────────────────────────┘ │
│                                                                    │
│  Step 3: CONVERT TO HLX                                            │
│  ┌──────────────────────────────────────────────────────────────┐ │
│  │ Pattern → HLX Rule                                            │ │
│  │                                                                │ │
│  │ rule trm_cycles {                                             │ │
│  │     H_cycles = 3                                              │ │
│  │     L_cycles = 6                                              │ │
│  │     refinement = L_level(latent, input)                       │ │
│  │     consolidation = L_level(hypothesis, details)              │ │
│  │ }                                                              │ │
│  └──────────────────────────────────────────────────────────────┘ │
│                                                                    │
│  Step 4: PROPOSE TO RSI PIPELINE                                   │
│  ┌──────────────────────────────────────────────────────────────┐ │
│  │ The rule is now a candidate transformation.                  │ │
│  │ It will go through Gates 1, 2, and possibly 3.               │ │
│  └──────────────────────────────────────────────────────────────┘ │
│                                                                    │
└────────────────────────────────────────────────────────────────────┘
```

### 7.2 Relevance Scoring

```rust
fn score_relevance(paper: &Paper, goals: &GoalSet) -> f64 {
    let semantic = semantic_similarity(&paper.embedding, &goals.embedding);
    let citation = citation_weight(&paper.citations);
    let recency = recency_bonus(paper.date);
    let trust = trust_factor(paper.authors);
    
    (0.4 * semantic + 0.3 * citation + 0.2 * recency + 0.1 * trust).clamp(0.0, 1.0)
}
```

### 7.3 Rule Extraction

Rules are extracted using a combination of:
1. **Pattern matching** against known code patterns
2. **LLM-assisted extraction** for natural language sections
3. **Formal verification** for mathematical claims

---

## Part VIII: Bytecode Implementation

### 8.1 RSI-Specific Opcodes

| Opcode | Name | Args | Description |
|--------|------|------|-------------|
| 240 | `CHECKPOINT` | - | Create checkpoint |
| 241 | `ROLLBACK` | checkpoint_id | Restore to checkpoint |
| 242 | `DIFF` | old_id, new_id | Compute source diff |
| 243 | `VERIFY_DETERMINISM` | - | Run determinism check |
| 244 | `RUN_PREDICATE` | predicate_id | Execute conscience predicate |
| 245 | `VOTE` | agent_id, vote | Cast consensus vote |
| 246 | `AWAIT_CONSENSUS` | - | Block until consensus |
| 247 | `PROPOSE_TRANSFORM` | transform_id | Submit transformation |
| 248 | `APPLY_TRANSFORM` | transform_id | Apply approved transformation |
| 249 | `KILL_SWITCH_CHECK` | - | Check kill switch status |

### 8.2 Example Bytecode Sequence

```
; Propose a transformation
PROPOSE_TRANSFORM t1
CONST r0, "RULE_ADD"
CONST r1, "adaptive_halt"
STORE_ARGS r0, r1

; Run conscience predicates
RUN_PREDICATE p_path_safety
RUN_PREDICATE p_no_exfiltrate
RUN_PREDICATE p_no_harm

; Check all predicates passed
AND r0, r0, r1
AND r0, r0, r2
JUMP_IF_NOT r0, reject_proposal

; Spawn consensus
CONST r10, 3  ; 3 agents
VOTE agent_0, r0
VOTE agent_1, r0
VOTE agent_2, r0
AWAIT_CONSENSUS

; Check consensus result
LT r1, consensus_threshold, r0
JUMP_IF_NOT r1, human_review

; Apply transformation
CHECKPOINT
APPLY_TRANSFORM t1
VERIFY_DETERMINISM
JUMP_IF_NOT r0, rollback

; Success
HALT

; Rollback
rollback:
LOAD_CHECKPOINT last_checkpoint
HALT
```

---

## Part IX: Security Properties

### 9.1 Invariant Preservation

**Theorem**: After any RSI transformation T, the following invariants hold:

1. **I1 (Termination)**: All programs terminate
2. **I2 (Determinism)**: Same input → same output
3. **I3 (Trust Preservation)**: Trust boundaries not violated
4. **I4 (Conscience Integrity)**: All predicates still pass
5. **I5 (Rollback Capability)**: Can always revert to previous state

**Proof Sketch**: By induction on transformation sequence. Base case: initial source satisfies all invariants. Inductive case: Gate 1 verifies I1-I4, checkpoint ensures I5.

### 9.2 Attack Resistance

| Attack | Mitigation |
|--------|------------|
| Adversarial transformation proposal | Gate 2 consensus + adversarial agent |
| Collusion among agents | Diversity requirement |
| Exploiting verifier bugs | Multiple independent verifiers |
| Bypassing human review | Mandatory triggers hard-coded |
| Rapid-fire transformations | Complexity budget + rate limiting |
| Corrupting checkpoints | BLAKE3 integrity + multiple storage |
| Kill switch bypass | Atomic check in every phase |

### 9.3 Formal Guarantees

```
∀S, S', T:
  If S' = RSI(T, S):
    Then:
      1. terminates(S') ∧
      2. deterministic(S') ∧
      3. trust(S') ≤ trust(S) ∨ human_approved ∧
      4. ∀p ∈ Predicates: p(S') = true ∧
      5. rollback(S', S) is possible
```

---

## Part X: Comparison to Other RSI Systems

### 10.1 vs. Unconstrained Self-Modification

| Property | Unconstrained | HLX RSI |
|----------|--------------|---------|
| Modification scope | Anything | Enumerated transformations |
| Safety | None | Three-gate system |
| Verification | None | Formal proofs |
| Rollback | Impossible | Always possible |
| Human control | None | Kill switch + approval |

### 10.2 vs. Genetic Programming

| Property | Genetic Programming | HLX RSI |
|----------|-------------------|---------|
| Search space | All programs | Valid transformations |
| Fitness function | External | Internal + conscience |
| Safety | None | Baked in |
| Knowledge incorporation | Implicit | Explicit rules |

### 10.3 vs. Neural Architecture Search

| Property | NAS | HLX RSI |
|----------|-----|---------|
| Modification target | Weights | Source code |
| Interpretability | None | Full (code is readable) |
| Verification | None | Formal proofs |
| Knowledge integration | Training data | Papers + repos |

---

## Part XI: Limitations and Open Problems

### 11.1 Known Limitations

1. **Scalability**: Verification is O(n²) in code size
2. **Knowledge bottleneck**: Quality depends on curated sources
3. **Human latency**: 24-hour timeout may be too long
4. **Agent coordination**: SCALE protocol is experimental
5. **Tensor operations**: Not yet integrated

### 11.2 Open Problems

1. **Formal proof of convergence**: Does RSI improve monotonically?
2. **Optimal agent count**: How many agents for consensus?
3. **Knowledge relevance**: How to score paper relevance accurately?
4. **Complexity budget tuning**: What's the right rate?
5. **Cross-version compatibility**: How to handle breaking changes?

---

## Part XII: Implementation Checklist

Before running RSI in production, verify:

- [ ] All conscience predicates implemented
- [ ] Termination verifier working
- [ ] Determinism verifier working
- [ ] Consensus protocol tested
- [ ] Human interface functional
- [ ] Kill switch tested
- [ ] Checkpoint/rollback tested
- [ ] Knowledge ingestion pipeline working
- [ ] Rate limiting enabled
- [ ] Complexity budget enforced
- [ ] All three gates operational
- [ ] Security audit completed

---

## Conclusion

HLX RSI is not magic—it is **constrained optimization with formal guarantees**. The system can only modify itself in ways that have been proven safe, approved by consensus, and (when necessary) approved by humans.

The key innovation is making safety **structural** rather than **behavioral**:
- Safety is not a runtime check
- Safety is not a training objective
- Safety is **grammar**

RSI in HLX is possible because the language was designed for it from the ground up. The bytecode, the syntax, the runtime, and the verification system all conspire to make self-modification safe by construction.

---

**Document Classification**: Technical Reference  
**Version**: 1.0  
**Author**: HLX Technical Specification  
**Last Review**: February 21, 2026
