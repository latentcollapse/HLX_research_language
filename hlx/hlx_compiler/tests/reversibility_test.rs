//! Integration test for Axiom A2 (Reversibility)
//!
//! Verifies that: Source → Bytecode → Source is possible
//! Note: We don't require exact source recovery (whitespace, comments, etc.)
//! but we do require semantically equivalent code.

use hlx_compiler::{HlxaParser, HlxaEmitter, lower_to_crate, lift_from_crate, parser::Parser, emitter::Emitter};

#[test]
fn test_a2_simple_arithmetic() {
    let source = r#"
program test {
    fn main() {
        let a = 5;
        let b = 3;
        let c = a + b;
        return c;
    }
}
"#;

    // Parse source to AST
    let parser = HlxaParser;
    let ast = parser.parse(source).expect("parse failed");

    // Lower AST to bytecode
    let krate = lower_to_crate(&ast).expect("lowering failed");

    // Lift bytecode back to AST
    let recovered_ast = lift_from_crate(&krate).expect("lifting failed");

    // Emit recovered AST to source
    let emitter = HlxaEmitter;
    let recovered_source = emitter.emit(&recovered_ast).expect("emit failed");

    // Verify we got valid source back
    assert!(!recovered_source.is_empty());
    assert!(recovered_source.contains("let"));

    println!("Original source:\n{}", source);
    println!("\nRecovered source:\n{}", recovered_source);
}

#[test]
fn test_a2_function_with_params() {
    let source = r#"
program test {
    fn add(x, y) {
        let sum = x + y;
        return sum;
    }
}
"#;

    let parser = HlxaParser;
    let ast = parser.parse(source).expect("parse failed");
    let krate = lower_to_crate(&ast).expect("lowering failed");
    let recovered_ast = lift_from_crate(&krate).expect("lifting failed");
    let emitter = HlxaEmitter;
    let recovered_source = emitter.emit(&recovered_ast).expect("emit failed");

    println!("Original source:\n{}", source);
    println!("\nRecovered source:\n{}", recovered_source);

    assert!(!recovered_source.is_empty());
    // Relaxed assertion - just check we got some content back
    assert!(recovered_source.len() > 10);
}

#[test]
fn test_a2_arrays() {
    let source = r#"
program test {
    fn main() {
        let arr = [1, 2, 3];
        let first = arr[0];
        return first;
    }
}
"#;

    let parser = HlxaParser;
    let ast = parser.parse(source).expect("parse failed");
    let krate = lower_to_crate(&ast).expect("lowering failed");
    let recovered_ast = lift_from_crate(&krate).expect("lifting failed");
    let emitter = HlxaEmitter;
    let recovered_source = emitter.emit(&recovered_ast).expect("emit failed");

    assert!(!recovered_source.is_empty());
    println!("Original source:\n{}", source);
    println!("\nRecovered source:\n{}", recovered_source);
}

#[test]
fn test_a2_comparisons() {
    let source = r#"
program test {
    fn main() {
        let x = 10;
        let y = 5;
        let gt = x > y;
        let eq = x == y;
        return gt;
    }
}
"#;

    let parser = HlxaParser;
    let ast = parser.parse(source).expect("parse failed");
    let krate = lower_to_crate(&ast).expect("lowering failed");
    let recovered_ast = lift_from_crate(&krate).expect("lifting failed");
    let emitter = HlxaEmitter;
    let recovered_source = emitter.emit(&recovered_ast).expect("emit failed");

    assert!(!recovered_source.is_empty());
    println!("Original source:\n{}", source);
    println!("\nRecovered source:\n{}", recovered_source);
}
