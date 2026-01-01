//! HLX Capsule
//!
//! A Capsule is the compiled unit of HLX:
//! - Contains a sequence of instructions
//! - Integrity-protected with BLAKE3 hash
//! - Version-tagged for compatibility
//! - Metadata for debugging/inspection
//!
//! ## Invariants
//! - `validate()` must pass for any legally-constructed Capsule
//! - Hash computed over serialized instructions only
//! - Same instructions always produce same hash (determinism)

use serde::{Deserialize, Serialize};
use crate::instruction::Instruction;
use crate::error::{HlxError, Result};
use crate::CAPSULE_VERSION;

/// A compiled HLX program with integrity verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capsule {
    /// Format version (for future compatibility)
    pub version: u8,
    
    /// The instruction sequence
    pub instructions: Vec<Instruction>,
    
    /// BLAKE3 hash of serialized instructions
    pub hash: [u8; 32],
    
    /// Optional metadata (not included in hash)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<CapsuleMetadata>,
}

/// Optional metadata for debugging and introspection
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CapsuleMetadata {
    /// Source file name (if known)
    pub source_file: Option<String>,
    
    /// Compilation timestamp (ISO 8601)
    pub compiled_at: Option<String>,
    
    /// Compiler version
    pub compiler_version: Option<String>,
    
    /// Register count (for VM allocation)
    pub register_count: Option<u32>,
    
    /// Debug symbols (instruction index -> source location)
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub debug_symbols: Vec<DebugSymbol>,
}

/// Debug symbol mapping instruction to source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugSymbol {
    /// Instruction index
    pub inst_idx: usize,
    /// Source line number
    pub line: u32,
    /// Source column
    pub col: u32,
}

impl Capsule {
    /// Create a new Capsule from instructions
    ///
    /// Computes the BLAKE3 hash automatically.
    pub fn new(instructions: Vec<Instruction>) -> Self {
        let hash = Self::compute_hash(&instructions);
        Self {
            version: CAPSULE_VERSION,
            instructions,
            hash,
            metadata: None,
        }
    }

    /// Create a Capsule with metadata
    pub fn with_metadata(instructions: Vec<Instruction>, metadata: CapsuleMetadata) -> Self {
        let hash = Self::compute_hash(&instructions);
        Self {
            version: CAPSULE_VERSION,
            instructions,
            hash,
            metadata: Some(metadata),
        }
    }

    /// Compute BLAKE3 hash of instructions
    fn compute_hash(instructions: &[Instruction]) -> [u8; 32] {
        let bytes = bincode::serialize(instructions)
            .expect("Instruction serialization should never fail");
        *blake3::hash(&bytes).as_bytes()
    }

    /// Validate capsule integrity
    ///
    /// Returns Ok(()) if hash matches, Err otherwise.
    pub fn validate(&self) -> Result<()> {
        // Check version
        if self.version != CAPSULE_VERSION {
            return Err(HlxError::CapsuleVersion { version: self.version });
        }

        // Verify hash
        let computed = Self::compute_hash(&self.instructions);
        if computed != self.hash {
            return Err(HlxError::CapsuleInvalid);
        }

        Ok(())
    }

    /// Check if capsule is valid (non-error version of validate)
    pub fn is_valid(&self) -> bool {
        self.validate().is_ok()
    }

    /// Get the hash as a hex string
    pub fn hash_hex(&self) -> String {
        hex_encode(&self.hash)
    }

    /// Number of instructions
    pub fn len(&self) -> usize {
        self.instructions.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }

    /// Serialize to LC-B binary format
    pub fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).expect("Capsule serialization should never fail")
    }

    /// Deserialize from LC-B binary format
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let capsule: Capsule = bincode::deserialize(bytes)
            .map_err(|e| HlxError::LcBinaryDecode { reason: e.to_string() })?;
        
        // Validate after deserialization
        capsule.validate()?;
        
        Ok(capsule)
    }

    /// Compute the maximum register used
    pub fn max_register(&self) -> u32 {
        let mut max = 0;
        for inst in &self.instructions {
            if let Some(out) = inst.output_register() {
                max = max.max(out);
            }
            for reg in inst.input_registers() {
                max = max.max(reg);
            }
        }
        max
    }

    /// Add metadata to this capsule
    pub fn set_metadata(&mut self, metadata: CapsuleMetadata) {
        self.metadata = Some(metadata);
    }

    /// Verify that all input registers are defined before use
    pub fn validate_registers(&self) -> Result<()> {
        use std::collections::HashSet;
        let mut defined: HashSet<u32> = HashSet::new();

        for (idx, inst) in self.instructions.iter().enumerate() {
            // Check that all inputs are defined
            for reg in inst.input_registers() {
                if !defined.contains(&reg) {
                    return Err(HlxError::ValidationFail {
                        message: format!(
                            "Register {} used at instruction {} before definition",
                            reg, idx
                        ),
                    });
                }
            }

            // Mark output as defined
            if let Some(out) = inst.output_register() {
                defined.insert(out);
            }
        }

        Ok(())
    }
}

impl PartialEq for Capsule {
    fn eq(&self, other: &Self) -> bool {
        // Compare by hash (content-addressed equality)
        self.hash == other.hash
    }
}

impl Eq for Capsule {}

impl std::hash::Hash for Capsule {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.hash.hash(state);
    }
}

/// Encode bytes as hex string
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Decode hex string to bytes
#[allow(dead_code)]
fn hex_decode(s: &str) -> Result<Vec<u8>> {
    if s.len() % 2 != 0 {
        return Err(HlxError::parse("Invalid hex string: odd length"));
    }
    
    (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16)
                .map_err(|_| HlxError::parse(format!("Invalid hex at position {}", i)))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Value;

    fn sample_instructions() -> Vec<Instruction> {
        vec![
            Instruction::Constant { out: 0, val: Value::Integer(5) },
            Instruction::Constant { out: 1, val: Value::Integer(3) },
            Instruction::Add { out: 2, lhs: 0, rhs: 1 },
            Instruction::Return { val: 2 },
        ]
    }

    #[test]
    fn test_capsule_creation() {
        let capsule = Capsule::new(sample_instructions());
        assert_eq!(capsule.version, CAPSULE_VERSION);
        assert_eq!(capsule.len(), 4);
        assert!(capsule.is_valid());
    }

    #[test]
    fn test_capsule_determinism() {
        // Same instructions must produce same hash
        let cap1 = Capsule::new(sample_instructions());
        let cap2 = Capsule::new(sample_instructions());
        assert_eq!(cap1.hash, cap2.hash);
        assert_eq!(cap1, cap2);
    }

    #[test]
    fn test_capsule_validation() {
        let capsule = Capsule::new(sample_instructions());
        assert!(capsule.validate().is_ok());

        // Tamper with hash
        let mut tampered = capsule.clone();
        tampered.hash[0] ^= 0xFF;
        assert!(tampered.validate().is_err());
    }

    #[test]
    fn test_capsule_serialization() {
        let original = Capsule::new(sample_instructions());
        let bytes = original.to_bytes();
        let restored = Capsule::from_bytes(&bytes).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn test_register_validation() {
        // Valid: registers defined before use
        let valid = Capsule::new(sample_instructions());
        assert!(valid.validate_registers().is_ok());

        // Invalid: register used before definition
        let invalid = Capsule::new(vec![
            Instruction::Add { out: 1, lhs: 0, rhs: 99 }, // reg 99 undefined
        ]);
        assert!(invalid.validate_registers().is_err());
    }

    #[test]
    fn test_max_register() {
        let capsule = Capsule::new(sample_instructions());
        assert_eq!(capsule.max_register(), 2);
    }
}
