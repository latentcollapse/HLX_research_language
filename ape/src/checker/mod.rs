use std::collections::HashMap;
use crate::parser::ast::*;
use crate::error::{AxiomError, AxiomResult, ErrorKind};
use crate::lexer::token::Span;

/// Axiom's type representation for the checker
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    I64,
    F64,
    Bool,
    String,
    Bytes,
    Handle,
    Provenance,
    Seed,
    AgentId,
    PrivilegeId,
    Void,
    Array(Box<Type>),
    Map(Box<Type>, Box<Type>),
    Tensor(Vec<TensorDim>),
    Sealed(Box<Type>),
    Contract(String),
    Enum(String),
    /// Function type for type checking calls
    Function {
        params: Vec<Type>,
        ret: Box<Type>,
    },
    /// Intent return type
    IntentResult(String),
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::I64 => write!(f, "i64"),
            Type::F64 => write!(f, "f64"),
            Type::Bool => write!(f, "bool"),
            Type::String => write!(f, "String"),
            Type::Bytes => write!(f, "Bytes"),
            Type::Handle => write!(f, "Handle"),
            Type::Provenance => write!(f, "Provenance"),
            Type::Seed => write!(f, "Seed"),
            Type::AgentId => write!(f, "AgentId"),
            Type::PrivilegeId => write!(f, "PrivilegeId"),
            Type::Void => write!(f, "void"),
            Type::Array(inner) => write!(f, "[{}]", inner),
            Type::Map(k, v) => write!(f, "Map<{}, {}>", k, v),
            Type::Tensor(dims) => {
                write!(f, "Tensor[")?;
                for (i, d) in dims.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    match d {
                        TensorDim::Fixed(n) => write!(f, "{}", n)?,
                        TensorDim::Wildcard => write!(f, "?")?,
                        TensorDim::Named(n) => write!(f, "{}", n)?,
                    }
                }
                write!(f, "]")
            }
            Type::Sealed(inner) => write!(f, "Sealed<{}>", inner),
            Type::Contract(name) => write!(f, "{}", name),
            Type::Enum(name) => write!(f, "{}", name),
            Type::Function { params, ret } => {
                write!(f, "fn(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", p)?;
                }
                write!(f, ") -> {}", ret)
            }
            Type::IntentResult(name) => write!(f, "IntentResult<{}>", name),
        }
    }
}

/// Contract field info
#[derive(Debug, Clone)]
pub struct FieldInfo {
    pub index: u32,
    pub name: String,
    pub ty: Type,
}

/// Contract info for the type environment
#[derive(Debug, Clone)]
pub struct ContractInfo {
    pub name: String,
    pub fields: Vec<FieldInfo>,
}

/// Intent info for the type environment
#[derive(Debug, Clone)]
pub struct IntentInfo {
    pub name: String,
    pub takes: Vec<(String, Type)>,
    pub gives: Vec<(String, Type)>,
    pub effect: Option<String>,
    pub has_fallback: bool,
}

/// Enum info
#[derive(Debug, Clone)]
pub struct EnumInfo {
    pub name: String,
    pub variants: Vec<String>,
}

/// Type-checking environment
pub struct TypeChecker {
    /// Variable types in current scope
    scopes: Vec<HashMap<String, Type>>,
    /// Registered contracts
    contracts: HashMap<String, ContractInfo>,
    /// Registered intents
    intents: HashMap<String, IntentInfo>,
    /// Registered enums
    enums: HashMap<String, EnumInfo>,
    /// Registered functions
    functions: HashMap<String, Type>,
    /// Collected errors (non-fatal)
    pub warnings: Vec<AxiomError>,
}

impl TypeChecker {
    pub fn new() -> Self {
        let mut contracts = HashMap::new();
        // Built-in contract: ConscienceQuery (Section 6.4)
        contracts.insert(
            "ConscienceQuery".to_string(),
            ContractInfo {
                name: "ConscienceQuery".to_string(),
                fields: vec![
                    FieldInfo { index: 0, name: "permitted".to_string(), ty: Type::Bool },
                    FieldInfo { index: 1, name: "category".to_string(), ty: Type::Enum("QueryCategory".to_string()) },
                    FieldInfo { index: 2, name: "guidance".to_string(), ty: Type::String },
                ],
            },
        );

        let mut enums = HashMap::new();
        // Built-in enums
        enums.insert("QueryCategory".to_string(), EnumInfo {
            name: "QueryCategory".to_string(),
            variants: vec![
                "CHANNEL_POLICY".to_string(), "RESOURCE_POLICY".to_string(),
                "IRREVERSIBLE_ACTION".to_string(), "CONSCIENCE_CORE".to_string(),
            ],
        });
        enums.insert("AnomalyType".to_string(), EnumInfo {
            name: "AnomalyType".to_string(),
            variants: vec![
                "ResourceStarvation".to_string(), "RepeatedRejection".to_string(),
                "CoherenceConcern".to_string(), "ConstraintAmbiguity".to_string(),
                "Isolation".to_string(),
            ],
        });
        enums.insert("Provenance".to_string(), EnumInfo {
            name: "Provenance".to_string(),
            variants: vec![
                "TRUSTED_INTERNAL".to_string(), "TRUSTED_VERIFIED".to_string(),
                "UNTRUSTED_EXTERNAL".to_string(), "UNTRUSTED_TAINTED".to_string(),
            ],
        });

        TypeChecker {
            scopes: vec![HashMap::new()],
            contracts,
            intents: HashMap::new(),
            enums,
            functions: HashMap::new(),
            warnings: Vec::new(),
        }
    }

    pub fn check_program(&mut self, program: &Program) -> AxiomResult<()> {
        self.check_module(&program.module)
    }

    fn check_module(&mut self, module: &Module) -> AxiomResult<()> {
        // First pass: register all type declarations
        for item in &module.items {
            match item {
                Item::Contract(c) => self.register_contract(c)?,
                Item::Intent(i) => self.register_intent(i)?,
                Item::Enum(e) => self.register_enum(e)?,
                Item::Function(f) => self.register_function(f)?,
                _ => {}
            }
        }

        // Second pass: check function bodies
        for item in &module.items {
            if let Item::Function(f) = item {
                self.check_function(f)?;
            }
        }
        Ok(())
    }

    fn register_contract(&mut self, decl: &ContractDecl) -> AxiomResult<()> {
        if decl.composed_of.is_some() {
            // Composed contracts — just register the name for now
            self.contracts.insert(
                decl.name.clone(),
                ContractInfo {
                    name: decl.name.clone(),
                    fields: Vec::new(),
                },
            );
            return Ok(());
        }

        let mut fields = Vec::new();
        for f in &decl.fields {
            fields.push(FieldInfo {
                index: f.index,
                name: f.name.clone(),
                ty: self.resolve_type(&f.ty)?,
            });
        }
        self.contracts.insert(
            decl.name.clone(),
            ContractInfo {
                name: decl.name.clone(),
                fields,
            },
        );
        Ok(())
    }

    fn register_intent(&mut self, decl: &IntentDecl) -> AxiomResult<()> {
        let mut takes = Vec::new();
        for p in &decl.clauses.takes {
            takes.push((p.name.clone(), self.resolve_type(&p.ty)?));
        }
        let mut gives = Vec::new();
        for p in &decl.clauses.gives {
            gives.push((p.name.clone(), self.resolve_type(&p.ty)?));
        }
        self.intents.insert(
            decl.name.clone(),
            IntentInfo {
                name: decl.name.clone(),
                takes,
                gives,
                effect: decl.clauses.effect.clone(),
                has_fallback: decl.clauses.fallback.is_some(),
            },
        );
        Ok(())
    }

    fn register_enum(&mut self, decl: &EnumDecl) -> AxiomResult<()> {
        let variants = decl.variants.iter().map(|v| v.name.clone()).collect();
        self.enums.insert(
            decl.name.clone(),
            EnumInfo {
                name: decl.name.clone(),
                variants,
            },
        );
        Ok(())
    }

    fn register_function(&mut self, decl: &FunctionDecl) -> AxiomResult<()> {
        let params: Vec<Type> = decl
            .params
            .iter()
            .map(|p| self.resolve_type(&p.ty))
            .collect::<AxiomResult<_>>()?;
        let ret = match &decl.return_type {
            Some(ty) => self.resolve_type(ty)?,
            None => Type::Void,
        };
        self.functions.insert(
            decl.name.clone(),
            Type::Function {
                params,
                ret: Box::new(ret),
            },
        );
        Ok(())
    }

    fn check_function(&mut self, decl: &FunctionDecl) -> AxiomResult<()> {
        self.push_scope();

        // Add params to scope
        for p in &decl.params {
            let ty = self.resolve_type(&p.ty)?;
            self.define(&p.name, ty);
        }

        // Check body
        self.check_block(&decl.body)?;

        self.pop_scope();
        Ok(())
    }

    fn check_block(&mut self, block: &Block) -> AxiomResult<()> {
        for stmt in &block.stmts {
            self.check_stmt(stmt)?;
        }
        Ok(())
    }

    fn check_stmt(&mut self, stmt: &Stmt) -> AxiomResult<()> {
        match stmt {
            Stmt::Let(let_stmt) => {
                let val_ty = self.check_expr(&let_stmt.value)?;
                if let Some(declared_ty) = &let_stmt.ty {
                    let expected = self.resolve_type(declared_ty)?;
                    self.check_assignable(&expected, &val_ty, &let_stmt.span)?;
                }
                let ty = if let Some(t) = &let_stmt.ty {
                    self.resolve_type(t)?
                } else {
                    val_ty
                };
                self.define(&let_stmt.name, ty);
                Ok(())
            }
            Stmt::Return(ret) => {
                if let Some(val) = &ret.value {
                    self.check_expr(val)?;
                }
                Ok(())
            }
            Stmt::If(if_stmt) => {
                let cond_ty = self.check_expr(&if_stmt.condition)?;
                self.check_assignable(&Type::Bool, &cond_ty, &if_stmt.span)?;
                self.push_scope();
                self.check_block(&if_stmt.then_block)?;
                self.pop_scope();
                if let Some(else_block) = &if_stmt.else_block {
                    self.push_scope();
                    self.check_block(else_block)?;
                    self.pop_scope();
                }
                Ok(())
            }
            Stmt::Loop(loop_stmt) => {
                let cond_ty = self.check_expr(&loop_stmt.condition)?;
                self.check_assignable(&Type::Bool, &cond_ty, &loop_stmt.span)?;
                let iter_ty = self.check_expr(&loop_stmt.max_iter)?;
                self.check_assignable(&Type::I64, &iter_ty, &loop_stmt.span)?;
                self.push_scope();
                self.check_block(&loop_stmt.body)?;
                self.pop_scope();
                Ok(())
            }
            Stmt::Match(match_stmt) => {
                self.check_expr(&match_stmt.value)?;
                for arm in &match_stmt.arms {
                    self.check_expr(&arm.body)?;
                }
                Ok(())
            }
            Stmt::Expr(expr_stmt) => {
                self.check_expr(&expr_stmt.expr)?;
                Ok(())
            }
            Stmt::Assign(assign) => {
                let var_ty = self.lookup(&assign.target, &assign.span)?;
                let val_ty = self.check_expr(&assign.value)?;
                self.check_assignable(&var_ty, &val_ty, &assign.span)?;
                Ok(())
            }
            Stmt::Break(_) | Stmt::Continue(_) => Ok(()),
        }
    }

    fn check_expr(&mut self, expr: &Expr) -> AxiomResult<Type> {
        match expr {
            Expr::IntLiteral(_, _) => Ok(Type::I64),
            Expr::FloatLiteral(_, _) => Ok(Type::F64),
            Expr::StringLiteral(_, _) => Ok(Type::String),
            Expr::BoolLiteral(_, _) => Ok(Type::Bool),
            Expr::Ident(name, span) => self.lookup(name, span),
            Expr::Binary(left, op, right, span) => {
                let lt = self.check_expr(left)?;
                let rt = self.check_expr(right)?;
                self.check_binary_op(&lt, op, &rt, span)
            }
            Expr::Unary(op, inner, span) => {
                let t = self.check_expr(inner)?;
                match op {
                    UnaryOp::Neg => {
                        if matches!(t, Type::I64 | Type::F64) {
                            Ok(t)
                        } else {
                            Err(self.type_error(span, &format!("Cannot negate type {}", t)))
                        }
                    }
                    UnaryOp::Not => {
                        self.check_assignable(&Type::Bool, &t, span)?;
                        Ok(Type::Bool)
                    }
                }
            }
            Expr::Call(name, args, span) => self.check_call(name, args, span),
            Expr::Pipeline(left, right, span) => {
                // left |> right desugars to right(left)
                let arg_ty = self.check_expr(left)?;
                if let Expr::Ident(fname, _) | Expr::Call(fname, _, _) = right.as_ref() {
                    // Check that right is callable with left as first arg
                    if let Some(Type::Function { params, ret }) = self.functions.get(fname).cloned()
                    {
                        if params.is_empty() {
                            return Err(self.type_error(
                                span,
                                &format!("Function '{}' takes no arguments for pipeline", fname),
                            ));
                        }
                        self.check_assignable(&params[0], &arg_ty, span)?;
                        return Ok(*ret);
                    }
                }
                // Fallback: just check both sides
                self.check_expr(right)?;
                Ok(Type::Void)
            }
            Expr::FieldAccess(obj, field, span) => {
                let obj_ty = self.check_expr(obj)?;
                match &obj_ty {
                    // Sealed type enforcement (Section 4.3):
                    // Cannot access fields through Sealed<T> — must unwrap via Verify first
                    Type::Sealed(inner) => {
                        Err(self.type_error(
                            span,
                            &format!(
                                "Cannot access field '{}' through Sealed type. Sealed values must be unwrapped via `do Verify` before field access. Inner type: {}",
                                field, inner
                            ),
                        ))
                    }
                    Type::Contract(name) => {
                        if let Some(info) = self.contracts.get(name) {
                            for f in &info.fields {
                                if f.name == *field {
                                    return Ok(f.ty.clone());
                                }
                            }
                            Err(self.type_error(
                                span,
                                &format!("Contract '{}' has no field '{}'", name, field),
                            ))
                        } else {
                            Err(self.type_error(
                                span,
                                &format!("Unknown contract '{}'", name),
                            ))
                        }
                    }
                    _ => {
                        // Allow field access on anything for now (e.g. query_conscience results)
                        Ok(Type::Void)
                    }
                }
            }
            Expr::ContractInit(name, fields, span) => {
                if let Some(info) = self.contracts.get(name).cloned() {
                    for (fname, fexpr) in fields {
                        let fty = self.check_expr(fexpr)?;
                        if let Some(expected) = info.fields.iter().find(|f| f.name == *fname) {
                            self.check_assignable(&expected.ty, &fty, span)?;
                        }
                    }
                    Ok(Type::Contract(name.clone()))
                } else {
                    Err(self.type_error(span, &format!("Unknown contract '{}'", name)))
                }
            }
            Expr::Do(intent_name, fields, span) => {
                if let Some(info) = self.intents.get(intent_name).cloned() {
                    // Check provided fields against takes
                    for (fname, fexpr) in fields {
                        let fty = self.check_expr(fexpr)?;
                        if let Some((_, expected)) =
                            info.takes.iter().find(|(n, _)| n == fname)
                        {
                            self.check_assignable(expected, &fty, span)?;
                        }
                    }
                    // Return type is the gives type(s)
                    if info.gives.len() == 1 {
                        Ok(info.gives[0].1.clone())
                    } else {
                        Ok(Type::IntentResult(intent_name.clone()))
                    }
                } else {
                    // Unknown intent — return a generic result type
                    for (_, fexpr) in fields {
                        self.check_expr(fexpr)?;
                    }
                    Ok(Type::IntentResult(intent_name.clone()))
                }
            }
            Expr::QueryConscience(_, fields, _span) => {
                for (_, fexpr) in fields {
                    self.check_expr(fexpr)?;
                }
                Ok(Type::Contract("ConscienceQuery".to_string()))
            }
            Expr::DeclareAnomaly(ty_expr, fields, _span) => {
                self.check_expr(ty_expr)?;
                for (_, fexpr) in fields {
                    self.check_expr(fexpr)?;
                }
                Ok(Type::Void)
            }
            Expr::Collapse(inner, _span) => {
                self.check_expr(inner)?;
                Ok(Type::Handle)
            }
            Expr::Resolve(inner, _span) => {
                self.check_expr(inner)?;
                // Resolve returns the original type, but we can't know it statically
                Ok(Type::Void)
            }
            Expr::ArrayLiteral(elems, _span) => {
                if elems.is_empty() {
                    Ok(Type::Array(Box::new(Type::Void)))
                } else {
                    let first_ty = self.check_expr(&elems[0])?;
                    for elem in &elems[1..] {
                        self.check_expr(elem)?;
                    }
                    Ok(Type::Array(Box::new(first_ty)))
                }
            }
            Expr::EnumAccess(enum_name, variant, span) => {
                if let Some(info) = self.enums.get(enum_name) {
                    if info.variants.contains(variant) {
                        Ok(Type::Enum(enum_name.clone()))
                    } else {
                        Err(self.type_error(
                            span,
                            &format!("Enum '{}' has no variant '{}'", enum_name, variant),
                        ))
                    }
                } else {
                    // Could be a contract field access - allow it
                    Ok(Type::Void)
                }
            }
            Expr::Block(block, _span) => {
                self.push_scope();
                self.check_block(block)?;
                self.pop_scope();
                Ok(Type::Void)
            }
            Expr::Index(arr, idx, _span) => {
                let arr_ty = self.check_expr(arr)?;
                self.check_expr(idx)?;
                match arr_ty {
                    Type::Array(inner) => Ok(*inner),
                    Type::String => Ok(Type::String),
                    _ => Ok(Type::Void),
                }
            }
        }
    }

    fn check_call(
        &mut self,
        name: &str,
        args: &[Expr],
        span: &Span,
    ) -> AxiomResult<Type> {
        // Check args
        let mut arg_types = Vec::new();
        for arg in args {
            arg_types.push(self.check_expr(arg)?);
        }

        // Built-in functions
        match name {
            "log" | "print" => return Ok(Type::Void),
            "length" | "len" => return Ok(Type::I64),
            "sqrt" | "abs" => {
                if let Some(t) = arg_types.first() {
                    return Ok(t.clone());
                }
                return Ok(Type::F64);
            }
            "to_string" => return Ok(Type::String),
            // Self-hosting primitives (Session 2)
            "char_at" => return Ok(Type::String),       // returns single-char string
            "char_code" => return Ok(Type::I64),         // returns char code point
            "char_from_code" => return Ok(Type::String), // returns single-char string
            "substring" => return Ok(Type::String),      // returns substring
            "push" => {
                // push(array, val) returns a new array of the same type
                if let Some(t) = arg_types.first() {
                    return Ok(t.clone());
                }
                return Ok(Type::Array(Box::new(Type::Void)));
            }
            "parse_int" => return Ok(Type::I64),
            "as_f64" => return Ok(Type::F64),
            "as_i64" => return Ok(Type::I64),
            "contains" => return Ok(Type::Bool),
            // Map operations
            "map_new" => return Ok(Type::Map(Box::new(Type::String), Box::new(Type::Void))),
            "map_insert" => {
                if let Some(t) = arg_types.first() {
                    return Ok(t.clone());
                }
                return Ok(Type::Map(Box::new(Type::String), Box::new(Type::Void)));
            }
            "map_get" => return Ok(Type::Void), // unknown value type
            "map_has_key" => return Ok(Type::Bool),
            "map_keys" => return Ok(Type::Array(Box::new(Type::String))),
            "map_size" => return Ok(Type::I64),
            // Guard helpers
            "path_exists" | "path_is_safe" | "bounded" => return Ok(Type::Bool),
            _ => {}
        }

        // User-defined functions
        if let Some(Type::Function { params, ret }) = self.functions.get(name).cloned() {
            if args.len() != params.len() {
                return Err(AxiomError {
                    kind: ErrorKind::ArgumentCount,
                    message: format!(
                        "Function '{}' expects {} arguments, got {}",
                        name,
                        params.len(),
                        args.len()
                    ),
                    span: Some(span.clone()),
                });
            }
            for (_i, (expected, actual)) in params.iter().zip(arg_types.iter()).enumerate() {
                self.check_assignable(expected, actual, span)?;
            }
            return Ok(*ret);
        }

        // Unknown function — allow it with Void return for flexibility
        Ok(Type::Void)
    }

    fn check_binary_op(
        &self,
        left: &Type,
        op: &BinOp,
        right: &Type,
        span: &Span,
    ) -> AxiomResult<Type> {
        match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                // Void acts as a dynamic/unknown type — allow operations and
                // propagate the known type (or Void if both unknown)
                if matches!(left, Type::Void) {
                    return Ok(right.clone());
                }
                if matches!(right, Type::Void) {
                    return Ok(left.clone());
                }
                if matches!(left, Type::I64) && matches!(right, Type::I64) {
                    Ok(Type::I64)
                } else if matches!(left, Type::F64) && matches!(right, Type::F64) {
                    Ok(Type::F64)
                } else if matches!(left, Type::I64 | Type::F64)
                    && matches!(right, Type::I64 | Type::F64)
                {
                    Ok(Type::F64)
                } else if matches!(op, BinOp::Add)
                    && matches!(left, Type::String)
                    && matches!(right, Type::String)
                {
                    Ok(Type::String)
                } else {
                    Err(self.type_error(
                        span,
                        &format!("Cannot apply {:?} to {} and {}", op, left, right),
                    ))
                }
            }
            BinOp::Eq | BinOp::NotEq => Ok(Type::Bool),
            BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq => {
                if matches!(left, Type::I64 | Type::F64 | Type::Void)
                    && matches!(right, Type::I64 | Type::F64 | Type::Void)
                {
                    Ok(Type::Bool)
                } else {
                    Err(self.type_error(
                        span,
                        &format!("Cannot compare {} and {}", left, right),
                    ))
                }
            }
            BinOp::And | BinOp::Or => {
                if matches!(left, Type::Bool | Type::Void) && matches!(right, Type::Bool | Type::Void) {
                    Ok(Type::Bool)
                } else {
                    Err(self.type_error(
                        span,
                        &format!("Logical operators require bool, got {} and {}", left, right),
                    ))
                }
            }
        }
    }

    fn check_assignable(&self, expected: &Type, actual: &Type, span: &Span) -> AxiomResult<()> {
        // Allow Void to match anything (for unknown types)
        if matches!(expected, Type::Void) || matches!(actual, Type::Void) {
            return Ok(());
        }
        // Allow I64/F64 interop for numeric types
        if matches!(expected, Type::I64 | Type::F64) && matches!(actual, Type::I64 | Type::F64) {
            return Ok(());
        }
        // Tensor shape compatibility (Section 4.7):
        // Wildcards (?) match any dimension, named dims must match
        if let (Type::Tensor(expected_dims), Type::Tensor(actual_dims)) = (expected, actual) {
            return self.check_tensor_shapes(expected_dims, actual_dims, span);
        }
        // Sealed<T> can only be assigned to Sealed<T> (Section 4.3)
        if let (Type::Sealed(inner_e), Type::Sealed(inner_a)) = (expected, actual) {
            return self.check_assignable(inner_e, inner_a, span);
        }
        if std::mem::discriminant(expected) != std::mem::discriminant(actual) {
            return Err(AxiomError {
                kind: ErrorKind::TypeMismatch,
                message: format!("Type mismatch: expected {}, found {}", expected, actual),
                span: Some(span.clone()),
            });
        }
        Ok(())
    }

    /// Check tensor shape compatibility (Section 4.7)
    fn check_tensor_shapes(
        &self,
        expected: &[TensorDim],
        actual: &[TensorDim],
        span: &Span,
    ) -> AxiomResult<()> {
        if expected.len() != actual.len() {
            return Err(self.type_error(
                span,
                &format!(
                    "Tensor rank mismatch: expected {} dimensions, found {}",
                    expected.len(),
                    actual.len()
                ),
            ));
        }
        for (i, (e, a)) in expected.iter().zip(actual.iter()).enumerate() {
            match (e, a) {
                // Wildcards match anything
                (TensorDim::Wildcard, _) | (_, TensorDim::Wildcard) => {}
                // Named dims must match names
                (TensorDim::Named(n1), TensorDim::Named(n2)) => {
                    if n1 != n2 {
                        return Err(self.type_error(
                            span,
                            &format!(
                                "Tensor dimension {} name mismatch: expected '{}', found '{}'",
                                i, n1, n2
                            ),
                        ));
                    }
                }
                // Fixed dims must match values
                (TensorDim::Fixed(n1), TensorDim::Fixed(n2)) => {
                    if n1 != n2 {
                        return Err(self.type_error(
                            span,
                            &format!(
                                "Tensor dimension {} size mismatch: expected {}, found {}",
                                i, n1, n2
                            ),
                        ));
                    }
                }
                // Named can match fixed (flexible)
                _ => {}
            }
        }
        Ok(())
    }

    // --- Scope management ---

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn define(&mut self, name: &str, ty: Type) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), ty);
        }
    }

    fn lookup(&self, name: &str, span: &Span) -> AxiomResult<Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Ok(ty.clone());
            }
        }
        // Check if it's a contract name (for constructor use)
        if self.contracts.contains_key(name) {
            return Ok(Type::Contract(name.to_string()));
        }
        // Check if it's an enum name
        if self.enums.contains_key(name) {
            return Ok(Type::Enum(name.to_string()));
        }
        Err(AxiomError {
            kind: ErrorKind::UndefinedVariable,
            message: format!("Undefined variable '{}'", name),
            span: Some(span.clone()),
        })
    }

    // --- Type resolution ---

    fn resolve_type(&self, ty: &TypeExpr) -> AxiomResult<Type> {
        match ty {
            TypeExpr::Named(name, _) => match name.as_str() {
                "i64" => Ok(Type::I64),
                "f64" => Ok(Type::F64),
                "bool" => Ok(Type::Bool),
                "String" => Ok(Type::String),
                "Bytes" => Ok(Type::Bytes),
                "Handle" => Ok(Type::Handle),
                "Provenance" => Ok(Type::Provenance),
                "Seed" => Ok(Type::Seed),
                "AgentId" => Ok(Type::AgentId),
                "PrivilegeId" => Ok(Type::PrivilegeId),
                _ => Ok(Type::Contract(name.clone())),
            },
            TypeExpr::Array(inner, _) => Ok(Type::Array(Box::new(self.resolve_type(inner)?))),
            TypeExpr::Map(k, v, _) => Ok(Type::Map(
                Box::new(self.resolve_type(k)?),
                Box::new(self.resolve_type(v)?),
            )),
            TypeExpr::Tensor(dims, _) => Ok(Type::Tensor(dims.clone())),
            TypeExpr::Sealed(inner, _) => Ok(Type::Sealed(Box::new(self.resolve_type(inner)?))),
        }
    }

    fn type_error(&self, span: &Span, message: &str) -> AxiomError {
        AxiomError {
            kind: ErrorKind::TypeMismatch,
            message: message.to_string(),
            span: Some(span.clone()),
        }
    }
}
