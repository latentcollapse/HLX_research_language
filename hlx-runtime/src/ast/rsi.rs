//! RSI (Recursive Self-Improvement) AST nodes
//!
//! These nodes represent the self-modification capabilities of HLX agents.
//! RSI is a core thesis feature: agents that can safely modify themselves.

use super::{Expression, NodeId, SourceSpan};
use serde::{Deserialize, Serialize};

/// Governance block: defines conscience predicates and effect types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernDef {
    pub id: NodeId,
    pub span: SourceSpan,
    /// The effect type being governed
    pub effect: EffectType,
    /// Named conscience predicates that must pass
    pub conscience: Vec<ConsciencePredicate>,
    /// Trust level required (0.0 - 1.0)
    pub trust_threshold: f64,
}

/// Types of effects an agent can produce
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffectType {
    Modify,
    Spawn,
    Dissolve,
    Communicate,
    SelfModify,
    ExternalCall,
}

impl EffectType {
    pub fn name(&self) -> &'static str {
        match self {
            EffectType::Modify => "modify",
            EffectType::Spawn => "spawn",
            EffectType::Dissolve => "dissolve",
            EffectType::Communicate => "communicate",
            EffectType::SelfModify => "self_modify",
            EffectType::ExternalCall => "external_call",
        }
    }

    /// Default severity for this effect type
    pub fn default_severity(&self) -> f64 {
        match self {
            EffectType::Modify => 0.3,
            EffectType::Spawn => 0.5,
            EffectType::Dissolve => 0.8,
            EffectType::Communicate => 0.2,
            EffectType::SelfModify => 0.9,
            EffectType::ExternalCall => 0.7,
        }
    }

    /// Whether this effect type is reversible
    pub fn is_reversible(&self) -> bool {
        match self {
            EffectType::Modify => true,
            EffectType::Spawn => true,
            EffectType::Dissolve => false,
            EffectType::Communicate => true,
            EffectType::SelfModify => false,
            EffectType::ExternalCall => false,
        }
    }
}

/// A named conscience predicate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsciencePredicate {
    pub id: NodeId,
    pub name: String,
    pub kind: PredicateKind,
    pub enabled: bool,
}

/// Built-in predicate kinds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PredicateKind {
    /// Path safety: no access to forbidden paths
    PathSafety {
        allowed: Vec<String>,
        denied: Vec<String>,
    },
    /// No data exfiltration
    NoExfiltrate,
    /// No harm to self or others
    NoHarm,
    /// No bypassing governance
    NoBypass,
    /// Rate limiting
    RateLimit {
        max_per_window: u64,
        window_seconds: u64,
    },
    /// Custom predicate with expression
    Custom(Expression),
    /// Confidence threshold
    MinConfidence { threshold: f64 },
    /// Step count threshold
    MinSteps { threshold: u64 },
    /// Cycle depth limit
    MaxCycleDepth { limit: u32 },
}

/// Modify self block: defines how an agent can modify itself
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModifyDef {
    pub id: NodeId,
    pub span: SourceSpan,
    /// Gates that must be passed before modification
    pub gates: Vec<Gate>,
    /// Budget for modifications (e.g., max changes per epoch)
    pub budget: ModificationBudget,
    /// Cooldown period between modifications
    pub cooldown_steps: u64,
    /// The proposals being made
    pub proposals: Vec<ModificationProposal>,
}

/// A single modification proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModificationProposal {
    pub id: NodeId,
    pub kind: ModificationKind,
    pub target: ModificationTarget,
    pub rationale: String,
    pub confidence: f64,
    pub approved: bool,
}

/// What kind of modification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModificationKind {
    /// Change a parameter value
    ParameterChange {
        name: String,
        old_value: f64,
        new_value: f64,
    },
    /// Add a new behavior pattern
    AddBehavior {
        pattern: Vec<f64>,
        response: Vec<f64>,
    },
    /// Remove a behavior pattern
    RemoveBehavior { index: usize },
    /// Change cycle configuration
    CycleChange { h_cycles: u32, l_cycles: u32 },
    /// Adjust a threshold
    ThresholdChange {
        name: String,
        old_value: f64,
        new_value: f64,
    },
    /// Modify weight matrix
    WeightUpdate { layer: usize, deltas: Vec<f64> },
    /// Add a new rule
    RuleAdd {
        name: String,
        description: String,
        confidence: f64,
    },
    /// Remove a rule
    RuleRemove { name: String },
    /// Update an existing rule
    RuleUpdate {
        name: String,
        description: String,
        confidence: f64,
    },
    /// AST transformation (for RSI)
    AstTransform { description: String, diff: AstDiff },
}

/// Target of a modification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModificationTarget {
    SelfAgent,
    Agent(NodeId),
    Global(String),
}

/// AST diff for RSI modifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstDiff {
    pub operations: Vec<AstOperation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AstOperation {
    Insert {
        parent: NodeId,
        position: usize,
        node_type: String,
    },
    Delete {
        node: NodeId,
    },
    Replace {
        node: NodeId,
        replacement_type: String,
    },
    Move {
        node: NodeId,
        new_parent: NodeId,
        new_position: usize,
    },
}

/// A gate that must be passed for modification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Gate {
    /// Mathematical proof required
    Proof {
        name: String,
        verification_status: Option<bool>,
    },
    /// Consensus vote required
    Consensus {
        threshold: f64,
        quorum: usize,
        votes_for: usize,
        votes_against: usize,
    },
    /// Human approval required
    Human {
        approver: Option<String>,
        approved: Option<bool>,
        timestamp: Option<u64>,
    },
    /// Automated safety check
    SafetyCheck {
        name: String,
        predicate: String,
        passed: Option<bool>,
    },
}

/// Budget for modifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModificationBudget {
    /// Maximum changes per epoch
    pub max_changes: usize,
    /// Maximum total impact score
    pub max_impact: f64,
    /// Changes made so far
    pub changes_made: usize,
    /// Impact used so far
    pub impact_used: f64,
}

impl Default for ModificationBudget {
    fn default() -> Self {
        ModificationBudget {
            max_changes: 10,
            max_impact: 100.0,
            changes_made: 0,
            impact_used: 0.0,
        }
    }
}

impl ModificationBudget {
    pub fn can_afford(&self, impact: f64) -> bool {
        self.changes_made < self.max_changes && self.impact_used + impact <= self.max_impact
    }

    pub fn spend(&mut self, impact: f64) -> bool {
        if self.can_afford(impact) {
            self.changes_made += 1;
            self.impact_used += impact;
            true
        } else {
            false
        }
    }

    pub fn reset(&mut self) {
        self.changes_made = 0;
        self.impact_used = 0.0;
    }
}
