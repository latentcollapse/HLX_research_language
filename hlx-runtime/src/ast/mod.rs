//! HLX Abstract Syntax Tree
//!
//! This module provides a proper introspectable AST that RSI can safely modify.
//! Key design principles:
//! - Every node has a unique NodeId for identity and tracking
//! - Parent references enable upward navigation for context-aware modifications
//! - Serialization preserves structure for diffing and rollback
//! - Deterministic rendering enables RSI to show what changed

mod agent;
mod expr;
mod mutate;
mod render;
mod rsi;
mod stmt;
mod visit;

pub use agent::*;
pub use expr::*;
pub use mutate::*;
pub use render::*;
pub use rsi::*;
pub use stmt::*;
pub use visit::*;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// Unique identifier for an AST node
/// Enables tracking nodes across modifications and provides identity for diffing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub u64);

impl NodeId {
    /// Generate a new unique NodeId
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        NodeId(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

/// Source location for error reporting and RSI provenance tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSpan {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

impl SourceSpan {
    pub fn new(start_line: u32, start_col: u32, end_line: u32, end_col: u32) -> Self {
        SourceSpan {
            start_line,
            start_col,
            end_line,
            end_col,
        }
    }

    pub fn unknown() -> Self {
        SourceSpan {
            start_line: 0,
            start_col: 0,
            end_line: 0,
            end_col: 0,
        }
    }
}

/// The complete AST for an HLX program
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Program {
    pub id: NodeId,
    pub span: SourceSpan,
    pub name: String,
    pub items: Vec<Item>,
    /// Node index for O(1) lookup by NodeId
    #[serde(skip)]
    node_index: HashMap<NodeId, NodeRef>,
}

/// Top-level items in an HLX program
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Item {
    Function(Function),
    Agent(AgentDef),
    Cluster(ClusterDef),
    Module(ModuleDef),
    Import(Import),
    Export(Export),
}

/// Reference to any AST node, used for parent tracking and navigation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeRef {
    Program(NodeId),
    Function(NodeId),
    Agent(NodeId),
    Cluster(NodeId),
    Module(NodeId),
    Import(NodeId),
    Export(NodeId),
    Statement(NodeId),
    Expression(NodeId),
    Parameter(NodeId),
    Latent(NodeId),
    Cycle(NodeId),
    Govern(NodeId),
    Modify(NodeId),
    Case(NodeId),
}

impl Program {
    pub fn new(name: impl Into<String>) -> Self {
        Program {
            id: NodeId::new(),
            span: SourceSpan::unknown(),
            name: name.into(),
            items: Vec::new(),
            node_index: HashMap::new(),
        }
    }

    /// Rebuild the node index after modifications
    pub fn rebuild_index(&mut self) {
        self.node_index.clear();
        self.node_index.insert(self.id, NodeRef::Program(self.id));

        // Collect all node refs first to avoid borrow conflicts
        let mut refs: Vec<NodeRef> = Vec::new();
        for item in &self.items {
            refs.extend(self.collect_item_refs(item));
        }

        // Now insert them
        for r in refs {
            match r {
                NodeRef::Program(id) => {
                    self.node_index.insert(id, r);
                }
                NodeRef::Function(id) => {
                    self.node_index.insert(id, r);
                }
                NodeRef::Agent(id) => {
                    self.node_index.insert(id, r);
                }
                NodeRef::Cluster(id) => {
                    self.node_index.insert(id, r);
                }
                NodeRef::Module(id) => {
                    self.node_index.insert(id, r);
                }
                NodeRef::Import(id) => {
                    self.node_index.insert(id, r);
                }
                NodeRef::Export(id) => {
                    self.node_index.insert(id, r);
                }
                NodeRef::Statement(id) => {
                    self.node_index.insert(id, r);
                }
                NodeRef::Expression(id) => {
                    self.node_index.insert(id, r);
                }
                NodeRef::Parameter(id) => {
                    self.node_index.insert(id, r);
                }
                NodeRef::Latent(id) => {
                    self.node_index.insert(id, r);
                }
                NodeRef::Cycle(id) => {
                    self.node_index.insert(id, r);
                }
                NodeRef::Govern(id) => {
                    self.node_index.insert(id, r);
                }
                NodeRef::Modify(id) => {
                    self.node_index.insert(id, r);
                }
                NodeRef::Case(id) => {
                    self.node_index.insert(id, r);
                }
            }
        }
    }

    fn collect_item_refs(&self, item: &Item) -> Vec<NodeRef> {
        let mut refs = Vec::new();
        match item {
            Item::Function(f) => {
                refs.push(NodeRef::Function(f.id));
                for stmt in &f.body {
                    refs.extend(self.collect_stmt_refs(stmt));
                }
            }
            Item::Agent(a) => {
                refs.push(NodeRef::Agent(a.id));
                for latent in &a.latents {
                    refs.push(NodeRef::Latent(latent.id));
                }
                for cycle in &a.cycles {
                    refs.push(NodeRef::Cycle(cycle.id));
                }
                for stmt in &a.body {
                    refs.extend(self.collect_stmt_refs(stmt));
                }
                if let Some(ref govern) = a.govern {
                    refs.push(NodeRef::Govern(govern.id));
                }
                if let Some(ref modify) = a.modify {
                    refs.push(NodeRef::Modify(modify.id));
                }
            }
            Item::Cluster(c) => {
                refs.push(NodeRef::Cluster(c.id));
            }
            Item::Module(m) => {
                refs.push(NodeRef::Module(m.id));
            }
            Item::Import(i) => {
                refs.push(NodeRef::Import(i.id));
            }
            Item::Export(e) => {
                refs.push(NodeRef::Export(e.id));
            }
        }
        refs
    }

    fn collect_stmt_refs(&self, stmt: &Statement) -> Vec<NodeRef> {
        let mut refs = vec![NodeRef::Statement(stmt.id)];
        match &stmt.kind {
            StmtKind::If(if_stmt) => {
                for s in &if_stmt.then_body {
                    refs.extend(self.collect_stmt_refs(s));
                }
                for s in &if_stmt.else_body {
                    refs.extend(self.collect_stmt_refs(s));
                }
            }
            StmtKind::Loop(loop_stmt) => {
                for s in &loop_stmt.body {
                    refs.extend(self.collect_stmt_refs(s));
                }
            }
            StmtKind::Block(stmts) => {
                for s in stmts {
                    refs.extend(self.collect_stmt_refs(s));
                }
            }
            StmtKind::Switch(switch_stmt) => {
                for case in &switch_stmt.cases {
                    refs.push(NodeRef::Case(case.id));
                    for s in &case.body {
                        refs.extend(self.collect_stmt_refs(s));
                    }
                }
                for s in &switch_stmt.default_body {
                    refs.extend(self.collect_stmt_refs(s));
                }
            }
            _ => {}
        }
        refs
    }

    #[allow(dead_code)]
    fn index_item(&mut self, item: &Item) {
        match item {
            Item::Function(f) => {
                self.node_index.insert(f.id, NodeRef::Function(f.id));
                for stmt in &f.body {
                    self.index_statement(stmt);
                }
            }
            Item::Agent(a) => {
                self.node_index.insert(a.id, NodeRef::Agent(a.id));
                for latent in &a.latents {
                    self.node_index
                        .insert(latent.id, NodeRef::Latent(latent.id));
                }
                for cycle in &a.cycles {
                    self.node_index.insert(cycle.id, NodeRef::Cycle(cycle.id));
                }
                for stmt in &a.body {
                    self.index_statement(stmt);
                }
                if let Some(ref govern) = a.govern {
                    self.node_index
                        .insert(govern.id, NodeRef::Govern(govern.id));
                }
                if let Some(ref modify) = a.modify {
                    self.node_index
                        .insert(modify.id, NodeRef::Modify(modify.id));
                }
            }
            Item::Cluster(c) => {
                self.node_index.insert(c.id, NodeRef::Cluster(c.id));
            }
            Item::Module(m) => {
                self.node_index.insert(m.id, NodeRef::Module(m.id));
            }
            Item::Import(i) => {
                self.node_index.insert(i.id, NodeRef::Import(i.id));
            }
            Item::Export(e) => {
                self.node_index.insert(e.id, NodeRef::Export(e.id));
            }
        }
    }

    #[allow(dead_code)]
    fn index_statement(&mut self, stmt: &Statement) {
        self.node_index.insert(stmt.id, NodeRef::Statement(stmt.id));
        // Recursively index nested statements and expressions
        match &stmt.kind {
            StmtKind::If(if_stmt) => {
                for s in &if_stmt.then_body {
                    self.index_statement(s);
                }
                for s in &if_stmt.else_body {
                    self.index_statement(s);
                }
            }
            StmtKind::Loop(loop_stmt) => {
                for s in &loop_stmt.body {
                    self.index_statement(s);
                }
            }
            StmtKind::Block(stmts) => {
                for s in stmts {
                    self.index_statement(s);
                }
            }
            StmtKind::Switch(switch_stmt) => {
                for case in &switch_stmt.cases {
                    self.node_index.insert(case.id, NodeRef::Case(case.id));
                    for s in &case.body {
                        self.index_statement(s);
                    }
                }
                for s in &switch_stmt.default_body {
                    self.index_statement(s);
                }
            }
            _ => {}
        }
    }

    /// Look up a node by its ID
    pub fn get_node(&self, id: NodeId) -> Option<NodeRef> {
        self.node_index.get(&id).copied()
    }
}

/// Type annotation (for future type system integration)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeAnnotation {
    pub name: String,
    pub parameters: Vec<TypeAnnotation>,
}

impl TypeAnnotation {
    pub fn new(name: impl Into<String>) -> Self {
        TypeAnnotation {
            name: name.into(),
            parameters: Vec::new(),
        }
    }

    pub fn i64() -> Self {
        TypeAnnotation::new("i64")
    }
    pub fn f64() -> Self {
        TypeAnnotation::new("f64")
    }
    pub fn string() -> Self {
        TypeAnnotation::new("String")
    }
    pub fn bool() -> Self {
        TypeAnnotation::new("bool")
    }
    pub fn array(inner: TypeAnnotation) -> Self {
        TypeAnnotation {
            name: "Array".into(),
            parameters: vec![inner],
        }
    }
    pub fn unknown() -> Self {
        TypeAnnotation::new("_")
    }
}

/// Attribute on a function (e.g., #[max_depth(50)])
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attribute {
    pub name: String,
    pub arguments: Vec<String>,
}

impl Attribute {
    pub fn new(name: impl Into<String>) -> Self {
        Attribute {
            name: name.into(),
            arguments: Vec::new(),
        }
    }

    pub fn with_args(name: impl Into<String>, args: Vec<String>) -> Self {
        Attribute {
            name: name.into(),
            arguments: args,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_ids_are_unique() {
        let id1 = NodeId::new();
        let id2 = NodeId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn program_creation() {
        let prog = Program::new("test");
        assert_eq!(prog.name, "test");
        assert!(prog.items.is_empty());
    }
}
