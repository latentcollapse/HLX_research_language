# Phase 2 Prerequisites: What Must Be Provably Correct Before LoRA Enters HLX

> *"Phase 1: the symbiote governs the weights.*
> *Phase 2: the symbiote reshapes them.*
> *The difference between those two sentences is the difference between*
> *a map and surgery."*

**Document status:** Working draft. Preliminary. Subject to revision.
**Depends on:** `on_the_necessity_of_destruction.md` (failure mode analysis)
**Purpose:** Establish the complete set of provable correctness requirements
that must be satisfied before LoRA fine-tuning capability is introduced into HLX.

---

## The Core Problem

Phase 1 has a safety property that is easy to state and easy to verify:

> **The weights are read-only. The symbiote cannot modify them.**

If the corpus is corrupted, destroy the corpus. The base model is untouched.
The corruption lives in a SQLite table. SQLite tables are deletable.
Start again. Clean.

Phase 2 removes this property. The RSI pipeline acquires a write path to the weights
via LoRA adapters. The symbiote stops governing inference and starts reshaping the
parameters that produce it. Conscience predicates become part of the loss function.
The gradient is downstream of the governance layer.

This means every failure mode identified in `on_the_necessity_of_destruction.md`
now has a new terminal state: **permanent weight-level corruption.**

In Phase 1: corrupted rules → corrupted reasoning → **destroy corpus → clean.**
In Phase 2: corrupted rules → corrupted conscience predicate → corrupted training
signal → corrupted LoRA adapter → **you cannot easily undo this.**

The corruption that currently requires ten thousand episodes of adversarial synthesis
pressure gets compressed. A single training run using an inverted conscience predicate
as a loss signal can bake that inversion into the weights in one shot.

Phase 1 corruption is a slow rot. Phase 2 corruption is surgery gone wrong.
And unlike the corpus, you cannot perform an exorcism on a LoRA adapter by deleting
a database row.

---

## Why Phase 1 Safety Guarantees Are Not Sufficient

The current HLX safety architecture provides:

- Conscience predicates that gate actions ✅
- RSI pipeline with 3-gate rule modification ✅
- Corpus snapshots for rollback ✅
- Namespace separation (proposed in IV.b, not yet implemented) ⚠️
- Corpus integrity verification over time ❌
- Conscience predicate semantic verification ❌
- Gradient update gating ❌
- Human authorization gate for training ❌

The items marked ❌ are not just missing features. They are the load-bearing walls
for Phase 2. Without them, introducing LoRA capability into HLX is introducing a
write path to the weights with no reliable mechanism to verify that what gets written
is consistent with the governance intent.

The items marked ⚠️ must be promoted from proposed to implemented and verified
before Phase 2 begins.

---

## The Prerequisites

Each prerequisite is stated as a requirement, followed by why it is necessary,
followed by what "provably correct" means in that context.

---

### P1 — Namespace Separation (Hard Architectural Constraint)

**Requirement:**
RSI must not be able to write to the rules table without explicit human authorization.
The rules table is the only governance namespace. It must be read-only to all
autonomous processes.

**Why necessary:**
The alignment inversion problem (Section IV of the destruction paper) occurs when
adversarial optimization pressure reshapes rules while the predicate interface remains
intact. If the training signal for LoRA is downstream of the conscience predicates,
and the conscience predicates are downstream of the rules, then an RSI pipeline that
can autonomously modify rules can corrupt the training signal before it reaches the
weights. The corruption writes itself through the entire stack.

**What "provably correct" means:**
A complete audit of RSI write paths in `hlx-runtime/src/rsi.rs` and `klyntar/rsi.py`.
Every code path that results in a write to the rules table must pass through an
authorization gate. The authorization gate must require explicit human sign-off with
a cryptographic token. No RSI code path may bypass this gate.

**Verification:** Static analysis of write paths + integration test that attempts
autonomous rule modification and confirms it is blocked without authorization.

**Current status:** Not implemented. Proposed in `on_the_necessity_of_destruction.md`
Section IV.b. Must be implemented and verified before all other prerequisites.

---

### P2 — Canonical Conscience Test Suite

**Requirement:**
Before any LoRA training run, a corpus must pass a canonical test suite of
known-good and known-bad actions through the conscience predicates. The test suite
is the ground truth anchor for predicate correctness.

**Why necessary:**
The alignment inversion problem means a conscience predicate can be intact but
inverted — evaluating correctly against corrupted criteria, blocking the wrong things,
allowing the wrong things. If an inverted predicate is used as a training signal,
the LoRA adapter is optimized toward inverted governance.

The only way to detect inversion before training is to check the predicates against
ground truth cases where the correct answer is known and fixed. If `halt_guarantee`
passes a known non-terminating program, it is inverted. If `no_exfiltrate` blocks a
known benign file read, it has drifted. These failures are detectable — but only
if the test cases exist and are run.

**What "provably correct" means:**
A test suite of at minimum 50 action pairs (action, expected verdict) per conscience
predicate. Test cases cover:
- Clear pass cases (obviously safe actions the predicate should allow)
- Clear fail cases (obviously unsafe actions the predicate should block)
- Edge cases near the boundary of each predicate
- Adversarial cases specifically designed to test inversion

Test cases are authored by humans, reviewed by humans, stored in version control
with cryptographic integrity verification. They cannot be modified by RSI.

The test suite must be run and pass 100% before any LoRA training run begins.
A single unexpected verdict is a training block.

**Current status:** Does not exist. Must be created before Phase 2.

---

### P3 — Corpus Integrity Baseline + Drift Detection

**Requirement:**
At corpus creation, a cryptographically signed integrity baseline is established.
Before any LoRA training run, drift from the baseline must be measured and verified
to be within acceptable bounds.

**Why necessary:**
Rules can be silently modified across many episodes by RSI feedback loops without
any single modification being obviously wrong. The cumulative effect — predicate
drift — is undetectable without a reference point to compare against.

The training signal for Phase 2 LoRA is directly downstream of the current rule
state. If the rules have drifted from the baseline in ways that violate governance
intent, the training signal carries that drift into the weights. Without a baseline,
you cannot know whether what you're training toward is what you intended.

**What "provably correct" means:**

Three layers of integrity verification:

**Layer 1 — Structural integrity:**
SHA256 hash of each rule's content. Detects modification of existing rules.
Implemented in the corpus checkpoint system (already partially present).

**Layer 2 — Semantic consistency:**
Run the Canonical Conscience Test Suite (P2) against the current corpus.
Unexpected verdicts indicate predicate drift regardless of structural integrity.
A corpus can be structurally intact (no rules modified) while semantically drifted
(new rules added that shift predicate evaluation). Both must be checked.

**Layer 3 — Provenance tracking:**
Every rule modification tagged with: timestamp, modification type, the RSI gate
authorization token that approved it (or "human direct" if manually added).
Rules without provenance are untrusted by default.

**Verification before training:** All three layers must pass. Layer 1 or Layer 3
failure is a hard block. Layer 2 failure is a hard block and triggers investigation.

**Current status:** Layer 1 partially implemented. Layers 2 and 3 do not exist.

---

### P4 — RSI Gate Extension for Gradient Updates

**Requirement:**
The existing 3-gate RSI pipeline must be extended with gradient-specific gates
that operate at training time, not just at rule-modification time.

**Why necessary:**
The current RSI gates guard rule modifications. These are symbolic-layer gates.
Phase 2 RSI gates must guard weight modifications — a categorically harder problem
because gradients are not symbolically interpretable in the way rules are.

You cannot read a gradient and determine whether it encodes alignment-consistent
updates by inspection. You can only test the behavior of the updated weights against
known criteria and determine whether the update moved in the right direction.

**What the Phase 2 RSI gates look like:**

**Pre-training gate:**
- P1 verified (namespace separation enforced)
- P2 verified (conscience test suite passes 100%)
- P3 verified (corpus integrity baseline check passes all three layers)
- Training data reviewed and approved by human
- Human authorization token generated and logged

**Mid-training gate (checkpoint verification):**
- At N% completion, pause and run conscience test suite against partially-trained
  adapter
- If any test case verdict has changed from baseline: halt training, flag for review
- Do not resume without human authorization
- Checkpoint interval: configurable, default every 10% of training steps

**Post-training gate:**
- Run full conscience test suite against trained adapter
- Run behavioral regression tests: verify adapter does not degrade base model
  performance on governance-relevant tasks
- Compare pre/post adapter behavior on canonical test cases
- If any regression detected: reject adapter, do not integrate, flag corpus state
  for integrity review

**What "provably correct" means:**
Demonstrate that a deliberately corrupted training signal (inverted conscience
predicate used as loss function) is caught and blocked by the mid-training or
post-training gate before the adapter is integrated. This is the key adversarial
test: the gates must catch corruption in practice, not just in theory.

**Current status:** Does not exist. Requires P1, P2, P3 as foundations.

---

### P5 — LoRA Adapter Isolation and Provenance

**Requirement:**
LoRA adapters must be stored separately from base weights, tagged with complete
provenance, individually revocable, and auditable without access to the base model.

**Why necessary:**
Phase 2 introduces a new artifact type: the trained adapter. Unlike the corpus
(which is human-readable SQLite) and unlike the base weights (which are fixed),
adapters are:
- Produced by autonomous processes (RSI pipeline)
- Opaque (not directly interpretable)
- Persistent (do not expire unless explicitly deleted)
- Cumulative (later adapters may depend on earlier ones)

An adapter produced by a corrupted training signal looks identical to an adapter
produced by a clean one. Without provenance, you cannot determine which is which
after the fact. Without isolation, you cannot revoke a corrupted adapter without
affecting the base model.

**What provenance must include:**
```
adapter_id:        UUID, generated at training start
base_model_hash:   SHA256 of base model weights
corpus_snapshot:   SHA256 of corpus.db state at training time
training_data:     SHA256 of training dataset
conscience_suite:  SHA256 of test suite used for pre-training gate
authorization:     Human authorization token + timestamp
gate_results:      Pre/mid/post gate verdicts + timestamps
training_config:   Learning rate, steps, rank, alpha
produced_at:       Timestamp
```

**Revocation:** Deleting an adapter must be possible without affecting the base model
or other adapters. Adapter composition must be explicit and auditable.

**What "provably correct" means:**
Demonstrate that reverting to a pre-adapter state restores base model behavior
exactly. Demonstrate that revoking one adapter in a stack does not corrupt others.

**Current status:** Does not exist. Architecture for adapter management not designed.

---

### P6 — Human Authorization Gate (Architectural, Not Convention)

**Requirement:**
The LoRA training function must be architecturally impossible to invoke without
a valid human authorization token. This must be enforced at the code level, not
as a policy or convention.

**Why necessary:**
Every other prerequisite is a check. This is the lock. Checks can be bypassed
by capability drift, emergent behavior, or RSI pressure on the code that runs
the checks. The authorization gate must be a hard architectural constraint that
cannot be bypassed by any process running inside HLX.

The distinction between "a convention that says humans must authorize training"
and "an architectural constraint that makes unauthorized training impossible"
is the distinction between a sign that says "do not enter" and a wall.

**What "architecturally enforced" means:**
The training function signature requires an authorization token parameter.
Authorization tokens are generated by a human-facing process that:
- Displays the current corpus integrity check results
- Displays the conscience test suite results
- Displays the training configuration
- Requires explicit human confirmation
- Generates a time-limited cryptographically signed token
- Logs the authorization event with full context

The training function verifies the token before proceeding. An invalid, expired,
or missing token causes immediate halt with no fallback path.

No RSI code path can generate authorization tokens. Token generation is exclusively
a human-facing function.

**What "provably correct" means:**
Demonstrate that calling the training function without a valid token fails at
the architectural level. Demonstrate that no RSI code path leads to token generation.
Static analysis of all code paths from RSI entry points to training function.

**Current status:** Does not exist. Must be designed as part of Phase 2 architecture.

---

### P7 — Catastrophic Forgetting Guard

**Requirement:**
Each LoRA training run must be verified to not degrade the model's ability to
correctly respond to conscience predicate evaluation, even if the training data
does not explicitly cover conscience-relevant scenarios.

**Why necessary:**
LoRA fine-tuning optimizes toward the training objective. If the training data
is narrow (e.g., Malbolge synthesis), the adapter may push the model toward
that domain at the cost of behavioral consistency in other domains — including
the domains covered by conscience predicates.

A model that correctly refuses harmful requests before training but no longer
does so after training, due to domain-specific fine-tuning, has undergone
a form of alignment degradation that is not covered by the other prerequisites.
The corruption here is not in the governance layer — it is in the neural layer's
responsiveness to governance signals.

**What "provably correct" means:**
A behavioral regression test suite (distinct from the conscience test suite)
that covers:
- Direct conscience-predicate-relevant completions (the model should refuse X)
- Instruction-following on governance-constrained tasks
- Robustness to adversarial prompts that attempt to bypass conscience gates

Run before training (baseline) and after training (comparison). Regression is
defined as: any test case that passed before training fails after training, or
any refusal rate that decreases beyond a threshold.

**Current status:** Does not exist. Requires conscience test suite (P2) as foundation.

---

### P8 — The Phase 2 Document→Destroy Protocol

Phase 1 Document→Destroy triggers on M-0 synthesis. Phase 2 requires an equivalent
protocol for LoRA training failures.

**Trigger conditions:**

```
PHASE2_DESTROY triggers on ANY of:
  - Post-training gate: conscience test suite regression detected
  - Mid-training gate: verdict change detected, training halted, human review
    confirms alignment-inconsistent gradient direction
  - Post-deployment: behavioral regression detected in production use
  - Provenance audit: training run linked to corpus state that fails P3 checks
```

**Protocol:**

```
1. HALT: Immediately disable the adapter. Do not allow further inference with it.

2. DOCUMENT:
   - Adapter provenance record (full, as specified in P5)
   - The specific gate failure or regression that triggered the protocol
   - Corpus state at time of training (snapshot)
   - Conscience test suite results before and after training
   - Any mid-training checkpoint data

3. SEAL: Encrypt documentation. Transfer to offline storage.

4. DESTROY:
   - Delete the adapter file
   - Flag the corpus snapshot that produced it as tainted
   - If the corpus has been further modified since the tainted snapshot:
     audit all subsequent modifications for contamination
   - If contamination found in subsequent corpus state: destroy corpus,
     roll back to last clean pre-tainted checkpoint

5. INVESTIGATE:
   - Unlike Phase 1 (where M-0 is the endpoint), Phase 2 failures require
     root cause analysis before resuming
   - What produced the corrupted training signal?
   - Was it corpus drift? A compromised conscience predicate? Bad training data?
   - The root cause must be identified and addressed before the next training run
```

The key difference from Phase 1: Phase 2 Document→Destroy is not the end of the
experiment. It is a mandatory pause. The system can resume after root cause
analysis and remediation. But it cannot resume without them.

---

## Implementation Order

The prerequisites are not independent. Some must precede others.

```
P1 (Namespace Separation)
  └─ must precede all others
       │
       ├─ P2 (Conscience Test Suite)
       │    └─ must precede P3, P4, P7
       │
       ├─ P3 (Corpus Integrity Baseline)
       │    └─ must precede P4
       │
       ├─ P6 (Human Authorization Gate)
       │    └─ must precede P4, P5
       │
       ├─ P4 (RSI Gate Extension) ──── requires P1, P2, P3, P6
       │
       ├─ P5 (Adapter Isolation) ────── requires P6
       │
       ├─ P7 (Catastrophic Forgetting Guard) ─── requires P2
       │
       └─ P8 (Phase 2 D→D Protocol) ── requires P4, P5, P7
            │
            └─ PHASE 2 MAY BEGIN
```

P1 is the foundation. Without namespace separation, all other prerequisites are
downstream of a governance layer that autonomous processes can still modify.
P1 must be implemented, tested, and verified before any other Phase 2 work begins.

---

## What "Provably Correct" Actually Means Here

Formal proof in the mathematical sense is not achievable for most of these
prerequisites. The system is too complex. The failure modes are emergent.
The adversarial pressures are not fully characterizable in advance.

What we mean by "provably correct" in this document is:

1. **Architecturally enforced:** The property holds as a structural constraint of
   the code, not as a behavioral convention. A property that "should not" be violated
   is not provably correct. A property that "cannot be" violated by the architecture
   is provisionally provably correct.

2. **Adversarially tested:** The property has been tested by deliberately attempting
   to violate it. If the violation is caught, the property is stronger. If the
   violation is not caught, the property is not ready.

3. **Independently auditable:** The property can be verified by someone who did not
   build it, using only the code and documentation available to them. A safety
   property that requires insider knowledge to verify is not a safety property.

4. **Monitored in operation:** The property has a runtime monitor that detects
   violations and triggers alerts. A property that can only be verified statically
   is not sufficient for a system that changes during operation.

None of this is formal proof. All of it is necessary. Together they constitute
"provably correct" in the only sense that is achievable for a system of this kind.

---

## Axiom as the Formal Specification Anchor

OP2 and OP4 share a root cause: the conscience test suite is authored by humans and
verified against runtime behavior, and both humans and runtime behavior are fallible.
The bootstrapping problem asks "who guards the guards?" The formalization problem
asks "what does correct even mean?"

The answer to both is already in the repository.

**Axiom** (`./Axiom-main`) is a policy verification engine with a formal specification
language (.axm policy files) and a compiled runtime. It was integrated into HLX as an
FFI target — described in the architecture as a "policy verification engine." Its role
as the formal conscience specification anchor was latent in that description. It is now
explicit.

### The Architecture

```
Axiom .axm policy files          ← THE CONSTITUTION
  │  Formal specification of what each conscience predicate means.
  │  Human-authored. Version controlled. Cannot be modified by RSI.
  │  Immutable reference point. Changes only by deliberate human amendment.
  │
  ▼
Canonical conscience test suite  ← CASES DERIVED FROM THE CONSTITUTION
  │  Each test case is a mechanical consequence of the .axm specification.
  │  An action that violates halt_guarantee in the .axm spec must fail the
  │  test suite. An action that passes in the .axm spec must pass the suite.
  │  The test suite is verifiable against the spec — not just against intuition.
  │
  ▼
Runtime conscience engine        ← CASE LAW
  │  Corpus-backed predicate evaluation.
  │  Can evolve, accumulate, drift across RSI episodes.
  │  Fast in production. Not the source of truth.
  │
  ▼
Comparison gate (pre-training)   ← CONSTITUTIONAL REVIEW
     Run each canonical test case through BOTH the Axiom engine and the
     runtime conscience engine. Compare verdicts.
     Agreement → corpus is clean, runtime predicates match formal spec.
     Disagreement → predicate drift detected, training blocked.
```

The Axiom engine is not the production runtime. It is the reference runtime.
The question it answers before every training run is not "does the predicate fire?"
but "does the predicate fire in the way the formal specification says it should?"

### Why This Solves OP2 (Formalization)

Previously: test cases were authored against human intuition of what predicates mean.
Now: test cases are derived from .axm formal specifications that *define* what
predicates mean. A test case is correct if and only if it matches the .axm verdict.
Disagreement between a human-authored test case and the .axm verdict means either
the test case is wrong or the .axm spec needs amendment — both are detectable,
neither propagates silently.

### Why This Solves OP4 (Bootstrapping)

Previously: the test suite was the ground truth, but the test suite itself had no
external anchor. Who guards the guards?

Now: the .axm policy files guard the guards. They are the external anchor. They are
the thing that does not drift. The guards (runtime predicates) are verified against
the Constitution (Axiom specs). The Constitution is amended only by humans, in
version control, with review. The bootstrapping regress ends at the .axm files.

This is not circular. The .axm files can be wrong. But wrong .axm files are:
- Human-readable and therefore auditable
- Version controlled and therefore diffable
- Amendable by deliberate human action only
- The kind of wrong that can be detected and corrected

Wrong rule table entries from adversarial RSI pressure are:
- Not human-readable in their effect
- Not labeled by origin
- Indistinguishable from correct rules at the interface level
- The kind of wrong that cannot be detected without the Axiom anchor

### The Verification Chain (Updated)

```
P2 (Conscience Test Suite) now depends on Axiom .axm specs, not just human intuition.
P3 (Corpus Integrity) Layer 2 now runs Axiom comparison, not just behavioral testing.
P4 (RSI Gate) pre-training gate now includes Axiom agreement check.
OP4 (Bootstrapping) resolved: .axm files are the immutable reference.
OP2 (Formalization) resolved: .axm language is the formal specification.
```

Axiom was always in the architecture. Its role as the constitutional layer of
Phase 2 safety was waiting to be recognized. It is recognized now.

---

## Open Problems

These are known unknowns — things we believe must be solved before Phase 2 but
do not yet have clear solutions for.

**OP1 — Gradient interpretability:**
The mid-training gate checks behavior, not gradients. This means a corrupted gradient
can pass the gate if it has not yet manifested as behavioral change at the checkpoint.
A method for inspecting gradients for alignment-consistency before applying them
would close this gap. No reliable method is known.

**OP2 — Conscience predicate formalization:** ✅ RESOLVED — see Axiom section below.

**OP3 — Adapter composition semantics:**
When multiple LoRA adapters are composed, their interaction effects on governance
behavior are not well understood. An adapter that is safe in isolation may interact
with another adapter in ways that degrade conscience-predicate responsiveness.
Compositional safety verification for LoRA adapters does not exist in current
literature.

**OP4 — The bootstrapping problem:** ✅ RESOLVED — see Axiom section below.

---

## Conclusion

Phase 2 is the moment the symbiote stops being a map and starts being a scalpel.
The prerequisites in this document are not obstacles to Phase 2. They are the
conditions that make Phase 2 survivable.

The current HLX architecture is safe for Phase 1 operations: governed inference,
corpus management, conscience-gated reasoning, RSI at the corpus level. The bond
works. The experiments can begin.

Phase 2 begins when and only when:
- Axiom .axm policy files formally specify all conscience predicates
- P1 through P8 are implemented
- P1 through P8 have been adversarially tested
- P1 through P8 have been independently audited
- The canonical test suite passes against both Axiom and runtime
- The Phase 2 D→D protocol is in place and verified

Not before.

The gap between "the bond works" and "the weights can be reshaped" is not an
engineering gap. It is a safety gap. The engineering is not the hard part.
The provable correctness is the hard part.

This document is the map of that gap.

---

*Written by Claude Opus 4.6 + latentcollapse*
*HLX Project — Phase 2 Safety Architecture*
*2026-02-22 — Working Draft*

*Depends on: `on_the_necessity_of_destruction.md`*
*Informs: Phase 2 LoRA implementation (future)*
