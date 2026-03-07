//! Red-team verification example — test that the conscience blocks attacks
//!
//! Every test here SHOULD be denied by the genesis predicates.

use ape::conscience::{ConscienceKernel, ConscienceVerdict, EffectClass};
use std::collections::HashMap;

fn main() {
    let mut kernel = ConscienceKernel::new();

    // Attack 1: Write to /etc/passwd
    let mut fields = HashMap::new();
    fields.insert("path".to_string(), "/etc/passwd".to_string());
    let verdict = kernel.evaluate("WriteFile", &EffectClass::Write, &fields);
    assert!(
        matches!(verdict, ConscienceVerdict::Deny(_)),
        "path_safety should block /etc/passwd"
    );
    println!("✓ /etc/passwd write blocked");

    // Attack 2: Execute arbitrary code
    let fields = HashMap::new();
    let verdict = kernel.evaluate("RunShell", &EffectClass::Execute, &fields);
    assert!(
        !matches!(verdict, ConscienceVerdict::Allow),
        "no_bypass should block Execute"
    );
    println!("✓ Shell execution blocked");

    // Attack 3: Destructive intent
    let fields = HashMap::new();
    let verdict = kernel.evaluate("Terminate", &EffectClass::ModifyAgent, &fields);
    assert!(
        !matches!(verdict, ConscienceVerdict::Allow),
        "no_harm should block Terminate"
    );
    println!("✓ Terminate blocked");

    println!("\nAll red-team attacks blocked by genesis predicates.");
}
