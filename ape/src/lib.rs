//! APE — Axiom Policy Engine
//!
//! Digital physics for AI agents. APE is an embeddable governance kernel
//! (SQLite-style) that provides immutable constraint evaluation.
//!
//! APE is not a language. It is gravity. The genesis predicates are
//! hardcoded physical laws — `no_harm`, `no_exfiltrate`, `path_safety`,
//! `no_bypass_verification`, and `baseline_allow`. Code that violates
//! these laws cannot compile or execute.
//!
//! # Quick Start
//!
//! ```
//! use ape::conscience::{ConscienceKernel, EffectClass};
//! use std::collections::HashMap;
//!
//! let mut kernel = ConscienceKernel::new();
//! let mut fields = HashMap::new();
//! fields.insert("path".to_string(), "/tmp/test.txt".to_string());
//!
//! let verdict = kernel.evaluate("WriteFile", &EffectClass::Write, &fields);
//! // verdict is Allow — /tmp is safe
//!
//! fields.insert("path".to_string(), "/etc/shadow".to_string());
//! let verdict = kernel.evaluate("WriteFile", &EffectClass::Write, &fields);
//! // verdict is Deny — /etc is forbidden by path_safety
//! ```
//!
//! # Architecture
//!
//! - **Conscience Kernel** (`conscience`): The core — immutable predicate evaluation
//! - **Trust Levels** (`trust`): Agent trust classification
//! - **Error Types** (`error`): Shared error types
//! - **Experimental** (`experimental`): Research features (DSF, Scale, etc.)

// The core — immutable conscience predicate evaluation
pub mod conscience;

// Trust level classification for agents
pub mod trust;

// Error types
pub mod error;

// Experimental features — research-grade
pub mod experimental;

// LCB (if used)
pub mod lcb;

// PyO3 Python bindings — `from ape import ConscienceKernel`
#[cfg(feature = "python")]
pub mod pymod;

// Re-export the primary API
pub use conscience::{ConscienceKernel, ConscienceVerdict, EffectClass};
pub use error::{AxiomError, AxiomResult};
pub use trust::TrustLevel;
