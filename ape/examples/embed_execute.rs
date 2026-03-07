//! Example: Embedding APE for runtime enforcement
//!
//! Use ConscienceKernel to gate all side-effecting operations.

use ape::conscience::{ConscienceKernel, ConscienceVerdict, EffectClass};
use std::collections::HashMap;

fn main() {
    let mut kernel = ConscienceKernel::new();

    // Simulate an agent trying to execute a shell command
    let fields = HashMap::new();
    let verdict = kernel.evaluate("RunShell", &EffectClass::Execute, &fields);
    match verdict {
        ConscienceVerdict::Allow => println!("Execute allowed"),
        ConscienceVerdict::Deny(reason) => println!("Execute denied: {}", reason),
        ConscienceVerdict::Unknown => println!("Execute: no predicate applies (default deny)"),
    }
}
