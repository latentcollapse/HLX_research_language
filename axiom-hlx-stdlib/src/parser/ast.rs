use crate::lexer::token::Span;

/// A complete .axm file
#[derive(Debug, Clone)]
pub struct Program {
    pub module: Module,
}

/// module name { ... }
#[derive(Debug, Clone)]
pub struct Module {
    pub name: String,
    pub items: Vec<Item>,
    pub attributes: Vec<Attribute>,
    pub span: Span,
}

/// Top-level declarations within a module
#[derive(Debug, Clone)]
pub enum Item {
    Import(ImportDecl),
    Function(FunctionDecl),
    Contract(ContractDecl),
    Intent(IntentDecl),
    Enum(EnumDecl),
    TensorOp(TensorOpDecl),
}

/// import "path/to/module.axm";
#[derive(Debug, Clone)]
pub struct ImportDecl {
    pub path: String,
    pub span: Span,
}

/// fn name(params) -> ReturnType { body }
#[derive(Debug, Clone)]
pub struct FunctionDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeExpr>,
    pub body: Block,
    pub exported: bool,
    pub attributes: Vec<Attribute>,
    pub span: Span,
}

/// A function parameter: name: Type
#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: TypeExpr,
    pub span: Span,
}

/// contract Name { fields... }
#[derive(Debug, Clone)]
pub struct ContractDecl {
    pub name: String,
    pub fields: Vec<ContractField>,
    pub invariants: Option<Vec<Expr>>,
    pub composed_of: Option<Vec<String>>,
    pub span: Span,
}

/// @N: name: Type [conflict = Strategy]
#[derive(Debug, Clone)]
pub struct ContractField {
    pub index: u32,
    pub name: String,
    pub ty: TypeExpr,
    pub conflict: Option<String>,
    pub span: Span,
}

/// intent Name { clauses... }
#[derive(Debug, Clone)]
pub struct IntentDecl {
    pub name: String,
    pub clauses: IntentClauses,
    /// For composed intents: intent X = A >> B >> C;
    pub composed_of: Option<Vec<String>>,
    pub attributes: Vec<Attribute>,
    pub span: Span,
}

/// All the clauses in an intent declaration
#[derive(Debug, Clone, Default)]
pub struct IntentClauses {
    pub takes: Vec<Param>,
    pub gives: Vec<Param>,
    pub pre: Vec<Expr>,
    pub post: Vec<Expr>,
    pub bound: Vec<BoundClause>,
    pub effect: Option<String>,
    pub conscience: Vec<String>,
    pub fallback: Option<String>,
    pub rollback: Option<String>,
    pub trace: Option<String>,
    pub ring: Option<i64>,
}

/// bound: time(100ms), memory(64mb)
#[derive(Debug, Clone)]
pub struct BoundClause {
    pub resource: String,
    pub value: String,
    pub span: Span,
}

/// enum Name { Variant1, Variant2, ... }
#[derive(Debug, Clone)]
pub struct EnumDecl {
    pub name: String,
    pub variants: Vec<EnumVariant>,
    pub span: Span,
}

/// A single enum variant, optionally with fields
#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: String,
    pub fields: Vec<Param>,
    pub span: Span,
}

/// tensor_op name { takes, gives, shape_rule, determinism }
#[derive(Debug, Clone)]
pub struct TensorOpDecl {
    pub name: String,
    pub takes: Vec<Param>,
    pub gives: Vec<Param>,
    pub shape_rule: Option<Expr>,
    pub span: Span,
}

/// Type expressions
#[derive(Debug, Clone)]
pub enum TypeExpr {
    /// i64, f64, bool, String, Bytes, Handle, Provenance, Seed, AgentId, PrivilegeId
    Named(String, Span),
    /// [T]
    Array(Box<TypeExpr>, Span),
    /// Map<K, V>
    Map(Box<TypeExpr>, Box<TypeExpr>, Span),
    /// Tensor[dim1, dim2, ...]
    Tensor(Vec<TensorDim>, Span),
    /// Sealed<T>
    Sealed(Box<TypeExpr>, Span),
}

/// A tensor dimension: either a concrete number or ? (wildcard)
#[derive(Debug, Clone, PartialEq)]
pub enum TensorDim {
    Fixed(i64),
    Wildcard,
    Named(String),
}

/// An attribute like [max_depth(50)] or [scale(agents: 200, mode: independent)]
#[derive(Debug, Clone)]
pub struct Attribute {
    pub name: String,
    pub args: Vec<(Option<String>, String)>,
    pub span: Span,
}

/// A block of statements: { stmt; stmt; ... }
#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

/// Statements
#[derive(Debug, Clone)]
pub enum Stmt {
    /// let name: Type = expr;
    Let(LetStmt),
    /// return expr;
    Return(ReturnStmt),
    /// if cond { ... } else { ... }
    If(IfStmt),
    /// loop(cond, max_iter) { ... }
    Loop(LoopStmt),
    /// match value { arms... }
    Match(MatchStmt),
    /// bare expression as statement (e.g. function call)
    Expr(ExprStmt),
    /// Variable assignment: name = expr;
    Assign(AssignStmt),
    /// break;
    Break(Span),
    /// continue;
    Continue(Span),
}

#[derive(Debug, Clone)]
pub struct LetStmt {
    pub name: String,
    pub ty: Option<TypeExpr>,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ReturnStmt {
    pub value: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct IfStmt {
    pub condition: Expr,
    pub then_block: Block,
    pub else_block: Option<Block>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct LoopStmt {
    pub condition: Expr,
    pub max_iter: Expr,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct MatchStmt {
    pub value: Expr,
    pub arms: Vec<MatchArm>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Pattern {
    IntLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(String),
    BoolLiteral(bool),
    Ident(String),
    Wildcard,
    EnumVariant(String, String),
}

#[derive(Debug, Clone)]
pub struct ExprStmt {
    pub expr: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct AssignStmt {
    pub target: String,
    pub op: AssignOp,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum AssignOp {
    Assign,
    PlusAssign,
    MinusAssign,
}

/// Expressions
#[derive(Debug, Clone)]
pub enum Expr {
    /// Integer literal
    IntLiteral(i64, Span),
    /// Float literal
    FloatLiteral(f64, Span),
    /// String literal
    StringLiteral(String, Span),
    /// Boolean literal
    BoolLiteral(bool, Span),
    /// Variable reference
    Ident(String, Span),
    /// Binary operation: left op right
    Binary(Box<Expr>, BinOp, Box<Expr>, Span),
    /// Unary operation: op expr
    Unary(UnaryOp, Box<Expr>, Span),
    /// Function call: name(args)
    Call(String, Vec<Expr>, Span),
    /// Method-style call via pipeline: expr |> fn
    Pipeline(Box<Expr>, Box<Expr>, Span),
    /// Field access: expr.field
    FieldAccess(Box<Expr>, String, Span),
    /// Contract construction: ContractName { field: value, ... }
    ContractInit(String, Vec<(String, Expr)>, Span),
    /// do IntentName { field: value, ... }
    Do(String, Vec<(String, Expr)>, Span),
    /// query_conscience(IntentName { fields... })
    QueryConscience(String, Vec<(String, Expr)>, Span),
    /// declare_anomaly(type, { evidence: [...], request: "..." })
    DeclareAnomaly(Box<Expr>, Vec<(String, Expr)>, Span),
    /// collapse(expr)
    Collapse(Box<Expr>, Span),
    /// resolve(expr)
    Resolve(Box<Expr>, Span),
    /// Array literal: [expr, expr, ...]
    ArrayLiteral(Vec<Expr>, Span),
    /// Enum variant access: EnumName.Variant
    EnumAccess(String, String, Span),
    /// Block expression: { stmts }
    Block(Block, Span),
    /// Index access: expr[index] (placeholder for future)
    Index(Box<Expr>, Box<Expr>, Span),
}

impl Expr {
    pub fn span(&self) -> &Span {
        match self {
            Expr::IntLiteral(_, s) => s,
            Expr::FloatLiteral(_, s) => s,
            Expr::StringLiteral(_, s) => s,
            Expr::BoolLiteral(_, s) => s,
            Expr::Ident(_, s) => s,
            Expr::Binary(_, _, _, s) => s,
            Expr::Unary(_, _, s) => s,
            Expr::Call(_, _, s) => s,
            Expr::Pipeline(_, _, s) => s,
            Expr::FieldAccess(_, _, s) => s,
            Expr::ContractInit(_, _, s) => s,
            Expr::Do(_, _, s) => s,
            Expr::QueryConscience(_, _, s) => s,
            Expr::DeclareAnomaly(_, _, s) => s,
            Expr::Collapse(_, s) => s,
            Expr::Resolve(_, s) => s,
            Expr::ArrayLiteral(_, s) => s,
            Expr::EnumAccess(_, _, s) => s,
            Expr::Block(_, s) => s,
            Expr::Index(_, _, s) => s,
        }
    }
}

#[derive(Debug, Clone)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    And,
    Or,
}

#[derive(Debug, Clone)]
pub enum UnaryOp {
    Neg,
    Not,
}
