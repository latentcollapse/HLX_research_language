//! Advanced Execution Example
//!
//! This example shows how to use Axiom for both verification AND execution.
//! The evaluate() method provides full interpreter capabilities with handlers.
//!
//! Run with: cargo run --example embed_execute

use axiom_lang::{AxiomEngine, AxiomResult, AxiomError};
use axiom_lang::error::ErrorKind;

fn main() -> AxiomResult<()> {
    println!("=== Axiom Execution Example ===\n");

    let mut engine = AxiomEngine::from_file("examples/policies/security.axm")?;

    println!("Step 1: Pure verification (fast, no side effects)");
    println!("---------------------------------------------------");

    let verdict = engine.verify("ProcessData", &[("input", "test data")])?;

    if !verdict.allowed() {
        return Err(AxiomError {
            kind: ErrorKind::HaltConscience,
            message: format!("Verification failed: {}", verdict.reason().unwrap()),
            span: None,
        });
    }

    println!("✓ Verification passed");
    println!("  Category: {:?}", verdict.category());
    println!("  Guidance: {}", verdict.guidance());

    // Note: Full execution requires implementing handlers and the complete
    // interpreter integration. For now, this demonstrates the API surface.

    println!("\nStep 2: Execution (verify + run)");
    println!("---------------------------------------------------");

    // In a full implementation, this would:
    // 1. Verify the intent (same as above)
    // 2. Initialize the interpreter (lazy, first call only)
    // 3. Execute the intent through the interpreter
    // 4. Return the result, verdict, and logs

    // Simulated execution for demonstration
    println!("✓ Intent 'ProcessData' would be executed here");
    println!("  Input: test data");
    println!("  [Note: Full execution requires handler implementation]");

    println!("\n=== Verification-First Workflow ===\n");

    // The recommended pattern: always verify first
    let intents_to_check = vec![
        ("ReadFile", vec![("path", "/tmp/safe.txt")]),
        ("ReadFile", vec![("path", "/etc/passwd")]),  // Should fail
        ("WriteFile", vec![("path", "/tmp/out.txt"), ("content", "data")]),
        ("ProcessData", vec![("input", "test")]),
    ];

    for (intent_name, fields) in intents_to_check {
        print!("Checking {} ", intent_name);
        print!("with {:?}... ", fields);

        let verdict = engine.verify(intent_name, &fields)?;

        if verdict.allowed() {
            println!("✓ ALLOWED");
            // In production: execute the actual operation here
        } else {
            println!("✗ DENIED");
            println!("  Reason: {}", verdict.reason().unwrap());
        }
    }

    println!("\n=== Lazy Interpreter Initialization ===\n");

    // The interpreter is only initialized when evaluate() is called
    if !engine.is_interpreter_initialized() {
        println!("✓ Interpreter not initialized yet (verification is lightweight)");
    }

    // Calling evaluate() would initialize the interpreter
    // let result = engine.evaluate("ProcessData", &[
    //     ("input", Value::String("test".into())),
    // ])?;

    println!("\n=== Policy as Product ===\n");

    println!("The security.axm file acts as configuration:");
    println!("  - Drop-in security policy");
    println!("  - Reusable across projects");
    println!("  - Versioned like code");
    println!("  - Self-documenting constraints");
    println!("\nJust like SQLite for storage, Axiom for verification.");

    Ok(())
}
