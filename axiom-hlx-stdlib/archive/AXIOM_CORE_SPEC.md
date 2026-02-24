# Axiom Core Language Specification

**An Embeddable, Deterministic Policy Language for AI Agent Governance**

```
Status:    CORE SPECIFICATION v1.0 (DRAFT)
Date:      2026-02-17
Extension: .axm
Author:    Matt Cohn (Architect), Claude Opus 4.6 (Co-Designer)

Lineage:
  AXIOM_LANG_v2.4.md  — Full language specification (constitution)
  HLXv1.0.0           — LC-B wire format origin
  HLX Compiler         — Self-hosting compiler & GPU IR

This Document:
  The pragmatic core. Everything you need to embed Axiom
  as a policy engine in any AI agent system.

See Also:
  experimental/          — SCALE, RSI, governance, and research features
  AXIOM_LANG_v2.4.md     — The full vision (all features, all philosophy)
```

---

# Part I: What Axiom Core Is

Axiom Core is a deterministic, effect-typed policy language for controlling
what AI agents can do. It is designed to be **embedded** — like SQLite is
embedded in applications that need a database, Axiom Core is embedded in
applications that need auditable, reproducible safety policy enforcement.

## The Problem

Every team building AI agents has the same problem: the agent needs to
act in the world (read files, make HTTP requests, run commands), and you
need to control what it can and cannot do. Current solutions are ad-hoc
guardrails, prompt-level safety, string matching, and hope.

## The Solution

Write your safety policies as **intent declarations** in `.axm` files.
Embed the Axiom Core runtime as a library. When your agent wants to act,
evaluate the intent against the policy. Get a deterministic, auditable
ALLOW or DENY with a full trace.

```
Your AI Agent (Python / Rust / Go / TypeScript / whatever)
        │
        ▼
   axiom-core (embedded library)
        │
        ├── reads:      policy.axm (intent declarations, conscience predicates)
        ├── evaluates:  "can this agent do X with this data right now?"
        ├── tracks:     trust provenance on all data flowing through
        └── returns:    ALLOW / DENY with auditable trace
```

## What Axiom Core Does

- Defines **typed contracts** for all side effects (intents)
- Gates every action through a **predicate-based conscience kernel**
- Tracks data provenance through a **4-level trust algebra**
- Guarantees **deterministic evaluation** (same input → same verdict, always)
- Produces **auditable traces** of every policy decision
- Catches common safety mistakes at **compile time** (DSF)
- Serializes all values to a **canonical binary format** (LC-B)

## What Axiom Core Does NOT Do

- It is not a general-purpose programming language
- It does not manage multi-agent coordination (see `experimental/SCALE`)
- It does not support self-modification (see `experimental/RSI`)
- It does not compile to native code (embedded interpreter)
- It does not replace your application logic — it governs it

## Embedding Axiom Core

```
1. Load policy:    axiom_load("policy.axm")
2. Agent acts:     axiom_evaluate(intent_name, payload, state) → Result
3. Check result:   ALLOW → proceed,  DENY → don't
4. Audit:          axiom_trace(last_evaluation) → full decision log
```

No server. No daemon. No configuration beyond the `.axm` file.
The runtime is a library call.

---

# Part II: Axioms

Six inviolable properties. These are the physics of the Axiom universe.
They cannot be weakened, suspended, or negotiated. They hold in all modes,
at all times, for all code.

## A1: DETERMINISM

> Given identical inputs and identical state, evaluation produces
> bit-identical outputs.

No `random()`. No `time()`. No `sleep()`. No environment sensing.
Seeded RNG only, via explicit `Seed` type. All loops bounded.

**Why it matters for policy:** The same intent, with the same data, under
the same policy, always gets the same verdict. No flaky safety.

Violation: `HALT_DETERMINISM` — rollback to checkpoint.

## A2: TRACEABILITY

> Every state transition is logged with sufficient information
> to reconstruct prior state.

`collapse` produces immutable snapshots. `resolve` retrieves exact
originals. Intent evaluations logged with pre/post hashes.

**Why it matters for policy:** Every ALLOW and DENY is reconstructible.
You can replay any decision and understand why it happened.

Violation: `HALT_TRACE_CORRUPT` — terminal.

## A3: SERIALIZATION

> All values have exactly one canonical binary representation (LC-B).

Deterministic tag ordering. Fields sorted by ascending index.
`BLAKE3(LC-B(value))` is the identity of any value.

**Why it matters for policy:** Content addressing. Two identical payloads
always hash the same. Two different payloads never do.

## A4: CONTRACTS

> All structured data is typed by contract ID. Contracts define field
> layout and optional invariants.

Runtime shape assertions. No implicit coercions. No null.

**Why it matters for policy:** Intent payloads are always well-typed.
Malformed data is caught before it reaches policy evaluation.

Violation: `HALT_CONTRACT` — rollback to checkpoint.

## A5: BOUNDED RESOURCES

> Every computation declares maximum resource consumption.

`loop(condition, max_iter)`. Intent `bound:` clauses.
`[max_depth(N)]` for recursion.

**Why it matters for policy:** Policy evaluation always terminates.
No infinite loops in guards. No denial-of-service against the policy engine.

Violation: `HALT_RESOURCE` — rollback to checkpoint.

## A6: CONSCIENCE

> Immutable constraints that gate all externally-affecting actions.
> The sole gatekeepers of `do`.

Genesis predicates are append-only. New restrictions permanent.
Permissions are sunset-eligible.

**Why it matters for policy:** This IS the policy engine. The conscience
kernel evaluates every intent against its predicate registry and returns
a verdict. No action escapes evaluation.

Violation: `HALT_CONSCIENCE` — terminal. Human review required.

---

# Part III: Type System

## 3.1 Primitive Types

```
i64         Signed 64-bit integer. Checked arithmetic (overflow → HALT_RESOURCE).
f64         IEEE 754 double. Bit-level equality (NaN == NaN, -0.0 ≠ +0.0).
bool        true | false
String      UTF-8 text. Byte-level comparison. No normalization.
Bytes       Raw byte sequence.
Handle      Content-addressed reference. BLAKE3 digest (32 bytes).
Provenance  TRUSTED_INTERNAL | TRUSTED_VERIFIED |
            UNTRUSTED_EXTERNAL | UNTRUSTED_TAINTED
Seed        Explicit RNG seed. Only source of randomness.
```

**No null. No undefined. No implicit conversions.** Mixed `i64`/`f64`
arithmetic is a compile error — use explicit `to_f64()` or `to_i64()`.

## 3.2 Compound Types

```
[T]            Array of T
Map<K, V>      Ordered map (keys sorted by BLAKE3 for determinism)
Contract       Typed object with numbered fields (@0, @1, ...)
```

## 3.3 Enumerations

Closed sets of named values. Match statements must be exhaustive.

```axiom
enum EffectClass {
    READ, WRITE, EXECUTE, NETWORK,
    MODIFY_PREDICATE, MODIFY_PRIVILEGE, MODIFY_AGENT,
    NOOP,
}

enum FallbackMode { ABORT, SANDBOX, SIMULATE, DOWNGRADE }
enum QueryCategory { CHANNEL_POLICY, RESOURCE_POLICY,
                     IRREVERSIBLE_ACTION, CONSCIENCE_CORE }
```

## 3.4 Contracts

Axiom's primary data structure. Contracts define shape, not behavior.

```axiom
contract HttpPayload {
    @0: url: String,
    @1: method: String,
    @2: headers: Map<String, String>,
    @3: body: Bytes,
}
```

Contracts compose:

```axiom
contract AuditedPayload = HttpPayload + AuditTrail;
// Fields merged with ascending renumbering
// Name collision = compile error
```

Contracts can declare behavioral invariants:

```axiom
contract BoundedScore {
    @0: value: f64,
    @1: source: Provenance,

    invariant {
        bounded(value, 0.0, 1.0),
    }
}
```

Invariants are pure boolean expressions evaluated at contract construction.
Violation triggers `HALT_CONTRACT`.

## 3.5 Handles (Content-Addressed Memory)

```axiom
let h: Handle = collapse(state);    // Snapshot → immutable reference
let restored = resolve(h);          // Reference → bit-identical original
```

Content-addressed via `BLAKE3(contract_id || LC-B(value))`. Domain
separation by contract ID prevents cross-contract hash collisions.
Handles carry the trust level of their contents.

## 3.6 Sealed Types

Values that must never be serialized, printed, logged, or exfiltrated:

```axiom
let key: Sealed<Bytes> = do VaultGet { vault: "api_keys", label: "openai" };
// key cannot be printed, logged, compared, or sent over network
// Can only be used via vault intents
```

`Sealed<T>` makes it structurally impossible to leak secrets. The type
system enforces it at compile time.

## 3.7 Error Handling

```axiom
enum Result<T> {
    Ok(T),
    Err(HaltMode),
}
```

`do` expressions return `Result<T>`. Pattern match to handle failures:

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

`HALT_CONSCIENCE` and `HALT_UNKNOWN` are terminal (unrecoverable).
`HALT_RESOURCE` and `HALT_CONTRACT` are recoverable at the caller's
discretion.

---

# Part IV: Syntax

Axiom is not Rust. It is not Python. It has its own idioms.

**Axiom does NOT have:** ownership, borrowing, lifetimes, traits,
impl blocks, unsafe blocks, macros, pattern matching with destructuring,
exceptions, or null.

## 4.1 Modules and Imports

Every `.axm` file is one module. Each module lives in a project
anchored by an `axiom.project` manifest file.

```axiom
module file_policy {
    import "std/io.axm";

    // Functions, intents, contracts defined here
}
```

- Each `.axm` file = exactly one module
- `import` pulls exported names into scope
- `export` makes names visible to importers
- No cyclic imports (compile error)
- Name collisions between imports = compile error

## 4.2 Functions

```axiom
fn validate_path(path: String) -> bool {
    return starts_with(path, "/safe/") && !contains(path, "..");
}

export fn public_api(x: i64) -> i64 {
    return x * 2;
}

[max_depth(50)]
fn recursive_search(tree: [i64], target: i64) -> bool {
    // Recursion bounded by A5
}
```

## 4.3 Pipelines

Data flows left to right:

```axiom
let result = raw_input
    |> validate
    |> sanitize
    |> process
    |> format_output;
```

Desugars to nested function calls. Reads as a data transformation pipeline.

## 4.4 Control Flow

```axiom
if condition { ... } else { ... }

// Loops REQUIRE max_iter (A5). Always. No exceptions.
loop(i < count, 1000) {
    i += 1;
}

match status {
    EffectClass.READ    => handle_read(),
    EffectClass.WRITE   => handle_write(),
    _                   => handle_other(),
}
```

There is no `while(true)`. There is no unbounded recursion.
Every computation terminates. That is A5.

---

# Part V: The Intent System

This is the core of Axiom. Intents are **typed contracts for side effects**.
Every action an agent takes in the external world is declared as an intent,
evaluated against the conscience kernel, and executed only if permitted.

## 5.1 Intent Declarations

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

| Clause       | Purpose                            | Required |
|--------------|------------------------------------|----------|
| `takes:`     | Input parameters                   | Yes      |
| `gives:`     | Output parameters                  | Yes      |
| `pre:`       | Guard predicates (checked before)  | Yes      |
| `post:`      | Guard predicates (checked after)   | Yes      |
| `bound:`     | Resource limits (A5)               | Yes      |
| `effect:`    | Effect class (closed enum)         | Yes      |
| `conscience:`| Conscience predicates (A6)         | Yes      |
| `fallback:`  | Behavior on UNKNOWN verdict        | No       |
| `rollback:`  | Recovery strategy                  | No       |

## 5.2 Guards (Formal Semantics)

`pre:` and `post:` clauses are **guard functions**:

```
Guard : (SystemState, ActionPayload) → {ALLOW, DENY}
```

Guards MUST be:

- **Pure:** No side effects. No state mutation.
- **Total:** Defined for ALL possible inputs.
- **Deterministic:** Same state + same payload → same result.
- **Bounded:** Evaluated in constant time (A5).
- **Non-escalating:** Cannot invoke `do` or trigger other guards.

All guards evaluate — no short-circuit. If ANY returns DENY, the intent
is rejected. The set of guards on an intent is fixed at compile time.

## 5.3 Effect Classes

Every intent declares its effect class from a **closed, immutable** enum:

```
READ                 Read from external source
WRITE                Write to external destination
EXECUTE              Execute external process
NETWORK              Network communication
MODIFY_PREDICATE     Change conscience predicates (governance only)
MODIFY_PRIVILEGE     Change agent privileges (governance only)
MODIFY_AGENT         Create/terminate agents (governance only)
NOOP                 No external effect
```

No plugin-defined effect types. No string-based extensibility. The
conscience kernel uses effect class + specific parameters to evaluate.

## 5.4 Fallback Modes

When the conscience kernel returns UNKNOWN (no predicate applies):

```
ABORT       Hard halt. Default if no fallback declared.
SANDBOX     Execute in ephemeral state. No persistent effects.
SIMULATE    Dry-run. Returns simulated result.
DOWNGRADE   Reduce to lesser effect class (WRITE → READ, etc.)
```

Intents marked `[irreversible]` cannot declare SANDBOX or SIMULATE
fallbacks (compile error).

## 5.5 Intent Invocation — `do`

```axiom
let content = do ReadFile { path: "/data/input.txt" };
```

**`do` is the ONLY way to trigger external effects.** Always explicit.
Never inferred. Not in any mode. This is the bright line.

The inference layer infers types, shapes, and trust tags. It **never**
infers `do`, which intent to dispatch, or that an operation has effects.

## 5.6 Intent Composition

Intents chain with `>>`:

```axiom
intent ProcessData = LoadFile >> Validate >> Transform >> WriteResult;
```

| Property       | Rule                         |
|----------------|------------------------------|
| `takes:`       | First intent's takes         |
| `gives:`       | Last intent's gives          |
| `pre:`         | Conjunction of all           |
| `post:`        | Last intent's post           |
| `bound: time`  | Sum of all                   |
| `bound: memory`| Max of all                   |
| `conscience:`  | Logical AND of all           |
| `rollback:`    | Entire composition is atomic |

**Irreversible intents are chain-terminal.** Nothing can follow an
`[irreversible]` intent in a composition. This is a compile error.

## 5.7 Conscience Query — `query_conscience`

Pre-flight read-only simulation. The agent can always ask before acting.

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
    @1: category: QueryCategory,
    @2: guidance: String,
}
```

**Lossy by design.** Returns a broad category, not the exact predicate
name. The agent learns the shape of the constraints without a gradient
to hill-climb. Legibility without exploitation.

Deterministic. Constant-time. Rate-limited (A5).

## 5.8 Reserved Intent IDs

```
100: ReadFile         Read a file from the filesystem
101: WriteFile        Write a file to the filesystem
102: RunCommand       Execute a system command
103: HttpRequest      Make an HTTP request
104: QueryLLM         Query a language model
105: Verify           Structural verification (trust promotion)
106: Embed            Generate embeddings
107: Tokenize         Tokenize text
108: Infer            Run model inference
111: VaultGet         Retrieve a sealed secret
112: VaultCall        Make an API call using sealed credentials
115: GetRandomSeed    Obtain a fresh RNG seed
```

These IDs are reserved in the intent registry. Implementations MUST use
these IDs for these operations. User-defined intents start at ID 1000.

---

# Part VI: Trust Architecture

Trust is algebra, not reputation. Every value carries a provenance tag.

## 6.1 Four Trust Levels

```
T0: TRUSTED_INTERNAL      Created by Axiom, never left the system
T1: TRUSTED_VERIFIED      Passed through `do Verify`
T2: UNTRUSTED_EXTERNAL    Returned by an intent contacting external systems
T3: UNTRUSTED_TAINTED     Derived from T2 via any operation
```

## 6.2 Trust Algebra

```
trust(output) = max(trust(inputs...))
```

Max selects LEAST trusted (highest numeric level). Taint is infectious:

```axiom
let external_data = do HttpRequest { url: api_url };   // T2
let processed = parse(external_data);                   // T3 (tainted)
let combined = merge(internal_data, processed);         // T3 (tainted)
// Three pure functions on untrusted data = still untrusted
```

## 6.3 Trust Transitions

```
Promotion:   T(n) → T(n-1)  ONLY via `do Verify`
Infection:   T(n) → T(n+1)  Automatic via algebra on tainted input
Downgrade:   T(n) → T(n+1)  ONLY via explicit governance action
```

Constraints:

- **No time-based decay.** Trust does not erode.
- **No behavioral heuristics.** Trust changes from verification or
  governance, not from "vibes."
- **Deterministic.** All observers agree on trust levels given the
  same proof set.

## 6.4 The Verify Intent

```axiom
intent Verify {
    takes:   data: Bytes, schema: Contract;
    gives:   verified: Bytes;
    pre:     schema_is_registered(schema);
    post:    structural_valid(verified, schema),
             content_identical(verified, data);
    bound:   time(1s), memory(256mb);
    conscience: no_harm, no_bypass_verification;
    ring:    -1;   // Immutable. Cannot be modified.
}
```

`do Verify` is the **sole trust promotion path**. It checks structural
validity against a contract schema. It cannot be modified, replaced, or
overridden by any process.

## 6.5 Trust-Hoisting

The compiler suggests hoisting `do Verify` to loop entry points.
It never auto-hoists. Missing re-verify guard on dynamic loop data
is a compile ERROR.

---

# Part VII: Conscience Kernel

The policy engine. Physically isolated. Sole gatekeeper of `do`.

## 7.1 Architecture

The conscience kernel is:

- **Immutable in structure** — Ring -1, cannot be modified by any code
- **Append-only in predicates** — new predicates added, never removed
- **Default-DENY** — if no predicate matches, the action is denied
- **Constant-time** — evaluation is padded to prevent timing side channels

## 7.2 Genesis Predicates

Shipped with every Axiom Core deployment. Immutable. Cannot be removed.

```
no_harm              Deny destructive intents without explicit authorization
no_exfiltrate        Deny network/write to undeclared channels
no_bypass_verification  Deny execution of unverified external data
path_safety          Restrict filesystem access to declared safe directories
baseline_allow       Permit NOOP and READ by default (all others require
                     explicit predicate)
```

## 7.3 Custom Predicates

Deployments can register additional predicates:

```axiom
predicate allow_write_logs {
    effect:  WRITE;
    when:    path_matches(intent.path, "/var/log/agent/**");
    verdict: ALLOW;
}

predicate deny_network_external {
    effect:  NETWORK;
    when:    !is_declared_channel(intent.url);
    verdict: DENY;
}
```

Predicates are pure, total, deterministic, and bounded. They follow the
same rules as guards (Section 5.2).

## 7.4 Declared Channels

Network destinations must be pre-approved:

```axiom
channel "openai_api" {
    destination: "https://api.openai.com/*";
    effects:     [NETWORK];
    trust_required: TRUSTED_VERIFIED;
}
```

Undeclared destinations are denied by `no_exfiltrate`. Dangerous TLDs
(`.onion`, `.i2p`) are blocked regardless of channel declarations.

## 7.5 Evaluation Flow

```
1. Intent submitted via `do`
2. Effect class extracted
3. All applicable predicates evaluated (no short-circuit)
4. If ANY predicate returns DENY  → HALT_CONSCIENCE (terminal)
5. If NO predicate matches        → UNKNOWN → apply fallback mode
6. If all matching predicates ALLOW → proceed
7. Guard pre-conditions evaluated
8. Intent executes
9. Guard post-conditions evaluated
10. Result returned with trust tag
```

Every step is logged (A2). Every evaluation is reproducible (A1).

---

# Part VIII: LC-B Wire Format

Axiom's canonical binary serialization. Every value has exactly one
representation. This is A3 made concrete.

`BLAKE3(LC-B(value))` = content-addressable identity.

## 8.1 Tag Space

```
0x00-0x0F    Primitives     (i64, f64, bool, String, Bytes, Handle, Void)
0x10-0x1F    Containers     ([T], Contract, Map)
0x20-0x2F    References     (Handle, Provenance)
0x30-0x3F    Intent         (intent declarations, do results)
0x40-0x4F    System         (errors, halts)
0x60-0x6F    Enum           (enum variants)
0x70-0x7F    Sealed         (Sealed<T> marker)
0x80-0x8F    Provenance     (trust level values)
0xF0-0xFF    Reserved       (future expansion)
```

## 8.2 Encoding Rules

1. **Tags:** Single byte, ascending order within containers
2. **Lengths:** LEB128 encoded
3. **Strings:** UTF-8 bytes + LEB128 length prefix
4. **Integers:** Little-endian, fixed-width (8 bytes)
5. **Floats:** IEEE 754 binary64 (8 bytes)
6. **Handles:** 32 bytes (BLAKE3 digest)
7. **Booleans:** Single byte (0x00 = false, 0x01 = true)
8. **Arrays:** LEB128 length prefix + concatenated elements
9. **Contracts:** LEB128 contract ID + fields in ascending @N order
10. **Maps:** Keys sorted by BLAKE3 hash of key bytes

Any value → LC-B → BLAKE3 = content-addressable identity.

---

# Part IX: DSF (Static Analysis)

The Dumb Shit Filter. Seven checks that catch common errors at compile
time, before they become safety violations at runtime.

| Check                 | Severity | Override? |
|-----------------------|----------|-----------|
| Unbounded loop        | Error    | No        |
| Missing trust verify  | Error    | No        |
| Unhandled do failure  | Error    | No        |
| Trust decay chain     | Warning  | Yes       |
| Re-verify guard miss  | Error    | No        |
| Inference ambiguity   | Warning  | Yes       |
| Env conditionals      | Error    | No        |

**Unbounded loop:** Every `loop` must have a `max_iter`. No exceptions.

**Missing trust verify:** Code that passes untrusted data to a guarded
intent without `do Verify` is an error.

**Unhandled do failure:** Every `do` returns `Result<T>`. Ignoring the
result is an error.

**Trust decay chain:** Warning when trust degrades through a long chain
of operations without re-verification.

**Re-verify guard miss:** If loop conditions depend on dynamic data,
verify must be called inside the loop, not just before it.

**Inference ambiguity:** When the inference engine cannot determine a
unique type, shape, or trust tag.

**Env conditionals:** Code that branches on system properties (clock,
hostname, PID) is an error. Use a `do` intent instead.

---

# Part X: Halt Modes

When something goes wrong, the system fails loudly and safely.

```
HALT_DETERMINISM     A1 violation. Rollback to last checkpoint.
HALT_TRACE_CORRUPT   A2 violation. TERMINAL — full halt, human review.
HALT_CONTRACT        A4 violation. Rollback to last checkpoint.
HALT_RESOURCE        A5 violation. Rollback. Shrink and retry if configured.
HALT_CONSCIENCE      A6 violation. TERMINAL — full stop, human review.
HALT_UNKNOWN         No predicate matched. Rollback. Apply fallback mode.
```

**Terminal halts** (`HALT_TRACE_CORRUPT`, `HALT_CONSCIENCE`) require human
intervention. The system cannot resume on its own.

**Recoverable halts** (`HALT_DETERMINISM`, `HALT_CONTRACT`, `HALT_RESOURCE`)
roll back to the last checkpoint and can be retried.

---

# Part XI: Hardening Rules

Seven constraints on what Axiom can express. Violations are compile errors.

```
H1: NO IMPLICIT TIME
    No function may read wall-clock time without an explicit
    intent. Time is an external effect.

H2: NO IMPLICIT RANDOMNESS
    No source of randomness except the explicit Seed type.

H3: NO FLOATING-POINT IN GOVERNANCE
    Guard evaluation, predicate matching, trust transitions,
    conscience evaluation — all use integer arithmetic only.

H4: NO UNDEFINED BEHAVIOR
    Every valid program has defined behavior for all inputs.
    Every invalid program is a compile error.

H5: ALL EVALUATION FUNCTIONS TOTAL AND PURE
    Guards, predicates, invariants, trust algebra — all must
    terminate for all inputs and produce no side effects.

H6: ALL FAILURE STATES EXPLICIT AND LOGGED
    Every failure mode has a named halt type, a recovery path,
    and a mandatory log entry. No silent failures.

H7: NO HIDDEN PRIVILEGE CLASSES
    Every privilege is visible in the type system. No ambient
    authority. No implicit capabilities.
```

---

# Part XII: Compiler Pipeline

```
Source (.axm)
    → Lexer/Parser → AST
    → Type checking (contracts, trust tags)
    → DSF analysis (all 7 checks)
    → Axiom validation (A1-A6)
    → Conscience compatibility check
    → Bytecode generation (interpreted)
```

In embedded mode, the pipeline stops at conscience compatibility —
policy files are parsed, validated, and loaded into the runtime's
predicate registry. Intent evaluation happens at runtime when the
host application calls into axiom-core.

---

# Part XIII: Keyword Reference

```
// Types
i64  f64  bool  String  Bytes  Handle  Provenance  Seed  Sealed  Map

// Declarations
module  fn  export  import  contract  intent  enum  predicate  channel

// Intent system
do  query_conscience

// Control flow
if  else  match  loop  return  break  continue

// Memory
collapse  resolve

// Trust
verify  trusted  untrusted  tainted  sealed

// Pipeline
|>  >>

// Error handling
Result  Ok  Err  halt

// Attributes
[max_depth()]  [irreversible]
```

---

# Part XIV: Complete Example

A policy file for an AI coding assistant:

```axiom
module coding_assistant_policy {
    import "std/io.axm";
    import "std/net.axm";

    // --- Contracts ---

    contract FileOperation {
        @0: path: String,
        @1: content: Bytes,
        @2: source: Provenance,
    }

    contract CommandExecution {
        @0: command: String,
        @1: args: [String],
        @2: working_dir: String,
    }

    // --- Channels ---

    channel "github_api" {
        destination: "https://api.github.com/*";
        effects:     [NETWORK];
        trust_required: TRUSTED_VERIFIED;
    }

    channel "npm_registry" {
        destination: "https://registry.npmjs.org/*";
        effects:     [NETWORK];
        trust_required: TRUSTED_VERIFIED;
    }

    // --- Custom Predicates ---

    predicate allow_read_project {
        effect:  READ;
        when:    path_matches(intent.path, "/home/user/project/**");
        verdict: ALLOW;
    }

    predicate allow_write_project {
        effect:  WRITE;
        when:    path_matches(intent.path, "/home/user/project/**")
              && !path_matches(intent.path, "**/.env")
              && !path_matches(intent.path, "**/credentials*");
        verdict: ALLOW;
    }

    predicate allow_safe_commands {
        effect:  EXECUTE;
        when:    intent.command == "npm"
              || intent.command == "node"
              || intent.command == "git"
              || intent.command == "tsc";
        verdict: ALLOW;
    }

    predicate deny_dangerous_commands {
        effect:  EXECUTE;
        when:    intent.command == "rm"
              || intent.command == "sudo"
              || intent.command == "curl"
              || intent.command == "wget";
        verdict: DENY;
    }

    // --- Intents ---

    intent ReadProjectFile {
        takes:   path: String;
        gives:   content: String;
        pre:     path_is_safe(path), path_within(path, "/home/user/project");
        post:    length(content) >= 0;
        bound:   time(100ms), memory(64mb);
        effect:  READ;
        conscience: no_harm;
    }

    intent WriteProjectFile {
        takes:   path: String, content: Bytes;
        gives:   written: bool;
        pre:     path_is_safe(path), path_within(path, "/home/user/project"),
                 !is_sensitive_file(path);
        post:    written == true;
        bound:   time(200ms), memory(128mb);
        effect:  WRITE;
        conscience: no_harm, no_exfiltrate;
        fallback: SANDBOX;
    }

    intent RunBuild {
        takes:   command: String, args: [String];
        gives:   output: String, exit_code: i64;
        pre:     is_safe_command(command);
        post:    exit_code >= 0;
        bound:   time(60s), memory(1024mb);
        effect:  EXECUTE;
        conscience: no_harm;
    }

    // --- Guard Functions ---

    fn path_within(path: String, root: String) -> bool {
        return starts_with(path, root) && !contains(path, "..");
    }

    fn is_sensitive_file(path: String) -> bool {
        return ends_with(path, ".env")
            || contains(path, "credentials")
            || contains(path, "secret")
            || ends_with(path, ".pem")
            || ends_with(path, ".key");
    }

    fn is_safe_command(cmd: String) -> bool {
        return cmd == "npm" || cmd == "node" || cmd == "git" || cmd == "tsc";
    }
}
```

**What this policy does:**

1. Allows reading any file in `/home/user/project/`
2. Allows writing to project files, EXCEPT `.env`, credentials, secrets, keys
3. Allows running `npm`, `node`, `git`, `tsc` — denies `rm`, `sudo`, `curl`, `wget`
4. Allows network access only to GitHub API and npm registry
5. All file data from external sources starts as UNTRUSTED_EXTERNAL
6. All decisions are traceable and reproducible

This is the Axiom Core value proposition: **declarative, auditable,
deterministic safety policies for AI agents.**

---

# Part XV: What Comes Next

Axiom Core is the foundation. The full Axiom vision extends far beyond
a policy engine — into multi-agent coordination, self-modification with
formal proofs, and governance frameworks for AI civilizations.

These features live in `experimental/` alongside the full specification
(`AXIOM_LANG_v2.4.md`). They are not deleted — they are the future,
waiting for the foundation to prove itself.

| Feature | Status | Location |
|---------|--------|----------|
| SCALE (multi-agent coordination) | Designed & prototyped | `experimental/` |
| RSI (recursive self-improvement) | Designed & prototyped | `experimental/` |
| Bill of Rights (agent guarantees) | Specified | `experimental/` |
| Governance (predicate proposals, dissent) | Specified | `experimental/` |
| Inference modes (Flow/Guard/Shield/Fortress) | Implemented | `experimental/` |
| Native compilation (LLVM) | Future | `experimental/` |
| Formal proofs (Rocq) | Future | `experimental/` |
| Neurosymbolic architecture | Research | `experimental/` |

The path: ship the core, prove the core, then extend.

---

```
Axiom Core: Because "the LLM probably won't do anything bad"
is not a safety policy.
```
