/// Source location for error reporting
#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub col: usize,
}

/// A token with its kind and source location
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
    pub lexeme: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    IntLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(String),
    True,
    False,

    // Identifier
    Ident(String),

    // Declaration keywords
    Module,
    Fn,
    Export,
    Import,
    Contract,
    TensorOp,
    Intent,
    Enum,
    Let,

    // Intent system keywords
    Do,
    QueryConscience,
    DeclareAnomaly,

    // Control flow keywords
    If,
    Else,
    Match,
    Loop,
    Return,
    Break,
    Continue,

    // Memory keywords
    Collapse,
    Resolve,

    // Trust keywords
    Verify,

    // SCALE keywords
    Scale,
    Barrier,
    ClaimTask,

    // Self-modification keywords
    SelfMod,
    DeltaProof,

    // Intent clause keywords
    Takes,
    Gives,
    Pre,
    Post,
    Bound,
    Effect,
    Conscience,
    Fallback,
    Rollback,
    Trace,
    Ring,

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    EqEq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    AndAnd,
    OrOr,
    Bang,
    Eq,
    PlusEq,
    MinusEq,
    Pipeline,  // |>
    Compose,   // >>
    Arrow,     // ->
    FatArrow,  // =>

    // Delimiters
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Colon,
    Semicolon,
    Dot,
    At,
    Hash,
    Question,
    Underscore,

    // Pragma modes
    PragmaFlow,
    PragmaGuard,
    PragmaArx,
    PragmaExplicit,
    PragmaKernel,

    // End of file
    Eof,
}

impl TokenKind {
    /// Check if this token is a keyword that can start a declaration
    pub fn is_declaration_start(&self) -> bool {
        matches!(
            self,
            TokenKind::Module
                | TokenKind::Fn
                | TokenKind::Export
                | TokenKind::Contract
                | TokenKind::Intent
                | TokenKind::Enum
                | TokenKind::TensorOp
                | TokenKind::Import
        )
    }
}

impl std::fmt::Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenKind::IntLiteral(n) => write!(f, "{}", n),
            TokenKind::FloatLiteral(n) => write!(f, "{}", n),
            TokenKind::StringLiteral(s) => write!(f, "\"{}\"", s),
            TokenKind::True => write!(f, "true"),
            TokenKind::False => write!(f, "false"),
            TokenKind::Ident(s) => write!(f, "{}", s),
            TokenKind::Module => write!(f, "module"),
            TokenKind::Fn => write!(f, "fn"),
            TokenKind::Export => write!(f, "export"),
            TokenKind::Import => write!(f, "import"),
            TokenKind::Contract => write!(f, "contract"),
            TokenKind::TensorOp => write!(f, "tensor_op"),
            TokenKind::Intent => write!(f, "intent"),
            TokenKind::Enum => write!(f, "enum"),
            TokenKind::Let => write!(f, "let"),
            TokenKind::Do => write!(f, "do"),
            TokenKind::QueryConscience => write!(f, "query_conscience"),
            TokenKind::DeclareAnomaly => write!(f, "declare_anomaly"),
            TokenKind::If => write!(f, "if"),
            TokenKind::Else => write!(f, "else"),
            TokenKind::Match => write!(f, "match"),
            TokenKind::Loop => write!(f, "loop"),
            TokenKind::Return => write!(f, "return"),
            TokenKind::Break => write!(f, "break"),
            TokenKind::Continue => write!(f, "continue"),
            TokenKind::Collapse => write!(f, "collapse"),
            TokenKind::Resolve => write!(f, "resolve"),
            TokenKind::Verify => write!(f, "verify"),
            TokenKind::Scale => write!(f, "scale"),
            TokenKind::Barrier => write!(f, "barrier"),
            TokenKind::ClaimTask => write!(f, "claim_task"),
            TokenKind::SelfMod => write!(f, "self_mod"),
            TokenKind::DeltaProof => write!(f, "delta_proof"),
            TokenKind::Takes => write!(f, "takes"),
            TokenKind::Gives => write!(f, "gives"),
            TokenKind::Pre => write!(f, "pre"),
            TokenKind::Post => write!(f, "post"),
            TokenKind::Bound => write!(f, "bound"),
            TokenKind::Effect => write!(f, "effect"),
            TokenKind::Conscience => write!(f, "conscience"),
            TokenKind::Fallback => write!(f, "fallback"),
            TokenKind::Rollback => write!(f, "rollback"),
            TokenKind::Trace => write!(f, "trace"),
            TokenKind::Ring => write!(f, "ring"),
            TokenKind::Plus => write!(f, "+"),
            TokenKind::Minus => write!(f, "-"),
            TokenKind::Star => write!(f, "*"),
            TokenKind::Slash => write!(f, "/"),
            TokenKind::Percent => write!(f, "%"),
            TokenKind::EqEq => write!(f, "=="),
            TokenKind::NotEq => write!(f, "!="),
            TokenKind::Lt => write!(f, "<"),
            TokenKind::Gt => write!(f, ">"),
            TokenKind::LtEq => write!(f, "<="),
            TokenKind::GtEq => write!(f, ">="),
            TokenKind::AndAnd => write!(f, "&&"),
            TokenKind::OrOr => write!(f, "||"),
            TokenKind::Bang => write!(f, "!"),
            TokenKind::Eq => write!(f, "="),
            TokenKind::PlusEq => write!(f, "+="),
            TokenKind::MinusEq => write!(f, "-="),
            TokenKind::Pipeline => write!(f, "|>"),
            TokenKind::Compose => write!(f, ">>"),
            TokenKind::Arrow => write!(f, "->"),
            TokenKind::FatArrow => write!(f, "=>"),
            TokenKind::LParen => write!(f, "("),
            TokenKind::RParen => write!(f, ")"),
            TokenKind::LBrace => write!(f, "{{"),
            TokenKind::RBrace => write!(f, "}}"),
            TokenKind::LBracket => write!(f, "["),
            TokenKind::RBracket => write!(f, "]"),
            TokenKind::Comma => write!(f, ","),
            TokenKind::Colon => write!(f, ":"),
            TokenKind::Semicolon => write!(f, ";"),
            TokenKind::Dot => write!(f, "."),
            TokenKind::At => write!(f, "@"),
            TokenKind::Hash => write!(f, "#"),
            TokenKind::Question => write!(f, "?"),
            TokenKind::Underscore => write!(f, "_"),
            TokenKind::PragmaFlow => write!(f, "#flow"),
            TokenKind::PragmaGuard => write!(f, "#guard"),
            TokenKind::PragmaArx => write!(f, "#arx"),
            TokenKind::PragmaExplicit => write!(f, "#explicit"),
            TokenKind::PragmaKernel => write!(f, "#kernel"),
            TokenKind::Eof => write!(f, "EOF"),
        }
    }
}
