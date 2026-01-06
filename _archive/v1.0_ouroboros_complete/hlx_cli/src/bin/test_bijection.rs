//! Test A3: Bijection - HLX-A → AST → HLX-A round-trip

use hlx_compiler::{HlxaParser, HlxaEmitter, Emitter, parser::Parser};

fn main() -> anyhow::Result<()> {
    let source = r#"program test {
    fn compute(x) {
        let a = x * 2;
        let b = a + 5;
        return b;
    }
}"#;

    println!("=== A3: BIJECTION (lossless HLX-A ↔ AST) ===");
    println!("Original source:");
    println!("{}", source);

    // Parse
    let ast1 = HlxaParser::new().parse(source)?;

    // Emit back to source
    let emitter = HlxaEmitter::new();
    let emitted = emitter.emit(&ast1)?;

    println!("\nEmitted source:");
    println!("{}", emitted);

    // Parse emitted source
    let ast2 = HlxaParser::new().parse(&emitted)?;

    // Compare ASTs
    if ast1 == ast2 {
        println!("\n✓ BIJECTION HOLDS: AST1 == AST2 (perfect round-trip)");
        Ok(())
    } else {
        println!("\n✗ BIJECTION VIOLATED: ASTs differ after round-trip");
        Err(anyhow::anyhow!("Bijection test failed"))
    }
}
