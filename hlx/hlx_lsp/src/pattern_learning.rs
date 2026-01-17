//! Pattern Learning System
//!
//! Learns from the user's codebase to provide personalized suggestions.
//! Adapts to coding style, common patterns, and frequently used contracts.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use hlx_compiler::ast::{Block, Expr, Program, Statement};

/// A learned pattern from the codebase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnedPattern {
    /// Pattern name
    pub name: String,
    /// Pattern description
    pub description: String,
    /// How often it appears
    pub frequency: usize,
    /// Example code
    pub example: String,
    /// Suggested completion text
    pub completion: String,
    /// Confidence score (0.0-1.0)
    pub confidence: f32,
}

/// Coding style preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodingStyle {
    /// Preferred indentation (spaces)
    pub indent_size: usize,
    /// Preferred naming convention
    pub naming_convention: NamingConvention,
    /// Average function length
    pub avg_function_length: u32,
    /// Prefers contracts vs manual ops
    pub contract_preference: f32, // 0.0 = manual, 1.0 = contracts
    /// Common imports
    pub common_imports: Vec<String>,
    /// Frequently used contracts
    pub favorite_contracts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NamingConvention {
    SnakeCase,
    CamelCase,
    PascalCase,
    Mixed,
}

/// Pattern learning engine
pub struct PatternLearner {
    /// Learned patterns
    patterns: Vec<LearnedPattern>,
    /// Coding style
    style: CodingStyle,
    /// Function name patterns
    function_names: HashMap<String, usize>,
    /// Variable name patterns
    variable_names: HashMap<String, usize>,
    /// Contract usage frequency
    contract_usage: HashMap<String, usize>,
    /// Common code sequences
    sequences: HashMap<String, usize>,
}

impl PatternLearner {
    pub fn new() -> Self {
        Self {
            patterns: Vec::new(),
            style: CodingStyle {
                indent_size: 4,
                naming_convention: NamingConvention::SnakeCase,
                avg_function_length: 10,
                contract_preference: 0.5,
                common_imports: Vec::new(),
                favorite_contracts: Vec::new(),
            },
            function_names: HashMap::new(),
            variable_names: HashMap::new(),
            contract_usage: HashMap::new(),
            sequences: HashMap::new(),
        }
    }

    /// Learn from a codebase
    pub fn learn_from_workspace(&mut self, files: &HashMap<String, Program>) {
        for (_, program) in files {
            self.learn_from_program(program);
        }

        self.analyze_patterns();
        self.update_style();
    }

    /// Learn from a single program
    fn learn_from_program(&mut self, program: &Program) {
        // Learn imports
        for import in &program.imports {
            *self.style.common_imports
                .iter_mut()
                .find(|i| *i == &import.path)
                .unwrap_or(&mut String::new()) = import.path.clone();

            if !self.style.common_imports.contains(&import.path) {
                self.style.common_imports.push(import.path.clone());
            }
        }

        // Learn from functions
        for block in &program.blocks {
            self.learn_from_block(block);
        }
    }

    /// Learn from a function/block
    fn learn_from_block(&mut self, block: &Block) {
        // Learn function name
        let name = &block.name;
        *self.function_names.entry(name.clone()).or_insert(0) += 1;

        // Track function length
        let length = block.items.len() as u32;
        self.style.avg_function_length =
            (self.style.avg_function_length + length) / 2;

        // Learn from items
        for item in &block.items {
            match &item.node {
                hlx_compiler::ast::Item::Statement(stmt) => {
                    self.learn_from_statement(stmt);
                }
                _ => {}
            }
        }
    }

    /// Learn from a statement
    fn learn_from_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Local { keyword_span: _, name, name_span: _, value, .. } => {
                // Learn variable name
                *self.variable_names.entry(name.clone()).or_insert(0) += 1;

                // Learn from value expression
                self.learn_from_expr(&value.node);
            }
            Statement::Expr(expr) => {
                self.learn_from_expr(&expr.node);
            }
            Statement::If { condition, then_branch, else_branch, .. } => {
                self.learn_from_expr(&condition.node);

                for stmt in then_branch {
                    self.learn_from_statement(&stmt.node);
                }

                if let Some(else_stmts) = else_branch {
                    for stmt in else_stmts {
                        self.learn_from_statement(&stmt.node);
                    }
                }
            }
            Statement::While { condition, body, .. } => {
                self.learn_from_expr(&condition.node);

                for stmt in body {
                    self.learn_from_statement(&stmt.node);
                }
            }
            Statement::Return { keyword_span: _, value } => {
                self.learn_from_expr(&value.node);
            }
            _ => {}
        }
    }

    /// Learn from an expression
    fn learn_from_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Call { func, args, .. } => {
                // Check if it's a contract call
                if let Expr::Ident(name) = &func.node {
                    if name.starts_with('@') {
                        *self.contract_usage.entry(name.clone()).or_insert(0) += 1;
                    }
                }

                // Learn from arguments
                for arg in args {
                    self.learn_from_expr(&arg.node);
                }
            }
            Expr::BinOp { lhs, rhs, .. } => {
                self.learn_from_expr(&lhs.node);
                self.learn_from_expr(&rhs.node);

                // Track manual operations (preference indicator)
                self.style.contract_preference =
                    (self.style.contract_preference * 0.95).max(0.0);
            }
            Expr::Array(elements) => {
                for elem in elements {
                    self.learn_from_expr(&elem.node);
                }
            }
            _ => {}
        }
    }

    /// Analyze learned data to extract patterns
    fn analyze_patterns(&mut self) {
        self.patterns.clear();

        // Pattern 1: Common function naming
        let top_func_names: Vec<_> = {
            let mut sorted: Vec<_> = self.function_names.iter().collect();
            sorted.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
            sorted.into_iter().take(5).collect()
        };

        for (name, count) in top_func_names {
            if *count > 2 {
                self.patterns.push(LearnedPattern {
                    name: format!("Function naming: {}", name),
                    description: format!("You frequently name functions like '{}'", name),
                    frequency: *count,
                    example: format!("fn {}() {{ }}", name),
                    completion: format!("fn {}_", name),
                    confidence: (*count as f32 / self.function_names.len() as f32).min(0.9),
                });
            }
        }

        // Pattern 2: Favorite contracts
        let top_contracts: Vec<_> = {
            let mut sorted: Vec<_> = self.contract_usage.iter().collect();
            sorted.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
            sorted.into_iter().take(5).collect()
        };

        for (contract, count) in top_contracts {
            if *count > 3 {
                self.patterns.push(LearnedPattern {
                    name: format!("Contract usage: {}", contract),
                    description: format!("You frequently use {} contract", contract),
                    frequency: *count,
                    example: format!("{} {{ }}", contract),
                    completion: format!("{} {{ }}", contract),
                    confidence: (*count as f32 / self.contract_usage.len() as f32).min(0.9),
                });

                // Add to favorites
                if !self.style.favorite_contracts.contains(contract) {
                    self.style.favorite_contracts.push(contract.clone());
                }
            }
        }

        // Pattern 3: Variable naming patterns
        let var_pattern = self.detect_naming_pattern(&self.variable_names);
        if let Some(pattern) = var_pattern {
            self.patterns.push(pattern);
        }
    }

    /// Detect naming convention pattern
    fn detect_naming_pattern(&self, names: &HashMap<String, usize>) -> Option<LearnedPattern> {
        let mut snake_case = 0;
        let mut camel_case = 0;
        let mut pascal_case = 0;

        for name in names.keys() {
            if name.contains('_') {
                snake_case += 1;
            } else if name.chars().next()?.is_lowercase() {
                camel_case += 1;
            } else if name.chars().next()?.is_uppercase() {
                pascal_case += 1;
            }
        }

        let total = snake_case + camel_case + pascal_case;
        if total == 0 {
            return None;
        }

        let (convention, count) = if snake_case > camel_case && snake_case > pascal_case {
            ("snake_case", snake_case)
        } else if camel_case > pascal_case {
            ("camelCase", camel_case)
        } else {
            ("PascalCase", pascal_case)
        };

        Some(LearnedPattern {
            name: format!("Naming convention: {}", convention),
            description: format!("You prefer {} naming style", convention),
            frequency: count,
            example: format!("let my_variable = ..."),
            completion: String::new(),
            confidence: (count as f32 / total as f32).min(0.9),
        })
    }

    /// Update coding style preferences
    fn update_style(&mut self) {
        // Update naming convention
        let mut snake = 0;
        let mut camel = 0;

        for name in self.variable_names.keys().chain(self.function_names.keys()) {
            if name.contains('_') {
                snake += 1;
            } else {
                camel += 1;
            }
        }

        self.style.naming_convention = if snake > camel {
            NamingConvention::SnakeCase
        } else {
            NamingConvention::CamelCase
        };

        // Update contract preference
        let total_ops = self.contract_usage.values().sum::<usize>() as f32;
        let manual_ops = 100.0; // Estimate from binary ops

        if total_ops + manual_ops > 0.0 {
            self.style.contract_preference = total_ops / (total_ops + manual_ops);
        }
    }

    /// Get personalized suggestions based on context
    pub fn suggest(&self, context: &str) -> Vec<LearnedPattern> {
        let context_lower = context.to_lowercase();

        self.patterns
            .iter()
            .filter(|p| {
                // Match on context
                context_lower.contains(&p.name.to_lowercase())
                    || p.description.to_lowercase().contains(&context_lower)
            })
            .cloned()
            .collect()
    }

    /// Get all learned patterns
    pub fn get_patterns(&self) -> &[LearnedPattern] {
        &self.patterns
    }

    /// Get coding style
    pub fn get_style(&self) -> &CodingStyle {
        &self.style
    }

    /// Suggest next likely token based on patterns
    pub fn predict_next(&self, current_text: &str) -> Vec<String> {
        let mut predictions = Vec::new();

        // If typing function name, suggest common patterns
        if current_text.ends_with("fn ") {
            for (name, _) in self.function_names.iter().take(5) {
                predictions.push(format!("{}()", name));
            }
        }

        // If typing contract, suggest favorites
        if current_text.ends_with('@') {
            for contract in &self.style.favorite_contracts {
                predictions.push(contract.strip_prefix('@').unwrap_or(contract).to_string());
            }
        }

        predictions
    }
}

impl Default for PatternLearner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_learning() {
        let mut learner = PatternLearner::new();

        // Simulate learning
        learner.function_names.insert("calculate_total".to_string(), 5);
        learner.function_names.insert("process_data".to_string(), 3);
        learner.contract_usage.insert("@200".to_string(), 10);

        learner.analyze_patterns();

        assert!(!learner.patterns.is_empty());
        assert!(learner.patterns.iter().any(|p| p.name.contains("calculate_total")));
    }

    #[test]
    fn test_naming_convention_detection() {
        let mut names = HashMap::new();
        names.insert("my_variable".to_string(), 1);
        names.insert("another_var".to_string(), 1);
        names.insert("camelCase".to_string(), 1);

        let learner = PatternLearner::new();
        let pattern = learner.detect_naming_pattern(&names);

        assert!(pattern.is_some());
    }

    #[test]
    fn test_contract_preference() {
        let mut learner = PatternLearner::new();
        learner.contract_usage.insert("@200".to_string(), 10);
        learner.update_style();

        // Should prefer contracts over manual ops
        assert!(learner.style.contract_preference > 0.0);
    }

    #[test]
    fn test_prediction() {
        let mut learner = PatternLearner::new();
        learner.style.favorite_contracts.push("@200".to_string());

        let predictions = learner.predict_next("@");
        assert!(!predictions.is_empty());
        assert!(predictions.iter().any(|p| p.contains("200")));
    }
}
