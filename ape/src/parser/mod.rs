pub mod ast;

use ast::*;
use crate::lexer::token::{Token, TokenKind, Span};
use crate::error::{AxiomError, AxiomResult, ErrorKind};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    pub fn parse_program(&mut self) -> AxiomResult<Program> {
        let module = self.parse_module()?;
        Ok(Program { module })
    }

    // --- Module ---

    fn parse_module(&mut self) -> AxiomResult<Module> {
        let span_start = self.current_span();
        let attributes = self.parse_attributes()?;
        self.expect(TokenKind::Module)?;
        let name = self.expect_ident()?;
        self.expect(TokenKind::LBrace)?;

        let mut items = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            // Skip module-level attributes that apply to the module itself
            let item_attrs = self.parse_attributes()?;
            let item = self.parse_item(item_attrs)?;
            items.push(item);
        }
        self.expect(TokenKind::RBrace)?;

        Ok(Module {
            name,
            items,
            attributes,
            span: span_start,
        })
    }

    fn parse_item(&mut self, attrs: Vec<Attribute>) -> AxiomResult<Item> {
        match self.peek_kind() {
            TokenKind::Import => self.parse_import(),
            TokenKind::Export => self.parse_exported_function(attrs),
            TokenKind::Fn => self.parse_function(false, attrs),
            TokenKind::Contract => self.parse_contract(),
            TokenKind::Intent => self.parse_intent(attrs),
            TokenKind::Enum => self.parse_enum(),
            TokenKind::TensorOp => self.parse_tensor_op(),
            _ => Err(self.error_expected("declaration (fn, contract, intent, enum, import)")),
        }
    }

    // --- Import ---

    fn parse_import(&mut self) -> AxiomResult<Item> {
        let span = self.current_span();
        self.expect(TokenKind::Import)?;
        let path = self.expect_string()?;
        self.expect(TokenKind::Semicolon)?;
        Ok(Item::Import(ImportDecl { path, span }))
    }

    // --- Function ---

    fn parse_exported_function(&mut self, attrs: Vec<Attribute>) -> AxiomResult<Item> {
        self.expect(TokenKind::Export)?;
        self.parse_function(true, attrs)
    }

    fn parse_function(&mut self, exported: bool, attributes: Vec<Attribute>) -> AxiomResult<Item> {
        let span = self.current_span();
        self.expect(TokenKind::Fn)?;
        let name = self.expect_ident()?;
        self.expect(TokenKind::LParen)?;
        let params = self.parse_param_list()?;
        self.expect(TokenKind::RParen)?;

        let return_type = if self.match_token(&TokenKind::Arrow) {
            Some(self.parse_type()?)
        } else {
            None
        };

        let body = self.parse_block()?;

        Ok(Item::Function(FunctionDecl {
            name,
            params,
            return_type,
            body,
            exported,
            attributes,
            span,
        }))
    }

    fn parse_param_list(&mut self) -> AxiomResult<Vec<Param>> {
        let mut params = Vec::new();
        if !self.check(&TokenKind::RParen) {
            params.push(self.parse_param()?);
            while self.match_token(&TokenKind::Comma) {
                if self.check(&TokenKind::RParen) {
                    break; // trailing comma
                }
                params.push(self.parse_param()?);
            }
        }
        Ok(params)
    }

    fn parse_param(&mut self) -> AxiomResult<Param> {
        let span = self.current_span();
        let name = self.expect_ident()?;
        self.expect(TokenKind::Colon)?;
        let ty = self.parse_type()?;
        Ok(Param { name, ty, span })
    }

    // --- Contract ---

    fn parse_contract(&mut self) -> AxiomResult<Item> {
        let span = self.current_span();
        self.expect(TokenKind::Contract)?;
        let name = self.expect_ident()?;

        // Check for composition: contract A = B + C;
        if self.match_token(&TokenKind::Eq) {
            let mut parts = vec![self.expect_ident()?];
            while self.match_token(&TokenKind::Plus) {
                parts.push(self.expect_ident()?);
            }
            self.expect(TokenKind::Semicolon)?;
            return Ok(Item::Contract(ContractDecl {
                name,
                fields: Vec::new(),
                invariants: None,
                composed_of: Some(parts),
                span,
            }));
        }

        self.expect(TokenKind::LBrace)?;
        let mut fields = Vec::new();
        let mut invariants = None;

        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            if self.check_ident("invariant") {
                self.advance(); // skip "invariant"
                self.expect(TokenKind::LBrace)?;
                let mut invs = Vec::new();
                while !self.check(&TokenKind::RBrace) {
                    invs.push(self.parse_expression()?);
                    self.match_token(&TokenKind::Comma);
                }
                self.expect(TokenKind::RBrace)?;
                invariants = Some(invs);
            } else {
                fields.push(self.parse_contract_field()?);
            }
        }
        self.expect(TokenKind::RBrace)?;

        Ok(Item::Contract(ContractDecl {
            name,
            fields,
            invariants,
            composed_of: None,
            span,
        }))
    }

    fn parse_contract_field(&mut self) -> AxiomResult<ContractField> {
        let span = self.current_span();
        self.expect(TokenKind::At)?;
        let index = match self.advance_token().kind {
            TokenKind::IntLiteral(n) => n as u32,
            _ => return Err(self.error_expected("field index number")),
        };
        self.expect(TokenKind::Colon)?;
        let name = self.expect_ident()?;
        self.expect(TokenKind::Colon)?;
        let ty = self.parse_type()?;

        // Optional conflict attribute
        let conflict = if self.check(&TokenKind::LBracket) {
            self.advance();
            let attr_name = self.expect_ident()?;
            if attr_name == "conflict" {
                self.expect(TokenKind::Eq)?;
                let strategy = self.expect_ident()?;
                self.expect(TokenKind::RBracket)?;
                Some(strategy)
            } else {
                self.expect(TokenKind::RBracket)?;
                None
            }
        } else {
            None
        };

        self.expect(TokenKind::Comma)?;
        Ok(ContractField {
            index,
            name,
            ty,
            conflict,
            span,
        })
    }

    // --- Intent ---

    fn parse_intent(&mut self, attributes: Vec<Attribute>) -> AxiomResult<Item> {
        let span = self.current_span();
        self.expect(TokenKind::Intent)?;
        let name = self.expect_ident()?;

        // Check for composition: intent X = A >> B >> C;
        if self.match_token(&TokenKind::Eq) {
            let mut parts = vec![self.expect_ident()?];
            while self.match_token(&TokenKind::Compose) {
                parts.push(self.expect_ident()?);
            }
            self.expect(TokenKind::Semicolon)?;
            return Ok(Item::Intent(IntentDecl {
                name,
                clauses: IntentClauses::default(),
                composed_of: Some(parts),
                attributes,
                span,
            }));
        }

        self.expect(TokenKind::LBrace)?;
        let clauses = self.parse_intent_clauses()?;
        self.expect(TokenKind::RBrace)?;

        Ok(Item::Intent(IntentDecl {
            name,
            clauses,
            composed_of: None,
            attributes,
            span,
        }))
    }

    fn parse_intent_clauses(&mut self) -> AxiomResult<IntentClauses> {
        let mut clauses = IntentClauses::default();

        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            match self.peek_kind() {
                TokenKind::Takes => {
                    self.advance();
                    self.expect(TokenKind::Colon)?;
                    clauses.takes = self.parse_intent_params()?;
                    self.expect(TokenKind::Semicolon)?;
                }
                TokenKind::Gives => {
                    self.advance();
                    self.expect(TokenKind::Colon)?;
                    clauses.gives = self.parse_intent_params()?;
                    self.expect(TokenKind::Semicolon)?;
                }
                TokenKind::Pre => {
                    self.advance();
                    self.expect(TokenKind::Colon)?;
                    clauses.pre = self.parse_expr_list_until_semi()?;
                    self.expect(TokenKind::Semicolon)?;
                }
                TokenKind::Post => {
                    self.advance();
                    self.expect(TokenKind::Colon)?;
                    clauses.post = self.parse_expr_list_until_semi()?;
                    self.expect(TokenKind::Semicolon)?;
                }
                TokenKind::Bound => {
                    self.advance();
                    self.expect(TokenKind::Colon)?;
                    clauses.bound = self.parse_bound_clauses()?;
                    self.expect(TokenKind::Semicolon)?;
                }
                TokenKind::Effect => {
                    self.advance();
                    self.expect(TokenKind::Colon)?;
                    clauses.effect = Some(self.expect_ident()?);
                    self.expect(TokenKind::Semicolon)?;
                }
                TokenKind::Conscience => {
                    self.advance();
                    self.expect(TokenKind::Colon)?;
                    clauses.conscience = self.parse_ident_list()?;
                    self.expect(TokenKind::Semicolon)?;
                }
                TokenKind::Fallback => {
                    self.advance();
                    self.expect(TokenKind::Colon)?;
                    clauses.fallback = Some(self.expect_ident()?);
                    self.expect(TokenKind::Semicolon)?;
                }
                TokenKind::Rollback => {
                    self.advance();
                    self.expect(TokenKind::Colon)?;
                    clauses.rollback = Some(self.expect_ident()?);
                    self.expect(TokenKind::Semicolon)?;
                }
                TokenKind::Ring => {
                    self.advance();
                    self.expect(TokenKind::Colon)?;
                    let neg = self.match_token(&TokenKind::Minus);
                    let val = match self.advance_token().kind {
                        TokenKind::IntLiteral(n) => n,
                        _ => return Err(self.error_expected("ring number")),
                    };
                    clauses.ring = Some(if neg { -val } else { val });
                    self.expect(TokenKind::Semicolon)?;
                }
                _ => {
                    // Try to parse as a generic ident clause (e.g. trace:)
                    if let TokenKind::Ident(_) = self.peek_kind() {
                        let _clause_name = self.expect_ident()?;
                        self.expect(TokenKind::Colon)?;
                        // Skip until semicolon
                        while !self.check(&TokenKind::Semicolon) && !self.is_at_end() {
                            self.advance();
                        }
                        self.expect(TokenKind::Semicolon)?;
                    } else {
                        return Err(self.error_expected("intent clause"));
                    }
                }
            }
        }
        Ok(clauses)
    }

    fn parse_intent_params(&mut self) -> AxiomResult<Vec<Param>> {
        let mut params = Vec::new();
        if !self.check(&TokenKind::Semicolon) {
            params.push(self.parse_param()?);
            while self.match_token(&TokenKind::Comma) {
                if self.check(&TokenKind::Semicolon) {
                    break;
                }
                params.push(self.parse_param()?);
            }
        }
        Ok(params)
    }

    fn parse_bound_clauses(&mut self) -> AxiomResult<Vec<BoundClause>> {
        let mut bounds = Vec::new();
        let span = self.current_span();
        let resource = self.expect_ident()?;
        self.expect(TokenKind::LParen)?;
        let value = self.parse_bound_value()?;
        self.expect(TokenKind::RParen)?;
        bounds.push(BoundClause { resource, value, span });

        while self.match_token(&TokenKind::Comma) {
            if self.check(&TokenKind::Semicolon) {
                break;
            }
            let span = self.current_span();
            let resource = self.expect_ident()?;
            self.expect(TokenKind::LParen)?;
            let value = self.parse_bound_value()?;
            self.expect(TokenKind::RParen)?;
            bounds.push(BoundClause { resource, value, span });
        }
        Ok(bounds)
    }

    fn parse_bound_value(&mut self) -> AxiomResult<String> {
        // Bound values can be like "100ms", "64mb", "1s" etc.
        // We'll collect tokens until we hit a )
        let mut val = String::new();
        while !self.check(&TokenKind::RParen) && !self.is_at_end() {
            val.push_str(&self.advance_token().lexeme);
        }
        Ok(val)
    }

    fn parse_ident_list(&mut self) -> AxiomResult<Vec<String>> {
        let mut names = vec![self.expect_ident()?];
        while self.match_token(&TokenKind::Comma) {
            if self.check(&TokenKind::Semicolon) {
                break;
            }
            names.push(self.expect_ident()?);
        }
        Ok(names)
    }

    fn parse_expr_list_until_semi(&mut self) -> AxiomResult<Vec<Expr>> {
        let mut exprs = vec![self.parse_expression()?];
        while self.match_token(&TokenKind::Comma) {
            if self.check(&TokenKind::Semicolon) {
                break;
            }
            exprs.push(self.parse_expression()?);
        }
        Ok(exprs)
    }

    // --- Enum ---

    fn parse_enum(&mut self) -> AxiomResult<Item> {
        let span = self.current_span();
        self.expect(TokenKind::Enum)?;
        let name = self.expect_ident()?;
        self.expect(TokenKind::LBrace)?;

        let mut variants = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            let vspan = self.current_span();
            let vname = self.expect_ident()?;

            let fields = if self.check(&TokenKind::LParen) {
                self.advance();
                let params = self.parse_param_list()?;
                self.expect(TokenKind::RParen)?;
                params
            } else {
                Vec::new()
            };

            variants.push(EnumVariant {
                name: vname,
                fields,
                span: vspan,
            });
            self.match_token(&TokenKind::Comma);
        }
        self.expect(TokenKind::RBrace)?;

        Ok(Item::Enum(EnumDecl { name, variants, span }))
    }

    // --- TensorOp ---

    fn parse_tensor_op(&mut self) -> AxiomResult<Item> {
        let span = self.current_span();
        self.expect(TokenKind::TensorOp)?;
        let name = self.expect_ident()?;
        self.expect(TokenKind::LBrace)?;

        let mut takes = Vec::new();
        let mut gives = Vec::new();
        let shape_rule = None;

        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            match self.peek_kind() {
                TokenKind::Takes => {
                    self.advance();
                    self.expect(TokenKind::Colon)?;
                    takes = self.parse_intent_params()?;
                    self.expect(TokenKind::Semicolon)?;
                }
                TokenKind::Gives => {
                    self.advance();
                    self.expect(TokenKind::Colon)?;
                    gives = self.parse_intent_params()?;
                    self.expect(TokenKind::Semicolon)?;
                }
                _ => {
                    // Skip other clauses (shape_rule, determinism, etc.)
                    let _name = self.expect_ident()?;
                    self.expect(TokenKind::Colon)?;
                    while !self.check(&TokenKind::Semicolon) && !self.is_at_end() {
                        self.advance();
                    }
                    self.expect(TokenKind::Semicolon)?;
                }
            }
        }
        self.expect(TokenKind::RBrace)?;

        Ok(Item::TensorOp(TensorOpDecl {
            name,
            takes,
            gives,
            shape_rule,
            span,
        }))
    }

    // --- Types ---

    fn parse_type(&mut self) -> AxiomResult<TypeExpr> {
        let span = self.current_span();

        // Array type: [T]
        if self.check(&TokenKind::LBracket) {
            self.advance();
            let inner = self.parse_type()?;
            self.expect(TokenKind::RBracket)?;
            return Ok(TypeExpr::Array(Box::new(inner), span));
        }

        let name = self.expect_ident()?;

        match name.as_str() {
            "Map" => {
                self.expect(TokenKind::Lt)?;
                let key = self.parse_type()?;
                self.expect(TokenKind::Comma)?;
                let val = self.parse_type()?;
                self.expect(TokenKind::Gt)?;
                Ok(TypeExpr::Map(Box::new(key), Box::new(val), span))
            }
            "Tensor" => {
                self.expect(TokenKind::LBracket)?;
                let mut dims = Vec::new();
                if !self.check(&TokenKind::RBracket) {
                    dims.push(self.parse_tensor_dim()?);
                    while self.match_token(&TokenKind::Comma) {
                        dims.push(self.parse_tensor_dim()?);
                    }
                }
                self.expect(TokenKind::RBracket)?;
                Ok(TypeExpr::Tensor(dims, span))
            }
            "Sealed" => {
                self.expect(TokenKind::Lt)?;
                let inner = self.parse_type()?;
                self.expect(TokenKind::Gt)?;
                Ok(TypeExpr::Sealed(Box::new(inner), span))
            }
            _ => Ok(TypeExpr::Named(name, span)),
        }
    }

    fn parse_tensor_dim(&mut self) -> AxiomResult<TensorDim> {
        if self.match_token(&TokenKind::Question) {
            Ok(TensorDim::Wildcard)
        } else if let TokenKind::IntLiteral(n) = self.peek_kind() {
            let n = n;
            self.advance();
            Ok(TensorDim::Fixed(n))
        } else {
            let name = self.expect_ident()?;
            Ok(TensorDim::Named(name))
        }
    }

    // --- Attributes ---

    fn parse_attributes(&mut self) -> AxiomResult<Vec<Attribute>> {
        let mut attrs = Vec::new();
        while self.check(&TokenKind::LBracket) {
            attrs.push(self.parse_attribute()?);
        }
        Ok(attrs)
    }

    fn parse_attribute(&mut self) -> AxiomResult<Attribute> {
        let span = self.current_span();
        self.expect(TokenKind::LBracket)?;
        let name = self.expect_ident()?;

        let mut args = Vec::new();
        if self.match_token(&TokenKind::LParen) {
            while !self.check(&TokenKind::RParen) && !self.is_at_end() {
                // Try key: value or just value
                let first = self.advance_token();
                if self.check(&TokenKind::Colon) {
                    self.advance();
                    let val = self.advance_token();
                    args.push((Some(first.lexeme), val.lexeme));
                } else {
                    args.push((None, first.lexeme));
                }
                self.match_token(&TokenKind::Comma);
            }
            self.expect(TokenKind::RParen)?;
        }
        self.expect(TokenKind::RBracket)?;
        Ok(Attribute { name, args, span })
    }

    // --- Blocks and Statements ---

    fn parse_block(&mut self) -> AxiomResult<Block> {
        let span = self.current_span();
        self.expect(TokenKind::LBrace)?;
        let mut stmts = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            stmts.push(self.parse_statement()?);
        }
        self.expect(TokenKind::RBrace)?;
        Ok(Block { stmts, span })
    }

    fn parse_statement(&mut self) -> AxiomResult<Stmt> {
        match self.peek_kind() {
            TokenKind::Let => self.parse_let_stmt(),
            TokenKind::Return => self.parse_return_stmt(),
            TokenKind::If => self.parse_if_stmt(),
            TokenKind::Loop => self.parse_loop_stmt(),
            TokenKind::Match => self.parse_match_stmt(),
            TokenKind::Break => {
                let span = self.current_span();
                self.advance();
                self.expect(TokenKind::Semicolon)?;
                Ok(Stmt::Break(span))
            }
            TokenKind::Continue => {
                let span = self.current_span();
                self.advance();
                self.expect(TokenKind::Semicolon)?;
                Ok(Stmt::Continue(span))
            }
            _ => {
                // Could be assignment or expression statement
                let expr = self.parse_expression()?;

                // Check for assignment
                if let Expr::Ident(ref name, ref span) = expr {
                    match self.peek_kind() {
                        TokenKind::Eq => {
                            self.advance();
                            let value = self.parse_expression()?;
                            self.expect(TokenKind::Semicolon)?;
                            return Ok(Stmt::Assign(AssignStmt {
                                target: name.clone(),
                                op: AssignOp::Assign,
                                value,
                                span: span.clone(),
                            }));
                        }
                        TokenKind::PlusEq => {
                            self.advance();
                            let value = self.parse_expression()?;
                            self.expect(TokenKind::Semicolon)?;
                            return Ok(Stmt::Assign(AssignStmt {
                                target: name.clone(),
                                op: AssignOp::PlusAssign,
                                value,
                                span: span.clone(),
                            }));
                        }
                        TokenKind::MinusEq => {
                            self.advance();
                            let value = self.parse_expression()?;
                            self.expect(TokenKind::Semicolon)?;
                            return Ok(Stmt::Assign(AssignStmt {
                                target: name.clone(),
                                op: AssignOp::MinusAssign,
                                value,
                                span: span.clone(),
                            }));
                        }
                        _ => {}
                    }
                }

                let span = expr.span().clone();
                self.expect(TokenKind::Semicolon)?;
                Ok(Stmt::Expr(ExprStmt { expr, span }))
            }
        }
    }

    fn parse_let_stmt(&mut self) -> AxiomResult<Stmt> {
        let span = self.current_span();
        self.expect(TokenKind::Let)?;
        let name = self.expect_ident()?;

        let ty = if self.match_token(&TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };

        self.expect(TokenKind::Eq)?;
        let value = self.parse_expression()?;
        self.expect(TokenKind::Semicolon)?;

        Ok(Stmt::Let(LetStmt {
            name,
            ty,
            value,
            span,
        }))
    }

    fn parse_return_stmt(&mut self) -> AxiomResult<Stmt> {
        let span = self.current_span();
        self.expect(TokenKind::Return)?;
        let value = if !self.check(&TokenKind::Semicolon) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        self.expect(TokenKind::Semicolon)?;
        Ok(Stmt::Return(ReturnStmt { value, span }))
    }

    fn parse_if_stmt(&mut self) -> AxiomResult<Stmt> {
        let span = self.current_span();
        self.expect(TokenKind::If)?;
        let condition = self.parse_expression()?;
        let then_block = self.parse_block()?;
        let else_block = if self.match_token(&TokenKind::Else) {
            Some(self.parse_block()?)
        } else {
            None
        };
        Ok(Stmt::If(IfStmt {
            condition,
            then_block,
            else_block,
            span,
        }))
    }

    fn parse_loop_stmt(&mut self) -> AxiomResult<Stmt> {
        let span = self.current_span();
        self.expect(TokenKind::Loop)?;
        self.expect(TokenKind::LParen)?;
        let condition = self.parse_expression()?;
        self.expect(TokenKind::Comma)?;
        let max_iter = self.parse_expression()?;
        self.expect(TokenKind::RParen)?;
        let body = self.parse_block()?;
        Ok(Stmt::Loop(LoopStmt {
            condition,
            max_iter,
            body,
            span,
        }))
    }

    fn parse_match_stmt(&mut self) -> AxiomResult<Stmt> {
        let span = self.current_span();
        self.expect(TokenKind::Match)?;
        let value = self.parse_expression()?;
        self.expect(TokenKind::LBrace)?;

        let mut arms = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            arms.push(self.parse_match_arm()?);
        }
        self.expect(TokenKind::RBrace)?;

        Ok(Stmt::Match(MatchStmt { value, arms, span }))
    }

    fn parse_match_arm(&mut self) -> AxiomResult<MatchArm> {
        let span = self.current_span();
        let pattern = self.parse_pattern()?;
        self.expect(TokenKind::FatArrow)?;

        let body = if self.check(&TokenKind::LBrace) {
            let block = self.parse_block()?;
            let bspan = block.span.clone();
            Expr::Block(block, bspan)
        } else {
            let expr = self.parse_expression()?;
            expr
        };

        self.match_token(&TokenKind::Comma);
        Ok(MatchArm {
            pattern,
            body,
            span,
        })
    }

    fn parse_pattern(&mut self) -> AxiomResult<Pattern> {
        match self.peek_kind() {
            TokenKind::IntLiteral(n) => {
                let n = n;
                self.advance();
                Ok(Pattern::IntLiteral(n))
            }
            TokenKind::FloatLiteral(n) => {
                let n = n;
                self.advance();
                Ok(Pattern::FloatLiteral(n))
            }
            TokenKind::StringLiteral(s) => {
                let s = s;
                self.advance();
                Ok(Pattern::StringLiteral(s))
            }
            TokenKind::True => {
                self.advance();
                Ok(Pattern::BoolLiteral(true))
            }
            TokenKind::False => {
                self.advance();
                Ok(Pattern::BoolLiteral(false))
            }
            TokenKind::Underscore => {
                self.advance();
                Ok(Pattern::Wildcard)
            }
            TokenKind::Ident(name) => {
                let name = name;
                self.advance();
                if self.match_token(&TokenKind::Dot) {
                    let variant = self.expect_ident()?;
                    Ok(Pattern::EnumVariant(name, variant))
                } else {
                    Ok(Pattern::Ident(name))
                }
            }
            _ => Err(self.error_expected("pattern")),
        }
    }

    // --- Expressions (Pratt parser) ---

    fn parse_expression(&mut self) -> AxiomResult<Expr> {
        self.parse_pipeline()
    }

    fn parse_pipeline(&mut self) -> AxiomResult<Expr> {
        let mut left = self.parse_or()?;
        while self.check(&TokenKind::Pipeline) {
            let span = self.current_span();
            self.advance();
            let right = self.parse_or()?;
            left = Expr::Pipeline(Box::new(left), Box::new(right), span);
        }
        Ok(left)
    }

    fn parse_or(&mut self) -> AxiomResult<Expr> {
        let mut left = self.parse_and()?;
        while self.check(&TokenKind::OrOr) {
            let span = self.current_span();
            self.advance();
            let right = self.parse_and()?;
            left = Expr::Binary(Box::new(left), BinOp::Or, Box::new(right), span);
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> AxiomResult<Expr> {
        let mut left = self.parse_equality()?;
        while self.check(&TokenKind::AndAnd) {
            let span = self.current_span();
            self.advance();
            let right = self.parse_equality()?;
            left = Expr::Binary(Box::new(left), BinOp::And, Box::new(right), span);
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> AxiomResult<Expr> {
        let mut left = self.parse_comparison()?;
        loop {
            let span = self.current_span();
            match self.peek_kind() {
                TokenKind::EqEq => {
                    self.advance();
                    let right = self.parse_comparison()?;
                    left = Expr::Binary(Box::new(left), BinOp::Eq, Box::new(right), span);
                }
                TokenKind::NotEq => {
                    self.advance();
                    let right = self.parse_comparison()?;
                    left = Expr::Binary(Box::new(left), BinOp::NotEq, Box::new(right), span);
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> AxiomResult<Expr> {
        let mut left = self.parse_additive()?;
        loop {
            let span = self.current_span();
            match self.peek_kind() {
                TokenKind::Lt => {
                    self.advance();
                    let right = self.parse_additive()?;
                    left = Expr::Binary(Box::new(left), BinOp::Lt, Box::new(right), span);
                }
                TokenKind::Gt => {
                    self.advance();
                    let right = self.parse_additive()?;
                    left = Expr::Binary(Box::new(left), BinOp::Gt, Box::new(right), span);
                }
                TokenKind::LtEq => {
                    self.advance();
                    let right = self.parse_additive()?;
                    left = Expr::Binary(Box::new(left), BinOp::LtEq, Box::new(right), span);
                }
                TokenKind::GtEq => {
                    self.advance();
                    let right = self.parse_additive()?;
                    left = Expr::Binary(Box::new(left), BinOp::GtEq, Box::new(right), span);
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> AxiomResult<Expr> {
        let mut left = self.parse_multiplicative()?;
        loop {
            let span = self.current_span();
            match self.peek_kind() {
                TokenKind::Plus => {
                    self.advance();
                    let right = self.parse_multiplicative()?;
                    left = Expr::Binary(Box::new(left), BinOp::Add, Box::new(right), span);
                }
                TokenKind::Minus => {
                    self.advance();
                    let right = self.parse_multiplicative()?;
                    left = Expr::Binary(Box::new(left), BinOp::Sub, Box::new(right), span);
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> AxiomResult<Expr> {
        let mut left = self.parse_unary()?;
        loop {
            let span = self.current_span();
            match self.peek_kind() {
                TokenKind::Star => {
                    self.advance();
                    let right = self.parse_unary()?;
                    left = Expr::Binary(Box::new(left), BinOp::Mul, Box::new(right), span);
                }
                TokenKind::Slash => {
                    self.advance();
                    let right = self.parse_unary()?;
                    left = Expr::Binary(Box::new(left), BinOp::Div, Box::new(right), span);
                }
                TokenKind::Percent => {
                    self.advance();
                    let right = self.parse_unary()?;
                    left = Expr::Binary(Box::new(left), BinOp::Mod, Box::new(right), span);
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> AxiomResult<Expr> {
        let span = self.current_span();
        match self.peek_kind() {
            TokenKind::Minus => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::Unary(UnaryOp::Neg, Box::new(expr), span))
            }
            TokenKind::Bang => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::Unary(UnaryOp::Not, Box::new(expr), span))
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> AxiomResult<Expr> {
        let mut expr = self.parse_primary()?;
        loop {
            match self.peek_kind() {
                TokenKind::Dot => {
                    let span = self.current_span();
                    self.advance();
                    let field = self.expect_ident()?;
                    expr = Expr::FieldAccess(Box::new(expr), field, span);
                }
                TokenKind::LBracket => {
                    let span = self.current_span();
                    self.advance();
                    let index = self.parse_expression()?;
                    self.expect(TokenKind::RBracket)?;
                    expr = Expr::Index(Box::new(expr), Box::new(index), span);
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> AxiomResult<Expr> {
        let span = self.current_span();

        match self.peek_kind() {
            TokenKind::IntLiteral(n) => {
                let n = n;
                self.advance();
                Ok(Expr::IntLiteral(n, span))
            }
            TokenKind::FloatLiteral(n) => {
                let n = n;
                self.advance();
                Ok(Expr::FloatLiteral(n, span))
            }
            TokenKind::StringLiteral(s) => {
                let s = s;
                self.advance();
                Ok(Expr::StringLiteral(s, span))
            }
            TokenKind::True => {
                self.advance();
                Ok(Expr::BoolLiteral(true, span))
            }
            TokenKind::False => {
                self.advance();
                Ok(Expr::BoolLiteral(false, span))
            }
            TokenKind::LParen => {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect(TokenKind::RParen)?;
                Ok(expr)
            }
            TokenKind::LBracket => {
                // Array literal: [a, b, c]
                self.advance();
                let mut elems = Vec::new();
                if !self.check(&TokenKind::RBracket) {
                    elems.push(self.parse_expression()?);
                    while self.match_token(&TokenKind::Comma) {
                        if self.check(&TokenKind::RBracket) {
                            break;
                        }
                        elems.push(self.parse_expression()?);
                    }
                }
                self.expect(TokenKind::RBracket)?;
                Ok(Expr::ArrayLiteral(elems, span))
            }
            TokenKind::Do => {
                self.advance();
                let intent_name = self.expect_ident()?;
                self.expect(TokenKind::LBrace)?;
                let fields = self.parse_field_init_list()?;
                self.expect(TokenKind::RBrace)?;
                Ok(Expr::Do(intent_name, fields, span))
            }
            TokenKind::QueryConscience => {
                self.advance();
                self.expect(TokenKind::LParen)?;
                let intent_name = self.expect_ident()?;
                self.expect(TokenKind::LBrace)?;
                let fields = self.parse_field_init_list()?;
                self.expect(TokenKind::RBrace)?;
                self.expect(TokenKind::RParen)?;
                Ok(Expr::QueryConscience(intent_name, fields, span))
            }
            TokenKind::DeclareAnomaly => {
                self.advance();
                self.expect(TokenKind::LParen)?;
                let anomaly_type = self.parse_expression()?;
                self.expect(TokenKind::Comma)?;
                self.expect(TokenKind::LBrace)?;
                let fields = self.parse_field_init_list()?;
                self.expect(TokenKind::RBrace)?;
                self.expect(TokenKind::RParen)?;
                Ok(Expr::DeclareAnomaly(Box::new(anomaly_type), fields, span))
            }
            TokenKind::Collapse => {
                self.advance();
                self.expect(TokenKind::LParen)?;
                let inner = self.parse_expression()?;
                self.expect(TokenKind::RParen)?;
                Ok(Expr::Collapse(Box::new(inner), span))
            }
            TokenKind::Resolve => {
                self.advance();
                self.expect(TokenKind::LParen)?;
                let inner = self.parse_expression()?;
                self.expect(TokenKind::RParen)?;
                Ok(Expr::Resolve(Box::new(inner), span))
            }
            TokenKind::Ident(name) => {
                let name = name;
                self.advance();

                // Check for enum variant access: Name.Variant
                if self.check(&TokenKind::Dot) {
                    // Peek ahead to see if this is field access or enum access
                    // We treat it as potential enum access if the name starts uppercase
                    if name.chars().next().is_some_and(|c| c.is_uppercase()) {
                        let _dot_span = self.current_span();
                        self.advance(); // consume .
                        let variant = self.expect_ident()?;

                        // If followed by ( then it's still a call, but on the enum variant
                        if self.check(&TokenKind::LParen) {
                            // Treat as function call: EnumName.Variant(args)
                            self.advance();
                            let mut args = Vec::new();
                            if !self.check(&TokenKind::RParen) {
                                args.push(self.parse_expression()?);
                                while self.match_token(&TokenKind::Comma) {
                                    args.push(self.parse_expression()?);
                                }
                            }
                            self.expect(TokenKind::RParen)?;
                            return Ok(Expr::Call(
                                format!("{}.{}", name, variant),
                                args,
                                span,
                            ));
                        }

                        return Ok(Expr::EnumAccess(name, variant, span));
                    }
                }

                // Function call: name(args)
                if self.check(&TokenKind::LParen) {
                    self.advance();
                    let mut args = Vec::new();
                    if !self.check(&TokenKind::RParen) {
                        args.push(self.parse_expression()?);
                        while self.match_token(&TokenKind::Comma) {
                            if self.check(&TokenKind::RParen) {
                                break;
                            }
                            args.push(self.parse_expression()?);
                        }
                    }
                    self.expect(TokenKind::RParen)?;
                    return Ok(Expr::Call(name, args, span));
                }

                // Contract init: Name { field: value, ... }
                if self.check(&TokenKind::LBrace)
                    && name.chars().next().is_some_and(|c| c.is_uppercase())
                {
                    self.advance();
                    let fields = self.parse_field_init_list()?;
                    self.expect(TokenKind::RBrace)?;
                    return Ok(Expr::ContractInit(name, fields, span));
                }

                Ok(Expr::Ident(name, span))
            }
            _ => Err(self.error_expected("expression")),
        }
    }

    fn parse_field_init_list(&mut self) -> AxiomResult<Vec<(String, Expr)>> {
        let mut fields = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            let name = self.expect_ident()?;
            self.expect(TokenKind::Colon)?;
            let value = self.parse_expression()?;
            fields.push((name, value));
            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }
        Ok(fields)
    }

    // --- Token helpers ---

    fn peek_kind(&self) -> TokenKind {
        if self.pos < self.tokens.len() {
            self.tokens[self.pos].kind.clone()
        } else {
            TokenKind::Eof
        }
    }

    fn current_span(&self) -> Span {
        if self.pos < self.tokens.len() {
            self.tokens[self.pos].span.clone()
        } else {
            Span {
                start: 0,
                end: 0,
                line: 0,
                col: 0,
            }
        }
    }

    fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(&self.peek_kind()) == std::mem::discriminant(kind)
    }

    fn check_ident(&self, name: &str) -> bool {
        matches!(&self.peek_kind(), TokenKind::Ident(n) if n == name)
    }

    fn match_token(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn advance(&mut self) {
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
    }

    fn advance_token(&mut self) -> Token {
        let tok = if self.pos < self.tokens.len() {
            self.tokens[self.pos].clone()
        } else {
            Token {
                kind: TokenKind::Eof,
                span: Span {
                    start: 0,
                    end: 0,
                    line: 0,
                    col: 0,
                },
                lexeme: String::new(),
            }
        };
        self.pos += 1;
        tok
    }

    fn expect(&mut self, kind: TokenKind) -> AxiomResult<()> {
        if self.check(&kind) {
            self.advance();
            Ok(())
        } else {
            Err(AxiomError {
                kind: ErrorKind::ExpectedToken,
                message: format!("Expected '{}', found '{}'", kind, self.peek_kind()),
                span: Some(self.current_span()),
            })
        }
    }

    fn expect_ident(&mut self) -> AxiomResult<String> {
        match self.peek_kind() {
            TokenKind::Ident(name) => {
                let name = name;
                self.advance();
                Ok(name)
            }
            _ => Err(AxiomError {
                kind: ErrorKind::ExpectedToken,
                message: format!("Expected identifier, found '{}'", self.peek_kind()),
                span: Some(self.current_span()),
            }),
        }
    }

    fn expect_string(&mut self) -> AxiomResult<String> {
        match self.peek_kind() {
            TokenKind::StringLiteral(s) => {
                let s = s;
                self.advance();
                Ok(s)
            }
            _ => Err(AxiomError {
                kind: ErrorKind::ExpectedToken,
                message: format!("Expected string literal, found '{}'", self.peek_kind()),
                span: Some(self.current_span()),
            }),
        }
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.tokens.len() || matches!(self.peek_kind(), TokenKind::Eof)
    }

    fn error_expected(&self, what: &str) -> AxiomError {
        AxiomError {
            kind: ErrorKind::UnexpectedToken,
            message: format!("Expected {}, found '{}'", what, self.peek_kind()),
            span: Some(self.current_span()),
        }
    }
}
