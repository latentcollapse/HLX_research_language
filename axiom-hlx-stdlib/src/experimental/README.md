# Experimental Features - HLX Idea Vault

This folder contains experimental, research-grade features that are not part of the core Axiom verification API. These represent advanced capabilities and ideas that may evolve into standalone projects.

## Modules

### `dsf/` - Dumb Shit Filter
Static analysis layer that catches common mistakes before execution:
- Unbounded loops
- Missing trust verification
- Unhandled `do` failures
- Trust decay chains
- Re-verification guard misses
- Inference ambiguities
- Environment-dependent conditionals

**Status:** Fully implemented, used by CLI with `--dsf-only` flag

### `scale/` - Multi-Agent Coordination
SCALE (Scalable Concurrent Agent Language Engine) for coordinating multiple agents:
- Agent spawning and lifecycle management
- State synchronization with conflict resolution strategies (LWW, Priority, Custom)
- Barrier synchronization for deterministic merge points
- Agent caps and resource management

**Status:** Fully implemented with conflict-free replicated data types (CRDTs)

### `inference/` - Advanced Type Inference
Configurable type inference modes:
- `strict` - All types must be explicit
- `local` - Local variable inference only
- `full` - Full Hindley-Milner style inference
- `hybrid` - Combination approach

**Status:** Implemented with mode selection via project manifest

### `selfmod/` - Self-Modification Tracking
Gate 3 system for safe runtime self-modification:
- Immutable prefix enforcement
- Complexity budget tracking with exponential backoff
- Proof-based modification proposals
- Cooling periods for safety
- Delta proofs for verification

**Status:** Fully implemented with multi-gate approval system

### `module/` - Module System
Module resolution with project manifests:
- `.axiom.project` manifest parsing
- Module path resolution
- Import dependency tracking
- Version specification

**Status:** Implemented, used by CLI with `--project` flag

## Usage

These features are available through both the library API and CLI:

```rust
// Library usage (backward compatible)
use axiom_lang::dsf::DsfAnalyzer;
use axiom_lang::scale::ScaleCoordinator;
// or
use axiom_lang::experimental::dsf::DsfAnalyzer;
```

```bash
# CLI usage
axiom program.axm --dsf-only      # Run DSF analysis only
axiom --project manifest.project  # Use module system
```

## Future: HLX

These experimental features represent the "idea vault" for HLX - a potential next-generation system building on Axiom's verification-first principles. Key concepts that might evolve:

- **DSF → Static Verification Layer**: More sophisticated analysis
- **SCALE → Distributed Agent Runtime**: True multi-node coordination
- **Inference → Gradual Typing System**: Progressive type refinement
- **SelfMod → Hot Code Replacement**: Production-safe updates
- **Module → Package Ecosystem**: Full dependency management

## Development Status

All modules in this folder are:
- ✅ Fully tested and working
- ✅ Used by existing CLI and integration tests
- ✅ Backward compatible through re-exports in lib.rs
- 🔬 Experimental and may evolve significantly

They're separated from core verification to keep the primary API focused and lightweight.
