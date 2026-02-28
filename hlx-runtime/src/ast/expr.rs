//! Expression AST nodes
//!
//! Expressions produce values and can be nested arbitrarily.
//! Each expression carries its source span and a unique NodeId.

use super::{NodeId, SourceSpan, TypeAnnotation};
use serde::{Deserialize, Serialize};

/// An expression node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Expression {
    pub id: NodeId,
    pub span: SourceSpan,
    pub kind: ExprKind,
    /// Inferred or annotated type (for future type checking)
    pub ty: Option<TypeAnnotation>,
}

impl Expression {
    pub fn new(kind: ExprKind) -> Self {
        Expression {
            id: NodeId::new(),
            span: SourceSpan::unknown(),
            kind,
            ty: None,
        }
    }

    pub fn with_span(mut self, span: SourceSpan) -> Self {
        self.span = span;
        self
    }

    pub fn with_type(mut self, ty: TypeAnnotation) -> Self {
        self.ty = Some(ty);
        self
    }
}

/// The kind of expression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExprKind {
    /// Integer literal: 42
    Int(i64),
    /// Float literal: 3.14
    Float(f64),
    /// String literal: "hello"
    String(String),
    /// Boolean literal: true, false
    Bool(bool),
    /// Identifier: x, foo, MyModule
    Identifier(String),
    /// Binary operation: a + b
    BinaryOp {
        op: BinaryOp,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    /// Unary operation: -x, !flag
    UnaryOp {
        op: UnaryOp,
        operand: Box<Expression>,
    },
    /// Function call: foo(a, b)
    Call {
        function: String,
        arguments: Vec<Expression>,
    },
    /// Array literal: [1, 2, 3]
    Array(Vec<Expression>),
    /// Array index: arr[i]
    Index {
        array: Box<Expression>,
        index: Box<Expression>,
    },
    /// Range: start..end
    Range {
        start: Box<Expression>,
        end: Box<Expression>,
        inclusive: bool,
    },
    /// Contract (struct-like) literal: contract { field: value }
    Contract {
        contract_type: String,
        fields: Vec<(String, Expression)>,
    },
    /// Field access: obj.field
    FieldAccess {
        object: Box<Expression>,
        field: String,
    },
    /// Method call: obj.method(args)
    MethodCall {
        object: Box<Expression>,
        method: String,
        arguments: Vec<Expression>,
    },
    /// Lambda: |x, y| x + y
    Lambda {
        parameters: Vec<String>,
        body: Box<Expression>,
    },
    /// Conditional expression: if cond then else
    Conditional {
        condition: Box<Expression>,
        then_expr: Box<Expression>,
        else_expr: Box<Expression>,
    },
    /// Match expression: match val { pattern => body, ... }
    Match {
        value: Box<Expression>,
        cases: Vec<MatchCase>,
    },
    /// Collapse: collapse value
    Collapse(Box<Expression>),
    /// Resolve: resolve handle
    Resolve(Box<Expression>),
    /// Dict literal: { "key": value, ... }
    Dict(Vec<(Expression, Expression)>),
    /// Nil literal
    Nil,
    /// Void (for functions that don't return)
    Void,
}

/// Binary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinaryOp {
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    // Comparison
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    // Logical
    And,
    Or,
    // Bitwise
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

impl BinaryOp {
    pub fn symbol(&self) -> &'static str {
        match self {
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Mod => "%",
            BinaryOp::Pow => "**",
            BinaryOp::Eq => "==",
            BinaryOp::Ne => "!=",
            BinaryOp::Lt => "<",
            BinaryOp::Le => "<=",
            BinaryOp::Gt => ">",
            BinaryOp::Ge => ">=",
            BinaryOp::And => "&&",
            BinaryOp::Or => "||",
            BinaryOp::BitAnd => "&",
            BinaryOp::BitOr => "|",
            BinaryOp::BitXor => "^",
            BinaryOp::Shl => "<<",
            BinaryOp::Shr => ">>",
        }
    }
}

/// Unary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnaryOp {
    Neg,
    Not,
    BitNot,
}

impl UnaryOp {
    pub fn symbol(&self) -> &'static str {
        match self {
            UnaryOp::Neg => "-",
            UnaryOp::Not => "!",
            UnaryOp::BitNot => "~",
        }
    }
}

/// A match case in a match expression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchCase {
    pub id: NodeId,
    pub pattern: Pattern,
    pub guard: Option<Expression>,
    pub body: Expression,
}

/// Pattern for matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Pattern {
    /// Wildcard: _
    Wildcard,
    /// Integer pattern: 42
    Int(i64),
    /// String pattern: "hello"
    String(String),
    /// Identifier binding: x
    Identifier(String),
    /// Destructuring: Contract { field, .. }
    Destructure {
        type_name: String,
        fields: Vec<(String, Option<Pattern>)>,
        rest: bool,
    },
    /// Or pattern: A | B
    Or(Vec<Pattern>),
    /// Range pattern: 1..10
    Range {
        start: i64,
        end: i64,
        inclusive: bool,
    },
}

// Helper constructors for common expressions
impl Expression {
    pub fn int(value: i64) -> Self {
        Expression::new(ExprKind::Int(value))
    }

    pub fn float(value: f64) -> Self {
        Expression::new(ExprKind::Float(value))
    }

    pub fn string(value: impl Into<String>) -> Self {
        Expression::new(ExprKind::String(value.into()))
    }

    pub fn bool(value: bool) -> Self {
        Expression::new(ExprKind::Bool(value))
    }

    pub fn identifier(name: impl Into<String>) -> Self {
        Expression::new(ExprKind::Identifier(name.into()))
    }

    pub fn binary(op: BinaryOp, left: Expression, right: Expression) -> Self {
        Expression::new(ExprKind::BinaryOp {
            op,
            left: Box::new(left),
            right: Box::new(right),
        })
    }

    pub fn unary(op: UnaryOp, operand: Expression) -> Self {
        Expression::new(ExprKind::UnaryOp {
            op,
            operand: Box::new(operand),
        })
    }

    pub fn call(function: impl Into<String>, arguments: Vec<Expression>) -> Self {
        Expression::new(ExprKind::Call {
            function: function.into(),
            arguments,
        })
    }

    pub fn array(elements: Vec<Expression>) -> Self {
        Expression::new(ExprKind::Array(elements))
    }

    pub fn dict(pairs: Vec<(Expression, Expression)>) -> Self {
        Expression::new(ExprKind::Dict(pairs))
    }

    pub fn index(array: Expression, index: Expression) -> Self {
        Expression::new(ExprKind::Index {
            array: Box::new(array),
            index: Box::new(index),
        })
    }

    pub fn field_access(object: Expression, field: impl Into<String>) -> Self {
        Expression::new(ExprKind::FieldAccess {
            object: Box::new(object),
            field: field.into(),
        })
    }

    pub fn nil() -> Self {
        Expression::new(ExprKind::Nil)
    }
}
