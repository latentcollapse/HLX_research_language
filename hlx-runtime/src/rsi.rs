use crate::governance::{Effect, EffectType, Governance};
use crate::tensor::Tensor;
use crate::{RuntimeError, RuntimeResult};
use serde::{Deserialize, Serialize};
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

    pub fn with_expected_improvement(mut self, improvement: f64) -> Self {
        self.expected_improvement = improvement;
        self
    }

    pub fn assess_risk(&mut self) {
        self.risk_assessment = match &self.modification {
            ModificationType::ParameterUpdate { .. } => 0.2,
            ModificationType::BehaviorAdd { .. } => 0.3,
            ModificationType::BehaviorRemove { .. } => 0.6,
            ModificationType::CycleConfigChange { .. } => 0.4,
            ModificationType::ThresholdChange { .. } => 0.5,
            ModificationType::WeightMatrixUpdate { layer: _, delta } => {
                let max_delta = delta.iter().fold(0.0f64, |acc, &x| acc.max(x.abs()));
                (max_delta * 2.0).min(1.0)
            }
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

    pub fn has_voted(&self, agent_id: u64) -> bool {
        self.voters.contains(&agent_id)
    }

    pub fn voter_count(&self) -> usize {
        self.voters.len()
    }

    pub fn approval_ratio(&self) -> f64 {
        let total = self.votes_for + self.votes_against;
        if total == 0 {
            return 0.0;
        }
        self.votes_for as f64 / total as f64
    }

    pub fn is_approved(&self, threshold: f64, total_agents: usize) -> bool {
        let min_quorum = Self::min_quorum(total_agents);
        let total_votes = (self.votes_for + self.votes_against) as usize;
        total_votes >= min_quorum && self.approval_ratio() >= threshold
    }

    pub fn min_quorum(total_agents: usize) -> usize {
        let min_absolute = 3;
        let proportional = (total_agents as f64 * 0.2).ceil() as usize;
        min_absolute.max(proportional)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AgentMemorySnapshot {
    behaviors: Vec<(Vec<f64>, Vec<f64>)>,
    parameters: HashMap<String, f64>,
    weight_matrices: Vec<TensorSnapshot>,
    cycle_config: (u32, u32),
    hash: [u8; 32],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TensorSnapshot {
    data: Vec<f64>,
    shape: Vec<usize>,
}

impl From<&Tensor> for TensorSnapshot {
    fn from(t: &Tensor) -> Self {
        TensorSnapshot {
            data: t.data.clone(),
            shape: t.shape.clone(),
        }
    }
}

impl From<TensorSnapshot> for Tensor {
    fn from(s: TensorSnapshot) -> Self {
        Tensor {
            data: s.data,
            shape: s.shape,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AgentMemory {
    pub behaviors: Vec<(Vec<f64>, Vec<f64>)>,
    pub parameters: HashMap<String, f64>,
    pub weight_matrices: Vec<Tensor>,
    pub cycle_config: (u32, u32),
}

impl AgentMemory {
    pub fn new() -> Self {
        let mut params = HashMap::new();
        params.insert("learning_rate".to_string(), 0.01);
        params.insert("exploration".to_string(), 0.1);
        params.insert("confidence_threshold".to_string(), 0.95);

        AgentMemory {
            behaviors: Vec::new(),
            parameters: params,
            weight_matrices: Vec::new(),
            cycle_config: (3, 6),
        }
    }

    pub fn apply_modification(
        &mut self,
        modification: &ModificationType,
    ) -> RuntimeResult<Vec<u8>> {
        let rollback = self.serialize_snapshot()?;

        match modification {
            ModificationType::ParameterUpdate {
                name, new_value, ..
            } => {
                self.parameters.insert(name.clone(), *new_value);
            }
            ModificationType::BehaviorAdd { pattern, response } => {
                self.behaviors.push((pattern.clone(), response.clone()));
            }
            ModificationType::BehaviorRemove { index } => {
                if *index < self.behaviors.len() {
                    self.behaviors.remove(*index);
                }
            }
            ModificationType::CycleConfigChange { h_cycles, l_cycles } => {
                self.cycle_config = (*h_cycles, *l_cycles);
            }
            ModificationType::ThresholdChange {
                name, new_value, ..
            } => {
                self.parameters.insert(name.clone(), *new_value);
            }
            ModificationType::WeightMatrixUpdate { layer, delta } => {
                if *layer < self.weight_matrices.len() {
                    let weights = &mut self.weight_matrices[*layer];
                    for (i, &d) in delta.iter().enumerate() {
                        if i < weights.data.len() {
                            weights.data[i] += d;
                        }
                    }
                }
            }
        }

        Ok(rollback)
    }

    fn serialize_snapshot(&self) -> RuntimeResult<Vec<u8>> {
        let snapshot = AgentMemorySnapshot {
            behaviors: self.behaviors.clone(),
            parameters: self.parameters.clone(),
            weight_matrices: self
                .weight_matrices
                .iter()
                .map(TensorSnapshot::from)
                .collect(),
            cycle_config: self.cycle_config,
            hash: self.compute_hash(),
        };
        bincode::serialize(&snapshot)
            .map_err(|e| RuntimeError::new(format!("RSI serialization failed: {}", e), 0))
    }

    pub fn rollback(&mut self, data: &[u8]) -> Result<(), String> {
        let snapshot: AgentMemorySnapshot =
            bincode::deserialize(data).map_err(|e| format!("Deserialization failed: {}", e))?;

        self.behaviors = snapshot.behaviors;
        self.parameters = snapshot.parameters;
        self.weight_matrices = snapshot
            .weight_matrices
            .into_iter()
            .map(Tensor::from)
            .collect();
        self.cycle_config = snapshot.cycle_config;
        Ok(())
    }

    fn compute_hash(&self) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();

        for (pattern, response) in &self.behaviors {
            for p in pattern {
                hasher.update(&p.to_le_bytes());
            }
            for r in response {
                hasher.update(&r.to_le_bytes());
            }
        }

        for (key, value) in &self.parameters {
            hasher.update(key.as_bytes());
            hasher.update(&value.to_le_bytes());
        }

        for tensor in &self.weight_matrices {
            for d in &tensor.data {
                hasher.update(&d.to_le_bytes());
            }
            for s in &tensor.shape {
                hasher.update(&(*s as u64).to_le_bytes());
            }
        }

        hasher.update(&self.cycle_config.0.to_le_bytes());
        hasher.update(&self.cycle_config.1.to_le_bytes());

        *hasher.finalize().as_bytes()
    }
}

impl Default for AgentMemory {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct RSIPipeline {
    proposals: HashMap<u64, RSIProposal>,
    proposal_queue: Vec<u64>,
    next_proposal_id: u64,
    approval_threshold: f64,
    min_confidence: f64,
    max_risk: f64,
    modification_history: Vec<(u64, ModificationType, bool)>,
}

impl RSIPipeline {
    pub fn new() -> Self {
        RSIPipeline {
            proposals: HashMap::new(),
            proposal_queue: Vec::new(),
            next_proposal_id: 0,
            approval_threshold: 0.66,
            min_confidence: 0.8,
            max_risk: 0.7,
            modification_history: Vec::new(),
        }
    }

    pub fn create_proposal(
        &mut self,
        proposer: u64,
        modification: ModificationType,
        confidence: f64,
    ) -> RuntimeResult<u64> {
        if confidence < self.min_confidence {
            return Err(RuntimeError::new(
                format!(
                    "Confidence {} below minimum {}",
                    confidence, self.min_confidence
                ),
                0,
            ));
        }

        let id = self.next_proposal_id;
        self.next_proposal_id += 1;

        let mut proposal = RSIProposal::new(id, proposer, modification).with_confidence(confidence);
        proposal.assess_risk();

        if proposal.risk_assessment > self.max_risk {
            return Err(RuntimeError::new(
                format!(
                    "Risk {} exceeds maximum {}",
                    proposal.risk_assessment, self.max_risk
                ),
                0,
            ));
        }

        self.proposals.insert(id, proposal);
        self.proposal_queue.push(id);
        Ok(id)
    }

    pub fn get_proposal(&self, id: u64) -> Option<&RSIProposal> {
        self.proposals.get(&id)
    }

    pub fn get_proposal_mut(&mut self, id: u64) -> Option<&mut RSIProposal> {
        self.proposals.get_mut(&id)
    }

    pub fn vote(&mut self, proposal_id: u64, agent_id: u64, approve: bool) -> RuntimeResult<()> {
        let proposal = self
            .proposals
            .get_mut(&proposal_id)
            .ok_or_else(|| RuntimeError::new(format!("Proposal {} not found", proposal_id), 0))?;
        proposal.vote(agent_id, approve).map_err(|e| {
            RuntimeError::new(
                match e {
                    VoteError::AlreadyVoted => format!(
                        "Agent {} already voted on proposal {}",
                        agent_id, proposal_id
                    ),
                    VoteError::InvalidVoter => format!("Agent {} is not a valid voter", agent_id),
                },
                0,
            )
        })?;
        Ok(())
    }

    pub fn validate_proposal(
        &mut self,
        proposal_id: u64,
        governance: &mut Governance,
    ) -> RuntimeResult<bool> {
        let proposal = self
            .proposals
            .get(&proposal_id)
            .ok_or_else(|| RuntimeError::new(format!("Proposal {} not found", proposal_id), 0))?
            .clone();

        let effect = Effect::new(EffectType::SelfModify, &proposal.justification);
        let mut effect_mut = effect.clone();

        let allowed = governance.check_effect(&mut effect_mut)?;

        if let Some(p) = self.proposals.get_mut(&proposal_id) {
            p.status = if allowed {
                ProposalStatus::Validating
            } else {
                ProposalStatus::Rejected
            };
        }

        Ok(allowed)
    }

    pub fn finalize_proposal(
        &mut self,
        proposal_id: u64,
        total_agents: usize,
    ) -> RuntimeResult<bool> {
        let proposal = self
            .proposals
            .get(&proposal_id)
            .ok_or_else(|| RuntimeError::new(format!("Proposal {} not found", proposal_id), 0))?;

        if proposal.status != ProposalStatus::Validating {
            return Ok(false);
        }

        let approved = proposal.is_approved(self.approval_threshold, total_agents);

        if let Some(p) = self.proposals.get_mut(&proposal_id) {
            p.status = if approved {
                ProposalStatus::Approved
            } else {
                ProposalStatus::Rejected
            };
        }

        Ok(approved)
    }

    pub fn apply_proposal(
        &mut self,
        proposal_id: u64,
        memory: &mut AgentMemory,
    ) -> RuntimeResult<()> {
        let proposal = self
            .proposals
            .get(&proposal_id)
            .ok_or_else(|| RuntimeError::new(format!("Proposal {} not found", proposal_id), 0))?
            .clone();

        if proposal.status != ProposalStatus::Approved {
            return Err(RuntimeError::new("Proposal not approved", 0));
        }

        let rollback_data = memory.apply_modification(&proposal.modification)?;

        if let Some(p) = self.proposals.get_mut(&proposal_id) {
            p.rollback_data = Some(rollback_data);
            p.status = ProposalStatus::Applied;
        }

        self.modification_history.push((
            proposal.proposer_agent,
            proposal.modification.clone(),
            true,
        ));

        Ok(())
    }

    pub fn rollback_proposal(
        &mut self,
        proposal_id: u64,
        memory: &mut AgentMemory,
    ) -> RuntimeResult<()> {
        let proposal = self
            .proposals
            .get(&proposal_id)
            .ok_or_else(|| RuntimeError::new(format!("Proposal {} not found", proposal_id), 0))?
            .clone();

        if proposal.status != ProposalStatus::Applied {
            return Err(RuntimeError::new("Proposal not applied", 0));
        }

        if let Some(rollback_data) = &proposal.rollback_data {
            memory
                .rollback(rollback_data)
                .map_err(|e| RuntimeError::new(e, 0))?;
        }

        if let Some(p) = self.proposals.get_mut(&proposal_id) {
            p.status = ProposalStatus::RolledBack;
        }

        Ok(())
    }

    pub fn pending_count(&self) -> usize {
        self.proposals
            .values()
            .filter(|p| p.status == ProposalStatus::Pending)
            .count()
    }

    pub fn applied_count(&self) -> usize {
        self.proposals
            .values()
            .filter(|p| p.status == ProposalStatus::Applied)
            .count()
    }

    pub fn history(&self) -> &[(u64, ModificationType, bool)] {
        &self.modification_history
    }
}

impl Default for RSIPipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proposal_creation() {
        let mut pipeline = RSIPipeline::new();
        let mod_type = ModificationType::ParameterUpdate {
            name: "learning_rate".to_string(),
            old_value: 0.01,
            new_value: 0.02,
        };

        let id = pipeline.create_proposal(0, mod_type, 0.9).unwrap();
        let proposal = pipeline.get_proposal(id).unwrap();

        assert_eq!(proposal.status, ProposalStatus::Pending);
        assert_eq!(proposal.confidence, 0.9);
    }

    #[test]
    fn test_proposal_voting() {
        let mut pipeline = RSIPipeline::new();
        let mod_type = ModificationType::BehaviorAdd {
            pattern: vec![1.0, 2.0],
            response: vec![3.0],
        };

        let id = pipeline.create_proposal(0, mod_type, 0.9).unwrap();

        pipeline.vote(id, 1, true).unwrap();
        pipeline.vote(id, 2, true).unwrap();
        pipeline.vote(id, 3, false).unwrap();

        let proposal = pipeline.get_proposal(id).unwrap();
        assert_eq!(proposal.votes_for, 2);
        assert_eq!(proposal.votes_against, 1);
        assert_eq!(proposal.voter_count(), 3);
    }

    #[test]
    fn test_duplicate_vote_rejected() {
        let mut pipeline = RSIPipeline::new();
        let mod_type = ModificationType::ParameterUpdate {
            name: "learning_rate".to_string(),
            old_value: 0.01,
            new_value: 0.02,
        };

        let id = pipeline.create_proposal(0, mod_type, 0.9).unwrap();

        pipeline.vote(id, 1, true).unwrap();
        let result = pipeline.vote(id, 1, false);
        assert!(result.is_err());

        let proposal = pipeline.get_proposal(id).unwrap();
        assert_eq!(proposal.votes_for, 1);
        assert_eq!(proposal.votes_against, 0);
        assert!(proposal.has_voted(1));
        assert!(!proposal.has_voted(2));
    }

    #[test]
    fn test_multiple_agents_vote() {
        let mut pipeline = RSIPipeline::new();
        let mod_type = ModificationType::ParameterUpdate {
            name: "learning_rate".to_string(),
            old_value: 0.01,
            new_value: 0.02,
        };

        let id = pipeline.create_proposal(0, mod_type, 0.9).unwrap();

        for agent_id in 1..=5 {
            pipeline.vote(id, agent_id, true).unwrap();
        }

        let proposal = pipeline.get_proposal(id).unwrap();
        assert_eq!(proposal.votes_for, 5);
        assert_eq!(proposal.voter_count(), 5);
    }

    #[test]
    fn test_low_confidence_rejected() {
        let mut pipeline = RSIPipeline::new();
        let mod_type = ModificationType::ParameterUpdate {
            name: "learning_rate".to_string(),
            old_value: 0.01,
            new_value: 0.02,
        };

        let result = pipeline.create_proposal(0, mod_type, 0.5);
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_modification() {
        let mut memory = AgentMemory::new();
        let mod_type = ModificationType::ParameterUpdate {
            name: "learning_rate".to_string(),
            old_value: 0.01,
            new_value: 0.05,
        };

        memory.apply_modification(&mod_type).unwrap();
        assert_eq!(*memory.parameters.get("learning_rate").unwrap(), 0.05);
    }

    #[test]
    fn test_cycle_config_change() {
        let mut memory = AgentMemory::new();
        let mod_type = ModificationType::CycleConfigChange {
            h_cycles: 5,
            l_cycles: 10,
        };

        memory.apply_modification(&mod_type).unwrap();
        assert_eq!(memory.cycle_config, (5, 10));
    }

    #[test]
    fn test_rollback_restores_state() {
        let mut memory = AgentMemory::new();
        memory.parameters.insert("test_param".to_string(), 1.0);
        memory.behaviors.push((vec![1.0, 2.0], vec![3.0]));
        memory.cycle_config = (2, 4);

        let snapshot = memory.serialize_snapshot().unwrap();

        memory.parameters.insert("test_param".to_string(), 99.0);
        memory.behaviors.push((vec![5.0, 6.0], vec![7.0]));
        memory.cycle_config = (10, 20);

        assert_eq!(*memory.parameters.get("test_param").unwrap(), 99.0);
        assert_eq!(memory.behaviors.len(), 2);
        assert_eq!(memory.cycle_config, (10, 20));

        memory.rollback(&snapshot).unwrap();

        assert_eq!(*memory.parameters.get("test_param").unwrap(), 1.0);
        assert_eq!(memory.behaviors.len(), 1);
        assert_eq!(memory.behaviors[0], (vec![1.0, 2.0], vec![3.0]));
        assert_eq!(memory.cycle_config, (2, 4));
    }

    #[test]
    fn test_rollback_after_multiple_modifications() {
        let mut memory = AgentMemory::new();

        let snapshot1 = memory
            .apply_modification(&ModificationType::ParameterUpdate {
                name: "learning_rate".to_string(),
                old_value: 0.01,
                new_value: 0.02,
            })
            .unwrap();

        let snapshot2 = memory
            .apply_modification(&ModificationType::CycleConfigChange {
                h_cycles: 5,
                l_cycles: 10,
            })
            .unwrap();

        assert_eq!(*memory.parameters.get("learning_rate").unwrap(), 0.02);
        assert_eq!(memory.cycle_config, (5, 10));

        memory.rollback(&snapshot2).unwrap();
        assert_eq!(*memory.parameters.get("learning_rate").unwrap(), 0.02);
        assert_eq!(memory.cycle_config, (3, 6));

        memory.rollback(&snapshot1).unwrap();
        assert_eq!(*memory.parameters.get("learning_rate").unwrap(), 0.01);
        assert_eq!(memory.cycle_config, (3, 6));
    }

    #[test]
    fn test_hash_changes_with_state() {
        let mut memory = AgentMemory::new();
        let hash1 = memory.compute_hash();

        memory.parameters.insert("test".to_string(), 1.0);
        let hash2 = memory.compute_hash();

        assert_ne!(hash1, hash2);

        memory.parameters.insert("test".to_string(), 2.0);
        let hash3 = memory.compute_hash();

        assert_ne!(hash2, hash3);
    }

    #[test]
    fn test_serialize_deserialize_roundtrip() {
        let mut memory = AgentMemory::new();
        memory.parameters.insert("custom".to_string(), 0.5);
        memory.behaviors.push((vec![1.0, 2.0, 3.0], vec![4.0]));
        memory.cycle_config = (7, 14);

        let serialized = memory.serialize_snapshot().unwrap();
        let mut restored = AgentMemory::new();
        restored.rollback(&serialized).unwrap();

        assert_eq!(restored.parameters.get("custom"), Some(&0.5));
        assert_eq!(restored.behaviors.len(), 1);
        assert_eq!(restored.behaviors[0], (vec![1.0, 2.0, 3.0], vec![4.0]));
        assert_eq!(restored.cycle_config, (7, 14));
    }

    #[test]
    fn test_min_quorum_small_pool() {
        assert_eq!(RSIProposal::min_quorum(1), 3);
        assert_eq!(RSIProposal::min_quorum(5), 3);
        assert_eq!(RSIProposal::min_quorum(10), 3);
        assert_eq!(RSIProposal::min_quorum(14), 3);
        assert_eq!(RSIProposal::min_quorum(15), 3);
    }

    #[test]
    fn test_min_quorum_large_pool() {
        assert_eq!(RSIProposal::min_quorum(20), 4);
        assert_eq!(RSIProposal::min_quorum(50), 10);
        assert_eq!(RSIProposal::min_quorum(100), 20);
        assert_eq!(RSIProposal::min_quorum(1000), 200);
    }

    #[test]
    fn test_is_approved_with_quorum() {
        let mut proposal = RSIProposal::new(
            0,
            0,
            ModificationType::ParameterUpdate {
                name: "test".to_string(),
                old_value: 0.0,
                new_value: 1.0,
            },
        );

        proposal.vote(1, true).unwrap();
        proposal.vote(2, true).unwrap();
        proposal.vote(3, true).unwrap();

        assert!(proposal.is_approved(0.66, 5));
        assert!(proposal.is_approved(0.66, 10));
        assert!(proposal.is_approved(0.66, 14));

        assert!(!proposal.is_approved(0.66, 20));
        assert!(!proposal.is_approved(0.66, 100));
    }

    #[test]
    fn test_approval_with_scaled_quorum() {
        let mut proposal = RSIProposal::new(
            0,
            0,
            ModificationType::ParameterUpdate {
                name: "test".to_string(),
                old_value: 0.0,
                new_value: 1.0,
            },
        );

        for agent_id in 1..=20 {
            proposal.vote(agent_id, true).unwrap();
        }

        assert!(proposal.is_approved(0.66, 100));
        assert!(proposal.is_approved(0.5, 100));

        proposal.vote(21, false).unwrap();
        proposal.vote(22, false).unwrap();
        proposal.vote(23, false).unwrap();
        proposal.vote(24, false).unwrap();
        proposal.vote(25, false).unwrap();

        assert!(proposal.is_approved(0.66, 100));
    }
}
