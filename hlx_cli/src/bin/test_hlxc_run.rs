//! Test Runner for HLX-C
//!
//! Compiles and executes an HLX-C program.

use hlx_compiler::{hlxc::HlxcParser, parser::Parser, lower::lower_to_capsule};
use hlx_runtime::{Executor, RuntimeConfig};
use hlx_core::Value;

fn main() -> anyhow::Result<()> {
    // Enable tracing
    tracing_subscriber::fmt::init();

    // 1. Define HLX-C Source (Iterative Fibonacci)
    let source = r#" 
        fn main() -> i32 {
            let n = 10;
            let a = 0;
            let b = 1;
            let i = 0;
            
            // Loop n times to compute fib(n)
            loop (i < n, 20) {
                let temp = a + b;
                let a = b;
                let b = temp;
                let i = i + 1;
            }
            
            return a;
        }
    "#;

    println!("--- HLX-C Source ---");
    println!("{}", source.trim());

    // 2. Parse
    println!("\n[1/3] Parsing...");
    let parser = HlxcParser::new();
    let ast = parser.parse(source).map_err(|e| anyhow::anyhow!("Parse error: {:?}", e))?;
    println!("Parsed successfully.");

    // 3. Lower to Capsule (LC-B)
    println!("[2/3] Lowering to LC-B Capsule...");
    let capsule = lower_to_capsule(&ast)?;
    println!("Capsule generated with {} instructions.", capsule.instructions.len());
    
    // Debug print instructions
    for (i, inst) in capsule.instructions.iter().enumerate() {
        println!("{:03}: {:?}", i, inst);
    }

    // 4. Execute
    println!("[3/3] Executing...");
    let config = RuntimeConfig { debug: true, ..Default::default() };
    let executor = Executor::new(&config)?;
    let result = executor.run(&capsule)?;

    println!("\n--- Result ---");
    println!("{:?}", result);

    // Verify fib(10) = 55
    if result == Value::Integer(55) {
        println!("SUCCESS: Fibonacci sequence calculated correctly!");
    } else {
        println!("FAILURE: Expected 55, got {:?}", result);
        std::process::exit(1);
    }

    Ok(())
}
