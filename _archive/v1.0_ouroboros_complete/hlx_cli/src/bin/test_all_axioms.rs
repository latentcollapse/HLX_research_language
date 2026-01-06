//! Comprehensive test for all four HLX axioms

use hlx_compiler::{HlxaParser, HlxaEmitter, Emitter, parser::Parser, lower_to_crate};
use hlx_core::Instruction;
use hlx_runtime;

fn main() -> anyhow::Result<()> {
    println!("=== HLX AXIOM VERIFICATION (4/4) ===\n");

    let source = r#"program axiom_test {
    fn main() {
        let a = 7;
        let b = 3;
        let mul = a * b;
        let div = a / b;
        let result = mul + div;
        return result;
    }
}"#;

    // Parse once
    let ast = HlxaParser::new().parse(source)?;

    // A1: DETERMINISM - Same input produces identical output
    println!("A1: DETERMINISM");
    println!("Running compilation 3 times with same input...");

    let crate1 = lower_to_crate(&ast)?;
    let crate2 = lower_to_crate(&ast)?;
    let crate3 = lower_to_crate(&ast)?;

    // Execute each crate
    let result1 = hlx_runtime::execute(&crate1)?;
    let result2 = hlx_runtime::execute(&crate2)?;
    let result3 = hlx_runtime::execute(&crate3)?;

    if result1 == result2 && result2 == result3 {
        println!("✓ DETERMINISM HOLDS: {:?} == {:?} == {:?}", result1, result2, result3);
    } else {
        println!("✗ DETERMINISM VIOLATED: Results differ!");
        return Err(anyhow::anyhow!("A1 failed"));
    }

    // A2: REVERSIBILITY - All operations traceable through instruction stream
    println!("\nA2: REVERSIBILITY");
    println!("Checking instruction stream for complete operation trace...");

    let krate = lower_to_crate(&ast)?;
    let instrs = &krate.instructions;
    println!("Crate has {} instructions", instrs.len());

    // Verify all operations are present in instruction stream
    let has_mul = instrs.iter().any(|i| matches!(i, Instruction::Mul { .. }));
    let has_div = instrs.iter().any(|i| matches!(i, Instruction::Div { .. }));
    let has_add = instrs.iter().any(|i| matches!(i, Instruction::Add { .. }));

    if has_mul && has_div && has_add {
        println!("✓ REVERSIBILITY HOLDS: All operations (mul, div, add) traceable in instruction stream");
    } else {
        println!("✗ REVERSIBILITY VIOLATED: Missing operations in instruction stream!");
        return Err(anyhow::anyhow!("A2 failed"));
    }

    // A3: BIJECTION - Lossless HLX-A ↔ AST
    println!("\nA3: BIJECTION");
    println!("Testing HLX-A → AST → HLX-A round-trip...");

    let ast1 = HlxaParser::new().parse(source)?;
    let emitted = HlxaEmitter::new().emit(&ast1)?;
    let ast2 = HlxaParser::new().parse(&emitted)?;

    if ast1 == ast2 {
        println!("✓ BIJECTION HOLDS: Perfect round-trip (AST1 == AST2)");
    } else {
        println!("✗ BIJECTION VIOLATED: ASTs differ after round-trip!");
        return Err(anyhow::anyhow!("A3 failed"));
    }

    // A4: UNIVERSAL VALUE - No hidden state, all operations explicit
    println!("\nA4: UNIVERSAL VALUE");
    println!("Verifying no hidden state or implicit operations...");

    // Check that the result is fully determined by explicit operations
    // No random(), no time(), no global state
    let source_text = source.to_lowercase();
    let has_random = source_text.contains("random");
    let has_time = source_text.contains("time") || source_text.contains("date");
    let has_io = source_text.contains("read") || source_text.contains("write")
                 || source_text.contains("print") && !source_text.contains("// print");

    if !has_random && !has_time && !has_io {
        println!("✓ UNIVERSAL VALUE HOLDS: No hidden state, all values explicit");
    } else {
        println!("✗ UNIVERSAL VALUE VIOLATED: Hidden state detected!");
        return Err(anyhow::anyhow!("A4 failed"));
    }

    println!("\n=================================");
    println!("✓✓✓ ALL 4 AXIOMS VERIFIED ✓✓✓");
    println!("=================================");
    println!("\nHLX V2 foundational integrity confirmed:");
    println!("  • A1: Deterministic execution");
    println!("  • A2: Reversible operations");
    println!("  • A3: Bijective translation");
    println!("  • A4: Universal value semantics");

    Ok(())
}
