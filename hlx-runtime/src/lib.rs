mod agent;
mod ast;
mod ast_parser;
mod bond;
mod builtins;
mod bytecode;
mod communication;
mod compiler;
mod dd_protocol;
mod debugger;
mod forgetting_guard;
mod governance;
mod homeostasis;
mod human_auth;
mod integrity;
mod lora_adapter;
mod lowerer;
mod memory_pool;
mod module_cache;
mod promotion;
mod resolver;
mod rsi;
mod scale;
mod shader_attestation;
mod tensor;
mod training_gate;
mod value;
mod vm;

pub use agent::{Agent, AgentPool, AgentState};
pub use ast::{
    walk_program, AgentDef, AstDiffBatch, ClusterDef, Function, Item, ModificationTarget, Mutation,
    MutationBatch, NodeCounter, NodeId, Program, Render, Statement, TypeAnnotation, VisitResult,
    Visitor,
};
pub use ast_parser::{AstParser, ParseError, Token};
pub use bond::{
    deserialize_response, deserialize_response_json, serialize_request, serialize_request_json,
    BondPhase, BondRequest, BondResponse, Capability, SymbioteState,
};
pub use bytecode::{Bytecode, Opcode};
pub use communication::{
    ChannelStats, CommunicationChannel, CommunicationChannelConfig, Message, TimestampedMessage,
};
pub use compiler::Compiler;
pub use dd_protocol::{
    DdError, DdOperation, DdProtocol, DdSnapshot, DdState, DdTarget, DdTargetType,
};
pub use debugger::{DapServer, DebugEvent, Debugger, StepMode, StopReason};
pub use forgetting_guard::{
    ForgettingError, ForgettingEvent, ForgettingGuard, HealthStatus, RetentionResult,
    RetentionTest, WeightImportance,
};
pub use governance::{
    ConfigError, Effect, EffectType, Governance, GovernanceConfig, GovernanceContext,
    GovernanceRegistry, PredicateResult,
};
pub use homeostasis::{GateDecision, HomeostasisGate, HomeostasisStatus, ImprovementAxis};
pub use human_auth::{
    AuthAuditEntry, AuthError, AuthorizationGate, HumanAuthToken, PendingRequest,
    ProtectedNamespace, RiskLevel,
};
pub use integrity::{
    CorpusHash, IntegrityEntry, IntegrityError, IntegritySystem, ProvenanceRecord,
    VerificationReport,
};
pub use lora_adapter::{
    AdapterError, AdapterMetadata, AdapterProvenance, AdapterRegistry, AdapterState,
    AdapterVersion, AdapterVersionHistory,
};
pub use lowerer::Lowerer;
pub use memory_pool::{
    Exchange, ExchangeRole, MemoryPool, MemoryPoolConfig, MemoryStats, Observation, Pattern,
    Question,
};
pub use module_cache::{CompiledModule, ImportStyle, ModuleCache};
pub use promotion::{
    CriteriaProgress, ModificationTypeClass, PromotionCriteria, PromotionGate, PromotionLevel,
};
pub use resolver::{ModuleResolver, ResolvedModule};
pub use rsi::{AgentMemory, ModificationType, ProposalStatus, RSIPipeline, RSIProposal};
pub use scale::{Barrier, Consensus, ConsensusResult, Scale, ScalePool};
pub use shader_attestation::{ShaderAttestationError, ShaderInfo, ShaderRegistry};
pub use tensor::{
    get_global_allocation, reset_global_allocation, set_global_limit, Tensor, TensorLimits,
    DEFAULT_MAX_TENSOR_ELEMENTS,
};
pub use training_gate::{
    CheckResult, CheckpointData, GateResult, GateStage, TrainingGate, TrainingProposal,
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
