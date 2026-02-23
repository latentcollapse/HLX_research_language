//! Agent definition AST nodes
//!
//! Recursive agents are the core computational unit in HLX.
//! They combine latent state, cycle-based reasoning, governance, and self-modification.

use super::rsi::GovernDef;
use super::rsi::ModifyDef;
use super::{Attribute, NodeId, Parameter, SourceSpan, Statement, TypeAnnotation};
use serde::{Deserialize, Serialize};

/// Recursive agent definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDef {
    pub id: NodeId,
    pub span: SourceSpan,
    pub name: String,
    /// Latent state declarations
    pub latents: Vec<LatentDef>,
    /// Intent input parameters
    pub takes: Vec<Parameter>,
    /// Intent output specification
    pub gives: Option<IntentOutput>,
    /// TRM-style cycle definitions
    pub cycles: Vec<CycleDef>,
    /// Main agent body
    pub body: Vec<Statement>,
    /// Governance configuration
    pub govern: Option<GovernDef>,
    /// Self-modification configuration
    pub modify: Option<ModifyDef>,
    /// Agent attributes
    pub attributes: Vec<Attribute>,
    /// Whether this agent can dissolve
    pub dissolvable: bool,
    /// Dissolution handler
    pub on_dissolve: Option<Vec<Statement>>,
}

impl AgentDef {
    pub fn new(name: impl Into<String>) -> Self {
        AgentDef {
            id: NodeId::new(),
            span: SourceSpan::unknown(),
            name: name.into(),
            latents: Vec::new(),
            takes: Vec::new(),
            gives: None,
            cycles: Vec::new(),
            body: Vec::new(),
            govern: None,
            modify: None,
            attributes: Vec::new(),
            dissolvable: false,
            on_dissolve: None,
        }
    }

    pub fn with_latent(mut self, latent: LatentDef) -> Self {
        self.latents.push(latent);
        self
    }

    pub fn with_cycle(mut self, cycle: CycleDef) -> Self {
        self.cycles.push(cycle);
        self
    }

    pub fn with_govern(mut self, govern: GovernDef) -> Self {
        self.govern = Some(govern);
        self
    }

    pub fn with_modify(mut self, modify: ModifyDef) -> Self {
        self.modify = Some(modify);
        self
    }

    pub fn dissolvable(mut self) -> Self {
        self.dissolvable = true;
        self
    }
}

/// Latent state declaration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatentDef {
    pub id: NodeId,
    pub span: SourceSpan,
    pub name: String,
    pub ty: TypeAnnotation,
    pub initializer: Option<super::Expression>,
    /// Whether this latent persists across cycles
    pub persistent: bool,
}

impl LatentDef {
    pub fn new(name: impl Into<String>, ty: TypeAnnotation) -> Self {
        LatentDef {
            id: NodeId::new(),
            span: SourceSpan::unknown(),
            name: name.into(),
            ty,
            initializer: None,
            persistent: true,
        }
    }

    pub fn with_initializer(mut self, init: super::Expression) -> Self {
        self.initializer = Some(init);
        self
    }
}

/// Intent output specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentOutput {
    pub name: String,
    pub ty: TypeAnnotation,
}

/// TRM-style cycle definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleDef {
    pub id: NodeId,
    pub span: SourceSpan,
    /// Cycle level: "H" (hypothesis) or "L" (detail)
    pub level: CycleLevel,
    /// Number of iterations
    pub iterations: u64,
    /// Cycle body
    pub body: Vec<Statement>,
    /// Accumulation mode
    pub accumulation: AccumulationMode,
}

impl CycleDef {
    pub fn new(level: CycleLevel, iterations: u64, body: Vec<Statement>) -> Self {
        CycleDef {
            id: NodeId::new(),
            span: SourceSpan::unknown(),
            level,
            iterations,
            body,
            accumulation: AccumulationMode::default(),
        }
    }
}

/// TRM cycle levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CycleLevel {
    /// High-level hypothesis cycles
    H,
    /// Low-level detail cycles
    L,
    /// Named custom level
    Custom(u32),
}

impl CycleLevel {
    pub fn name(&self) -> String {
        match self {
            CycleLevel::H => "H".to_string(),
            CycleLevel::L => "L".to_string(),
            CycleLevel::Custom(n) => format!("level_{}", n),
        }
    }
}

/// How results accumulate across cycles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccumulationMode {
    /// Last value wins
    Last,
    /// Sum of all values
    Sum,
    /// Product of all values
    Product,
    /// Maximum value
    Max,
    /// Minimum value
    Min,
    /// Collect all values
    Collect,
}

impl Default for AccumulationMode {
    fn default() -> Self {
        AccumulationMode::Last
    }
}

/// Halt condition for an agent
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HaltCondition {
    pub id: NodeId,
    pub span: SourceSpan,
    /// Confidence threshold for halting
    pub confidence_threshold: Option<f64>,
    /// Maximum steps before forced halt
    pub max_steps: Option<u64>,
    /// Maximum wall-clock time
    pub max_time_ms: Option<u64>,
    /// Custom halt predicate
    pub predicate: Option<super::Expression>,
}

#[allow(dead_code)]
impl HaltCondition {
    pub fn new() -> Self {
        HaltCondition {
            id: NodeId::new(),
            span: SourceSpan::unknown(),
            confidence_threshold: None,
            max_steps: None,
            max_time_ms: None,
            predicate: None,
        }
    }

    pub fn with_confidence(mut self, threshold: f64) -> Self {
        self.confidence_threshold = Some(threshold);
        self
    }

    pub fn with_max_steps(mut self, steps: u64) -> Self {
        self.max_steps = Some(steps);
        self
    }

    pub fn should_halt(&self, confidence: f64, steps: u64) -> bool {
        if let Some(threshold) = self.confidence_threshold {
            if confidence >= threshold {
                return true;
            }
        }
        if let Some(max) = self.max_steps {
            if steps >= max {
                return true;
            }
        }
        false
    }
}

/// SCALE cluster definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterDef {
    pub id: NodeId,
    pub span: SourceSpan,
    pub name: String,
    /// Agents in this cluster
    pub agents: Vec<AgentRef>,
    /// Barriers for synchronization
    pub barriers: Vec<BarrierDef>,
    /// Channels for communication
    pub channels: Vec<ChannelDef>,
}

/// Reference to an agent in a cluster
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRef {
    pub id: NodeId,
    pub name: String,
    pub role: Option<String>,
}

/// Barrier definition for synchronization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarrierDef {
    pub id: NodeId,
    pub name: String,
    pub expected: usize,
    pub timeout_ms: Option<u64>,
}

/// Channel definition for communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelDef {
    pub id: NodeId,
    pub name: String,
    pub direction: ChannelDirection,
    pub message_type: TypeAnnotation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChannelDirection {
    Send,
    Receive,
    Bidirectional,
}

/// Action definition (named, governable operation)
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionDef {
    pub id: NodeId,
    pub span: SourceSpan,
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub body: Vec<Statement>,
    pub govern: Option<GovernDef>,
}

#[allow(dead_code)]
impl ActionDef {
    pub fn new(name: impl Into<String>, parameters: Vec<Parameter>, body: Vec<Statement>) -> Self {
        ActionDef {
            id: NodeId::new(),
            span: SourceSpan::unknown(),
            name: name.into(),
            parameters,
            body,
            govern: None,
        }
    }
}
