//! Test LLVM Backend Graphics Support

use hlx_compiler::{HlxaParser, parser::Parser, lower::lower_to_crate};
use hlx_backend_llvm::CodeGen;
use inkwell::context::Context;
use std::fs;

fn main() -> anyhow::Result<()> {
    let context = Context::create();
    let mut codegen = CodeGen::new(&context, "graphics_test");
    
    println!("Loading libraries...");
    let gfx_src = fs::read_to_string("lib/graphics.hlxa")?;
    let mut combined_ast = HlxaParser::new().parse(&gfx_src).map_err(|e| anyhow::anyhow!("{:?}", e))?;
    
    println!("Loading test program...");
    let test_src = fs::read_to_string("examples/test_graphics.hlxa")?;
    let test_ast = HlxaParser::new().parse(&test_src).map_err(|e| anyhow::anyhow!("{:?}", e))?;
    combined_ast.blocks.extend(test_ast.blocks);
    
    println!("Compiling Crate...");
    let combined_crate = lower_to_crate(&combined_ast).map_err(|e| anyhow::anyhow!("{:?}", e))?;
    codegen.compile_crate(&combined_crate)?;
    
    println!("Executing JIT (A Red window should appear)...");
    let result = codegen.run_jit()?;
    println!("JIT Result: {}", result);
    
    Ok(())
}
