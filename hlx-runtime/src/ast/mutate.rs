//! Mutation support for AST nodes
//!
//! Enables safe modification of the AST with:
//! - Parent tracking for context-aware changes
//! - Undo/redo via operation log
//! - Diff generation for RSI proposals

use super::{NodeId, Program};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A mutation operation on the AST
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Mutation {
    /// Insert a new node
    Insert {
        parent: NodeId,
        position: usize,
        node_type: String,
        serialized_node: String, // JSON-serialized node
    },
    /// Delete a node
    Delete {
        node: NodeId,
        serialized_backup: String, // For undo
    },
    /// Replace a node's content
    Replace {
        node: NodeId,
        old_serialized: String,
        new_serialized: String,
    },
    /// Move a node to a new location
    Move {
        node: NodeId,
        old_parent: NodeId,
        old_position: usize,
        new_parent: NodeId,
        new_position: usize,
    },
    /// Rename an identifier
    Rename {
        node: NodeId,
        old_name: String,
        new_name: String,
    },
    /// Change a literal value
    ChangeLiteral {
        node: NodeId,
        old_value: String,
        new_value: String,
    },
}

impl Mutation {
    /// Create the inverse mutation for undo
    pub fn inverse(&self) -> Self {
        match self.clone() {
            Mutation::Insert {
                serialized_node,
                ..
            } => {
                Mutation::Delete {
                    node: NodeId(0), // Would need to extract from serialized_node
                    serialized_backup: serialized_node,
                }
            }
            Mutation::Delete {
                serialized_backup,
                ..
            } => {
                Mutation::Insert {
                    parent: NodeId(0), // Would need to store parent
                    position: 0,
                    node_type: "unknown".to_string(),
                    serialized_node: serialized_backup,
                }
            }
            Mutation::Replace {
                node,
                old_serialized,
                new_serialized,
            } => Mutation::Replace {
                node,
                old_serialized: new_serialized,
                new_serialized: old_serialized,
            },
            Mutation::Move {
                node,
                old_parent,
                old_position,
                new_parent,
                new_position,
            } => Mutation::Move {
                node,
                old_parent: new_parent,
                old_position: new_position,
                new_parent: old_parent,
                new_position: old_position,
            },
            Mutation::Rename {
                node,
                old_name,
                new_name,
            } => Mutation::Rename {
                node,
                old_name: new_name,
                new_name: old_name,
            },
            Mutation::ChangeLiteral {
                node,
                old_value,
                new_value,
            } => Mutation::ChangeLiteral {
                node,
                old_value: new_value,
                new_value: old_value,
            },
        }
    }

    /// Human-readable description
    pub fn describe(&self) -> String {
        match self {
            Mutation::Insert { node_type, .. } => format!("Insert {}", node_type),
            Mutation::Delete { .. } => "Delete node".to_string(),
            Mutation::Replace { .. } => "Replace node".to_string(),
            Mutation::Move { .. } => "Move node".to_string(),
            Mutation::Rename {
                old_name, new_name, ..
            } => {
                format!("Rename {} to {}", old_name, new_name)
            }
            Mutation::ChangeLiteral {
                old_value,
                new_value,
                ..
            } => {
                format!("Change {} to {}", old_value, new_value)
            }
        }
    }
}

/// A batch of mutations with atomic application
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MutationBatch {
    pub mutations: Vec<Mutation>,
    pub description: String,
    pub author: Option<String>,
    pub timestamp: Option<u64>,
}

impl MutationBatch {
    pub fn new(description: impl Into<String>) -> Self {
        MutationBatch {
            mutations: Vec::new(),
            description: description.into(),
            author: None,
            timestamp: None,
        }
    }

    pub fn add(&mut self, mutation: Mutation) {
        self.mutations.push(mutation);
    }

    pub fn inverse(&self) -> MutationBatch {
        MutationBatch {
            mutations: self.mutations.iter().rev().map(|m| m.inverse()).collect(),
            description: format!("Undo: {}", self.description),
            author: self.author.clone(),
            timestamp: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.mutations.is_empty()
    }
}

/// Mutation context with parent tracking
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct MutationContext {
    /// Parent of each node
    parents: HashMap<NodeId, NodeId>,
    /// Position within parent's children
    positions: HashMap<NodeId, usize>,
}

#[allow(dead_code)]
impl MutationContext {
    pub fn new() -> Self {
        MutationContext {
            parents: HashMap::new(),
            positions: HashMap::new(),
        }
    }

    pub fn build_from_program(&mut self, prog: &Program) {
        self.parents.clear();
        self.positions.clear();
        self.index_program(prog);
    }

    fn index_program(&mut self, prog: &Program) {
        for (pos, item) in prog.items.iter().enumerate() {
            self.index_item(item, prog.id, pos);
        }
    }

    fn index_item(&mut self, item: &super::Item, parent: NodeId, pos: usize) {
        match item {
            super::Item::Function(f) => {
                self.parents.insert(f.id, parent);
                self.positions.insert(f.id, pos);
                for (i, stmt) in f.body.iter().enumerate() {
                    self.index_statement(stmt, f.id, i);
                }
            }
            super::Item::Agent(a) => {
                self.parents.insert(a.id, parent);
                self.positions.insert(a.id, pos);
                for (i, stmt) in a.body.iter().enumerate() {
                    self.index_statement(stmt, a.id, i);
                }
            }
            super::Item::Cluster(c) => {
                self.parents.insert(c.id, parent);
                self.positions.insert(c.id, pos);
            }
            super::Item::Module(m) => {
                self.parents.insert(m.id, parent);
                self.positions.insert(m.id, pos);
                for (i, item) in m.items.iter().enumerate() {
                    self.index_item(item, m.id, i);
                }
            }
            super::Item::Import(i) => {
                self.parents.insert(i.id, parent);
                self.positions.insert(i.id, pos);
            }
            super::Item::Export(e) => {
                self.parents.insert(e.id, parent);
                self.positions.insert(e.id, pos);
            }
        }
    }

    fn index_statement(&mut self, stmt: &super::Statement, parent: NodeId, pos: usize) {
        self.parents.insert(stmt.id, parent);
        self.positions.insert(stmt.id, pos);

        use super::StmtKind;
        match &stmt.kind {
            StmtKind::Block(stmts) => {
                for (i, s) in stmts.iter().enumerate() {
                    self.index_statement(s, stmt.id, i);
                }
            }
            StmtKind::If(if_stmt) => {
                for (i, s) in if_stmt.then_body.iter().enumerate() {
                    self.index_statement(s, stmt.id, i);
                }
                for (i, s) in if_stmt.else_body.iter().enumerate() {
                    self.index_statement(s, stmt.id, i + if_stmt.then_body.len());
                }
            }
            StmtKind::Loop(loop_stmt) => {
                for (i, s) in loop_stmt.body.iter().enumerate() {
                    self.index_statement(s, stmt.id, i);
                }
            }
            StmtKind::Switch(switch) => {
                for (i, s) in switch.default_body.iter().enumerate() {
                    self.index_statement(s, stmt.id, i);
                }
            }
            _ => {}
        }
    }

    pub fn get_parent(&self, node: NodeId) -> Option<NodeId> {
        self.parents.get(&node).copied()
    }

    pub fn get_position(&self, node: NodeId) -> Option<usize> {
        self.positions.get(&node).copied()
    }
}

impl Default for MutationContext {
    fn default() -> Self {
        Self::new()
    }
}

/// RSI diff between two AST states
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstDiffBatch {
    pub from_version: u64,
    pub to_version: u64,
    pub mutations: MutationBatch,
    pub impact_score: f64,
    pub risk_level: RiskLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl AstDiffBatch {
    pub fn new(mutations: MutationBatch) -> Self {
        let impact_score = Self::calculate_impact(&mutations);
        let risk_level = Self::assess_risk(impact_score);

        AstDiffBatch {
            from_version: 0,
            to_version: 1,
            mutations,
            impact_score,
            risk_level,
        }
    }

    fn calculate_impact(batch: &MutationBatch) -> f64 {
        let mut score = 0.0;
        for mutation in &batch.mutations {
            score += match mutation {
                Mutation::Insert { .. } => 1.0,
                Mutation::Delete { .. } => 2.0,
                Mutation::Replace { .. } => 1.5,
                Mutation::Move { .. } => 0.5,
                Mutation::Rename { .. } => 0.3,
                Mutation::ChangeLiteral { .. } => 0.2,
            };
        }
        score
    }

    fn assess_risk(impact: f64) -> RiskLevel {
        if impact < 1.0 {
            RiskLevel::Low
        } else if impact < 3.0 {
            RiskLevel::Medium
        } else if impact < 6.0 {
            RiskLevel::High
        } else {
            RiskLevel::Critical
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "AST Diff: {} mutations, impact={:.1}, risk={:?}",
            self.mutations.mutations.len(),
            self.impact_score,
            self.risk_level
        )
    }
}
