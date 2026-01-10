//! Semantic Tokens Provider for HLX LSP
//!
//! Provides semantic syntax highlighting by walking the HLX AST and classifying tokens
//! based on their meaning (function vs variable, keyword vs identifier, etc.)

use tower_lsp::lsp_types::*;
use hlx_compiler::{HlxaParser, ast::*, parser::Parser};
use crate::symbol_index::SymbolIndex;
use crate::builtins::BuiltinRegistry;
use std::sync::Arc;

/// Token information before encoding
#[derive(Debug, Clone)]
struct TokenInfo {
    line: u32,         // 0-indexed (LSP format)
    start_char: u32,   // 0-indexed
    length: u32,
    token_type: u32,   // Index into token_types legend
    token_modifiers: u32,  // Bitfield of modifiers
}

/// Provides semantic tokens for HLX documents
pub struct SemanticTokensProvider {
    token_types: Vec<SemanticTokenType>,
    token_modifiers: Vec<SemanticTokenModifier>,
}

impl SemanticTokensProvider {
    pub fn new() -> Self {
        // Define token types in order (index = position in legend)
        let token_types = vec![
            SemanticTokenType::NAMESPACE,
            SemanticTokenType::TYPE,
            SemanticTokenType::CLASS,
            SemanticTokenType::ENUM,
            SemanticTokenType::INTERFACE,
            SemanticTokenType::STRUCT,
            SemanticTokenType::TYPE_PARAMETER,
            SemanticTokenType::PARAMETER,
            SemanticTokenType::VARIABLE,
            SemanticTokenType::PROPERTY,
            SemanticTokenType::ENUM_MEMBER,
            SemanticTokenType::EVENT,
            SemanticTokenType::FUNCTION,
            SemanticTokenType::METHOD,
            SemanticTokenType::MACRO,
            SemanticTokenType::KEYWORD,
            SemanticTokenType::MODIFIER,
            SemanticTokenType::COMMENT,
            SemanticTokenType::STRING,
            SemanticTokenType::NUMBER,
            SemanticTokenType::REGEXP,
            SemanticTokenType::OPERATOR,
        ];

        let token_modifiers = vec![
            SemanticTokenModifier::DECLARATION,
            SemanticTokenModifier::DEFINITION,
            SemanticTokenModifier::READONLY,
            SemanticTokenModifier::STATIC,
            SemanticTokenModifier::DEPRECATED,
            SemanticTokenModifier::ABSTRACT,
            SemanticTokenModifier::ASYNC,
            SemanticTokenModifier::MODIFICATION,
            SemanticTokenModifier::DOCUMENTATION,
            SemanticTokenModifier::DEFAULT_LIBRARY,
        ];

        Self { token_types, token_modifiers }
    }

    /// Get the semantic tokens legend for capability advertisement
    pub fn get_legend(&self) -> SemanticTokensLegend {
        SemanticTokensLegend {
            token_types: self.token_types.clone(),
            token_modifiers: self.token_modifiers.clone(),
        }
    }

    /// Generate semantic tokens for an entire document
    pub fn provide_semantic_tokens(
        &self,
        source: &str,
        symbol_index: &Arc<SymbolIndex>,
        builtin_registry: &Arc<BuiltinRegistry>,
        uri: &Url,
    ) -> Option<Vec<SemanticToken>> {
        let parser = HlxaParser::new();

        // Try to parse the document
        match parser.parse(source) {
            Ok(program) => {
                // Walk AST and collect tokens
                let mut walker = ASTWalker::new(source, symbol_index, builtin_registry, uri);
                walker.visit_program(&program);
                Some(walker.encode_tokens())
            }
            Err(_) => {
                // Parse failed - provide fallback keyword highlighting
                let mut walker = ASTWalker::new(source, symbol_index, builtin_registry, uri);
                walker.fallback_tokenize(source);
                Some(walker.encode_tokens())
            }
        }
    }

    /// Generate semantic tokens for a specific range (viewport optimization)
    pub fn provide_semantic_tokens_range(
        &self,
        source: &str,
        range: Range,
        symbol_index: &Arc<SymbolIndex>,
        builtin_registry: &Arc<BuiltinRegistry>,
        uri: &Url,
    ) -> Option<Vec<SemanticToken>> {
        // For now, generate full tokens and filter
        // TODO: Optimize to only walk AST nodes within range
        let tokens = self.provide_semantic_tokens(source, symbol_index, builtin_registry, uri)?;

        // Filter tokens within range
        let filtered: Vec<SemanticToken> = tokens.into_iter()
            .filter(|token| {
                // TODO: Convert delta encoding back to absolute positions for filtering
                // For now, return all tokens (works but not optimal)
                true
            })
            .collect();

        Some(filtered)
    }

    // Helper: Get token type index
    fn token_type_index(&self, token_type: &SemanticTokenType) -> u32 {
        self.token_types.iter()
            .position(|t| t == token_type)
            .unwrap_or(0) as u32
    }

    // Helper: Get modifier bit flag
    fn modifier_bitflag(&self, modifier: &SemanticTokenModifier) -> u32 {
        self.token_modifiers.iter()
            .position(|m| m == modifier)
            .map(|idx| 1 << idx)
            .unwrap_or(0)
    }
}

/// AST walker that collects semantic tokens
struct ASTWalker<'a> {
    tokens: Vec<TokenInfo>,
    #[allow(dead_code)]
    symbol_index: &'a SymbolIndex,
    builtin_registry: &'a BuiltinRegistry,
    #[allow(dead_code)]
    uri: &'a Url,
    #[allow(dead_code)]
    source: &'a str,
    current_function: Option<String>,  // Track current function for parameter identification
}

impl<'a> ASTWalker<'a> {
    fn new(
        source: &'a str,
        symbol_index: &'a SymbolIndex,
        builtin_registry: &'a BuiltinRegistry,
        uri: &'a Url,
    ) -> Self {
        Self {
            tokens: Vec::new(),
            symbol_index,
            builtin_registry,
            uri,
            source,
            current_function: None,
        }
    }

    /// Add a token to the list
    fn add_token(&mut self, span: Span, token_type: SemanticTokenType, modifiers: u32) {
        // Convert 1-indexed span to 0-indexed LSP position
        let line = if span.line > 0 { span.line - 1 } else { 0 };
        let start_char = if span.col > 0 { span.col - 1 } else { 0 };
        let length = (span.end - span.start) as u32;

        let token_type_idx = Self::type_to_index(&token_type);

        self.tokens.push(TokenInfo {
            line,
            start_char,
            length,
            token_type: token_type_idx,
            token_modifiers: modifiers,
        });
    }

    /// Convert token type to index
    fn type_to_index(token_type: &SemanticTokenType) -> u32 {
        match token_type {
            t if *t == SemanticTokenType::NAMESPACE => 0,
            t if *t == SemanticTokenType::TYPE => 1,
            t if *t == SemanticTokenType::CLASS => 2,
            t if *t == SemanticTokenType::ENUM => 3,
            t if *t == SemanticTokenType::INTERFACE => 4,
            t if *t == SemanticTokenType::STRUCT => 5,
            t if *t == SemanticTokenType::TYPE_PARAMETER => 6,
            t if *t == SemanticTokenType::PARAMETER => 7,
            t if *t == SemanticTokenType::VARIABLE => 8,
            t if *t == SemanticTokenType::PROPERTY => 9,
            t if *t == SemanticTokenType::ENUM_MEMBER => 10,
            t if *t == SemanticTokenType::EVENT => 11,
            t if *t == SemanticTokenType::FUNCTION => 12,
            t if *t == SemanticTokenType::METHOD => 13,
            t if *t == SemanticTokenType::MACRO => 14,
            t if *t == SemanticTokenType::KEYWORD => 15,
            t if *t == SemanticTokenType::MODIFIER => 16,
            t if *t == SemanticTokenType::COMMENT => 17,
            t if *t == SemanticTokenType::STRING => 18,
            t if *t == SemanticTokenType::NUMBER => 19,
            t if *t == SemanticTokenType::REGEXP => 20,
            t if *t == SemanticTokenType::OPERATOR => 21,
            _ => 0,
        }
    }

    /// Convert modifier to bitflag
    fn modifier_to_bitflag(modifier: &SemanticTokenModifier) -> u32 {
        match modifier {
            m if *m == SemanticTokenModifier::DECLARATION => 1 << 0,
            m if *m == SemanticTokenModifier::DEFINITION => 1 << 1,
            m if *m == SemanticTokenModifier::READONLY => 1 << 2,
            m if *m == SemanticTokenModifier::STATIC => 1 << 3,
            m if *m == SemanticTokenModifier::DEPRECATED => 1 << 4,
            m if *m == SemanticTokenModifier::ABSTRACT => 1 << 5,
            m if *m == SemanticTokenModifier::ASYNC => 1 << 6,
            m if *m == SemanticTokenModifier::MODIFICATION => 1 << 7,
            m if *m == SemanticTokenModifier::DOCUMENTATION => 1 << 8,
            m if *m == SemanticTokenModifier::DEFAULT_LIBRARY => 1 << 9,
            _ => 0,
        }
    }

    /// Visit the top-level program
    fn visit_program(&mut self, program: &Program) {
        // Highlight "program" keyword (we don't have its span, skip for now)

        // Visit each block (function)
        for block in &program.blocks {
            self.visit_block(block);
        }
    }

    /// Visit a block (function definition)
    fn visit_block(&mut self, block: &Block) {
        // Highlight "fn" keyword (TODO: need keyword spans from parser)

        // Highlight function name as function declaration
        // TODO: We don't have the span for the function name itself
        // For now, we'll rely on symbol_index to provide this

        self.current_function = Some(block.name.clone());

        // Highlight parameters
        for (param_name, type_annotation) in &block.params {
            // TODO: Need param name spans from parser
            // For now, skip - we'll get these from symbol_index

            // Highlight type annotation if present
            if let Some(typ) = type_annotation {
                // TODO: Need type span
            }
        }

        // Highlight return type if present
        if let Some(return_type) = &block.return_type {
            // TODO: Need return type span
        }

        // Visit statements in function body
        for item in &block.items {
            match &item.node {
                Item::Statement(stmt) => self.visit_statement(stmt, item.span),
                Item::Node(_) => {} // Netlist nodes not commonly used in HLX-A
            }
        }

        self.current_function = None;
    }

    /// Visit a statement
    fn visit_statement(&mut self, stmt: &Statement, span: Span) {
        match stmt {
            Statement::Let { name, type_annotation, value } => {
                // Highlight "let" keyword (TODO: need keyword span)

                // Highlight variable name as variable declaration
                // TODO: Need variable name span

                // Highlight type annotation if present
                if let Some(typ) = type_annotation {
                    // TODO: Need type span
                }

                // Visit the value expression
                self.visit_expr(&value.node, value.span);
            }

            Statement::Return { value } => {
                // Highlight "return" keyword (TODO: need keyword span)
                self.visit_expr(&value.node, value.span);
            }

            Statement::If { condition, then_branch, else_branch } => {
                // Highlight "if" keyword (TODO)
                self.visit_expr(&condition.node, condition.span);

                for stmt in then_branch {
                    self.visit_statement(&stmt.node, stmt.span);
                }

                if let Some(else_stmts) = else_branch {
                    // Highlight "else" keyword (TODO)
                    for stmt in else_stmts {
                        self.visit_statement(&stmt.node, stmt.span);
                    }
                }
            }

            Statement::While { condition, body, .. } => {
                // Highlight "loop" keyword (TODO)
                self.visit_expr(&condition.node, condition.span);
                for stmt in body {
                    self.visit_statement(&stmt.node, stmt.span);
                }
            }

            Statement::Expr(expr) => {
                self.visit_expr(&expr.node, expr.span);
            }

            _ => {}
        }
    }

    /// Visit an expression
    fn visit_expr(&mut self, expr: &Expr, span: Span) {
        match expr {
            Expr::Literal(lit) => {
                match lit {
                    Literal::String(_) => {
                        self.add_token(span, SemanticTokenType::STRING, 0);
                    }
                    Literal::Int(_) | Literal::Float(_) => {
                        self.add_token(span, SemanticTokenType::NUMBER, 0);
                    }
                    Literal::Bool(_) => {
                        // Booleans are keywords (true/false)
                        self.add_token(span, SemanticTokenType::KEYWORD, 0);
                    }
                    Literal::Null => {
                        self.add_token(span, SemanticTokenType::KEYWORD, 0);
                    }
                    _ => {}  // Array and Object literals handled elsewhere
                }
            }

            Expr::Ident(name) => {
                // Check if it's a builtin function
                if self.builtin_registry.exists(name) {
                    let modifiers = Self::modifier_to_bitflag(&SemanticTokenModifier::DEFAULT_LIBRARY);
                    self.add_token(span, SemanticTokenType::FUNCTION, modifiers);
                } else {
                    // Check symbol index to determine type
                    // For now, default to variable
                    self.add_token(span, SemanticTokenType::VARIABLE, 0);
                }
            }

            Expr::Array(elements) => {
                for elem in elements {
                    self.visit_expr(&elem.node, elem.span);
                }
            }

            Expr::Object(fields) => {
                for (field_name, value) in fields {
                    // Highlight field name as property
                    // TODO: Need field name span
                    self.visit_expr(&value.node, value.span);
                }
            }

            Expr::BinOp { op, lhs, rhs } => {
                // Highlight operator (TODO: need operator span)
                self.visit_expr(&lhs.node, lhs.span);
                self.visit_expr(&rhs.node, rhs.span);
            }

            Expr::UnaryOp { op, operand } => {
                // Highlight operator (TODO: need operator span)
                self.visit_expr(&operand.node, operand.span);
            }

            Expr::Call { func, args } => {
                // The function is highlighted in visit_expr for the func node
                self.visit_expr(&func.node, func.span);
                for arg in args {
                    self.visit_expr(&arg.node, arg.span);
                }
            }

            Expr::Index { object, index } => {
                self.visit_expr(&object.node, object.span);
                self.visit_expr(&index.node, index.span);
            }

            Expr::Field { object, field } => {
                self.visit_expr(&object.node, object.span);
                // Highlight field name as property (TODO: need field span)
            }

            _ => {}
        }
    }

    /// Fallback tokenization when parse fails (keyword-only highlighting)
    fn fallback_tokenize(&mut self, source: &str) {
        // Simple keyword highlighting using regex
        let keywords = ["fn", "let", "return", "if", "else", "loop", "break", "continue", "program", "block", "true", "false", "null"];

        for (line_idx, line) in source.lines().enumerate() {
            for keyword in &keywords {
                if let Some(pos) = line.find(keyword) {
                    // Check if it's a whole word
                    let is_word_boundary = |c: char| !c.is_alphanumeric() && c != '_';
                    let before_ok = pos == 0 || line.chars().nth(pos - 1).map_or(true, is_word_boundary);
                    let after_ok = pos + keyword.len() >= line.len()
                        || line.chars().nth(pos + keyword.len()).map_or(true, is_word_boundary);

                    if before_ok && after_ok {
                        self.tokens.push(TokenInfo {
                            line: line_idx as u32,
                            start_char: pos as u32,
                            length: keyword.len() as u32,
                            token_type: Self::type_to_index(&SemanticTokenType::KEYWORD),
                            token_modifiers: 0,
                        });
                    }
                }
            }
        }
    }

    /// Encode tokens in LSP delta format and return
    fn encode_tokens(mut self) -> Vec<SemanticToken> {
        // Sort tokens by position (line, then char)
        self.tokens.sort_by(|a, b| {
            a.line.cmp(&b.line).then(a.start_char.cmp(&b.start_char))
        });

        let mut encoded = Vec::new();
        let mut prev_line = 0;
        let mut prev_start = 0;

        for token in self.tokens {
            let delta_line = token.line - prev_line;
            let delta_start = if delta_line == 0 {
                token.start_char - prev_start
            } else {
                token.start_char
            };

            encoded.push(SemanticToken {
                delta_line,
                delta_start,
                length: token.length,
                token_type: token.token_type,
                token_modifiers_bitset: token.token_modifiers,
            });

            prev_line = token.line;
            prev_start = token.start_char;
        }

        encoded
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_encoding_single() {
        let mut walker = ASTWalker {
            tokens: vec![
                TokenInfo {
                    line: 0,
                    start_char: 0,
                    length: 2,
                    token_type: 15, // keyword
                    token_modifiers: 0,
                }
            ],
            symbol_index: &SymbolIndex::new(),
            builtin_registry: &BuiltinRegistry::new(),
            uri: &Url::parse("file:///test.hlxa").unwrap(),
            source: "fn main() {}",
            current_function: None,
        };

        let encoded = walker.encode_tokens();
        assert_eq!(encoded.len(), 1);
        assert_eq!(encoded[0].delta_line, 0);
        assert_eq!(encoded[0].delta_start, 0);
        assert_eq!(encoded[0].length, 2);
    }

    #[test]
    fn test_token_encoding_multiple_lines() {
        let mut walker = ASTWalker {
            tokens: vec![
                TokenInfo { line: 0, start_char: 0, length: 2, token_type: 15, token_modifiers: 0 },
                TokenInfo { line: 1, start_char: 4, length: 3, token_type: 15, token_modifiers: 0 },
            ],
            symbol_index: &SymbolIndex::new(),
            builtin_registry: &BuiltinRegistry::new(),
            uri: &Url::parse("file:///test.hlxa").unwrap(),
            source: "fn\n    let",
            current_function: None,
        };

        let encoded = walker.encode_tokens();
        assert_eq!(encoded.len(), 2);
        assert_eq!(encoded[0].delta_line, 0);
        assert_eq!(encoded[0].delta_start, 0);
        assert_eq!(encoded[1].delta_line, 1); // 1 line down
        assert_eq!(encoded[1].delta_start, 4); // absolute position on new line
    }
}
