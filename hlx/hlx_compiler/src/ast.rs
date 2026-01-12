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
    pub modules: Vec<Module>,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Module {
    pub name: String,
    pub capabilities: Vec<String>,
    pub constants: Vec<Constant>,
    pub structs: Vec<StructDef>,
    pub enums: Vec<EnumDef>,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Constant {
    pub name: String,
    pub typ: Type,
    pub value: Spanned<Expr>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<(String, Type)>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnumDef {
    pub name: String,
    pub variants: Vec<String>,
}

/// A named block (function-like)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Block {
    pub name: String,
    /// Attributes like #[no_mangle], #[entry], etc.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub attributes: Vec<String>,
    /// Span of the function name identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_span: Option<Span>,
    /// Span of the "fn" keyword (if present)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fn_keyword_span: Option<Span>,
    /// Parameters: (name, name_span, optional (type, type_span))
    pub params: Vec<(String, Option<Span>, Option<(Type, Option<Span>)>)>,
    pub return_type: Option<Type>,
    /// Span of the return type annotation (if present)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_type_span: Option<Span>,
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

/// Type annotations
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Type {
    /// int
    Int,
    /// float
    Float,
    /// string
    String,
    /// bool
    Bool,
    /// Array of specific type
    Array(Box<Type>),
    /// Named type (for future use)
    Named(String),
}

impl Type {
    /// Parse a type string like "Array<Float>" into a Type
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "Int" | "int" => Some(Type::Int),
            "Float" | "float" => Some(Type::Float),
            "String" | "string" => Some(Type::String),
            "Bool" | "bool" => Some(Type::Bool),
            _ if s.starts_with("Array<") && s.ends_with('>') => {
                let inner = &s[6..s.len()-1];
                let inner_type = Self::parse(inner)?;
                Some(Type::Array(Box::new(inner_type)))
            }
            _ => Some(Type::Named(s.to_string())),
        }
    }

    /// Convert Type back to string representation
    pub fn to_string(&self) -> String {
        match self {
            Type::Int => "Int".to_string(),
            Type::Float => "Float".to_string(),
            Type::String => "String".to_string(),
            Type::Bool => "Bool".to_string(),
            Type::Array(inner) => format!("Array<{}>", inner.to_string()),
            Type::Named(name) => name.clone(),
        }
    }
}

/// Statements
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Statement {
    /// let x = expr or let x: Type = expr
    Let {
        /// Span of "let" keyword
        #[serde(skip_serializing_if = "Option::is_none")]
        keyword_span: Option<Span>,
        name: String,
        /// Span of the variable name
        #[serde(skip_serializing_if = "Option::is_none")]
        name_span: Option<Span>,
        type_annotation: Option<Type>,
        /// Span of the type annotation (if present)
        #[serde(skip_serializing_if = "Option::is_none")]
        type_span: Option<Span>,
        value: Spanned<Expr>,
    },

    /// local x = expr (block-scoped)
    Local {
        /// Span of "local" keyword
        #[serde(skip_serializing_if = "Option::is_none")]
        keyword_span: Option<Span>,
        name: String,
        /// Span of the variable name
        #[serde(skip_serializing_if = "Option::is_none")]
        name_span: Option<Span>,
        value: Spanned<Expr>,
    },

    /// x = expr (reassignment) or arr[i] = expr
    Assign {
        lhs: Spanned<Expr>,
        value: Spanned<Expr>,
    },

    /// return expr
    Return {
        /// Span of "return" keyword
        #[serde(skip_serializing_if = "Option::is_none")]
        keyword_span: Option<Span>,
        value: Spanned<Expr>,
    },

    If {
        /// Span of "if" keyword
        #[serde(skip_serializing_if = "Option::is_none")]
        if_keyword_span: Option<Span>,
        condition: Spanned<Expr>,
        then_branch: Vec<Spanned<Statement>>,
        /// Span of "else" keyword (if present)
        #[serde(skip_serializing_if = "Option::is_none")]
        else_keyword_span: Option<Span>,
        else_branch: Option<Vec<Spanned<Statement>>>,
    },
    While {
        /// Span of "loop" keyword
        #[serde(skip_serializing_if = "Option::is_none")]
        loop_keyword_span: Option<Span>,
        condition: Spanned<Expr>,
        body: Vec<Spanned<Statement>>,
        max_iter: u32,
    },
    Break,
    Continue,
    Expr(Spanned<Expr>),
    /// Inline assembly: asm("template" : outputs : inputs : clobbers)
    Asm {
        template: String,
        outputs: Vec<(String, String)>,  // (constraint, variable)
        inputs: Vec<(String, Spanned<Expr>)>,  // (constraint, expression)
        clobbers: Vec<String>,
    },
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
    
    /// Type cast: expr as Type
    Cast {
        expr: Box<Spanned<Expr>>,
        target_type: Type,
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

    // Bitwise
    BitAnd, // &
    BitOr,  // |
    BitXor, // ^
    Shl,    // <<
    Shr,    // >>
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
            BinOp::BitAnd => "&",
            BinOp::BitOr => "|",
            BinOp::BitXor => "^",
            BinOp::Shl => "<<",
            BinOp::Shr => ">>",
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
            BinOp::BitAnd => "&",
            BinOp::BitOr => "|",
            BinOp::BitXor => "⊕",
            BinOp::Shl => "≪",
            BinOp::Shr => "≫",
        }
    }

    /// Get operator precedence (higher = binds tighter)
    pub fn precedence(&self) -> u8 {
        match self {
            BinOp::Or => 1,
            BinOp::And => 2,
            BinOp::BitOr => 3,
            BinOp::BitXor => 4,
            BinOp::BitAnd => 5,
            BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => 6,
            BinOp::Shl | BinOp::Shr => 7,
            BinOp::Add | BinOp::Sub => 8,
            BinOp::Mul | BinOp::Div | BinOp::Mod => 9,
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
