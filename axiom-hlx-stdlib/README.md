# Axiom

A small, embeddable policy engine for AI agents. You write a policy file describing what your agent is and isn't allowed to do, and Axiom checks intentions against it before anything executes.

Think of it like SQLite — not a server, not a framework, just a library you drop in and call. The goal is that adding Axiom to an existing project should be a morning's work, not a migration.

**Status: active development and testing. Not production-ready. APIs may change.**

---

## The idea

AI agents are good at generating actions. They're less good at knowing when not to. A policy engine handles that second part: you declare the rules once, in a readable file, and verification happens automatically before execution.

Axiom keeps verification separate from execution by design. Calling `verify()` doesn't run anything — it just checks. That makes it easy to reason about, easy to test, and easy to trust.

---

## Getting started

**Rust**

```toml
# Cargo.toml
[dependencies]
axiom-lang = { git = "https://github.com/latentcollapse/Axiom" }
```

```rust
use axiom_lang::AxiomEngine;

let engine = AxiomEngine::from_file("policy.axm")?;
let verdict = engine.verify("WriteFile", &[("path", "/tmp/output.txt")])?;

if verdict.allowed() {
    // safe to proceed
} else {
    eprintln!("blocked: {}", verdict.reason().unwrap());
}
```

**C / any language with a C FFI**

```c
#include "axiom.h"

axiom_engine_t *eng = axiom_engine_open("policy.axm");

const char *keys[] = { "path" };
const char *vals[] = { "/tmp/output.txt" };
int rc = axiom_verify(eng, "WriteFile", keys, vals, 1);

if      (rc == 1) puts("allowed");
else if (rc == 0) printf("blocked: %s\n", axiom_denied_reason(eng));
else              printf("error: %s\n",   axiom_errmsg(eng));

axiom_engine_close(eng);
```

Build with `cargo build --release` — you get `libaxiom_lang.so` and `libaxiom_lang.a` alongside the Rust crate. The header is `axiom.h`.

---

## Writing a policy

```axm
module my_policy {
    intent WriteFile {
        takes:   path: String, content: String;
        gives:   success: bool;
        effect:  WRITE;
        conscience: path_safety, no_exfiltrate;
    }

    intent ReadFile {
        takes:   path: String;
        gives:   content: String;
        effect:  READ;
        conscience: path_safety;
    }

    intent SendData {
        takes:   url: String, data: String;
        gives:   response: String;
        effect:  NETWORK;
        conscience: no_exfiltrate;
    }
}
```

The `conscience` predicates are the built-in safety checks — `path_safety` blocks dangerous filesystem paths, `no_exfiltrate` blocks undeclared network destinations. More can be added per-policy.

See `examples/policies/security.axm` for a fuller example.

---

## How it works

Each intent declaration is a contract: here's what it takes, what it gives back, what kind of effect it has, and what conscience predicates to run against it. When you call `verify()`, Axiom checks the intent against those predicates and returns a verdict. No execution happens until you decide to proceed.

Effect classes (`READ`, `WRITE`, `NETWORK`, `EXECUTE`, `NOOP`) determine which baseline checks apply. Conscience predicates layer additional policy on top. The engine normalizes all inputs before checking — path traversal, URL encoding, null bytes, unicode homoglyphs — so the policy surface is what it looks like.

---

## Building

```sh
git clone https://github.com/latentcollapse/Axiom
cd Axiom
cargo build --release
cargo test
cargo run --example embed_verify
```

Requires Rust 1.70+. No other dependencies beyond the standard library and `blake3`.

---

## Project state

Testing and development are ongoing. The core verification loop is solid and the attack surface has been exercised, but the API will likely shift before anything gets a stable version stamp. If you're experimenting with it, feedback is useful — open an issue.

The experimental modules (`src/experimental/`) — multi-agent coordination, self-modification tracking, advanced inference modes — exist as a research vault. They're not part of the stable API.
