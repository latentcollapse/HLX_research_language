//! Emitter trait for HLX/HLXL
//!
//! Emitters convert AST back to source text.

use crate::ast::Program;
use hlx_core::Result;

/// Trait for emitting HLX source code
pub trait Emitter {
    /// Emit AST to source code
    fn emit(&self, program: &Program) -> Result<String>;
    
    /// Get emitter name (for error messages)
    fn name(&self) -> &'static str;
}
