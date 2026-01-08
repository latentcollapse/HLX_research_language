//! Abstract Syntax Tree for HLX/HLXL
//!
//! The AST is the shared representation between:
//! - HLXL (ASCII) parser/emitter
//! - HLX (Runic) parser/emitter
//!
//! This enables bijective translation between the two forms.

use hlx_core::value::{Value, ContractId, FieldIndex};
use serde::{Deserialize, Serialize};

/// Source location for error reporting
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: u32,
    pub col: u32,
}

impl Span {
    pub fn new(start: usize, end: usize, line: u32, col: u32) -> Self {
        Self { start, end, line, col }
    }
}

/// A spanned node (AST node with location info)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub fn new(node: T, span: Span) -> Self {
        Self { node, span }
    }
    
    pub fn dummy(node: T) -> Self {
        Self { node, span: Span::default() }
    }
}

/// Top-level program
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Program {
    pub name: String,
    pub blocks: Vec<Block>,
}

/// A named block (function-like)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Block {
    pub name: String,
    pub params: Vec<String>,
    pub return_type: Option<String>,
    pub items: Vec<Spanned<Item>>,
}

/// An item within a block (sequential or topological)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Item {
    /// A sequential statement (HLX-A style)
    Statement(Statement),
    
    /// A topological node (HLX-R style / Netlist)
    Node(Node),
}

/// A topological node in the execution graph
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    /// Optional name/ID for the node
    pub id: Option<String>,
    /// The operation to perform
    pub op: String,
    /// Input variables or constant expressions
    pub inputs: Vec<Spanned<Expr>>,
    /// Output registers/variables
    pub outputs: Vec<String>,
}

/// Statements
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Statement {
    /// let x = expr
    Let {
        name: String,
        value: Spanned<Expr>,
    },
    
    /// local x = expr (block-scoped)
    Local {
        name: String,
        value: Spanned<Expr>,
    },
    
    /// x = expr (reassignment) or arr[i] = expr
    Assign {
        lhs: Spanned<Expr>,
        value: Spanned<Expr>,
    },
    
    /// return expr
    Return {
        value: Spanned<Expr>,
    },
    
    If { condition: Spanned<Expr>, then_branch: Vec<Spanned<Statement>>, else_branch: Option<Vec<Spanned<Statement>>> },
    While { condition: Spanned<Expr>, body: Vec<Spanned<Statement>>, max_iter: u32 },
    Break,
    Continue,
    Expr(Spanned<Expr>),
}

/// Expressions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expr {
    // === Literals ===
    Literal(Literal),
    
    /// Variable reference
    Ident(String),
    
    /// Array constructor: [e1, e2, ...]
    Array(Vec<Spanned<Expr>>),
    
    /// Object constructor: { "k1": e1, "k2": e2, ... }
    Object(Vec<(String, Spanned<Expr>)>),
    
    // === Binary Operations ===
    BinOp {
        op: BinOp,
        lhs: Box<Spanned<Expr>>,
        rhs: Box<Spanned<Expr>>,
    },
    
    /// Unary operation
    UnaryOp {
        op: UnaryOp,
        operand: Box<Spanned<Expr>>,
    },
    
    /// Function call
    Call {
        func: Box<Spanned<Expr>>,
        args: Vec<Spanned<Expr>>,
    },
    
    /// Index access: expr[index]
    Index {
        object: Box<Spanned<Expr>>,
        index: Box<Spanned<Expr>>,
    },
    
    /// Field access: expr.field
    Field {
        object: Box<Spanned<Expr>>,
        field: String,
    },
    
    /// Pipe: expr |> func
    Pipe {
        value: Box<Spanned<Expr>>,
        func: Box<Spanned<Expr>>,
    },
    
    // === Latent Space Operations ===
    
    /// ls.collapse table namespace value
    Collapse {
        table: String,
        namespace: String,
        value: Box<Spanned<Expr>>,
    },
    
    /// ls.resolve handle_or_mode
    Resolve {
        target: Box<Spanned<Expr>>,
    },
    
    /// ls.snapshot
    Snapshot,
    
    /// ls.transaction { ... }
    Transaction {
        body: Vec<Spanned<Statement>>,
    },
    
    /// Handle reference: &h_xxx or ⟁xxx
    Handle(String),
    
    // === Contract Construction ===
    
    /// @14 { @0: val, @1: val, ... }
    Contract {
        id: ContractId,
        fields: Vec<(FieldIndex, Spanned<Expr>)>,
    },
}

/// Literal values
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Literal {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Array(Vec<Spanned<Expr>>),
    Object(Vec<(String, Spanned<Expr>)>),
}

impl From<Value> for Literal {
    fn from(v: Value) -> Self {
        match v {
            Value::Null => Literal::Null,
            Value::Boolean(b) => Literal::Bool(b),
            Value::Integer(i) => Literal::Int(i),
            Value::Float(f) => Literal::Float(f),
            Value::String(s) => Literal::String(s),
            Value::Array(arr) => Literal::Array(
                arr.iter()
                    .map(|v| Spanned::dummy(Expr::Literal(v.clone().into())))
                    .collect()
            ),
            Value::Object(obj) => Literal::Object(
                obj.iter()
                    .map(|(k, v)| (k.clone(), Spanned::dummy(Expr::Literal(v.clone().into()))))
                    .collect()
            ),
            Value::Contract(_c) => {
                // This shouldn't normally happen during literal conversion
                Literal::Object(vec![])
            }
            Value::Handle(h) => Literal::String(h), // Handles as strings
        }
    }
}

/// Binary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinOp {
    // Arithmetic
    Add,    // +
    Sub,    // -
    Mul,    // *
    Div,    // /
    Mod,    // %

    // Comparison
    Eq,     // ==
    Ne,     // !=
    Lt,     // <
    Le,     // <=
    Gt,     // >
    Ge,     // >=

    // Logical
    And,    // and, ∧
    Or,     // or, ∨
}

impl BinOp {
    /// Get the HLXL representation
    pub fn hlxl_str(&self) -> &'static str {
        match self {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Mod => "%",
            BinOp::Eq => "==",
            BinOp::Ne => "!=",
            BinOp::Lt => "<",
            BinOp::Le => "<=",
            BinOp::Gt => ">",
            BinOp::Ge => ">=",
            BinOp::And => "and",
            BinOp::Or => "or",
        }
    }

    /// Get the HLX (runic) representation
    pub fn runic_str(&self) -> &'static str {
        match self {
            BinOp::Add => "⊕",
            BinOp::Sub => "⊖",
            BinOp::Mul => "⊗",
            BinOp::Div => "⊘",
            BinOp::Mod => "⊙",
            BinOp::Eq => "⩵",
            BinOp::Ne => "≠",
            BinOp::Lt => "≪",
            BinOp::Le => "≤",
            BinOp::Gt => "≫",
            BinOp::Ge => "≥",
            BinOp::And => "∧",
            BinOp::Or => "∨",
        }
    }

    /// Get operator precedence (higher = binds tighter)
    pub fn precedence(&self) -> u8 {
        match self {
            BinOp::Or => 1,
            BinOp::And => 2,
            BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => 3,
            BinOp::Add | BinOp::Sub => 4,
            BinOp::Mul | BinOp::Div | BinOp::Mod => 5,
        }
    }
}

/// Unary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnaryOp {
    Neg,    // -
    Not,    // not, ¬
}

impl UnaryOp {
    pub fn hlxl_str(&self) -> &'static str {
        match self {
            UnaryOp::Neg => "-",
            UnaryOp::Not => "not",
        }
    }
    
    pub fn runic_str(&self) -> &'static str {
        match self {
            UnaryOp::Neg => "⊖",
            UnaryOp::Not => "¬",
        }
    }
}

// === Glyph Translation Tables ===

/// HLXL keyword to HLX glyph mapping
pub mod glyphs {
    // Structure
    pub const PROGRAM: &str = "⟠";
    pub const BLOCK: &str = "◇";
    pub const LET: &str = "⊢";
    pub const LOCAL: &str = "⊡";
    pub const ASSIGN: &str = "←";
    pub const RETURN: &str = "↩";
    pub const IF: &str = "❓";
    pub const ELSE: &str = "❗";
    pub const WHILE: &str = "⟳";
    pub const FOR: &str = "⟲";
    
    // Latent Space
    pub const COLLAPSE: &str = "⚳";
    pub const RESOLVE: &str = "⚯";
    pub const SNAPSHOT: &str = "⚶";
    pub const TRANSACTION: &str = "⚿";
    
    // Types
    pub const TYPE_NULL: &str = "Ⓝ";
    pub const TYPE_TRUE: &str = "Ⓣ";
    pub const TYPE_FALSE: &str = "Ⓕ";
    pub const TYPE_INT: &str = "ⓘ";
    pub const TYPE_FLOAT: &str = "ⓡ";
    pub const TYPE_STRING: &str = "ⓢ";
    pub const TYPE_ARRAY: &str = "ⓐ";
    pub const TYPE_OBJECT: &str = "ⓞ";
    pub const TYPE_CONTRACT: &str = "ⓒ";
    
    // Brackets
    pub const PAREN_OPEN: &str = "⟨";
    pub const PAREN_CLOSE: &str = "⟩";
    pub const ARRAY_OPEN: &str = "⌊";
    pub const ARRAY_CLOSE: &str = "⌋";
    pub const OBJ_OPEN: &str = "ⴾ";
    pub const OBJ_CLOSE: &str = "ⴿ";
    
    // Misc
    pub const PIPE: &str = "▷";
    pub const HANDLE_PREFIX: &str = "⟁";
    pub const BIND: &str = "✦";
    
    // LC markers (for wire format display)
    pub const LC_OBJ_START: &str = "🜊";
    pub const LC_FIELD_START: &str = "🜁";
    pub const LC_OBJ_END: &str = "🜂";
    pub const LC_ARR_START: &str = "🜃";
    pub const LC_ARR_END: &str = "🜄";
    pub const LC_HANDLE_REF: &str = "🜇";
    pub const LC_STREAM_END: &str = "🜋";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binop_precedence() {
        // Multiplication binds tighter than addition
        assert!(BinOp::Mul.precedence() > BinOp::Add.precedence());
        // And binds tighter than Or
        assert!(BinOp::And.precedence() > BinOp::Or.precedence());
    }

    #[test]
    fn test_glyph_roundtrip() {
        // Verify glyph strings are valid UTF-8
        assert!(!glyphs::PROGRAM.is_empty());
        assert!(!glyphs::COLLAPSE.is_empty());
    }
}
