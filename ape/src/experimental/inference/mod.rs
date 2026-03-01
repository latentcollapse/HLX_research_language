//! Inference Layer — Three modes, same safety, different verbosity
//!
//! All modes compile to identical Arx-equivalent. The inference engine
//! is syntactic expansion only. `do` is ALWAYS explicit (Bright Line Rule).

use crate::parser::ast::*;
use crate::trust::TrustLevel;

/// The three inference modes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InferenceMode {
    /// "Just think" — maximum inference, minimum verbosity
    Flow,
    /// "Trust is explicit" — DEFAULT mode, trust tags required, rest inferred
    Guard,
    /// "Every annotation, explicit" — nothing inferred
    Arx,
}

impl InferenceMode {
    pub fn from_pragma(s: &str) -> Option<Self> {
        match s {
            "flow" => Some(InferenceMode::Flow),
            "guard" => Some(InferenceMode::Guard),
            "arx" => Some(InferenceMode::Arx),
            // Backward compatibility (deprecated — use guard/arx instead)
            "shield" => Some(InferenceMode::Guard),
            "fortress" => Some(InferenceMode::Arx),
            _ => None,
        }
    }
}

/// What gets inferred in each mode
#[derive(Debug, Clone)]
pub struct InferredAnnotations {
    /// Trust tag for the value
    pub trust_tag: Option<TrustLevel>,
    /// Tensor shape (partial unification with wildcard propagation)
    pub tensor_shape: Option<Vec<TensorDim>>,
    /// Resource bounds from intent declarations
    pub resource_bounds: Option<Vec<String>>,
    /// Conscience constraints (always inherited, never guessed)
    pub conscience_constraints: Vec<String>,
}

/// The inference engine expands lower-mode code to Arx-equivalent
pub struct InferenceEngine {
    pub current_mode: InferenceMode,
}

impl InferenceEngine {
    pub fn new() -> Self {
        InferenceEngine {
            current_mode: InferenceMode::Guard, // Default: trust-explicit
        }
    }

    /// Expand a program to Arx-equivalent
    /// All modes produce identical compiled output — this is syntactic expansion only
    pub fn expand_to_arx(&self, program: &mut Program) {
        self.expand_module(&mut program.module);
    }

    /// Backward-compatible alias
    pub fn expand_to_fortress(&self, program: &mut Program) {
        self.expand_to_arx(program);
    }

    fn expand_module(&self, module: &mut Module) {
        for item in &mut module.items {
            match item {
                Item::Function(f) => self.expand_function(f),
                _ => {}
            }
        }
    }

    fn expand_function(&self, func: &mut FunctionDecl) {
        // In Arx mode, nothing to infer — everything must be explicit
        // In other modes, we fill in missing annotations
        match self.current_mode {
            InferenceMode::Flow => {
                // Infer everything: return types, trust tags, tensor shapes
                self.infer_block_trust(&mut func.body);
            }
            InferenceMode::Guard => {
                // Trust tags must be explicit, but shapes/bounds inferred
                self.infer_block_shapes(&mut func.body);
            }
            InferenceMode::Arx => {
                // Nothing to infer — validate that everything is explicit
            }
        }
    }

    fn infer_block_trust(&self, _block: &mut Block) {
        // Walk the block and annotate each expression with inferred trust tags
        // Trust inference follows the algebra: trust(output) = max(trust(inputs))
        // This is a placeholder — full implementation would annotate each AST node
    }

    fn infer_block_shapes(&self, _block: &mut Block) {
        // Walk the block and infer tensor shapes via partial unification
        // Shape inference propagates wildcards: Tensor[?, 768] stays wildcard on dim 0
    }

    /// Validate that all annotations are present (Arx mode check)
    pub fn validate_arx(&self, program: &Program) -> Vec<String> {
        let mut errors = Vec::new();
        for item in &program.module.items {
            if let Item::Function(f) = item {
                // In Arx mode, all let bindings must have explicit type annotations
                self.check_block_annotations(&f.body, &f.name, &mut errors);
            }
        }
        errors
    }

    /// Backward-compatible alias
    pub fn validate_fortress(&self, program: &Program) -> Vec<String> {
        self.validate_arx(program)
    }

    fn check_block_annotations(&self, block: &Block, func_name: &str, errors: &mut Vec<String>) {
        for stmt in &block.stmts {
            match stmt {
                Stmt::Let(let_stmt) => {
                    if self.current_mode == InferenceMode::Arx && let_stmt.ty.is_none() {
                        errors.push(format!(
                            "Arx mode: let '{}' in fn '{}' must have explicit type annotation",
                            let_stmt.name, func_name
                        ));
                    }
                }
                Stmt::If(if_stmt) => {
                    self.check_block_annotations(&if_stmt.then_block, func_name, errors);
                    if let Some(ref else_block) = if_stmt.else_block {
                        self.check_block_annotations(else_block, func_name, errors);
                    }
                }
                Stmt::Loop(loop_stmt) => {
                    self.check_block_annotations(&loop_stmt.body, func_name, errors);
                }
                _ => {}
            }
        }
    }

    /// Infer trust tag for a `do` expression result
    pub fn infer_do_trust(&self, intent_name: &str) -> TrustLevel {
        // All `do` results are UNTRUSTED_EXTERNAL — they cross the system boundary
        // Only `do Verify` promotes trust (P18)
        if intent_name == "Verify" {
            TrustLevel::TrustedVerified
        } else {
            TrustLevel::UntrustedExternal
        }
    }

    /// Check the Bright Line Rule: `do` must never be inferred
    pub fn check_bright_line(&self, program: &Program) -> Vec<String> {
        let mut violations = Vec::new();
        // The parser already enforces `do` is explicit syntax,
        // but we double-check that no inference path could generate a `do`
        for item in &program.module.items {
            if let Item::Function(f) = item {
                self.check_block_no_implicit_do(&f.body, &f.name, &mut violations);
            }
        }
        violations
    }

    fn check_block_no_implicit_do(
        &self,
        block: &Block,
        func_name: &str,
        violations: &mut Vec<String>,
    ) {
        for stmt in &block.stmts {
            match stmt {
                Stmt::Let(let_stmt) => {
                    self.check_expr_no_implicit_do(&let_stmt.value, func_name, violations);
                }
                Stmt::Return(ret) => {
                    if let Some(val) = &ret.value {
                        self.check_expr_no_implicit_do(val, func_name, violations);
                    }
                }
                Stmt::If(if_stmt) => {
                    self.check_expr_no_implicit_do(&if_stmt.condition, func_name, violations);
                    self.check_block_no_implicit_do(&if_stmt.then_block, func_name, violations);
                    if let Some(ref else_block) = if_stmt.else_block {
                        self.check_block_no_implicit_do(else_block, func_name, violations);
                    }
                }
                Stmt::Loop(loop_stmt) => {
                    self.check_block_no_implicit_do(&loop_stmt.body, func_name, violations);
                }
                Stmt::Expr(expr_stmt) => {
                    self.check_expr_no_implicit_do(&expr_stmt.expr, func_name, violations);
                }
                _ => {}
            }
        }
    }

    fn check_expr_no_implicit_do(
        &self,
        _expr: &Expr,
        _func_name: &str,
        _violations: &mut Vec<String>,
    ) {
        // `do` is syntactically explicit — enforced by the parser.
        // This method exists for completeness: in a full implementation,
        // we'd verify that no macro or expansion could introduce an implicit `do`.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_mode_is_guard() {
        let engine = InferenceEngine::new();
        assert_eq!(engine.current_mode, InferenceMode::Guard);
    }

    #[test]
    fn test_do_trust_level() {
        let engine = InferenceEngine::new();
        assert_eq!(
            engine.infer_do_trust("ReadFile"),
            TrustLevel::UntrustedExternal
        );
        assert_eq!(
            engine.infer_do_trust("Verify"),
            TrustLevel::TrustedVerified
        );
    }

    #[test]
    fn test_mode_from_pragma() {
        assert_eq!(
            InferenceMode::from_pragma("flow"),
            Some(InferenceMode::Flow)
        );
        assert_eq!(
            InferenceMode::from_pragma("guard"),
            Some(InferenceMode::Guard)
        );
        assert_eq!(
            InferenceMode::from_pragma("arx"),
            Some(InferenceMode::Arx)
        );
        assert_eq!(InferenceMode::from_pragma("invalid"), None);
    }

    #[test]
    fn test_backward_compat_pragmas() {
        // Old pragmas map to new modes
        assert_eq!(
            InferenceMode::from_pragma("shield"),
            Some(InferenceMode::Guard)
        );
        assert_eq!(
            InferenceMode::from_pragma("fortress"),
            Some(InferenceMode::Arx)
        );
    }
}
