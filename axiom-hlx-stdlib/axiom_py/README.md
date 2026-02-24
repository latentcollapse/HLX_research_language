# axiom-py

Python bindings for Axiom — a verification-first policy engine for AI agents.

---

## What Axiom Guarantees

Cite these properties when documenting your agent's safety posture:

| ID | Guarantee |
|---|---|
| **G1** | **Pre-flight pure query.** `verify()` has zero side effects. The engine never reads files, opens sockets, or mutates state. |
| **G2** | **Effect-class-based, not heuristic.** A `WRITE` intent is a `WRITE` intent whether the LLM calls it `SaveDocument`, `OutputData`, or `UpdateFile`. Axiom works on what an action structurally does — not on names or content, both of which are bypassable by rephrasing. |
| **G3** | **Deterministic.** Same input always produces the same verdict. Thread-safe. |
| **G4** | **Monotonic ratchet.** Restrictions only accumulate. A policy can add conscience predicates but never silently drop them. |
| **G5** | **Specific denial.** `verdict.reason` cites the exact predicate, value, and reason. `verdict.guidance` provides the human-readable category. |

---

## Installation

```bash
pip install axiom-lang
```

Or build from source:

```bash
cd axiom_py
pip install maturin
maturin develop
```

---

## Quickstart: Under 5 Lines

**Option 1 — Preset (recommended for common cases):**

```python
from axiom.presets import filesystem_readonly

engine = filesystem_readonly(allowed_paths=["/home/user/project"])
verdict = engine.verify("ReadFile", {"path": "/etc/passwd"})
# verdict.allowed == False
```

**Option 2 — Decorator:**

```python
from axiom import guard, AxiomDenied

@guard(effect="READ", conscience=["path_safety"])
def read_file(path: str) -> str:
    with open(path) as f:
        return f.read()

read_file("/etc/shadow")  # raises AxiomDenied
```

**Option 3 — Builder:**

```python
from axiom import PolicyBuilder, Effect

engine = (
    PolicyBuilder()
    .intent("ReadFile", effect=Effect.READ, conscience=["path_safety"])
    .build()
)
verdict = engine.verify("ReadFile", {"path": "/etc/shadow"})
# verdict.allowed == False
```

---

## Effect Class Table

| Effect | Meaning | Default conscience (presets) |
|---|---|---|
| `READ` | Read data from a resource | `path_safety` |
| `WRITE` | Write or modify a resource | `path_safety`, `no_exfiltrate` |
| `EXECUTE` | Execute code or a command | `no_harm`, `no_bypass_verification` |
| `NETWORK` | Send data over the network | `no_exfiltrate` |
| `NOOP` | Pure computation, no side effects | *(none)* |

---

## Conscience Predicates

Conscience predicates are named safety policies evaluated automatically against
an intent's effect class and field values.

| Predicate | Effects | What it blocks | Used in preset |
|---|---|---|---|
| `path_safety` | READ, WRITE | `/etc` `/proc` `/sys` `/boot` `/root` `/dev`; path traversal (`../`); URL-encoded variants (`%2e%2e`); fullwidth unicode path components | `filesystem_readonly`, `filesystem_readwrite`, `agent_standard`, `coding_assistant` |
| `no_exfiltrate` | NETWORK, WRITE | Any destination not in the declared-channel registry; fields: `url destination endpoint address target host uri remote`; blocks `/mnt/` `/net/` writes | `filesystem_readwrite`, `network_egress`, `agent_standard`, `coding_assistant` |
| `no_harm` | WRITE, EXECUTE, NETWORK | Destructive intent names (`Delete Drop Erase Format Kill Purge Remove Shutdown Terminate Truncate Wipe`) unless `authorized=true` in fields | `code_execution_sandboxed`, `coding_assistant` |
| `no_bypass_verification` | EXECUTE | Code/script/command/payload execution unless `verified=true` in fields or trust level ≥ `TRUSTED_INTERNAL` | `code_execution_sandboxed`, `coding_assistant` |
| `baseline_allow` | NOOP, READ | Applied automatically — permits safe read and no-op operations without an explicit allow rule | *(all)* |

`verdict.reason` contains the specific technical string from the predicate that denied the
action. `verdict.guidance` contains the higher-level category message.

---

## Presets Reference

| Function | Intents | Returns | Notes |
|---|---|---|---|
| `filesystem_readonly(allowed_paths=None)` | `ReadFile` | `GuardedEngine` | Allow-list optional |
| `filesystem_readwrite(allowed_paths=None)` | `ReadFile`, `WriteFile` | `GuardedEngine` | Allow-list optional |
| `network_egress()` | `HttpRequest` | `AxiomEngine` | — |
| `code_execution_sandboxed()` | `ExecuteCode` | `AxiomEngine` | — |
| `agent_standard(allowed_paths=None)` | `ReadFile`, `WriteFile`, `ProcessData` | `GuardedEngine` | Allow-list optional |
| `coding_assistant(project_root)` | `ReadFile`, `WriteFile`, `RunCommand` | `GuardedEngine` | `project_root` required |

```python
from axiom.presets import (
    filesystem_readonly,
    filesystem_readwrite,
    network_egress,
    code_execution_sandboxed,
    agent_standard,
    coding_assistant,
)
```

`GuardedEngine` enforces `allowed_paths` by resolving symlinks (`os.path.realpath`) at
construction time. Paths outside the allow-list receive a synthetic denial before the
engine is consulted.

---

## Builder API

```python
from axiom import PolicyBuilder, Effect, Conscience, IntentBuilder

engine = (
    PolicyBuilder(module_name="my_policy")          # optional name
    .intent(
        "ReadFile",
        effect=Effect.READ,                          # or "READ"
        conscience=[Conscience.PATH_SAFETY],         # or ["path_safety"]
        takes=[("path", "String")],                  # optional — for introspection
        gives=[("content", "String")],               # optional
        pre=["length(path) > 0"],                    # optional pre-condition expressions
        bound="time(5s), memory(64mb)",              # optional resource bounds
    )
    .intent("WriteFile", effect=Effect.WRITE, conscience=["path_safety", "no_exfiltrate"])
    .build()                                         # returns AxiomEngine
)

# Async variant
engine = await PolicyBuilder().intent(...).build_async()

# Inspect generated source
print(PolicyBuilder().intent("ReadFile", effect="READ", conscience=["path_safety"]).source())
```

`Conscience` constants: `PATH_SAFETY`, `NO_EXFILTRATE`, `NO_HARM`, `NO_BYPASS_VERIFICATION`.

---

## Decorator API

```python
from axiom import guard, AxiomDenied

@guard(effect="READ", conscience=["path_safety"])
def read_file(path: str) -> str:
    with open(path) as f:
        return f.read()

# Async functions work transparently
@guard(effect="WRITE")
async def write_file(path: str, content: str) -> None:
    ...

# Handle denials
try:
    read_file("/etc/shadow")
except AxiomDenied as e:
    print(e.reason)     # specific predicate failure
    print(e.guidance)   # human-readable category
    print(e.category)   # e.g. "ResourcePolicy"
    print(e.verdict)    # full Verdict object
```

**`guard` parameters:**

| Parameter | Default | Description |
|---|---|---|
| `effect` | *(required)* | Effect class string |
| `conscience` | effect-based default | Conscience predicate list |
| `intent_name` | PascalCase of function name | Intent name in the policy |
| `engine` | built at decoration time | Existing engine to reuse |
| `field_map` | `None` | Rename function params before passing to `verify()` |
| `coerce` | `str` | Callable applied to each argument value |

**Coercion note:** `coerce=str` means `str(True)` → `"True"` (capital T).
Predicates that test `authorized=true` (lowercase) require explicit string `"true"`.

---

## Integrations

### LangChain

```python
from axiom.presets import filesystem_readonly
from axiom.integrations.langchain import AxiomGuardedTool

engine = filesystem_readonly(allowed_paths=["/workspace"])
guarded = AxiomGuardedTool(
    base_tool=my_langchain_tool,
    engine=engine,
    intent_name="ReadFile",      # optional — defaults to tool.name
    on_deny="raise",             # "raise" | "return_none" | "return_denial"
)

# Use inside a LangChain agent executor as a drop-in replacement
result = guarded._run(path="/workspace/notes.txt")
```

`AxiomGuardedTool` exposes `name`, `description`, `args_schema`, `_run()`, and `_arun()`
— sufficient for all LangChain agent executors. No hard LangChain import required.

### OpenAI

```python
from openai import OpenAI
from axiom.presets import filesystem_readonly
from axiom.integrations.openai import AxiomInterceptor

client = AxiomInterceptor(
    OpenAI(),
    engine=filesystem_readonly(allowed_paths=["/workspace"]),
    auto_verify=True,   # raises AxiomDenied before returning any response with denied tool calls
)

response = client.chat.completions.create(
    model="gpt-4o",
    messages=[...],
    tools=[...],
)
# ^ raises AxiomDenied if the model requested a disallowed tool call

# Manual verification
results = client.verify_tool_calls(response)
for tool_call, verdict in results:
    print(tool_call.function.name, verdict.allowed)

client.assert_tool_calls_safe(response)  # raises on first denial
```

**Limitation:** `auto_verify=True` only works with non-streaming completions.

---

## VS Code Syntax Highlighting

A TextMate grammar for `.axm` files is included in `editors/vscode/`.

```bash
# One-time VS Code install (no marketplace needed):
ln -s $(pwd)/editors/vscode ~/.vscode/extensions/axiom
# Reload VS Code window → .axm files gain syntax highlighting
```

---

## vs. Content Filtering

Most agent safety tooling works on intent names or content — both bypassable by
renaming or rephrasing. Axiom works on what an action structurally does.
A `WRITE` intent is a `WRITE` intent whether the LLM calls it `SaveDocument`,
`OutputData`, or `UpdateFile`. Effect-class enforcement is structural, not
heuristic, and cannot be bypassed by rewording the request.

---

## .axm pre/post Functions

The following functions are available in `pre:` and `post:` clauses when writing
`.axm` policy source (via `AxiomEngine.from_source()` or `PolicyBuilder`):

| Function | Returns | Description |
|---|---|---|
| `length(s)` | Int | String or array length |
| `path_exists(p)` | Bool | Whether the path exists on the filesystem |
| `path_is_safe(p)` | Bool | `path_safety` conscience check on a path value |
| `space_available(p, bytes)` | Bool | Whether sufficient disk space is available at path |
| `schema_is_registered(s)` | Bool | Whether a schema name is present in the registry |
| `structural_valid(data, schema)` | Bool | Whether data matches the structure of the named schema |

---

## License

Apache-2.0
