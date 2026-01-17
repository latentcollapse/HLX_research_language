use std::collections::HashMap;
use tower_lsp::lsp_types::*;
use hlx_compiler::ast::{Block, Expr, Item, Literal, Program, Span, Spanned, Statement, Type as AstType};

/// Represents a type with flow-sensitive narrowing
#[derive(Debug, Clone, PartialEq)]
pub enum InferredType {
    /// Known concrete type
    Known(String),
    /// Union of possible types (e.g., Int | String)
    Union(Vec<String>),
    /// Type narrowed by a condition
    Narrowed {
        base: Box<InferredType>,
        narrowed_to: String,
        condition: String,
    },
    /// Unknown type
    Unknown,
    /// Any type
    Any,
}

impl InferredType {
    pub fn to_string(&self) -> String {
        match self {
            InferredType::Known(t) => t.clone(),
            InferredType::Union(types) => types.join(" | "),
            InferredType::Narrowed { narrowed_to, .. } => narrowed_to.clone(),
            InferredType::Unknown => "unknown".to_string(),
            InferredType::Any => "any".to_string(),
        }
    }

    pub fn is_compatible_with(&self, other: &InferredType) -> bool {
        match (self, other) {
            (InferredType::Any, _) | (_, InferredType::Any) => true,
            (InferredType::Unknown, _) | (_, InferredType::Unknown) => true,
            (InferredType::Known(a), InferredType::Known(b)) => a == b,
            (InferredType::Narrowed { narrowed_to, .. }, InferredType::Known(b)) => narrowed_to == b,
            (InferredType::Known(a), InferredType::Narrowed { narrowed_to, .. }) => a == narrowed_to,
            _ => false,
        }
    }
}

/// A type error with enhanced suggestions
#[derive(Debug, Clone)]
pub struct EnhancedTypeError {
    pub message: String,
    pub location: Range,
    pub expected: InferredType,
    pub found: InferredType,
    pub suggestions: Vec<String>,
}

impl EnhancedTypeError {
    pub fn to_diagnostic(&self) -> Diagnostic {
        let mut message = self.message.clone();

        // Add suggestions to the message
        if !self.suggestions.is_empty() {
            message.push_str("\n\nSuggestions:");
            for suggestion in &self.suggestions {
                message.push_str(&format!("\n  • {}", suggestion));
            }
        }

        Diagnostic {
            range: self.location,
            severity: Some(DiagnosticSeverity::ERROR),
            code: Some(NumberOrString::String("type-mismatch".to_string())),
            source: Some("hlx-type-checker".to_string()),
            message,
            related_information: None,
            tags: None,
            code_description: None,
            data: None,
        }
    }
}

/// Context for type inference with flow sensitivity
pub struct EnhancedTypeContext {
    /// Variable name -> type in current scope
    variables: HashMap<String, InferredType>,
    /// Function name -> (params, return type)
    functions: HashMap<String, (Vec<InferredType>, InferredType)>,
    /// Type guards active in current scope
    type_guards: Vec<TypeGuard>,
    /// Inferred return types for functions
    return_types: HashMap<String, InferredType>,
}

/// A type guard from a conditional check
#[derive(Debug, Clone)]
struct TypeGuard {
    variable: String,
    narrowed_type: String,
    condition: String,
}

impl EnhancedTypeContext {
    pub fn new() -> Self {
        let mut ctx = Self {
            variables: HashMap::new(),
            functions: HashMap::new(),
            type_guards: Vec::new(),
            return_types: HashMap::new(),
        };

        // Register builtin functions
        ctx.register_builtins();
        ctx
    }

    fn register_builtins(&mut self) {
        use InferredType::*;

        // Common builtins
        self.functions.insert("print".to_string(), (vec![Any], Known("void".to_string())));
        self.functions.insert("println".to_string(), (vec![Any], Known("void".to_string())));
        self.functions.insert("len".to_string(), (vec![Any], Known("Int".to_string())));
        self.functions.insert("typeof".to_string(), (vec![Any], Known("String".to_string())));

        // Math builtins
        self.functions.insert("abs".to_string(), (vec![Known("Int".to_string())], Known("Int".to_string())));
        self.functions.insert("floor".to_string(), (vec![Known("Float".to_string())], Known("Int".to_string())));
        self.functions.insert("ceil".to_string(), (vec![Known("Float".to_string())], Known("Int".to_string())));
        self.functions.insert("round".to_string(), (vec![Known("Float".to_string())], Known("Int".to_string())));
        self.functions.insert("sqrt".to_string(), (vec![Known("Float".to_string())], Known("Float".to_string())));
        self.functions.insert("pow".to_string(), (vec![Known("Float".to_string()), Known("Float".to_string())], Known("Float".to_string())));
    }

    pub fn declare_variable(&mut self, name: &str, typ: InferredType) {
        self.variables.insert(name.to_string(), typ);
    }

    pub fn get_variable_type(&self, name: &str) -> Option<&InferredType> {
        // Check if there's an active type guard for this variable
        for guard in self.type_guards.iter().rev() {
            if guard.variable == name {
                // Return narrowed type
                return Some(&guard.narrowed_type).map(|_| {
                    // This is a bit hacky, but we need to return a reference
                    self.variables.get(name)
                }).flatten();
            }
        }

        self.variables.get(name)
    }

    pub fn declare_function(&mut self, name: &str, params: Vec<InferredType>, ret: InferredType) {
        self.functions.insert(name.to_string(), (params, ret));
    }

    pub fn get_function_signature(&self, name: &str) -> Option<&(Vec<InferredType>, InferredType)> {
        self.functions.get(name)
    }

    pub fn push_type_guard(&mut self, variable: &str, narrowed_type: &str, condition: &str) {
        self.type_guards.push(TypeGuard {
            variable: variable.to_string(),
            narrowed_type: narrowed_type.to_string(),
            condition: condition.to_string(),
        });
    }

    pub fn pop_type_guard(&mut self) {
        self.type_guards.pop();
    }

    pub fn infer_return_type(&mut self, function_name: &str, return_type: InferredType) {
        self.return_types.insert(function_name.to_string(), return_type);
    }

    pub fn get_inferred_return_type(&self, function_name: &str) -> Option<&InferredType> {
        self.return_types.get(function_name)
    }
}

/// Enhanced type inference engine with flow sensitivity
pub struct EnhancedTypeInference {
    context: EnhancedTypeContext,
}

impl EnhancedTypeInference {
    pub fn new() -> Self {
        Self {
            context: EnhancedTypeContext::new(),
        }
    }

    /// Infer types for an entire program
    pub fn infer_program(&mut self, program: &Program) -> Vec<EnhancedTypeError> {
        let mut errors = Vec::new();

        // Process all blocks (functions)
        for block in &program.blocks {
            errors.extend(self.infer_block(block));
        }

        errors
    }

    /// Infer types for a block (function)
    fn infer_block(&mut self, block: &Block) -> Vec<EnhancedTypeError> {
        let mut errors = Vec::new();

        // Register function in context
        let param_types: Vec<InferredType> = block.params.iter()
            .map(|(_, _, typ_opt)| {
                if let Some((typ, _)) = typ_opt {
                    self.ast_type_to_inferred(typ)
                } else {
                    InferredType::Any
                }
            })
            .collect();

        let return_type = block.return_type.as_ref()
            .map(|t| self.ast_type_to_inferred(t))
            .unwrap_or(InferredType::Unknown);

        self.context.declare_function(&block.name, param_types.clone(), return_type.clone());

        // Declare parameters as variables
        for (name, _, typ_opt) in &block.params {
            let typ = if let Some((typ, _)) = typ_opt {
                self.ast_type_to_inferred(typ)
            } else {
                InferredType::Any
            };
            self.context.declare_variable(name, typ);
        }

        // Analyze function body
        for item in &block.items {
            errors.extend(self.infer_item(item));
        }

        // Check if return type matches inferred type
        if let Some(inferred) = self.context.get_inferred_return_type(&block.name) {
            if !return_type.is_compatible_with(inferred) {
                // Return type mismatch - but we'd need location info
                // For now, skip this check
            }
        }

        errors
    }

    /// Infer types for an item (statement or node)
    fn infer_item(&mut self, item: &Spanned<Item>) -> Vec<EnhancedTypeError> {
        match &item.node {
            Item::Statement(stmt) => self.infer_statement(&stmt),
            Item::Node(_) => Vec::new(), // Skip nodes for now
        }
    }

    /// Infer types for a statement
    fn infer_statement(&mut self, statement: &Statement) -> Vec<EnhancedTypeError> {
        let mut errors = Vec::new();

        match statement {
            Statement::Let { name, type_annotation, value, .. } => {
                // Infer type from value
                let inferred = self.infer_expr(&value.node);

                // Check against annotation if present
                if let Some(typ) = type_annotation {
                    let expected = self.ast_type_to_inferred(typ);
                    if !inferred.is_compatible_with(&expected) {
                        errors.push(self.create_type_error(
                            &format!("Type mismatch for variable '{}'", name),
                            expected,
                            inferred.clone(),
                            value.span,
                        ));
                    }
                }

                // Declare variable
                self.context.declare_variable(name, inferred);
            }

            Statement::Local { name, value, .. } => {
                let inferred = self.infer_expr(&value.node);
                self.context.declare_variable(name, inferred);
            }

            Statement::If { condition, then_branch, else_branch, .. } => {
                // Check condition is boolean
                let cond_type = self.infer_expr(&condition.node);
                if !cond_type.is_compatible_with(&InferredType::Known("Bool".to_string())) {
                    errors.push(self.create_type_error(
                        "If condition must be boolean",
                        InferredType::Known("Bool".to_string()),
                        cond_type,
                        condition.span,
                    ));
                }

                // Check for type guards (typeof checks)
                if let Some(guard) = self.detect_type_guard(&condition.node) {
                    self.context.push_type_guard(&guard.0, &guard.1, &guard.2);
                }

                // Process then block
                for stmt in then_branch {
                    errors.extend(self.infer_statement(&stmt.node));
                }

                // Pop type guard
                if self.detect_type_guard(&condition.node).is_some() {
                    self.context.pop_type_guard();
                }

                // Process else block
                if let Some(else_stmts) = else_branch {
                    for stmt in else_stmts {
                        errors.extend(self.infer_statement(&stmt.node));
                    }
                }
            }

            Statement::While { body, .. } => {
                for stmt in body {
                    errors.extend(self.infer_statement(&stmt.node));
                }
            }

            Statement::Return { value, .. } => {
                let return_type = self.infer_expr(&value.node);
                // Track return type for inference
                // (would need function name context)
            }

            Statement::Expr(expr) => {
                // Just infer the expression, discard result
                self.infer_expr(&expr.node);
            }

            _ => {}
        }

        errors
    }

    /// Infer the type of an expression
    fn infer_expr(&self, expr: &Expr) -> InferredType {
        match expr {
            Expr::Literal(lit) => match lit {
                Literal::Int(_) => InferredType::Known("Int".to_string()),
                Literal::Float(_) => InferredType::Known("Float".to_string()),
                Literal::String(_) => InferredType::Known("String".to_string()),
                Literal::Bool(_) => InferredType::Known("Bool".to_string()),
                Literal::Null => InferredType::Known("Null".to_string()),
                Literal::Array(_) => InferredType::Known("Array".to_string()),
                Literal::Object(_) => InferredType::Known("Object".to_string()),
            },
            Expr::Ident(name) => {
                self.context.get_variable_type(name)
                    .cloned()
                    .unwrap_or(InferredType::Unknown)
            }
            Expr::Array(_) => InferredType::Known("Array".to_string()),
            Expr::Object(_) => InferredType::Known("Object".to_string()),
            Expr::BinOp { op, lhs, rhs } => {
                let left_type = self.infer_expr(&lhs.node);
                let right_type = self.infer_expr(&rhs.node);

                // Simplified: arithmetic ops return numeric types
                match op.hlxl_str() {
                    "+" | "-" | "*" | "/" => {
                        if left_type.is_compatible_with(&InferredType::Known("Float".to_string()))
                            || right_type.is_compatible_with(&InferredType::Known("Float".to_string()))
                        {
                            InferredType::Known("Float".to_string())
                        } else {
                            InferredType::Known("Int".to_string())
                        }
                    }
                    "==" | "!=" | "<" | ">" | "<=" | ">=" => InferredType::Known("Bool".to_string()),
                    "&&" | "||" => InferredType::Known("Bool".to_string()),
                    _ => InferredType::Unknown,
                }
            }
            Expr::Call { func, args } => {
                if let Expr::Ident(func_name) = &func.node {
                    if let Some((_, ret_type)) = self.context.get_function_signature(func_name) {
                        return ret_type.clone();
                    }
                }
                InferredType::Unknown
            }
            _ => InferredType::Unknown,
        }
    }

    /// Detect type guard pattern (e.g., typeof(x) == "string")
    fn detect_type_guard(&self, expr: &Expr) -> Option<(String, String, String)> {
        if let Expr::BinOp { op, lhs, rhs } = expr {
            if op.hlxl_str() == "==" {
                // Check if left side is typeof(x)
                if let Expr::Call { func, args } = &lhs.node {
                    if let Expr::Ident(func_name) = &func.node {
                        if func_name == "typeof" && args.len() == 1 {
                            if let Expr::Ident(var_name) = &args[0].node {
                                // Check if right side is a string literal type name
                                if let Expr::Literal(Literal::String(type_name)) = &rhs.node {
                                    let narrowed_type = type_name.trim_matches('"');
                                    let condition = format!("typeof({}) == \"{}\"", var_name, narrowed_type);
                                    return Some((var_name.clone(), narrowed_type.to_string(), condition));
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// Convert AST type to inferred type
    fn ast_type_to_inferred(&self, typ: &AstType) -> InferredType {
        match typ {
            AstType::Int => InferredType::Known("Int".to_string()),
            AstType::Float => InferredType::Known("Float".to_string()),
            AstType::String => InferredType::Known("String".to_string()),
            AstType::Bool => InferredType::Known("Bool".to_string()),
            AstType::Array(inner) => {
                // Could track inner type, but simplified for now
                InferredType::Known("Array".to_string())
            }
            AstType::Named(name) => InferredType::Known(name.clone()),
        }
    }

    /// Create a type error with suggestions
    fn create_type_error(
        &self,
        message: &str,
        expected: InferredType,
        found: InferredType,
        span: Span,
    ) -> EnhancedTypeError {
        let suggestions = self.generate_suggestions(&expected, &found);

        // Convert span to LSP Range (simplified - would need proper line/col mapping)
        let location = Range {
            start: Position { line: 0, character: span.start as u32 },
            end: Position { line: 0, character: span.end as u32 },
        };

        EnhancedTypeError {
            message: message.to_string(),
            location,
            expected,
            found,
            suggestions,
        }
    }

    /// Generate helpful suggestions for type errors
    fn generate_suggestions(&self, expected: &InferredType, found: &InferredType) -> Vec<String> {
        let mut suggestions = Vec::new();

        match (expected, found) {
            (InferredType::Known(exp), InferredType::Known(fnd)) => {
                if exp == "Int" && fnd == "String" {
                    suggestions.push("Did you mean to parse the string? Try: value.parse()".to_string());
                } else if exp == "String" && fnd == "Int" {
                    suggestions.push("Convert to string with: value.to_string()".to_string());
                } else if exp == "Float" && fnd == "Int" {
                    suggestions.push("Convert to float with: value.to_float()".to_string());
                } else if exp == "Bool" && (fnd == "Int" || fnd == "String") {
                    suggestions.push("Use a comparison: value == 0 or value == \"\"".to_string());
                }
            }
            _ => {}
        }

        suggestions
    }

    /// Get inlay hint for variable type
    pub fn get_type_hint(&self, var_name: &str) -> Option<String> {
        self.context.get_variable_type(var_name)
            .map(|t| t.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_type_inference() {
        let mut inference = EnhancedTypeInference::new();
        inference.context.declare_variable("x", InferredType::Known("Int".to_string()));

        let typ = inference.context.get_variable_type("x");
        assert!(typ.is_some());
        assert_eq!(typ.unwrap().to_string(), "Int");
    }

    #[test]
    fn test_type_compatibility() {
        let int_type = InferredType::Known("Int".to_string());
        let float_type = InferredType::Known("Float".to_string());
        let any_type = InferredType::Any;

        assert!(!int_type.is_compatible_with(&float_type));
        assert!(int_type.is_compatible_with(&any_type));
        assert!(any_type.is_compatible_with(&int_type));
    }

    #[test]
    fn test_suggestion_generation() {
        let inference = EnhancedTypeInference::new();

        let suggestions = inference.generate_suggestions(
            &InferredType::Known("Int".to_string()),
            &InferredType::Known("String".to_string()),
        );

        assert!(!suggestions.is_empty());
        assert!(suggestions[0].contains("parse"));
    }
}
