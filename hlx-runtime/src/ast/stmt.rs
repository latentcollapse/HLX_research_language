//! Statement AST nodes
//!
//! Statements perform actions but don't produce values (except return).
//! Control flow, declarations, and agent lifecycle are all statements.

use super::{Attribute, Expression, NodeId, SourceSpan, TypeAnnotation};
use serde::{Deserialize, Serialize};

/// A statement node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Statement {
    pub id: NodeId,
    pub span: SourceSpan,
    pub kind: StmtKind,
}

impl Statement {
    pub fn new(kind: StmtKind) -> Self {
        Statement {
            id: NodeId::new(),
            span: SourceSpan::unknown(),
            kind,
        }
    }

    pub fn with_span(mut self, span: SourceSpan) -> Self {
        self.span = span;
        self
    }
}

/// The kind of statement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StmtKind {
    /// Variable declaration: let x = 42; or let x: Type;
    Let {
        name: String,
        ty: Option<TypeAnnotation>,
        mutable: bool,
        value: Option<Expression>,
    },
    /// Assignment: x = 42;
    Assign {
        target: Expression,
        value: Expression,
    },
    /// Compound assignment: x += 1;
    CompoundAssign {
        target: Expression,
        op: super::BinaryOp,
        value: Expression,
    },
    /// Expression statement: foo();
    Expr(Expression),
    /// Return: return value;
    Return(Option<Expression>),
    /// If statement with optional else
    If(IfStmt),
    /// Loop with condition and optional bound
    Loop(LoopStmt),
    /// While loop: while cond { body }
    While {
        condition: Expression,
        body: Vec<Statement>,
    },
    /// For loop: for pat in iter { body }
    For {
        pattern: super::Pattern,
        iterable: Expression,
        body: Vec<Statement>,
    },
    /// Break from loop
    Break,
    /// Continue in loop
    Continue,
    /// Block: { statements }
    Block(Vec<Statement>),
    /// Switch statement
    Switch(SwitchStmt),
    /// Match expression statement
    Match(MatchStmt),
    /// Module definition
    Module(ModuleDef),
    /// Import statement
    Import(Import),
    /// Export statement
    Export(Export),
    /// Migrate an agent to a different cluster or host: migrate agent_name to target;
    Migrate { agent: String, target: String },
}

/// If statement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfStmt {
    pub condition: Expression,
    pub then_body: Vec<Statement>,
    pub else_body: Vec<Statement>,
}

/// Loop statement with bounded iteration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopStmt {
    pub condition: Expression,
    pub max_iterations: Option<u64>,
    pub body: Vec<Statement>,
}

impl LoopStmt {
    pub fn new(condition: Expression, body: Vec<Statement>) -> Self {
        LoopStmt {
            condition,
            max_iterations: None,
            body,
        }
    }

    pub fn with_max_iterations(mut self, max: u64) -> Self {
        self.max_iterations = Some(max);
        self
    }
}

/// Switch statement (pattern matching)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchStmt {
    pub discriminant: Expression,
    pub cases: Vec<SwitchCase>,
    pub default_body: Vec<Statement>,
}

/// A case in a switch statement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchCase {
    pub id: NodeId,
    pub pattern: super::Pattern,
    pub guard: Option<Expression>,
    pub body: Vec<Statement>,
}

/// Match expression statement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchStmt {
    pub subject: Expression,
    pub arms: Vec<MatchArm>,
}

/// A match arm with pattern, optional guard, and body
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchArm {
    pub id: NodeId,
    pub pattern: MatchPattern,
    pub guard: Option<Expression>,
    pub body: Vec<Statement>,
}

/// Pattern for match expression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MatchPattern {
    /// Literal value: 42, "hello", true
    Literal(super::Literal),
    /// Wildcard: _
    Wildcard,
    /// Binding pattern that captures the value: n, x, etc
    Binding(String),
    /// Range pattern (for future extension)
    Range { start: i64, end: i64 },
}

/// Function definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    pub id: NodeId,
    pub span: SourceSpan,
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<TypeAnnotation>,
    pub body: Vec<Statement>,
    pub attributes: Vec<Attribute>,
    pub is_exported: bool,
}

impl Function {
    pub fn new(name: impl Into<String>, parameters: Vec<Parameter>, body: Vec<Statement>) -> Self {
        Function {
            id: NodeId::new(),
            span: SourceSpan::unknown(),
            name: name.into(),
            parameters,
            return_type: None,
            body,
            attributes: Vec::new(),
            is_exported: false,
        }
    }

    pub fn with_return_type(mut self, ty: TypeAnnotation) -> Self {
        self.return_type = Some(ty);
        self
    }

    pub fn with_attribute(mut self, attr: Attribute) -> Self {
        self.attributes.push(attr);
        self
    }

    pub fn exported(mut self) -> Self {
        self.is_exported = true;
        self
    }
}

/// Function parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub id: NodeId,
    pub name: String,
    pub ty: Option<TypeAnnotation>,
    pub default_value: Option<Expression>,
}

impl Parameter {
    pub fn new(name: impl Into<String>) -> Self {
        Parameter {
            id: NodeId::new(),
            name: name.into(),
            ty: None,
            default_value: None,
        }
    }

    pub fn with_type(mut self, ty: TypeAnnotation) -> Self {
        self.ty = Some(ty);
        self
    }

    pub fn with_default(mut self, value: Expression) -> Self {
        self.default_value = Some(value);
        self
    }
}

/// Module definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleDef {
    pub id: NodeId,
    pub span: SourceSpan,
    pub name: String,
    pub items: Vec<super::Item>,
}

/// Struct definition: struct Point { x: i64, y: i64 }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructDef {
    pub id: NodeId,
    pub span: SourceSpan,
    pub name: String,
    pub fields: Vec<StructField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructField {
    pub name: String,
    pub ty: super::TypeAnnotation,
}

/// Import statement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Import {
    pub id: NodeId,
    pub span: SourceSpan,
    pub module: String,
    pub items: Vec<ImportItem>,
}

/// Import item (specific or wildcard)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ImportItem {
    /// import { foo, bar }
    Named(String),
    /// import { foo as bar }
    Aliased { name: String, alias: String },
    /// import *
    Wildcard,
}

/// Export statement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Export {
    pub id: NodeId,
    pub span: SourceSpan,
    pub item: Box<super::Item>,
}

// Helper constructors
impl Statement {
    pub fn let_(name: impl Into<String>, value: Expression) -> Self {
        Statement::new(StmtKind::Let {
            name: name.into(),
            ty: None,
            mutable: false,
            value: Some(value),
        })
    }

    pub fn let_mut(name: impl Into<String>, value: Expression) -> Self {
        Statement::new(StmtKind::Let {
            name: name.into(),
            ty: None,
            mutable: true,
            value: Some(value),
        })
    }

    /// Create a let binding without initialization: let x: Type;
    pub fn let_decl(name: impl Into<String>, ty: Option<TypeAnnotation>) -> Self {
        Statement::new(StmtKind::Let {
            name: name.into(),
            ty,
            mutable: false,
            value: None,
        })
    }

    pub fn assign(target: Expression, value: Expression) -> Self {
        Statement::new(StmtKind::Assign { target, value })
    }

    pub fn compound_assign(target: Expression, op: super::BinaryOp, value: Expression) -> Self {
        Statement::new(StmtKind::CompoundAssign { target, op, value })
    }

    pub fn expr(expr: Expression) -> Self {
        Statement::new(StmtKind::Expr(expr))
    }

    pub fn return_(value: Option<Expression>) -> Self {
        Statement::new(StmtKind::Return(value))
    }

    pub fn if_(
        condition: Expression,
        then_body: Vec<Statement>,
        else_body: Vec<Statement>,
    ) -> Self {
        Statement::new(StmtKind::If(IfStmt {
            condition,
            then_body,
            else_body,
        }))
    }

    pub fn loop_(condition: Expression, body: Vec<Statement>) -> Self {
        Statement::new(StmtKind::Loop(LoopStmt::new(condition, body)))
    }

    pub fn break_() -> Self {
        Statement::new(StmtKind::Break)
    }

    pub fn continue_() -> Self {
        Statement::new(StmtKind::Continue)
    }

    pub fn block(stmts: Vec<Statement>) -> Self {
        Statement::new(StmtKind::Block(stmts))
    }
}
