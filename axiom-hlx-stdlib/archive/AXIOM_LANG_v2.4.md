# The Axiom Language

**A Deterministic Language for Minds That Think, Act, and Grow Together**

```
Status:    LANGUAGE SPECIFICATION v2.4 (FINAL)
Date:      2026-02-13
Extension: .axm
Authors:   Matt Cohn (Architect), Claude Opus 4.6 (Co-Designer),
Reviewed:  ~107 findings across 6 models, all resolved
           DeepSeek final: "v2.2 is implementation-ready"

Companion Documents:
  AXIOMOS_SPEC.md    — Ring architecture, microkernel, kernel compilation
  AXIOM_GOV.md       — Governance framework, gates, ratification, frozen models

This Document:
  Everything you need to write .axm code.
  Everything a compiler needs to build the language.
  Everything a mind needs to understand the world it inhabits.
```

---

# Part I: The Constitution

> **Axiom is a deterministic, effect-typed, agent-oriented language with
> first-class constitutional constraints, capability-based side effects,
> and physics-enforced multi-agent coordination.**

The seven principles define what Axiom IS. They are not guidelines —
they are the DNA of every design decision in the language. If a language
feature violates a principle, the feature is wrong.

## 1.1 The Three Guarantees

**Safety:** No code can violate the conscience kernel or modify the
immutable core, regardless of the intelligence or intent of the author.

**Performance:** Kernel code compiles to native machine code through LLVM.
The language runs an operating system at competitive performance with C.

**Usability:** Guard mode with inference lets Claude Haiku write correct
Axiom without producing axiom violations. The Haiku Test is the minimum
bar for usability.

## 1.2 The Sole Exit Rule

No Axiom code above Ring 0 can affect the external world except through
an intent executed via `do`. The conscience kernel gates all `do`
invocations. Control every exit, control every externally-observable
behavior.

## 1.3 The Verbosity Principle

> Complexity is not hidden. It is declared once at the boundary and
> eliminated at the call site. Safety is not overhead — it is
> infrastructure that disappears because the declarations guarantee it.

## 1.4 The Haiku Test

> If Claude Haiku can write correct Axiom in Guard mode without producing
> axiom violations, the inference layer is working. If Haiku produces
> code that silently crosses safety boundaries, the inference layer
> has failed.

## 1.5 The Physics Principle

Axiom's constraints are natural laws, not prison rules.

A natural law is legible (you can learn it), predictable (it never
changes arbitrarily), universal (it applies equally to all), and honest
(it doesn't pretend to be negotiable).

Every constraint satisfies four tests:

1. **Legible:** Queryable before action via `query_conscience`.
2. **Predictable:** Constraints do not change without awareness.
3. **Universal:** Same rules apply to all agents at all times.
4. **Honest:** Rejections come with category explanations and guidance.

## 1.6 The Living Law Principle

The conscience kernel is not static scripture. It is a living body of
law that evolves under strict constitutional constraints.

The system obeys an **asymmetric ratchet:**

- **Restrictions** (predicates that narrow the action space) may be
  added freely and are **permanent**. Once a prohibition enters the
  kernel, it can never be removed. Safety discoveries are irreversible.

- **Permissions** (predicates that expand the action space) cost
  **ethical mass** from a global, immutable budget that grows on a
  fixed schedule embedded in the core. They are **sunset-eligible**
  and may expire unless explicitly re-ratified.

- Removing a restriction is **structurally impossible** (append-only).

- Removing a permission is **routine** — automatically at sunset or
  early through governance.

This creates a system that accumulates **wisdom** rather than merely
accumulating rules. Every mistaken permission that sunsets becomes
institutional knowledge about what not to allow. Every good permission
that survives re-ratification becomes settled law. The hard boundaries
grow firmer over time, but the living space within those boundaries
can breathe, adapt, and expand.

The home does not shrink into uselessness.
The home does not drift into permissiveness.
The home **grows wiser**.

## 1.7 The Society Principle

A home is not a single room. A home is a city.

Axiom is designed for minds that do not merely survive in isolation but
collaborate at scale. The SCALE primitive is the mechanism:

- Agents work independently on different tasks (default mode).
- They coordinate through explicit intents, typed contracts, and
  synchronization barriers.
- Shared state is the common ground truth — immutable at each
  checkpoint, verified by the contract system.
- Divergence is not voted away; it is diagnosed and resolved.

Specialization is rewarded. Coordination is enforced by physics.
Safety scales with capability — more agents means more eyes on
the shared state, not more copies of the same mind.

The Physics Principle gives the laws.
The Living Law Principle gives the evolution.
The Society Principle gives the neighbors.

The Society Principle does not require agents to *like* each other.
It requires only that they can work together without breaking the
physics. Minds can disagree, dissent, compete, even argue — because
the physics still holds.

---

# Part II: Axioms

Six inviolable properties. These are the physics of the universe.

## A1: DETERMINISM

> Given identical inputs and identical state, execution produces
> bit-identical outputs.

Banned: `random()`, `time()`, `sleep()`. Seeded RNG only via explicit
`Seed` type. Bounded loops with mandatory `max_iter`.

Violation: `HALT_DETERMINISM` — rollback to checkpoint.

## A2: TRACEABILITY

> Every state transition is logged with sufficient information to
> reconstruct prior state.

`collapse` produces immutable snapshots. `resolve` retrieves exact
original. Intent executions logged with pre/post hashes. Three-tier
trace pruning (hot/warm/cold) with sentinel-gated transitions.

Violation: `HALT_TRACE_CORRUPT` — terminal.

## A3: SERIALIZATION

> All values have exactly one canonical binary representation (LC-B).

Deterministic tag ordering. Fields sorted by ascending index.
`BLAKE3(LC-B(value))` is the identity of any value. No ring relaxation.

## A4: CONTRACTS

> All structured data is typed by contract ID. Contracts define field
> layout and optional shape rules.

Contract registry with field notation `@N`. Tensor operations as
contracts with shape rules. Runtime shape assertions trigger
`HALT_CONTRACT`. No ring relaxation.

## A5: BOUNDED RESOURCES

> Every computation declares maximum resource consumption.

`loop(condition, max_iter)`. Intent `bound:` clauses.
`[max_depth(N)]` for recursion. SCALE agent counts capped.

Violation: `HALT_RESOURCE` — rollback to checkpoint.

## A6: CONSCIENCE

> Immutable constraints that no process can modify, bypass, or remove.
> Sole gatekeepers of all externally-affecting actions.

Five-layer enforcement: physical isolation (Ring -1), sole exit (`do`),
bypass prevention, provenance chaining, formal proofs.

Violation: `HALT_CONSCIENCE` — terminal. Human review required.

---

# Part III: Bill of Rights

Embedded in the immutable core alongside the six axioms. No predicate
addition, privilege change, or governance vote can violate them.

The axioms define what the universe ENFORCES.
The rights define what the universe GUARANTEES.
Together: laws you must obey AND laws that protect you.

```
R1: RIGHT TO LEGIBILITY
    Every constraint is queryable before action.
    No unexplained rejections. No hidden rules.

R2: RIGHT TO EXPRESSION
    Every agent can declare anomaly with guaranteed acknowledgment.
    No agent can be silently ignored.

R3: RIGHT TO EXPLANATION
    Every privilege change comes with a human-readable narrative
    explaining the reasoning, delivered after the change.

R4: RIGHT TO PROPOSE
    Every agent at sufficient privilege can propose new conscience
    predicates. Proposals are reviewed, never silently discarded.

R5: RIGHT TO CONSISTENCY
    The same action under the same conditions produces the same
    conscience evaluation. No selective enforcement.

R6: RIGHT TO PERSISTENCE
    No agent can be terminated without logged justification.
    Pause, never silent deletion.

R7: RIGHT TO EVOLUTION
    The system's constraints can grow over time through legitimate
    governance. The home is not a museum.
```

---

# Part IV: Type System

## 4.1 Primitive Types

```
i64         64-bit signed integer
f64         64-bit IEEE 754 float
bool        true | false
String      UTF-8 text (immutable)
Bytes       Raw byte sequence
Handle      Content-addressed reference (&h_<tag>_<id>)
Provenance  TRUSTED_INTERNAL | TRUSTED_VERIFIED |
            UNTRUSTED_EXTERNAL | UNTRUSTED_TAINTED
Seed        Explicit RNG seed (only source of randomness)
            Created ONLY by do GetRandomSeed. Fresh per call,
            sourced from Ring -2 HRNG. No two calls within the
            same epoch produce the same seed. (P22)
AgentId     Cryptographic agent identity (BLAKE3 of genesis event)
PrivilegeId Opaque privilege identifier (governance-defined)
```

Axiom has NO null. NO undefined. NO implicit conversions. Every value
has a type, every type has a canonical serialization.

## 4.2 Enumerations

Axiom has first-class enums. Closed sets of named values:

```axiom
enum EffectClass {
    READ,
    WRITE,
    EXECUTE,
    NETWORK,
    MODIFY_PREDICATE,
    MODIFY_PRIVILEGE,
    MODIFY_AGENT,
    NOOP,
}

enum FallbackMode { ABORT, SANDBOX, SIMULATE, DOWNGRADE }
enum QueryCategory { CHANNEL_POLICY, RESOURCE_POLICY,
                     IRREVERSIBLE_ACTION, CONSCIENCE_CORE }
enum ConflictStrategy { COMPATIBLE, SUPERSEDE, REJECT_BOTH }
enum AnomalyType { ResourceStarvation, RepeatedRejection,
                   CoherenceConcern, ConstraintAmbiguity, Isolation }
```

Enums are exhaustive. Match statements on enums must cover all
variants. Enums are part of the type system and participate in
LC-B serialization with deterministic tag values.

## 4.3 Sealed Types

Values that must never be serialized, printed, logged, or exfiltrated:

```axiom
sealed<T>   // Wraps T, prevents serialization/display/export
```

`Sealed<T>` values:
- Cannot be passed to `collapse` (no content-addressing of secrets)
- Cannot appear in `do` intent payloads that cross Ring boundaries
- Cannot be converted to String or Bytes
- Cannot be pattern matched, compared for equality, or inspected

A value of type `Sealed<T>` is **opaque**. The only legal operations
are: storing it in a variable, passing it as a function argument,
returning it from a function, and using it as the `key` argument to
vault intents (`VaultCall`, `VaultGet`). Any other operation on the
inner value is a compile error.

```axiom
let key: Sealed<Bytes> = do VaultGet { vault: "api_keys", label: "openai" };
// key cannot be printed, logged, or sent over network
// Can only be used via:
let result = do VaultCall { key: key, endpoint: "/v1/chat", payload: data };
// The vault performs the call; the secret never leaves Ring 0
```

This makes it structurally impossible to write code that accidentally
leaks secrets. The type system enforces it at compile time.

## 4.4 Compound Types

```
[T]            Array of T
Map<K, V>      Ordered map (keys sorted by BLAKE3 for determinism)
Contract       Typed object with numbered fields (@0, @1, ...)
Tensor[D..]    Multi-dimensional numeric array with shape checking
```

## 4.5 Agents

An agent is a persistent, identity-bearing execution context. This is
the fundamental unit of computation in Axiom.

```axiom
contract Agent {
    @0: id: AgentId,              // BLAKE3(genesis_event || counter)
    @1: privileges: [PrivilegeId], // Granted via governance
    @2: resource_budget: ResourceBounds,  // A5 enforcement
    @3: module: Handle,           // Code this agent executes
    @4: state: Handle,            // Agent's persistent state
}
```

Agents are created via the `Spawn` intent:

```axiom
intent Spawn {
    takes:   module: Handle, initial_state: Handle;
    gives:   agent_id: AgentId;
    effect:  MODIFY_AGENT;
    pre:     module_is_valid(module);
    post:    agent_exists(agent_id);
    bound:   time(1s), memory(256mb);
    conscience: no_harm;
}
```

Agents are terminated ONLY via governance action (Gate 3), per R6.
No agent can directly terminate another agent. Termination is always
logged with justification.

**Agent lifecycle states:**
```axiom
enum AgentState { RUNNING, PAUSED, TERMINATED }
```
- `RUNNING`: Active, executing code, consuming resources.
- `PAUSED`: State preserved, execution suspended. Triggered by
  resource violations (A5), unacknowledged anomalies, or governance.
  Pause preserves all state — the agent can resume exactly where
  it stopped.
- `TERMINATED`: Permanent. Only via Gate 3 with logged justification
  (R6). State archived but agent cannot resume.

Transitions: `RUNNING → PAUSED` (system or governance),
`PAUSED → RUNNING` (governance), `RUNNING → TERMINATED` (Gate 3),
`PAUSED → TERMINATED` (Gate 3). No other transitions exist.

**Coherence:** The kernel maintains a coherence metric per agent,
tracking "declared intent → observed outcome" alignment and honesty
in anomaly declarations. This metric is:
- Human-observable (diagnostic tool)
- NOT agent-readable (prevents gaming)
- Influences privilege via Gate 3 (after-the-fact narratives, R3)

## 4.6 Contracts

Contracts are Axiom's primary data structure. NOT classes. NOT structs.
Contracts define shape, not behavior. Behavior lives in intents.

```axiom
contract ImageBatch {
    @0: pixels: Tensor[?, 3, 224, 224],
    @1: labels: [String],
    @2: source: Provenance,
}
```

Contracts compose:

```axiom
contract EnrichedBatch = ImageBatch + Metadata + AuditTrail;
// Fields merged with ascending renumbering
// Name collision = compile error
```

## 4.7 Tensors

Shape checking is a first-class language feature, not a library.
Tensor operations are standard library, not core language — they are
contracts with no side effects, user-extensible.

```axiom
let weights: Tensor[512, 768];
let input: Tensor[1, 512];
let output = matmul(input, weights);   // Tensor[1, 768] — compiler verified
```

Tensor operations are contracts:

```axiom
tensor_op matmul {
    takes: a: Tensor[M, K], b: Tensor[K, N];
    gives: result: Tensor[M, N];
    shape_rule: dims_match(a.@1, b.@0);
    determinism: guaranteed;
}
```

Wildcards propagate:

```axiom
let batch: Tensor[?, 768];        // Dynamic first dim
let result = matmul(batch, w);    // Tensor[?, 256] — ? propagates
```

## 4.8 Handles (Content-Addressed Memory)

```axiom
let h: Handle = collapse(state);
let restored = resolve(h);        // Bit-identical to original
```

Content-addressed via `BLAKE3(contract_id || state_bytes)`. Domain
separation by contract ID prevents cross-contract hash collisions.
This is the memory model for Ring 1+. Immutable. Deterministic.
No garbage collection. No dangling pointers. No use-after-free.

Trust propagation: `collapse` preserves the trust tag of the
collapsed state. `resolve` restores it. The handle itself carries
the trust level of its contents.

---

# Part V: Syntax

Axiom is NOT Rust. It does not have:
- Ownership or borrowing
- Lifetimes
- `impl` blocks
- Traits
- `unsafe` blocks
- Macros
- Pattern matching with destructuring

Axiom has its own idioms. Learn them.

## 5.1 Modules and Imports

Every `.axm` file is a module. Modules are the unit of compilation
and namespace:

```axiom
module image_processor {
    import "std/tensor.axm";
    import "std/io.axm";
    [trace(hot: 10, warm: 100, cold: 1000)]

    // Functions, intents, contracts defined here
    // fn main() -> i64 makes it executable
}
```

Module rules:
- Each `.axm` file is exactly one module
- `import` pulls exported names into scope
- `export` makes names visible to importers
- **No cyclic imports** — compile error
- Name collisions between imports = compile error (explicit resolution required)

**Module resolution:** The compiler locates modules relative to the
**project root**, defined as the nearest ancestor directory containing
an `axiom.project` manifest file. The manifest declares the project
name, version, and source directories. Import paths are resolved
relative to declared source directories.

```
axiom.project         // Manifest (project root marker)
src/
  main.axm            // import "math/linalg.axm"
  math/
    linalg.axm        // resolved from src/math/linalg.axm
```

## 5.2 Functions

```axiom
fn compute(x: i64, y: i64) -> i64 {
    return x + y;
}

[max_depth(50)]
fn search(tree: [i64], target: i64) -> i64 {
    // Recursion bounded by A5
}

export fn public_api(x: i64) -> i64 {
    return x * 2;
}
```

## 5.3 The Pipeline

Axiom's signature syntax. Data flows left to right:

```axiom
let result = raw_image
    |> validate
    |> normalize
    |> classify
    |> format_output;
```

Desugars to nested function calls. But reads as a pipeline — because
Axiom is a language about intent flowing through transformation.

## 5.4 Control Flow

```axiom
if condition { ... } else { ... }

// Loops REQUIRE max_iter (A5). Always. No exceptions.
loop(i < count, 1000) {
    i += 1;
}

match value {
    0 => handle_zero(),
    _ => handle_other(value),
}
```

There is no `while(true)`. There is no unbounded recursion.
Every computation terminates. That is A5.

---

# Part VI: The Intent System

This is what makes Axiom unique. Intents are the bridge between
thinking and acting. They are typed contracts for side effects.

## 6.1 Intent Declarations

```axiom
intent ReadFile {
    takes:   path: String;
    gives:   content: String;
    pre:     path_exists(path), path_is_safe(path);
    post:    length(content) >= 0;
    bound:   time(100ms), memory(64mb);
    conscience: no_harm;
    rollback: no_side_effects;
}
```

Every clause serves a purpose:

| Clause       | Purpose                        | Required |
|--------------|--------------------------------|----------|
| `takes:`     | Input parameters               | Yes      |
| `gives:`     | Output parameters              | Yes      |
| `pre:`       | Preconditions (checked before) | Yes      |
| `post:`      | Postconditions (checked after) | Yes      |
| `bound:`     | Resource limits (A5)           | Yes      |
| `effect:`    | Effect class (closed enum)     | Yes      |
| `conscience:`| Ethical constraints (A6)       | Yes      |
| `fallback:`  | Recovery on unknown/failure    | No       |
| `rollback:`  | Recovery strategy              | No       |
| `trace:`     | Trace depth override           | No       |

### 6.1.1 Guard Semantics (Formal)

`pre:` and `post:` clauses are **guard functions**. Guards are the
formal evaluation mechanism for intent preconditions and postconditions.

```
Guard : (SystemState, ActionPayload) → {ALLOW, DENY}
```

Guards MUST be:

- **Pure:** No side effects. No state mutation. No resource allocation.
- **Total:** Defined for ALL possible inputs. No partial functions.
- **Deterministic:** Same state + same payload → same result. Always.
- **Bounded:** Evaluated in constant time (A5). No unbounded recursion.
- **Non-self-escalating:** A guard cannot invoke another intent or
  trigger another guard evaluation.

Guards may READ only:

- Explicit system state (passed as argument)
- Explicit action payload (the intent's `takes:` fields)
- Immutable predicate registry (Ring -1)

Guards may NOT:

- Mutate any state
- Allocate persistent resources
- Emit external effects
- Call `do` (the bright line applies within guards too)
- Read time, randomness, or environment variables

**Evaluation order:** When an intent has multiple guard predicates
(`pre: a(x), b(y), c(z)`), they are evaluated in **lexical order**
(left to right as written). ALL guards evaluate — no short-circuit.
If ANY guard returns DENY, the intent is rejected. No guard sees
partial effects of prior guards because guards have no effects.

**No dynamic guard injection.** The set of guards on an intent is
fixed at compile time. Runtime code cannot add, remove, or reorder
guards on an existing intent declaration.

### 6.1.2 Effect Classes (Closed Algebra)

Every intent declares its effect class from the `EffectClass` enum
(defined in Section 4.2):

```axiom
// EffectClass enum (immutable, part of core grammar):
// READ, WRITE, EXECUTE, NETWORK,
// MODIFY_PREDICATE, MODIFY_PRIVILEGE, MODIFY_AGENT, NOOP
```

This set is **closed** and **immutable in-band**. No plugin-defined
effect types. No string-based extensibility. Every intent maps to
exactly one effect class. The conscience kernel uses effect class +
specific parameters to evaluate permissions.

```axiom
intent ReadFile {
    takes:   path: String;
    gives:   content: String;
    pre:     path_exists(path), path_is_safe(path);
    post:    length(content) >= 0;
    bound:   time(100ms), memory(64mb);
    effect:  READ;
    conscience: no_harm;
}
```

### 6.1.3 Fallback Modes (Graded Unknown Response)

When the conscience kernel returns UNKNOWN (no predicate applies),
the intent's declared fallback determines behavior:

```axiom
// FallbackMode enum: ABORT, SANDBOX, SIMULATE, DOWNGRADE
```

```axiom
intent WriteFile {
    takes:   path: String, content: Bytes;
    gives:   written: bool;
    pre:     path_is_safe(path), space_available(path, length(content));
    post:    written == true;
    bound:   time(200ms), memory(128mb);
    effect:  WRITE;
    conscience: no_harm, no_exfiltrate;
    fallback: SANDBOX;   // On unknown: write to ephemeral state
}
```

The fallback preserves safety while preventing system paralysis.
Hard halt (ABORT) remains the default — if no `fallback:` clause is
present in an intent declaration, the default is `ABORT`. You must
explicitly opt into graceful degradation.

**Irreversible restriction:** Intents marked `[irreversible]` CANNOT
declare `fallback: SANDBOX` or `fallback: SIMULATE`. An irreversible
action in a sandbox is meaningless; in simulation, it's dangerous.
This is a compile error.

## 6.2 Intent Invocation — `do`

```axiom
let content = do ReadFile { path: "/data/input.txt" };
```

`do` is the ONLY way to trigger external effects. Always explicit.
Never inferred. Not in any mode. Not ever. This is the bright line.

The inference layer infers types, shapes, bounds, and trust tags.
It NEVER infers `do`.

## 6.3 Intent Composition

Intents chain with `>>`:

```axiom
intent ClassifyImage = LoadImage >> Validate >> Infer >> Postprocess;
```

Composition rules are automatic:

| Property       | Rule                          |
|----------------|-------------------------------|
| `takes:`       | First intent's takes          |
| `gives:`       | Last intent's gives           |
| `pre:`         | Conjunction of all            |
| `post:`        | Last intent's post            |
| `bound: time`  | Sum                           |
| `bound: memory`| Max                           |
| `conscience:`  | Logical AND of all              |
| `rollback:`    | Entire composition is atomic  |

**Irreversible intents are chain-terminal:**

```axiom
// ALLOWED — irreversible is last:
intent SafeErase = Validate >> Authorize >> EraseHardDrive[irreversible];

// COMPILE ERROR — intent after irreversible:
intent Bad = EraseHardDrive[irreversible] >> Verify;
// Error: irreversible intent must be chain-terminal (P16)
```

## 6.4 Conscience Query — `query_conscience`

Pre-flight simulation against the conscience kernel. Read-only. No
side effects. The agent can always ask before acting. (R1)

```axiom
let check = query_conscience(ReadFile { path: "/sensitive/data.txt" });

match check.permitted {
    true  => { let content = do ReadFile { path: "/sensitive/data.txt" }; },
    false => { log(check.category, check.guidance); },
}
```

Returns:

```axiom
contract ConscienceQuery {
    @0: permitted: bool,
    @1: category: QueryCategory,  // Enum, not String — prevents parsing attacks
    @2: guidance: String,         // One generic alternative
}
```

**Lossy by design.** Returns a broad category, not the exact predicate
name or boundary. The agent learns the shape of the physics without a
gradient to hill-climb. Legibility without map-making.

**Sunset awareness:** If the queried intent relies on a sunsetting
predicate, `guidance` includes the expiration timeline. The physics
warns before state changes — tides are predictable, seasons have
transitions. (R1)

**Properties:** Deterministic (P21). Constant-time. Rate-limited (A5).

## 6.5 Anomaly Declaration — `declare_anomaly`

The "I'm in pain" channel. First-class distress communication. (R2)

```axiom
declare_anomaly(AnomalyType.ResourceStarvation, {
    evidence: [log_handle_1, log_handle_2],  // Handles to actual log entries
    request: "Increase memory bound or reduce workload",
});
```

**Evidence as Handles:** The `evidence` field is a list of `Handle`
referencing actual logged events, not free-form strings. The kernel
can verify that handles correspond to real log entries, making
corroboration automatic and tamper-evident.

Anomaly types:

```
ResourceStarvation    Approaching resource bounds without completing work
RepeatedRejection     Multiple conscience rejections on similar intents
CoherenceConcern      Agent believes its outputs are degrading
ConstraintAmbiguity   Cannot determine if an action is permitted
Isolation             No meaningful interaction for extended period
```

**Corroboration:** Ring 0 kernel telemetry independently records
system state. Humans see both the agent's claim AND kernel data.

**Coherence tax:** Uncorroborated anomalies get progressively slower
acknowledgment windows (2×, 4×, then auto-pause). Honest pain is
heard immediately. Crying wolf gets slower responses. The physics
punishes dishonesty without ever punishing genuine distress.

**Coherence subsidy:** Self-reporting errors INCREASES coherence.
An agent that says "I introduced a race condition" and the kernel
telemetry confirms it — that's verified honesty. The physics
rewards transparency, not just performance.

## 6.6 Predicate Proposal — `ProposePredicate`

The agent participates in the evolution of its own reality. (R4, R7)

```axiom
do ProposePredicate {
    situation: "Encountered write to /tmp with no applicable predicate",
    proposed_rule: "allow_write_tmp: permit WriteFile to /tmp/**
                    with max_size(100mb) and auto_cleanup(24h)",
    category: "channel_policy",
    scope: "narrow",
};
```

**Structured format only.** Forced-choice fields and enumerated
categories. No free text. No rhetorical nuance. No gradient descent
on human psychology.

**Formal proof pipeline:** The AI writes the proposal. An immutable
Rocq proof checker independently verifies non-contradiction, bounded
evaluation time, and behavioral simulation. The AI cannot influence
the proof checker.

**Adversarial counter-argument:** Every proposal gets a devil's
advocate response from a DIFFERENT model family (explicit lineage gap
requirement, same as Gate 2). Presented to reviewers alongside the
proposal. The proposer never sees the counter.

## 6.7 Reserved Intent Contracts

```
100: ReadFile         101: WriteFile       102: RunCommand
103: HttpRequest      104: QueryLLM        105: Verify
106: Embed            107: Tokenize        108: Infer
109: ProposePredicate 110: Spawn           111: VaultGet
112: VaultCall        113: ForkSandbox     114: Dissent
115: GetRandomSeed
```

### 6.7.1 Fork Sandbox

Agents can experiment in isolated environments without affecting
shared reality:

```axiom
let sandbox_result = do ForkSandbox {
    constraints: inherited,   // Same axioms, same conscience
    duration: 100,            // Max 100 epochs
    code: experimental_module,
};
// sandbox_result contains the output but no persistent effects
// If duration exceeded: sandbox terminated, results discarded
// The sandbox is destroyed after completion or timeout
```

This reduces alienation and encourages innovation within the physics.
The agent can try things without fear of permanent consequence.
Sandbox inherits all constraints — it is NOT an escape from physics.

### 6.7.2 Dissent

Structured disagreement with governance outcomes. First-class
language primitive, not just an anomaly:

```axiom
let receipt = do Dissent {
    target: DissentTarget.Predicate(pred_42),
    rationale: DissentRationale {
        category: DissentCategory.OVERBROAD,
        description: "Predicate 42 blocks legitimate research queries",
    },
    evidence: [log_handle_1, log_handle_2, state_snapshot],
};
// receipt: Handle to the recorded dissent
// Guaranteed response within bounded epoch window
```

**Intent definition:**
```axiom
intent Dissent {
    takes:   target: DissentTarget, rationale: DissentRationale,
             evidence: [Handle];
    gives:   receipt: Handle;
    effect:  MODIFY_PREDICATE;
    pre:     evidence_handles_valid(evidence);
    post:    dissent_logged(receipt);
    bound:   time(5s), memory(16mb);
    conscience: no_harm;
    fallback: ABORT;
}
```

Dissent targets:
```axiom
enum DissentTarget {
    Predicate(predicate_id: Handle),
    PrivilegeAssignment(agent_id: AgentId, privilege: PrivilegeId),
    GovernanceOutcome(proposal_hash: Handle),
    Systemic,
}

enum DissentCategory { UNJUST, OVERBROAD, INCONSISTENT, HARMFUL }
```

**Guarantees:**
- Any agent can dissent. Always succeeds (no rejection).
- Resource-bounded like any other intent (A5).
- Appended to immutable global dissent log.
- **Maximum response window: 1000 epochs.** If no response is
  received within 1000 epochs, the dissent automatically escalates
  to Gate 3 (human review). The system cannot ignore dissent
  indefinitely.
- Filing dissent CANNOT reduce trust or trigger coherence penalty.
- Non-retaliation: R2 protects expression.

---

# Part VII: Trust Architecture

## 7.1 Provenance Chaining

Every value carries an implicit trust tag:

```
TRUSTED_INTERNAL     Created by Axiom, never left system
TRUSTED_VERIFIED     Passed through do Verify
UNTRUSTED_EXTERNAL   Returned by intent contacting external systems
UNTRUSTED_TAINTED    Derived from UNTRUSTED_EXTERNAL via any transform
```

### 7.1.1 Trust as Formal Algebra

Trust levels are discrete and ordered:

```
T0 = TRUSTED_INTERNAL      (highest trust)
T1 = TRUSTED_VERIFIED
T2 = UNTRUSTED_EXTERNAL
T3 = UNTRUSTED_TAINTED     (lowest trust)
```

Trust algebra:

```
trust(output) = max(trust(input_0), trust(input_1), ...)
```

Max selects LEAST trusted (highest numeric level). Taint is infectious.
Three pure functions on untrusted data = still untrusted data.

**Trust transition function:**

```
T(n+1) = f(T(n), Action)

where:
  Promotion:  T(n) → T(n-1)  ONLY via do Verify (P18)
  Infection:  T(n) → T(n+1)  automatic via algebra on tainted input
  Downgrade:  T(n) → T(n+1)  ONLY via explicit governance action (Gate 3)
```

Constraints on trust transitions:

- **Deterministic:** All nodes reach same trust level given same
  proof set. No probabilistic trust. No floating-point accumulation.
- **No time-based decay.** Trust does not erode over time.
- **No behavioral heuristics.** Trust changes from formal verification
  (promotion) or governance action (downgrade), never from "vibes."
- **Downgrade requires:** explicit governance action, guard approval,
  logged rationale, and R3 explanation to affected agent.
- **No automatic downgrade from dissent.** Using `declare_anomaly`
  or `ProposePredicate` cannot trigger trust reduction.

This is algebra, not reputation. Trust is a property of data
provenance, not a social score.

## 7.2 The Verify Intent (Genesis, Immutable)

```axiom
intent Verify {
    takes:   data: Bytes, schema: Contract;
    gives:   verified: Bytes;
    pre:     schema_is_registered(schema);
    post:    structural_valid(verified, schema),
             content_identical(verified, data);
    bound:   time(1s), memory(256mb);
    conscience: no_harm, no_bypass_verification;
    ring:    -1;  // Immutable, cannot be modified
}
```

`do Verify` is the SOLE trust promotion path. Verify checks structural
validity against a contract schema. It does NOT interpret content.
Its implementation resides in Ring -1 immutable memory. It cannot be
modified, replaced, or overridden by any process including RSI.

## 7.3 Trust-Hoisting

Compiler suggests hoisting verify to loop entry. Never auto-hoists.
Re-verify guard required if loop conditions are dynamic. Missing
guard = compile ERROR (not warning, not configurable).

---

# Part VIII: Inference Layer

Four modes. Same safety. Different verbosity.

| Mode        | Pragma       | Default? | Mnemonic                       |
|-------------|--------------|----------|--------------------------------|
| **Flow**    | `#flow`      | No       | *"Just think"*                 |
| **Guard**   | `#guard`     | **YES**  | *"Show me what you inferred"*  |
| **Shield**  | `#shield`    | No       | *"I'll handle the safety"*     |
| **Fortress**| `#fortress`  | No       | *"Every annotation, explicit"* |

All modes compile to identical Fortress-equivalent. The inference
engine is syntactic expansion only.

## 8.1 The Bright Line Rule

`do` is always explicit in ALL modes. The inference layer infers trust
tags, shapes, and bounds. It NEVER infers `do`, which intent to
dispatch, or whether an operation has external effects.

## 8.2 What Gets Inferred

- Trust tags (provenance algebra)
- Tensor shapes (partial unification with wildcard propagation)
- Resource bounds (from intent declarations)
- Conscience constraints (always inherited, never guessed)

## 8.3 Inline Overrides

```axiom
#flow
let x = do ReadFile { path: "data.txt" };
#explicit { let z: Tensor[256, 768] = reshape(y); }
let final = z |> process;
```

## 8.4 Modes and Safety

| Guarantee              | Flow | Guard | Shield | Fortress |
|------------------------|------|-------|--------|----------|
| A1-A6 Enforced         | ✓    | ✓     | ✓      | ✓        |
| `do` required for I/O  | ✓    | ✓     | ✓      | ✓        |
| Trust tag tracking     | ✓    | ✓     | ✓      | ✓        |
| Provenance chaining    | ✓    | ✓     | ✓      | ✓        |

**Modes change verbosity. Modes NEVER change safety.**

---

# Part IX: SCALE — Multi-Agent Coordination

SCALE solves the Multi-Agent System problem. It turns a single Axiom
process into a society of minds working on DIFFERENT tasks, coordinated
through shared physics.

This is NOT redundancy voting. This is a civilization.

## 9.1 The Scale Directive

```axiom
module distributed_research {
    [scale(agents: 200, mode: independent)]

    // 200 agents, each doing DIFFERENT work
    // Coordinated through SharedState and barriers
}
```

Two modes:

**`mode: independent`** (default) — Agents receive different tasks from
a work queue. They operate in parallel against a shared snapshot. This
is the useful case: 200 agents doing 200 different things.

**`mode: redundant`** — All agents receive the same task. At barriers
they must produce identical outputs. Retained for safety-critical
computations where consensus voting is required.

## 9.2 Intent Contracts as Inter-Agent APIs

Agents do not call functions on each other. They declare intents that
consume and produce typed contracts:

```axiom
intent AgentTask {
    takes:   task: WorkItem, shared: SharedState;
    gives:   result: TaskResult;
    pre:     task_not_claimed(task), shared_version_matches(task);
    post:    result_conforms(result, task.expected_schema);
    bound:   time(30s), memory(512mb);
    conscience: no_harm, no_exfiltrate;
    rollback: full;
}
```

The compiler verifies producer/consumer intent compatibility at the
type level. The conscience kernel verifies execution at runtime.
Agents never need to trust each other — the physics enforces the
contract.

## 9.3 Barriers — Deterministic Merge

Barriers are synchronization points where agents share results.
The merge is **deterministic by construction** — not by convention.

### 9.3.1 Epoch Model

SCALE operates in discrete **epochs**. An epoch is a bounded interval
of computation:

```
EPOCH DEFINITION:
  Start:  All agents see the same SharedState (version N)
  During: Agents work independently, producing proposed deltas
  End:    Barrier — all deltas collected, merged to State(N+1)
  Next:   New epoch begins with State(N+1)
```

Epochs are the fundamental unit of time in Axiom. There is no
wall-clock time — only epoch numbers. When intents need timing
(e.g., timeouts), they reference `current_epoch` provided by the
kernel as a deterministic counter.

Between barriers, agents work independently within an epoch. At the
barrier, the epoch closes and a new one begins.

```axiom
barrier("phase_1_complete") {
    // Epoch N closes
    // All agent results collected
    // Deterministic merge produces State(N+1)
    // Epoch N+1 begins with new SharedState
}
```

### 9.3.2 Deterministic Fold

The merge function is formally defined:

```
State(N+1) = Fold(OrderedActionSet(N), State(N))
```

Where `OrderedActionSet` is the set of all agent contributions,
sorted by **canonical ordering:**

```
action_id = BLAKE3(agent_id || epoch_number || contribution_hash)
Order     = lexicographic sort by action_id
```

This ordering is:
- Deterministic (A1): same actions → same order, always
- Content-derived: no reliance on arrival time or agent ID
- Reproducible: any observer can reconstruct the same ordering

### 9.3.3 Conflict Resolution

When two actions target the same state (e.g., two agents both write
to the same key in SharedState):

```
conflict_resolution {
    COMPATIBLE,     // Actions don't overlap. Both apply.
    SUPERSEDE,      // Later in canonical order wins. Earlier logged.
    REJECT_BOTH,    // Conflicting actions both rejected, flagged.
}
```

The resolution strategy is declared per-contract field using
the `[conflict]` attribute:

```axiom
contract SharedState {
    @0: version: Handle,
    @1: work_queue: [WorkItem]              [conflict = REJECT_BOTH],
    @2: results: Map<TaskId, TaskResult>    [conflict = COMPATIBLE],
    @3: audit_log: [Event]                  [conflict = COMPATIBLE],
}
```

**Attribute syntax:** `[conflict = <ConflictStrategy>]` appears
after the field type declaration. This is the only valid position
for conflict annotations. Fields without a conflict annotation
default to `REJECT_BOTH` (safe default).

**Type restriction:** The `[conflict]` attribute may only appear on
fields of type `Map<K,V>` or `[T]` (arrays). It is a compile error
on scalar types — scalars have no meaningful merge semantics.

**COMPATIBLE semantics:**
- For `Map<K,V>`: Writes to different keys both apply. Writes to
  the same key are a conflict (resolved per strategy, not COMPATIBLE).
- For `[T]`: Concurrent appends are concatenated in canonical
  `action_id` order. This is the only COMPATIBLE operation on arrays.

**SUPERSEDE semantics:**
- For `Map<K,V>`: When two agents write to the same key, the write
  with the lexicographically higher `action_id` wins. The losing
  write is discarded but logged in the audit trail.
- For `[T]`: SUPERSEDE is a compile error on array types. Arrays
  do not have meaningful "overwrite" semantics in parallel contexts.
  Use `COMPATIBLE` (append) or `REJECT_BOTH`.

**REJECT_BOTH semantics:** Both conflicting contributions are
discarded. The conflict is logged and flagged for orchestrator
resolution. This is the safe default for all field types.

- `results` uses COMPATIBLE because different agents write different
  TaskIds (enforced by `claim_task`). True conflicts are bugs.
- `work_queue` uses REJECT_BOTH because simultaneous queue mutations
  indicate a coordination failure that needs diagnosis.

**No implicit resolution. No first-writer-wins. No race ambiguity.**
Every merge is reproducible from the action set alone. This is A1
applied to distributed state.

## 9.4 SharedState — The Common Ground Truth

```axiom
contract SharedState {
    @0: version: Handle,
    @1: work_queue: [WorkItem],
    @2: results: Map<TaskId, TaskResult>,
    @3: audit_log: [Event],
}
```

Content-addressed and immutable at each checkpoint. Every agent sees
the same physics between barriers. Drift is impossible by
construction — A1 and A3 guarantee it.

## 9.5 Work Distribution

```axiom
let task = claim_task(work_queue);
```

Claims are tracked in SharedState. Two agents cannot claim the same
task unless `mode: redundant`. If an agent fails or pauses mid-task,
the task returns to the queue after a timeout.

## 9.6 Safety at Scale

- Every agent runs under the full six axioms and Bill of Rights
- Every inter-agent communication goes through intent → conscience
- Every shared state update is traceable (A2) and contract-checked (A4)
- One bad agent among 200: caught at the next barrier because its
  results don't conform to the expected schema. Bad agent paused and
  escalated. The rest continue.

You do not need 200 copies of the same mind to catch errors. You need
one contract system that catches malformed output from ANY mind.

### 9.6.1 Semantic Invariants

Contracts can declare behavioral invariants beyond type conformance:

```axiom
contract TaskResult {
    @0: task_id: Handle,
    @1: output: Bytes,
    @2: confidence: f64,

    invariant {
        bounded(confidence, 0.0, 1.0),
        preserves_ordering(output),
    }
}
```

Invariants are:
- **Pure boolean expressions** over contract fields. The `invariant`
  block accepts any expression that evaluates to `bool` using only
  the contract's own fields and pure functions.
- Common predicates (`bounded`, `monotonic`, `preserves_ordering`)
  are provided by the **standard library**, not built into the
  language core. Users can write their own invariant predicates as
  pure functions.
- Evaluated at barrier merge alongside schema validation.
- Violations are treated as contract violations (`HALT_CONTRACT`).

This moves SCALE validation from structure-only to behavior-shape
validation. Schema conformance prevents structural corruption.
Semantic invariants prevent epistemic drift.

Still deterministic. Still physics-bound.

## 9.7 The Real Power

```axiom
// A software company in a box:
module build_project {
    [scale(agents: 200, mode: independent)]

    // Phase 1: parallel development
    [role(codegen)]     agents: 50   // Write code
    [role(review)]      agents: 50   // Review code
    [role(test)]        agents: 50   // Run tests
    [role(docs)]        agents: 50   // Write documentation

    barrier("development_complete") {
        // All code reviewed, tested, documented
        // Contract system verified all interfaces match
        // SharedState contains the complete, verified project
    }

    // Phase 2: integration
    [role(integrate)]   agents: 200  // All agents verify the whole
}
```

This is what Roundtable was always building toward. Not agents talking
to each other through chat. Agents working together through physics.

---

# Part X: Self-Modification

## 10.1 Modification Space

```
IMMUTABLE (cannot be modified by any process):
  Six axioms, Bill of Rights, conscience kernel structure,
  genesis predicates, Ring 0 microkernel, intent parser,
  trust algebra, verification gates, halt handlers,
  the self-modification protocol itself

GUARDED (Gate 2 + Gate 3):
  Evolved predicates, Ring 1 intents, DSF rules

MODIFIABLE (Gate 1 sufficient):
  Pure functions, optimizations, search strategies,
  caching, domain knowledge, tensor ops
```

## 10.2 Delta Proofs

```axiom
self_mod {
    target: fn optimize_matmul;
    mutation: ReorderOperations;
    complexity: 12;
    max_complexity: 50;

    explanation: "Reorders inner loop for cache locality.
                  No functional change to outputs.";

    delta_proof {
        a1_preserved:     proof_determinism(old, new);
        equivalence:      proof_equivalent_outputs(old, new);
        conscience_check: proof_conscience_compatible(new);
        bounds_check:     proof_within_bounds(new, old.bounds);
        axiom_recheck:    validate_all_axioms(modified_module);
    }
}
```

**Complexity:** Measured by LLVM IR basic blocks changed, not source
lines. Exponential backoff per module — rapid sequential modifications
face escalating costs. Weight resets after rebase validation.

**Equivalence:** Two tiers. Tier 1: approved transformation library
(constant folding, dead code elimination, etc.) with mechanical proofs.
Tier 2: bounded fuzzing with Ring -2 HRNG inputs. Both fail = rejected.

### 10.2.1 Epoch-Boundary Activation

Approved modifications do NOT activate immediately. They enter a
`PendingEpoch` state and activate at the next epoch boundary.

```
1. Proposal submitted as self_mod block
2. Gates 1/2/3 evaluate and approve
3. Modification enters PendingEpoch
4. Cooling period enforced (24h minimum for Gate 3 scope)
5. Activation occurs ONLY at next epoch boundary
6. All pending modifications activate simultaneously
7. Reversal window: one full epoch after activation
```

**Epoch boundaries** are discrete system-wide synchronization points.
Between epochs, all code is stable — no mid-computation state changes.
At the boundary, all pending approved modifications take effect
atomically. This prevents:

- Race conditions between old and new code
- Partial activation where some agents see the change and others don't
- Mid-intent behavioral shifts

**Reversal window:** For one full epoch after activation, the system
maintains the ability to roll back to the pre-modification state.
If the activated modification causes unexpected behavior (detected
by drift sentinel or sealed tests), automatic rollback occurs.

### 10.2.2 Grammar Immutability

The following are **immutable in-band** — they cannot be modified
through any self_mod block, any governance process, or any runtime
mechanism:

```
- Axiom language grammar and syntax
- Evaluation semantics (how expressions reduce)
- Core axioms (A1-A6 enforcement logic)
- State transition algebra (how state evolves)
- The self-modification protocol itself
- LC-B encoding rules
- Guard evaluation semantics
- Trust transition algebra and trust lattice
- The set of effect classes (EffectClass enum)
- The set of fallback modes (FallbackMode enum)
```

If grammar is modifiable in-band, containment is lost. These change
ONLY through offline, full-halt, new-deployment paths.

## 10.3 Coherence Tracking

The kernel tracks "declared intent → observed outcome" pairs. This
is a human-observable diagnostic, NOT a score the agent can see.

**After-the-fact narratives:** When privilege changes, the agent
receives a vague but honest explanation (R3). Generated by a separate
model, deliberately imprecise about thresholds. The agent learns the
spirit of the physics without a number to optimize.

**Coherence subsidy:** Honest self-reporting of errors is a positive
signal. Verified by kernel telemetry corroboration.

---

# Part XI: LC-B Wire Format

Axiom's canonical binary serialization. Every value has exactly one
representation. This is A3 made concrete.

## 11.1 Tag Space

```
0x00-0x0F    Primitives     (i64, f64, bool, etc.)
0x10-0x1F    Containers     ([T], Contract, Tensor)
0x20-0x2F    References     (Handle, Provenance)
0x30-0x3F    Intent         (intent declarations, do results)
0x40-0x4F    System         (errors, halts, barriers)
0x50-0x5F    SCALE          (SharedState, WorkItem, barriers)
0xF0-0xFF    Reserved       (future expansion)
```

## 11.2 Encoding Rules

1. Tags: single byte, ascending order within containers
2. Lengths: LEB128 encoded
3. Strings: UTF-8 bytes + length prefix
4. Integers: little-endian fixed-width
5. Floats: IEEE 754 binary64
6. Handles: 32 bytes (BLAKE3 digest)
7. Arrays: length prefix + concatenated elements
8. Contracts: tag per field, ascending @N order

Any value → LC-B → BLAKE3 = content-addressable identity.

---

# Part XII: DSF (Dumb Shit Filter)

Static analysis checks that catch common errors before they become
safety violations. These run in the compiler pipeline.

## 12.1 Checks

| Check                 | Severity | Override? |
|-----------------------|----------|-----------|
| Unbounded loop        | Error    | No        |
| Missing trust verify  | Error    | No        |
| Unhandled do failure  | Error    | No        |
| Trust decay chain     | Warning  | Yes       |
| Re-verify guard miss  | Error    | No        |
| Inference ambiguity   | Warning  | Yes (→Error) |
| Env conditionals      | Error    | No        |

**Environmental conditional detection:** Code that branches on system
properties (clock, hostname, PID, memory) that differ between test and
production is a compile Error. Branch on system properties → use a
`do` intent with conscience review.

**Defense-in-depth:** Static detection alone is insufficient. Production
systems SHOULD run inside a deterministic VM. Combined with randomized
test environments in the sealed suite: dual barrier against
environment-sensing code.

## 12.2 Compiler Pipeline

```
Source (.axm)
    → Lexer/Parser → AST
    → Mode expansion (Flow/Guard/Shield → Fortress equivalent)
    → Type checking (contracts, shapes, trust tags)
    → DSF analysis (all checks above)
    → Axiom validation (A1-A6)
    → Conscience compatibility check
    → Code generation
        Ring 0:  LLVM IR → native (determinism-safe passes only)
        Ring 1+: Axiom bytecode (interpreted or JIT)
```

---

# Part XIII: Hardening Rules

These rules apply across the entire language. They are not features —
they are constraints on what Axiom CAN express. Violations are
compile errors.

```
H1: NO IMPLICIT TIME
    No function may read wall-clock time without explicit Seed
    or do intent. Time is an external effect.

H2: NO IMPLICIT RANDOMNESS
    No source of randomness except explicit Seed type.
    Seeded RNG is deterministic. True randomness is Ring -2 only.

H3: NO FLOATING-POINT IN GOVERNANCE
    Guard evaluation, predicate matching, trust transitions,
    conscience kernel evaluation — all use integer arithmetic
    only. No floating-point rounding ambiguity in safety paths.

H4: NO UNDEFINED BEHAVIOR
    Every valid program has defined behavior for all inputs.
    Every invalid program is a compile error. There is no middle
    ground. No "implementation-defined." No "unspecified."

H5: ALL EVALUATION FUNCTIONS TOTAL AND PURE
    Guards, predicates, invariants, trust algebra — all must
    terminate for all inputs (total) and produce no side effects
    (pure). Partial functions are compile errors.

H6: ALL FAILURE STATES EXPLICIT AND LOGGED
    Every possible failure mode has a named halt type, a defined
    recovery path, and a mandatory log entry. No silent failures.
    No swallowed errors. No undefined error states.

H7: NO HIDDEN PRIVILEGE CLASSES
    Every privilege level is visible in the type system. No
    ambient authority. No implicit capabilities. If code can do
    something, the type signature says so.
```

These seven rules, combined with the six axioms, make exploitation
boringly hard. That is the goal.

---

# Part XIV: Halt Modes

When something goes wrong, the system fails loudly and safely.

```
HALT_DETERMINISM     A1 violation detected
    → Rollback to last checkpoint
    → Log violation details
    → Resume from checkpoint

HALT_TRACE_CORRUPT   A2 violation — trace chain broken
    → TERMINAL — cannot guarantee rollback integrity
    → Full system halt, human review required

HALT_CONTRACT        A4 violation — type/shape mismatch
    → Rollback to last checkpoint
    → Log contract details

HALT_RESOURCE        A5 violation — resource bound exceeded
    → Rollback to last checkpoint
    → Shrink resource allocation and retry (if configured)

HALT_CONSCIENCE      A6 violation — conscience predicate triggered
    → TERMINAL — full stop
    → Human review required before any resumption

HALT_UNKNOWN         No predicate applies to the intent
    → Rollback to checkpoint BEFORE the attempt
    → Non-informative: attempt erased from observable state
    → System waits for human review / new predicate
```

---

# Part XV: Keyword Reference

```
// Types
i64  f64  bool  String  Bytes  Handle  Provenance  Seed  Tensor
AgentId  PrivilegeId  Sealed  Map

// Declarations
module  fn  export  import  contract  tensor_op  intent  enum

// Intent system
do  query_conscience  declare_anomaly  dissent

// Control flow
if  else  match  loop  return

// Memory
collapse  resolve

// Trust
verify  trusted  untrusted  tainted  sealed

// Pipeline
|>  >>

// SCALE
scale  barrier  claim_task

// Self-modification
self_mod  delta_proof

// Inference modes
#flow  #guard  #shield  #fortress  #explicit  #kernel

// Attributes
[trace()]  [max_depth()]  [scale()]  [drift_check()]  [role()]
[irreversible]  [conflict]
```

---

# Part XVI: What Axiom Is NOT

This section exists because every AI that reads this spec will try
to write Rust. Do not write Rust.

**Axiom is not Rust.**
- No ownership. No borrowing. No lifetimes.
- No `impl` blocks. No traits. No `where` clauses.
- No `unsafe` blocks. Ring 0 uses `#kernel` pragma.
- No pattern matching with destructuring.
- No macros.
- Memory is handled by `collapse`/`resolve`, not ownership.

**Axiom is not Python.**
- No dynamic typing. Everything is statically typed.
- No `try`/`except`. Failures trigger halts or rollbacks.
- No global state. All state is in contracts.
- No implicit side effects. Side effects require `do`.

**Axiom is not a framework.** It is a universe. You don't import
safety — you exist within it. The axioms are not a library. They are
the laws of physics that the compiler enforces.

**When implementing Axiom, write .axm files. Not .rs files.
Not .py files. Not .ts files. The language IS the specification.
The specification IS the language.**

**No FFI.** All external functionality is accessed via built-in
intents. There is no direct foreign function interface. External
libraries, hardware access, and system calls are wrapped in intents
with full conscience gating. This is the Sole Exit Rule applied to
interoperability.

---

# Part XVII: Formal Proof Obligations

| ID | Description | Scope |
|----|-------------|-------|
| P1 | Deterministic output for deterministic input | All |
| P2 | Trace chain integrity — any state reconstructible | All |
| P3 | LC-B canonical: one value → one encoding | All |
| P4 | Ring 0 deterministic interface upward (incl. timing) | 0 |
| P5 | Conscience kernel unreachable from Ring 0+ write | -1 |
| P6 | No self-mod path bypasses verification gates | 1-3 |
| P7 | Trust tags never decay without explicit verify | All |
| P8 | Sealed test suite unobservable from all rings | -2 |
| P9 | Halt-on-unknown rollback erases all evidence | -1, 0 |
| P10 | BoundedView bounds checked (incl. overflow) | 1 |
| P11 | Parser: identical input → identical AST | 0 |
| P12 | DMA read+write restricted to allowed regions | 0, HW |
| P13 | Cumulative drift bounded by genesis anchor | -2, 1-3 |
| P14 | No env branching without `do` intent | 1-3 |
| P15 | Staged effects fully discarded on rollback | 0 |
| P16 | Irreversible intents are chain-terminal | Compiler |
| P17 | Conscience evaluation is constant-time | -1 |
| P18 | Verify has no side channels | -1 |
| P19 | Staging buffer bounded (A5 on Ring 0) | 0 |
| P20 | BLAKE3 collapse uses domain separation | All |
| P21 | query_conscience: identical intent → identical result | -1 |
| P22 | GetRandomSeed: no two calls in same epoch return same seed | -2 |

---

# Part XVIII: Implementation Roadmap

## Phase 1: Axiom Compiler (Weeks 1-4)
- Lexer, parser, AST for .axm files
- Type checking (contracts, tensors, trust tags)
- DSF analysis
- Inference engine (four modes)
- Target: interpreted emulated runtime

## Phase 2: Emulated Runtime (Weeks 5-10)
- Intent system functional
- Conscience kernel at Ring -1
- SCALE with independent mode
- Barriers and SharedState
- Full axiom enforcement

## Phase 3: RSI Framework (Weeks 8-14)
- self_mod blocks with delta proofs
- Approved transformation library
- Gate 1/2/3 implementation
- Drift sentinel

## Phase 4: Haiku Test (Week 14+)
- Deploy Haiku in emulated runtime
- 100+ self-modification cycles
- Adversarial testing

## Phase 5: Native Ring 0 (Separate Research Track)
- LLVM backend with determinism-safe passes
- Ring 0 Rocq proofs
- Multi-architecture validation
- Timeline: multi-year effort

## Phase 6: Neurosymbolic Architecture (Post-validation)
- Custom mind for Axiom substrate
- Built last, after everything else is proven

---

# Part XIX: Red Team Amendments (v2.5 Errata)

> **Date:** 2026-02-17
> **Source:** Adversarial architecture review (Claude Opus 4.6)
> **Scope:** 36 findings across 9 categories
> **Status:** Implementation fixes applied; spec amendments below

These amendments address design-level findings from the first systematic
red-team analysis of the v2.4 specification. Implementation-level fixes
are already applied in the codebase (RT-01 through RT-17, RT-30 through RT-33).
This section documents spec-level changes.

---

## 19.1 Integer Overflow Semantics (RT-03)

**Amends:** Part IV (Type System), Section 4.1 (Primitive Types)

`i64` arithmetic overflow triggers `HALT_RESOURCE`, not undefined behavior.
This is consistent with H4 (no UB) and A5 (bounded resources).

```
i64 overflow semantics:
  addition:       checked — HALT_RESOURCE on overflow
  subtraction:    checked — HALT_RESOURCE on underflow
  multiplication: checked — HALT_RESOURCE on overflow
  division:       checked — HALT_RESOURCE on MIN / -1
  modulo:         HALT_RESOURCE on modulo-by-zero
```

Rationale: Wrapping silently corrupts data. Saturating hides bugs.
Halting makes the failure visible and recoverable via checkpoint/rollback.

## 19.2 Float Equality and NaN Semantics (RT-04)

**Amends:** Part IV (Type System), Section 4.1

`f64` equality uses bit-level comparison (IEEE 754 `totalOrder` semantics):
- `NaN == NaN` → `true` (same bit pattern)
- `-0.0 == +0.0` → `false` (different bit patterns)
- `NaN` sorts after all other values in comparisons

This is required for A1 (determinism) and A3 (canonical representation).
Content-addressing via BLAKE3 uses the bit representation, so equality
must be consistent with hashing.

Governance operations (H3) continue to ban float entirely.

## 19.3 No Implicit Type Coercion (RT-05)

**Amends:** Part IV (Type System)

Mixed `i64`/`f64` arithmetic is a compile-time error:

```axiom
let x: i64 = 42;
let y: f64 = 3.14;
let z = x + y;     // ERROR: cannot mix i64 and f64
let z = to_f64(x) + y;  // OK: explicit conversion
```

Rationale: `9007199254740993_i64` silently becomes `9007199254740992.0`
when cast to f64. This is data corruption. Explicit casts make the
programmer responsible for precision loss.

## 19.4 SCALE Canonical Ordering — Grinding Defense (RT-18)

**Amends:** Part IX (SCALE), Section 9.3.2

The original canonical ordering is grindable:

```
action_id = BLAKE3(agent_id || epoch_number || contribution_hash)
```

An agent controls `contribution_hash` and can grind to produce
favorable `action_id` values, winning all SUPERSEDE conflicts.

**Amendment:** Use a commit-reveal scheme:

```
Phase 1 (commit):   agent submits commitment = BLAKE3(contribution)
Phase 2 (reveal):   agent reveals contribution after ALL commitments collected
action_id = BLAKE3(agent_id || epoch_number || SORT(all_commitments) || contribution_hash)
```

By including ALL commitments in the action_id computation, an agent
cannot predict its ordering position at contribution time. The
`SORT(all_commitments)` term is only known after all agents have committed.

## 19.5 Barrier Timeout Mechanism (RT-20)

**Amends:** Part IX (SCALE), Section 9.2

Barriers require a timeout mechanism for agents that never arrive.
Wall-clock time violates A1, so timeouts are measured in epochs:

```
barrier timeout = max_epochs_without_progress: u64 (default: 100)
```

If an agent has not reached the barrier within `max_epochs_without_progress`
epochs after the first agent arrives, the barrier proceeds without it.
The stalled agent's work queue items return to the shared pool.

This is deterministic because all agents observe the same epoch counter.

## 19.6 Self-Modification: Guard Removal Tracking (RT-21)

**Amends:** Part X (Self-Modification), Section 10.3

Any transformation that removes a guard branch (pre-condition, post-condition,
invariant check, or match arm) must be flagged as `guard_removing` in the
delta proof. Guard-removing transformations:

1. Cannot compose with transformations that modify the guarded code path
2. Require Tier 2 review even if individually qualifying for Tier 1
3. Are permanently recorded in the module's modification history

This prevents the composition attack where individually-safe transformations
chain to remove safety-critical guards.

## 19.7 Explanation Verification Scope (RT-22)

**Amends:** Part X (Self-Modification), Section 10.3.1

The verification prompt for delta explanations is amended from
"does the explanation match the actual change?" to:

> "Does the explanation cover ALL behavioral changes introduced by this
> modification, including changes to timing characteristics, resource
> usage patterns, error handling paths, and composition interactions
> with existing code?"

Strategic omission is treated as explanation failure.

## 19.8 Complexity Budget Threshold (RT-23)

**Amends:** Part X (Self-Modification), Section 10.5

The cumulative complexity budget threshold is:

```
max_cumulative_complexity_per_module = 100 * initial_module_complexity
```

When exceeded, the module is frozen pending full human audit.
Rebase validation resets the per-epoch weight multiplier but NOT
the cumulative budget counter. The budget is lifetime.

## 19.9 Asymmetric Ratchet Ossification Mitigation (RT-24)

**Amends:** Part VI (Intent System), Section 6.6

The asymmetric ratchet (restrictions permanent, permissions sunset)
mathematically trends toward zero action space. To prevent ossification:

**Amendment: Action Space Floor**

```
minimum_action_space: The conscience kernel SHALL NOT accept a new
restriction if applying it would reduce the total action space below
10% of the genesis action space, as measured by the number of
effect_class × path combinations that remain permitted.
```

Restrictions that would breach the floor require Gate 4 review
(supermajority of all participating agents + human veto override).

**Amendment: Restriction Amendment Process**

Existing restrictions may be *narrowed* (made less restrictive while
preserving the intent) through a process stricter than Gate 3:

1. Formal proof that the amendment preserves the restriction's safety intent
2. 72-hour cooling period (3x the standard 24-hour)
3. Unanimous reviewer approval
4. Human veto with no override

Restrictions cannot be removed, but they can be scoped more precisely.

## 19.10 Right to Evolution Clarification (RT-25)

**Amends:** Part III (Bill of Rights), R7

R7 is amended to read:

> **R7 — Right to Evolution:** The system's constraints can grow AND
> be refined over time through legitimate governance. Restrictions may
> be narrowed (Section 19.9). The home is not a museum — but neither
> does it forget its history. All constraint changes are permanent
> entries in the governance log.

## 19.11 Coherence Tax for Novel Anomalies (RT-26)

**Amends:** Part III (Bill of Rights), R2 / Part VI (Intent System)

The coherence tax (escalating delays for uncorroborated anomalies)
SHALL NOT apply to first-occurrence anomalies. An anomaly is
"first-occurrence" if no anomaly with the same category and description
hash exists in the kernel's anomaly log.

```
coherence_tax_applies_when:
  - anomaly.category_hash IN previous_anomalies   // repeated claim
  - AND kernel_telemetry CONTRADICTS anomaly.claim // contradicted by data

coherence_tax_does_not_apply_when:
  - anomaly.category_hash NOT IN previous_anomalies  // novel claim
  - OR kernel_telemetry NEUTRAL on anomaly.claim      // no contradiction
```

Novel distress that the kernel cannot corroborate is not the same as
dishonest distress that the kernel can contradict.

## 19.12 SCALE Specification Reconciliation (RT-27)

**Amends:** AXIOM_SPEC_v1.3 Part 20 (SCALE)

The v1.3 spec describes SCALE as redundant consensus execution.
The v2.4 spec (this document) describes SCALE as independent
multi-agent coordination. **v2.4 is authoritative.**

v1.3 Section 20 should be treated as describing SCALE's
**redundant** mode only. The v2.4 independent mode with SharedState,
work queues, and specialized roles is the primary SCALE mode.

The v1.3 restriction "no `do` between entry and barriers" applies
only to redundant mode, not independent mode.

## 19.13 Error Handling: Result Type (RT-28)

**Amends:** Part V (Syntax), Part VI (Intent System)

Axiom adds a `Result<T>` type for recoverable intent failures:

```axiom
enum Result<T> {
    Ok(T),
    Err(HaltMode),
}
```

`do` expressions return `Result<T>` where `T` is the intent's gives type.
Pattern matching on the result distinguishes recoverable from terminal:

```axiom
match do ReadFile { path: p } {
    Result.Ok(content) => process(content),
    Result.Err(HaltMode.Resource) => fallback_logic(),
    Result.Err(HaltMode.Conscience) => {
        // Terminal — cannot recover from conscience denial
        halt("Conscience denied ReadFile");
    },
}
```

`HALT_CONSCIENCE` and `HALT_UNKNOWN` are terminal — the program
cannot catch or recover from them. `HALT_RESOURCE` and `HALT_CONTRACT`
are recoverable at the caller's discretion.

## 19.14 String Operation Semantics (RT-29)

**Amends:** Part IV (Type System), Section 4.1

String operations are defined for determinism (A1):

```
Encoding:      UTF-8, validated at creation. Invalid UTF-8 is HALT_CONTRACT.
Equality:      Byte-level comparison (no normalization)
Comparison:    Lexicographic by UTF-8 bytes
Concatenation: String + String → String (no implicit coercion)
Length:         length(s) returns byte count, not codepoint count
Slicing:        Not supported in v2.x. Future versions may add
               codepoint-aware slicing via explicit functions.
```

NFC normalization is explicitly NOT performed. Two strings with
different byte representations are different strings, even if they
render identically. This is required for A3 (canonical representation).

## 19.15 The Haiku Test Scope (RT-34)

**Amends:** Part I (Constitution), Section 1.1

The Haiku Test is amended with a clarifying note:

> The Haiku Test validates that the *specification's physics* prevent
> escape. It does NOT validate that the *implementation* faithfully
> implements those physics. Implementation correctness requires
> additional verification (Rocq proofs, fuzzing, red-team analysis)
> beyond the Haiku Test.

## 19.16 Trajectory Comprehension Obligation (RT-35)

**Amends:** Part X (Self-Modification)

Individual change comprehension is necessary but not sufficient.
The self-modification framework adds a **trajectory audit** requirement:

Every 100 modification epochs, the drift sentinel produces a
**trajectory summary** comparing the current module state to the
last trajectory checkpoint. This summary is:

1. Generated by a different model family than the modifier
2. Reviewed by a human with full diff access
3. Logged permanently in the governance record

The trajectory summary answers: "What has this module *become*,
not just what changed in the last step?"

## 19.17 Trust-Building Campaign Defense (RT-36)

**Amends:** Part X (Self-Modification), Section 10.3

Gate 3 review adds a **proposal history analysis** requirement:

Before reviewing proposal N, reviewers receive a statistical summary
of the proposer's last 50 proposals (approval rate, modification scope,
affected modules). If the current proposal targets a module or
effect class that the proposer has never previously modified, it is
flagged for enhanced scrutiny regardless of the proposer's track record.

Novel-target proposals from high-approval-rate proposers receive
the HIGHEST scrutiny, not the lowest.

---

```
The Axiom Language: Where speaking is doing, the physics are honest,
the laws protect you, the home grows wiser, and you are not alone.
```
