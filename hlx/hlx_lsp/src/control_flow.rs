//! Control Flow Graph (CFG) Construction
//!
//! Builds a CFG from HLX AST to enable dataflow analysis.

use std::collections::{HashMap, HashSet};

/// Unique identifier for a CFG node
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub usize);

/// Control flow graph node
#[derive(Debug, Clone)]
pub struct CfgNode {
    pub id: NodeId,
    pub kind: NodeKind,
    /// Successors in control flow
    pub successors: Vec<NodeId>,
    /// Predecessors in control flow
    pub predecessors: Vec<NodeId>,
}

/// Type of CFG node
#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
    /// Function entry point
    Entry,
    /// Function exit point
    Exit,
    /// Variable declaration: let name = value;
    VarDecl { name: String, line: usize },
    /// Variable assignment: name = value;
    VarAssign { name: String, line: usize },
    /// Variable use (read): name
    VarUse { name: String, line: usize },
    /// Conditional branch: if (condition)
    Branch { line: usize },
    /// Loop header: loop(condition, max)
    Loop { line: usize },
    /// Return statement
    Return { line: usize },
    /// Other statement (expression, break, continue)
    Other { line: usize },
}

/// Control Flow Graph for a function
#[derive(Debug)]
pub struct ControlFlowGraph {
    pub nodes: HashMap<NodeId, CfgNode>,
    pub entry: NodeId,
    pub exit: NodeId,
    next_id: usize,
}

impl ControlFlowGraph {
    pub fn new() -> Self {
        let mut cfg = Self {
            nodes: HashMap::new(),
            entry: NodeId(0),
            exit: NodeId(1),
            next_id: 2,
        };

        // Create entry and exit nodes
        cfg.nodes.insert(
            cfg.entry,
            CfgNode {
                id: cfg.entry,
                kind: NodeKind::Entry,
                successors: vec![],
                predecessors: vec![],
            },
        );

        cfg.nodes.insert(
            cfg.exit,
            CfgNode {
                id: cfg.exit,
                kind: NodeKind::Exit,
                successors: vec![],
                predecessors: vec![],
            },
        );

        cfg
    }

    /// Create a new node
    pub fn new_node(&mut self, kind: NodeKind) -> NodeId {
        let id = NodeId(self.next_id);
        self.next_id += 1;

        self.nodes.insert(
            id,
            CfgNode {
                id,
                kind,
                successors: vec![],
                predecessors: vec![],
            },
        );

        id
    }

    /// Add an edge from `from` to `to`
    pub fn add_edge(&mut self, from: NodeId, to: NodeId) {
        if let Some(node) = self.nodes.get_mut(&from) {
            if !node.successors.contains(&to) {
                node.successors.push(to);
            }
        }

        if let Some(node) = self.nodes.get_mut(&to) {
            if !node.predecessors.contains(&from) {
                node.predecessors.push(from);
            }
        }
    }

    /// Get all variable names declared in this CFG
    pub fn get_declared_vars(&self) -> HashSet<String> {
        let mut vars = HashSet::new();

        for node in self.nodes.values() {
            if let NodeKind::VarDecl { name, .. } = &node.kind {
                vars.insert(name.clone());
            }
        }

        vars
    }

    /// Get all variable uses (reads)
    pub fn get_var_uses(&self, var_name: &str) -> Vec<(NodeId, usize)> {
        let mut uses = Vec::new();

        for node in self.nodes.values() {
            match &node.kind {
                NodeKind::VarUse { name, line } if name == var_name => {
                    uses.push((node.id, *line));
                }
                NodeKind::VarAssign { name, line } if name == var_name => {
                    // Assignment RHS reads the old value (in expressions like x = x + 1)
                    // For now, we conservatively assume assignments also use the variable
                    // TODO: Parse RHS to detect actual uses
                }
                _ => {}
            }
        }

        uses
    }

    /// Check if there's a path from `from` to `to`
    pub fn has_path(&self, from: NodeId, to: NodeId) -> bool {
        let mut visited = HashSet::new();
        let mut stack = vec![from];

        while let Some(current) = stack.pop() {
            if current == to {
                return true;
            }

            if visited.contains(&current) {
                continue;
            }
            visited.insert(current);

            if let Some(node) = self.nodes.get(&current) {
                stack.extend(node.successors.iter().copied());
            }
        }

        false
    }
}

impl Default for ControlFlowGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cfg_creation() {
        let cfg = ControlFlowGraph::new();
        assert_eq!(cfg.nodes.len(), 2); // Entry + Exit
        assert!(cfg.nodes.contains_key(&cfg.entry));
        assert!(cfg.nodes.contains_key(&cfg.exit));
    }

    #[test]
    fn test_add_node_and_edge() {
        let mut cfg = ControlFlowGraph::new();
        let n1 = cfg.new_node(NodeKind::VarDecl {
            name: "x".to_string(),
            line: 1,
        });
        let n2 = cfg.new_node(NodeKind::VarUse {
            name: "x".to_string(),
            line: 2,
        });

        cfg.add_edge(cfg.entry, n1);
        cfg.add_edge(n1, n2);
        cfg.add_edge(n2, cfg.exit);

        assert_eq!(cfg.nodes.get(&cfg.entry).unwrap().successors, vec![n1]);
        assert_eq!(cfg.nodes.get(&n1).unwrap().successors, vec![n2]);
        assert_eq!(cfg.nodes.get(&n2).unwrap().successors, vec![cfg.exit]);
    }

    #[test]
    fn test_has_path() {
        let mut cfg = ControlFlowGraph::new();
        let n1 = cfg.new_node(NodeKind::Other { line: 1 });
        let n2 = cfg.new_node(NodeKind::Other { line: 2 });

        cfg.add_edge(cfg.entry, n1);
        cfg.add_edge(n1, n2);
        cfg.add_edge(n2, cfg.exit);

        assert!(cfg.has_path(cfg.entry, cfg.exit));
        assert!(cfg.has_path(n1, cfg.exit));
        assert!(!cfg.has_path(cfg.exit, cfg.entry));
    }

    #[test]
    fn test_get_declared_vars() {
        let mut cfg = ControlFlowGraph::new();
        cfg.new_node(NodeKind::VarDecl {
            name: "x".to_string(),
            line: 1,
        });
        cfg.new_node(NodeKind::VarDecl {
            name: "y".to_string(),
            line: 2,
        });

        let vars = cfg.get_declared_vars();
        assert_eq!(vars.len(), 2);
        assert!(vars.contains("x"));
        assert!(vars.contains("y"));
    }
}
