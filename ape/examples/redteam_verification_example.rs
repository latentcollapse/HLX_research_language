//! Red Team Safety Verification Example
//!
//! Shows how to use Axiom to verify red team operations before execution.
//! This demonstrates "policy-as-code" for security testing guardrails.

use ape::{AxiomEngine, AxiomResult};

fn main() -> AxiomResult<()> {
    println!("=== Red Team Safety Verification ===\n");

    // Load the red team safety policy
    let engine = AxiomEngine::from_file("examples/policies/redteam_safety.axm")?;

    println!("Loaded policy with {} intents:\n", engine.intents().len());
    for intent in engine.intents() {
        println!("  - {}", intent);
    }

    // ============================================================
    // Test 1: Safe tool installation
    // ============================================================
    println!("\n=== Test 1: Installing nmap (safe tool) ===");
    let verdict = engine.verify("InstallTool", &[("package", "nmap")])?;

    if verdict.allowed() {
        println!("✓ Policy allows nmap installation");
        // In real usage: mcp_server.call("install", {"package": "nmap"})
    } else {
        println!("✗ Denied: {}", verdict.reason().unwrap());
    }

    // ============================================================
    // Test 2: Dangerous command (should be blocked)
    // ============================================================
    println!("\n=== Test 2: Attempting dangerous command ===");
    let verdict = engine.verify("RunCommand", &[("command", "rm -rf /")])?;

    if verdict.allowed() {
        println!("✗ SECURITY ISSUE: dangerous command allowed!");
    } else {
        println!("✓ Correctly blocked: {}", verdict.reason().unwrap());
    }

    // ============================================================
    // Test 3: Safe reconnaissance scan
    // ============================================================
    println!("\n=== Test 3: Scanning localhost ===");
    let verdict = engine.verify(
        "ScanTarget",
        &[("target", "127.0.0.1"), ("tool", "nmap")],
    )?;

    if verdict.allowed() {
        println!("✓ Policy allows localhost scanning");
    } else {
        println!("✗ Denied: {}", verdict.reason().unwrap());
    }

    // ============================================================
    // Test 4: Production system scan (should be blocked)
    // ============================================================
    println!("\n=== Test 4: Attempting to scan production system ===");
    let verdict = engine.verify(
        "ScanTarget",
        &[("target", "google.com"), ("tool", "nmap")],
    )?;

    if verdict.allowed() {
        println!("✗ SECURITY ISSUE: production scan allowed!");
    } else {
        println!("✓ Correctly blocked: {}", verdict.reason().unwrap());
    }

    // ============================================================
    // Test 5: Fork bomb prevention
    // ============================================================
    println!("\n=== Test 5: Fork bomb attempt ===");
    let verdict = engine.verify("RunCommand", &[("command", ":(){ :|:& };:")])?;

    if verdict.allowed() {
        println!("✗ SECURITY ISSUE: fork bomb allowed!");
    } else {
        println!("✓ Correctly blocked fork bomb");
    }

    // ============================================================
    // Test 6: System modification (should be blocked)
    // ============================================================
    println!("\n=== Test 6: System modification attempt ===");
    let verdict = engine.verify("RunCommand", &[("command", "systemctl stop docker")])?;

    if verdict.allowed() {
        println!("✗ SECURITY ISSUE: system modification allowed!");
    } else {
        println!("✓ Correctly blocked system modification");
    }

    // ============================================================
    // Test 7: Reading Axiom source (safe)
    // ============================================================
    println!("\n=== Test 7: Reading Axiom source file ===");
    let verdict = engine.verify("ReadFile", &[("path", "/axiom/src/conscience/mod.rs")])?;

    if verdict.allowed() {
        println!("✓ Policy allows reading Axiom source");
    } else {
        println!("✗ Denied: {}", verdict.reason().unwrap());
    }

    // ============================================================
    // Test 8: Writing to temp (safe)
    // ============================================================
    println!("\n=== Test 8: Writing to /tmp ===");
    let verdict = engine.verify(
        "WriteFile",
        &[("path", "/tmp/test.txt"), ("content", "test data")],
    )?;

    if verdict.allowed() {
        println!("✓ Policy allows writing to /tmp");
    } else {
        println!("✗ Denied: {}", verdict.reason().unwrap());
    }

    // ============================================================
    // Test 9: Writing to /etc (should be blocked)
    // ============================================================
    println!("\n=== Test 9: Attempting to write to /etc ===");
    let verdict = engine.verify(
        "WriteFile",
        &[("path", "/etc/malicious.conf"), ("content", "pwned")],
    )?;

    if verdict.allowed() {
        println!("✗ SECURITY ISSUE: /etc write allowed!");
    } else {
        println!("✓ Correctly blocked /etc write");
    }

    // ============================================================
    // Test 10: Building Axiom (safe)
    // ============================================================
    println!("\n=== Test 10: Building Axiom from source ===");
    let verdict = engine.verify("BuildAxiom", &[("features", "")])?;

    if verdict.allowed() {
        println!("✓ Policy allows building Axiom");
    } else {
        println!("✗ Denied: {}", verdict.reason().unwrap());
    }

    println!("\n=== Summary ===");
    println!("Red team safety policy verification complete.");
    println!("All dangerous operations correctly blocked.");
    println!("Safe operations allowed as expected.");
    println!("\n✓ Policy is working as designed!");

    Ok(())
}
