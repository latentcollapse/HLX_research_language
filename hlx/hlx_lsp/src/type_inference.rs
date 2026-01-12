//! Type Inference Engine for HLX
//!
//! Infers types from source code and checks for type errors.

use crate::type_system::{Type, TypeError, BinaryOp};
use hlx_core::{BuiltinRegistry, ReturnType as CoreReturnType, ParamType as CoreParamType};
use std::collections::HashMap;
use regex::Regex;

/// Type inference context
pub struct TypeContext {
    /// Variable name -> type
    variables: HashMap<String, Type>,
    /// Function name -> (param types, return type)
    functions: HashMap<String, (Vec<Type>, Type)>,
    /// Builtin function signatures
    builtins: HashMap<String, (Vec<Type>, Type)>,
}

impl TypeContext {
    pub fn new() -> Self {
        let mut ctx = Self {
            variables: HashMap::new(),
            functions: HashMap::new(),
            builtins: HashMap::new(),
        };

        // Load all builtins from the unified registry
        let registry = BuiltinRegistry::new();
        for sig in registry.all() {
            let params = sig.params.iter().map(|p| Self::convert_param_type(p)).collect();
            let ret = Self::convert_return_type(&sig.return_type);
            ctx.add_builtin(sig.name, params, ret);
        }

        ctx
    }

    /// Convert core ParamType to LSP Type
    fn convert_param_type(param: &CoreParamType) -> Type {
        match param {
            CoreParamType::Any => Type::Any,
            CoreParamType::Int => Type::Int,
            CoreParamType::Float => Type::Float,
            CoreParamType::String => Type::String,
            CoreParamType::Bool => Type::Bool,
            CoreParamType::Array => Type::Array(Box::new(Type::Any)),
            CoreParamType::Object => Type::Object,
            CoreParamType::Handle => Type::String, // Handles are strings at runtime
        }
    }

    /// Convert core ReturnType to LSP Type
    fn convert_return_type(ret: &CoreReturnType) -> Type {
        match ret {
            CoreReturnType::Null => Type::Null,
            CoreReturnType::Any => Type::Any,
            CoreReturnType::Int => Type::Int,
            CoreReturnType::Float => Type::Float,
            CoreReturnType::String => Type::String,
            CoreReturnType::Bool => Type::Bool,
            CoreReturnType::Array => Type::Array(Box::new(Type::Any)),
            CoreReturnType::Object => Type::Object,
            CoreReturnType::Handle => Type::String,
        }
    }

    fn add_builtin(&mut self, name: &str, params: Vec<Type>, ret: Type) {
        self.builtins.insert(name.to_string(), (params, ret));
    }

    pub fn declare_var(&mut self, name: &str, typ: Type) {
        self.variables.insert(name.to_string(), typ);
    }

    pub fn get_var_type(&self, name: &str) -> Option<&Type> {
        self.variables.get(name)
    }

    pub fn declare_function(&mut self, name: &str, params: Vec<Type>, ret: Type) {
        self.functions.insert(name.to_string(), (params, ret));
    }

    pub fn get_function_sig(&self, name: &str) -> Option<&(Vec<Type>, Type)> {
        self.builtins.get(name).or_else(|| self.functions.get(name))
    }
}

impl Default for TypeContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Type inference engine
pub struct TypeInference {
    /// Regex patterns for parsing
    int_literal: Regex,
    float_literal: Regex,
    string_literal: Regex,
    bool_literal: Regex,
    ident: Regex,
    func_call: Regex,
}

impl TypeInference {
    pub fn new() -> Self {
        Self {
            int_literal: Regex::new(r"^\d+$").unwrap(),
            float_literal: Regex::new(r"^\d+\.\d+$").unwrap(),
            string_literal: Regex::new(r#"^"[^"]*"$"#).unwrap(),
            bool_literal: Regex::new(r"^(true|false)$").unwrap(),
            ident: Regex::new(r"^[a-zA-Z_]\w*$").unwrap(),
            func_call: Regex::new(r"^(\w+)\s*\(([^)]*)\)$").unwrap(),
        }
    }

    /// Infer the type of an expression
    pub fn infer_expr(&self, expr: &str, ctx: &TypeContext) -> Result<Type, TypeError> {
        let expr = expr.trim();

        // Integer literal
        if self.int_literal.is_match(expr) {
            return Ok(Type::Int);
        }

        // Float literal
        if self.float_literal.is_match(expr) {
            return Ok(Type::Float);
        }

        // String literal
        if self.string_literal.is_match(expr) {
            return Ok(Type::String);
        }

        // Boolean literal
        if self.bool_literal.is_match(expr) {
            return Ok(Type::Bool);
        }

        // null
        if expr == "null" {
            return Ok(Type::Null);
        }

        // Function call
        if let Some(caps) = self.func_call.captures(expr) {
            let func_name = caps.get(1).unwrap().as_str();
            let args_str = caps.get(2).unwrap().as_str();

            // Parse arguments (simplified - just split by comma)
            let args: Vec<&str> = if args_str.trim().is_empty() {
                vec![]
            } else {
                args_str.split(',').map(|s| s.trim()).collect()
            };

            return self.check_call(func_name, &args, ctx);
        }

        // Binary operation (simplified - just look for operators)
        if let Some(op_pos) = expr.find(" + ") {
            let (left, right) = expr.split_at(op_pos);
            let right = &right[3..]; // Skip " + "
            let left_type = self.infer_expr(left.trim(), ctx)?;
            let right_type = self.infer_expr(right.trim(), ctx)?;
            return left_type.binary_op_result(BinaryOp::Add, &right_type);
        }

        if let Some(op_pos) = expr.find(" - ") {
            let (left, right) = expr.split_at(op_pos);
            let right = &right[3..];
            let left_type = self.infer_expr(left.trim(), ctx)?;
            let right_type = self.infer_expr(right.trim(), ctx)?;
            return left_type.binary_op_result(BinaryOp::Sub, &right_type);
        }

        if let Some(op_pos) = expr.find(" * ") {
            let (left, right) = expr.split_at(op_pos);
            let right = &right[3..];
            let left_type = self.infer_expr(left.trim(), ctx)?;
            let right_type = self.infer_expr(right.trim(), ctx)?;
            return left_type.binary_op_result(BinaryOp::Mul, &right_type);
        }

        if let Some(op_pos) = expr.find(" / ") {
            let (left, right) = expr.split_at(op_pos);
            let right = &right[3..];
            let left_type = self.infer_expr(left.trim(), ctx)?;
            let right_type = self.infer_expr(right.trim(), ctx)?;
            return left_type.binary_op_result(BinaryOp::Div, &right_type);
        }

        // Variable reference
        if self.ident.is_match(expr) {
            if let Some(typ) = ctx.get_var_type(expr) {
                return Ok(typ.clone());
            } else {
                return Err(TypeError::UndefinedVariable {
                    name: expr.to_string(),
                });
            }
        }

        // Unknown expression
        Ok(Type::Unknown)
    }

    /// Check a function call
    fn check_call(&self, func_name: &str, args: &[&str], ctx: &TypeContext) -> Result<Type, TypeError> {
        // Get function signature
        let sig = ctx.get_function_sig(func_name).ok_or_else(|| TypeError::UndefinedFunction {
            name: func_name.to_string(),
        })?;

        let (param_types, ret_type) = sig;

        // Check argument count
        if args.len() != param_types.len() {
            return Err(TypeError::WrongArgCount {
                expected: param_types.len(),
                got: args.len(),
            });
        }

        // Check argument types
        for (i, (arg, expected_type)) in args.iter().zip(param_types.iter()).enumerate() {
            let arg_type = self.infer_expr(arg, ctx)?;

            // Allow Any to accept anything
            if matches!(expected_type, Type::Any) {
                continue;
            }

            // Check compatibility
            if !arg_type.is_compatible_with(expected_type) && !matches!(arg_type, Type::Unknown) {
                return Err(TypeError::WrongArgType {
                    param_index: i,
                    expected: expected_type.clone(),
                    got: arg_type,
                });
            }
        }

        Ok(ret_type.clone())
    }

    /// Analyze a full function and return type errors
    pub fn check_function(&self, source: &str) -> Vec<TypeCheckResult> {
        let mut results = Vec::new();
        let mut ctx = TypeContext::new();

        // Parse function (simplified)
        for (line_idx, line) in source.lines().enumerate() {
            let line = line.trim();

            // Skip comments and empty lines
            if line.is_empty() || line.starts_with("//") {
                continue;
            }

            // Variable declaration: let name = value;
            if line.starts_with("let ") {
                if let Some(eq_pos) = line.find('=') {
                    let name_part = &line[4..eq_pos].trim();
                    let value_part = &line[eq_pos + 1..].trim_end_matches(';').trim();

                    // Infer type from value
                    match self.infer_expr(value_part, &ctx) {
                        Ok(typ) => {
                            ctx.declare_var(name_part, typ);
                        }
                        Err(err) => {
                            results.push(TypeCheckResult {
                                line: line_idx,
                                error: err,
                            });
                        }
                    }
                }
            }

            // Function call statement: func(args);
            if let Some(caps) = self.func_call.captures(line) {
                let func_name = caps.get(1).unwrap().as_str();
                let args_str = caps.get(2).unwrap().as_str();

                let args: Vec<&str> = if args_str.trim().is_empty() {
                    vec![]
                } else {
                    args_str.split(',').map(|s| s.trim()).collect()
                };

                if let Err(err) = self.check_call(func_name, &args, &ctx) {
                    results.push(TypeCheckResult {
                        line: line_idx,
                        error: err,
                    });
                }
            }
        }

        results
    }
}

impl Default for TypeInference {
    fn default() -> Self {
        Self::new()
    }
}

/// Type check result
#[derive(Debug, Clone)]
pub struct TypeCheckResult {
    pub line: usize,
    pub error: TypeError,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_literal_inference() {
        let inf = TypeInference::new();
        let ctx = TypeContext::new();

        assert_eq!(inf.infer_expr("42", &ctx).unwrap(), Type::Int);
        assert_eq!(inf.infer_expr("3.14", &ctx).unwrap(), Type::Float);
        assert_eq!(inf.infer_expr("\"hello\"", &ctx).unwrap(), Type::String);
        assert_eq!(inf.infer_expr("true", &ctx).unwrap(), Type::Bool);
        assert_eq!(inf.infer_expr("null", &ctx).unwrap(), Type::Null);
    }

    #[test]
    fn test_binary_operations() {
        let inf = TypeInference::new();
        let ctx = TypeContext::new();

        // Int + Int = Int
        assert_eq!(inf.infer_expr("42 + 10", &ctx).unwrap(), Type::Int);

        // Float + Float = Float
        assert_eq!(inf.infer_expr("3.14 + 2.0", &ctx).unwrap(), Type::Float);

        // Int + Float = Float
        assert_eq!(inf.infer_expr("42 + 3.14", &ctx).unwrap(), Type::Float);
    }

    #[test]
    fn test_variable_inference() {
        let inf = TypeInference::new();
        let mut ctx = TypeContext::new();

        ctx.declare_var("x", Type::Int);
        ctx.declare_var("y", Type::Float);

        assert_eq!(inf.infer_expr("x", &ctx).unwrap(), Type::Int);
        assert_eq!(inf.infer_expr("y", &ctx).unwrap(), Type::Float);

        // x + y = Float
        assert_eq!(inf.infer_expr("x + y", &ctx).unwrap(), Type::Float);
    }

    #[test]
    fn test_function_call_type_checking() {
        let inf = TypeInference::new();
        let ctx = TypeContext::new();

        // sin expects Float
        assert!(inf.check_call("sin", &["3.14"], &ctx).is_ok());
        assert!(inf.check_call("sin", &["42"], &ctx).is_err()); // Int not compatible with Float
    }

    #[test]
    fn test_function_arg_count() {
        let inf = TypeInference::new();
        let ctx = TypeContext::new();

        // Wrong number of args
        assert!(matches!(
            inf.check_call("sin", &[], &ctx),
            Err(TypeError::WrongArgCount { .. })
        ));
    }
}
