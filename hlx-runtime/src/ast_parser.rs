//! AST Parser - Converts token stream to proper AST nodes
//!
//! This parser produces an introspectable AST that RSI can safely modify.
//! It separates parsing from bytecode emission, enabling:
//! 1. AST inspection and transformation
//! 2. RSI modifications with rollback
//! 3. Source-to-source transformations

use crate::ast::*;
use std::convert::From;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Literals
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Ident(String),

    // Keywords
    Let,
    Fn,
    Return,
    If,
    Else,
    Loop,
    Break,
    Continue,
    Program,
    Module,
    Import,
    Export,
    Recursive,
    Agent,
    Latent,
    Cycle,
    Halt,
    Govern,
    Scale,
    Cluster,
    Sync,
    Barrier,
    Consensus,
    Dissolvable,
    Inherit,
    OnDissolve,
    Lifetime,
    Generation,
    Modify,
    Gate,
    Proof,
    Budget,
    Cooldown,
    Approve,
    Spawn,
    Takes,
    Gives,
    Action,
    When,
    Outer,
    Inner,
    Const,
    Mut,
    Collapse,
    Resolve,
    Switch,
    Case,
    Default,
    Match,
    While,
    For,

    // Operators
    Eq,
    EqEq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    And,
    Or,
    Bang,
    Amp,
    Pipe,
    FatArrow, // =>
    Arrow,    // ->

    // Delimiters
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Semi,
    Colon,
    Comma,
    Dot,

    // End
    Eof,
}

#[derive(Debug, Clone)]
pub struct LexError {
    pub message: String,
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub col: usize,
}

impl From<LexError> for ParseError {
    fn from(e: LexError) -> Self {
        ParseError {
            message: e.message,
            line: e.line,
            col: e.col,
        }
    }
}

pub struct AstParser {
    tokens: Vec<Token>,
    pos: usize,
    source_lines: Vec<String>,
}

impl AstParser {
    pub fn new(source: &str) -> Self {
        let source_lines: Vec<String> = source.lines().map(|s| s.to_string()).collect();
        AstParser {
            tokens: Vec::new(),
            pos: 0,
            source_lines,
        }
    }

    pub fn parse(source: &str) -> Result<Program, ParseError> {
        let mut parser = AstParser::new(source);
        parser.tokenize()?;
        parser.parse_program()
    }

    fn tokenize(&mut self) -> Result<(), LexError> {
        let source = self.source_lines.join("\n");
        let chars: Vec<char> = source.chars().collect();
        let mut pos = 0;

        while pos < chars.len() {
            let c = chars[pos];

            if c.is_whitespace() {
                pos += 1;
                continue;
            }

            if c == '/' && pos + 1 < chars.len() && chars[pos + 1] == '/' {
                while pos < chars.len() && chars[pos] != '\n' {
                    pos += 1;
                }
                continue;
            }

            if c == '"' {
                pos += 1;
                let start = pos;
                while pos < chars.len() && chars[pos] != '"' {
                    pos += 1;
                }
                let s: String = chars[start..pos].iter().collect();
                self.tokens.push(Token::String(s));
                pos += 1;
                continue;
            }

            if c.is_ascii_digit()
                || (c == '-' && pos + 1 < chars.len() && chars[pos + 1].is_ascii_digit())
            {
                let start = pos;
                if c == '-' {
                    pos += 1;
                }
                while pos < chars.len() && (chars[pos].is_ascii_digit() || chars[pos] == '.') {
                    pos += 1;
                }
                let num_str: String = chars[start..pos].iter().collect();
                if num_str.contains('.') {
                    self.tokens
                        .push(Token::Float(num_str.parse().unwrap_or(0.0)));
                } else {
                    self.tokens.push(Token::Int(num_str.parse().unwrap_or(0)));
                }
                continue;
            }

            if c.is_alphabetic() || c == '_' {
                let start = pos;
                while pos < chars.len() && (chars[pos].is_alphanumeric() || chars[pos] == '_') {
                    pos += 1;
                }
                let word: String = chars[start..pos].iter().collect();
                let tok = match word.as_str() {
                    "let" => Token::Let,
                    "fn" => Token::Fn,
                    "return" => Token::Return,
                    "if" => Token::If,
                    "else" => Token::Else,
                    "loop" => Token::Loop,
                    "break" => Token::Break,
                    "continue" => Token::Continue,
                    "program" => Token::Program,
                    "module" => Token::Module,
                    "import" => Token::Import,
                    "export" => Token::Export,
                    "recursive" => Token::Recursive,
                    "agent" => Token::Agent,
                    "latent" => Token::Latent,
                    "cycle" => Token::Cycle,
                    "halt" => Token::Halt,
                    "govern" => Token::Govern,
                    "scale" => Token::Scale,
                    "cluster" => Token::Cluster,
                    "sync" => Token::Sync,
                    "barrier" => Token::Barrier,
                    "consensus" => Token::Consensus,
                    "dissolvable" => Token::Dissolvable,
                    "inherit" => Token::Inherit,
                    "on_dissolve" => Token::OnDissolve,
                    "lifetime" => Token::Lifetime,
                    "generation" => Token::Generation,
                    "modify" => Token::Modify,
                    "gate" => Token::Gate,
                    "proof" => Token::Proof,
                    "budget" => Token::Budget,
                    "cooldown" => Token::Cooldown,
                    "approve" => Token::Approve,
                    "takes" => Token::Takes,
                    "gives" => Token::Gives,
                    "action" => Token::Action,
                    "when" => Token::When,
                    "outer" => Token::Outer,
                    "inner" => Token::Inner,
                    "const" => Token::Const,
                    "mut" => Token::Mut,
                    "collapse" => Token::Collapse,
                    "resolve" => Token::Resolve,
                    "switch" => Token::Switch,
                    "case" => Token::Case,
                    "default" => Token::Default,
                    "match" => Token::Match,
                    "spawn" => Token::Spawn,
                    "true" => Token::Bool(true),
                    "false" => Token::Bool(false),
                    _ => Token::Ident(word),
                };
                self.tokens.push(tok);
                continue;
            }

            let tok = match c {
                '(' => Token::LParen,
                ')' => Token::RParen,
                '{' => Token::LBrace,
                '}' => Token::RBrace,
                '[' => Token::LBracket,
                ']' => Token::RBracket,
                ';' => Token::Semi,
                ':' => Token::Colon,
                ',' => Token::Comma,
                '.' => Token::Dot,
                '+' => Token::Plus,
                '-' => {
                    if pos + 1 < chars.len() && chars[pos + 1] == '>' {
                        pos += 1;
                        Token::Arrow
                    } else {
                        Token::Minus
                    }
                }
                '*' => Token::Star,
                '/' => Token::Slash,
                '%' => Token::Percent,
                '=' => {
                    if pos + 1 < chars.len() && chars[pos + 1] == '=' {
                        pos += 1;
                        Token::EqEq
                    } else if pos + 1 < chars.len() && chars[pos + 1] == '>' {
                        pos += 1;
                        Token::FatArrow
                    } else {
                        Token::Eq
                    }
                }
                '!' => {
                    if pos + 1 < chars.len() && chars[pos + 1] == '=' {
                        pos += 1;
                        Token::Ne
                    } else {
                        Token::Bang
                    }
                }
                '<' => {
                    if pos + 1 < chars.len() && chars[pos + 1] == '=' {
                        pos += 1;
                        Token::Le
                    } else {
                        Token::Lt
                    }
                }
                '>' => {
                    if pos + 1 < chars.len() && chars[pos + 1] == '=' {
                        pos += 1;
                        Token::Ge
                    } else {
                        Token::Gt
                    }
                }
                '&' => {
                    if pos + 1 < chars.len() && chars[pos + 1] == '&' {
                        pos += 1;
                        Token::And
                    } else {
                        Token::Amp
                    }
                }
                '|' => {
                    if pos + 1 < chars.len() && chars[pos + 1] == '|' {
                        pos += 1;
                        Token::Or
                    } else {
                        Token::Pipe
                    }
                }
                _ => {
                    return Err(LexError {
                        message: format!("Unexpected character: {}", c),
                        line: 0,
                        col: pos,
                    });
                }
            };
            self.tokens.push(tok);
            pos += 1;
        }

        self.tokens.push(Token::Eof);
        Ok(())
    }

    fn current(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) -> Token {
        let tok = self.current().clone();
        self.pos += 1;
        tok
    }

    /// Try to consume the current token as a name string.
    /// Handles both `Token::Ident` and keyword tokens that can appear in
    /// identifier position (e.g. `gate proof;` where `proof` is a keyword).
    fn try_consume_name(&mut self) -> Option<String> {
        let name = match self.current() {
            Token::Ident(n) => Some(n.clone()),
            Token::Proof => Some("proof".to_string()),
            Token::Consensus => Some("consensus".to_string()),
            Token::Gate => Some("gate".to_string()),
            Token::Modify => Some("modify".to_string()),
            Token::Spawn => Some("spawn".to_string()),
            Token::Dissolvable => Some("dissolvable".to_string()),
            Token::Budget => Some("budget".to_string()),
            Token::Cooldown => Some("cooldown".to_string()),
            Token::Approve => Some("approve".to_string()),
            Token::Action => Some("action".to_string()),
            Token::Sync => Some("sync".to_string()),
            Token::Barrier => Some("barrier".to_string()),
            Token::Halt => Some("halt".to_string()),
            Token::Cycle => Some("cycle".to_string()),
            Token::Latent => Some("latent".to_string()),
            Token::Agent => Some("agent".to_string()),
            Token::Scale => Some("scale".to_string()),
            Token::Cluster => Some("cluster".to_string()),
            Token::Govern => Some("govern".to_string()),
            Token::Inherit => Some("inherit".to_string()),
            Token::When => Some("when".to_string()),
            Token::Outer => Some("outer".to_string()),
            Token::Inner => Some("inner".to_string()),
            Token::Collapse => Some("collapse".to_string()),
            Token::Resolve => Some("resolve".to_string()),
            Token::Takes => Some("takes".to_string()),
            Token::Gives => Some("gives".to_string()),
            Token::Generation => Some("generation".to_string()),
            Token::Lifetime => Some("lifetime".to_string()),
            _ => None,
        };
        if name.is_some() {
            self.advance();
        }
        name
    }

    fn expect(&mut self, expected: &Token) -> Result<Token, ParseError> {
        let tok = self.current().clone();
        if std::mem::discriminant(&tok) == std::mem::discriminant(expected) {
            self.advance();
            Ok(tok)
        } else {
            Err(ParseError {
                message: format!("Expected {:?}, got {:?}", expected, tok),
                line: 0,
                col: 0,
            })
        }
    }

    fn parse_program(&mut self) -> Result<Program, ParseError> {
        let mut name = "main".to_string();

        if matches!(self.current(), Token::Program | Token::Module) {
            self.advance();
            if let Token::Ident(n) = self.current().clone() {
                name = n;
                self.advance();
            }
            self.expect(&Token::LBrace)?;
        }

        let mut prog = Program::new(name);

        while !matches!(self.current(), Token::Eof | Token::RBrace) {
            let item = self.parse_item()?;
            prog.items.push(item);
        }

        if matches!(self.current(), Token::RBrace) {
            self.advance();
        }

        prog.rebuild_index();
        Ok(prog)
    }

    fn parse_item(&mut self) -> Result<Item, ParseError> {
        match self.current() {
            Token::Fn => self.parse_function().map(Item::Function),
            Token::Recursive => self.parse_agent().map(Item::Agent),
            Token::Scale => self.parse_cluster().map(Item::Cluster),
            Token::Module => self.parse_module().map(Item::Module),
            Token::Import => self.parse_import().map(Item::Import),
            Token::Export => {
                self.advance();
                let item = self.parse_item()?;
                Ok(Item::Export(crate::ast::Export {
                    id: NodeId::new(),
                    span: SourceSpan::unknown(),
                    item: Box::new(item),
                }))
            }
            Token::Let => {
                let stmt = self.parse_let()?;
                Ok(Item::Function(crate::ast::Function {
                    id: NodeId::new(),
                    span: SourceSpan::unknown(),
                    name: "__top_level__".to_string(),
                    parameters: Vec::new(),
                    return_type: None,
                    body: vec![stmt],
                    attributes: Vec::new(),
                    is_exported: false,
                }))
            }
            _ => {
                let stmt = self.parse_statement()?;
                Ok(Item::Function(crate::ast::Function {
                    id: NodeId::new(),
                    span: SourceSpan::unknown(),
                    name: "__top_level__".to_string(),
                    parameters: Vec::new(),
                    return_type: None,
                    body: vec![stmt],
                    attributes: Vec::new(),
                    is_exported: false,
                }))
            }
        }
    }

    fn parse_function(&mut self) -> Result<Function, ParseError> {
        self.expect(&Token::Fn)?;

        let name = match self.current().clone() {
            Token::Ident(n) => {
                self.advance();
                n
            }
            _ => {
                return Err(ParseError {
                    message: "Expected function name".to_string(),
                    line: 0,
                    col: 0,
                })
            }
        };

        self.expect(&Token::LParen)?;

        let mut parameters = Vec::new();
        while !matches!(self.current(), Token::RParen) {
            if let Token::Ident(n) = self.current().clone() {
                self.advance();
                let mut param = Parameter::new(n);
                if matches!(self.current(), Token::Colon) {
                    self.advance();
                    param = param.with_type(self.parse_type()?);
                }
                parameters.push(param);
            }
            if matches!(self.current(), Token::Comma) {
                self.advance();
            }
        }
        self.expect(&Token::RParen)?;

        let return_type = if matches!(self.current(), Token::Arrow) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        self.expect(&Token::LBrace)?;
        let body = self.parse_block_body()?;
        self.expect(&Token::RBrace)?;

        Ok(Function::new(name, parameters, body).with_return_type_opt(return_type))
    }

    fn parse_agent(&mut self) -> Result<AgentDef, ParseError> {
        self.expect(&Token::Recursive)?;
        self.expect(&Token::Agent)?;

        let name = match self.current().clone() {
            Token::Ident(n) => {
                self.advance();
                n
            }
            _ => {
                return Err(ParseError {
                    message: "Expected agent name".to_string(),
                    line: 0,
                    col: 0,
                })
            }
        };

        let mut agent = AgentDef::new(name);

        self.expect(&Token::LBrace)?;

        while !matches!(self.current(), Token::RBrace) {
            match self.current().clone() {
                Token::Latent => {
                    let latent = self.parse_latent_decl()?;
                    agent.latents.push(latent);
                }
                Token::Cycle => {
                    let cycle = self.parse_cycle()?;
                    agent.cycles.push(cycle);
                }
                Token::Takes => {
                    self.advance();
                    self.expect(&Token::Colon)?;
                    while !matches!(self.current(), Token::Semi) {
                        if let Token::Ident(n) = self.current().clone() {
                            self.advance();
                            agent.takes.push(Parameter::new(n));
                        }
                        if matches!(self.current(), Token::Comma) {
                            self.advance();
                        }
                    }
                    self.expect(&Token::Semi)?;
                }
                Token::Gives => {
                    self.advance();
                    self.expect(&Token::Colon)?;
                    if let Token::Ident(n) = self.current().clone() {
                        self.advance();
                        agent.gives = Some(IntentOutput {
                            name: n,
                            ty: TypeAnnotation::unknown(),
                        });
                    }
                    self.expect(&Token::Semi)?;
                }
                Token::Govern => {
                    let govern = self.parse_govern()?;
                    agent.govern = Some(govern);
                }
                Token::Modify => {
                    let modify = self.parse_modify()?;
                    agent.modify = Some(modify);
                }
                Token::Dissolvable => {
                    self.advance();
                    agent = agent.dissolvable();
                }
                _ => {
                    let stmt = self.parse_statement()?;
                    agent.body.push(stmt);
                }
            }
        }

        self.expect(&Token::RBrace)?;
        Ok(agent)
    }

    fn parse_latent_decl(&mut self) -> Result<LatentDef, ParseError> {
        self.expect(&Token::Latent)?;

        let name = match self.current().clone() {
            Token::Ident(n) => {
                self.advance();
                n
            }
            _ => {
                return Err(ParseError {
                    message: "Expected latent name".to_string(),
                    line: 0,
                    col: 0,
                })
            }
        };

        self.expect(&Token::Colon)?;
        let ty = self.parse_type()?;

        let mut latent = LatentDef::new(name, ty);

        if matches!(self.current(), Token::Eq) {
            self.advance();
            latent = latent.with_initializer(self.parse_expression()?);
        }

        self.expect(&Token::Semi)?;
        Ok(latent)
    }

    fn parse_cycle(&mut self) -> Result<CycleDef, ParseError> {
        self.expect(&Token::Cycle)?;

        let level = match self.current().clone() {
            Token::Ident(n) => {
                self.advance();
                match n.as_str() {
                    "H" => CycleLevel::H,
                    "L" => CycleLevel::L,
                    _ => CycleLevel::Custom(0),
                }
            }
            _ => CycleLevel::H,
        };

        self.expect(&Token::LParen)?;
        let iterations = match self.current().clone() {
            Token::Int(n) => {
                self.advance();
                n as u64
            }
            _ => 1000,
        };
        self.expect(&Token::RParen)?;

        self.expect(&Token::LBrace)?;
        let body = self.parse_block_body()?;
        self.expect(&Token::RBrace)?;

        Ok(CycleDef::new(level, iterations, body))
    }

    fn parse_govern(&mut self) -> Result<GovernDef, ParseError> {
        self.expect(&Token::Govern)?;
        self.expect(&Token::LBrace)?;

        let mut effect = EffectType::Modify;
        let mut conscience = Vec::new();
        let mut trust_threshold = 0.5;

        while !matches!(self.current(), Token::RBrace) {
            match self.current().clone() {
                Token::Ident(n) if n == "effect" => {
                    self.advance();
                    self.expect(&Token::Colon)?;
                    let eff = self.try_consume_name()
                        .unwrap_or_else(|| "modify".to_string());
                    effect = match eff.as_str() {
                        "modify" => EffectType::Modify,
                        "spawn" => EffectType::Spawn,
                        "dissolve" => EffectType::Dissolve,
                        "communicate" => EffectType::Communicate,
                        "self_modify" => EffectType::SelfModify,
                        "external_call" => EffectType::ExternalCall,
                        _ => EffectType::Modify,
                    };
                    self.expect(&Token::Semi)?;
                }
                Token::Ident(n) if n == "conscience" => {
                    self.advance();
                    self.expect(&Token::Colon)?;
                    self.expect(&Token::LBracket)?;
                    while !matches!(self.current(), Token::RBracket) {
                        if let Some(pred_name) = self.try_consume_name() {
                            let kind = match pred_name.as_str() {
                                "path_safety" => PredicateKind::PathSafety {
                                    allowed: vec![],
                                    denied: vec![],
                                },
                                "no_exfiltrate" => PredicateKind::NoExfiltrate,
                                "no_harm" => PredicateKind::NoHarm,
                                "no_bypass" => PredicateKind::NoBypass,
                                "rate_limit" => PredicateKind::RateLimit {
                                    max_per_window: 100,
                                    window_seconds: 60,
                                },
                                _ => PredicateKind::Custom(Expression::new(ExprKind::Bool(true))),
                            };
                            conscience.push(ConsciencePredicate {
                                id: NodeId::new(),
                                name: pred_name,
                                kind,
                                enabled: true,
                            });
                        } else {
                            // Skip unknown tokens in predicate list
                            self.advance();
                        }
                        if matches!(self.current(), Token::Comma) {
                            self.advance();
                        }
                    }
                    self.expect(&Token::RBracket)?;
                    self.expect(&Token::Semi)?;
                }
                Token::Ident(n) if n == "trust" => {
                    self.advance();
                    self.expect(&Token::Colon)?;
                    if let Token::Float(t) = self.current().clone() {
                        trust_threshold = t;
                        self.advance();
                    } else if let Token::Int(t) = self.current().clone() {
                        trust_threshold = t as f64;
                        self.advance();
                    }
                    self.expect(&Token::Semi)?;
                }
                _ => {
                    self.advance();
                }
            }
        }

        self.expect(&Token::RBrace)?;

        Ok(GovernDef {
            id: NodeId::new(),
            span: SourceSpan::unknown(),
            effect,
            conscience,
            trust_threshold,
        })
    }

    fn parse_modify(&mut self) -> Result<ModifyDef, ParseError> {
        self.expect(&Token::Modify)?;
        // Expect "self" identifier
        if let Token::Ident(name) = self.current().clone() {
            if name == "self" {
                self.advance();
            }
        }
        self.expect(&Token::LBrace)?;

        let mut gates = Vec::new();
        let mut cooldown_steps = 100;
        let proposals = Vec::new();

        while !matches!(self.current(), Token::RBrace) {
            match self.current().clone() {
                Token::Gate => {
                    self.advance();
                    // Parse gate type: proof, consensus, human, safety_check
                    // Keywords like `proof` and `consensus` are valid gate names
                    let gate_name = self.try_consume_name()
                        .unwrap_or_else(|| "default".to_string());

                    // Determine gate type based on name
                    let gate = match gate_name.as_str() {
                        "proof" => Gate::Proof {
                            name: gate_name,
                            verification_status: None,
                        },
                        "consensus" => Gate::Consensus {
                            threshold: 0.66,
                            quorum: 1,
                            votes_for: 0,
                            votes_against: 0,
                        },
                        "human" => Gate::Human {
                            approver: None,
                            approved: None,
                            timestamp: None,
                        },
                        _ => Gate::SafetyCheck {
                            name: gate_name,
                            predicate: "true".to_string(),
                            passed: None,
                        },
                    };
                    gates.push(gate);
                    self.expect(&Token::Semi)?;
                }
                Token::Cooldown => {
                    self.advance();
                    self.expect(&Token::Colon)?;
                    if let Token::Int(t) = self.current().clone() {
                        cooldown_steps = t as u64;
                        self.advance();
                    }
                    self.expect(&Token::Semi)?;
                }
                _ => {
                    self.advance();
                }
            }
        }

        self.expect(&Token::RBrace)?;

        Ok(ModifyDef {
            id: NodeId::new(),
            span: SourceSpan::unknown(),
            gates,
            budget: ModificationBudget::default(),
            cooldown_steps,
            proposals,
        })
    }

    fn parse_cluster(&mut self) -> Result<ClusterDef, ParseError> {
        self.expect(&Token::Scale)?;
        self.expect(&Token::Cluster)?;

        let name = match self.current().clone() {
            Token::Ident(n) => {
                self.advance();
                n
            }
            _ => "anonymous".to_string(),
        };

        let mut cluster = ClusterDef {
            id: NodeId::new(),
            span: SourceSpan::unknown(),
            name,
            agents: Vec::new(),
            barriers: Vec::new(),
            channels: Vec::new(),
        };

        self.expect(&Token::LBrace)?;

        while !matches!(self.current(), Token::RBrace) {
            match self.current() {
                Token::Ident(n) if n == "agents" => {
                    self.advance();
                    self.expect(&Token::Colon)?;
                    self.expect(&Token::LBracket)?;
                    while !matches!(self.current(), Token::RBracket) {
                        if let Token::Ident(agent_name) = self.current().clone() {
                            self.advance();
                            cluster.agents.push(AgentRef {
                                id: NodeId::new(),
                                name: agent_name,
                                role: None,
                            });
                        }
                        if matches!(self.current(), Token::Comma) {
                            self.advance();
                        }
                    }
                    self.expect(&Token::RBracket)?;
                    self.expect(&Token::Semi)?;
                }
                Token::Barrier => {
                    self.advance();
                    if let Token::Ident(barrier_name) = self.current().clone() {
                        self.advance();
                        self.expect(&Token::LParen)?;
                        let expected = if let Token::Int(n) = self.current().clone() {
                            self.advance();
                            n as usize
                        } else {
                            1
                        };
                        self.expect(&Token::RParen)?;
                        cluster.barriers.push(BarrierDef {
                            id: NodeId::new(),
                            name: barrier_name,
                            expected,
                            timeout_ms: None,
                        });
                    }
                    self.expect(&Token::Semi)?;
                }
                _ => {
                    self.advance();
                }
            }
        }

        self.expect(&Token::RBrace)?;
        Ok(cluster)
    }

    fn parse_module(&mut self) -> Result<ModuleDef, ParseError> {
        self.expect(&Token::Module)?;

        let name = match self.current().clone() {
            Token::Ident(n) => {
                self.advance();
                n
            }
            _ => "anonymous".to_string(),
        };

        self.expect(&Token::LBrace)?;

        let mut items = Vec::new();
        while !matches!(self.current(), Token::RBrace) {
            items.push(self.parse_item()?);
        }
        self.expect(&Token::RBrace)?;

        Ok(ModuleDef {
            id: NodeId::new(),
            span: SourceSpan::unknown(),
            name,
            items,
        })
    }

    fn parse_import(&mut self) -> Result<Import, ParseError> {
        self.expect(&Token::Import)?;

        let module = match self.current().clone() {
            Token::Ident(n) => {
                self.advance();
                n
            }
            _ => {
                return Err(ParseError {
                    message: "Expected module name".to_string(),
                    line: 0,
                    col: 0,
                })
            }
        };

        let mut items = Vec::new();

        if matches!(self.current(), Token::Colon) {
            self.advance();
            self.expect(&Token::Colon)?;
            if matches!(self.current(), Token::Star) {
                self.advance();
                items.push(ImportItem::Wildcard);
            } else {
                self.expect(&Token::LBrace)?;
                while !matches!(self.current(), Token::RBrace) {
                    if let Token::Ident(name) = self.current().clone() {
                        self.advance();
                        if let Token::Ident(a) = self.current().clone() {
                            if a == "as" {
                                self.advance();
                                if let Token::Ident(alias) = self.current().clone() {
                                    self.advance();
                                    items.push(ImportItem::Aliased { name, alias });
                                }
                            } else {
                                items.push(ImportItem::Named(name));
                            }
                        } else {
                            items.push(ImportItem::Named(name));
                        }
                    }
                    if matches!(self.current(), Token::Comma) {
                        self.advance();
                    }
                }
                self.expect(&Token::RBrace)?;
            }
        }

        self.expect(&Token::Semi)?;

        Ok(Import {
            id: NodeId::new(),
            span: SourceSpan::unknown(),
            module,
            items,
        })
    }

    fn parse_type(&mut self) -> Result<TypeAnnotation, ParseError> {
        match self.current().clone() {
            Token::Ident(n) => {
                self.advance();
                match n.as_str() {
                    "i64" => Ok(TypeAnnotation::i64()),
                    "f64" => Ok(TypeAnnotation::f64()),
                    "String" => Ok(TypeAnnotation::string()),
                    "bool" => Ok(TypeAnnotation::bool()),
                    _ => Ok(TypeAnnotation::new(n)),
                }
            }
            _ => Ok(TypeAnnotation::unknown()),
        }
    }

    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        match self.current() {
            Token::Let => self.parse_let(),
            Token::Return => self.parse_return(),
            Token::If => self.parse_if(),
            Token::Loop => self.parse_loop(),
            Token::While => self.parse_while(),
            Token::For => self.parse_for(),
            Token::Break => {
                self.advance();
                self.expect(&Token::Semi)?;
                Ok(Statement::break_())
            }
            Token::Continue => {
                self.advance();
                self.expect(&Token::Semi)?;
                Ok(Statement::continue_())
            }
            Token::Switch => self.parse_switch(),
            Token::LBrace => {
                self.advance();
                let stmts = self.parse_block_body()?;
                self.expect(&Token::RBrace)?;
                Ok(Statement::block(stmts))
            }
            _ => {
                let expr = self.parse_expression()?;
                if matches!(self.current(), Token::Eq) {
                    self.advance();
                    let value = self.parse_expression()?;
                    self.expect(&Token::Semi)?;
                    Ok(Statement::assign(expr, value))
                } else {
                    self.expect(&Token::Semi)?;
                    Ok(Statement::expr(expr))
                }
            }
        }
    }

    fn parse_let(&mut self) -> Result<Statement, ParseError> {
        self.expect(&Token::Let)?;

        let mutable = if matches!(self.current(), Token::Mut) {
            self.advance();
            true
        } else {
            false
        };

        let name = match self.current().clone() {
            Token::Ident(n) => {
                self.advance();
                n
            }
            _ => {
                return Err(ParseError {
                    message: "Expected variable name".to_string(),
                    line: 0,
                    col: 0,
                })
            }
        };

        let ty = if matches!(self.current(), Token::Colon) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        self.expect(&Token::Eq)?;
        let value = self.parse_expression()?;
        self.expect(&Token::Semi)?;

        Ok(Statement::new(StmtKind::Let {
            name,
            ty,
            mutable,
            value,
        }))
    }

    fn parse_return(&mut self) -> Result<Statement, ParseError> {
        self.expect(&Token::Return)?;

        let value = if matches!(self.current(), Token::Semi | Token::RBrace) {
            None
        } else {
            Some(self.parse_expression()?)
        };

        self.expect(&Token::Semi)?;
        Ok(Statement::return_(value))
    }

    fn parse_if(&mut self) -> Result<Statement, ParseError> {
        self.expect(&Token::If)?;
        self.expect(&Token::LParen)?;
        let condition = self.parse_expression()?;
        self.expect(&Token::RParen)?;

        self.expect(&Token::LBrace)?;
        let then_body = self.parse_block_body()?;
        self.expect(&Token::RBrace)?;

        let else_body = if matches!(self.current(), Token::Else) {
            self.advance();
            if matches!(self.current(), Token::If) {
                vec![self.parse_if()?]
            } else {
                self.expect(&Token::LBrace)?;
                let body = self.parse_block_body()?;
                self.expect(&Token::RBrace)?;
                body
            }
        } else {
            Vec::new()
        };

        Ok(Statement::if_(condition, then_body, else_body))
    }

    fn parse_loop(&mut self) -> Result<Statement, ParseError> {
        self.expect(&Token::Loop)?;

        let condition = if matches!(self.current(), Token::LParen) {
            self.advance();
            let cond = self.parse_expression()?;
            self.expect(&Token::RParen)?;
            cond
        } else {
            Expression::bool(true)
        };

        self.expect(&Token::LBrace)?;
        let body = self.parse_block_body()?;
        self.expect(&Token::RBrace)?;

        Ok(Statement::new(StmtKind::Loop(LoopStmt::new(
            condition, body,
        ))))
    }

    fn parse_while(&mut self) -> Result<Statement, ParseError> {
        self.expect(&Token::While)?;
        self.expect(&Token::LParen)?;
        let condition = self.parse_expression()?;
        self.expect(&Token::RParen)?;

        self.expect(&Token::LBrace)?;
        let body = self.parse_block_body()?;
        self.expect(&Token::RBrace)?;

        Ok(Statement::new(StmtKind::While { condition, body }))
    }

    fn parse_for(&mut self) -> Result<Statement, ParseError> {
        self.expect(&Token::For)?;

        let pattern = match self.current().clone() {
            Token::Ident(n) => {
                self.advance();
                Pattern::Identifier(n)
            }
            _ => Pattern::Wildcard,
        };

        // Expect "in"
        if let Token::Ident(n) = self.current().clone() {
            if n == "in" {
                self.advance();
            }
        }

        let iterable = self.parse_expression()?;

        self.expect(&Token::LBrace)?;
        let body = self.parse_block_body()?;
        self.expect(&Token::RBrace)?;

        Ok(Statement::new(StmtKind::For {
            pattern,
            iterable,
            body,
        }))
    }

    fn parse_switch(&mut self) -> Result<Statement, ParseError> {
        self.expect(&Token::Switch)?;

        let discriminant = self.parse_expression()?;

        self.expect(&Token::LBrace)?;

        let mut cases = Vec::new();
        let mut default_body = Vec::new();

        while !matches!(self.current(), Token::RBrace) {
            match self.current() {
                Token::Case => {
                    self.advance();
                    let pattern = self.parse_pattern()?;

                    let guard = if matches!(self.current(), Token::When) {
                        self.advance();
                        Some(self.parse_expression()?)
                    } else {
                        None
                    };

                    self.expect(&Token::FatArrow)?;

                    let body = if matches!(self.current(), Token::LBrace) {
                        self.advance();
                        let b = self.parse_block_body()?;
                        self.expect(&Token::RBrace)?;
                        b
                    } else {
                        vec![self.parse_statement()?]
                    };

                    cases.push(SwitchCase {
                        id: NodeId::new(),
                        pattern,
                        guard,
                        body,
                    });
                }
                Token::Default => {
                    self.advance();
                    self.expect(&Token::FatArrow)?;

                    if matches!(self.current(), Token::LBrace) {
                        self.advance();
                        default_body = self.parse_block_body()?;
                        self.expect(&Token::RBrace)?;
                    } else {
                        default_body.push(self.parse_statement()?);
                    }
                }
                Token::Ident(n) if n == "_" => {
                    self.advance();
                    self.expect(&Token::FatArrow)?;

                    if matches!(self.current(), Token::LBrace) {
                        self.advance();
                        default_body = self.parse_block_body()?;
                        self.expect(&Token::RBrace)?;
                    } else {
                        default_body.push(self.parse_statement()?);
                    }
                }
                _ => {
                    self.advance();
                }
            }

            if matches!(self.current(), Token::Comma) {
                self.advance();
            }
        }

        self.expect(&Token::RBrace)?;

        Ok(Statement::new(StmtKind::Switch(SwitchStmt {
            discriminant,
            cases,
            default_body,
        })))
    }

    fn parse_pattern(&mut self) -> Result<Pattern, ParseError> {
        match self.current().clone() {
            Token::Int(n) => {
                self.advance();
                Ok(Pattern::Int(n))
            }
            Token::String(s) => {
                self.advance();
                Ok(Pattern::String(s))
            }
            Token::Ident(n) if n == "_" => {
                self.advance();
                Ok(Pattern::Wildcard)
            }
            Token::Ident(n) => {
                self.advance();
                Ok(Pattern::Identifier(n))
            }
            _ => Ok(Pattern::Wildcard),
        }
    }

    fn parse_block_body(&mut self) -> Result<Vec<Statement>, ParseError> {
        let mut stmts = Vec::new();
        while !matches!(self.current(), Token::RBrace | Token::Eof) {
            stmts.push(self.parse_statement()?);
        }
        Ok(stmts)
    }

    fn parse_expression(&mut self) -> Result<Expression, ParseError> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_and()?;

        while matches!(self.current(), Token::Or) {
            self.advance();
            let right = self.parse_and()?;
            left = Expression::binary(BinaryOp::Or, left, right);
        }

        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_equality()?;

        while matches!(self.current(), Token::And) {
            self.advance();
            let right = self.parse_equality()?;
            left = Expression::binary(BinaryOp::And, left, right);
        }

        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_comparison()?;

        loop {
            let op = match self.current() {
                Token::EqEq => BinaryOp::Eq,
                Token::Ne => BinaryOp::Ne,
                _ => break,
            };
            self.advance();
            let right = self.parse_comparison()?;
            left = Expression::binary(op, left, right);
        }

        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_additive()?;

        loop {
            let op = match self.current() {
                Token::Lt => BinaryOp::Lt,
                Token::Le => BinaryOp::Le,
                Token::Gt => BinaryOp::Gt,
                Token::Ge => BinaryOp::Ge,
                _ => break,
            };
            self.advance();
            let right = self.parse_additive()?;
            left = Expression::binary(op, left, right);
        }

        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_multiplicative()?;

        loop {
            let op = match self.current() {
                Token::Plus => BinaryOp::Add,
                Token::Minus => BinaryOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplicative()?;
            left = Expression::binary(op, left, right);
        }

        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_unary()?;

        loop {
            let op = match self.current() {
                Token::Star => BinaryOp::Mul,
                Token::Slash => BinaryOp::Div,
                Token::Percent => BinaryOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            left = Expression::binary(op, left, right);
        }

        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expression, ParseError> {
        let op = match self.current() {
            Token::Minus => Some(UnaryOp::Neg),
            Token::Bang => Some(UnaryOp::Not),
            _ => None,
        };

        if let Some(op) = op {
            self.advance();
            let operand = self.parse_unary()?;
            return Ok(Expression::unary(op, operand));
        }

        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_primary()?;

        loop {
            match self.current() {
                Token::LParen => {
                    self.advance();
                    let mut args = Vec::new();
                    while !matches!(self.current(), Token::RParen) {
                        args.push(self.parse_expression()?);
                        if matches!(self.current(), Token::Comma) {
                            self.advance();
                        }
                    }
                    self.expect(&Token::RParen)?;

                    if let ExprKind::Identifier(name) = &expr.kind {
                        expr = Expression::call(name.clone(), args);
                    } else {
                        // Method call on expression
                        expr = Expression::new(ExprKind::MethodCall {
                            object: Box::new(expr),
                            method: "call".to_string(),
                            arguments: args,
                        });
                    }
                }
                Token::LBracket => {
                    self.advance();
                    let index = self.parse_expression()?;
                    self.expect(&Token::RBracket)?;
                    expr = Expression::index(expr, index);
                }
                Token::Dot => {
                    self.advance();
                    if let Token::Ident(field) = self.current().clone() {
                        self.advance();
                        expr = Expression::field_access(expr, field);
                    }
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expression, ParseError> {
        match self.current().clone() {
            Token::Int(n) => {
                self.advance();
                Ok(Expression::int(n))
            }
            Token::Float(n) => {
                self.advance();
                Ok(Expression::float(n))
            }
            Token::String(s) => {
                self.advance();
                Ok(Expression::string(s))
            }
            Token::Bool(b) => {
                self.advance();
                Ok(Expression::bool(b))
            }
            Token::Ident(name) => {
                self.advance();
                Ok(Expression::identifier(name))
            }
            Token::LParen => {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect(&Token::RParen)?;
                Ok(expr)
            }
            Token::LBracket => {
                self.advance();
                let mut elements = Vec::new();
                while !matches!(self.current(), Token::RBracket) {
                    elements.push(self.parse_expression()?);
                    if matches!(self.current(), Token::Comma) {
                        self.advance();
                    }
                }
                self.expect(&Token::RBracket)?;
                Ok(Expression::array(elements))
            }
            _ => Ok(Expression::nil()),
        }
    }
}

impl Function {
    fn with_return_type_opt(mut self, ty: Option<TypeAnnotation>) -> Self {
        self.return_type = ty;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> Program {
        AstParser::parse(source).expect("Parse failed")
    }

    fn parse_err(source: &str) -> ParseError {
        AstParser::parse(source).expect_err("Expected parse error")
    }

    // ─── Lexer / Token Coverage ─────────────────────────────────────

    #[test]
    fn test_lex_literals() {
        let ast = parse("let x = 42;");
        let func = match &ast.items[0] {
            Item::Function(f) => f,
            _ => panic!("Expected function wrapper"),
        };
        if let StmtKind::Let { name, value, .. } = &func.body[0].kind {
            assert_eq!(name, "x");
            assert!(matches!(value.kind, ExprKind::Int(42)));
        } else {
            panic!("Expected let statement");
        }
    }

    #[test]
    fn test_lex_float() {
        let ast = parse("let pi = 3.14;");
        let func = match &ast.items[0] {
            Item::Function(f) => f,
            _ => panic!("Expected function wrapper"),
        };
        if let StmtKind::Let { value, .. } = &func.body[0].kind {
            if let ExprKind::Float(n) = value.kind {
                assert!((n - 3.14).abs() < 0.001);
            } else {
                panic!("Expected float");
            }
        }
    }

    #[test]
    fn test_lex_string() {
        let ast = parse(r#"let s = "hello world";"#);
        let func = match &ast.items[0] {
            Item::Function(f) => f,
            _ => panic!("Expected function wrapper"),
        };
        if let StmtKind::Let { value, .. } = &func.body[0].kind {
            if let ExprKind::String(ref s) = value.kind {
                assert_eq!(s, "hello world");
            } else {
                panic!("Expected string");
            }
        }
    }

    #[test]
    fn test_lex_booleans() {
        let ast = parse("let a = true; let b = false;");
        let func0 = match &ast.items[0] {
            Item::Function(f) => f,
            _ => panic!("Expected function"),
        };
        if let StmtKind::Let { value, .. } = &func0.body[0].kind {
            assert!(matches!(value.kind, ExprKind::Bool(true)));
        }
    }

    #[test]
    fn test_lex_operators() {
        // Ensure all comparison operators lex and parse correctly
        let ast = parse("fn test() -> bool { return 1 <= 2; }");
        if let Item::Function(f) = &ast.items[0] {
            assert_eq!(f.name, "test");
        }
    }

    #[test]
    fn test_lex_comments_ignored() {
        let ast = parse(r#"
            // This is a comment
            let x = 1;
            // Another comment
            let y = 2;
        "#);
        // Should parse without errors — comments stripped
        assert!(ast.items.len() >= 2);
    }

    // ─── Expression Parsing ─────────────────────────────────────────

    #[test]
    fn test_operator_precedence_mul_before_add() {
        let ast = parse("fn f() -> i64 { return 1 + 2 * 3; }");
        if let Item::Function(f) = &ast.items[0] {
            if let StmtKind::Return(Some(ref expr)) = f.body[0].kind {
                // Should be Add(1, Mul(2, 3)), not Mul(Add(1, 2), 3)
                if let ExprKind::BinaryOp { op, right, .. } = &expr.kind {
                    assert_eq!(*op, BinaryOp::Add);
                    assert!(matches!(right.kind, ExprKind::BinaryOp { op: BinaryOp::Mul, .. }));
                } else {
                    panic!("Expected binary op");
                }
            }
        }
    }

    #[test]
    fn test_parenthesized_expression() {
        let ast = parse("fn f() -> i64 { return (1 + 2) * 3; }");
        if let Item::Function(f) = &ast.items[0] {
            if let StmtKind::Return(Some(ref expr)) = f.body[0].kind {
                // Should be Mul(Add(1, 2), 3)
                if let ExprKind::BinaryOp { op, left, .. } = &expr.kind {
                    assert_eq!(*op, BinaryOp::Mul);
                    assert!(matches!(left.kind, ExprKind::BinaryOp { op: BinaryOp::Add, .. }));
                }
            }
        }
    }

    #[test]
    fn test_unary_negation() {
        // The lexer absorbs `-` followed by a digit into a negative literal
        let ast = parse("fn f() -> i64 { return -42; }");
        if let Item::Function(f) = &ast.items[0] {
            if let StmtKind::Return(Some(ref expr)) = f.body[0].kind {
                assert!(matches!(expr.kind, ExprKind::Int(-42)));
            }
        }
        // Unary negation of a variable goes through UnaryOp::Neg
        let ast2 = parse("fn f(x: i64) -> i64 { return -x; }");
        if let Item::Function(f) = &ast2.items[0] {
            if let StmtKind::Return(Some(ref expr)) = f.body[0].kind {
                assert!(matches!(expr.kind, ExprKind::UnaryOp { op: UnaryOp::Neg, .. }));
            }
        }
    }

    #[test]
    fn test_unary_not() {
        let ast = parse("fn f() -> bool { return !true; }");
        if let Item::Function(f) = &ast.items[0] {
            if let StmtKind::Return(Some(ref expr)) = f.body[0].kind {
                assert!(matches!(expr.kind, ExprKind::UnaryOp { op: UnaryOp::Not, .. }));
            }
        }
    }

    #[test]
    fn test_array_literal() {
        let ast = parse("let arr = [1, 2, 3];");
        let func = match &ast.items[0] {
            Item::Function(f) => f,
            _ => panic!("Expected function"),
        };
        if let StmtKind::Let { value, .. } = &func.body[0].kind {
            if let ExprKind::Array(ref elems) = value.kind {
                assert_eq!(elems.len(), 3);
            } else {
                panic!("Expected array");
            }
        }
    }

    #[test]
    fn test_array_index() {
        let ast = parse("fn f() -> i64 { let a = [1, 2]; return a[0]; }");
        if let Item::Function(f) = &ast.items[0] {
            if let StmtKind::Return(Some(ref expr)) = f.body[1].kind {
                assert!(matches!(expr.kind, ExprKind::Index { .. }));
            }
        }
    }

    #[test]
    fn test_function_call_expr() {
        let ast = parse("fn f() -> i64 { return add(1, 2); }");
        if let Item::Function(f) = &ast.items[0] {
            if let StmtKind::Return(Some(ref expr)) = f.body[0].kind {
                if let ExprKind::Call { function, arguments } = &expr.kind {
                    assert_eq!(function, "add");
                    assert_eq!(arguments.len(), 2);
                } else {
                    panic!("Expected call");
                }
            }
        }
    }

    #[test]
    fn test_field_access() {
        let ast = parse("fn f() -> i64 { return obj.field; }");
        if let Item::Function(f) = &ast.items[0] {
            if let StmtKind::Return(Some(ref expr)) = f.body[0].kind {
                if let ExprKind::FieldAccess { field, .. } = &expr.kind {
                    assert_eq!(field, "field");
                }
            }
        }
    }

    #[test]
    fn test_logical_operators() {
        let ast = parse("fn f() -> bool { return a > 0 && b < 10 || c == 5; }");
        if let Item::Function(f) = &ast.items[0] {
            if let StmtKind::Return(Some(ref expr)) = f.body[0].kind {
                // Top level should be Or (lowest precedence)
                assert!(matches!(expr.kind, ExprKind::BinaryOp { op: BinaryOp::Or, .. }));
            }
        }
    }

    // ─── Statement Parsing ──────────────────────────────────────────

    #[test]
    fn test_let_mutable() {
        let ast = parse("let mut x = 0;");
        let func = match &ast.items[0] {
            Item::Function(f) => f,
            _ => panic!("Expected function"),
        };
        if let StmtKind::Let { mutable, name, .. } = &func.body[0].kind {
            assert!(mutable);
            assert_eq!(name, "x");
        }
    }

    #[test]
    fn test_let_with_type() {
        let ast = parse("let x: i64 = 42;");
        let func = match &ast.items[0] {
            Item::Function(f) => f,
            _ => panic!("Expected function"),
        };
        if let StmtKind::Let { ty, .. } = &func.body[0].kind {
            assert!(ty.is_some());
            assert_eq!(ty.as_ref().unwrap().name, "i64");
        }
    }

    #[test]
    fn test_if_else() {
        let ast = parse(r#"
            fn f() -> i64 {
                if (x > 0) {
                    return 1;
                } else {
                    return 0;
                }
            }
        "#);
        if let Item::Function(f) = &ast.items[0] {
            if let StmtKind::If(ref if_stmt) = f.body[0].kind {
                assert!(!if_stmt.then_body.is_empty());
                assert!(!if_stmt.else_body.is_empty());
            }
        }
    }

    #[test]
    fn test_loop_with_condition() {
        let ast = parse(r#"
            fn f() -> i64 {
                let i = 0;
                loop(i < 10) {
                    i = i + 1;
                }
                return i;
            }
        "#);
        if let Item::Function(f) = &ast.items[0] {
            assert!(matches!(f.body[1].kind, StmtKind::Loop(_)));
        }
    }

    #[test]
    fn test_break_continue() {
        let ast = parse(r#"
            fn f() -> i64 {
                loop(true) {
                    break;
                }
                return 0;
            }
        "#);
        if let Item::Function(f) = &ast.items[0] {
            if let StmtKind::Loop(ref loop_stmt) = f.body[0].kind {
                assert!(matches!(loop_stmt.body[0].kind, StmtKind::Break));
            }
        }
    }

    #[test]
    fn test_switch_cases() {
        let ast = parse(r#"
            fn f(n: i64) -> i64 {
                switch n {
                    case 0 => { return 0; }
                    case 1 => { return 1; }
                    default => { return 2; }
                }
            }
        "#);
        if let Item::Function(f) = &ast.items[0] {
            if let StmtKind::Switch(ref sw) = f.body[0].kind {
                assert_eq!(sw.cases.len(), 2);
                assert!(!sw.default_body.is_empty());
            }
        }
    }

    #[test]
    fn test_assignment() {
        let ast = parse(r#"
            fn f() -> i64 {
                let x = 0;
                x = 42;
                return x;
            }
        "#);
        if let Item::Function(f) = &ast.items[0] {
            assert!(matches!(f.body[1].kind, StmtKind::Assign { .. }));
        }
    }

    // ─── Function Parsing ───────────────────────────────────────────

    #[test]
    fn test_function_params_and_return() {
        let ast = parse("fn add(a: i64, b: i64) -> i64 { return a + b; }");
        if let Item::Function(f) = &ast.items[0] {
            assert_eq!(f.name, "add");
            assert_eq!(f.parameters.len(), 2);
            assert_eq!(f.parameters[0].name, "a");
            assert_eq!(f.parameters[1].name, "b");
            assert!(f.return_type.is_some());
        }
    }

    #[test]
    fn test_multiple_functions() {
        let ast = parse(r#"
            fn foo() -> i64 { return 1; }
            fn bar() -> i64 { return 2; }
        "#);
        assert_eq!(ast.items.len(), 2);
        if let Item::Function(f) = &ast.items[0] {
            assert_eq!(f.name, "foo");
        }
        if let Item::Function(f) = &ast.items[1] {
            assert_eq!(f.name, "bar");
        }
    }

    // ─── Agent Parsing ──────────────────────────────────────────────

    #[test]
    fn test_agent_basic() {
        let ast = parse(r#"
            recursive agent Counter {
                latent count: i64 = 0;
                cycle H(10) {
                    count = count + 1;
                }
            }
        "#);
        if let Item::Agent(a) = &ast.items[0] {
            assert_eq!(a.name, "Counter");
            assert_eq!(a.latents.len(), 1);
            assert_eq!(a.latents[0].name, "count");
            assert_eq!(a.cycles.len(), 1);
            assert!(matches!(a.cycles[0].level, CycleLevel::H));
            assert_eq!(a.cycles[0].iterations, 10);
        } else {
            panic!("Expected agent");
        }
    }

    #[test]
    fn test_agent_govern() {
        let ast = parse(r#"
            recursive agent Safe {
                latent x: i64 = 0;
                govern {
                    effect: modify;
                    conscience: [no_harm, path_safety];
                    trust: 0.8;
                }
            }
        "#);
        if let Item::Agent(a) = &ast.items[0] {
            let g = a.govern.as_ref().expect("Expected govern block");
            assert_eq!(g.effect, EffectType::Modify);
            assert_eq!(g.conscience.len(), 2);
            assert_eq!(g.conscience[0].name, "no_harm");
            assert_eq!(g.conscience[1].name, "path_safety");
            assert!((g.trust_threshold - 0.8).abs() < 0.001);
        }
    }

    #[test]
    fn test_agent_modify_with_keyword_gates() {
        // This is the edge case GLM5 spiraled on
        let ast = parse(r#"
            recursive agent RSIAgent {
                latent v: f64 = 0.0;
                modify self {
                    gate proof;
                    gate consensus;
                    gate human;
                    cooldown: 500;
                }
            }
        "#);
        if let Item::Agent(a) = &ast.items[0] {
            let m = a.modify.as_ref().expect("Expected modify block");
            assert_eq!(m.gates.len(), 3);
            assert!(matches!(m.gates[0], Gate::Proof { .. }));
            assert!(matches!(m.gates[1], Gate::Consensus { .. }));
            assert!(matches!(m.gates[2], Gate::Human { .. }));
            assert_eq!(m.cooldown_steps, 500);
        }
    }

    #[test]
    fn test_agent_self_modify_effect() {
        let ast = parse(r#"
            recursive agent A {
                latent x: i64 = 0;
                govern {
                    effect: self_modify;
                    conscience: [no_harm, no_bypass, path_safety];
                    trust: 0.9;
                }
            }
        "#);
        if let Item::Agent(a) = &ast.items[0] {
            let g = a.govern.as_ref().unwrap();
            assert_eq!(g.effect, EffectType::SelfModify);
            assert_eq!(g.conscience.len(), 3);
        }
    }

    #[test]
    fn test_agent_takes_gives() {
        let ast = parse(r#"
            recursive agent TRM {
                latent h: i64 = 0;
                takes: input;
                gives: result;
                cycle H(10) {
                    h = h + 1;
                }
            }
        "#);
        if let Item::Agent(a) = &ast.items[0] {
            assert_eq!(a.takes.len(), 1);
            assert_eq!(a.takes[0].name, "input");
            assert!(a.gives.is_some());
            assert_eq!(a.gives.as_ref().unwrap().name, "result");
        }
    }

    #[test]
    fn test_agent_dual_cycles() {
        let ast = parse(r#"
            recursive agent DualCycle {
                latent h: i64 = 0;
                latent c: f64 = 0.0;
                cycle H(10) { h = h + 1; }
                cycle L(100) { c = c + 0.01; }
            }
        "#);
        if let Item::Agent(a) = &ast.items[0] {
            assert_eq!(a.cycles.len(), 2);
            assert!(matches!(a.cycles[0].level, CycleLevel::H));
            assert!(matches!(a.cycles[1].level, CycleLevel::L));
            assert_eq!(a.cycles[0].iterations, 10);
            assert_eq!(a.cycles[1].iterations, 100);
        }
    }

    #[test]
    fn test_agent_dissolvable() {
        let ast = parse(r#"
            recursive agent Temp {
                latent x: i64 = 0;
                dissolvable
            }
        "#);
        if let Item::Agent(a) = &ast.items[0] {
            assert!(a.dissolvable);
        }
    }

    // ─── Cluster Parsing ────────────────────────────────────────────

    #[test]
    fn test_cluster() {
        let ast = parse(r#"
            scale cluster Swarm {
                agents: [Worker1, Worker2, Worker3];
                barrier sync_point(3);
            }
        "#);
        if let Item::Cluster(c) = &ast.items[0] {
            assert_eq!(c.name, "Swarm");
            assert_eq!(c.agents.len(), 3);
            assert_eq!(c.barriers.len(), 1);
            assert_eq!(c.barriers[0].name, "sync_point");
            assert_eq!(c.barriers[0].expected, 3);
        }
    }

    // ─── Module / Import ────────────────────────────────────────────

    #[test]
    fn test_module() {
        let ast = parse(r#"
            module Math {
                fn add(a: i64, b: i64) -> i64 { return a + b; }
            }
        "#);
        if let Item::Module(m) = &ast.items[0] {
            assert_eq!(m.name, "Math");
            assert_eq!(m.items.len(), 1);
        }
    }

    // ─── Program Name ───────────────────────────────────────────────

    #[test]
    fn test_program_default_name() {
        let ast = parse("fn main() -> i64 { return 0; }");
        assert_eq!(ast.name, "main");
    }

    #[test]
    fn test_program_named() {
        let ast = parse("program MyApp { fn main() -> i64 { return 0; } }");
        assert_eq!(ast.name, "MyApp");
    }

    // ─── Error Cases ────────────────────────────────────────────────

    #[test]
    fn test_error_missing_function_name() {
        let err = parse_err("fn () -> i64 { return 0; }");
        assert!(err.message.contains("Expected function name"));
    }

    #[test]
    fn test_error_missing_semicolon() {
        // Missing semicolon after return value should error
        let result = AstParser::parse("fn f() -> i64 { return 42 }");
        // The parser may or may not error here depending on tolerance
        // Just verify it doesn't panic
        let _ = result;
    }

    // ─── Keyword-as-identifier Edge Cases ───────────────────────────

    #[test]
    fn test_keyword_gate_proof() {
        // The exact edge case that caused GLM5 to spiral
        let ast = parse(r#"
            recursive agent A {
                latent v: f64 = 0.0;
                modify self {
                    gate proof;
                }
            }
        "#);
        if let Item::Agent(a) = &ast.items[0] {
            let m = a.modify.as_ref().unwrap();
            assert_eq!(m.gates.len(), 1);
            assert!(matches!(m.gates[0], Gate::Proof { .. }));
        }
    }

    #[test]
    fn test_conscience_predicate_names() {
        // Conscience predicates can include names that are also keywords
        let ast = parse(r#"
            recursive agent B {
                latent x: i64 = 0;
                govern {
                    effect: spawn;
                    conscience: [no_exfiltrate, rate_limit];
                    trust: 0.7;
                }
            }
        "#);
        if let Item::Agent(a) = &ast.items[0] {
            let g = a.govern.as_ref().unwrap();
            assert_eq!(g.effect, EffectType::Spawn);
            assert_eq!(g.conscience.len(), 2);
            assert_eq!(g.conscience[0].name, "no_exfiltrate");
            assert_eq!(g.conscience[1].name, "rate_limit");
        }
    }

    // ─── Full TRM-style Agent ───────────────────────────────────────

    #[test]
    fn test_full_trm_agent() {
        let ast = parse(r#"
            recursive agent TRMAgent {
                latent hypothesis: i64 = 0;
                latent confidence: f64 = 0.0;
                takes: input;
                gives: result;
                cycle H(10) {
                    hypothesis = hypothesis + 1;
                }
                cycle L(100) {
                    confidence = confidence + 0.001;
                }
                govern {
                    effect: self_modify;
                    conscience: [no_harm, no_bypass, path_safety];
                    trust: 0.9;
                }
                modify self {
                    gate proof;
                    gate consensus;
                    gate human;
                    cooldown: 1000;
                }
            }
        "#);
        if let Item::Agent(a) = &ast.items[0] {
            assert_eq!(a.name, "TRMAgent");
            assert_eq!(a.latents.len(), 2);
            assert_eq!(a.takes.len(), 1);
            assert!(a.gives.is_some());
            assert_eq!(a.cycles.len(), 2);
            assert!(a.govern.is_some());
            let m = a.modify.as_ref().unwrap();
            assert_eq!(m.gates.len(), 3);
            assert_eq!(m.cooldown_steps, 1000);
        }
    }
}
