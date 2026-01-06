//! Test LLVM Backend Tensor Support

use hlx_compiler::{HlxaParser, parser::Parser, lower::lower_to_crate};
use hlx_backend_llvm::CodeGen;
use inkwell::context::Context;
use std::fs;

fn main() -> anyhow::Result<()> {
    let context = Context::create();
    let mut codegen = CodeGen::new(&context, "tensor_test");
    
    println!("Loading tensor library...");
    let lib_src = fs::read_to_string("lib/tensor.hlxa")?;
    let mut combined_ast = HlxaParser::new().parse(&lib_src).map_err(|e| anyhow::anyhow!("{:?}", e))?;
    
    println!("Loading test program...");
    let test_src = fs::read_to_string("examples/test_tensor.hlxa")?;
    let test_ast = HlxaParser::new().parse(&test_src).map_err(|e| anyhow::anyhow!("{:?}", e))?;
    combined_ast.blocks.extend(test_ast.blocks);
    
    println!("Compiling Crate...");
    let combined_crate = lower_to_crate(&combined_ast).map_err(|e| anyhow::anyhow!("{:?}", e))?;
    codegen.compile_crate(&combined_crate)?;
    
    println!("Executing JIT (Add: 11,22,33,44 | Trans: 1,3,2,4)...");
    let result = codegen.run_jit()?;
    println!("JIT Result: {}", result);
    
    if result == 4 {
        println!("SUCCESS: Tensor Operations Correct");
    } else {
        println!("FAILURE: Expected 4, got {}", result);
    }
    
    Ok(())
}
