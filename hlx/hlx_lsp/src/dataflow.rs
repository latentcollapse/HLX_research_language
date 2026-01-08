//! Dataflow Analysis for Variable Initialization Tracking
//!
//! Detects use of uninitialized variables by tracking initialization
//! state through control flow paths.

use std::collections::{HashMap, HashSet};
use crate::control_flow::{ControlFlowGraph, NodeId, NodeKind};

/// Variable initialization state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarState {
    /// Definitely uninitialized
    Uninitialized,
    /// Definitely initialized
    Initialized,
    /// Maybe initialized (depends on control flow path)
    MaybeInitialized,
}

/// Dataflow facts at a program point
#[derive(Debug, Clone)]
pub struct DataflowState {
    /// Variable name -> initialization state
    pub vars: HashMap<String, VarState>,
}

impl DataflowState {
    pub fn new() -> Self {
        Self {
            vars: HashMap::new(),
        }
    }

    /// Mark a variable as initialized
    pub fn initialize(&mut self, var: &str) {
        self.vars.insert(var.to_string(), VarState::Initialized);
    }

    /// Mark a variable as uninitialized
    pub fn declare(&mut self, var: &str) {
        self.vars.insert(var.to_string(), VarState::Uninitialized);
    }

    /// Get the state of a variable
    pub fn get_state(&self, var: &str) -> VarState {
        self.vars.get(var).copied().unwrap_or(VarState::Uninitialized)
    }

    /// Merge two dataflow states (used at join points)
    pub fn merge(&self, other: &DataflowState) -> DataflowState {
        let mut result = DataflowState::new();

        // Collect all variables from both states
        let mut all_vars = HashSet::new();
        all_vars.extend(self.vars.keys().cloned());
        all_vars.extend(other.vars.keys().cloned());

        for var in all_vars {
            let state1 = self.get_state(&var);
            let state2 = other.get_state(&var);

            let merged_state = match (state1, state2) {
                // Both initialized -> Initialized
                (VarState::Initialized, VarState::Initialized) => VarState::Initialized,
                // Both uninitialized -> Uninitialized
                (VarState::Uninitialized, VarState::Uninitialized) => VarState::Uninitialized,
                // One path initializes, one doesn't -> MaybeInitialized
                _ => VarState::MaybeInitialized,
            };

            result.vars.insert(var, merged_state);
        }

        result
    }
}

/// Dataflow analyzer
pub struct DataflowAnalyzer {
    /// State before each node
    in_states: HashMap<NodeId, DataflowState>,
    /// State after each node
    out_states: HashMap<NodeId, DataflowState>,
}

impl DataflowAnalyzer {
    pub fn new() -> Self {
        Self {
            in_states: HashMap::new(),
            out_states: HashMap::new(),
        }
    }

    /// Run forward dataflow analysis on a CFG
    pub fn analyze(&mut self, cfg: &ControlFlowGraph) {
        // Initialize entry state (all variables uninitialized)
        let mut entry_state = DataflowState::new();
        for var in cfg.get_declared_vars() {
            entry_state.declare(&var);
        }
        self.out_states.insert(cfg.entry, entry_state);

        // Worklist algorithm
        let mut worklist: Vec<NodeId> = vec![cfg.entry];
        let mut visited = HashSet::new();

        while let Some(node_id) = worklist.pop() {
            if visited.contains(&node_id) && node_id != cfg.entry {
                continue;
            }
            visited.insert(node_id);

            let node = match cfg.nodes.get(&node_id) {
                Some(n) => n,
                None => continue,
            };

            // Compute in_state by merging out_states of predecessors
            let in_state = if node.predecessors.is_empty() {
                // Entry node
                DataflowState::new()
            } else {
                let mut merged: Option<DataflowState> = None;
                for pred_id in &node.predecessors {
                    if let Some(pred_out) = self.out_states.get(pred_id) {
                        merged = Some(match merged {
                            None => pred_out.clone(),
                            Some(m) => m.merge(pred_out),
                        });
                    }
                }
                merged.unwrap_or_else(DataflowState::new)
            };

            self.in_states.insert(node_id, in_state.clone());

            // Apply transfer function
            let out_state = self.transfer(node, in_state);
            self.out_states.insert(node_id, out_state);

            // Add successors to worklist
            for succ_id in &node.successors {
                worklist.push(*succ_id);
            }
        }
    }

    /// Transfer function: compute out_state from in_state and node
    fn transfer(&self, node: &crate::control_flow::CfgNode, mut state: DataflowState) -> DataflowState {
        match &node.kind {
            NodeKind::VarDecl { name, .. } => {
                // Declaration with initialization: let x = value;
                // In HLX, `let` always has an initializer, so mark as initialized
                state.initialize(name);
            }
            NodeKind::VarAssign { name, .. } => {
                // Assignment: x = value;
                state.initialize(name);
            }
            NodeKind::VarUse { .. } => {
                // Variable use doesn't change state
            }
            NodeKind::Branch { .. } | NodeKind::Loop { .. } => {
                // Control flow doesn't change variable state
            }
            NodeKind::Return { .. } | NodeKind::Other { .. } => {
                // Other statements don't change variable state
            }
            NodeKind::Entry | NodeKind::Exit => {
                // Entry/exit don't change state
            }
        }

        state
    }

    /// Get the state before a node
    pub fn get_in_state(&self, node_id: NodeId) -> Option<&DataflowState> {
        self.in_states.get(&node_id)
    }

    /// Get the state after a node
    pub fn get_out_state(&self, node_id: NodeId) -> Option<&DataflowState> {
        self.out_states.get(&node_id)
    }

    /// Check if a variable use is potentially uninitialized
    pub fn check_use(&self, cfg: &ControlFlowGraph, var_name: &str) -> Vec<UninitializedUse> {
        let mut problems = Vec::new();

        for (node_id, node) in &cfg.nodes {
            if let NodeKind::VarUse { name, line } = &node.kind {
                if name == var_name {
                    // Check the state before this use
                    if let Some(in_state) = self.get_in_state(*node_id) {
                        let state = in_state.get_state(var_name);
                        match state {
                            VarState::Uninitialized => {
                                problems.push(UninitializedUse {
                                    var_name: var_name.to_string(),
                                    line: *line,
                                    certainty: UseCertainty::Definitely,
                                });
                            }
                            VarState::MaybeInitialized => {
                                problems.push(UninitializedUse {
                                    var_name: var_name.to_string(),
                                    line: *line,
                                    certainty: UseCertainty::Maybe,
                                });
                            }
                            VarState::Initialized => {
                                // OK
                            }
                        }
                    }
                }
            }
        }

        problems
    }

    /// Check all variables for uninitialized uses
    pub fn check_all(&self, cfg: &ControlFlowGraph) -> Vec<UninitializedUse> {
        let mut all_problems = Vec::new();

        for var in cfg.get_declared_vars() {
            all_problems.extend(self.check_use(cfg, &var));
        }

        all_problems
    }
}

impl Default for DataflowAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Report of an uninitialized variable use
#[derive(Debug, Clone)]
pub struct UninitializedUse {
    pub var_name: String,
    pub line: usize,
    pub certainty: UseCertainty,
}

/// How certain we are about uninitialized use
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UseCertainty {
    /// Definitely uninitialized on all paths
    Definitely,
    /// Maybe uninitialized on some paths
    Maybe,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control_flow::ControlFlowGraph;

    #[test]
    fn test_dataflow_merge() {
        let mut state1 = DataflowState::new();
        state1.initialize("x");
        state1.declare("y");

        let mut state2 = DataflowState::new();
        state2.declare("x");
        state2.initialize("y");

        let merged = state1.merge(&state2);

        // x: initialized in state1, uninitialized in state2 -> maybe
        assert_eq!(merged.get_state("x"), VarState::MaybeInitialized);
        // y: uninitialized in state1, initialized in state2 -> maybe
        assert_eq!(merged.get_state("y"), VarState::MaybeInitialized);
    }

    #[test]
    fn test_simple_uninitialized_use() {
        let mut cfg = ControlFlowGraph::new();

        // let x;  (declare without init - hypothetical)
        let n1 = cfg.new_node(NodeKind::VarDecl {
            name: "x".to_string(),
            line: 1,
        });
        // use x;
        let n2 = cfg.new_node(NodeKind::VarUse {
            name: "x".to_string(),
            line: 2,
        });

        cfg.add_edge(cfg.entry, n1);
        cfg.add_edge(n1, n2);
        cfg.add_edge(n2, cfg.exit);

        let mut analyzer = DataflowAnalyzer::new();
        analyzer.analyze(&cfg);

        // In HLX, `let` always initializes, so this should NOT be flagged
        // But let's test the analyzer mechanics
        let problems = analyzer.check_use(&cfg, "x");
        // Since VarDecl marks as initialized, should be OK
        assert_eq!(problems.len(), 0);
    }

    #[test]
    fn test_conditional_initialization() {
        let mut cfg = ControlFlowGraph::new();

        // let x;
        let decl = cfg.new_node(NodeKind::VarDecl {
            name: "x".to_string(),
            line: 1,
        });
        // if branch
        let branch = cfg.new_node(NodeKind::Branch { line: 2 });
        // then: x = 1;
        let then_init = cfg.new_node(NodeKind::VarAssign {
            name: "x".to_string(),
            line: 3,
        });
        // else: (no init)
        let else_skip = cfg.new_node(NodeKind::Other { line: 4 });
        // use x;
        let use_x = cfg.new_node(NodeKind::VarUse {
            name: "x".to_string(),
            line: 5,
        });

        cfg.add_edge(cfg.entry, decl);
        cfg.add_edge(decl, branch);
        cfg.add_edge(branch, then_init); // then branch
        cfg.add_edge(branch, else_skip);  // else branch
        cfg.add_edge(then_init, use_x);
        cfg.add_edge(else_skip, use_x);
        cfg.add_edge(use_x, cfg.exit);

        let mut analyzer = DataflowAnalyzer::new();
        analyzer.analyze(&cfg);

        // x is maybe initialized at use_x (initialized on then path, not on else)
        let problems = analyzer.check_use(&cfg, "x");
        // Should have ONE problem: maybe uninitialized
        assert_eq!(problems.len(), 1);
        assert_eq!(problems[0].certainty, UseCertainty::Maybe);
    }
}
