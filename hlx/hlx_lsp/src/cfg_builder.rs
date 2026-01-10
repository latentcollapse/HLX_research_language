//! CFG Builder - Constructs Control Flow Graphs from HLX Source Code
//!
//! This module parses HLX source and builds CFGs for dataflow analysis.

use crate::control_flow::{ControlFlowGraph, NodeKind};
use regex::Regex;
use std::collections::HashMap;

/// CFG Builder
pub struct CfgBuilder {
    /// Pattern for variable declarations
    let_pattern: Regex,
    /// Pattern for variable assignments
    assign_pattern: Regex,
    /// Pattern for variable uses
    ident_pattern: Regex,
    /// Pattern for function definitions
    fn_pattern: Regex,
}

impl CfgBuilder {
    pub fn new() -> Self {
        Self {
            // let name = value;
            let_pattern: Regex::new(r"^\s*let\s+(\w+)\s*=").unwrap(),
            // name = value; (not let)
            assign_pattern: Regex::new(r"^\s*(\w+)\s*=\s*[^=]").unwrap(),
            // Identifier uses (simplified)
            ident_pattern: Regex::new(r"\b(\w+)\b").unwrap(),
            // fn name(...) {
            fn_pattern: Regex::new(r"^\s*fn\s+(\w+)\s*\(").unwrap(),
        }
    }

    /// Build CFGs for all functions in source code
    pub fn build_all(&self, source: &str) -> HashMap<String, ControlFlowGraph> {
        let mut functions = HashMap::new();

        // Parse source to find functions
        let function_blocks = self.extract_functions(source);

        for (func_name, func_body) in function_blocks {
            let cfg = self.build_function_cfg(&func_body);
            functions.insert(func_name, cfg);
        }

        functions
    }

    /// Extract function blocks from source
    fn extract_functions(&self, source: &str) -> Vec<(String, Vec<(usize, String)>)> {
        let mut functions = Vec::new();
        let lines: Vec<&str> = source.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i];

            // Check if this is a function definition
            if let Some(caps) = self.fn_pattern.captures(line) {
                let func_name = caps.get(1).unwrap().as_str().to_string();

                // Find matching closing brace
                let mut brace_count = 0;
                let mut started = false;
                let mut func_lines = Vec::new();

                for (offset, func_line) in lines[i..].iter().enumerate() {
                    for ch in func_line.chars() {
                        if ch == '{' {
                            brace_count += 1;
                            started = true;
                        } else if ch == '}' {
                            brace_count -= 1;
                        }
                    }

                    if started {
                        func_lines.push((i + offset, func_line.to_string()));
                    }

                    if started && brace_count == 0 {
                        i += offset + 1;
                        break;
                    }
                }

                functions.push((func_name, func_lines));
                continue;
            }

            i += 1;
        }

        functions
    }

    /// Build CFG for a single function
    fn build_function_cfg(&self, lines: &[(usize, String)]) -> ControlFlowGraph {
        let mut cfg = ControlFlowGraph::new();
        let mut current = cfg.entry;
        let mut declared_vars = HashMap::new();

        for (line_num, line) in lines {
            // Skip comments and empty lines
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("//") {
                continue;
            }

            // Check for variable declaration
            if let Some(caps) = self.let_pattern.captures(line) {
                let var_name = caps.get(1).unwrap().as_str().to_string();
                declared_vars.insert(var_name.clone(), *line_num);

                let node = cfg.new_node(NodeKind::VarDecl {
                    name: var_name,
                    line: *line_num,
                });
                cfg.add_edge(current, node);
                current = node;
                continue;
            }

            // Check for assignment (not declaration)
            if let Some(caps) = self.assign_pattern.captures(line) {
                let var_name = caps.get(1).unwrap().as_str().to_string();

                // Only treat as assignment if not a keyword
                if !Self::is_keyword(&var_name) {
                    let node = cfg.new_node(NodeKind::VarAssign {
                        name: var_name,
                        line: *line_num,
                    });
                    cfg.add_edge(current, node);
                    current = node;
                    continue;
                }
            }

            // Check for control flow keywords
            if trimmed.starts_with("if ") || trimmed.starts_with("if(") {
                let node = cfg.new_node(NodeKind::Branch { line: *line_num });
                cfg.add_edge(current, node);
                current = node;
                // TODO: Handle branches properly with multiple successors
                continue;
            }

            if trimmed.starts_with("loop ") || trimmed.starts_with("loop(") {
                let node = cfg.new_node(NodeKind::Loop { line: *line_num });
                cfg.add_edge(current, node);
                current = node;
                continue;
            }

            if trimmed.starts_with("return") {
                let node = cfg.new_node(NodeKind::Return { line: *line_num });
                cfg.add_edge(current, node);
                cfg.add_edge(node, cfg.exit);
                // Return ends control flow in this path
                continue;
            }

            // Check for variable uses
            // This is simplified - just looks for identifiers
            for var_name in declared_vars.keys() {
                if line.contains(var_name) && !line.contains(&format!("let {}", var_name)) {
                    // Potential use - add use node
                    let node = cfg.new_node(NodeKind::VarUse {
                        name: var_name.clone(),
                        line: *line_num,
                    });
                    cfg.add_edge(current, node);
                    current = node;
                }
            }

            // Default: other statement
            if !trimmed.starts_with("{") && !trimmed.starts_with("}") {
                let node = cfg.new_node(NodeKind::Other { line: *line_num });
                cfg.add_edge(current, node);
                current = node;
            }
        }

        // Connect to exit if not already connected
        if current != cfg.exit {
            cfg.add_edge(current, cfg.exit);
        }

        cfg
    }

    fn is_keyword(s: &str) -> bool {
        matches!(
            s,
            "fn" | "let"
                | "if"
                | "else"
                | "loop"
                | "return"
                | "break"
                | "continue"
                | "program"
                | "block"
        )
    }
}

impl Default for CfgBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_function_cfg() {
        let builder = CfgBuilder::new();
        let source = r#"
fn main() {
    let x = 5;
    let y = 10;
    return x + y;
}
        "#;

        let cfgs = builder.build_all(source);
        assert_eq!(cfgs.len(), 1);
        assert!(cfgs.contains_key("main"));

        let cfg = &cfgs["main"];
        // Should have: entry, 2 vardecls, 1 return, exit = 5 nodes minimum
        assert!(cfg.nodes.len() >= 5);
    }

    #[test]
    fn test_extract_functions() {
        let builder = CfgBuilder::new();
        let source = r#"
fn foo() {
    let x = 1;
}

fn bar() {
    let y = 2;
}
        "#;

        let funcs = builder.extract_functions(source);
        assert_eq!(funcs.len(), 2);
        assert_eq!(funcs[0].0, "foo");
        assert_eq!(funcs[1].0, "bar");
    }

    #[test]
    fn test_variable_declaration_detection() {
        let builder = CfgBuilder::new();
        let lines = vec![
            (1, "    let x = 5;".to_string()),
            (2, "    let y = x + 1;".to_string()),
        ];

        let cfg = builder.build_function_cfg(&lines);
        let decls = cfg.get_declared_vars();
        assert_eq!(decls.len(), 2);
        assert!(decls.contains("x"));
        assert!(decls.contains("y"));
    }
}
