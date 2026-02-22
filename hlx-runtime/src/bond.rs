//! Bond Protocol - Handshake between HLX Symbiote and LLM
//!
//! Phases: HELLO → SYNC → BOND → READY
//!
//! The bond protocol establishes a neurosymbolic connection between:
//! - HLX (symbolic runtime, deterministic, bounded)
//! - LLM (text model, statistical reasoning, language generation)

use crate::{RuntimeError, RuntimeResult, Value};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BondRequest {
    pub protocol_version: String,
    pub symbiote_version: String,
    pub capabilities: Vec<Capability>,
    pub initial_memory: HashMap<String, Value>,
    pub initial_latents: HashMap<String, Value>,
    pub governance_mode: String,
    pub max_steps: usize,
    pub rsi_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BondResponse {
    pub accepted: bool,
    pub model_name: String,
    pub model_version: String,
    pub context_window: usize,
    pub capabilities: Vec<Capability>,
    pub rejection_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbioteState {
    pub id: String,
    pub bonded: bool,
    pub model_name: Option<String>,
    pub phase: BondPhase,
    pub memory: HashMap<String, Value>,
    pub latents: HashMap<String, Value>,
    pub step_count: usize,
    pub max_steps: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BondPhase {
    Disconnected,
    Hello,
    Sync,
    Bond,
    Ready,
    Failed,
}

impl Default for SymbioteState {
    fn default() -> Self {
        SymbioteState {
            id: uuid::Uuid::new_v4().to_string(),
            bonded: false,
            model_name: None,
            phase: BondPhase::Disconnected,
            memory: HashMap::new(),
            latents: HashMap::new(),
            step_count: 0,
            max_steps: 1_000_000,
        }
    }
}

impl SymbioteState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_max_steps(mut self, max: usize) -> Self {
        self.max_steps = max;
        self
    }

    pub fn with_memory(mut self, key: &str, value: Value) -> Self {
        self.memory.insert(key.to_string(), value);
        self
    }

    pub fn with_latent(mut self, key: &str, value: Value) -> Self {
        self.latents.insert(key.to_string(), value);
        self
    }

    pub fn create_bond_request(&self) -> BondRequest {
        BondRequest {
            protocol_version: "1.0.0".to_string(),
            symbiote_version: env!("CARGO_PKG_VERSION").to_string(),
            capabilities: vec![
                Capability {
                    name: "tensor_ops".to_string(),
                    version: "1.0".to_string(),
                    description: Some("Tensor operations (matmul, softmax, etc.)".to_string()),
                },
                Capability {
                    name: "image_io".to_string(),
                    version: "1.0".to_string(),
                    description: Some("Image load/save (PNG, JPEG)".to_string()),
                },
                Capability {
                    name: "governance".to_string(),
                    version: "1.0".to_string(),
                    description: Some("Effect governance (flow/guard/shield/fortress)".to_string()),
                },
                Capability {
                    name: "rsi".to_string(),
                    version: "1.0".to_string(),
                    description: Some("Recursive self-improvement pipeline".to_string()),
                },
                Capability {
                    name: "scale".to_string(),
                    version: "1.0".to_string(),
                    description: Some("SCALE coordination (barriers, consensus)".to_string()),
                },
            ],
            initial_memory: self.memory.clone(),
            initial_latents: self.latents.clone(),
            governance_mode: "guard".to_string(),
            max_steps: self.max_steps,
            rsi_enabled: true,
        }
    }

    pub fn process_hello(&mut self, response: &BondResponse) -> RuntimeResult<()> {
        if !response.accepted {
            self.phase = BondPhase::Failed;
            return Err(RuntimeError::new(
                format!(
                    "Bond rejected: {}",
                    response.rejection_reason.as_deref().unwrap_or("unknown")
                ),
                0,
            ));
        }

        self.model_name = Some(response.model_name.clone());
        self.phase = BondPhase::Hello;
        Ok(())
    }

    pub fn process_sync(&mut self) -> RuntimeResult<()> {
        if self.phase != BondPhase::Hello {
            return Err(RuntimeError::new(
                format!("Invalid phase transition: {:?} → Sync", self.phase),
                0,
            ));
        }
        self.phase = BondPhase::Sync;
        Ok(())
    }

    pub fn process_bond(&mut self) -> RuntimeResult<()> {
        if self.phase != BondPhase::Sync {
            return Err(RuntimeError::new(
                format!("Invalid phase transition: {:?} → Bond", self.phase),
                0,
            ));
        }
        self.phase = BondPhase::Bond;
        self.bonded = true;
        Ok(())
    }

    pub fn process_ready(&mut self) -> RuntimeResult<()> {
        if self.phase != BondPhase::Bond {
            return Err(RuntimeError::new(
                format!("Invalid phase transition: {:?} → Ready", self.phase),
                0,
            ));
        }
        self.phase = BondPhase::Ready;
        Ok(())
    }

    pub fn is_ready(&self) -> bool {
        self.phase == BondPhase::Ready && self.bonded
    }

    pub fn to_context_string(&self) -> String {
        let mut ctx = String::new();
        ctx.push_str(&format!("# Symbiote State\n"));
        ctx.push_str(&format!("ID: {}\n", self.id));
        ctx.push_str(&format!("Phase: {:?}\n", self.phase));
        ctx.push_str(&format!("Bonded: {}\n", self.bonded));
        if let Some(ref model) = self.model_name {
            ctx.push_str(&format!("Model: {}\n", model));
        }
        ctx.push_str(&format!("Steps: {}/{}\n", self.step_count, self.max_steps));

        if !self.memory.is_empty() {
            ctx.push_str("\n## Memory\n");
            for (k, v) in &self.memory {
                ctx.push_str(&format!("- {}: {}\n", k, v));
            }
        }

        if !self.latents.is_empty() {
            ctx.push_str("\n## Latents\n");
            for (k, v) in &self.latents {
                ctx.push_str(&format!("- {}: {}\n", k, v));
            }
        }

        ctx
    }
}

pub fn serialize_request(request: &BondRequest) -> RuntimeResult<Vec<u8>> {
    bincode::serialize(request)
        .map_err(|e| RuntimeError::new(format!("Failed to serialize bond request: {}", e), 0))
}

pub fn deserialize_response(bytes: &[u8]) -> RuntimeResult<BondResponse> {
    bincode::deserialize(bytes)
        .map_err(|e| RuntimeError::new(format!("Failed to deserialize bond response: {}", e), 0))
}

pub fn serialize_request_json(request: &BondRequest) -> RuntimeResult<String> {
    serde_json::to_string_pretty(request).map_err(|e| {
        RuntimeError::new(
            format!("Failed to serialize bond request to JSON: {}", e),
            0,
        )
    })
}

pub fn deserialize_response_json(json: &str) -> RuntimeResult<BondResponse> {
    serde_json::from_str(json).map_err(|e| {
        RuntimeError::new(
            format!("Failed to deserialize bond response from JSON: {}", e),
            0,
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbiote_state_creation() {
        let state = SymbioteState::new();
        assert_eq!(state.phase, BondPhase::Disconnected);
        assert!(!state.bonded);
    }

    #[test]
    fn test_bond_request_creation() {
        let state = SymbioteState::new().with_memory("test_key", Value::I64(42));
        let request = state.create_bond_request();

        assert_eq!(request.protocol_version, "1.0.0");
        assert!(request.capabilities.len() >= 5);
        assert!(request.initial_memory.contains_key("test_key"));
    }

    #[test]
    fn test_phase_transitions() {
        let mut state = SymbioteState::new();

        let response = BondResponse {
            accepted: true,
            model_name: "test-model".to_string(),
            model_version: "1.0".to_string(),
            context_window: 4096,
            capabilities: vec![],
            rejection_reason: None,
        };

        state.process_hello(&response).unwrap();
        assert_eq!(state.phase, BondPhase::Hello);

        state.process_sync().unwrap();
        assert_eq!(state.phase, BondPhase::Sync);

        state.process_bond().unwrap();
        assert_eq!(state.phase, BondPhase::Bond);
        assert!(state.bonded);

        state.process_ready().unwrap();
        assert_eq!(state.phase, BondPhase::Ready);
        assert!(state.is_ready());
    }

    #[test]
    fn test_invalid_phase_transition() {
        let mut state = SymbioteState::new();

        let result = state.process_sync();
        assert!(result.is_err());
    }

    #[test]
    fn test_rejected_bond() {
        let mut state = SymbioteState::new();

        let response = BondResponse {
            accepted: false,
            model_name: "".to_string(),
            model_version: "".to_string(),
            context_window: 0,
            capabilities: vec![],
            rejection_reason: Some("Incompatible version".to_string()),
        };

        let result = state.process_hello(&response);
        assert!(result.is_err());
        assert_eq!(state.phase, BondPhase::Failed);
    }

    #[test]
    fn test_request_serialization() {
        let state = SymbioteState::new();
        let request = state.create_bond_request();

        let bytes = serialize_request(&request).unwrap();
        assert!(!bytes.is_empty());

        let json = serialize_request_json(&request).unwrap();
        assert!(json.contains("protocol_version"));
    }

    #[test]
    fn test_context_string() {
        let state = SymbioteState::new()
            .with_memory("name", Value::String("test".to_string()))
            .with_latent("z", Value::F64(0.5));

        let ctx = state.to_context_string();
        assert!(ctx.contains("Symbiote State"));
        assert!(ctx.contains("name"));
        assert!(ctx.contains("z"));
    }
}
