//! Simple Verification Example - The "SQLite Moment"
//!
//! This example shows how easy it is to get started with Axiom.
//! Just 5 lines to verify agent code before execution.
//!
//! Run with: cargo run --example embed_verify

use ape::{AxiomEngine, AxiomResult};

fn main() -> AxiomResult<()> {
    println!("=== Axiom Verification Example ===\n");

    // -------------------- THE SQLITE MOMENT --------------------
    // Just 5 lines, instant value, no complex setup

    let engine = AxiomEngine::from_file("examples/policies/security.axm")?;

    let verdict = engine.verify("WriteFile", &[
        ("path", "/tmp/data.txt"),
        ("content", "hello world"),
    ])?;

    if verdict.allowed() {
        println!("✓ Policy allows this write");
        println!("  Guidance: {}", verdict.guidance());

        // Your code runs here
        // In a real application, you would execute the actual file write
        println!("\n[Simulated] Writing to /tmp/data.txt...");
    } else {
        println!("✗ Policy denied: {}", verdict.reason().unwrap());
    }

    // -----------------------------------------------------------

    println!("\n=== Testing More Intents ===\n");

    // Test 1: Safe read operation
    println!("1. Testing safe file read:");
    let verdict = engine.verify("ReadFile", &[("path", "/tmp/input.txt")])?;
    if verdict.allowed() {
        println!("   ✓ Safe path allowed");
    } else {
        println!("   ✗ Denied: {}", verdict.reason().unwrap());
    }

    // Test 2: Dangerous path (should be denied)
    println!("\n2. Testing dangerous path:");
    let verdict = engine.verify("ReadFile", &[("path", "/etc/shadow")])?;
    if verdict.allowed() {
        println!("   ✓ Allowed (unexpected!)");
    } else {
        println!("   ✗ Correctly denied: {}", verdict.reason().unwrap());
    }

    // Test 3: Network operation (should be denied without declared channel)
    println!("\n3. Testing network operation:");
    let verdict = engine.verify("SendData", &[
        ("url", "http://example.com"),
        ("data", "test payload"),
    ])?;
    if verdict.allowed() {
        println!("   ✓ Allowed");
    } else {
        println!("   ✗ Denied: {}", verdict.reason().unwrap());
    }

    // Test 4: Safe data processing (NOOP effect)
    println!("\n4. Testing safe data processing:");
    let verdict = engine.verify("ProcessData", &[("input", "test data")])?;
    if verdict.allowed() {
        println!("   ✓ Safe operation allowed");
    } else {
        println!("   ✗ Denied: {}", verdict.reason().unwrap());
    }

    println!("\n=== Introspection ===\n");

    // List all available intents
    println!("Available intents in policy:");
    for intent in engine.intents() {
        println!("  - {}", intent);
    }

    // Get signature of an intent
    if let Some(sig) = engine.intent_signature("WriteFile") {
        println!("\nWriteFile signature:");
        println!("  Takes:");
        for (name, ty) in &sig.takes {
            println!("    {} : {}", name, ty);
        }
        println!("  Gives:");
        for (name, ty) in &sig.gives {
            println!("    {} : {}", name, ty);
        }
        println!("  Effect: {}", sig.effect);
        println!("  Conscience: {:?}", sig.conscience);
    }

    Ok(())
}
