# HLX — A Language for Recursive Intelligence

> "Physics gives us freedom through constraints. AI should have a similarly bound world."

## What Is HLX?

HLX is a self-hosting programming language designed around a single insight: **recursive intelligence can be a syntactic primitive**.

Instead of building AI agents and bolting on safety afterward, HLX makes **conscience, coordination, and self-modification** first-class language constructs. The syntax reads like its own documentation—designed for both human readability and dense training signal for AI models.

## Why HLX?

### The Python Insight

Python dominates ML not just because of its ecosystem, but because its English-like syntax creates a tight alignment between code and natural language. This produces denser training signal for AI models—code and explanations share vocabulary.

HLX applies this principle to **safety and recursive intelligence**. When conscience predicates are first-class syntax, any model trained on HLX code absorbs "conscience" as a fundamental primitive, like it absorbs "if" or "for".

### The Axiom Connection

HLX is designed to integrate with [Axiom](../Axiom-main), a verification-first policy engine for AI agents. Through FFI, HLX agents can have their conscience predicates enforced by Axiom at runtime:

```hlx
recursive agent Thinker {
    govern {
        effect: WRITE
        conscience: [path_safety, no_exfiltrate]
    }
}
```

The boundary between "building the agent" and "securing the agent" dissolves. One cognitive frame.

## Core Concepts

### Recursive Agents

Agents that refine their own state through cycles (inspired by TRM):

```hlx
recursive agent Thinker {
    latent hypothesis: Tensor[512]
    latent details: Tensor[512]
    
    cycle outer(H: 3) {
        cycle inner(L: 6) {
            details = refine(details, hypothesis + input)
        }
        hypothesis = consolidate(hypothesis, details)
    }
    
    halt when confidence > 0.95 or steps >= 16
}
```

### SCALE — Coordination

Multiple agents synchronizing at barriers:

```hlx
scale cluster Swarm {
    agents: [Thinker; 5]
    
    sync at barrier consensus {
        consensus: cross_model_family
        aggregate hypothesis: weighted_mean(by: confidence)
    }
}
```

### Governance — Conscience as Syntax

Safety baked into the agent's nature:

```hlx
govern {
    effect: READ | WRITE | NETWORK
    conscience: [path_safety, no_exfiltrate, rate_limit]
    trust: TRUSTED_INTERNAL
}
```

### Self-Modification — Safe Evolution

Agents that can propose and apply changes through three gates:

```hlx
modify self {
    gate proof { verify: no_infinite_loops }
    gate consensus { quorum: Swarm.agents; threshold: 2/3 }
    gate human { trigger: complexity_delta > 100 }
    budget { complexity: 1000; backoff: exponential }
}
```

### Dissolvable Agents

Intelligence that forms, executes, and dissolves:

```hlx
dissolvable agent Analyzer {
    lifetime: task_completion
    inherit: parent.hypothesis
    
    action analyze(data: Dataset) -> Report { ... }
    
    on_dissolve {
        archive: report -> parent.memory
    }
}
```

## Architecture

```
HLX Source (.hlx)
      ↓
┌─────────────────────────────────────────┐
│ Self-Hosting Compiler (written in HLX)  │
│  - lexer.hlx                            │
│  - parser.hlx                           │
│  - lower.hlx                            │
│  - emit.hlx                             │
└─────────────────────────────────────────┘
      ↓
LC-B Bytecode (deterministic, BLAKE3-addressed)
      ↓
┌─────────────────────────────────────────┐
│ Backends                                │
│  - LLVM (native code)                   │
│  - Vulkan (GPU compute)                 │
│  - Interpreter (bootstrap)              │
└─────────────────────────────────────────┘
```

## Determinism by Default

HLX is built on four axioms (ported from RustD):

1. **Determinism**: Same input → same output, always
2. **Reversibility**: Can always decompile compiled code
3. **Injectivity**: Different source → different bytecode
4. **Serialization**: All values serializable

This enables reproducible reasoning traces and auditability.

## The Virtuous Alignment Cycle

1. Syntax embeds conscience as first-class
2. Inference propagates conscience through expressions
3. Models trained on HLX absorb conscience as fundamental
4. Alignment becomes syntactic, not post-hoc

## Project Status

- **Lexer**: 757 lines, 40+ recursive intelligence tokens
- **Parser**: 2500+ lines, AST for agents/cycles/barriers
- **Lowerer**: 1300+ lines, bytecode for recursive execution
- **Backends**: LLVM and Vulkan backends ported and adapted
- **Self-hosting**: In progress

## Repository Structure

```
hlx-compiler/
├── hlx/
│   └── hlx_bootstrap/      # Self-hosting compiler
│       ├── lexer.hlx
│       ├── parser.hlx
│       ├── lower.hlx
│       ├── emit.hlx
│       └── ...
├── backends/
│   ├── llvm/               # Native code generation
│   └── vulkan/             # GPU compute shaders
├── experimental/
│   ├── axiom_demo.hlx      # Full syntax demo
│   ├── test_all_phases.hlx # Test suite
│   └── recursive_seed.hlx  # Minimal agent example
└── HLX_ARCHITECTURE_PLAN.md
```

## Design Philosophy

- **Structure beats scale**: TRM proved 7M params with recursive cycles beats massive models
- **Readability = training signal**: Code that reads like English teaches models better
- **Safety as syntax**: Conscience predicates aren't comments—they're grammar
- **Determinism enables scale**: Reproducible execution enables multi-agent coordination

## Related Projects

- **[Axiom](../Axiom-main)**: Policy engine for AI agents (FFI integration target)
- **[RustD](../rustd)**: Original LLVM/Vulkan backends (ported to HLX)

## License

Apache-2.0

---

*"Less is more" — but only when structure is encoded in the language itself.*
