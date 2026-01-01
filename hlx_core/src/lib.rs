//! # HLX Core
//!
//! Core types and IR for the HLX deterministic ML substrate.
//!
//! This crate defines:
//! - `Value`: The 7 fundamental types + Contract
//! - `Instruction`: The IR for computation
//! - `Capsule`: Integrity-wrapped instruction sequences
//! - LC-B encoding/decoding
//!
//! ## Axioms
//! - **A1 (Determinism)**: Same input → same LC-B output
//! - **A2 (Reversibility)**: decode(encode(v)) == v
//! - **A3 (Bijection)**: 1:1 correspondence between values and encodings
//! - **A4 (Universal Value)**: All types lower to this core

pub mod value;
pub mod instruction;
pub mod capsule;
pub mod lcb;
pub mod error;

pub use value::{Value, Contract, FieldIndex};
pub use instruction::{Instruction, TensorShape, Register};
pub use capsule::Capsule;
pub use error::{HlxError, Result};

/// Magic byte for LC-B format
pub const LCB_MAGIC: u8 = 0x7C; // '|'

/// Maximum nesting depth (determinism constraint)
pub const MAX_DEPTH: usize = 64;

/// Capsule format version
pub const CAPSULE_VERSION: u8 = 1;
