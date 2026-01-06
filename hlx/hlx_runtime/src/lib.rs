//! # HLX Runtime
//! 
//! Deterministic execution engine for LC-B crates.
//! 
//! ## Architecture
//! 
//! ```text
//! LC-B Crate
//!      │
//!      ▼
//! ┌─────────────────┐
//! │  Runtime        │
//! │  ┌───────────┐  │
//! │  │ Validator │  │  ← Integrity check (BLAKE3)
//! │  └───────────┘  │
//! │       │         │
//! │       ▼         │
//! │  ┌───────────┐  │
//! │  │ Executor  │  │  ← Instruction dispatch
//! │  └───────────┘  │
//! │       │         │
//! │       ▼         │
//! │  ┌───────────┐  │
//! │  │ Backend   │◄─┼── CPU (ndarray) | Vulkan (SPIR-V)
//! │  └───────────┘  │
//! └─────────────────┘
//!      │
//!      ▼
//!   Result (Value)
//! ```
//! 
//! ## Determinism Guarantees
//! 
//! 1. Same crate + same config = same result
//! 2. Fixed workgroup sizes on GPU
//! 3. Deterministic reduction order
//! 4. No dynamic memory allocation during execution

pub mod config;
pub mod backend;
pub mod executor;
pub mod value_store;
pub mod tuning;

#[cfg(feature = "cpu")]
pub mod backends;

pub use config::RuntimeConfig;
pub use backend::Backend;
pub use executor::Executor;
pub use value_store::ValueStore;

use hlx_core::{HlxCrate, Value, Result};

/// Execute a crate with default configuration
pub fn execute(krate: &HlxCrate) -> Result<Value> {
    let config = RuntimeConfig::default();
    execute_with_config(krate, &config)
}

/// High-level entry point to execute a crate with specific config
pub fn execute_with_config(krate: &HlxCrate, config: &RuntimeConfig) -> Result<Value> {
    let mut executor = Executor::new(config)?;
    executor.run(krate)
}

/// Quick test: 5 + 3 = 8
#[cfg(test)]
mod smoke_tests {
    use super::*;
    use hlx_core::{HlxCrate, Instruction, Value};

    #[test]
    fn test_basic_addition() {
        // 🜃5 + 🜃3 = 8
        let krate = HlxCrate::new(vec![
            Instruction::Constant { out: 0, val: Value::Integer(5) },
            Instruction::Constant { out: 1, val: Value::Integer(3) },
            Instruction::Add { out: 2, lhs: 0, rhs: 1 },
            Instruction::Return { val: 2 },
        ]);
        
        let result = execute(&krate).unwrap();
        assert_eq!(result, Value::Integer(8));
    }

    #[test]
    fn test_determinism() {
        let krate = HlxCrate::new(vec![
            Instruction::Constant { out: 0, val: Value::Integer(42) },
            Instruction::Constant { out: 1, val: Value::Integer(10) },
            Instruction::Mul { out: 2, lhs: 0, rhs: 1 },
            Instruction::Return { val: 2 },
        ]);
        
        // Run 100 times
        let results: Vec<_> = (0..100)
            .map(|_| execute(&krate).unwrap())
            .collect();
        
        // All must be identical
        assert!(results.iter().all(|r| *r == Value::Integer(420)));
    }

    #[test]
    fn test_crate_validation() {
        let mut krate = HlxCrate::new(vec![
            Instruction::Constant { out: 0, val: Value::Integer(1) },
        ]);
        
        // Tamper with hash
        krate.hash[0] ^= 0xFF;
        
        // Should fail validation
        assert!(execute(&krate).is_err());
    }
}
