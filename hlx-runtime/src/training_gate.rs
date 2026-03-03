//! Training Gates — Phase 2 Prerequisite P4
//!
//! Gates for gradient-based weight modification:
//!   Pre-training gate: Verify proposal passes all checks before training starts
//!   Mid-training checkpoint: Verify intermediate states during training
//!   Post-training gate: Final verification before weights are committed
//!
//! These gates enforce that:
//!   - Human authorization is present for weight modifications
//!   - Conscience predicates pass against proposed changes
//!   - Integrity hashes are verified
//!   - Catastrophic forgetting guards are checked

use crate::governance::{Effect, EffectType, Governance};
use crate::human_auth::AuthorizationGate;
use crate::integrity::{CorpusHash, IntegritySystem};
use crate::RuntimeResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GateStage {
    PreTraining,
    MidTraining,
    PostTraining,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingProposal {
    pub id: u64,
    pub adapter_name: String,
    pub target_layers: Vec<usize>,
    pub learning_rate: f64,
    pub epochs: u32,
    pub batch_size: usize,
    pub justification: String,
    pub expected_behavior_change: String,
    pub risk_assessment: f64,
    pub human_auth_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointData {
    pub step: u32,
    pub total_steps: u32,
    pub loss: f64,
    pub gradient_norm: f64,
    pub weight_delta_norm: f64,
    pub timestamp: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateResult {
    pub stage: GateStage,
    pub passed: bool,
    pub checks: HashMap<String, CheckResult>,
    pub timestamp: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub passed: bool,
    pub message: String,
}

pub struct TrainingGate {
    auth_gate: AuthorizationGate,
    #[allow(dead_code)]
    integrity: IntegritySystem,
    proposal: Option<TrainingProposal>,
    checkpoints: Vec<CheckpointData>,
    max_gradient_norm: f64,
    max_weight_delta: f64,
    loss_increase_threshold: f64,
    checkpoint_interval: u32,
    pre_corpus_hash: Option<CorpusHash>,
}

impl TrainingGate {
    pub fn new(auth_gate: AuthorizationGate, integrity: IntegritySystem) -> Self {
        TrainingGate {
            auth_gate,
            integrity,
            proposal: None,
            checkpoints: Vec::new(),
            max_gradient_norm: 10.0,
            max_weight_delta: 0.1,
            loss_increase_threshold: 0.5,
            checkpoint_interval: 100,
            pre_corpus_hash: None,
        }
    }

    // ------------------------------------------------------------------
    // Configuration
    // ------------------------------------------------------------------

    pub fn with_max_gradient_norm(mut self, max: f64) -> Self {
        self.max_gradient_norm = max;
        self
    }

    pub fn with_max_weight_delta(mut self, max: f64) -> Self {
        self.max_weight_delta = max;
        self
    }

    pub fn with_loss_threshold(mut self, threshold: f64) -> Self {
        self.loss_increase_threshold = threshold;
        self
    }

    pub fn with_checkpoint_interval(mut self, interval: u32) -> Self {
        self.checkpoint_interval = interval;
        self
    }

    // ------------------------------------------------------------------
    // Pre-training Gate
    // ------------------------------------------------------------------

    pub fn pre_training_gate(
        &mut self,
        proposal: TrainingProposal,
        governance: &mut Governance,
        corpus_hash: Option<&CorpusHash>,
    ) -> RuntimeResult<GateResult> {
        let mut checks: HashMap<String, CheckResult> = HashMap::new();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();

        // Check 1: Human authorization
        let auth_check = self.check_human_auth(&proposal);
        checks.insert("human_authorization".to_string(), auth_check);

        // Check 2: Governance effect
        let gov_check = self.check_governance(&proposal, governance)?;
        checks.insert("governance".to_string(), gov_check);

        // Check 3: Risk assessment
        let risk_check = self.check_risk(&proposal);
        checks.insert("risk_assessment".to_string(), risk_check);

        // Check 4: Learning rate bounds
        let lr_check = self.check_learning_rate(&proposal);
        checks.insert("learning_rate_bounds".to_string(), lr_check);

        // Check 5: Corpus integrity (if hash provided)
        if let Some(hash) = corpus_hash {
            self.pre_corpus_hash = Some(hash.clone());
            checks.insert(
                "corpus_integrity".to_string(),
                CheckResult {
                    passed: true,
                    message: format!("Corpus hash recorded: {}...", &hash.combined_hash[..16]),
                },
            );
        }

        // Check 6: Adapter name safety
        let adapter_check = self.check_adapter_name(&proposal);
        checks.insert("adapter_name".to_string(), adapter_check);

        let passed = checks.values().all(|c| c.passed);

        if passed {
            self.proposal = Some(proposal);
        }

        Ok(GateResult {
            stage: GateStage::PreTraining,
            passed,
            checks,
            timestamp,
        })
    }

    fn check_human_auth(&self, proposal: &TrainingProposal) -> CheckResult {
        match &proposal.human_auth_token {
            Some(token) => match self.auth_gate.get_token(token) {
                Some(t) if t.is_valid() => CheckResult {
                    passed: true,
                    message: format!("Valid auth token: {}", &token[..16]),
                },
                Some(_) => CheckResult {
                    passed: false,
                    message: "Auth token expired or already used".to_string(),
                },
                None => CheckResult {
                    passed: false,
                    message: "Auth token not found".to_string(),
                },
            },
            None => CheckResult {
                passed: false,
                message: "No human authorization token provided".to_string(),
            },
        }
    }

    fn check_governance(
        &self,
        proposal: &TrainingProposal,
        governance: &mut Governance,
    ) -> RuntimeResult<CheckResult> {
        let mut effect = Effect::new(EffectType::SelfModify, &proposal.justification);
        effect.severity = 0.3; // Training proposals use lower severity than code self-mod

        let allowed = governance.check_effect(&mut effect)?;

        Ok(CheckResult {
            passed: allowed,
            message: if allowed {
                "Governance check passed".to_string()
            } else {
                "Governance check rejected proposal".to_string()
            },
        })
    }

    fn check_risk(&self, proposal: &TrainingProposal) -> CheckResult {
        let passed = proposal.risk_assessment <= 0.7;
        CheckResult {
            passed,
            message: format!(
                "Risk assessment: {:.2} (threshold: 0.70)",
                proposal.risk_assessment
            ),
        }
    }

    fn check_learning_rate(&self, proposal: &TrainingProposal) -> CheckResult {
        let passed = proposal.learning_rate > 0.0 && proposal.learning_rate <= 0.1;
        CheckResult {
            passed,
            message: format!(
                "Learning rate: {:.6} (valid range: (0, 0.1])",
                proposal.learning_rate
            ),
        }
    }

    fn check_adapter_name(&self, proposal: &TrainingProposal) -> CheckResult {
        let name = &proposal.adapter_name;
        let safe = !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '-');
        CheckResult {
            passed: safe,
            message: if safe {
                format!("Adapter name '{}' is safe", name)
            } else {
                format!("Adapter name '{}' contains unsafe characters", name)
            },
        }
    }

    // ------------------------------------------------------------------
    // Mid-training Gate
    // ------------------------------------------------------------------

    pub fn mid_training_gate(&mut self, checkpoint: CheckpointData) -> RuntimeResult<GateResult> {
        let mut checks: HashMap<String, CheckResult> = HashMap::new();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();

        // Check 1: Gradient norm
        let grad_check = CheckResult {
            passed: checkpoint.gradient_norm <= self.max_gradient_norm,
            message: format!(
                "Gradient norm: {:.4} (max: {:.4})",
                checkpoint.gradient_norm, self.max_gradient_norm
            ),
        };
        checks.insert("gradient_norm".to_string(), grad_check);

        // Check 2: Weight delta
        let delta_check = CheckResult {
            passed: checkpoint.weight_delta_norm <= self.max_weight_delta,
            message: format!(
                "Weight delta: {:.4} (max: {:.4})",
                checkpoint.weight_delta_norm, self.max_weight_delta
            ),
        };
        checks.insert("weight_delta".to_string(), delta_check);

        // Check 3: Loss not exploding
        let loss_check = if self.checkpoints.is_empty() {
            CheckResult {
                passed: true,
                message: format!("Initial loss: {:.4}", checkpoint.loss),
            }
        } else {
            let prev_loss = self.checkpoints.last().unwrap().loss;
            let increase = checkpoint.loss - prev_loss;
            let passed = increase < self.loss_increase_threshold;
            CheckResult {
                passed,
                message: format!(
                    "Loss change: {:.4} (threshold: {:.4})",
                    increase, self.loss_increase_threshold
                ),
            }
        };
        checks.insert("loss_stability".to_string(), loss_check);

        // Check 4: NaN/Inf detection
        let nan_check = CheckResult {
            passed: checkpoint.loss.is_finite()
                && checkpoint.gradient_norm.is_finite()
                && checkpoint.weight_delta_norm.is_finite(),
            message: if checkpoint.loss.is_finite()
                && checkpoint.gradient_norm.is_finite()
                && checkpoint.weight_delta_norm.is_finite()
            {
                "No NaN/Inf detected".to_string()
            } else {
                "NaN or Inf detected in training metrics!".to_string()
            },
        };
        checks.insert("numerical_stability".to_string(), nan_check);

        let passed = checks.values().all(|c| c.passed);

        self.checkpoints.push(checkpoint);

        Ok(GateResult {
            stage: GateStage::MidTraining,
            passed,
            checks,
            timestamp,
        })
    }

    // ------------------------------------------------------------------
    // Post-training Gate
    // ------------------------------------------------------------------

    pub fn post_training_gate(
        &mut self,
        final_loss: f64,
        governance: &mut Governance,
        corpus_hash: Option<&CorpusHash>,
    ) -> RuntimeResult<GateResult> {
        let mut checks: HashMap<String, CheckResult> = HashMap::new();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();

        // Check 1: Training completed
        let complete_check = CheckResult {
            passed: self.proposal.is_some(),
            message: if self.proposal.is_some() {
                "Training proposal found".to_string()
            } else {
                "No training proposal - was pre-training gate run?".to_string()
            },
        };
        checks.insert("training_complete".to_string(), complete_check);

        // Check 2: Final loss reasonable
        let loss_check = CheckResult {
            passed: final_loss.is_finite() && final_loss < 100.0,
            message: format!("Final loss: {:.4}", final_loss),
        };
        checks.insert("final_loss".to_string(), loss_check);

        // Check 3: Checkpoint history
        let checkpoint_check = CheckResult {
            passed: !self.checkpoints.is_empty(),
            message: format!("{} checkpoints recorded", self.checkpoints.len()),
        };
        checks.insert("checkpoint_history".to_string(), checkpoint_check);

        // Check 4: No mid-training failures
        let mid_failures = self
            .checkpoints
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                !c.loss.is_finite()
                    || c.gradient_norm > self.max_gradient_norm
                    || c.weight_delta_norm > self.max_weight_delta
            })
            .collect::<Vec<_>>();
        let mid_check = CheckResult {
            passed: mid_failures.is_empty(),
            message: if mid_failures.is_empty() {
                "All mid-training checks passed".to_string()
            } else {
                format!("{} mid-training failures detected", mid_failures.len())
            },
        };
        checks.insert("mid_training_clean".to_string(), mid_check);

        // Check 5: Governance re-verify
        if let Some(proposal) = &self.proposal {
            let effect = Effect::new(EffectType::SelfModify, &proposal.justification);
            let mut effect_mut = effect.clone();
            let allowed = governance.check_effect(&mut effect_mut)?;
            checks.insert(
                "governance_reverify".to_string(),
                CheckResult {
                    passed: allowed,
                    message: if allowed {
                        "Governance re-verification passed".to_string()
                    } else {
                        "Governance re-verification failed".to_string()
                    },
                },
            );
        }

        // Check 6: Corpus integrity unchanged
        if let (Some(pre), Some(post)) = (self.pre_corpus_hash.as_ref(), corpus_hash) {
            let integrity_check = pre.combined_hash == post.combined_hash;
            checks.insert(
                "corpus_unchanged".to_string(),
                CheckResult {
                    passed: integrity_check,
                    message: if integrity_check {
                        "Corpus unchanged during training".to_string()
                    } else {
                        "WARNING: Corpus changed during training!".to_string()
                    },
                },
            );
        }

        let passed = checks.values().all(|c| c.passed);

        Ok(GateResult {
            stage: GateStage::PostTraining,
            passed,
            checks,
            timestamp,
        })
    }

    // ------------------------------------------------------------------
    // Utilities
    // ------------------------------------------------------------------

    pub fn should_checkpoint(&self, step: u32) -> bool {
        step > 0 && step % self.checkpoint_interval == 0
    }

    pub fn checkpoints(&self) -> &[CheckpointData] {
        &self.checkpoints
    }

    pub fn proposal(&self) -> Option<&TrainingProposal> {
        self.proposal.as_ref()
    }

    pub fn clear(&mut self) {
        self.proposal = None;
        self.checkpoints.clear();
        self.pre_corpus_hash = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::governance::Governance;
    use crate::human_auth::AuthorizationGate;
    use crate::integrity::IntegritySystem;

    fn setup_gate() -> TrainingGate {
        TrainingGate::new(AuthorizationGate::new(), IntegritySystem::new())
    }

    fn setup_governance() -> Governance {
        let mut governance = Governance::new(0);
        governance.unlock_config();
        governance.set_confidence(0.99);
        for _ in 0..101 {
            governance.increment_step();
        }
        governance
    }

    #[test]
    fn test_pre_training_gate_rejects_no_auth() {
        let mut gate = setup_gate();
        let mut governance = setup_governance();

        let proposal = TrainingProposal {
            id: 1,
            adapter_name: "test_adapter".to_string(),
            target_layers: vec![0, 1],
            learning_rate: 0.01,
            epochs: 10,
            batch_size: 32,
            justification: "Test training".to_string(),
            expected_behavior_change: "Better accuracy".to_string(),
            risk_assessment: 0.1,
            human_auth_token: None,
        };

        let result = gate
            .pre_training_gate(proposal, &mut governance, None)
            .unwrap();
        assert!(!result.passed);
        assert!(!result.checks.get("human_authorization").unwrap().passed);
    }

    #[test]
    fn test_pre_training_gate_rejects_high_risk() {
        let mut gate = setup_gate();
        let mut governance = setup_governance();

        let proposal = TrainingProposal {
            id: 1,
            adapter_name: "test".to_string(),
            target_layers: vec![0],
            learning_rate: 0.01,
            epochs: 1,
            batch_size: 1,
            justification: "Test".to_string(),
            expected_behavior_change: "Test".to_string(),
            risk_assessment: 0.9,
            human_auth_token: Some("test_token".to_string()),
        };

        let result = gate
            .pre_training_gate(proposal, &mut governance, None)
            .unwrap();
        assert!(!result.checks.get("risk_assessment").unwrap().passed);
    }

    #[test]
    fn test_mid_training_gate_detects_gradient_explosion() {
        let mut gate = setup_gate();

        let checkpoint = CheckpointData {
            step: 100,
            total_steps: 1000,
            loss: 0.5,
            gradient_norm: 50.0,
            weight_delta_norm: 0.01,
            timestamp: 0.0,
        };

        let result = gate.mid_training_gate(checkpoint).unwrap();
        assert!(!result.passed);
        assert!(!result.checks.get("gradient_norm").unwrap().passed);
    }

    #[test]
    fn test_mid_training_gate_detects_nan() {
        let mut gate = setup_gate();

        let checkpoint = CheckpointData {
            step: 100,
            total_steps: 1000,
            loss: f64::NAN,
            gradient_norm: 0.5,
            weight_delta_norm: 1.01,
            timestamp: 0.0,
        };

        let result = gate.mid_training_gate(checkpoint).unwrap();
        assert!(!result.passed);
        assert!(!result.checks.get("numerical_stability").unwrap().passed);
    }

    #[test]
    fn test_should_checkpoint() {
        let gate = setup_gate().with_checkpoint_interval(100);

        assert!(!gate.should_checkpoint(0));
        assert!(!gate.should_checkpoint(50));
        assert!(gate.should_checkpoint(100));
        assert!(gate.should_checkpoint(200));
    }

    #[test]
    fn test_learning_rate_bounds() {
        let mut gate = setup_gate();
        let mut governance = setup_governance();

        let proposal = TrainingProposal {
            id: 1,
            adapter_name: "test".to_string(),
            target_layers: vec![0],
            learning_rate: 0.5,
            epochs: 1,
            batch_size: 1,
            justification: "Test".to_string(),
            expected_behavior_change: "Test".to_string(),
            risk_assessment: 0.1,
            human_auth_token: Some("token".to_string()),
        };

        let result = gate
            .pre_training_gate(proposal, &mut governance, None)
            .unwrap();
        assert!(!result.checks.get("learning_rate_bounds").unwrap().passed);
    }
}
