//! APE — Axiom Policy Engine
//!
//! A verification-first policy engine for AI agents. Provides policy-as-code
//! with deterministic verification before execution.
//!
//! APE is the governance layer of HLX: "the physics of what Bit cannot do."
//! It defines conscience predicates, gate enforcement, and formal proofs (G1-G6).
//!
//! APE can also be used standalone — embed it in any Rust or Python application
//! to add formal policy verification.
//!
//! # Quick Start
//!
//! ```no_run
//! use ape::AxiomEngine;
//!
//! let engine = AxiomEngine::from_file("policy.axm")?;
//! let verdict = engine.verify("WriteFile", &[("path", "/tmp/test.txt")])?;
//! if verdict.allowed() {
//!     // Your code runs
//! }
//! # Ok::<(), ape::error::AxiomError>(())
//! ```
//!
//! # Architecture
//!
//! - **Policy Loading** (`policy` module): Load .axm policy files without execution
//! - **Pure Verification** (`verification` module): Check intents against policy (no side effects)
//! - **Optional Execution** (`engine` module): Verify + execute with full interpreter
//!
//! # Primary API (Embedder-Friendly)
//!
//! The main embedder API provides a simple, SQLite-style interface:

// Primary embedder API - this is what most users should use
pub mod policy;
pub mod verification;
pub mod engine;

// Re-export the main types for convenience
pub use engine::{AxiomEngine, ExecutionResult, IntentSignature};
pub use policy::{Policy, PolicyLoader};
pub use verification::{Verifier, Verdict};
pub use interpreter::value::Value;
pub use error::{AxiomError, AxiomResult};

// Advanced API - for users who need direct access to internals
// These modules are still public for backward compatibility and advanced use cases
pub mod error;
pub mod lexer;
pub mod parser;
pub mod checker;
pub mod interpreter;
pub mod lcb;
pub mod trust;
pub mod conscience;

// C FFI — SQLite-style embedding for any language
pub mod ffi;

// Experimental features - HLX idea vault
// These are research-grade features for advanced use cases
pub mod experimental;

// Backward compatibility re-exports
pub use experimental::dsf;
pub use experimental::scale;
pub use experimental::inference;
pub use experimental::selfmod;
pub use experimental::module;
