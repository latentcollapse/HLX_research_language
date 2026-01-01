//! Parser trait for HLX/HLXL
//!
//! Parsers convert source text to AST.

use crate::ast::Program;
use hlx_core::Result;

/// Trait for parsing HLX source code
pub trait Parser {
    /// Parse source code to AST
    fn parse(&self, source: &str) -> Result<Program>;
    
    /// Get parser name (for error messages)
    fn name(&self) -> &'static str;
}
