//! # HLX Runtime
//!
//! Deterministic execution engine for LC-B capsules.
//!
//! ## Architecture
//!
//! ```text
//! LC-B Capsule
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
//! 1. Same capsule + same config = same result
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

#[cfg(feature = "vulkan")]
pub mod spirv_gen;

pub use config::RuntimeConfig;
pub use backend::Backend;
pub use executor::Executor;
pub use value_store::ValueStore;

use hlx_core::{Capsule, Value, Result, HlxError};

/// Execute a capsule with default configuration
pub fn execute(capsule: &Capsule) -> Result<Value> {
    let config = RuntimeConfig::default();
    execute_with_config(capsule, &config)
}

/// Execute a capsule with custom configuration
pub fn execute_with_config(capsule: &Capsule, config: &RuntimeConfig) -> Result<Value> {
    // Validate capsule integrity
    capsule.validate()?;
    
    // Create executor with appropriate backend
    let executor = Executor::new(config)?;
    
    // Run the capsule
    executor.run(capsule)
}

/// Quick test: 5 + 3 = 8
#[cfg(test)]
mod smoke_tests {
    use super::*;
    use hlx_core::{Capsule, Instruction, Value};

    #[test]
    fn test_basic_addition() {
        // 🜃5 + 🜃3 = 8
        let capsule = Capsule::new(vec![
            Instruction::Constant { out: 0, val: Value::Integer(5) },
            Instruction::Constant { out: 1, val: Value::Integer(3) },
            Instruction::Add { out: 2, lhs: 0, rhs: 1 },
            Instruction::Return { val: 2 },
        ]);
        
        let result = execute(&capsule).unwrap();
        assert_eq!(result, Value::Integer(8));
    }

    #[test]
    fn test_determinism() {
        let capsule = Capsule::new(vec![
            Instruction::Constant { out: 0, val: Value::Integer(42) },
            Instruction::Constant { out: 1, val: Value::Integer(10) },
            Instruction::Mul { out: 2, lhs: 0, rhs: 1 },
            Instruction::Return { val: 2 },
        ]);
        
        // Run 100 times
        let results: Vec<_> = (0..100)
            .map(|_| execute(&capsule).unwrap())
            .collect();
        
        // All must be identical
        assert!(results.iter().all(|r| *r == Value::Integer(420)));
    }

    #[test]
    fn test_capsule_validation() {
        let mut capsule = Capsule::new(vec![
            Instruction::Constant { out: 0, val: Value::Integer(1) },
        ]);
        
        // Tamper with hash
        capsule.hash[0] ^= 0xFF;
        
        // Should fail validation
        assert!(execute(&capsule).is_err());
    }
}
