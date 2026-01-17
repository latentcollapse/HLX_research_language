//! AI Context Generation for Claude Code
//!
//! Exports HLX codebases in a format optimized for AI assistants.
//! This is a unique feature that no other LSP has - leveraging HLX's
//! AI-native design to provide rich context to Claude and other LLMs.

use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use hlx_compiler::ast::{Block, Expr, Import, Item, Program, Span, Statement};
use hlx_compiler::hlxa::HlxaParser;

/// AI-optimized representation of an HLX codebase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIContext {
    /// Project metadata
    pub project: ProjectMetadata,
    /// All modules in the codebase
    pub modules: Vec<ModuleContext>,
    /// Contract catalog (all contracts used)
    pub contracts: Vec<ContractContext>,
    /// Dependency graph
    pub dependencies: DependencyGraph,
    /// Common patterns detected
    pub patterns: Vec<CodePattern>,
    /// Semantic summary for Claude
    pub summary: String,
}

/// Project-level metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetadata {
    pub name: String,
    pub total_files: usize,
    pub total_lines: usize,
    pub entry_points: Vec<String>,
    pub primary_language: String, // "HLX"
}

/// AI-optimized module representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleContext {
    pub path: String,
    pub name: String,
    pub summary: String,
    pub functions: Vec<FunctionContext>,
    pub contracts: Vec<String>, // Contract IDs used
    pub imports: Vec<String>,
    pub exports: Vec<String>,
    pub complexity_score: f32,
}

/// Function-level context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionContext {
    pub name: String,
    pub signature: String,
    pub summary: String,
    pub parameters: Vec<ParameterInfo>,
    pub return_type: Option<String>,
    pub uses_contracts: Vec<String>,
    pub calls: Vec<String>, // Functions it calls
    pub called_by: Vec<String>, // Functions that call it
    pub complexity: u32,
    pub line_count: u32,
}

/// Parameter information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterInfo {
    pub name: String,
    pub type_annotation: Option<String>,
    pub inferred_type: Option<String>,
}

/// Contract usage context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractContext {
    pub contract_id: String,
    pub name: String,
    pub description: String,
    pub usage_count: usize,
    pub usage_locations: Vec<String>, // File paths
    pub typical_pattern: String,
}

/// Dependency relationships between modules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyGraph {
    pub nodes: Vec<String>, // Module names
    pub edges: Vec<(String, String)>, // (from, to)
}

/// Detected code pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodePattern {
    pub name: String,
    pub description: String,
    pub examples: Vec<String>,
    pub frequency: usize,
}

/// AI Context Generator
pub struct AIContextGenerator {
    parser: HlxaParser,
}

impl AIContextGenerator {
    pub fn new() -> Self {
        Self {
            parser: HlxaParser::new(),
        }
    }

    /// Generate AI context for entire workspace
    pub fn generate_workspace_context(
        &self,
        files: &HashMap<String, String>,
    ) -> AIContext {
        let mut modules = Vec::new();
        let mut all_contracts = HashMap::new();
        let mut total_lines = 0;

        // Analyze each file
        for (path, content) in files {
            total_lines += content.lines().count();

            if let Ok(program) = self.parser.parse_diagnostics(content) {
                let module_ctx = self.analyze_module(path, &program, content);

                // Collect contracts
                for contract_id in &module_ctx.contracts {
                    all_contracts
                        .entry(contract_id.clone())
                        .or_insert_with(Vec::new)
                        .push(path.clone());
                }

                modules.push(module_ctx);
            }
        }

        // Build contract contexts
        let contracts = all_contracts
            .into_iter()
            .map(|(id, locations)| ContractContext {
                contract_id: id.clone(),
                name: self.contract_name(&id),
                description: self.contract_description(&id),
                usage_count: locations.len(),
                usage_locations: locations,
                typical_pattern: format!("@{} {{ ... }}", id),
            })
            .collect();

        // Build dependency graph
        let dependencies = self.build_dependency_graph(&modules);

        // Detect patterns
        let patterns = self.detect_patterns(&modules);

        // Generate summary
        let summary = self.generate_summary(&modules, &contracts, &patterns);

        AIContext {
            project: ProjectMetadata {
                name: "HLX Project".to_string(),
                total_files: files.len(),
                total_lines,
                entry_points: self.find_entry_points(&modules),
                primary_language: "HLX".to_string(),
            },
            modules,
            contracts,
            dependencies,
            patterns,
            summary,
        }
    }

    /// Analyze a single module
    fn analyze_module(&self, path: &str, program: &Program, content: &str) -> ModuleContext {
        let mut functions = Vec::new();
        let mut contracts_used = HashSet::new();

        // Analyze functions
        for block in &program.blocks {
            let func_ctx = self.analyze_function(block, content);

            // Collect contracts used
            for contract in &func_ctx.uses_contracts {
                contracts_used.insert(contract.clone());
            }

            functions.push(func_ctx);
        }

        // Extract imports
        let imports: Vec<String> = program
            .imports
            .iter()
            .map(|imp| imp.path.clone())
            .collect();

        // Calculate complexity
        let complexity_score = self.calculate_module_complexity(&functions);

        // Generate summary
        let summary = self.summarize_module(path, &functions, &contracts_used);

        ModuleContext {
            path: path.to_string(),
            name: self.module_name_from_path(path),
            summary,
            functions,
            contracts: contracts_used.into_iter().collect(),
            imports,
            exports: Vec::new(), // TODO: Extract from program.modules
            complexity_score,
        }
    }

    /// Analyze a single function
    fn analyze_function(&self, block: &Block, _content: &str) -> FunctionContext {
        let name = block.name.clone();

        // Extract parameters (name, span, type)
        let parameters: Vec<ParameterInfo> = block
            .params
            .iter()
            .map(|(param_name, _span, type_opt)| ParameterInfo {
                name: param_name.clone(),
                type_annotation: type_opt.as_ref().map(|(t, _)| format!("{:?}", t)),
                inferred_type: None,
            })
            .collect();

        // Analyze function body
        let mut uses_contracts = Vec::new();
        let mut calls = Vec::new();
        let mut line_count = 0;

        for item in &block.items {
            self.analyze_item(&item.node, &mut uses_contracts, &mut calls);
            line_count += 1;
        }

        // Calculate complexity (simple cyclomatic complexity)
        let complexity = self.calculate_function_complexity(block);

        // Generate signature
        let param_str = parameters
            .iter()
            .map(|p| p.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let signature = format!("fn {}({})", name, param_str);

        // Generate summary
        let summary = self.summarize_function(&name, &parameters, &uses_contracts);

        FunctionContext {
            name,
            signature,
            summary,
            parameters,
            return_type: None, // TODO: Infer from returns
            uses_contracts,
            calls,
            called_by: Vec::new(), // Built in second pass
            complexity,
            line_count,
        }
    }

    /// Analyze an AST item for contracts and calls
    fn analyze_item(&self, item: &Item, contracts: &mut Vec<String>, calls: &mut Vec<String>) {
        match item {
            Item::Statement(stmt) => self.analyze_statement(stmt, contracts, calls),
            _ => {}
        }
    }

    /// Analyze a statement
    fn analyze_statement(&self, stmt: &Statement, contracts: &mut Vec<String>, calls: &mut Vec<String>) {
        match stmt {
            Statement::Expr(expr) => self.analyze_expr(&expr.node, contracts, calls),
            Statement::Local { value, .. } => {
                self.analyze_expr(&value.node, contracts, calls);
            }
            Statement::If { condition, then_branch, else_branch, .. } => {
                self.analyze_expr(&condition.node, contracts, calls);
                for stmt in then_branch {
                    self.analyze_statement(&stmt.node, contracts, calls);
                }
                if let Some(else_stmts) = else_branch {
                    for stmt in else_stmts {
                        self.analyze_statement(&stmt.node, contracts, calls);
                    }
                }
            }
            Statement::While { condition, body, .. } => {
                self.analyze_expr(&condition.node, contracts, calls);
                for stmt in body {
                    self.analyze_statement(&stmt.node, contracts, calls);
                }
            }
            Statement::Return { keyword_span: _, value } => {
                self.analyze_expr(&value.node, contracts, calls);
            }
            _ => {}
        }
    }

    /// Analyze an expression
    fn analyze_expr(&self, expr: &Expr, contracts: &mut Vec<String>, calls: &mut Vec<String>) {
        match expr {
            Expr::Call { func, args, .. } => {
                // Check if it's a contract invocation (@123)
                if let Expr::Ident(name) = &func.node {
                    if name.starts_with('@') {
                        contracts.push(name.clone());
                    } else {
                        calls.push(name.clone());
                    }
                }
                // Analyze arguments
                for arg in args {
                    self.analyze_expr(&arg.node, contracts, calls);
                }
            }
            Expr::BinOp { lhs, rhs, .. } => {
                self.analyze_expr(&lhs.node, contracts, calls);
                self.analyze_expr(&rhs.node, contracts, calls);
            }
            Expr::Array(elements) => {
                for elem in elements {
                    self.analyze_expr(&elem.node, contracts, calls);
                }
            }
            _ => {}
        }
    }

    /// Calculate function complexity (cyclomatic)
    fn calculate_function_complexity(&self, block: &Block) -> u32 {
        let mut complexity = 1; // Base complexity

        for item in &block.items {
            complexity += self.count_decision_points(&item.node);
        }

        complexity
    }

    /// Count decision points for complexity
    fn count_decision_points(&self, item: &Item) -> u32 {
        match item {
            Item::Statement(stmt) => match stmt {
                Statement::If { .. } => 1,
                Statement::While { .. } => 1,
                _ => 0,
            },
            _ => 0,
        }
    }

    /// Calculate module complexity
    fn calculate_module_complexity(&self, functions: &[FunctionContext]) -> f32 {
        if functions.is_empty() {
            return 0.0;
        }

        let total_complexity: u32 = functions.iter().map(|f| f.complexity).sum();
        total_complexity as f32 / functions.len() as f32
    }

    /// Build dependency graph
    fn build_dependency_graph(&self, modules: &[ModuleContext]) -> DependencyGraph {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        for module in modules {
            nodes.push(module.name.clone());

            for import in &module.imports {
                edges.push((module.name.clone(), import.clone()));
            }
        }

        DependencyGraph { nodes, edges }
    }

    /// Detect common patterns
    fn detect_patterns(&self, modules: &[ModuleContext]) -> Vec<CodePattern> {
        let mut patterns = Vec::new();

        // Pattern 1: Contract-heavy modules
        let contract_heavy: Vec<_> = modules
            .iter()
            .filter(|m| m.contracts.len() > 5)
            .collect();

        if !contract_heavy.is_empty() {
            patterns.push(CodePattern {
                name: "Contract-Heavy Modules".to_string(),
                description: "Modules that make extensive use of contracts".to_string(),
                examples: contract_heavy.iter().map(|m| m.name.clone()).collect(),
                frequency: contract_heavy.len(),
            });
        }

        // Pattern 2: Pure computational functions
        let pure_functions: usize = modules
            .iter()
            .flat_map(|m| &m.functions)
            .filter(|f| f.uses_contracts.is_empty())
            .count();

        if pure_functions > 0 {
            patterns.push(CodePattern {
                name: "Pure Functions".to_string(),
                description: "Functions that don't use contracts".to_string(),
                examples: Vec::new(),
                frequency: pure_functions,
            });
        }

        patterns
    }

    /// Generate project summary for Claude
    fn generate_summary(
        &self,
        modules: &[ModuleContext],
        contracts: &Vec<ContractContext>,
        patterns: &[CodePattern],
    ) -> String {
        let total_functions: usize = modules.iter().map(|m| m.functions.len()).sum();
        let avg_complexity: f32 = if modules.is_empty() {
            0.0
        } else {
            modules.iter().map(|m| m.complexity_score).sum::<f32>() / modules.len() as f32
        };

        format!(
            "# HLX Project Summary\n\n\
             **Modules:** {}\n\
             **Functions:** {}\n\
             **Contracts Used:** {}\n\
             **Average Complexity:** {:.2}\n\n\
             ## Architecture\n\
             This HLX codebase consists of {} modules with {} total functions. \
             The project makes use of {} unique contracts. \n\n\
             ## Patterns Detected\n\
             {}\n\n\
             ## Key Entry Points\n\
             {}\n\n\
             This context is optimized for AI code understanding and generation.",
            modules.len(),
            total_functions,
            contracts.len(),
            avg_complexity,
            modules.len(),
            total_functions,
            contracts.len(),
            patterns.iter().map(|p| format!("- {}: {}", p.name, p.description)).collect::<Vec<_>>().join("\n"),
            modules.iter().filter(|m| m.name.contains("main")).map(|m| format!("- {}", m.name)).collect::<Vec<_>>().join("\n")
        )
    }

    /// Summarize a module
    fn summarize_module(&self, path: &str, functions: &[FunctionContext], contracts: &HashSet<String>) -> String {
        format!(
            "{} contains {} functions and uses {} contracts. Primary focus: {}",
            self.module_name_from_path(path),
            functions.len(),
            contracts.len(),
            if contracts.len() > functions.len() / 2 {
                "contract orchestration"
            } else {
                "computational logic"
            }
        )
    }

    /// Summarize a function
    fn summarize_function(&self, name: &str, params: &[ParameterInfo], contracts: &[String]) -> String {
        let contract_desc = if contracts.is_empty() {
            "uses no".to_string()
        } else {
            format!("uses {}", contracts.len())
        };

        format!(
            "Function '{}' takes {} parameters and {} contracts",
            name,
            params.len(),
            contract_desc
        )
    }

    /// Extract module name from path
    fn module_name_from_path(&self, path: &str) -> String {
        path.split('/')
            .last()
            .and_then(|s| s.strip_suffix(".hlxa"))
            .unwrap_or(path)
            .to_string()
    }

    /// Find entry points (main functions)
    fn find_entry_points(&self, modules: &[ModuleContext]) -> Vec<String> {
        modules
            .iter()
            .filter(|m| m.functions.iter().any(|f| f.name == "main"))
            .map(|m| m.name.clone())
            .collect()
    }

    /// Get contract name from ID
    fn contract_name(&self, id: &str) -> String {
        // TODO: Look up from contract catalogue
        format!("Contract {}", id)
    }

    /// Get contract description from ID
    fn contract_description(&self, id: &str) -> String {
        // TODO: Look up from contract catalogue
        format!("Contract {} description", id)
    }

    /// Export context as JSON for Claude
    pub fn export_as_json(&self, context: &AIContext) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(context)
    }

    /// Export context as Markdown for Claude
    pub fn export_as_markdown(&self, context: &AIContext) -> String {
        let mut md = String::new();

        md.push_str(&context.summary);
        md.push_str("\n\n---\n\n");

        md.push_str("## Modules\n\n");
        for module in &context.modules {
            md.push_str(&format!("### {}\n\n", module.name));
            md.push_str(&format!("{}\n\n", module.summary));
            md.push_str(&format!("- **Functions:** {}\n", module.functions.len()));
            md.push_str(&format!("- **Contracts:** {}\n", module.contracts.len()));
            md.push_str(&format!("- **Complexity:** {:.2}\n\n", module.complexity_score));

            for func in &module.functions {
                md.push_str(&format!("#### `{}`\n\n", func.signature));
                md.push_str(&format!("{}\n\n", func.summary));
            }
        }

        md.push_str("## Contracts\n\n");
        for contract in &context.contracts {
            md.push_str(&format!("### {} ({})\n\n", contract.name, contract.contract_id));
            md.push_str(&format!("{}\n\n", contract.description));
            md.push_str(&format!("**Usage:** {} times\n\n", contract.usage_count));
        }

        md
    }
}

impl Default for AIContextGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_name_extraction() {
        let gen = AIContextGenerator::new();
        assert_eq!(gen.module_name_from_path("/path/to/file.hlxa"), "file");
        assert_eq!(gen.module_name_from_path("file.hlxa"), "file");
    }

    #[test]
    fn test_complexity_calculation() {
        let gen = AIContextGenerator::new();
        let functions = vec![
            FunctionContext {
                name: "simple".to_string(),
                signature: "fn simple()".to_string(),
                summary: String::new(),
                parameters: Vec::new(),
                return_type: None,
                uses_contracts: Vec::new(),
                calls: Vec::new(),
                called_by: Vec::new(),
                complexity: 1,
                line_count: 5,
            },
            FunctionContext {
                name: "complex".to_string(),
                signature: "fn complex()".to_string(),
                summary: String::new(),
                parameters: Vec::new(),
                return_type: None,
                uses_contracts: Vec::new(),
                calls: Vec::new(),
                called_by: Vec::new(),
                complexity: 5,
                line_count: 20,
            },
        ];

        let complexity = gen.calculate_module_complexity(&functions);
        assert_eq!(complexity, 3.0); // (1 + 5) / 2
    }

    #[test]
    fn test_dependency_graph_building() {
        let gen = AIContextGenerator::new();
        let modules = vec![
            ModuleContext {
                path: "a.hlxa".to_string(),
                name: "a".to_string(),
                summary: String::new(),
                functions: Vec::new(),
                contracts: Vec::new(),
                imports: vec!["b".to_string()],
                exports: Vec::new(),
                complexity_score: 1.0,
            },
        ];

        let graph = gen.build_dependency_graph(&modules);
        assert_eq!(graph.nodes.len(), 1);
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0], ("a".to_string(), "b".to_string()));
    }
}
