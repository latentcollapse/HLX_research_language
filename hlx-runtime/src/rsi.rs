use serde::{Serialize, Deserialize};
use serde_json::json;
use crate::governance::{Effect, EffectType, Governance};
#[allow(unused_imports)]
use crate::homeostasis::{GateDecision, HomeostasisGate};
use crate::promotion::{PromotionGate, PromotionLevel};
use crate::tensor::Tensor;
use crate::training_gate::TrainingGate;
use crate::human_auth::{AuthorizationGate, RiskLevel};
use crate::forgetting_guard::ForgettingGuard;
use crate::integrity::IntegritySystem;
use crate::{RuntimeError, RuntimeResult};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProposalStatus {
    Pending,
    Validating,
    Approved,
    Rejected,
    Applied,
    RolledBack,
}

#[derive(Debug, Clone)]
pub enum ModificationType {
    ParameterUpdate {
        name: String,
        old_value: f64,
        new_value: f64,
    },
    BehaviorAdd {
        pattern: Vec<f64>,
        response: Vec<f64>,
    },
    BehaviorRemove {
        index: usize,
    },
    CycleConfigChange {
        h_cycles: u32,
        l_cycles: u32,
    },
    ThresholdChange {
        name: String,
        old_value: f64,
        new_value: f64,
    },
    WeightMatrixUpdate {
        layer: usize,
        delta: Vec<f64>,
    },
    RuleAdd {
        name: String,
        description: String,
        confidence: f64,
    },
    RuleRemove {
        name: String,
    },
    RuleUpdate {
        name: String,
        description: String,
        confidence: f64,
    },
    BeliefAdd {
        subject: String,
        predicate: String,
        object: String,
        source: String,
    },
    BeliefRetract {
        subject: String,
        predicate: String,
        object: String,
        reason: String,
    },
}

impl ModificationType {
    pub fn level(&self) -> u32 {
        match self {
            ModificationType::BeliefAdd { .. } | ModificationType::BeliefRetract { .. } => 0,
            ModificationType::ParameterUpdate { .. } | ModificationType::ThresholdChange { .. } => 1,
            _ => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoteError {
    AlreadyVoted,
    InvalidVoter,
}

#[derive(Debug, Clone)]
pub struct RSIProposal {
    pub id: u64,
    pub proposer_agent: u64,
    pub modification: ModificationType,
    pub justification: String,
    pub confidence: f64,
    pub expected_improvement: f64,
    pub risk_assessment: f64,
    pub status: ProposalStatus,
    pub votes_for: u32,
    pub votes_against: u32,
    pub rollback_data: Option<Vec<u8>>,
    pub voters: HashSet<u64>,
}

impl RSIProposal {
    pub fn new(id: u64, proposer: u64, modification: ModificationType) -> Self {
        RSIProposal {
            id,
            proposer_agent: proposer,
            modification,
            justification: String::new(),
            confidence: 0.0,
            expected_improvement: 0.0,
            risk_assessment: 1.0,
            status: ProposalStatus::Pending,
            votes_for: 0,
            votes_against: 0,
            rollback_data: None,
            voters: HashSet::new(),
        }
    }

    pub fn with_justification(mut self, justification: &str) -> Self {
        self.justification = justification.to_string();
        self
    }

    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence;
        self
    }

    pub fn assess_risk(&mut self) {
        self.risk_assessment = match &self.modification {
            ModificationType::BeliefAdd { .. } => 0.1,
            ModificationType::BeliefRetract { .. } => 0.2,
            ModificationType::ParameterUpdate { .. } => 0.2,
            ModificationType::ThresholdChange { .. } => 0.5,
            ModificationType::BehaviorAdd { .. } => 0.3,
            ModificationType::BehaviorRemove { .. } => 0.6,
            ModificationType::CycleConfigChange { .. } => 0.4,
            ModificationType::WeightMatrixUpdate { .. } => 0.8,
            ModificationType::RuleAdd { .. } => 0.7,
            ModificationType::RuleRemove { .. } => 0.9,
            ModificationType::RuleUpdate { .. } => 0.8,
        };
    }

    pub fn vote(&mut self, agent_id: u64, approve: bool) -> Result<(), VoteError> {
        if self.voters.contains(&agent_id) {
            return Err(VoteError::AlreadyVoted);
        }
        self.voters.insert(agent_id);
        if approve {
            self.votes_for += 1;
        } else {
            self.votes_against += 1;
        }
        Ok(())
    }

    pub fn voter_count(&self) -> usize {
        self.voters.len()
    }

    pub fn approval_ratio(&self) -> f64 {
        let total = self.votes_for + self.votes_against;
        if total == 0 { return 0.0; }
        self.votes_for as f64 / total as f64
    }

    pub fn is_approved(&self, threshold: f64, total_agents: usize) -> bool {
        let min_quorum = 3.max((total_agents as f64 * 0.2).ceil() as usize);
        self.voter_count() >= min_quorum && self.approval_ratio() >= threshold
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AgentMemory {
    pub behaviors: Vec<(Vec<f64>, Vec<f64>)>,
    pub parameters: HashMap<String, f64>,
    pub weight_matrices: Vec<Tensor>,
    pub cycle_config: (u32, u32),
    pub rules: Vec<(String, String, f64)>,
    pub beliefs: Vec<(String, String, String)>,
}

impl AgentMemory {
    pub fn new() -> Self {
        AgentMemory {
            behaviors: Vec::new(),
            parameters: HashMap::new(),
            weight_matrices: Vec::new(),
            cycle_config: (3, 6),
            rules: Vec::new(),
            beliefs: Vec::new(),
        }
    }
    pub fn apply_modification(&mut self, modification: &ModificationType) -> RuntimeResult<Vec<u8>> {
        let snapshot = bincode::serialize(self).map_err(|e| RuntimeError::new(format!("Rollback snapshot failed: {}", e), 0))?;
        match modification {
            ModificationType::BeliefAdd { subject, predicate, object, .. } => {
                self.beliefs.push((subject.clone(), predicate.clone(), object.clone()));
            }
            ModificationType::BeliefRetract { subject, predicate, object, .. } => {
                self.beliefs.retain(|(s, p, o)| s != subject || p != predicate || o != object);
            }
            ModificationType::ParameterUpdate { name, new_value, .. } => {
                self.parameters.insert(name.clone(), *new_value);
            }
            _ => {}
        }
        Ok(snapshot)
    }
    pub fn rollback(&mut self, data: &[u8]) -> Result<(), String> {
        let restored: Self = bincode::deserialize(data).map_err(|e| format!("Rollback failed: {}", e))?;
        *self = restored;
        Ok(())
    }
}

pub struct RSIPipeline {
    proposals: HashMap<u64, RSIProposal>,
    next_proposal_id: u64,
    homeostasis_gate: HomeostasisGate,
    promotion_gate: PromotionGate,
    #[allow(dead_code)]
    training_gate: TrainingGate,
    human_auth: AuthorizationGate,
    forgetting_guard: ForgettingGuard,
}

impl RSIPipeline {
    pub fn new() -> Self {
        let auth = AuthorizationGate::new();
        let integrity = IntegritySystem::new();
        RSIPipeline {
            proposals: HashMap::new(),
            next_proposal_id: 0,
            homeostasis_gate: HomeostasisGate::new(),
            promotion_gate: PromotionGate::new(),
            training_gate: TrainingGate::new(auth, integrity),
            human_auth: AuthorizationGate::new(),
            forgetting_guard: ForgettingGuard::new(),
        }
    }

    pub fn propose_belief_retraction(
        &mut self,
        proposer: u64,
        subject: &str,
        predicate: &str,
        object: &str,
        reason: &str,
    ) -> RuntimeResult<u64> {
        let modification = ModificationType::BeliefRetract {
            subject: subject.to_string(),
            predicate: predicate.to_string(),
            object: object.to_string(),
            reason: reason.to_string(),
        };
        self.create_proposal(proposer, modification, 1.0)
    }

    pub fn propose_belief_addition(
        &mut self,
        proposer: u64,
        subject: &str,
        predicate: &str,
        object: &str,
        confidence: f64,
        source: &str,
    ) -> RuntimeResult<u64> {
        let modification = ModificationType::BeliefAdd {
            subject: subject.to_string(),
            predicate: predicate.to_string(),
            object: object.to_string(),
            source: source.to_string(),
        };
        self.create_proposal(proposer, modification, confidence)
    }

    pub fn create_proposal(
        &mut self,
        proposer: u64,
        modification: ModificationType,
        confidence: f64,
    ) -> RuntimeResult<u64> {
        self.homeostasis_gate.evaluate(&modification);
        let status = self.homeostasis_gate.status(); eprintln!("{}", json!({"event": "rsi_homeostasis", "health": 1.0 - status.pressure, "forgetting_triggered": false}));

        if !self.promotion_gate.is_modification_allowed(&modification) {
            return Err(RuntimeError::new("Promotion level insufficient", 0));
        }

        let id = self.next_proposal_id;
        self.next_proposal_id += 1;
        let mut proposal = RSIProposal::new(id, proposer, modification).with_confidence(confidence);
        proposal.assess_risk();

        if proposal.modification.level() >= 2 {
            self.human_auth.request_access(
                "RSI Pipeline",
                match proposal.modification.level() {
                    2 => RiskLevel::Medium,
                    3 => RiskLevel::High,
                    _ => RiskLevel::Critical,
                }
            ).map_err(|e| RuntimeError::new(format!("Human auth required: {:?}", e), 0))?;
        }

        self.proposals.insert(id, proposal);
        Ok(id)
    }

    pub fn vote(&mut self, proposal_id: u64, agent_id: u64, approve: bool) -> RuntimeResult<()> {
        let proposal = self.proposals.get_mut(&proposal_id)
            .ok_or_else(|| RuntimeError::new("Proposal not found", 0))?;
        proposal.vote(agent_id, approve).map_err(|e| RuntimeError::new(format!("{:?}", e), 0))?;
        Ok(())
    }

    pub fn validate_proposal(&mut self, proposal_id: u64, governance: &mut Governance) -> RuntimeResult<bool> {
        let proposal = self.proposals.get(&proposal_id)
            .ok_or_else(|| RuntimeError::new("Proposal not found", 0))?;
        let mut effect = Effect::new(EffectType::SelfModify, &proposal.justification);
        let allowed = governance.check_effect(&mut effect)?;
        if let Some(p) = self.proposals.get_mut(&proposal_id) {
            p.status = if allowed { ProposalStatus::Validating } else { ProposalStatus::Rejected };
        }
        Ok(allowed)
    }

    pub fn apply_proposal(&mut self, proposal_id: u64, memory: &mut AgentMemory) -> RuntimeResult<()> {
        let proposal = self.proposals.get(&proposal_id)
            .ok_or_else(|| RuntimeError::new("Proposal not found", 0))?.clone();

        if let ModificationType::BeliefRetract { subject, .. } = &proposal.modification {
            let core_predicates = ["name", "creator", "governance"];
            if core_predicates.contains(&subject.as_str()) {
                return Err(RuntimeError::new("ForgettingGuard blocked: attempting to retract core belief", 0));
            }
        }

        let rollback_data = memory.apply_modification(&proposal.modification)?;

        if !self.forgetting_guard.check_retention().is_healthy() {
             memory.rollback(&rollback_data).map_err(|e| RuntimeError::new(e, 0))?;
             return Err(RuntimeError::new("ForgettingGuard check failed: Core beliefs compromised", 0));
        }

        if let Some(p) = self.proposals.get_mut(&proposal_id) {
            p.status = ProposalStatus::Applied;
            p.rollback_data = Some(rollback_data);
        }

        eprintln!("{}", json!({"event": "rsi_proposal", "action": match proposal.modification { ModificationType::BeliefAdd{..} => "BeliefAdd", ModificationType::BeliefRetract{..} => "BeliefRetract", _ => "Other" }, "approved": true}));
        self.promotion_gate.on_successful_interaction();
        Ok(())
    }

    pub fn rollback_proposal(&mut self, proposal_id: u64, memory: &mut AgentMemory) -> RuntimeResult<()> {
        let proposal = self.proposals.get_mut(&proposal_id)
            .ok_or_else(|| RuntimeError::new("Proposal not found", 0))?;
        if let Some(data) = &proposal.rollback_data {
            memory.rollback(data).map_err(|e| RuntimeError::new(e, 0))?;
            proposal.status = ProposalStatus::RolledBack;
        }
        Ok(())
    }

    pub fn get_proposal(&self, proposal_id: u64) -> Option<&RSIProposal> {
        self.proposals.get(&proposal_id)
    }

    pub fn check_promotion(&mut self) -> Option<PromotionLevel> {
        self.promotion_gate.evaluate_promotion()
    }
}
