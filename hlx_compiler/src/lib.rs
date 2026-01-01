//! # HLX Compiler
//!
//! Bijective compiler for HLX/HLXL source code.

pub mod ast;
pub mod parser;
pub mod emitter;
pub mod hlxl;
pub mod runic;
pub mod lower;

pub use ast::*;
pub use parser::Parser;
pub use emitter::Emitter;
pub use hlxl::{HlxlParser, HlxlEmitter};
pub use runic::{RunicParser, RunicEmitter};
pub use lower::lower_to_capsule;

use hlx_core::{capsule::Capsule, Result};

/// Compile HLXL source to Capsule
pub fn compile_hlxl(source: &str) -> Result<Capsule> {
    let parser = HlxlParser::new();
    let ast = parser.parse(source)?;
    lower_to_capsule(&ast)
}

/// Compile HLX (runic) source to Capsule
pub fn compile_hlx(source: &str) -> Result<Capsule> {
    let parser = RunicParser::new();
    let ast = parser.parse(source)?;
    lower_to_capsule(&ast)
}

/// Decompile Capsule back to HLXL
pub fn decompile_hlxl(capsule: &Capsule) -> Result<String> {
    let emitter = HlxlEmitter::new();
    let ast = lower::lift_from_capsule(capsule)?;
    emitter.emit(&ast)
}

/// Decompile Capsule back to HLX (runic)
pub fn decompile_hlx(capsule: &Capsule) -> Result<String> {
    let emitter = RunicEmitter::new();
    let ast = lower::lift_from_capsule(capsule)?;
    emitter.emit(&ast)
}

/// Transliterate HLXL to HLX (runic)
pub fn hlxl_to_hlx(source: &str) -> Result<String> {
    let ast = HlxlParser::new().parse(source)?;
    RunicEmitter::new().emit(&ast)
}

/// Transliterate HLX (runic) to HLXL
pub fn hlx_to_hlxl(source: &str) -> Result<String> {
    let ast = RunicParser::new().parse(source)?;
    HlxlEmitter::new().emit(&ast)
}