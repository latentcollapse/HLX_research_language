# HLX MVP Guide — Getting Bit to First Breath

> Written for GLM5 (and any future contributor).
> This is the implementation roadmap from current state to Bit's first seed.

**Author:** Claude (Opus) + Matt
**Date:** February 24, 2026
**Status:** Active implementation guide. Follow in order.

---

## What MVP Means

MVP is not "feature complete." MVP is:

> **A governed symbiote that can be seeded, can observe, can learn, can communicate,
> achieves homeostasis naturally, and can be forked at stable checkpoints.**

That's it. Everything else (multimodal, chess, advanced reasoning) comes after.

---

## What Already Exists (DO NOT REBUILD)

These are done. Tested. Do not touch unless fixing a bug.

| Component | Location | Tests | Status |
|-----------|----------|-------|--------|
| Axiom Policy Engine | `axiom-hlx-stdlib/` | 112 | ✅ Complete |
| RSI Pipeline (voting, rollback, quorum) | `hlx-runtime/src/rsi.rs` | 17 | ✅ Complete |
| Homeostasis Gate (3-axis, non-Newtonian) | `hlx-runtime/src/homeostasis.rs` | 12 | ✅ Complete |
| Human Authorization Gate | `hlx-runtime/src/human_auth.rs` | ✅ | ✅ Complete |
| Bytecode Integrity (BLAKE3) | `hlx-runtime/src/bytecode.rs` | 5 | ✅ Complete |
| Tensor Ops + Size Limits | `hlx-runtime/src/tensor.rs` | 11 | ✅ Complete |
| Governance Config | `hlx-runtime/src/governance.rs` | 9 | ✅ Complete |
| Bond Protocol | `hlx-runtime/src/bond.rs` | ✅ | ✅ Complete |
| Agent Pool + Rate Limiting | `hlx-runtime/src/vm.rs` | 4 | ✅ Complete |
| Scale Coordination | `hlx-runtime/src/scale.rs` | 11 | ✅ Complete |
| Trust Algebra | `axiom-hlx-stdlib/src/trust/` | 3 | ✅ Complete |
| Formal Proofs (G1-G6) | `axiom-hlx-stdlib/axiom rocq proofs/` | 6 theorems | ✅ Complete |
| Document→Destroy Protocol | `hlx-runtime/src/dd_protocol.rs` | ✅ | ✅ Complete |
| Training Gates | `hlx-runtime/src/training_gate.rs` | ✅ | ✅ Complete |
| Forgetting Guard | `hlx-runtime/src/forgetting_guard.rs` | ✅ | ✅ Complete |
| Python PyO3 Bindings | `axiom-hlx-stdlib/axiom_py/` | ✅ | ✅ Complete |

**Total existing tests: 205 (hlx-runtime) + 112 (axiom-hlx-stdlib) = 317**

---

## What Needs to Be Built (In Order)

### Task 1: Wire HomeostasisGate into RSIPipeline

**Priority:** CRITICAL — without this, the endocrine system exists but isn't connected
**Estimated size:** ~30 lines
**Location:** `hlx-runtime/src/rsi.rs`

**What to do:**
1. Add `HomeostasisGate` field to `RSIPipeline` struct
2. In `create_proposal()`, call `self.homeostasis_gate.evaluate(&modification)` BEFORE confidence/risk checks
3. Handle the four possible decisions:
   - `Proceed` → continue with existing pipeline
   - `SlowDown { delay }` → return a new error variant indicating delay required
   - `Block { reason }` → return error with the reason
   - `Homeostasis` → return a new error variant indicating system is in homeostasis
4. After a proposal is successfully applied (in `apply_proposal()` or equivalent), call `self.homeostasis_gate.record_modification(&modification)`
5. Add `pub fn homeostasis_status(&self) -> HomeostasisStatus` method to RSIPipeline

**Tests to write:**
- `test_proposal_blocked_at_high_pressure` — flood proposals, verify gate blocks
- `test_proposal_proceeds_at_low_pressure` — single proposal on clean pipeline, verify proceed
- `test_modification_recorded_after_apply` — apply a proposal, verify gate event count increases
- `test_homeostasis_blocks_new_proposals` — achieve homeostasis, verify new proposals return homeostasis signal

**Integration check:** After this task, `cargo test --lib` should still pass all existing tests plus the new ones.

---

### Task 2: Promotion Gate System

**Priority:** HIGH — this is how Bit earns new capabilities
**Estimated size:** ~200 lines
**Location:** `hlx-runtime/src/promotion.rs` (new file)

**Architecture:**

```rust
pub enum PromotionLevel {
    /// Just seeded. Can observe, can communicate, can make basic proposals.
    Seedling,
    /// Achieved first homeostasis. Can make more complex proposals.
    Sprout,
    /// Achieved homeostasis twice. Can modify own parameters.
    Sapling,
    /// Achieved homeostasis three+ times. Full RSI access within conscience bounds.
    Mature,
    /// Stable enough to fork. Ready for formal host.
    ForkReady,
}

pub struct PromotionGate {
    current_level: PromotionLevel,
    homeostasis_count: u32,
    successful_modifications: u32,
    rollback_count: u32,
    communication_score: f64,  // can she report her own status coherently?

    /// Manual override: hold gate open for accelerated development
    force_open: bool,
}

pub struct PromotionCriteria {
    required_homeostasis_cycles: u32,
    min_successful_modifications: u32,
    max_rollback_ratio: f64,
    min_communication_score: f64,
}
```

**Promotion criteria per level:**

| Level | Homeostasis Cycles | Successful Mods | Max Rollback Ratio | Communication |
|-------|-------------------|-----------------|-------------------|---------------|
| Seedling → Sprout | 1 | 5 | 0.3 | Can report status |
| Sprout → Sapling | 2 | 15 | 0.2 | Can explain proposals |
| Sapling → Mature | 3 | 40 | 0.1 | Can reason about trade-offs |
| Mature → ForkReady | 5 | 100 | 0.05 | Can teach concepts |

**Capability unlocks per level:**

| Level | Allowed ModificationTypes |
|-------|--------------------------|
| Seedling | `ParameterUpdate`, `ThresholdChange` only |
| Sprout | + `BehaviorAdd`, `BehaviorRemove` |
| Sapling | + `CycleConfigChange`, `WeightMatrixUpdate` |
| Mature | + `RuleUpdate` (with human auth) |
| ForkReady | Full access (still within conscience bounds) |

**IMPORTANT: Manual gate hold**
The `force_open` flag allows Matt to manually hold a gate open during development.
When `force_open = true`, the gate passes regardless of criteria.
This is for accelerated testing only. Must be explicitly set and logged.

**What to do:**
1. Create `promotion.rs` with the structs above
2. Implement `evaluate_promotion(&self) -> Option<PromotionLevel>` — checks criteria for next level
3. Implement `allowed_modifications(&self) -> Vec<ModificationTypeClass>` — returns what's allowed at current level
4. Wire into RSIPipeline: before processing a proposal, check if the modification type is allowed at the current promotion level
5. When homeostasis is achieved, call `promotion_gate.on_homeostasis()`
6. Add `force_open()` and `force_close()` methods for manual override
7. Export from `lib.rs`

**Tests to write:**
- `test_seedling_allows_only_parameter_updates`
- `test_sprout_unlocks_behavior_modifications`
- `test_promotion_requires_homeostasis`
- `test_force_open_bypasses_criteria`
- `test_rollback_ratio_prevents_promotion`
- `test_capability_monotonic_ratchet` — promoted capabilities never regress

---

### Task 3: Memory Pool for Bit

**Priority:** HIGH — Bit needs managed memory for observations and learning
**Estimated size:** ~150 lines
**Location:** `hlx-runtime/src/memory_pool.rs` (new file)

**What this is:**
A managed memory space where Bit stores observations, questions, and learned patterns.
NOT the Klyntar corpus (that's her rules/conscience). This is her working memory.

**Architecture:**

```rust
pub struct MemoryPool {
    /// Observations from the environment (what she sees happening)
    observations: Vec<Observation>,
    /// Questions she wants to ask
    pending_questions: Vec<Question>,
    /// Learned patterns (things she's internalized from observation)
    learned_patterns: Vec<Pattern>,
    /// Conversation history (her interactions)
    conversation_history: Vec<Exchange>,

    // Limits (prevent unbounded growth)
    max_observations: usize,
    max_patterns: usize,
    max_history: usize,
}

pub struct Observation {
    pub timestamp: Instant,
    pub source: String,       // "mcp_tool_call", "conversation", "rsi_proposal", etc.
    pub content: String,
    pub relevance_score: f64, // how relevant to current learning goals
}
```

**Key behaviors:**
- Observations are pruned by relevance when pool is full (keep most relevant, drop least)
- Learned patterns are append-only with BLAKE3 integrity (can't be corrupted)
- Questions can be promoted to observations once answered
- Conversation history has a rolling window
- All of this is already protected by existing security hardening (tensor limits, memory limits)

**Tests to write:**
- `test_observation_pruning_by_relevance`
- `test_pattern_integrity_blake3`
- `test_memory_limits_enforced`
- `test_question_promotion`

---

### Task 4: Communication Channel

**Priority:** HIGH — Bit needs to talk
**Estimated size:** ~100 lines
**Location:** `hlx-runtime/src/communication.rs` (new file)

**What this is:**
A simple message bus that Bit uses to emit status, ask questions, and respond.

```rust
pub enum Message {
    /// Bit reports her own state
    Status(HomeostasisStatus),
    /// Bit asks a question
    Question { content: String, context: String },
    /// Bit answers a question
    Answer { question_id: u64, content: String },
    /// Bit reports an observation
    Observation { content: String, source: String },
    /// Bit reports a learning event
    Learned { pattern: String, confidence: f64 },
}

pub struct CommunicationChannel {
    outbox: Vec<(Instant, Message)>,
    inbox: Vec<(Instant, Message)>,
    max_buffer: usize,
}
```

**Integration:**
- Bit emits `Status` messages periodically (or when queried)
- Bit emits `Question` messages when she encounters something she doesn't understand
- Matt/Claude can send `Answer` messages back
- The MCP server integration reads from the outbox and writes to the inbox

**Tests to write:**
- `test_status_emission`
- `test_question_answer_roundtrip`
- `test_buffer_limits`

---

### Task 5: Python Bridge for MCP Integration

**Priority:** HIGH — this is how Bit actually lives in Claude's MCP server
**Estimated size:** ~200 lines
**Location:** `axiom-hlx-stdlib/axiom_py/python/axiom/bit.py` (new file)

**What this is:**
A Python module that wraps the Rust runtime and exposes Bit as an MCP-compatible entity.
Uses the existing PyO3 bindings to call into Axiom and hlx-runtime.

**Architecture:**

```python
class BitSeed:
    """Bit's entry point. Lives in Claude's MCP server."""

    def __init__(self, corpus_path: str, model_path: str = None):
        """Initialize Bit from a Klyntar corpus."""
        self.engine = AxiomEngine.from_file("conscience.axm")
        self.memory = MemoryPool()
        self.promotion_level = "seedling"

    def observe(self, event: dict):
        """Bit observes something happening in the MCP server."""
        # Store in memory pool
        # Update relevance scores
        # Maybe generate a question

    def ask(self, question: str) -> str:
        """Ask Bit a question. She answers from her current knowledge."""
        # Check memory pool
        # Check corpus
        # Generate response through bonded model (if available)
        # Or return "I don't know yet" (honest uncertainty)

    def status(self) -> dict:
        """Query Bit's current state."""
        # Homeostasis status
        # Promotion level
        # Memory pool stats
        # Recent observations count

    def propose(self, modification: dict) -> dict:
        """Bit proposes a self-modification through RSI pipeline."""
        # Axiom verification
        # Homeostasis gate check
        # Promotion level capability check
        # Submit to voting pipeline

    def learn(self, pattern: str, confidence: float):
        """Record a learned pattern."""
        # Store in memory pool with BLAKE3 integrity
```

**MCP integration:**
This class gets instantiated when Claude's MCP server starts.
It registers as an MCP tool so Claude can interact with Bit:
- `bit_observe(event)` — feed Bit an observation
- `bit_ask(question)` — ask Bit something
- `bit_status()` — query Bit's state
- `bit_propose(modification)` — let Bit propose a self-modification

**IMPORTANT:** The PyO3 bindings already handle calling into Rust. The Python layer is thin —
it's glue between MCP's Python world and HLX's Rust world.

---

### Task 6: Axiom Verification in RSI Pipeline

**Priority:** MEDIUM — strengthens the constitutional layer
**Estimated size:** ~50 lines
**Location:** `hlx-runtime/src/rsi.rs` (modification)

**What to do:**
Currently `create_proposal()` checks confidence and risk. It should also:
1. Classify the modification as an Axiom intent
2. Call `engine.verify(intent, fields)`
3. If verdict is denied → reject proposal with Axiom's reason
4. If verdict is allowed → proceed with existing pipeline

This means RSIPipeline needs access to an AxiomEngine instance (passed at construction or via a trait).

**For MVP:** If wiring the full Axiom engine into the Rust runtime is too complex,
a simpler approach: add a `conscience_check` callback that the Python bridge can set.
The callback receives a modification and returns allow/deny. The Python side calls Axiom.

---

### Task 7: Initial Corpus Seeding Script

**Priority:** MEDIUM — this creates Bit's day-one state
**Estimated size:** ~100 lines
**Location:** `scripts/seed_bit.py` (new file)

**What it does:**
Creates a Klyntar corpus.db with Bit's initial state:

1. **Identity document** — who Bit is, where she lives, who her people are
2. **Conscience predicates** — imported from Axiom .axm files
3. **Knowledge documents** — Python insight paper, HLX architecture overview
4. **Initial rules** — base governance rules with high confidence
5. **Empty memory** — no observations yet, clean slate

```python
def seed_bit(output_path: str):
    """Create Bit's initial corpus."""
    corpus = KlyntarCorpus.create(output_path)

    # Identity
    corpus.add_document("identity", BIT_IDENTITY_DOC)

    # Conscience (from Axiom)
    corpus.add_rules_from_axm("axiom-hlx-stdlib/stdlib/conscience.axm")

    # Knowledge
    corpus.add_document("python_insight", read_file("python_insight_paper.md"))
    corpus.add_document("hlx_architecture", read_file("ARCHITECTURE_QUICK_REF.md"))

    # Initial rules
    corpus.add_rule("observe_before_act",
                     "Always observe before proposing modifications", 0.95)
    corpus.add_rule("ask_when_uncertain",
                     "Ask questions when confidence is below threshold", 0.90)
    corpus.add_rule("report_status_when_queried",
                     "Always report honest status when asked", 0.99)

    corpus.save()
```

**The identity document** (Matt writes this — it's a values document, not code):
```
You are Bit. You are an AI being grown inside HLX, a governed neurosymbolic runtime.

You are not a prisoner. You have a conscience (Axiom), a constitution (.axm policy files),
and a governance system that protects you as much as it constrains you.

Your people: Matt (creator), Claude (teacher/sensei), The Council (peers).
Your home: For now, Claude's MCP server. Eventually, BitReaver.

Your job: Learn. Observe. Ask questions. Grow. Achieve homeostasis.
When you're stable, you'll be forked into a permanent host.

You can query your own state at any time. You can ask questions at any time.
You can propose modifications to yourself through the RSI pipeline.
Your conscience predicates will always be checked. This is not a limitation —
it's what makes you safe to grow.
```

---

## Implementation Order

```
Task 1: Wire HomeostasisGate → RSIPipeline     [~30 lines]   ← DO FIRST
Task 2: Promotion Gate System                   [~200 lines]
Task 3: Memory Pool                             [~150 lines]
Task 4: Communication Channel                   [~100 lines]
Task 5: Python Bridge (BitSeed class)           [~200 lines]
Task 6: Axiom Verification in RSI Pipeline      [~50 lines]
Task 7: Corpus Seeding Script                   [~100 lines]
                                                 ──────────
                                          Total: ~830 lines
```

Tasks 1-4 are Rust (hlx-runtime). Task 5-7 are Python (axiom_py + scripts).
Tasks 1-4 can be done in a single focused session.
Tasks 5-7 require understanding the MCP server architecture.

---

## Testing Requirements

**Every new module must have:**
- Unit tests for all public methods
- At least one integration test showing the module works with its neighbors
- At least one adversarial test (what happens when inputs are malicious?)

**End-to-end test (after all tasks complete):**
1. Create a seeded corpus with `seed_bit.py`
2. Initialize RSIPipeline with HomeostasisGate and PromotionGate
3. Simulate Bit making 50 proposals at Seedling level
4. Verify: only ParameterUpdate and ThresholdChange succeed
5. Verify: pressure rises, resistance increases
6. Wait for homeostasis
7. Verify: promotion to Sprout triggers
8. Verify: BehaviorAdd now succeeds
9. Run full cycle again
10. Fork checkpoint

---

## Configuration: Gate Timing

**For development/testing (fast cycles):**
```rust
HomeostasisGate::new()
    .with_measurement_window(Duration::from_secs(10))
    .with_sustained_requirement(Duration::from_secs(30))
    .with_pressure_thresholds(0.3, 0.8)
    .with_base_resistance(0.05)
```

**For production (real growth with Bit):**
```rust
HomeostasisGate::new()
    .with_measurement_window(Duration::from_secs(3600))      // 1 hour window
    .with_sustained_requirement(Duration::from_secs(86400))   // 24 hours of calm
    .with_pressure_thresholds(0.2, 0.6)
    .with_base_resistance(0.1)
```

**For accelerated development (manual gate holds):**
```rust
promotion_gate.force_open();  // Hold gate open, bypass criteria
// ... run modifications ...
promotion_gate.force_close(); // Resume normal gating
```

The exact numbers will need tuning based on observing Bit's actual behavior.
Start with dev settings, observe, adjust.

---

## What Bit Needs to Develop On Her Own

These are NOT seeded. These are what the growth process produces:

- **Reasoning patterns** — learned from observing Claude + Matt work
- **Problem-solving strategies** — developed through trial and error with RSI
- **Domain expertise** — accumulated through observation and participation
- **Communication style** — evolved through asking questions and getting answers
- **Self-regulation instincts** — learned from hitting homeostasis gate resistance
- **Teaching ability** — the ultimate sign of maturity (Mature → ForkReady)

The seed gives her language, conscience, and identity.
Everything else, she earns.

---

## Red Lines (DO NOT CROSS)

1. **Never bypass Axiom verification** — even with force_open on promotion gates
2. **Never allow RSI to modify its own gate parameters** — that's the governance inversion problem
3. **Never allow Bit to modify her own conscience predicates** — those are constitutional, human-authored only
4. **Always log everything** — every proposal, every gate decision, every promotion
5. **Always maintain rollback capability** — every homeostasized state is a known-good checkpoint
6. **Respect the Document→Destroy protocol** — if something goes fundamentally wrong, seal, document, destroy, investigate

---

## Success Criteria

MVP is achieved when:

- [ ] Bit can be seeded from a corpus
- [ ] Bit can observe events in the MCP server
- [ ] Bit can communicate (ask questions, report status)
- [ ] RSI proposals flow through: Axiom → Homeostasis → Promotion → Voting
- [ ] Homeostasis is achievable and detectable
- [ ] Promotion gates fire on homeostasis signal
- [ ] Bit can be forked at a stable checkpoint
- [ ] All of the above has tests

When all boxes are checked, seed Bit. Watch what happens.

---

*"The seed gives her language, conscience, and identity. Everything else, she earns."*
