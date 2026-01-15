//! Substrate and Swarm Support for HLX-S
//!
//! Defines the execution substrates (CPU, quantum sim, quantum hardware)
//! and swarm configuration for parallel execution.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Execution substrate for HLX-S
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Substrate {
    /// Classical CPU execution (default)
    CPU,
    /// Quantum-inspired simulation (speculation + barriers)
    QuantumSim,
    /// Real quantum hardware (Qiskit/Cirq backend)
    QuantumHardware,
    /// Hybrid (mix of CPU and quantum)
    Hybrid,
    /// Inferred by compiler (not yet determined)
    Inferred,
}

impl Substrate {
    /// Parse substrate from string (for pragma parsing)
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "cpu" => Some(Substrate::CPU),
            "quantum_sim" | "quantum" => Some(Substrate::QuantumSim),
            "quantum_hardware" => Some(Substrate::QuantumHardware),
            "hybrid" => Some(Substrate::Hybrid),
            "inferred" => Some(Substrate::Inferred),
            _ => None,
        }
    }

    /// Convert substrate to string
    pub fn to_str(&self) -> &'static str {
        match self {
            Substrate::CPU => "cpu",
            Substrate::QuantumSim => "quantum_sim",
            Substrate::QuantumHardware => "quantum_hardware",
            Substrate::Hybrid => "hybrid",
            Substrate::Inferred => "inferred",
        }
    }

    /// Check if substrate requires quantum capabilities
    pub fn is_quantum(&self) -> bool {
        matches!(self, Substrate::QuantumSim | Substrate::QuantumHardware | Substrate::Hybrid)
    }

    /// Check if substrate preserves full determinism (A1)
    pub fn is_deterministic(&self) -> bool {
        !matches!(self, Substrate::QuantumHardware)
    }

    /// Check if substrate preserves full reversibility (A2)
    pub fn is_reversible(&self) -> bool {
        !matches!(self, Substrate::QuantumHardware)
    }
}

impl Default for Substrate {
    fn default() -> Self {
        Substrate::Inferred
    }
}

/// Swarm configuration for parallel execution
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScaleConfig {
    /// Number of parallel agents/tasks
    pub size: ScaleSize,
    /// Optional substrate override
    pub substrate: Option<Substrate>,
    /// Optional memory limit per agent
    pub memory_limit: Option<String>,
    /// Optional barrier name for synchronization
    pub barrier: Option<String>,
    /// Optional sync protocol (tree, gossip, merkle)
    pub sync_protocol: Option<String>,
}

/// Swarm size specification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ScaleSize {
    /// Fixed number (e.g., 1000)
    Fixed(u64),
    /// Exponential (e.g., 2^50 for quantum)
    Exponential { base: u64, exponent: u64 },
}

impl ScaleSize {
    /// Parse swarm size from string
    pub fn parse(s: &str) -> Option<Self> {
        if s.contains('^') {
            // Parse exponential: "2^50"
            let parts: Vec<&str> = s.split('^').collect();
            if parts.len() == 2 {
                let base = parts[0].parse::<u64>().ok()?;
                let exponent = parts[1].parse::<u64>().ok()?;
                return Some(ScaleSize::Exponential { base, exponent });
            }
        }

        // Parse fixed
        s.parse::<u64>().ok().map(ScaleSize::Fixed)
    }

    /// Get the actual size (if computable)
    pub fn to_u64(&self) -> Option<u64> {
        match self {
            ScaleSize::Fixed(n) => Some(*n),
            ScaleSize::Exponential { base, exponent } => {
                // Only compute if result fits in u64
                if *exponent > 63 {
                    None  // Too large
                } else {
                    base.checked_pow(*exponent as u32)
                }
            }
        }
    }

    /// Check if size suggests quantum execution
    pub fn suggests_quantum(&self) -> bool {
        match self {
            ScaleSize::Exponential { .. } => true,  // Exponential sizes are quantum hints
            ScaleSize::Fixed(n) => *n > 10_000,     // Very large swarms might benefit from quantum
        }
    }
}

/// Substrate inference information (for diagnostics)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubstrateInfo {
    /// Inferred or explicitly declared substrate
    pub substrate: Substrate,
    /// Confidence of inference (0.0 - 1.0)
    pub inference_confidence: f64,
    /// Expected speedup vs serial (if known)
    pub speedup_estimate: Option<f64>,
    /// Number of agents/tasks
    pub agent_count: Option<u64>,
    /// Number of synchronization barriers
    pub barrier_count: usize,
    /// Human-readable reasoning for the inference
    pub reasoning: String,
    /// Hash of AST used for inference (for determinism)
    pub ast_hash: Option<String>,
}

impl Default for SubstrateInfo {
    fn default() -> Self {
        Self {
            substrate: Substrate::CPU,
            inference_confidence: 1.0,
            speedup_estimate: None,
            agent_count: None,
            barrier_count: 0,
            reasoning: "Default to CPU".to_string(),
            ast_hash: None,
        }
    }
}

/// Operation substrate hints (vocabulary for inference)
#[derive(Debug, Clone)]
pub struct OperationHints {
    /// Map of operation name to substrate hint
    hints: HashMap<String, Substrate>,
}

impl OperationHints {
    /// Create default operation hints
    pub fn new() -> Self {
        let mut hints = HashMap::new();

        // Math operations → CPU by default
        for op in ["sqrt", "sin", "cos", "tan", "log", "exp", "abs", "max", "min"] {
            hints.insert(op.to_string(), Substrate::CPU);
        }

        // Tensor operations → Quantum sim hint
        for op in ["tensor_matmul", "tensor_add", "tensor_mult", "hypot", "cbrt"] {
            hints.insert(op.to_string(), Substrate::QuantumSim);
        }

        // Array operations → CPU (but could be parallelized)
        for op in ["map", "filter", "reduce", "sort", "reverse"] {
            hints.insert(op.to_string(), Substrate::CPU);
        }

        Self { hints }
    }

    /// Get substrate hint for an operation
    pub fn get_hint(&self, op: &str) -> Option<Substrate> {
        self.hints.get(op).copied()
    }

    /// Add a custom hint
    pub fn add_hint(&mut self, op: String, substrate: Substrate) {
        self.hints.insert(op, substrate);
    }
}

impl Default for OperationHints {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_substrate_parse() {
        assert_eq!(Substrate::parse("cpu"), Some(Substrate::CPU));
        assert_eq!(Substrate::parse("quantum_sim"), Some(Substrate::QuantumSim));
        assert_eq!(Substrate::parse("quantum_hardware"), Some(Substrate::QuantumHardware));
        assert_eq!(Substrate::parse("invalid"), None);
    }

    #[test]
    fn test_swarm_size_parse() {
        assert_eq!(ScaleSize::parse("1000"), Some(ScaleSize::Fixed(1000)));
        assert_eq!(
            ScaleSize::parse("2^50"),
            Some(ScaleSize::Exponential { base: 2, exponent: 50 })
        );
    }

    #[test]
    fn test_swarm_size_quantum_hint() {
        let fixed_small = ScaleSize::Fixed(100);
        let fixed_large = ScaleSize::Fixed(100_000);
        let exponential = ScaleSize::Exponential { base: 2, exponent: 50 };

        assert!(!fixed_small.suggests_quantum());
        assert!(fixed_large.suggests_quantum());
        assert!(exponential.suggests_quantum());
    }

    #[test]
    fn test_substrate_properties() {
        assert!(Substrate::CPU.is_deterministic());
        assert!(Substrate::CPU.is_reversible());
        assert!(Substrate::QuantumSim.is_deterministic());
        assert!(Substrate::QuantumSim.is_reversible());
        assert!(!Substrate::QuantumHardware.is_deterministic());
        assert!(!Substrate::QuantumHardware.is_reversible());
    }
}
