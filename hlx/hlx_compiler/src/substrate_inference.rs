//! Substrate Inference Engine
//!
//! Determines the optimal execution substrate for HLX-S code based on:
//! - AST structure (deterministic via hashing)
//! - Operation vocabulary (which ops hint at which substrate)
//! - Explicit pragmas (user overrides)
//! - Swarm configuration (size hints)

use crate::ast::{Block, Expr, Item, Program, Statement};
use crate::substrate::{Substrate, SubstrateInfo, OperationHints, ScaleConfig, ScaleSize};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

/// Substrate inference engine
pub struct SubstrateInference {
    /// Operation hints for vocabulary-based inference
    hints: OperationHints,
    /// Cache of inferred substrates (AST hash → substrate)
    cache: HashMap<u64, SubstrateInfo>,
}

impl SubstrateInference {
    /// Create a new inference engine
    pub fn new() -> Self {
        Self {
            hints: OperationHints::new(),
            cache: HashMap::new(),
        }
    }

    /// Infer substrate for an entire program
    pub fn infer_program(&mut self, program: &Program) -> Vec<(String, SubstrateInfo)> {
        let mut results = Vec::new();

        // Infer for top-level blocks
        for block in &program.blocks {
            let info = self.infer_block(block);
            results.push((block.name.clone(), info));
        }

        // Infer for module blocks
        for module in &program.modules {
            for block in &module.blocks {
                let info = self.infer_block(block);
                results.push((format!("{}.{}", module.name, block.name), info));
            }
        }

        results
    }

    /// Infer substrate for a single block/function
    pub fn infer_block(&mut self, block: &Block) -> SubstrateInfo {
        // Step 1: Check for explicit substrate pragma
        if let Some(explicit) = self.parse_explicit_substrate(block) {
            return SubstrateInfo {
                substrate: explicit,
                inference_confidence: 1.0,
                speedup_estimate: None,
                agent_count: None,
                barrier_count: 0,
                reasoning: format!("Explicit pragma: @substrate({})", explicit.to_str()),
                ast_hash: None,
            };
        }

        // Step 2: Check for swarm configuration
        if let Some(swarm) = self.parse_scale_config(block) {
            return self.infer_from_scale(&swarm, block);
        }

        // Step 3: Hash-based inference (deterministic)
        let ast_hash = self.hash_block(block);

        // Check cache
        if let Some(cached) = self.cache.get(&ast_hash) {
            return cached.clone();
        }

        // Step 4: Analyze operations to infer substrate
        let info = self.analyze_operations(block, ast_hash);

        // Cache the result
        self.cache.insert(ast_hash, info.clone());

        info
    }

    /// Parse explicit @substrate pragma from block attributes
    fn parse_explicit_substrate(&self, block: &Block) -> Option<Substrate> {
        for attr in &block.attributes {
            if attr.starts_with("substrate(") && attr.ends_with(')') {
                let substrate_str = &attr[10..attr.len()-1];
                return Substrate::parse(substrate_str);
            }
        }
        None
    }

    /// Parse @scale configuration from block attributes
    fn parse_scale_config(&self, block: &Block) -> Option<ScaleConfig> {
        for attr in &block.attributes {
            if attr.starts_with("scale(") && attr.ends_with(')') {
                // Simple parsing for now: scale(size=1000)
                let config_str = &attr[6..attr.len()-1];
                return self.parse_scale_simple(config_str);
            }
        }
        None
    }

    /// Simple swarm config parser (basic key=value pairs)
    fn parse_scale_simple(&self, config: &str) -> Option<ScaleConfig> {
        let mut size = None;
        let mut substrate = None;

        for pair in config.split(',') {
            let parts: Vec<&str> = pair.trim().split('=').collect();
            if parts.len() == 2 {
                match parts[0].trim() {
                    "size" => {
                        size = ScaleSize::parse(parts[1].trim());
                    }
                    "substrate" => {
                        substrate = Substrate::parse(parts[1].trim());
                    }
                    _ => {}
                }
            }
        }

        size.map(|s| ScaleConfig {
            size: s,
            substrate,
            memory_limit: None,
            barrier: None,
            sync_protocol: None,
        })
    }

    /// Infer substrate from swarm configuration
    fn infer_from_scale(&self, swarm: &ScaleConfig, block: &Block) -> SubstrateInfo {
        // If substrate explicitly specified in swarm, use it
        if let Some(explicit) = swarm.substrate {
            return SubstrateInfo {
                substrate: explicit,
                inference_confidence: 1.0,
                speedup_estimate: None,
                agent_count: swarm.size.to_u64(),
                barrier_count: self.count_barriers(block),
                reasoning: format!("Swarm with explicit substrate: {}", explicit.to_str()),
                ast_hash: Some(format!("{:x}", self.hash_block(block))),
            };
        }

        // Infer from swarm size
        let substrate = if swarm.size.suggests_quantum() {
            Substrate::QuantumSim
        } else {
            Substrate::CPU
        };

        SubstrateInfo {
            substrate,
            inference_confidence: 0.8,
            speedup_estimate: swarm.size.to_u64().map(|n| n as f64 / 10.0),
            agent_count: swarm.size.to_u64(),
            barrier_count: self.count_barriers(block),
            reasoning: format!(
                "Swarm size {:?} suggests {} substrate",
                swarm.size,
                substrate.to_str()
            ),
            ast_hash: Some(format!("{:x}", self.hash_block(block))),
        }
    }

    /// Hash a block's AST for deterministic inference
    fn hash_block(&self, block: &Block) -> u64 {
        let mut hasher = DefaultHasher::new();

        // Hash block name
        block.name.hash(&mut hasher);

        // Hash parameters
        for (param_name, _, param_type) in &block.params {
            param_name.hash(&mut hasher);
            if let Some((typ, _)) = param_type {
                format!("{:?}", typ).hash(&mut hasher);
            }
        }

        // Hash return type
        if let Some(ret_type) = &block.return_type {
            format!("{:?}", ret_type).hash(&mut hasher);
        }

        // Hash items (statements/nodes)
        for item in &block.items {
            self.hash_item(&item.node, &mut hasher);
        }

        hasher.finish()
    }

    /// Hash an item (statement or node)
    fn hash_item(&self, item: &Item, hasher: &mut DefaultHasher) {
        match item {
            Item::Statement(stmt) => {
                format!("{:?}", stmt).hash(hasher);
            }
            Item::Node(node) => {
                node.id.hash(hasher);
                node.op.hash(hasher);
                node.outputs.len().hash(hasher);
            }
        }
    }

    /// Analyze operations in a block to infer substrate
    fn analyze_operations(&self, block: &Block, ast_hash: u64) -> SubstrateInfo {
        let mut cpu_score: f64 = 0.0;
        let mut quantum_score: f64 = 0.0;
        let mut operation_count = 0;

        // Collect all operations
        let ops = self.collect_operations(block);

        for op in &ops {
            operation_count += 1;

            // Check hints
            if let Some(hint) = self.hints.get_hint(op) {
                match hint {
                    Substrate::CPU => cpu_score += 1.0,
                    Substrate::QuantumSim => quantum_score += 1.0,
                    _ => {}
                }
            } else {
                // Default to CPU for unknown operations
                cpu_score += 0.5;
            }
        }

        // Determine substrate based on scores
        let substrate = if quantum_score > cpu_score * 1.5 {
            // Significant quantum advantage
            Substrate::QuantumSim
        } else if quantum_score > 0.0 && cpu_score > 0.0 {
            // Mixed operations → hybrid
            Substrate::Hybrid
        } else {
            // Default to CPU
            Substrate::CPU
        };

        let confidence = if operation_count > 0 {
            (cpu_score.max(quantum_score) / operation_count as f64).min(1.0)
        } else {
            1.0  // Empty block → CPU with high confidence
        };

        SubstrateInfo {
            substrate,
            inference_confidence: confidence,
            speedup_estimate: None,
            agent_count: None,
            barrier_count: self.count_barriers(block),
            reasoning: format!(
                "Inferred from {} operations (CPU: {:.1}, Quantum: {:.1})",
                operation_count, cpu_score, quantum_score
            ),
            ast_hash: Some(format!("{:x}", ast_hash)),
        }
    }

    /// Collect all operation names from a block
    fn collect_operations(&self, block: &Block) -> Vec<String> {
        let mut ops = Vec::new();

        for item in &block.items {
            match &item.node {
                Item::Statement(stmt) => {
                    self.collect_ops_from_statement(stmt, &mut ops);
                }
                Item::Node(node) => {
                    ops.push(node.op.clone());
                }
            }
        }

        ops
    }

    /// Collect operations from a statement (recursively)
    fn collect_ops_from_statement(&self, stmt: &Statement, ops: &mut Vec<String>) {
        match stmt {
            Statement::Let { value, .. } | Statement::Assign { value, .. } => {
                self.collect_ops_from_expr(&value.node, ops);
            }
            Statement::Local { value, .. } => {
                self.collect_ops_from_expr(&value.node, ops);
            }
            Statement::Return { value, .. } => {
                self.collect_ops_from_expr(&value.node, ops);
            }
            Statement::If { condition, then_branch, else_branch, .. } => {
                self.collect_ops_from_expr(&condition.node, ops);
                for stmt_spanned in then_branch {
                    self.collect_ops_from_statement(&stmt_spanned.node, ops);
                }
                if let Some(else_stmts) = else_branch {
                    for stmt_spanned in else_stmts {
                        self.collect_ops_from_statement(&stmt_spanned.node, ops);
                    }
                }
            }
            Statement::While { condition, body, .. } => {
                self.collect_ops_from_expr(&condition.node, ops);
                for stmt_spanned in body {
                    self.collect_ops_from_statement(&stmt_spanned.node, ops);
                }
            }
            Statement::Expr(expr) => {
                self.collect_ops_from_expr(&expr.node, ops);
            }
            _ => {}
        }
    }

    /// Collect operations from an expression
    fn collect_ops_from_expr(&self, expr: &Expr, ops: &mut Vec<String>) {
        match expr {
            Expr::Call { func, args } => {
                // Try to extract function name if it's an identifier
                if let Expr::Ident(name) = &func.node {
                    ops.push(name.clone());
                }
                // Recurse into func and args
                self.collect_ops_from_expr(&func.node, ops);
                for arg in args {
                    self.collect_ops_from_expr(&arg.node, ops);
                }
            }
            Expr::BinOp { lhs, rhs, .. } => {
                self.collect_ops_from_expr(&lhs.node, ops);
                self.collect_ops_from_expr(&rhs.node, ops);
            }
            Expr::UnaryOp { operand, .. } => {
                self.collect_ops_from_expr(&operand.node, ops);
            }
            Expr::Index { object, index } => {
                self.collect_ops_from_expr(&object.node, ops);
                self.collect_ops_from_expr(&index.node, ops);
            }
            Expr::Array(elements) => {
                for elem in elements {
                    self.collect_ops_from_expr(&elem.node, ops);
                }
            }
            Expr::Field { object, .. } => {
                self.collect_ops_from_expr(&object.node, ops);
            }
            Expr::Pipe { value, func } => {
                self.collect_ops_from_expr(&value.node, ops);
                self.collect_ops_from_expr(&func.node, ops);
            }
            _ => {}
        }
    }

    /// Count synchronization barriers in a block
    fn count_barriers(&self, block: &Block) -> usize {
        let mut count = 0;
        for item in &block.items {
            if let Item::Statement(stmt) = &item.node {
                self.count_barriers_in_statement(stmt, &mut count);
            }
        }
        count
    }

    /// Count barriers recursively in a statement
    fn count_barriers_in_statement(&self, stmt: &Statement, count: &mut usize) {
        match stmt {
            Statement::Barrier { .. } => {
                *count += 1;
            }
            Statement::If { then_branch, else_branch, .. } => {
                for stmt_spanned in then_branch {
                    self.count_barriers_in_statement(&stmt_spanned.node, count);
                }
                if let Some(else_stmts) = else_branch {
                    for stmt_spanned in else_stmts {
                        self.count_barriers_in_statement(&stmt_spanned.node, count);
                    }
                }
            }
            Statement::While { body, .. } => {
                for stmt_spanned in body {
                    self.count_barriers_in_statement(&stmt_spanned.node, count);
                }
            }
            _ => {}
        }
    }
}

impl Default for SubstrateInference {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Block, Type};

    #[test]
    fn test_explicit_substrate_pragma() {
        let mut inference = SubstrateInference::new();

        let block = Block {
            name: "test".to_string(),
            attributes: vec!["substrate(quantum_sim)".to_string()],
            name_span: None,
            fn_keyword_span: None,
            params: vec![],
            return_type: None,
            return_type_span: None,
            items: vec![],
        };

        let info = inference.infer_block(&block);
        assert_eq!(info.substrate, Substrate::QuantumSim);
        assert_eq!(info.inference_confidence, 1.0);
    }

    #[test]
    fn test_swarm_size_inference() {
        let mut inference = SubstrateInference::new();

        let block = Block {
            name: "test".to_string(),
            attributes: vec!["swarm(size=2^50)".to_string()],
            name_span: None,
            fn_keyword_span: None,
            params: vec![],
            return_type: None,
            return_type_span: None,
            items: vec![],
        };

        let info = inference.infer_block(&block);
        assert_eq!(info.substrate, Substrate::QuantumSim);
        assert!(info.reasoning.contains("Swarm size"));
    }

    #[test]
    fn test_hash_determinism() {
        let inference = SubstrateInference::new();

        let block = Block {
            name: "test".to_string(),
            attributes: vec![],
            name_span: None,
            fn_keyword_span: None,
            params: vec![("x".to_string(), None, Some((Type::Float, None)))],
            return_type: Some(Type::Float),
            return_type_span: None,
            items: vec![],
        };

        let hash1 = inference.hash_block(&block);
        let hash2 = inference.hash_block(&block);

        // Same block should produce same hash
        assert_eq!(hash1, hash2);
    }
}
