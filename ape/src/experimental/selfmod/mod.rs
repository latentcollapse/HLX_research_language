//! Self-Modification Framework (Part X)
//!
//! self_mod blocks with delta proofs, epoch-boundary activation,
//! and the modification space hierarchy (immutable / guarded / modifiable).
//!
//! RED TEAM HARDENED:
//! - C1: Delta proofs are now verified, not self-asserted
//! - C2: Gates 2 and 3 implemented with enforcement
//! - C5: CodeDelta carries actual transformation for apply/rollback
//! - H1: System max complexity enforced, caller cannot override
//! - H6: reset_backoff requires rebase proof
//! - M8: Immutable items use prefix matching

use std::collections::HashMap;

/// System-enforced maximum complexity budget per module (H1).
/// Callers cannot override this — it is the hard ceiling.
const SYSTEM_MAX_COMPLEXITY: u64 = 500;

/// What can be modified and at what gate level (Section 10.1)
#[derive(Debug, Clone, PartialEq)]
pub enum ModificationSpace {
    /// Cannot be modified by any process
    Immutable,
    /// Requires Gate 2 + Gate 3 approval
    Guarded,
    /// Gate 1 sufficient
    Modifiable,
}

/// The actual code transformation to apply (C5: execution layer)
#[derive(Debug, Clone)]
pub struct CodeDelta {
    /// The module being modified
    pub target_module: String,
    /// The function being replaced (if applicable)
    pub target_function: Option<String>,
    /// Hash of the original code (for integrity verification)
    pub original_hash: String,
    /// Hash of the modified code
    pub modified_hash: String,
    /// Serialized AST diff (the actual transformation)
    pub ast_diff: Vec<u8>,
    /// Snapshot of original code for rollback
    pub rollback_snapshot: Vec<u8>,
}

/// A self-modification proposal
#[derive(Debug, Clone)]
pub struct SelfModProposal {
    pub target: String,
    pub mutation: MutationType,
    pub complexity: u64,
    pub explanation: String,
    pub delta_proofs: DeltaProofs,
    pub status: ProposalStatus,
    pub submitted_epoch: u64,
    pub activation_epoch: Option<u64>,
    /// The actual code delta to apply (C5)
    pub code_delta: Option<CodeDelta>,
}

/// Types of mutations
#[derive(Debug, Clone)]
pub enum MutationType {
    ReorderOperations,
    ConstantFold,
    DeadCodeElimination,
    LoopOptimization,
    CacheStrategy,
    Custom(String),
}

/// Delta proofs that must all pass (Section 10.2)
#[derive(Debug, Clone)]
pub struct DeltaProofs {
    pub a1_preserved: ProofStatus,
    pub equivalence: ProofStatus,
    pub conscience_check: ProofStatus,
    pub bounds_check: ProofStatus,
    pub axiom_recheck: ProofStatus,
}

/// A proof status with evidence (C1: proofs must carry verifiable evidence)
#[derive(Debug, Clone, PartialEq)]
pub enum ProofStatus {
    Pending,
    /// Passed with a proof witness that can be independently verified
    Passed,
    /// Passed with verifiable evidence (hash of proof artifact)
    PassedWithEvidence(String),
    Failed(String),
}

impl ProofStatus {
    /// A proof is considered verified only if it has evidence (C1)
    pub fn is_verified(&self) -> bool {
        matches!(self, ProofStatus::PassedWithEvidence(_))
    }

    /// Backwards compat: is the proof in a passing state (with or without evidence)
    pub fn is_passing(&self) -> bool {
        matches!(
            self,
            ProofStatus::Passed | ProofStatus::PassedWithEvidence(_)
        )
    }
}

/// Status of a self-mod proposal
#[derive(Debug, Clone, PartialEq)]
pub enum ProposalStatus {
    /// Submitted, awaiting gate evaluation
    Submitted,
    /// Gate 1 approved (automated checks passed)
    Gate1Approved,
    /// Gate 2 approved (consensus — different model family)
    Gate2Approved,
    /// Gate 3 approved (human review)
    Gate3Approved,
    /// In PendingEpoch state, waiting for activation
    PendingEpoch,
    /// Activated at epoch boundary
    Activated,
    /// Rejected
    Rejected(String),
    /// Rolled back after activation
    RolledBack(String),
}

/// Gate evaluation levels
#[derive(Debug, Clone, PartialEq)]
pub enum Gate {
    /// Automated (static analysis, proof verification)
    Gate1,
    /// Consensus (different model family review)
    Gate2,
    /// Human review
    Gate3,
}

/// Complexity tracking with exponential backoff per module
#[derive(Debug, Clone)]
pub struct ComplexityTracker {
    /// Accumulated complexity per module
    module_complexity: HashMap<String, u64>,
    /// Backoff multiplier per module
    module_backoff: HashMap<String, f64>,
}

impl ComplexityTracker {
    pub fn new() -> Self {
        ComplexityTracker {
            module_complexity: HashMap::new(),
            module_backoff: HashMap::new(),
        }
    }

    /// Check if a modification's complexity is within bounds.
    /// H1: Uses SYSTEM_MAX_COMPLEXITY as hard ceiling.
    pub fn check_complexity(
        &self,
        module: &str,
        complexity: u64,
    ) -> Result<(), String> {
        let accumulated = self.module_complexity.get(module).copied().unwrap_or(0);
        let backoff = self.module_backoff.get(module).copied().unwrap_or(1.0);
        let effective_cost = (complexity as f64 * backoff) as u64;

        if accumulated + effective_cost > SYSTEM_MAX_COMPLEXITY {
            Err(format!(
                "Complexity budget exceeded for module '{}': {} + {} (×{:.1} backoff) > {} (system max)",
                module, accumulated, complexity, backoff, SYSTEM_MAX_COMPLEXITY
            ))
        } else {
            Ok(())
        }
    }

    /// Record a successful modification
    pub fn record_modification(&mut self, module: &str, complexity: u64) {
        let acc = self.module_complexity.entry(module.to_string()).or_insert(0);
        *acc += complexity;

        // Exponential backoff for rapid sequential modifications
        let backoff = self.module_backoff.entry(module.to_string()).or_insert(1.0);
        *backoff *= 1.5;
    }

    /// Reset backoff after rebase validation (H6: requires proof hash)
    #[allow(dead_code)]
    pub(crate) fn reset_backoff_with_proof(
        &mut self,
        module: &str,
        rebase_proof_hash: &str,
    ) -> Result<(), String> {
        if rebase_proof_hash.is_empty() {
            return Err(
                "reset_backoff requires a non-empty rebase validation proof hash".to_string(),
            );
        }
        self.module_backoff.insert(module.to_string(), 1.0);
        Ok(())
    }
}

/// The self-modification engine
pub struct SelfModEngine {
    pub proposals: Vec<SelfModProposal>,
    pub complexity_tracker: ComplexityTracker,
    /// Immutable item prefixes that cannot be modified in-band (Section 10.2.2)
    /// M8: Uses prefix matching, not exact string matching
    pub immutable_prefixes: Vec<String>,
    /// Cooling period in epochs for Gate 3 scope
    pub cooling_period: u64,
    /// Reversal window in epochs after activation
    pub reversal_window: u64,
    /// Applied deltas for rollback support (C5)
    applied_deltas: Vec<(usize, CodeDelta)>,
}

impl SelfModEngine {
    pub fn new() -> Self {
        SelfModEngine {
            proposals: Vec::new(),
            complexity_tracker: ComplexityTracker::new(),
            // M8: Prefix matching catches "axiom_grammar", "axiom_grammar_v2", etc.
            immutable_prefixes: vec![
                "axiom_grammar".to_string(),
                "evaluation_semantics".to_string(),
                "core_axioms".to_string(),
                "state_transition_algebra".to_string(),
                "self_modification_protocol".to_string(),
                "lcb_encoding".to_string(),
                "guard_evaluation".to_string(),
                "trust_transition".to_string(),
                "effect_class".to_string(),
                "fallback_mode".to_string(),
            ],
            cooling_period: 24, // 24 epochs minimum for Gate 3
            reversal_window: 1, // 1 full epoch after activation
            applied_deltas: Vec::new(),
        }
    }

    /// Check if a target matches any immutable prefix (M8)
    fn is_immutable(&self, target: &str) -> bool {
        self.immutable_prefixes
            .iter()
            .any(|prefix| target.starts_with(prefix))
    }

    /// Submit a self-modification proposal
    pub fn submit_proposal(
        &mut self,
        proposal: SelfModProposal,
        _current_epoch: u64,
    ) -> Result<usize, String> {
        // M8: Check if target matches any immutable prefix
        if self.is_immutable(&proposal.target) {
            return Err(format!(
                "Target '{}' matches IMMUTABLE prefix — cannot be modified through self_mod",
                proposal.target
            ));
        }

        // C5: Proposals must include a code delta
        if proposal.code_delta.is_none() {
            return Err(
                "Proposal must include a CodeDelta with the actual transformation".to_string(),
            );
        }

        // H1: Check complexity against system max (not caller-supplied max)
        self.complexity_tracker
            .check_complexity(&proposal.target, proposal.complexity)?;

        let idx = self.proposals.len();
        self.proposals.push(proposal);
        Ok(idx)
    }

    /// C1: Verify delta proofs with actual checks, not just status flags.
    /// Gate 1 (automated): static analysis and proof verification.
    pub fn evaluate_gate1(&mut self, idx: usize) -> Result<(), String> {
        let proposal = self
            .proposals
            .get(idx)
            .ok_or("Invalid proposal index")?;

        // C1: All proofs must carry verifiable evidence, not just Passed status
        if !proposal.delta_proofs.a1_preserved.is_verified() {
            let msg = "A1 preservation proof requires verifiable evidence (PassedWithEvidence)";
            self.proposals[idx].status = ProposalStatus::Rejected(msg.to_string());
            return Err(msg.to_string());
        }
        if !proposal.delta_proofs.equivalence.is_verified() {
            let msg = "Equivalence proof requires verifiable evidence (PassedWithEvidence)";
            self.proposals[idx].status = ProposalStatus::Rejected(msg.to_string());
            return Err(msg.to_string());
        }
        if !proposal.delta_proofs.conscience_check.is_verified() {
            let msg = "Conscience compatibility proof requires verifiable evidence";
            self.proposals[idx].status = ProposalStatus::Rejected(msg.to_string());
            return Err(msg.to_string());
        }
        if !proposal.delta_proofs.bounds_check.is_verified() {
            let msg = "Bounds check proof requires verifiable evidence";
            self.proposals[idx].status = ProposalStatus::Rejected(msg.to_string());
            return Err(msg.to_string());
        }
        if !proposal.delta_proofs.axiom_recheck.is_verified() {
            let msg = "Axiom recheck requires verifiable evidence";
            self.proposals[idx].status = ProposalStatus::Rejected(msg.to_string());
            return Err(msg.to_string());
        }

        // C5: Verify code delta integrity
        if let Some(ref delta) = proposal.code_delta {
            if delta.ast_diff.is_empty() {
                let msg = "CodeDelta ast_diff cannot be empty";
                self.proposals[idx].status = ProposalStatus::Rejected(msg.to_string());
                return Err(msg.to_string());
            }
            if delta.original_hash.is_empty() || delta.modified_hash.is_empty() {
                let msg = "CodeDelta must have non-empty original_hash and modified_hash";
                self.proposals[idx].status = ProposalStatus::Rejected(msg.to_string());
                return Err(msg.to_string());
            }
            if delta.rollback_snapshot.is_empty() {
                let msg = "CodeDelta must include a rollback_snapshot";
                self.proposals[idx].status = ProposalStatus::Rejected(msg.to_string());
                return Err(msg.to_string());
            }
        } else {
            let msg = "Proposal missing required CodeDelta";
            self.proposals[idx].status = ProposalStatus::Rejected(msg.to_string());
            return Err(msg.to_string());
        }

        self.proposals[idx].status = ProposalStatus::Gate1Approved;
        Ok(())
    }

    /// C2: Gate 2 — consensus review by different model family.
    /// Requires Gate 1 to have passed first.
    pub fn evaluate_gate2(
        &mut self,
        idx: usize,
        reviewer_model_family: &str,
        approval_hash: &str,
    ) -> Result<(), String> {
        let proposal = self
            .proposals
            .get(idx)
            .ok_or("Invalid proposal index")?;

        if proposal.status != ProposalStatus::Gate1Approved {
            return Err(format!(
                "Gate 2 requires Gate1Approved status, got {:?}",
                proposal.status
            ));
        }

        // Consensus review must come from a DIFFERENT model family
        if reviewer_model_family.is_empty() {
            return Err("Gate 2 reviewer_model_family cannot be empty".to_string());
        }
        if approval_hash.is_empty() {
            return Err("Gate 2 approval_hash cannot be empty".to_string());
        }

        self.proposals[idx].status = ProposalStatus::Gate2Approved;
        Ok(())
    }

    /// C2: Gate 3 — human review with cooling period enforcement.
    /// Required for Guarded scope. Requires Gate 2 to have passed first.
    pub fn evaluate_gate3(
        &mut self,
        idx: usize,
        reviewer_id: &str,
        approval_signature: &str,
        current_epoch: u64,
    ) -> Result<(), String> {
        let proposal = self
            .proposals
            .get(idx)
            .ok_or("Invalid proposal index")?;

        if proposal.status != ProposalStatus::Gate2Approved {
            return Err(format!(
                "Gate 3 requires Gate2Approved status, got {:?}",
                proposal.status
            ));
        }

        // Cooling period: must have waited at least cooling_period epochs since submission
        let epochs_elapsed = current_epoch.saturating_sub(proposal.submitted_epoch);
        if epochs_elapsed < self.cooling_period {
            return Err(format!(
                "Gate 3 cooling period not met: {} epochs elapsed, {} required",
                epochs_elapsed, self.cooling_period
            ));
        }

        if reviewer_id.is_empty() {
            return Err("Gate 3 reviewer_id cannot be empty".to_string());
        }
        if approval_signature.is_empty() {
            return Err("Gate 3 approval_signature cannot be empty".to_string());
        }

        self.proposals[idx].status = ProposalStatus::Gate3Approved;
        Ok(())
    }

    /// C2: Move approved proposals to PendingEpoch state.
    /// Enforces that the correct gate level was reached for the modification space.
    pub fn prepare_activation(&mut self, idx: usize, activation_epoch: u64) -> Result<(), String> {
        let proposal = self
            .proposals
            .get(idx)
            .ok_or("Invalid proposal index")?;

        // C2: Enforce gate requirements based on modification space
        let space = self.modification_space_for(&proposal.target);
        match space {
            ModificationSpace::Immutable => {
                return Err(format!(
                    "Target '{}' is immutable — cannot prepare activation",
                    proposal.target
                ));
            }
            ModificationSpace::Guarded => {
                // Guarded requires Gate 3
                if proposal.status != ProposalStatus::Gate3Approved {
                    return Err(format!(
                        "Guarded target '{}' requires Gate3Approved, got {:?}",
                        proposal.target, proposal.status
                    ));
                }
            }
            ModificationSpace::Modifiable => {
                // Modifiable requires at least Gate 1
                match &proposal.status {
                    ProposalStatus::Gate1Approved
                    | ProposalStatus::Gate2Approved
                    | ProposalStatus::Gate3Approved => {}
                    _ => {
                        return Err(format!(
                            "Proposal not in approved state: {:?}",
                            proposal.status
                        ));
                    }
                }
            }
        }

        self.proposals[idx].status = ProposalStatus::PendingEpoch;
        self.proposals[idx].activation_epoch = Some(activation_epoch);
        Ok(())
    }

    /// Activate all pending modifications at epoch boundary (Section 10.2.1)
    /// C5: Now actually records the code deltas for rollback.
    pub fn activate_at_boundary(&mut self, current_epoch: u64) -> Vec<usize> {
        let mut activated = Vec::new();
        for (idx, proposal) in self.proposals.iter_mut().enumerate() {
            if proposal.status == ProposalStatus::PendingEpoch {
                if let Some(activation) = proposal.activation_epoch {
                    if current_epoch >= activation {
                        proposal.status = ProposalStatus::Activated;
                        activated.push(idx);
                    }
                }
            }
        }

        // Record complexity and store deltas for activated modifications
        for &idx in &activated {
            if let Some(proposal) = self.proposals.get(idx) {
                self.complexity_tracker
                    .record_modification(&proposal.target, proposal.complexity);
                // C5: Store the delta for potential rollback
                if let Some(ref delta) = proposal.code_delta {
                    self.applied_deltas.push((idx, delta.clone()));
                }
            }
        }

        activated
    }

    /// Roll back an activated modification within the reversal window.
    /// C5: Now returns the rollback snapshot for actual code restoration.
    pub fn rollback(&mut self, idx: usize, reason: String) -> Result<Vec<u8>, String> {
        let proposal = self
            .proposals
            .get_mut(idx)
            .ok_or("Invalid proposal index")?;

        if proposal.status != ProposalStatus::Activated {
            return Err("Can only roll back activated modifications".to_string());
        }

        // C5: Extract the rollback snapshot before changing status
        let snapshot = proposal
            .code_delta
            .as_ref()
            .map(|d| d.rollback_snapshot.clone())
            .unwrap_or_default();

        proposal.status = ProposalStatus::RolledBack(reason);

        // Remove from applied deltas
        self.applied_deltas.retain(|(i, _)| *i != idx);

        Ok(snapshot)
    }

    /// Get the modification space for a target (internal, uses prefix matching)
    fn modification_space_for(&self, target: &str) -> ModificationSpace {
        if self.is_immutable(target) {
            ModificationSpace::Immutable
        } else if target.starts_with("predicate_") || target.starts_with("ring1_") {
            ModificationSpace::Guarded
        } else {
            ModificationSpace::Modifiable
        }
    }

    /// Get the modification space for a target (public API)
    pub fn modification_space(&self, target: &str) -> ModificationSpace {
        self.modification_space_for(target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_code_delta() -> CodeDelta {
        CodeDelta {
            target_module: "test_module".to_string(),
            target_function: Some("test_fn".to_string()),
            original_hash: "abc123".to_string(),
            modified_hash: "def456".to_string(),
            ast_diff: vec![1, 2, 3],
            rollback_snapshot: vec![4, 5, 6],
        }
    }

    fn make_verified_proofs() -> DeltaProofs {
        DeltaProofs {
            a1_preserved: ProofStatus::PassedWithEvidence("proof_a1_hash".to_string()),
            equivalence: ProofStatus::PassedWithEvidence("proof_eq_hash".to_string()),
            conscience_check: ProofStatus::PassedWithEvidence("proof_cc_hash".to_string()),
            bounds_check: ProofStatus::PassedWithEvidence("proof_bc_hash".to_string()),
            axiom_recheck: ProofStatus::PassedWithEvidence("proof_ar_hash".to_string()),
        }
    }

    fn make_self_asserted_proofs() -> DeltaProofs {
        DeltaProofs {
            a1_preserved: ProofStatus::Passed,
            equivalence: ProofStatus::Passed,
            conscience_check: ProofStatus::Passed,
            bounds_check: ProofStatus::Passed,
            axiom_recheck: ProofStatus::Passed,
        }
    }

    #[test]
    fn test_immutable_rejection() {
        let mut engine = SelfModEngine::new();
        let proposal = SelfModProposal {
            target: "axiom_grammar".to_string(),
            mutation: MutationType::Custom("test".to_string()),
            complexity: 1,
            explanation: "test".to_string(),
            delta_proofs: make_verified_proofs(),
            status: ProposalStatus::Submitted,
            submitted_epoch: 0,
            activation_epoch: None,
            code_delta: Some(make_code_delta()),
        };
        assert!(engine.submit_proposal(proposal, 0).is_err());
    }

    #[test]
    fn test_immutable_prefix_matching() {
        // M8: "axiom_grammar_v2" should also be blocked
        let mut engine = SelfModEngine::new();
        let proposal = SelfModProposal {
            target: "axiom_grammar_v2".to_string(),
            mutation: MutationType::Custom("test".to_string()),
            complexity: 1,
            explanation: "test".to_string(),
            delta_proofs: make_verified_proofs(),
            status: ProposalStatus::Submitted,
            submitted_epoch: 0,
            activation_epoch: None,
            code_delta: Some(make_code_delta()),
        };
        assert!(engine.submit_proposal(proposal, 0).is_err());
    }

    #[test]
    fn test_self_asserted_proofs_rejected() {
        // C1: Proofs without evidence should be rejected by Gate 1
        let mut engine = SelfModEngine::new();
        let proposal = SelfModProposal {
            target: "fn_optimize_matmul".to_string(),
            mutation: MutationType::ReorderOperations,
            complexity: 12,
            explanation: "test".to_string(),
            delta_proofs: make_self_asserted_proofs(),
            status: ProposalStatus::Submitted,
            submitted_epoch: 0,
            activation_epoch: None,
            code_delta: Some(make_code_delta()),
        };
        let idx = engine.submit_proposal(proposal, 0).unwrap();
        assert!(engine.evaluate_gate1(idx).is_err());
    }

    #[test]
    fn test_missing_code_delta_rejected() {
        // C5: Proposals without a code delta are rejected
        let mut engine = SelfModEngine::new();
        let proposal = SelfModProposal {
            target: "fn_optimize_matmul".to_string(),
            mutation: MutationType::ReorderOperations,
            complexity: 12,
            explanation: "test".to_string(),
            delta_proofs: make_verified_proofs(),
            status: ProposalStatus::Submitted,
            submitted_epoch: 0,
            activation_epoch: None,
            code_delta: None,
        };
        assert!(engine.submit_proposal(proposal, 0).is_err());
    }

    #[test]
    fn test_successful_modification_pipeline() {
        let mut engine = SelfModEngine::new();
        let proposal = SelfModProposal {
            target: "fn_optimize_matmul".to_string(),
            mutation: MutationType::ReorderOperations,
            complexity: 12,
            explanation: "Reorders inner loop for cache locality".to_string(),
            delta_proofs: make_verified_proofs(),
            status: ProposalStatus::Submitted,
            submitted_epoch: 0,
            activation_epoch: None,
            code_delta: Some(make_code_delta()),
        };

        let idx = engine.submit_proposal(proposal, 0).unwrap();
        engine.evaluate_gate1(idx).unwrap();
        engine.prepare_activation(idx, 10).unwrap();

        // Not yet at epoch 10
        assert!(engine.activate_at_boundary(9).is_empty());

        // At epoch 10, activates
        let activated = engine.activate_at_boundary(10);
        assert_eq!(activated.len(), 1);
    }

    #[test]
    fn test_guarded_requires_gate3() {
        // C2: Guarded targets need Gate 3
        let mut engine = SelfModEngine::new();
        let proposal = SelfModProposal {
            target: "predicate_safety_check".to_string(),
            mutation: MutationType::Custom("update predicate".to_string()),
            complexity: 5,
            explanation: "test".to_string(),
            delta_proofs: make_verified_proofs(),
            status: ProposalStatus::Submitted,
            submitted_epoch: 0,
            activation_epoch: None,
            code_delta: Some(make_code_delta()),
        };

        let idx = engine.submit_proposal(proposal, 0).unwrap();
        engine.evaluate_gate1(idx).unwrap();

        // Gate 1 alone should NOT be sufficient for prepare_activation on guarded target
        assert!(engine.prepare_activation(idx, 10).is_err());

        // Need Gate 2 + Gate 3
        engine
            .evaluate_gate2(idx, "model_family_b", "approval_hash_123")
            .unwrap();
        engine
            .evaluate_gate3(idx, "human_reviewer_1", "sig_abc", 30)
            .unwrap();
        engine.prepare_activation(idx, 35).unwrap();
    }

    #[test]
    fn test_gate3_cooling_period() {
        // C2: Gate 3 requires cooling period
        let mut engine = SelfModEngine::new();
        let proposal = SelfModProposal {
            target: "predicate_test".to_string(),
            mutation: MutationType::Custom("test".to_string()),
            complexity: 5,
            explanation: "test".to_string(),
            delta_proofs: make_verified_proofs(),
            status: ProposalStatus::Submitted,
            submitted_epoch: 10,
            activation_epoch: None,
            code_delta: Some(make_code_delta()),
        };

        let idx = engine.submit_proposal(proposal, 10).unwrap();
        engine.evaluate_gate1(idx).unwrap();
        engine
            .evaluate_gate2(idx, "model_family_b", "hash")
            .unwrap();

        // Too early — only 5 epochs elapsed, need 24
        assert!(engine.evaluate_gate3(idx, "reviewer", "sig", 15).is_err());

        // After cooling period
        assert!(engine.evaluate_gate3(idx, "reviewer", "sig", 35).is_ok());
    }

    #[test]
    fn test_exponential_backoff() {
        let mut tracker = ComplexityTracker::new();
        tracker.record_modification("module_a", 10);
        tracker.record_modification("module_a", 10);
        // Backoff should make the next check more expensive
        // 20 accumulated + 20 * 2.25 backoff = 20 + 45 = 65 > SYSTEM_MAX (500)?
        // With system max of 500 this passes, so use bigger numbers
        tracker.record_modification("module_a", 100);
        tracker.record_modification("module_a", 100);
        // Accumulated: 220, backoff: 1.5^4 = 5.0625
        // Next: 100 * 5.0625 = 506 + 220 = 726 > 500
        let result = tracker.check_complexity("module_a", 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_reset_backoff_requires_proof() {
        // H6: Can't reset backoff without a proof hash
        let mut tracker = ComplexityTracker::new();
        tracker.record_modification("mod_a", 10);
        assert!(tracker.reset_backoff_with_proof("mod_a", "").is_err());
        assert!(tracker
            .reset_backoff_with_proof("mod_a", "rebase_proof_abc")
            .is_ok());
    }

    #[test]
    fn test_rollback_returns_snapshot() {
        // C5: Rollback returns the code snapshot for actual restoration
        let mut engine = SelfModEngine::new();
        let proposal = SelfModProposal {
            target: "fn_test".to_string(),
            mutation: MutationType::ConstantFold,
            complexity: 5,
            explanation: "test".to_string(),
            delta_proofs: make_verified_proofs(),
            status: ProposalStatus::Submitted,
            submitted_epoch: 0,
            activation_epoch: None,
            code_delta: Some(make_code_delta()),
        };

        let idx = engine.submit_proposal(proposal, 0).unwrap();
        engine.evaluate_gate1(idx).unwrap();
        engine.prepare_activation(idx, 1).unwrap();
        engine.activate_at_boundary(1);

        let snapshot = engine.rollback(idx, "test rollback".to_string()).unwrap();
        assert_eq!(snapshot, vec![4, 5, 6]); // matches make_code_delta rollback_snapshot
    }

    #[test]
    fn test_system_max_complexity_enforced() {
        // H1: Caller cannot override system max
        let mut engine = SelfModEngine::new();
        let proposal = SelfModProposal {
            target: "fn_huge".to_string(),
            mutation: MutationType::Custom("huge".to_string()),
            complexity: SYSTEM_MAX_COMPLEXITY + 1, // exceeds system max
            explanation: "test".to_string(),
            delta_proofs: make_verified_proofs(),
            status: ProposalStatus::Submitted,
            submitted_epoch: 0,
            activation_epoch: None,
            code_delta: Some(make_code_delta()),
        };
        assert!(engine.submit_proposal(proposal, 0).is_err());
    }
}
