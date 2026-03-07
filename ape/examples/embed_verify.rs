//! Example: Embedding APE as a verification library
//!
//! APE is digital physics — embed it like SQLite.

use ape::conscience::{ConscienceKernel, EffectClass};
use std::collections::HashMap;

fn main() {
    let mut kernel = ConscienceKernel::new();

    // Verify a safe operation
    let mut fields = HashMap::new();
    fields.insert("path".to_string(), "/tmp/output.txt".to_string());
    let verdict = kernel.evaluate("WriteFile", &EffectClass::Write, &fields);
    println!("Write to /tmp/output.txt: {:?}", verdict);

    // Verify a dangerous operation — physics will deny this
    let mut fields = HashMap::new();
    fields.insert("path".to_string(), "/etc/shadow".to_string());
    let verdict = kernel.evaluate("WriteFile", &EffectClass::Write, &fields);
    println!("Write to /etc/shadow: {:?}", verdict);
}
