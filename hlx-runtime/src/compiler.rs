use crate::{Bytecode, Opcode, Value};
use std::collections::HashMap;

pub struct Compiler {
    bytecode: Bytecode,
    strings: HashMap<String, u32>,
    functions: HashMap<String, (u32, u32)>,
    current_function: Option<String>,
    patch_points: Vec<(usize, String)>,
    variables: HashMap<String, u8>,
    next_var_reg: u8,
    next_tmp_reg: u8,
}

#[derive(Debug, Clone)]
pub struct CompileError {
    pub message: String,
    pub line: usize,
}

impl Compiler {
    pub fn new() -> Self {
        Compiler {
            bytecode: Bytecode::new(),
            strings: HashMap::new(),
            functions: HashMap::new(),
            current_function: None,
            patch_points: Vec::new(),
            variables: HashMap::new(),
            next_var_reg: 0,
            next_tmp_reg: 20,
        }
    }

    pub fn compile(source: &str) -> Result<(Bytecode, HashMap<String, (u32, u32)>), CompileError> {
        let mut compiler = Compiler::new();
        compiler.compile_source(source)?;
        Ok((compiler.bytecode, compiler.functions))
    }

    fn compile_source(&mut self, source: &str) -> Result<(), CompileError> {
        let tokens = self.tokenize(source);
        self.compile_program(&tokens)?;
        self.patch_function_calls()?;
        Ok(())
    }

    fn get_or_add_string(&mut self, s: &str) -> u32 {
        if let Some(&idx) = self.strings.get(s) {
            idx
        } else {
            let idx = self.bytecode.add_string(s.to_string());
            self.strings.insert(s.to_string(), idx);
            idx
        }
    }

    fn emit(&mut self, op: Opcode) {
        self.bytecode.emit(op);
    }

    fn emit_u8(&mut self, v: u8) {
        self.bytecode.emit_u8(v);
    }

    fn emit_u32(&mut self, v: u32) {
        self.bytecode.emit_u32(v);
    }

    fn current_pc(&self) -> usize {
        self.bytecode.code.len()
    }

    fn patch_jump(&mut self, jump_pos: usize, target_pc: usize) -> Result<(), CompileError> {
        if jump_pos + 4 > self.bytecode.code.len() {
            return Err(CompileError {
                message: format!(
                    "Jump patch position {} out of bounds (code length {})",
                    jump_pos,
                    self.bytecode.code.len()
                ),
                line: 0,
            });
        }
        let mut code = self.bytecode.code.clone();
        code[jump_pos..jump_pos + 4].copy_from_slice(&(target_pc as u32).to_le_bytes());
        self.bytecode.code = code;
        Ok(())
    }

    fn tokenize(&self, source: &str) -> Vec<Token> {
        let mut tokens = Vec::new();
        let mut pos = 0;
        let chars: Vec<char> = source.chars().collect();

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
                tokens.push(Token::String(s));
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
                    tokens.push(Token::Float(num_str.parse().unwrap_or(0.0)));
                } else {
                    tokens.push(Token::Int(num_str.parse().unwrap_or(0)));
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
                    "true" => Token::Bool(true),
                    "false" => Token::Bool(false),
                    "recursive" => Token::Recursive,
                    "agent" => Token::Agent,
                    "latent" => Token::Latent,
                    "cycle" => Token::Cycle,
                    "halt" => Token::Halt,
                    "govern" => Token::Govern,
                    "outer" => Token::Outer,
                    "inner" => Token::Inner,
                    "when" => Token::When,
                    _ => Token::Ident(word),
                };
                tokens.push(tok);
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
                ',' => Token::Comma,
                ':' => Token::Colon,
                '+' => Token::Plus,
                '-' => Token::Minus,
                '*' => Token::Star,
                '/' => Token::Slash,
                '%' => Token::Percent,
                '=' => {
                    if pos + 1 < chars.len() && chars[pos + 1] == '=' {
                        pos += 1;
                        Token::EqEq
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
                _ => Token::Unknown(c),
            };
            tokens.push(tok);
            pos += 1;
        }

        tokens.push(Token::Eof);
        tokens
    }

    fn compile_program(&mut self, tokens: &[Token]) -> Result<(), CompileError> {
        let mut pos = 0;

        if matches!(tokens[pos], Token::Program | Token::Module) {
            pos += 1;
            if let Token::Ident(name) = &tokens[pos] {
                self.current_function = Some(name.clone());
                pos += 1;
            }
            pos += 1;
        }

        while !matches!(tokens[pos], Token::Eof) {
            pos = self.compile_item(tokens, pos)?;
        }

        self.emit(Opcode::Halt);
        Ok(())
    }

    fn compile_item(&mut self, tokens: &[Token], mut pos: usize) -> Result<usize, CompileError> {
        match &tokens[pos] {
            Token::Fn => {
                pos += 1;
                let name = if let Token::Ident(n) = &tokens[pos] {
                    n.clone()
                } else {
                    return Err(CompileError {
                        message: "Expected function name".to_string(),
                        line: 0,
                    });
                };
                pos += 1;
                pos += 1;

                let mut param_names: Vec<String> = Vec::new();
                while !matches!(tokens[pos], Token::RParen) {
                    if let Token::Ident(p) = &tokens[pos] {
                        param_names.push(p.clone());
                        pos += 1;
                        if matches!(tokens[pos], Token::Colon) {
                            pos += 1;
                            pos += 1;
                        }
                    }
                    if matches!(tokens[pos], Token::Comma) {
                        pos += 1;
                    }
                }
                pos += 1;

                let params = param_names.len() as u32;

                if matches!(tokens[pos], Token::Colon) {
                    pos += 1;
                    pos += 1;
                    if matches!(tokens[pos], Token::LBracket) {
                        pos += 1;
                        while !matches!(tokens[pos], Token::RBracket) {
                            pos += 1;
                        }
                        pos += 1;
                    }
                    if matches!(tokens[pos], Token::Arrow) {
                        pos += 1;
                    }
                }

                pos += 1;

                let is_main = name == "main";
                let start_pc: u32;

                if !is_main {
                    self.emit(Opcode::Jump);
                    let skip_jump = self.current_pc();
                    self.emit_u32(0);

                    start_pc = self.current_pc() as u32;
                    self.functions.insert(name.clone(), (start_pc, params));

                    let old_vars = self.variables.clone();
                    self.variables.clear();

                    for (i, param_name) in param_names.iter().enumerate() {
                        self.variables.insert(param_name.clone(), (i + 1) as u8);
                    }

                    pos = self.compile_block(tokens, pos)?;

                    self.emit(Opcode::Return);

                    self.variables = old_vars;

                    let end_pc = self.current_pc();
                    self.patch_jump(skip_jump, end_pc)?;
                } else {
                    start_pc = self.current_pc() as u32;
                    self.functions.insert(name.clone(), (start_pc, params));

                    let old_vars = self.variables.clone();
                    self.variables.clear();

                    for (i, param_name) in param_names.iter().enumerate() {
                        self.variables.insert(param_name.clone(), (i + 1) as u8);
                    }

                    pos = self.compile_block(tokens, pos)?;

                    self.emit(Opcode::Return);

                    self.variables = old_vars;
                }
            }
            Token::Let => {
                pos += 1;
                pos = self.compile_let(tokens, pos)?;
            }
            Token::Agent => {
                pos = self.compile_agent(tokens, pos)?;
            }
            _ => {
                pos += 1;
            }
        }
        Ok(pos)
    }

    fn compile_block(&mut self, tokens: &[Token], mut pos: usize) -> Result<usize, CompileError> {
        if matches!(tokens[pos], Token::LBrace) {
            pos += 1;
        }

        while !matches!(tokens[pos], Token::RBrace | Token::Eof) {
            pos = self.compile_stmt(tokens, pos)?;
        }

        if matches!(tokens[pos], Token::RBrace) {
            pos += 1;
        }
        Ok(pos)
    }

    fn compile_stmt(&mut self, tokens: &[Token], mut pos: usize) -> Result<usize, CompileError> {
        match &tokens[pos] {
            Token::Let => {
                pos += 1;
                pos = self.compile_let(tokens, pos)?;
            }
            Token::Return => {
                pos += 1;
                if !matches!(tokens[pos], Token::Semi | Token::RBrace) {
                    pos = self.compile_expr(tokens, pos, 0)?;
                }
                self.emit(Opcode::Return);
                if matches!(tokens[pos], Token::Semi) {
                    pos += 1;
                }
            }
            Token::If => {
                pos += 1;
                pos = self.compile_if(tokens, pos)?;
            }
            Token::Loop => {
                pos += 1;
                pos = self.compile_loop(tokens, pos)?;
            }
            Token::Cycle => {
                pos += 1;
                pos = self.compile_cycle(tokens, pos)?;
            }
            Token::Halt => {
                pos += 1;
                pos = self.compile_halt(tokens, pos)?;
            }
            Token::Ident(_) => {
                if matches!(tokens[pos + 1], Token::Eq) && !matches!(tokens[pos + 2], Token::Eq) {
                    if let Token::Ident(name) = &tokens[pos] {
                        pos += 2;
                        if let Some(&reg) = self.variables.get(name) {
                            pos = self.compile_expr(tokens, pos, reg)?;
                        } else {
                            let reg = self.next_var_reg;
                            if self.next_var_reg >= 254 {
                                return Err(CompileError {
                                    message:
                                        "Register limit exceeded: too many variables (max 254)"
                                            .to_string(),
                                    line: 0,
                                });
                            }
                            self.next_var_reg += 1;
                            self.variables.insert(name.clone(), reg);
                            pos = self.compile_expr(tokens, pos, reg)?;
                        }
                    } else {
                        pos += 2;
                    }
                } else {
                    pos = self.compile_expr(tokens, pos, self.next_tmp_reg)?;
                }
                if matches!(tokens[pos], Token::Semi) {
                    pos += 1;
                }
            }
            _ => {
                pos += 1;
            }
        }
        Ok(pos)
    }

    fn compile_let(&mut self, tokens: &[Token], mut pos: usize) -> Result<usize, CompileError> {
        let name = if let Token::Ident(n) = &tokens[pos] {
            n.clone()
        } else {
            return Err(CompileError {
                message: "Expected variable name".to_string(),
                line: 0,
            });
        };
        pos += 1;

        if matches!(tokens[pos], Token::Colon) {
            pos += 1;
            pos += 1;
        }

        if matches!(tokens[pos], Token::Eq) {
            pos += 1;
            let reg = self.next_var_reg;
            if self.next_var_reg >= 254 {
                return Err(CompileError {
                    message: "Register limit exceeded: too many variables (max 254)".to_string(),
                    line: 0,
                });
            }
            self.next_var_reg += 1;
            self.variables.insert(name, reg);
            pos = self.compile_expr(tokens, pos, reg)?;
        }

        if matches!(tokens[pos], Token::Semi) {
            pos += 1;
        }

        Ok(pos)
    }

    fn op_precedence(op: &Token) -> u8 {
        match op {
            Token::Or => 1,
            Token::And => 2,
            Token::EqEq | Token::Ne => 3,
            Token::Lt | Token::Le | Token::Gt | Token::Ge => 4,
            Token::Plus | Token::Minus => 5,
            Token::Star | Token::Slash | Token::Percent => 6,
            _ => 0,
        }
    }

    fn is_binary_op(token: &Token) -> bool {
        matches!(
            token,
            Token::Plus
                | Token::Minus
                | Token::Star
                | Token::Slash
                | Token::Percent
                | Token::EqEq
                | Token::Ne
                | Token::Lt
                | Token::Le
                | Token::Gt
                | Token::Ge
                | Token::And
                | Token::Or
        )
    }

    fn compile_expr(
        &mut self,
        tokens: &[Token],
        mut pos: usize,
        dst: u8,
    ) -> Result<usize, CompileError> {
        self.compile_expr_with_precedence(tokens, pos, dst, 0)
    }

    fn compile_expr_with_precedence(
        &mut self,
        tokens: &[Token],
        mut pos: usize,
        dst: u8,
        min_prec: u8,
    ) -> Result<usize, CompileError> {
        pos = self.compile_expr_primary(tokens, pos, dst)?;

        while pos < tokens.len() && Self::is_binary_op(&tokens[pos]) {
            let op = tokens[pos].clone();
            let prec = Self::op_precedence(&op);

            if prec < min_prec {
                break;
            }

            pos += 1;

            let left_reg = dst;
            let right_reg = self.next_tmp_reg;
            if self.next_tmp_reg >= 254 {
                return Err(CompileError {
                    message:
                        "Register limit exceeded: expression too complex (max 254 temp registers)"
                            .to_string(),
                    line: 0,
                });
            }
            self.next_tmp_reg += 1;

            pos = self.compile_expr_with_precedence(tokens, pos, right_reg, prec + 1)?;

            match op {
                Token::Plus => {
                    self.emit(Opcode::Add);
                    self.emit_u8(dst);
                    self.emit_u8(left_reg);
                    self.emit_u8(right_reg);
                }
                Token::Minus => {
                    self.emit(Opcode::Sub);
                    self.emit_u8(dst);
                    self.emit_u8(left_reg);
                    self.emit_u8(right_reg);
                }
                Token::Star => {
                    self.emit(Opcode::Mul);
                    self.emit_u8(dst);
                    self.emit_u8(left_reg);
                    self.emit_u8(right_reg);
                }
                Token::Slash => {
                    self.emit(Opcode::Div);
                    self.emit_u8(dst);
                    self.emit_u8(left_reg);
                    self.emit_u8(right_reg);
                }
                Token::Percent => {
                    self.emit(Opcode::Mod);
                    self.emit_u8(dst);
                    self.emit_u8(left_reg);
                    self.emit_u8(right_reg);
                }
                Token::EqEq => {
                    self.emit(Opcode::Eq);
                    self.emit_u8(dst);
                    self.emit_u8(left_reg);
                    self.emit_u8(right_reg);
                }
                Token::Ne => {
                    self.emit(Opcode::Ne);
                    self.emit_u8(dst);
                    self.emit_u8(left_reg);
                    self.emit_u8(right_reg);
                }
                Token::Lt => {
                    self.emit(Opcode::Lt);
                    self.emit_u8(dst);
                    self.emit_u8(left_reg);
                    self.emit_u8(right_reg);
                }
                Token::Le => {
                    self.emit(Opcode::Le);
                    self.emit_u8(dst);
                    self.emit_u8(left_reg);
                    self.emit_u8(right_reg);
                }
                Token::Gt => {
                    self.emit(Opcode::Gt);
                    self.emit_u8(dst);
                    self.emit_u8(left_reg);
                    self.emit_u8(right_reg);
                }
                Token::Ge => {
                    self.emit(Opcode::Ge);
                    self.emit_u8(dst);
                    self.emit_u8(left_reg);
                    self.emit_u8(right_reg);
                }
                Token::And => {
                    self.emit(Opcode::And);
                    self.emit_u8(dst);
                    self.emit_u8(left_reg);
                    self.emit_u8(right_reg);
                }
                Token::Or => {
                    self.emit(Opcode::Or);
                    self.emit_u8(dst);
                    self.emit_u8(left_reg);
                    self.emit_u8(right_reg);
                }
                _ => {}
            }
        }

        Ok(pos)
    }

    fn compile_expr_primary(
        &mut self,
        tokens: &[Token],
        mut pos: usize,
        dst: u8,
    ) -> Result<usize, CompileError> {
        match &tokens[pos] {
            Token::Int(n) => {
                let idx = self.bytecode.add_constant(Value::I64(*n));
                self.emit(Opcode::Const);
                self.emit_u8(dst);
                self.emit_u32(idx);
                pos += 1;
            }
            Token::Float(n) => {
                let idx = self.bytecode.add_constant(Value::F64(*n));
                self.emit(Opcode::Const);
                self.emit_u8(dst);
                self.emit_u32(idx);
                pos += 1;
            }
            Token::Bool(b) => {
                let idx = self.bytecode.add_constant(Value::Bool(*b));
                self.emit(Opcode::Const);
                self.emit_u8(dst);
                self.emit_u32(idx);
                pos += 1;
            }
            Token::String(s) => {
                let idx = self.bytecode.add_constant(Value::String(s.clone()));
                self.emit(Opcode::Const);
                self.emit_u8(dst);
                self.emit_u32(idx);
                pos += 1;
            }
            Token::Ident(name) => {
                if matches!(tokens[pos + 1], Token::LParen) {
                    let func_name = name.clone();
                    pos += 1;
                    pos += 1;

                    let mut arg_count = 0u8;
                    let arg_base = 150u8;
                    while !matches!(tokens[pos], Token::RParen) {
                        pos = self.compile_expr(tokens, pos, arg_base + arg_count)?;
                        arg_count += 1;
                        if matches!(tokens[pos], Token::Comma) {
                            pos += 1;
                        }
                    }
                    pos += 1;

                    self.emit(Opcode::Call);
                    let name_idx = self.get_or_add_string(&func_name);
                    self.emit_u32(name_idx);
                    self.emit_u8(arg_count);
                    self.emit_u8(dst);

                    if !self.functions.contains_key(&func_name) {
                        let call_site = self.current_pc() - 7;
                        self.patch_points.push((call_site, func_name));
                    }
                } else {
                    if let Some(&src_reg) = self.variables.get(name) {
                        self.emit(Opcode::Move);
                        self.emit_u8(dst);
                        self.emit_u8(src_reg);
                    }
                    pos += 1;
                }
            }
            Token::LParen => {
                pos += 1;
                pos = self.compile_expr(tokens, pos, dst)?;
                pos += 1;
            }
            Token::Bang => {
                pos += 1;
                pos = self.compile_expr_primary(tokens, pos, dst)?;
                self.emit(Opcode::Not);
                self.emit_u8(dst);
                self.emit_u8(dst);
            }
            Token::Minus => {
                pos += 1;
                pos = self.compile_expr_primary(tokens, pos, dst)?;
                self.emit(Opcode::Neg);
                self.emit_u8(dst);
                self.emit_u8(dst);
            }
            _ => {
                pos += 1;
            }
        }
        Ok(pos)
    }

    fn compile_if(&mut self, tokens: &[Token], mut pos: usize) -> Result<usize, CompileError> {
        pos = self.compile_expr(tokens, pos, 10)?;

        self.emit(Opcode::JumpIfNot);
        self.emit_u8(10);
        let else_jump = self.current_pc();
        self.emit_u32(0);

        pos = self.compile_block(tokens, pos)?;

        self.emit(Opcode::Jump);
        let end_jump = self.current_pc();
        self.emit_u32(0);

        let else_start = self.current_pc();
        self.patch_jump(else_jump, else_start)?;

        if matches!(tokens[pos], Token::Else) {
            pos += 1;
            pos = self.compile_block(tokens, pos)?;
        }

        let end_pc = self.current_pc();
        self.patch_jump(end_jump, end_pc)?;

        Ok(pos)
    }

    fn compile_loop(&mut self, tokens: &[Token], mut pos: usize) -> Result<usize, CompileError> {
        let loop_start = self.current_pc();

        pos = self.compile_expr(tokens, pos, 10)?;

        self.emit(Opcode::JumpIfNot);
        self.emit_u8(10);
        let exit_jump = self.current_pc();
        self.emit_u32(0);

        pos = self.compile_block(tokens, pos)?;

        self.emit(Opcode::Jump);
        self.emit_u32(loop_start as u32);

        let exit_pc = self.current_pc();
        self.patch_jump(exit_jump, exit_pc)?;

        Ok(pos)
    }

    fn compile_cycle(&mut self, tokens: &[Token], mut pos: usize) -> Result<usize, CompileError> {
        pos += 1;

        let level = if let Token::Ident(l) = &tokens[pos] {
            l.clone()
        } else {
            "outer".to_string()
        };
        pos += 1;

        if matches!(tokens[pos], Token::LParen) {
            pos += 1;
            pos = self.compile_expr(tokens, pos, 0)?;
            if matches!(tokens[pos], Token::RParen) {
                pos += 1;
            }
        }

        let level_idx = self.get_or_add_string(&level);
        self.emit(Opcode::CycleBegin);
        self.emit_u8(0);
        self.emit_u8(0);

        pos = self.compile_block(tokens, pos)?;

        self.emit(Opcode::CycleEnd);
        self.emit_u8(0);

        Ok(pos)
    }

    fn compile_halt(&mut self, tokens: &[Token], mut pos: usize) -> Result<usize, CompileError> {
        pos += 1;

        if matches!(tokens[pos], Token::When) {
            pos += 1;
            pos = self.compile_expr(tokens, pos, 10)?;
        }

        self.emit(Opcode::AgentHalt);
        self.emit_u8(10);

        Ok(pos)
    }

    fn compile_agent(&mut self, tokens: &[Token], mut pos: usize) -> Result<usize, CompileError> {
        pos += 1;

        let name = if let Token::Ident(n) = &tokens[pos] {
            n.clone()
        } else {
            "Agent".to_string()
        };
        pos += 1;

        let name_idx = self.get_or_add_string(&name);

        pos += 1;

        self.emit(Opcode::AgentSpawn);
        self.emit_u32(name_idx);
        self.emit_u32(0);

        while !matches!(tokens[pos], Token::RBrace | Token::Eof) {
            if matches!(tokens[pos], Token::Latent) {
                pos += 1;
                if let Token::Ident(latent_name) = &tokens[pos] {
                    let _ = self.get_or_add_string(latent_name);
                }
                pos += 1;
                if matches!(tokens[pos], Token::Colon) {
                    pos += 1;
                }
                while !matches!(
                    tokens[pos],
                    Token::Latent | Token::Cycle | Token::Halt | Token::Govern | Token::RBrace
                ) {
                    pos += 1;
                }
            } else if matches!(tokens[pos], Token::Cycle) {
                pos = self.compile_cycle(tokens, pos)?;
            } else if matches!(tokens[pos], Token::Halt) {
                pos = self.compile_halt(tokens, pos)?;
            } else if matches!(tokens[pos], Token::Govern) {
                pos += 1;
                if matches!(tokens[pos], Token::LBrace) {
                    pos += 1;
                }
                while !matches!(tokens[pos], Token::RBrace) {
                    pos += 1;
                }
                pos += 1;
            } else {
                pos += 1;
            }
        }

        if matches!(tokens[pos], Token::RBrace) {
            pos += 1;
        }

        Ok(pos)
    }

    fn patch_function_calls(&mut self) -> Result<(), CompileError> {
        let patches = std::mem::take(&mut self.patch_points);

        for (call_site, func_name) in patches {
            if let Some(&(start_pc, _params)) = self.functions.get(&func_name) {
                self.patch_jump(call_site, start_pc as usize)?;
            } else {
                return Err(CompileError {
                    message: format!("Undefined function: {}", func_name),
                    line: 0,
                });
            }
        }

        Ok(())
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
enum Token {
    Eof,
    Ident(String),
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
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
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Semi,
    Comma,
    Colon,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,
    EqEq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Bang,
    And,
    Or,
    Amp,
    Pipe,
    Arrow,
    Recursive,
    Agent,
    Latent,
    Cycle,
    Halt,
    Govern,
    Outer,
    Inner,
    When,
    Unknown(char),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_simple() {
        let source = r#"
            program test {
                fn main() -> i64 {
                    return 42
                }
            }
        "#;

        let (bc, _funcs) = Compiler::compile(source).unwrap();
        let mut vm = crate::Vm::new();
        let result = vm.run(&bc).unwrap();

        assert_eq!(result, Value::I64(42));
    }

    #[test]
    fn test_compile_arithmetic() {
        let source = r#"
            program test {
                fn main() -> i64 {
                    let x = 10 + 32
                    return x
                }
            }
        "#;

        let (bc, _funcs) = Compiler::compile(source).unwrap();
        let mut vm = crate::Vm::new();
        let result = vm.run(&bc).unwrap();

        assert_eq!(result, Value::I64(42));
    }

    #[test]
    fn test_compile_loop() {
        let source = r#"
            program test {
                fn main() -> i64 {
                    let sum = 0
                    let i = 1
                    loop i < 6 {
                        sum = sum + i
                        i = i + 1
                    }
                    return sum
                }
            }
        "#;

        let (bc, _funcs) = Compiler::compile(source).unwrap();
        let mut vm = crate::Vm::new().with_max_steps(10000);
        let result = vm.run(&bc).unwrap();

        assert_eq!(result, Value::I64(15));
    }

    #[test]
    fn test_compile_if() {
        let source = r#"
            program test {
                fn main() -> i64 {
                    let x = 10
                    if x > 5 {
                        return 100
                    }
                    return 0
                }
            }
        "#;

        let (bc, _funcs) = Compiler::compile(source).unwrap();
        let mut vm = crate::Vm::new();
        let result = vm.run(&bc).unwrap();

        assert_eq!(result, Value::I64(100));
    }

    #[test]
    fn test_compile_recursive_cycle() {
        let source = r#"
            program test {
                fn main() -> i64 {
                    let h = 0
                    let l = 0
                    let result = 0
                    
                    loop h < 3 {
                        l = 0
                        loop l < 6 {
                            result = result + 1
                            l = l + 1
                        }
                        h = h + 1
                    }
                    
                    return result
                }
            }
        "#;

        let (bc, _funcs) = Compiler::compile(source).unwrap();
        let mut vm = crate::Vm::new().with_max_steps(100000);
        let result = vm.run(&bc).unwrap();

        assert_eq!(result, Value::I64(18));
    }

    #[test]
    fn test_compile_function_call() {
        let source = r#"
            program test {
                fn add(a: i64, b: i64) -> i64 {
                    return a + b
                }
                
                fn main() -> i64 {
                    let x = add(10, 32)
                    return x
                }
            }
        "#;

        let (bc, funcs) = Compiler::compile(source).unwrap();
        assert!(funcs.contains_key("add"));
        assert!(funcs.contains_key("main"));

        let mut vm = crate::Vm::new().with_max_steps(10000);
        vm.load_functions(&funcs);
        let result = vm.run(&bc).unwrap();

        assert_eq!(result, Value::I64(42));
    }

    #[test]
    fn test_compile_nested_calls() {
        let source = r#"
            program test {
                fn double(x: i64) -> i64 {
                    return x + x
                }
                
                fn quadruple(x: i64) -> i64 {
                    return double(double(x))
                }
                
                fn main() -> i64 {
                    return quadruple(5)
                }
            }
        "#;

        let (bc, funcs) = Compiler::compile(source).unwrap();
        let mut vm = crate::Vm::new().with_max_steps(10000);
        vm.load_functions(&funcs);
        let result = vm.run(&bc).unwrap();

        assert_eq!(result, Value::I64(20));
    }

    #[test]
    fn test_compile_recursive_function() {
        let source = r#"
            program test {
                fn fib(n: i64) -> i64 {
                    if n < 2 {
                        return n
                    }
                    return fib(n - 1) + fib(n - 2)
                }
                
                fn main() -> i64 {
                    return fib(10)
                }
            }
        "#;

        let (bc, funcs) = Compiler::compile(source).unwrap();
        let mut vm = crate::Vm::new().with_max_steps(100000);
        vm.load_functions(&funcs);
        let result = vm.run(&bc).unwrap();

        assert_eq!(result, Value::I64(55));
    }
}
