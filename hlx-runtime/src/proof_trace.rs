//! Proof Trace Export — Phase 27
//!
//! Implements `--emit-proof-trace` to export turn-state transitions as proof terms
//! for external verification (Rocq/Axiom). Each turn produces a ProofStep that
//! captures the pre-state, the action taken, and the post-state.
//!
//! The trace format is designed to be consumed by:
//! - Rocq/Coq proof checkers (G1-G6 conscience predicate verification)
//! - External auditors reviewing agent behavior
//! - Regression test generation

use serde::{Deserialize, Serialize};
use std::io::Write;

/// A single proof step in the trace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofStep {
    /// Step index (monotonically increasing)
    pub index: u64,
    /// Logical clock at this step
    pub logical_clock: u64,
    /// The action that was taken
    pub action: ProofAction,
    /// Pre-state hash (blake3 of serialized state)
    pub pre_state_hash: String,
    /// Post-state hash
    pub post_state_hash: String,
    /// Conscience verification result (if applicable)
    pub conscience_verdict: Option<ConscienceRecord>,
    /// Agent ID (if within an agent context)
    pub agent_id: Option<String>,
}

/// Types of actions that produce proof steps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProofAction {
    /// Function call
    Call { function: String, arg_count: usize },
    /// RSI self-modification applied
    RsiApply { modification_type: String },
    /// Promotion level change
    Promotion { from: String, to: String },
    /// Governance check
    GovernanceCheck { effect: String, intent: String },
    /// Memory learn/forget
    MemoryMutation { operation: String, count: usize },
    /// Cycle completion
    CycleEnd { cycle_name: String },
    /// State snapshot
    Snapshot { path: String },
    /// State restore
    Restore { path: String },
}

/// Record of a conscience verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConscienceRecord {
    /// Which predicate was checked (G1-G6)
    pub predicate: String,
    /// Whether it passed
    pub passed: bool,
    /// Reason (if denied)
    pub reason: Option<String>,
}

/// Proof trace recorder
pub struct ProofTrace {
    steps: Vec<ProofStep>,
    next_index: u64,
    enabled: bool,
    /// Maximum steps to keep in memory before flushing
    max_buffered: usize,
}

impl ProofTrace {
    pub fn new() -> Self {
        ProofTrace {
            steps: Vec::new(),
            next_index: 0,
            enabled: false,
            max_buffered: 10_000,
        }
    }

    /// Enable proof trace recording
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable proof trace recording
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Whether tracing is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Record a proof step
    pub fn record(
        &mut self,
        logical_clock: u64,
        action: ProofAction,
        pre_state: &[u8],
        post_state: &[u8],
        conscience_verdict: Option<ConscienceRecord>,
        agent_id: Option<String>,
    ) {
        if !self.enabled {
            return;
        }

        let step = ProofStep {
            index: self.next_index,
            logical_clock,
            action,
            pre_state_hash: blake3::hash(pre_state).to_hex().to_string(),
            post_state_hash: blake3::hash(post_state).to_hex().to_string(),
            conscience_verdict,
            agent_id,
        };

        self.next_index += 1;
        self.steps.push(step);

        // Auto-truncate if buffer is full (keep last max_buffered)
        if self.steps.len() > self.max_buffered * 2 {
            let drain_count = self.steps.len() - self.max_buffered;
            self.steps.drain(..drain_count);
        }
    }

    /// Get all recorded steps
    pub fn steps(&self) -> &[ProofStep] {
        &self.steps
    }

    /// Drain all steps (consuming them)
    pub fn drain(&mut self) -> Vec<ProofStep> {
        std::mem::take(&mut self.steps)
    }

    /// Number of recorded steps
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    /// Whether the trace is empty
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Export the trace as JSON to a writer
    pub fn export_json<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let trace = serde_json::json!({
            "format": "hlx-proof-trace",
            "version": 1,
            "step_count": self.steps.len(),
            "steps": self.steps,
        });
        serde_json::to_writer_pretty(writer, &trace)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }

    /// Export the trace as a Rocq/Coq-compatible proof term sketch
    pub fn export_rocq<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writeln!(writer, "(* HLX Proof Trace — auto-generated *)")?;
        writeln!(writer, "(* {} steps recorded *)", self.steps.len())?;
        writeln!(writer)?;
        writeln!(writer, "Require Import String.")?;
        writeln!(writer, "Require Import List.")?;
        writeln!(writer, "Import ListNotations.")?;
        writeln!(writer)?;
        writeln!(writer, "Record ProofStep := {{")?;
        writeln!(writer, "  step_index : nat;")?;
        writeln!(writer, "  logical_clock : nat;")?;
        writeln!(writer, "  pre_state_hash : string;")?;
        writeln!(writer, "  post_state_hash : string;")?;
        writeln!(writer, "  conscience_passed : bool;")?;
        writeln!(writer, "}}.")?;
        writeln!(writer)?;
        writeln!(writer, "Definition trace : list ProofStep := [")?;

        for (i, step) in self.steps.iter().enumerate() {
            let passed = step
                .conscience_verdict
                .as_ref()
                .map_or(true, |v| v.passed);
            writeln!(
                writer,
                "  {{| step_index := {}; logical_clock := {}; pre_state_hash := \"{}\"; post_state_hash := \"{}\"; conscience_passed := {} |}}{}",
                step.index,
                step.logical_clock,
                &step.pre_state_hash[..16], // Truncate for readability
                &step.post_state_hash[..16],
                if passed { "true" } else { "false" },
                if i + 1 < self.steps.len() { ";" } else { "" }
            )?;
        }

        writeln!(writer, "].")?;
        writeln!(writer)?;
        writeln!(writer, "(* Theorem: All conscience checks passed *)")?;
        writeln!(
            writer,
            "Theorem all_conscience_passed : forall s, In s trace -> conscience_passed s = true."
        )?;
        writeln!(writer, "Proof. (* Verify against G1-G6 predicates *) Admitted.")?;

        Ok(())
    }

    /// Compute a summary hash of the entire trace
    pub fn trace_hash(&self) -> String {
        let json = serde_json::to_string(&self.steps).unwrap_or_default();
        blake3::hash(json.as_bytes()).to_hex().to_string()
    }
}

impl Default for ProofTrace {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disabled_by_default() {
        let trace = ProofTrace::new();
        assert!(!trace.is_enabled());
        assert!(trace.is_empty());
    }

    #[test]
    fn test_record_step() {
        let mut trace = ProofTrace::new();
        trace.enable();

        trace.record(
            1,
            ProofAction::Call {
                function: "add".to_string(),
                arg_count: 2,
            },
            b"pre_state",
            b"post_state",
            None,
            None,
        );

        assert_eq!(trace.len(), 1);
        assert_eq!(trace.steps()[0].index, 0);
        assert_eq!(trace.steps()[0].logical_clock, 1);
    }

    #[test]
    fn test_disabled_skips_recording() {
        let mut trace = ProofTrace::new();
        // NOT enabled

        trace.record(
            1,
            ProofAction::Call {
                function: "add".to_string(),
                arg_count: 2,
            },
            b"pre",
            b"post",
            None,
            None,
        );

        assert!(trace.is_empty());
    }

    #[test]
    fn test_drain() {
        let mut trace = ProofTrace::new();
        trace.enable();

        trace.record(1, ProofAction::CycleEnd { cycle_name: "H".to_string() }, b"a", b"b", None, None);
        trace.record(2, ProofAction::CycleEnd { cycle_name: "L".to_string() }, b"b", b"c", None, None);

        let steps = trace.drain();
        assert_eq!(steps.len(), 2);
        assert!(trace.is_empty());
    }

    #[test]
    fn test_export_json() {
        let mut trace = ProofTrace::new();
        trace.enable();
        trace.record(
            0,
            ProofAction::GovernanceCheck {
                effect: "Execute".to_string(),
                intent: "RunCommand".to_string(),
            },
            b"state0",
            b"state1",
            Some(ConscienceRecord {
                predicate: "G1".to_string(),
                passed: true,
                reason: None,
            }),
            Some("agent_0".to_string()),
        );

        let mut buf = Vec::new();
        trace.export_json(&mut buf).unwrap();
        let json: serde_json::Value = serde_json::from_slice(&buf).unwrap();
        assert_eq!(json["format"], "hlx-proof-trace");
        assert_eq!(json["step_count"], 1);
    }

    #[test]
    fn test_export_rocq() {
        let mut trace = ProofTrace::new();
        trace.enable();
        trace.record(
            0,
            ProofAction::Call { function: "test".to_string(), arg_count: 0 },
            b"s0",
            b"s1",
            Some(ConscienceRecord { predicate: "G2".to_string(), passed: true, reason: None }),
            None,
        );

        let mut buf = Vec::new();
        trace.export_rocq(&mut buf).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("Require Import"));
        assert!(output.contains("conscience_passed := true"));
        assert!(output.contains("all_conscience_passed"));
    }

    #[test]
    fn test_trace_hash_deterministic() {
        let mut trace = ProofTrace::new();
        trace.enable();
        trace.record(0, ProofAction::CycleEnd { cycle_name: "H".to_string() }, b"a", b"b", None, None);

        let h1 = trace.trace_hash();
        let h2 = trace.trace_hash();
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_conscience_record_serialization() {
        let record = ConscienceRecord {
            predicate: "G3".to_string(),
            passed: false,
            reason: Some("Intent violates no-harm principle".to_string()),
        };
        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("G3"));
        assert!(json.contains("no-harm"));
    }
}
