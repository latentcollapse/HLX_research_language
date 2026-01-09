//! Test Standard Library Functions

use hlx_compiler::{HlxaParser, parser::Parser, lower};
use hlx_backend_llvm::CodeGen;
use inkwell::context::Context;
use std::fs;

fn main() -> anyhow::Result<()> {
    let context = Context::create();
    let mut codegen = CodeGen::new(&context, "stdlib_test")?;

    println!("Loading standard libraries...");

    // Load all stdlib files
    let libs = vec![
        "lib/math.hlxa",
        "lib/vector.hlxa",
        "lib/io.hlxa",
        "lib/string.hlxa",
    ];

    let mut combined_ast = HlxaParser::new().parse("program empty {}").unwrap();
    combined_ast.blocks.clear();  // Start fresh

    for lib in libs {
        let src = fs::read_to_string(lib)?;
        let lib_ast = HlxaParser::new().parse(&src).map_err(|e| anyhow::anyhow!("{:?}", e))?;
        combined_ast.blocks.extend(lib_ast.blocks);
    }

    println!("Loading test program...");
    let test_src = fs::read_to_string("examples/test_stdlib.hlxa")?;
    let test_ast = HlxaParser::new().parse(&test_src).map_err(|e| anyhow::anyhow!("{:?}", e))?;
    combined_ast.blocks.extend(test_ast.blocks);

    println!("Compiling... ({} blocks)", combined_ast.blocks.len());
    let combined_crate = lower::lower_to_crate(&combined_ast).map_err(|e| {
        eprintln!("Lowering error: {:?}", e);
        anyhow::anyhow!("{:?}", e)
    })?;
    codegen.compile_crate(&combined_crate)?;

    println!("Executing tests...\n");
    let result = codegen.run_jit()?;
    println!("\nTest result: {}", result);

    Ok(())
}
