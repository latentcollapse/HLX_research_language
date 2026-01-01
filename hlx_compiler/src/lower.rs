//! Lowering Pass: AST → Instructions → Capsule

use crate::ast::*;
use hlx_core::{
    capsule::{Capsule, CapsuleMetadata},
    instruction::{Instruction, Register, TensorShape, DType},
    value::{Value, Contract},
    Result, HlxError,
};
use std::collections::HashMap;

/// Lower an AST Program to a Capsule
pub fn lower_to_capsule(program: &Program) -> Result<Capsule> {
    let mut ctx = LoweringContext::new();
    
    // Lower each block
    for block in &program.blocks {
        ctx.lower_block(block)?;
    }
    
    // Build capsule with metadata
    let metadata = CapsuleMetadata {
        source_file: Some(format!("{}.hlxl", program.name)),
        compiler_version: Some("0.1.0".to_string()),
        register_count: Some(ctx.next_reg),
        ..Default::default()
    };
    
    Ok(Capsule::with_metadata(ctx.instructions, metadata))
}

/// Lift a Capsule back to AST (for decompilation/bijection)
pub fn lift_from_capsule(capsule: &Capsule) -> Result<Program> {
    capsule.validate()?;
    
    let mut ctx = LiftingContext::new();
    ctx.lift_instructions(&capsule.instructions)?;
    
    Ok(ctx.build_program())
}

struct LoweringContext {
    instructions: Vec<Instruction>,
    next_reg: Register,
    scopes: Vec<HashMap<String, Register>>,
}

impl LoweringContext {
    fn new() -> Self {
        Self {
            instructions: Vec::new(),
            next_reg: 0,
            scopes: vec![HashMap::new()],
        }
    }
    
    fn alloc_reg(&mut self) -> Register {
        let reg = self.next_reg;
        self.next_reg += 1;
        reg
    }
    
    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }
    
    fn pop_scope(&mut self) {
        self.scopes.pop();
    }
    
    fn bind(&mut self, name: &str, reg: Register) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), reg);
        }
    }
    
    fn lookup(&self, name: &str) -> Option<Register> {
        for scope in self.scopes.iter().rev() {
            if let Some(&reg) = scope.get(name) {
                return Some(reg);
            }
        }
        None
    }
    
    fn emit(&mut self, inst: Instruction) {
        self.instructions.push(inst);
    }
    
    fn lower_block(&mut self, block: &Block) -> Result<()> {
        self.push_scope();
        for param in &block.params {
            let reg = self.alloc_reg();
            self.bind(param, reg);
        }
        for stmt in &block.body {
            self.lower_stmt(&stmt.node)?;
        }
        self.pop_scope();
        Ok(())
    }
    
    fn lower_stmt(&mut self, stmt: &Statement) -> Result<()> {
        match stmt {
            Statement::Let { name, value } => {
                let reg = self.lower_expr(&value.node)?;
                self.bind(name, reg);
            }
            Statement::Local { name, value } => {
                let reg = self.lower_expr(&value.node)?;
                self.bind(name, reg);
            }
            Statement::Return { value } => {
                let reg = self.lower_expr(&value.node)?;
                self.emit(Instruction::Return { val: reg });
            }
            Statement::If { condition, then_branch, else_branch } => {
                let _cond_reg = self.lower_expr(&condition.node)?;
                self.push_scope();
                for s in then_branch { self.lower_stmt(&s.node)?; }
                self.pop_scope();
                if let Some(eb) = else_branch {
                    self.push_scope();
                    for s in eb { self.lower_stmt(&s.node)?; }
                    self.pop_scope();
                }
            }
            Statement::While { condition, body } => {
                let _cond_reg = self.lower_expr(&condition.node)?;
                self.push_scope();
                for s in body { self.lower_stmt(&s.node)?; }
                self.pop_scope();
            }
            Statement::For { variable, iterator, body } => {
                let _iter_reg = self.lower_expr(&iterator.node)?;
                self.push_scope();
                let reg = self.alloc_reg();
                self.bind(variable, reg);
                for s in body { self.lower_stmt(&s.node)?; }
                self.pop_scope();
            }
            Statement::Expr(e) => {
                self.lower_expr(&e.node)?;
            }
        }
        Ok(())
    }
    
    fn lower_expr(&mut self, expr: &Expr) -> Result<Register> {
        match expr {
            Expr::Literal(lit) => {
                let val = self.lower_literal(lit)?;
                let out = self.alloc_reg();
                self.emit(Instruction::Constant { out, val });
                Ok(out)
            }
            Expr::Ident(name) => {
                self.lookup(name).ok_or_else(|| HlxError::ValidationFail {
                    message: format!("Undefined variable: {}", name),
                })
            }
            Expr::BinOp { op, lhs, rhs } => {
                let lhs_reg = self.lower_expr(&lhs.node)?;
                let rhs_reg = self.lower_expr(&rhs.node)?;
                let out = self.alloc_reg();
                let inst = match op {
                    BinOp::Add => Instruction::Add { out, lhs: lhs_reg, rhs: rhs_reg },
                    BinOp::Sub => Instruction::Sub { out, lhs: lhs_reg, rhs: rhs_reg },
                    BinOp::Mul => Instruction::Mul { out, lhs: lhs_reg, rhs: rhs_reg },
                    BinOp::Div => Instruction::Div { out, lhs: lhs_reg, rhs: rhs_reg },
                    BinOp::Eq => Instruction::Eq { out, lhs: lhs_reg, rhs: rhs_reg },
                    BinOp::Ne => Instruction::Ne { out, lhs: lhs_reg, rhs: rhs_reg },
                    BinOp::Lt => Instruction::Lt { out, lhs: lhs_reg, rhs: rhs_reg },
                    BinOp::Le => Instruction::Le { out, lhs: lhs_reg, rhs: rhs_reg },
                    BinOp::Gt => Instruction::Gt { out, lhs: lhs_reg, rhs: rhs_reg },
                    BinOp::Ge => Instruction::Ge { out, lhs: lhs_reg, rhs: rhs_reg },
                    BinOp::And => Instruction::And { out, lhs: lhs_reg, rhs: rhs_reg },
                    BinOp::Or => Instruction::Or { out, lhs: lhs_reg, rhs: rhs_reg },
                };
                self.emit(inst);
                Ok(out)
            }
            Expr::UnaryOp { op, operand } => {
                let src = self.lower_expr(&operand.node)?;
                let out = self.alloc_reg();
                let inst = match op {
                    UnaryOp::Neg => Instruction::Neg { out, src },
                    UnaryOp::Not => Instruction::Not { out, src },
                };
                self.emit(inst);
                Ok(out)
            }
            Expr::Call { func, args } => {
                let mut arg_regs = Vec::new();
                for arg in args { arg_regs.push(self.lower_expr(&arg.node)?); }
                let func_name = match &func.node {
                    Expr::Ident(name) => name.clone(),
                    _ => return Err(HlxError::ValidationFail { message: "Call target must be identifier".to_string() }),
                };
                let out = self.alloc_reg();
                let inst = match func_name.as_str() {
                    "matmul" if arg_regs.len() == 2 => Instruction::MatMul { out, lhs: arg_regs[0], rhs: arg_regs[1] },
                    "gelu" if arg_regs.len() == 1 => Instruction::Gelu { out, input: arg_regs[0] },
                    "relu" if arg_regs.len() == 1 => Instruction::Relu { out, input: arg_regs[0] },
                    "softmax" if arg_regs.len() == 1 => Instruction::Softmax { out, input: arg_regs[0], dim: -1 },
                    "print" if arg_regs.len() == 1 => {
                        self.emit(Instruction::Print { val: arg_regs[0] });
                        return Ok(arg_regs[0]);
                    }
                    "type" if arg_regs.len() == 1 => Instruction::TypeOf { out, val: arg_regs[0] },
                    _ => Instruction::Call { out, func: func_name, args: arg_regs },
                };
                self.emit(inst);
                Ok(out)
            }
            Expr::Index { object, index } => {
                let obj_reg = self.lower_expr(&object.node)?;
                let idx_reg = self.lower_expr(&index.node)?;
                let out = self.alloc_reg();
                self.emit(Instruction::Index { out, container: obj_reg, index: idx_reg });
                Ok(out)
            }
            Expr::Field { object, field } => {
                let obj_reg = self.lower_expr(&object.node)?;
                let key_reg = self.alloc_reg();
                self.emit(Instruction::Constant { out: key_reg, val: Value::String(field.clone()) });
                let out = self.alloc_reg();
                self.emit(Instruction::Index { out, container: obj_reg, index: key_reg });
                Ok(out)
            }
            Expr::Pipe { value, func } => {
                let v = self.lower_expr(&value.node)?;
                let f_name = match &func.node {
                    Expr::Ident(n) => n.clone(),
                    _ => return Err(HlxError::ValidationFail { message: "Pipe target must be identifier".to_string() }),
                };
                let out = self.alloc_reg();
                self.emit(Instruction::Call { out, func: f_name, args: vec![v] });
                Ok(out)
            }
            Expr::Collapse { table: _, namespace: _, value } => {
                let v = self.lower_expr(&value.node)?;
                let out = self.alloc_reg();
                self.emit(Instruction::Collapse { handle_out: out, val: v });
                Ok(out)
            }
            Expr::Resolve { target } => {
                let h = self.lower_expr(&target.node)?;
                let out = self.alloc_reg();
                self.emit(Instruction::Resolve { val_out: out, handle: h });
                Ok(out)
            }
            Expr::Snapshot => {
                let out = self.alloc_reg();
                self.emit(Instruction::Snapshot { handle_out: out });
                Ok(out)
            }
            Expr::Transaction { body } => {
                self.push_scope();
                for s in body { self.lower_stmt(&s.node)?; }
                self.pop_scope();
                let out = self.alloc_reg();
                self.emit(Instruction::Constant { out, val: Value::Null });
                Ok(out)
            }
            Expr::Handle(h) => {
                let out = self.alloc_reg();
                self.emit(Instruction::Constant { out, val: Value::Handle(h.clone()) });
                Ok(out)
            }
            Expr::Contract { id, fields } => {
                let mut field_values = Vec::new();
                for (idx, val_expr) in fields {
                    field_values.push((*idx, self.lower_expr(&val_expr.node)?));
                }
                let out = self.alloc_reg();
                let mut contract_fields = Vec::new();
                for (idx, _reg) in &field_values {
                    contract_fields.push((*idx, Value::Null)); // Simplified
                }
                let contract = Contract::new_unchecked(*id, contract_fields);
                self.emit(Instruction::Constant { out, val: Value::Contract(contract) });
                Ok(out)
            }
        }
    }
    
    fn lower_literal(&self, lit: &Literal) -> Result<Value> {
        match lit {
            Literal::Null => Ok(Value::Null),
            Literal::Bool(b) => Ok(Value::Boolean(*b)),
            Literal::Int(i) => Ok(Value::Integer(*i)),
            Literal::Float(f) => Value::float(*f),
            Literal::String(s) => Ok(Value::String(s.clone())),
            Literal::Array(elems) => {
                let mut vals = Vec::new();
                for e in elems {
                    vals.push(match &e.node {
                        Expr::Literal(l) => self.lower_literal(l)?,
                        _ => return Err(HlxError::ValidationFail { message: "Non-constant array element".to_string() }),
                    });
                }
                Ok(Value::Array(vals))
            }
            Literal::Object(fields) => {
                let mut map = std::collections::BTreeMap::new();
                for (k, v_expr) in fields {
                    let v = match &v_expr.node {
                        Expr::Literal(l) => self.lower_literal(l)?,
                        _ => return Err(HlxError::ValidationFail { message: "Non-constant object field".to_string() }),
                    };
                    map.insert(k.clone(), v);
                }
                Ok(Value::Object(map))
            }
        }
    }
}

struct LiftingContext {
    current_stmts: Vec<Spanned<Statement>>,
    reg_names: HashMap<Register, String>,
    name_counter: usize,
}

impl LiftingContext {
    fn new() -> Self {
        Self {
            current_stmts: Vec::new(),
            reg_names: HashMap::new(),
            name_counter: 0,
        }
    }
    
    fn fresh_name(&mut self) -> String {
        let name = format!("_t{}", self.name_counter);
        self.name_counter += 1;
        name
    }
    
    fn get_or_create_name(&mut self, reg: Register) -> String {
        if let Some(name) = self.reg_names.get(&reg) {
            name.clone()
        } else {
            let name = self.fresh_name();
            self.reg_names.insert(reg, name.clone());
            name
        }
    }
    
    fn lift_instructions(&mut self, instructions: &[Instruction]) -> Result<()> {
        for inst in instructions {
            match inst {
                Instruction::Constant { out, val } => {
                    let name = self.get_or_create_name(*out);
                    let expr = self.value_to_expr(val);
                    self.current_stmts.push(Spanned::dummy(Statement::Let { name, value: Spanned::dummy(expr) }));
                }
                Instruction::Add { out, lhs, rhs } => self.emit_binop(*out, *lhs, *rhs, BinOp::Add),
                Instruction::Sub { out, lhs, rhs } => self.emit_binop(*out, *lhs, *rhs, BinOp::Sub),
                Instruction::Mul { out, lhs, rhs } => self.emit_binop(*out, *lhs, *rhs, BinOp::Mul),
                Instruction::Div { out, lhs, rhs } => self.emit_binop(*out, *lhs, *rhs, BinOp::Div),
                Instruction::Return { val } => {
                    let name = self.get_or_create_name(*val);
                    self.current_stmts.push(Spanned::dummy(Statement::Return { value: Spanned::dummy(Expr::Ident(name)) }));
                }
                Instruction::Print { val } => {
                    let name = self.get_or_create_name(*val);
                    self.current_stmts.push(Spanned::dummy(Statement::Expr(Spanned::dummy(Expr::Call {
                        func: Box::new(Spanned::dummy(Expr::Ident("print".to_string()))),
                        args: vec![Spanned::dummy(Expr::Ident(name))],
                    }))));
                }
                _ => {}
            }
        }
        Ok(())
    }
    
    fn emit_binop(&mut self, out: Register, lhs: Register, rhs: Register, op: BinOp) {
        let out_name = self.get_or_create_name(out);
        let lhs_name = self.get_or_create_name(lhs);
        let rhs_name = self.get_or_create_name(rhs);
        self.current_stmts.push(Spanned::dummy(Statement::Let {
            name: out_name,
            value: Spanned::dummy(Expr::BinOp { op, lhs: Box::new(Spanned::dummy(Expr::Ident(lhs_name))), rhs: Box::new(Spanned::dummy(Expr::Ident(rhs_name))) }),
        }));
    }
    
    fn value_to_expr(&self, val: &Value) -> Expr {
        match val {
            Value::Null => Expr::Literal(Literal::Null),
            Value::Boolean(b) => Expr::Literal(Literal::Bool(*b)),
            Value::Integer(i) => Expr::Literal(Literal::Int(*i)),
            Value::Float(f) => Expr::Literal(Literal::Float(*f)),
            Value::String(s) => Expr::Literal(Literal::String(s.clone())),
            Value::Array(arr) => Expr::Literal(Literal::Array(arr.iter().map(|v| Spanned::dummy(self.value_to_expr(v))).collect())),
            Value::Object(obj) => Expr::Literal(Literal::Object(obj.iter().map(|(k, v)| (k.clone(), Spanned::dummy(self.value_to_expr(v)))).collect())),
            Value::Contract(c) => Expr::Contract { id: c.id, fields: c.fields.iter().map(|(idx, v)| (*idx, Spanned::dummy(self.value_to_expr(v)))).collect() },
            Value::Handle(h) => Expr::Handle(h.clone()),
        }
    }
    
    fn build_program(self) -> Program {
        let main_block = Block { name: "main".to_string(), params: vec![], body: self.current_stmts };
        Program { name: "decompiled".to_string(), blocks: vec![main_block] }
    }
}