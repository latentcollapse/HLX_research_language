//! HLX Compiler Core
//!
//! Provides parsing, lowering, and lifting services for the HLX ecosystem.

pub mod ast;
pub mod parser;
pub mod emitter;
pub mod lower;
pub mod hlxa;
pub mod runic;

pub use hlxa::{HlxaParser, HlxaEmitter};
pub use runic::{RunicParser, RunicEmitter};
pub use emitter::Emitter;
pub use lower::lower_to_crate;

use hlx_core::{HlxCrate, Result as HlxResult, HlxError};
use ast::Program;

/// Stub for lifting until V2 lifter is implemented
pub fn lift_from_crate(_krate: &HlxCrate) -> HlxResult<Program> {
    Err(HlxError::ValidationFail { message: "Lifter not yet implemented for V2".to_string() })
}