//! HLX Crate
//!
//! A Crate is the compiled unit of HLX:
//! - Contains a sequence of instructions
//! - Integrity-protected with BLAKE3 hash
//! - Version-tagged for compatibility
//! - Metadata for debugging/inspection
//!
//! ## Invariants
//! - `validate()` must pass for any legally-constructed Crate
//! - Hash computed over serialized instructions only
//! - Same instructions always produce same hash (determinism)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::instruction::{Instruction, DType};
use crate::error::{HlxError, Result};
use crate::CRATE_VERSION;

/// A compiled HLX program with integrity verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HlxCrate {
    /// Format version (for future compatibility)
    pub version: u8,
    
    /// The instruction sequence
    pub instructions: Vec<Instruction>,
    
    /// BLAKE3 hash of serialized instructions
    pub hash: [u8; 32],
    
    /// Optional metadata (not included in hash)
    pub metadata: Option<CrateMetadata>,
}

/// Optional metadata for debugging and introspection
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CrateMetadata {
    /// Source file name (if known)
    pub source_file: Option<String>,

    /// Compilation timestamp (ISO 8601)
    pub compiled_at: Option<String>,

    /// Compiler version
    pub compiler_version: Option<String>,

    /// Register count (for VM allocation)
    pub register_count: Option<u32>,

    /// Function signatures: name -> [param_types]
    #[serde(default)]
    pub function_signatures: HashMap<String, Vec<DType>>,

    /// Debug symbols (instruction index -> source location)
    #[serde(default)]
    pub debug_symbols: Vec<DebugSymbol>,

    /// HLX-Scale substrate information: function name -> substrate config
    /// Used by runtime to route @swarm functions to speculation coordinator
    #[serde(default)]
    pub hlx_scale_substrates: HashMap<String, HlxScaleInfo>,
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

/// HLX-Scale configuration for a function (embedded in crate metadata)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HlxScaleInfo {
    /// Enable speculation for this function
    pub enable_speculation: bool,
    /// Number of agents to spawn (if speculation enabled)
    pub agent_count: usize,
    /// Substrate type (for diagnostics)
    pub substrate: String,
    /// Number of barriers in function body
    pub barrier_count: usize,
}

impl HlxCrate {
    /// Create a new Crate from instructions
    ///
    /// Computes the BLAKE3 hash automatically.
    pub fn new(instructions: Vec<Instruction>) -> Self {
        let hash = Self::compute_hash(&instructions);
        Self {
            version: CRATE_VERSION,
            instructions,
            hash,
            metadata: None,
        }
    }

    /// Create a Crate with metadata
    pub fn with_metadata(instructions: Vec<Instruction>, metadata: CrateMetadata) -> Self {
        let hash = Self::compute_hash(&instructions);
        Self {
            version: CRATE_VERSION,
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

    /// Validate crate integrity
    ///
    /// Returns Ok(()) if hash matches, Err otherwise.
    pub fn validate(&self) -> Result<()> {
        // Check version
        if self.version != CRATE_VERSION {
            return Err(HlxError::CrateVersion { version: self.version });
        }

        // Verify hash
        let computed = Self::compute_hash(&self.instructions);
        if computed != self.hash {
            return Err(HlxError::CrateInvalid);
        }

        Ok(())
    }

    /// Check if crate is valid (non-error version of validate)
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
        bincode::serialize(self).expect("Crate serialization should never fail")
    }

    /// Deserialize from LC-B binary format
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let krate: HlxCrate = bincode::deserialize(bytes)
            .map_err(|e| HlxError::LcBinaryDecode { reason: e.to_string() })?;
        
        // Validate after deserialization
        krate.validate()?;
        
        Ok(krate)
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

    /// Add metadata to this crate
    pub fn set_metadata(&mut self, metadata: CrateMetadata) {
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

impl PartialEq for HlxCrate {
    fn eq(&self, other: &Self) -> bool {
        // Compare by hash (content-addressed equality)
        self.hash == other.hash
    }
}

impl Eq for HlxCrate {}

impl std::hash::Hash for HlxCrate {
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
    fn test_crate_creation() {
        let krate = HlxCrate::new(sample_instructions());
        assert_eq!(krate.version, CRATE_VERSION);
        assert_eq!(krate.len(), 4);
        assert!(krate.is_valid());
    }

    #[test]
    fn test_crate_determinism() {
        // Same instructions must produce same hash
        let c1 = HlxCrate::new(sample_instructions());
        let c2 = HlxCrate::new(sample_instructions());
        assert_eq!(c1.hash, c2.hash);
        assert_eq!(c1, c2);
    }

    #[test]
    fn test_crate_validation() {
        let krate = HlxCrate::new(sample_instructions());
        assert!(krate.validate().is_ok());

        // Tamper with hash
        let mut tampered = krate.clone();
        tampered.hash[0] ^= 0xFF;
        assert!(tampered.validate().is_err());
    }

    #[test]
    fn test_crate_serialization() {
        let original = HlxCrate::new(sample_instructions());
        let bytes = original.to_bytes();
        let restored = HlxCrate::from_bytes(&bytes).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn test_register_validation() {
        // Valid: registers defined before use
        let valid = HlxCrate::new(sample_instructions());
        assert!(valid.validate_registers().is_ok());

        // Invalid: register used before definition
        let invalid = HlxCrate::new(vec![
            Instruction::Add { out: 1, lhs: 0, rhs: 99 }, // reg 99 undefined
        ]);
        assert!(invalid.validate_registers().is_err());
    }

    #[test]
    fn test_max_register() {
        let krate = HlxCrate::new(sample_instructions());
        assert_eq!(krate.max_register(), 2);
    }
}
