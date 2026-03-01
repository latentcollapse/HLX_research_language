pub mod token;

use token::{Span, Token, TokenKind};
use crate::error::{AxiomError, AxiomResult, ErrorKind};

pub struct Lexer {
    source: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Lexer {
            source: source.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    pub fn tokenize(&mut self) -> AxiomResult<Vec<Token>> {
        let mut tokens = Vec::new();
        loop {
            self.skip_whitespace_and_comments();
            if self.is_at_end() {
                tokens.push(Token {
                    kind: TokenKind::Eof,
                    span: self.make_span(self.pos, self.pos),
                    lexeme: String::new(),
                });
                break;
            }
            let token = self.next_token()?;
            tokens.push(token);
        }
        Ok(tokens)
    }

    fn next_token(&mut self) -> AxiomResult<Token> {
        let start = self.pos;
        let start_line = self.line;
        let start_col = self.col;
        let ch = self.advance();

        let kind = match ch {
            // Single-char tokens
            '(' => TokenKind::LParen,
            ')' => TokenKind::RParen,
            '{' => TokenKind::LBrace,
            '}' => TokenKind::RBrace,
            '[' => TokenKind::LBracket,
            ']' => TokenKind::RBracket,
            ',' => TokenKind::Comma,
            ':' => TokenKind::Colon,
            ';' => TokenKind::Semicolon,
            '.' => TokenKind::Dot,
            '@' => TokenKind::At,
            '?' => TokenKind::Question,
            '%' => TokenKind::Percent,

            // Two-char operators
            '+' => {
                if self.match_char('=') {
                    TokenKind::PlusEq
                } else {
                    TokenKind::Plus
                }
            }
            '*' => TokenKind::Star,
            '/' => TokenKind::Slash,
            '!' => {
                if self.match_char('=') {
                    TokenKind::NotEq
                } else {
                    TokenKind::Bang
                }
            }
            '=' => {
                if self.match_char('=') {
                    TokenKind::EqEq
                } else if self.match_char('>') {
                    TokenKind::FatArrow
                } else {
                    TokenKind::Eq
                }
            }
            '<' => {
                if self.match_char('=') {
                    TokenKind::LtEq
                } else {
                    TokenKind::Lt
                }
            }
            '>' => {
                if self.match_char('=') {
                    TokenKind::GtEq
                } else if self.match_char('>') {
                    TokenKind::Compose
                } else {
                    TokenKind::Gt
                }
            }
            '&' => {
                if self.match_char('&') {
                    TokenKind::AndAnd
                } else {
                    return Err(self.error(start, "Expected '&&'"));
                }
            }
            '|' => {
                if self.match_char('>') {
                    TokenKind::Pipeline
                } else if self.match_char('|') {
                    TokenKind::OrOr
                } else {
                    return Err(self.error(start, "Expected '|>' or '||'"));
                }
            }
            '-' => {
                if self.match_char('>') {
                    TokenKind::Arrow
                } else if self.match_char('=') {
                    TokenKind::MinusEq
                } else {
                    TokenKind::Minus
                }
            }

            // Pragma / Hash
            '#' => self.lex_pragma(start)?,

            // String literals
            '"' => self.lex_string(start)?,

            // Numbers
            c if c.is_ascii_digit() => self.lex_number(start)?,

            // Identifiers and keywords
            c if c.is_alphabetic() || c == '_' => self.lex_identifier(start)?,

            _ => return Err(self.error(start, &format!("Unexpected character: '{}'", ch))),
        };

        let lexeme: String = self.source[start..self.pos].iter().collect();
        Ok(Token {
            kind,
            span: Span {
                start,
                end: self.pos,
                line: start_line,
                col: start_col,
            },
            lexeme,
        })
    }

    fn lex_pragma(&mut self, _start: usize) -> AxiomResult<TokenKind> {
        // Read the pragma name
        let word_start = self.pos;
        while !self.is_at_end() && self.peek().is_alphanumeric() {
            self.advance();
        }
        let word: String = self.source[word_start..self.pos].iter().collect();
        match word.as_str() {
            "flow" => Ok(TokenKind::PragmaFlow),
            "guard" => Ok(TokenKind::PragmaGuard),
            "arx" => Ok(TokenKind::PragmaArx),
            // Backward compatibility (deprecated)
            "shield" => Ok(TokenKind::PragmaGuard),
            "fortress" => Ok(TokenKind::PragmaArx),
            "explicit" => Ok(TokenKind::PragmaExplicit),
            "kernel" => Ok(TokenKind::PragmaKernel),
            _ => Ok(TokenKind::Hash), // bare # for attributes
        }
    }

    fn lex_string(&mut self, start: usize) -> AxiomResult<TokenKind> {
        let mut value = String::new();
        while !self.is_at_end() && self.peek() != '"' {
            if self.peek() == '\\' {
                self.advance();
                if self.is_at_end() {
                    return Err(self.error(start, "Unterminated string"));
                }
                match self.advance() {
                    'n' => value.push('\n'),
                    't' => value.push('\t'),
                    'r' => value.push('\r'),
                    '\\' => value.push('\\'),
                    '"' => value.push('"'),
                    c => {
                        value.push('\\');
                        value.push(c);
                    }
                }
            } else {
                if self.peek() == '\n' {
                    self.line += 1;
                    self.col = 0;
                }
                value.push(self.advance());
            }
        }
        if self.is_at_end() {
            return Err(self.error(start, "Unterminated string"));
        }
        self.advance(); // closing "
        Ok(TokenKind::StringLiteral(value))
    }

    fn lex_number(&mut self, start: usize) -> AxiomResult<TokenKind> {
        while !self.is_at_end() && self.peek().is_ascii_digit() {
            self.advance();
        }
        // Check for float
        if !self.is_at_end() && self.peek() == '.' && self.peek_next().is_some_and(|c| c.is_ascii_digit()) {
            self.advance(); // consume '.'
            while !self.is_at_end() && self.peek().is_ascii_digit() {
                self.advance();
            }
            let text: String = self.source[start..self.pos].iter().collect();
            let value: f64 = text.parse().map_err(|_| self.error(start, "Invalid float literal"))?;
            Ok(TokenKind::FloatLiteral(value))
        } else {
            let text: String = self.source[start..self.pos].iter().collect();
            let value: i64 = text.parse().map_err(|_| self.error(start, "Invalid integer literal"))?;
            Ok(TokenKind::IntLiteral(value))
        }
    }

    fn lex_identifier(&mut self, start: usize) -> AxiomResult<TokenKind> {
        while !self.is_at_end() && (self.peek().is_alphanumeric() || self.peek() == '_') {
            self.advance();
        }
        let word: String = self.source[start..self.pos].iter().collect();
        // DoS prevention: cap identifier length at 255 characters
        if word.len() > 255 {
            return Err(self.error(start, &format!(
                "Identifier '{}...' exceeds maximum length of 255 characters",
                &word[..32]
            )));
        }
        Ok(match word.as_str() {
            // Declaration keywords
            "module" => TokenKind::Module,
            "fn" => TokenKind::Fn,
            "export" => TokenKind::Export,
            "import" => TokenKind::Import,
            "contract" => TokenKind::Contract,
            "tensor_op" => TokenKind::TensorOp,
            "intent" => TokenKind::Intent,
            "enum" => TokenKind::Enum,
            "let" => TokenKind::Let,

            // Intent system
            "do" => TokenKind::Do,
            "query_conscience" => TokenKind::QueryConscience,
            "declare_anomaly" => TokenKind::DeclareAnomaly,

            // Control flow
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "match" => TokenKind::Match,
            "loop" => TokenKind::Loop,
            "return" => TokenKind::Return,
        "break" => TokenKind::Break,
        "continue" => TokenKind::Continue,

            // Memory
            "collapse" => TokenKind::Collapse,
            "resolve" => TokenKind::Resolve,

            // Trust
            "verify" => TokenKind::Verify,

            // SCALE
            "scale" => TokenKind::Scale,
            "barrier" => TokenKind::Barrier,
            "claim_task" => TokenKind::ClaimTask,

            // Self-modification
            "self_mod" => TokenKind::SelfMod,
            "delta_proof" => TokenKind::DeltaProof,

            // Intent clause keywords
            "takes" => TokenKind::Takes,
            "gives" => TokenKind::Gives,
            "pre" => TokenKind::Pre,
            "post" => TokenKind::Post,
            "bound" => TokenKind::Bound,
            "effect" => TokenKind::Effect,
            "conscience" => TokenKind::Conscience,
            "fallback" => TokenKind::Fallback,
            "rollback" => TokenKind::Rollback,
            "trace" => TokenKind::Trace,
            "ring" => TokenKind::Ring,

            // Booleans
            "true" => TokenKind::True,
            "false" => TokenKind::False,

            // Underscore pattern
            "_" => TokenKind::Underscore,

            // Everything else is an identifier
            _ => TokenKind::Ident(word),
        })
    }

    // --- Helper methods ---

    fn advance(&mut self) -> char {
        let ch = self.source[self.pos];
        self.pos += 1;
        if ch == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        ch
    }

    fn peek(&self) -> char {
        if self.is_at_end() {
            '\0'
        } else {
            self.source[self.pos]
        }
    }

    fn peek_next(&self) -> Option<char> {
        if self.pos + 1 < self.source.len() {
            Some(self.source[self.pos + 1])
        } else {
            None
        }
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.is_at_end() || self.source[self.pos] != expected {
            false
        } else {
            self.pos += 1;
            self.col += 1;
            true
        }
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.source.len()
    }

    fn skip_whitespace_and_comments(&mut self) {
        while !self.is_at_end() {
            match self.peek() {
                ' ' | '\t' | '\r' | '\n' => {
                    self.advance();
                }
                '/' if self.peek_next() == Some('/') => {
                    // Line comment
                    while !self.is_at_end() && self.peek() != '\n' {
                        self.advance();
                    }
                }
                _ => break,
            }
        }
    }

    fn make_span(&self, start: usize, end: usize) -> Span {
        Span {
            start,
            end,
            line: self.line,
            col: self.col,
        }
    }

    fn error(&self, start: usize, message: &str) -> AxiomError {
        AxiomError {
            kind: ErrorKind::UnexpectedChar,
            message: message.to_string(),
            span: Some(self.make_span(start, self.pos)),
        }
    }
}
