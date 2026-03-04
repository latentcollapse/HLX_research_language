# HLX Research Language

HLX is a programming language I've been building where conscience and governance are part of the syntax itself, not something you bolt on after the fact. The idea started as an experiment in recursive self-improvement and kind of evolved from there into something I think is genuinely interesting.

The short version: most languages let you write what a program does. HLX also lets you write what it's allowed to do — and that's enforced at the runtime level, not just as a comment or a convention.

---

## What it looks like

```hlx
recursive agent Thinker {
    latent hypothesis: Tensor[512]

    cycle outer(H: 3) {
        cycle inner(L: 6) {
            details = refine(details, hypothesis + input)
        }
        hypothesis = consolidate(hypothesis, details)
    }

    govern {
        effect: READ | WRITE
        conscience: [path_safety, no_exfiltrate]
    }

    halt when confidence > 0.95 or steps >= 16
}
```

Agents have persistent state across cycles (`latent`), run at multiple scales (`H`, `L`), and carry their own governance rules inline. If a conscience predicate fails, the action doesn't happen — it's not a check you can forget to write, it's part of how the agent is defined.

---

## Getting started

You'll need the Rust toolchain. After that it's just:

```bash
git clone https://github.com/latentcollapse/HLX_research_language
cd HLX_research_language
cargo build --release

./target/release/hlx-run your_program.hlx
```

If you want to see what's happening under the hood:

```bash
./target/release/hlx-run --debug your_program.hlx
```

That'll show you a trace with named variables alongside their register values, which is useful when you're trying to figure out why something isn't doing what you expected.

A basic program just to make sure everything's working:

```hlx
fn main() -> String {
    let greeting: String = "Hello from HLX";
    return greeting;
}
```

---

## The governance layer

The part I'm probably most proud of is APE — the Axiom Policy Engine. It's a small embeddable policy engine you can drop into any Rust project. You write a policy file describing what your agent is and isn't allowed to do, and APE checks every intent before it executes.

```axm
module security_policy {
    intent WriteFile {
        takes:   path: String, content: String;
        gives:   success: bool;
        effect:  WRITE;
        conscience: path_safety, no_exfiltrate;
    }
}
```

```rust
let engine = AxiomEngine::from_file("policy.axm")?;
let verdict = engine.verify("WriteFile", &[("path", "/tmp/output.txt")])?;

if verdict.allowed() {
    // proceed
} else {
    eprintln!("blocked: {}", verdict.reason().unwrap());
}
```

Six core governance properties are mechanically verified in Rocq (Coq) — things like determinism, that trust can only increase and never be downgraded, and that every execution path terminates with a verdict. The red team suite has 15 attack vectors and all 15 are blocked.

Three modes depending on how strict you want to be:

- **Flow** — good for prototyping, infers trust where it can
- **Guard** — trust has to be explicit, which is what I run in production
- **Arx** — everything must be explicitly declared, for formal verification contexts

---

## The bond protocol

HLX also has a protocol for bonding a symbolic reasoning layer to a local GGUF model. The idea is that the symbolic layer — which holds rules, memory, conscience predicates — governs and persists across conversations, while the GGUF does the language generation. You can swap the underlying model and the symbolic layer stays intact because identity lives there, not in the weights.

```
HELLO  →  symbolic layer presents its capabilities
SYNC   →  model responds with context window, vocab size
BOND   →  corpus rules and memory injected as governing context
READY  →  the combined system enters the inference loop
```

---

## Where it's at

This is v0.9 — I'd call it MVP-ready. Language feature completeness is around 95%, 313 tests passing across the workspace, zero critical or warning-level debt. Phases 1 through 3 of the roadmap are done. The stuff that's left is either platform testing (Windows/Mac) or features that are deliberately deferred to a later version.

It's not production software in the sense that I'd tell you to run it in a hospital or whatever. But it's solid enough to build real things with, and I'm actively using it for research.

---

## License

AGPL v3. If you build on this and ship it, your changes stay open. That was a deliberate choice.

---

*Active research project. Things are moving fast.*
