# HLX — Governed Inference Runtime for Local AI

> *"You're not building a fast inference engine — llama.cpp already exists.
> You're building a governed inference engine."*

HLX is a runtime that bonds a **symbolic AI** (Klyntar corpus) to a local **GGUF language model**, creating a unified neurosymbolic system governed by conscience predicates, recursive reasoning cycles, and safe self-modification.

---

## What This Actually Is

Most local AI setups: `model + system prompt`. That's it.

HLX introduces a **symbiote** — a persistent symbolic layer that attaches to model weights via a formal bond protocol, injects structured knowledge and governing rules into every inference, and persists state across conversations and model swaps.

```
Without HLX:   GGUF model → responds

With HLX:      Klyntar corpus (rules, memory, conscience)
                      ↓  bond protocol
               GGUF model → governed, persistent, recursive
```

The model doesn't change. The weights don't change. But what it is — does.

---

## Quick Start

```bash
# Clone HLX
git clone https://github.com/latentcollapse/hlx-compiler
cd hlx-compiler

# Build the bond CLI
cd hlx-bond && cargo build --release

# Bond a local GGUF to a Klyntar corpus
./target/release/hlx-bond ~/models/Qwen3-0.6B.gguf \
    --corpus ~/klyntar/corpus.db
```

```
╔══════════════════════════════════════════════════╗
║  HLX Bond Protocol v0.1 — Native GGUF + Klyntar  ║
╚══════════════════════════════════════════════════╝

[HELLO] Initiating bond with model: Qwen3-0.6B
[HELLO] Capabilities offered: tensor_ops, image_io, governance, rsi, scale
[SYNC]  Bond accepted — synchronising state...
[BOND]  Corpus context injected — neurosymbolic link forming...
[READY] Bond complete. Symbiote is active.

you> Hello. What are you?
hlx> Hello! I'm a language model designed to help you with questions
     and tasks. How can I assist you today?
```

**No Python. No llama.cpp. No ollama. No running daemons.** One binary, one GGUF, one corpus.

---

## The Bond Protocol

The handshake between Klyntar (symbolic) and the GGUF model (neural):

```
HELLO  →  Symbiote presents capabilities to the model
SYNC   →  Model responds: accepted, context window, vocab size
BOND   →  Corpus rules + memory injected as governing context
READY  →  Unified NS-AI enters conversation loop
```

The bond is **persistent** — responses are stored back into the corpus memory, so the symbiote grows with every conversation. Swap the underlying model; the symbiote survives intact.

---

## Klyntar — The Symbiote

[Klyntar](https://github.com/latentcollapse/klyntar) is the symbolic half of the system. Think of it as a `.gguf` file for neurosymbolic AI — a pre-seeded package of:

- **Rules** — governing constraints with confidence scores
- **Memory** — persistent conversation history
- **Documents** — ingested knowledge base
- **Conscience predicates** — alignment primitives that govern what the system will and won't do

`pip install klyntar` gives you an inert SAI.
`pip install klyntar` + `hlx-bond` gives you a thinking, governed system.

---

## TRM Recursive Reasoning

Inspired by [Token Recursive Machines](https://github.com/latentcollapse/HLX/tree/main/TinyRecursiveModels-main), HLX implements H-cycles — recursive reasoning passes where each cycle feeds its output back as input context for the next:

```bash
# 3 recursive reasoning passes per message
hlx-bond model.gguf --corpus corpus.db --h-cycles 3
```

The TRM paper showed 7M parameter models with recursive cycles outperforming larger flat models. HLX applies this at the system level, not the weight level — the recursion is in the bond, not the architecture.

---

## HLX as a Language

Beyond the bond CLI, HLX is a self-hosting programming language where **conscience, coordination, and self-modification are syntactic primitives**:

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

When conscience predicates are first-class syntax, any model trained on HLX code absorbs "conscience" as a fundamental concept — alignment by osmosis, not by RLHF.

---

## Architecture

```
hlx-bond (Rust binary)
├── candle 0.9 — pure Rust GGUF inference (quantized_qwen3)
├── hlx-runtime — bond protocol, governance, RSI, SCALE, tensor ops
└── Klyntar corpus.db (SQLite) — rules, memory, documents

HLX Language Runtime
├── hlx-runtime/src/
│   ├── vm.rs          — bytecode VM
│   ├── bond.rs        — HELLO→SYNC→BOND→READY protocol
│   ├── governance.rs  — conscience predicate engine
│   ├── rsi.rs         — recursive self-improvement pipeline (3-gate)
│   ├── scale.rs       — multi-agent coordination (barriers, consensus)
│   ├── tensor.rs      — tensor primitives + image/audio I/O
│   └── agent.rs       — agent lifecycle (spawn/halt/dissolve)
├── backends/
│   ├── llvm/          — native code generation via LLVM
│   └── vulkan/        — GPU compute via Vulkan
└── Axiom-main/        — policy verification engine (FFI target)
```

---

## What's Working

| Component | Status |
|-----------|--------|
| Native GGUF inference (candle, pure Rust) | ✅ |
| Bond protocol (HELLO→SYNC→BOND→READY) | ✅ |
| Klyntar corpus injection (rules + memory) | ✅ |
| BPE tokenizer extracted from GGUF metadata | ✅ |
| TRM H-cycles (recursive reasoning) | ✅ |
| Governance engine (conscience predicates) | ✅ |
| RSI pipeline (3-gate self-modification) | ✅ |
| SCALE coordination (barriers, consensus) | ✅ |
| Agent lifecycle (spawn/halt/dissolve) | ✅ |
| Tensor ops + image/audio I/O | ✅ |
| LLVM JIT backend | ✅ |
| Vulkan GPU backend | ✅ |
| HLX self-hosting compiler | 🔄 In progress |
| LoRA weight-level RSI (Phase 2 bond) | 🔮 Future |

---

## Supported Models

Any Qwen3 GGUF works. Tested:

| Model | Size | Notes |
|-------|------|-------|
| Qwen3-0.6B Q8_K_XL | 844MB | Confirmed working |
| Qwen3-1.7B Q8_0 | 1.8GB | |
| Qwen3-4B Q4_K_M | 2.4GB | |

Other GGUF architectures (LLaMA, Mistral, etc.) require a corresponding `quantized_*` module — contributions welcome.

---

## Design Philosophy

- **Governed, not fast** — llama.cpp handles throughput. HLX handles what the system is *allowed to think*.
- **Conscience as syntax** — predicates aren't comments or system prompts. They're grammar.
- **Structure beats scale** — recursive cycles on small models beat flat inference on large ones.
- **The symbiote survives** — swap the model, keep the corpus. Identity lives in the symbolic layer.

---

## Roadmap

**Now (v0.1.3):** Phase 1 bond working. Symbolic corpus governs inference via context injection.

**Near term:** Corpus seeding experiments. Measure whether symbolic rules provably change model behavior. Multi-model swarm tests.

**Later:** Phase 2 bond — LoRA fine-tuning driven by RSI pipeline. The symbiote stops governing the weights and starts reshaping them. That's the paper.

---

## Related

- **[Klyntar](https://github.com/latentcollapse/klyntar)** — the symbolic AI package (`pip install klyntar`)
- **[Axiom](./Axiom-main)** — policy verification engine (FFI integration)
- **[TinyRecursiveModels](./TinyRecursiveModels-main)** — the TRM paper (theoretical foundation)

---

## License

Apache-2.0

---

*HLX is pre-research software. The bond works. The experiments are starting.*
