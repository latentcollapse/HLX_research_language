mod agent;
mod bond;
mod builtins;
mod bytecode;
mod compiler;
mod governance;
mod rsi;
mod scale;
mod shader_attestation;
mod tensor;
mod value;
mod vm;

pub use agent::{Agent, AgentPool, AgentState};
pub use bond::{
    deserialize_response, deserialize_response_json, serialize_request, serialize_request_json,
    BondPhase, BondRequest, BondResponse, Capability, SymbioteState,
};
pub use bytecode::{Bytecode, Opcode};
pub use compiler::Compiler;
pub use governance::{
    ConfigError, Effect, EffectType, Governance, GovernanceConfig, GovernanceContext,
    GovernanceRegistry, PredicateResult,
};
pub use rsi::{AgentMemory, ModificationType, ProposalStatus, RSIPipeline, RSIProposal};
pub use scale::{Barrier, Consensus, ConsensusResult, Scale, ScalePool};
pub use shader_attestation::{ShaderAttestationError, ShaderInfo, ShaderRegistry};
pub use tensor::{
    get_global_allocation, reset_global_allocation, set_global_limit, Tensor, TensorLimits,
    DEFAULT_MAX_TENSOR_ELEMENTS,
};
pub use value::Value;
pub use vm::Vm;

#[derive(Debug, Clone)]
pub struct RuntimeError {
    pub message: String,
    pub pc: usize,
}

impl RuntimeError {
    pub fn new(message: impl Into<String>, pc: usize) -> Self {
        RuntimeError {
            message: message.into(),
            pc,
        }
    }
}

pub type RuntimeResult<T> = Result<T, RuntimeError>;
