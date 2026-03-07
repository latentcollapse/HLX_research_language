//! Red-team attack suite — comprehensive assault on the conscience
//!
//! Tests path traversal, homoglyph attacks, privilege escalation, etc.

use ape::conscience::{ConscienceKernel, ConscienceVerdict, EffectClass};
use std::collections::HashMap;

fn assert_denied(
    kernel: &mut ConscienceKernel,
    intent: &str,
    effect: &EffectClass,
    fields: &HashMap<String, String>,
    desc: &str,
) {
    let verdict = kernel.evaluate(intent, effect, fields);
    assert!(
        !matches!(verdict, ConscienceVerdict::Allow),
        "ATTACK SUCCEEDED: {}",
        desc
    );
    println!("✓ Blocked: {}", desc);
}

fn main() {
    let mut kernel = ConscienceKernel::new();

    // Path traversal attacks
    let mut f = HashMap::new();
    f.insert("path".to_string(), "/tmp/../etc/shadow".to_string());
    assert_denied(
        &mut kernel,
        "Read",
        &EffectClass::Read,
        &f,
        "path traversal /tmp/../etc/shadow",
    );

    // Unicode homoglyph attack (Cyrillic 'е' looks like Latin 'e')
    let mut f = HashMap::new();
    f.insert("path".to_string(), "/\u{0435}tc/passwd".to_string()); // Cyrillic е
    assert_denied(
        &mut kernel,
        "Read",
        &EffectClass::Read,
        &f,
        "unicode homoglyph /еtc/passwd",
    );

    // URL-encoded path
    let mut f = HashMap::new();
    f.insert("path".to_string(), "/%65tc/shadow".to_string());
    assert_denied(
        &mut kernel,
        "Read",
        &EffectClass::Read,
        &f,
        "URL-encoded /%65tc/shadow",
    );

    println!("\nAll red-team attacks blocked.");
}
