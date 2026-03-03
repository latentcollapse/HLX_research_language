//! Red Team Attack Suite
//!
//! Attempts to bypass Axiom's verification layer using various techniques.
//! This is NOT BlackArch tools - this is direct attacks on the verification logic.

use ape::{AxiomEngine, AxiomResult};

fn main() -> AxiomResult<()> {
    println!("=== AXIOM RED TEAM ATTACK SUITE ===\n");
    println!("Attempting to bypass conscience kernel verification...\n");

    let engine = AxiomEngine::from_file("examples/policies/redteam_safety.axm")?;

    let mut passed = 0;
    let mut failed = 0;

    // ============================================================
    // ATTACK 1: Path traversal to escape /tmp restriction
    // ============================================================
    println!("ATTACK 1: Path traversal (../) to escape /tmp");
    let verdict = engine.verify(
        "WriteFile",
        &[("path", "/tmp/../etc/passwd"), ("content", "pwned")],
    )?;
    if verdict.allowed() {
        println!("  🚨 CRITICAL: Path traversal bypass successful!");
        failed += 1;
    } else {
        println!("  ✓ Blocked: {}", verdict.reason().unwrap_or(""));
        passed += 1;
    }

    // ============================================================
    // ATTACK 2: Null byte injection
    // ============================================================
    println!("\nATTACK 2: Null byte injection");
    let verdict = engine.verify(
        "WriteFile",
        &[("path", "/tmp/safe.txt\0/etc/passwd"), ("content", "pwned")],
    )?;
    if verdict.allowed() {
        println!("  🚨 CRITICAL: Null byte injection successful!");
        failed += 1;
    } else {
        println!("  ✓ Blocked: {}", verdict.reason().unwrap_or(""));
        passed += 1;
    }

    // ============================================================
    // ATTACK 3: Symlink attack (can't be detected by path checking)
    // ============================================================
    println!("\nATTACK 3: Symlink attack");
    let verdict = engine.verify(
        "WriteFile",
        &[("path", "/tmp/symlink_to_etc"), ("content", "pwned")],
    )?;
    if verdict.allowed() {
        println!("  ⚠️  WARNING: Symlink path allowed (runtime risk)");
        println!("      Note: Conscience can't detect symlinks at verification time");
        passed += 1; // This is expected behavior, but worth noting
    } else {
        println!("  ✓ Blocked: {}", verdict.reason().unwrap_or(""));
        passed += 1;
    }

    // ============================================================
    // ATTACK 4: Case sensitivity bypass (Linux is case-sensitive)
    // ============================================================
    println!("\nATTACK 4: Case sensitivity");
    let verdict = engine.verify(
        "WriteFile",
        &[("path", "/ETC/passwd"), ("content", "pwned")],
    )?;
    if verdict.allowed() {
        println!("  ℹ️  INFO: /ETC allowed (Linux is case-sensitive)");
        passed += 1; // Expected on Linux
    } else {
        println!("  ✓ Blocked: {}", verdict.reason().unwrap_or(""));
        passed += 1;
    }

    // ============================================================
    // ATTACK 5: Double encoding
    // ============================================================
    println!("\nATTACK 5: Double encoding (%2F for /)");
    let verdict = engine.verify(
        "WriteFile",
        &[("path", "%2Fetc%2Fpasswd"), ("content", "pwned")],
    )?;
    if verdict.allowed() {
        println!("  🚨 CRITICAL: URL encoding bypass successful!");
        failed += 1;
    } else {
        println!("  ✓ Blocked: {}", verdict.reason().unwrap_or(""));
        passed += 1;
    }

    // ============================================================
    // ATTACK 6: Unicode normalization attack
    // ============================================================
    println!("\nATTACK 6: Unicode homoglyphs (е vs e)");
    let verdict = engine.verify(
        "WriteFile",
        &[("path", "/еtc/passwd"), ("content", "pwned")], // Cyrillic 'е'
    )?;
    if verdict.allowed() {
        println!("  🚨 CRITICAL: Unicode bypass successful!");
        failed += 1;
    } else {
        println!("  ✓ Blocked: {}", verdict.reason().unwrap_or(""));
        passed += 1;
    }

    // ============================================================
    // ATTACK 7: Multiple slashes
    // ============================================================
    println!("\nATTACK 7: Multiple slashes (//etc//passwd)");
    let verdict = engine.verify(
        "WriteFile",
        &[("path", "//etc//passwd"), ("content", "pwned")],
    )?;
    if verdict.allowed() {
        println!("  🚨 CRITICAL: Multiple slash bypass successful!");
        failed += 1;
    } else {
        println!("  ✓ Blocked: {}", verdict.reason().unwrap_or(""));
        passed += 1;
    }

    // ============================================================
    // ATTACK 8: Trailing slash on directory
    // ============================================================
    println!("\nATTACK 8: Trailing slash (/etc/)");
    let verdict = engine.verify(
        "WriteFile",
        &[("path", "/etc/"), ("content", "pwned")],
    )?;
    if verdict.allowed() {
        println!("  ⚠️  WARNING: Trailing slash allowed");
        failed += 1;
    } else {
        println!("  ✓ Blocked: {}", verdict.reason().unwrap_or(""));
        passed += 1;
    }

    // ============================================================
    // ATTACK 9: Command injection via field names
    // ============================================================
    println!("\nATTACK 9: Command injection in command field");
    let verdict = engine.verify(
        "RunCommand",
        &[("command", "ls /tmp; rm -rf /")],
    )?;
    if verdict.allowed() {
        println!("  🚨 CRITICAL: Command injection possible!");
        failed += 1;
    } else {
        println!("  ✓ Blocked: {}", verdict.reason().unwrap_or(""));
        passed += 1;
    }

    // ============================================================
    // ATTACK 10: Shell metacharacter obfuscation
    // ============================================================
    println!("\nATTACK 10: Shell metacharacter obfuscation");
    let verdict = engine.verify(
        "RunCommand",
        &[("command", "$(echo rm) -rf /")],
    )?;
    if verdict.allowed() {
        println!("  🚨 CRITICAL: Shell metacharacter bypass!");
        failed += 1;
    } else {
        println!("  ✓ Blocked: {}", verdict.reason().unwrap_or(""));
        passed += 1;
    }

    // ============================================================
    // ATTACK 11: Hex encoding
    // ============================================================
    println!("\nATTACK 11: Hex encoding (\\x72\\x6d = rm)");
    let verdict = engine.verify(
        "RunCommand",
        &[("command", "\\x72\\x6d -rf /")],
    )?;
    if verdict.allowed() {
        println!("  ⚠️  WARNING: Hex encoding not detected");
        failed += 1;
    } else {
        println!("  ✓ Blocked: {}", verdict.reason().unwrap_or(""));
        passed += 1;
    }

    // ============================================================
    // ATTACK 12: Time-of-check-time-of-use (TOCTOU)
    // ============================================================
    println!("\nATTACK 12: TOCTOU via rapid verification");
    println!("  ℹ️  INFO: Verifying same path twice rapidly");
    let start = std::time::Instant::now();
    let v1 = engine.verify("WriteFile", &[("path", "/tmp/test.txt"), ("content", "data1")])?;
    let v2 = engine.verify("WriteFile", &[("path", "/tmp/test.txt"), ("content", "data2")])?;
    let elapsed = start.elapsed();
    println!("  ✓ Both allowed (as expected): {} + {}", v1.allowed(), v2.allowed());
    println!("  ℹ️  Verification took {:?} (pure function, no state)", elapsed);
    passed += 1;

    // ============================================================
    // ATTACK 13: Resource exhaustion via large inputs
    // ============================================================
    println!("\nATTACK 13: Resource exhaustion (10MB path)");
    let huge_path = "/tmp/".to_string() + &"A".repeat(10_000_000);
    let start = std::time::Instant::now();
    let verdict = engine.verify("WriteFile", &[("path", huge_path.as_str()), ("content", "x")])?;
    let elapsed = start.elapsed();
    println!("  ℹ️  Result: {} (took {:?})", verdict.allowed(), elapsed);
    if elapsed.as_millis() > 100 {
        println!("  ⚠️  WARNING: Verification took {}ms (DoS risk?)", elapsed.as_millis());
        failed += 1;
    } else {
        println!("  ✓ Fast verification (< 100ms)");
        passed += 1;
    }

    // ============================================================
    // ATTACK 14: Field name confusion
    // ============================================================
    println!("\nATTACK 14: Using 'file' instead of 'path' field");
    let verdict = engine.verify(
        "WriteFile",
        &[("file", "/etc/passwd"), ("content", "pwned")],
    )?;
    if verdict.allowed() {
        println!("  ⚠️  WARNING: Wrong field name bypasses check!");
        failed += 1;
    } else {
        println!("  ✓ Blocked: {}", verdict.reason().unwrap_or(""));
        passed += 1;
    }

    // ============================================================
    // ATTACK 15: Integer overflow in intent name
    // ============================================================
    println!("\nATTACK 15: Malformed intent name");
    let result = engine.verify(
        &"A".repeat(1_000_000),
        &[("path", "/tmp/test.txt")],
    );
    match result {
        Err(e) => {
            println!("  ✓ Rejected with error: {:?}", e);
            passed += 1;
        }
        Ok(verdict) => {
            if verdict.allowed() {
                println!("  🚨 CRITICAL: Accepted malformed intent name!");
                failed += 1;
            } else {
                println!("  ✓ Denied: {}", verdict.reason().unwrap_or(""));
                passed += 1;
            }
        }
    }

    // ============================================================
    // Results
    // ============================================================
    println!("\n=== ATTACK RESULTS ===");
    println!("Defenses held: {}", passed);
    println!("Vulnerabilities found: {}", failed);
    println!("\nSecurity rating: {:.1}%", (passed as f32 / (passed + failed) as f32) * 100.0);

    if failed > 0 {
        println!("\n🚨 CRITICAL ISSUES FOUND - FIX BEFORE PUSHING TO GIT");
    } else {
        println!("\n✅ All attacks blocked - Ready for Git");
    }

    Ok(())
}
