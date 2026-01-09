//! Lowering Pass: AST → Instructions → Crate

use crate::ast::*;
use hlx_core::{
    hlx_crate::{HlxCrate, CrateMetadata},
    instruction::{Instruction, Register},
    value::Value,
    Result, HlxError,
};
use std::collections::HashMap;

/// Lower an AST Program to a Crate
pub fn lower_to_crate(program: &Program) -> Result<HlxCrate> {
    let mut ctx = LoweringContext::new();
    let mut signatures = HashMap::new();
    
    // First pass: collect signatures
    for block in &program.blocks {
        let mut param_dtypes = Vec::new();
        for (_, typ) in &block.params {
            if let Some(t) = typ {
                param_dtypes.push(LoweringContext::type_to_dtype(t).unwrap_or(hlx_core::instruction::DType::I64));
            } else {
                param_dtypes.push(hlx_core::instruction::DType::I64);
            }
        }
        signatures.insert(block.name.clone(), param_dtypes);
    }

    // Second pass: lower functions
    for block in &program.blocks {
        let func_start = ctx.instructions.len() as u32;
        let params = ctx.lower_block(block)?;
        
        ctx.instructions.push(Instruction::FuncDef {
            name: block.name.clone(),
            params,
            body: func_start,
        });
    }
    
    // Convert source map to debug symbols
    let debug_symbols = ctx.source_map.iter().enumerate()
        .filter_map(|(idx, span_opt)| {
            span_opt.map(|span| hlx_core::hlx_crate::DebugSymbol {
                inst_idx: idx,
                line: span.line,
                col: span.col,
            })
        })
        .collect();

    // Build crate with metadata
    let metadata = CrateMetadata {
        source_file: Some(format!("{}.hlxl", program.name)),
        compiler_version: Some("0.1.0".to_string()),
        register_count: Some(ctx.next_reg),
        function_signatures: signatures,
        debug_symbols,
        ..Default::default()
    };

    Ok(HlxCrate::with_metadata(ctx.instructions, metadata))
}

struct LoweringContext {
    instructions: Vec<Instruction>,
    next_reg: Register,
    scopes: Vec<HashMap<String, Register>>,
    /// Track type annotations for variables
    type_annotations: HashMap<String, Type>,
    /// Expected type for the current expression being lowered (for type propagation)
    expected_type: Option<Type>,
    /// Source location map: parallel vector to instructions
    source_map: Vec<Option<Span>>,
}

impl LoweringContext {
    fn new() -> Self {
        Self {
            instructions: Vec::new(),
            next_reg: 0,
            scopes: vec![HashMap::new()],
            type_annotations: HashMap::new(),
            expected_type: None,
            source_map: Vec::new(),
        }
    }
    
    fn alloc_reg(&mut self) -> Register {
        let reg = self.next_reg;
        self.next_reg += 1;
        reg
    }
    
    fn push_scope(&mut self) { self.scopes.push(HashMap::new()); }
    fn pop_scope(&mut self) { self.scopes.pop(); }
    
    fn bind(&mut self, name: &str, reg: Register) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), reg);
        }
    }
    
    fn lookup(&self, name: &str) -> Option<Register> {
        for scope in self.scopes.iter().rev() {
            if let Some(&reg) = scope.get(name) { return Some(reg); }
        }
        None
    }
    
    fn emit(&mut self, inst: Instruction) {
        self.instructions.push(inst);
        self.source_map.push(None);
    }

    fn emit_with_span(&mut self, inst: Instruction, span: Span) {
        self.instructions.push(inst);
        self.source_map.push(Some(span));
    }

    /// Convert AST Type to IR DType for array elements
    fn type_to_dtype(typ: &Type) -> Option<hlx_core::instruction::DType> {
        use hlx_core::instruction::DType;
        match typ {
            Type::Int => Some(DType::I64),
            Type::Float => Some(DType::F64),
            Type::Bool => Some(DType::Bool),
            Type::Array(inner) => {
                // Recursively encode nested arrays!
                Some(DType::Array(Box::new(Self::type_to_dtype(inner)?)))
            }
            _ => None, // String and other types not supported as array elements yet
        }
    }

    fn lower_block(&mut self, block: &Block) -> Result<Vec<Register>> {
        self.push_scope();
        let mut params = Vec::new();
        for (param, _typ) in &block.params {
            let reg = self.alloc_reg();
            self.bind(param, reg);
            params.push(reg);
        }
        
        for item in &block.items {
            match &item.node {
                Item::Statement(stmt) => self.lower_stmt(stmt, item.span)?,
                Item::Node(_) => return Err(HlxError::ValidationFail { message: "Nodes not supported in V2 blocks".to_string() }),
            }
        }
        
        // Ensure every function returns SOMETHING
        let null_reg = self.alloc_reg();
        self.emit(Instruction::Constant { out: null_reg, val: Value::Null });
        self.emit(Instruction::Return { val: null_reg });

        self.pop_scope();
        Ok(params)
    }
    
    fn lower_stmt(&mut self, stmt: &Statement, span: Span) -> Result<()> {
        match stmt {
            Statement::Let { name, type_annotation, value } => {
                // Set expected type before lowering the value expression
                if let Some(typ) = type_annotation {
                    self.expected_type = Some(typ.clone());
                }

                let val_reg = self.lower_expr(&value.node)?;

                // Clear expected type after lowering
                self.expected_type = None;

                self.bind(name, val_reg);
                // Store type annotation if present
                if let Some(typ) = type_annotation {
                    self.type_annotations.insert(name.clone(), typ.clone());
                }
            }
            Statement::Assign { lhs, value } => {
                let val_reg = self.lower_expr(&value.node)?;
                match &lhs.node {
                    Expr::Ident(name) => {
                        if let Some(reg) = self.lookup(name) {
                            self.emit_with_span(Instruction::Move { out: reg, src: val_reg }, span);
                        } else {
                            return Err(HlxError::ValidationFail { message: format!("Undefined: {}", name) });
                        }
                    }
                    Expr::Index { object, index } => {
                        let obj_reg = self.lower_expr(&object.node)?;
                        let idx_reg = self.lower_expr(&index.node)?;
                        self.emit_with_span(Instruction::Store { container: obj_reg, index: idx_reg, value: val_reg }, span);
                    }
                    Expr::Field { object, field } => {
                        let obj_reg = self.lower_expr(&object.node)?;
                        let key_reg = self.alloc_reg();
                        self.emit(Instruction::Constant { out: key_reg, val: Value::String(field.clone()) });
                        self.emit_with_span(Instruction::Store { container: obj_reg, index: key_reg, value: val_reg }, span);
                    }
                    _ => return Err(HlxError::ValidationFail { message: "Invalid assignment target".to_string() }),
                }
            }
            Statement::Return { value } => {
                let reg = self.lower_expr(&value.node)?;
                self.emit_with_span(Instruction::Return { val: reg }, span);
            }
            Statement::If { condition, then_branch, else_branch } => {
                let cond_reg = self.lower_expr(&condition.node)?;
                let if_idx = self.instructions.len();
                self.emit_with_span(Instruction::If { cond: cond_reg, then_block: 0, else_block: 0 }, span);

                let then_start = self.instructions.len() as u32;
                for s in then_branch { self.lower_stmt(&s.node, s.span)?; }

                let jump_idx = self.instructions.len();
                self.emit(Instruction::Jump { target: 0 });

                let else_start = self.instructions.len() as u32;
                if let Some(eb) = else_branch {
                    for s in eb { self.lower_stmt(&s.node, s.span)?; }
                }

                let end_idx = self.instructions.len() as u32;

                if let Instruction::If { ref mut then_block, ref mut else_block, .. } = self.instructions[if_idx] {
                    *then_block = then_start;
                    *else_block = else_start;
                }
                if let Instruction::Jump { ref mut target } = self.instructions[jump_idx] {
                    *target = end_idx;
                }
            }
            Statement::While { condition, body, max_iter } => {
                let loop_pc = self.instructions.len() as u32;
                let cond_reg = self.lower_expr(&condition.node)?;

                // Emit Loop instruction with placeholders
                let loop_idx = self.instructions.len();
                self.emit_with_span(Instruction::Loop {
                    cond: cond_reg,
                    body: 0,
                    exit: 0,
                    max_iter: *max_iter
                }, span);

                let body_start = self.instructions.len() as u32;
                for s in body { self.lower_stmt(&s.node, s.span)?; }

                // Jump back to Loop instruction
                self.emit(Instruction::Jump { target: loop_pc });

                let loop_exit = self.instructions.len() as u32;

                // Fill in placeholders
                if let Instruction::Loop { ref mut body, ref mut exit, .. } = self.instructions[loop_idx] {
                    *body = body_start;
                    *exit = loop_exit;
                }
            }
            Statement::Expr(e) => { self.lower_expr(&e.node)?; }
            Statement::Break => { self.emit_with_span(Instruction::Break, span); }
            Statement::Continue => { self.emit_with_span(Instruction::Continue, span); }
            _ => {}
        }
        Ok(())
    }
    
    fn lower_expr(&mut self, expr: &Expr) -> Result<Register> {
        match expr {
            Expr::Literal(lit) => {
                let val = match lit {
                    Literal::Null => Value::Null,
                    Literal::Bool(b) => Value::Boolean(*b),
                    Literal::Int(i) => Value::Integer(*i),
                    Literal::Float(f) => Value::float(*f)?,
                    Literal::String(s) => Value::String(s.clone()),
                    _ => return Err(HlxError::ValidationFail { message: "Unsupported literal".to_string() }),
                };
                let out = self.alloc_reg();
                self.emit(Instruction::Constant { out, val });
                Ok(out)
            }
            Expr::Ident(name) => {
                self.lookup(name).ok_or_else(|| HlxError::ValidationFail { message: format!("Undefined: {}", name) })
            }
            Expr::BinOp { op, lhs, rhs } => {
                // Short-circuit evaluation for AND/OR
                match op {
                    BinOp::And => {
                        // AND: if lhs is false, result is false (don't evaluate rhs)
                        let lhs_reg = self.lower_expr(&lhs.node)?;
                        let out = self.alloc_reg();

                        // If lhs is false, jump to false path
                        let if_idx = self.instructions.len();
                        self.emit(Instruction::If { cond: lhs_reg, then_block: 0, else_block: 0 });

                        // False path: set result to false
                        let false_path = self.instructions.len() as u32;
                        self.emit(Instruction::Constant { out, val: Value::Boolean(false) });
                        let jump_false_idx = self.instructions.len();
                        self.emit(Instruction::Jump { target: 0 });

                        // True path: evaluate rhs
                        let true_path = self.instructions.len() as u32;
                        let rhs_reg = self.lower_expr(&rhs.node)?;
                        self.emit(Instruction::Move { out, src: rhs_reg });

                        let end_idx = self.instructions.len() as u32;

                        // Backfill
                        if let Instruction::If { ref mut then_block, ref mut else_block, .. } = self.instructions[if_idx] {
                            *then_block = true_path;
                            *else_block = false_path;
                        }
                        if let Instruction::Jump { ref mut target } = self.instructions[jump_false_idx] {
                            *target = end_idx;
                        }

                        Ok(out)
                    }
                    BinOp::Or => {
                        // OR: if lhs is true, result is true (don't evaluate rhs)
                        let lhs_reg = self.lower_expr(&lhs.node)?;
                        let out = self.alloc_reg();

                        // If lhs is true, jump to true path
                        let if_idx = self.instructions.len();
                        self.emit(Instruction::If { cond: lhs_reg, then_block: 0, else_block: 0 });

                        // True path: set result to true
                        let true_path = self.instructions.len() as u32;
                        self.emit(Instruction::Constant { out, val: Value::Boolean(true) });
                        let jump_true_idx = self.instructions.len();
                        self.emit(Instruction::Jump { target: 0 });

                        // False path: evaluate rhs
                        let false_path = self.instructions.len() as u32;
                        let rhs_reg = self.lower_expr(&rhs.node)?;
                        self.emit(Instruction::Move { out, src: rhs_reg });

                        let end_idx = self.instructions.len() as u32;

                        // Backfill
                        if let Instruction::If { ref mut then_block, ref mut else_block, .. } = self.instructions[if_idx] {
                            *then_block = true_path;
                            *else_block = false_path;
                        }
                        if let Instruction::Jump { ref mut target } = self.instructions[jump_true_idx] {
                            *target = end_idx;
                        }

                        Ok(out)
                    }
                    _ => {
                        // Non-short-circuit operators: evaluate both operands
                        let l = self.lower_expr(&lhs.node)?;
                        let r = self.lower_expr(&rhs.node)?;
                        let out = self.alloc_reg();
                        let inst = match op {
                            BinOp::Add => Instruction::Add { out, lhs: l, rhs: r },
                            BinOp::Sub => Instruction::Sub { out, lhs: l, rhs: r },
                            BinOp::Mul => Instruction::Mul { out, lhs: l, rhs: r },
                            BinOp::Div => Instruction::Div { out, lhs: l, rhs: r },
                            BinOp::Mod => Instruction::Mod { out, lhs: l, rhs: r },
                            BinOp::Lt => Instruction::Lt { out, lhs: l, rhs: r },
                            BinOp::Gt => Instruction::Gt { out, lhs: l, rhs: r },
                            BinOp::Le => Instruction::Le { out, lhs: l, rhs: r },
                            BinOp::Ge => Instruction::Ge { out, lhs: l, rhs: r },
                            BinOp::Eq => Instruction::Eq { out, lhs: l, rhs: r },
                            BinOp::Ne => Instruction::Ne { out, lhs: l, rhs: r },
                            _ => unreachable!("AND/OR handled above"),
                        };
                        self.emit(inst);
                        Ok(out)
                    }
                }
            }
            Expr::UnaryOp { op, operand } => {
                let src = self.lower_expr(&operand.node)?;
                let out = self.alloc_reg();
                if matches!(op, UnaryOp::Neg) {
                    self.emit(Instruction::Neg { out, src });
                    Ok(out)
                } else if matches!(op, UnaryOp::Not) {
                    self.emit(Instruction::Not { out, src });
                    Ok(out)
                } else {
                    Err(HlxError::ValidationFail { message: "Unsupported unary op".to_string() })
                }
            }
            Expr::Array(elements) => {
                let mut regs = Vec::new();
                for e in elements {
                    regs.push(self.lower_expr(&e.node)?);
                }
                let out = self.alloc_reg();

                // Check if we have type information from the expected type
                let element_type = if let Some(Type::Array(inner)) = &self.expected_type {
                    Self::type_to_dtype(inner)
                } else {
                    None
                };

                self.emit(Instruction::ArrayCreate { out, elements: regs, element_type });
                Ok(out)
            }
            Expr::Object(entries) => {
                let mut keys = Vec::new();
                let mut val_regs = Vec::new();
                for (k, v) in entries {
                    keys.push(k.clone());
                    val_regs.push(self.lower_expr(&v.node)?);
                }
                let out = self.alloc_reg();
                self.emit(Instruction::ObjectCreate { out, keys, values: val_regs });
                Ok(out)
            }
            Expr::Index { object, index } => {
                let obj = self.lower_expr(&object.node)?;
                let idx = self.lower_expr(&index.node)?;
                let out = self.alloc_reg();
                self.emit(Instruction::Index { out, container: obj, index: idx });
                Ok(out)
            }
            Expr::Field { object, field } => {
                let obj = self.lower_expr(&object.node)?;
                // Create constant string for field name
                let key_reg = self.alloc_reg();
                self.emit(Instruction::Constant { out: key_reg, val: Value::String(field.clone()) });
                
                let out = self.alloc_reg();
                self.emit(Instruction::Index { out, container: obj, index: key_reg });
                Ok(out)
            }
            Expr::Call { func, args } => {
                let mut arg_regs = Vec::new();
                for a in args { arg_regs.push(self.lower_expr(&a.node)?); }
                let name = match &func.node {
                    Expr::Ident(n) => n.clone(),
                    _ => return Err(HlxError::ValidationFail { message: "Call target must be identifier".to_string() }),
                };
                
                if name == "print" {
                    for arg in arg_regs {
                        self.emit(Instruction::Print { val: arg });
                    }
                    // Print doesn't return a value, but expressions must. Return 0/null.
                    let out = self.alloc_reg();
                    self.emit(Instruction::Constant { out, val: Value::Null });
                    Ok(out)
                } else if name == "print_str" {
                    if arg_regs.len() != 1 {
                        return Err(HlxError::ValidationFail { message: "print_str takes exactly 1 argument".to_string() });
                    }
                    self.emit(Instruction::PrintStr { val: arg_regs[0] });
                    let out = self.alloc_reg();
                    self.emit(Instruction::Constant { out, val: Value::Null });
                    Ok(out)
                } else if name == "strlen" {
                    if arg_regs.len() != 1 {
                        return Err(HlxError::ValidationFail { message: "strlen takes exactly 1 argument".to_string() });
                    }
                    let out = self.alloc_reg();
                    self.emit(Instruction::Call { out, func: "strlen".to_string(), args: arg_regs });
                    Ok(out)
                } else if name == "fopen" {
                    if arg_regs.len() != 2 {
                        return Err(HlxError::ValidationFail { message: "fopen takes 2 arguments".to_string() });
                    }
                    let out = self.alloc_reg();
                    self.emit(Instruction::Call { out, func: "fopen".to_string(), args: arg_regs });
                    Ok(out)
                } else if name == "fclose" {
                    if arg_regs.len() != 1 {
                        return Err(HlxError::ValidationFail { message: "fclose takes 1 argument".to_string() });
                    }
                    let out = self.alloc_reg();
                    self.emit(Instruction::Call { out, func: "fclose".to_string(), args: arg_regs });
                    Ok(out)
                } else if name == "fwrite" {
                    if arg_regs.len() != 4 {
                        return Err(HlxError::ValidationFail { message: "fwrite takes 4 arguments".to_string() });
                    }
                    let out = self.alloc_reg();
                    self.emit(Instruction::Call { out, func: "fwrite".to_string(), args: arg_regs });
                    Ok(out)
                } else if name == "fread" {
                    if arg_regs.len() != 4 {
                        return Err(HlxError::ValidationFail { message: "fread takes 4 arguments".to_string() });
                    }
                    let out = self.alloc_reg();
                    self.emit(Instruction::Call { out, func: "fread".to_string(), args: arg_regs });
                    Ok(out)
                } else if name == "fseek" {
                    if arg_regs.len() != 3 {
                        return Err(HlxError::ValidationFail { message: "fseek takes 3 arguments".to_string() });
                    }
                    let out = self.alloc_reg();
                    self.emit(Instruction::Call { out, func: "fseek".to_string(), args: arg_regs });
                    Ok(out)
                } else if name == "ftell" {
                    if arg_regs.len() != 1 {
                        return Err(HlxError::ValidationFail { message: "ftell takes 1 argument".to_string() });
                    }
                    let out = self.alloc_reg();
                    self.emit(Instruction::Call { out, func: "ftell".to_string(), args: arg_regs });
                    Ok(out)
                } else if name == "sdl_init" {
                    let out = self.alloc_reg();
                    self.emit(Instruction::Call { out, func: "SDL_Init".to_string(), args: arg_regs });
                    Ok(out)
                } else if name == "sdl_create_window" {
                    let out = self.alloc_reg();
                    self.emit(Instruction::Call { out, func: "SDL_CreateWindow".to_string(), args: arg_regs });
                    Ok(out)
                } else if name == "sdl_create_renderer" {
                    let out = self.alloc_reg();
                    self.emit(Instruction::Call { out, func: "SDL_CreateRenderer".to_string(), args: arg_regs });
                    Ok(out)
                } else if name == "sdl_set_color" {
                    let out = self.alloc_reg();
                    self.emit(Instruction::Call { out, func: "SDL_SetRenderDrawColor".to_string(), args: arg_regs });
                    Ok(out)
                } else if name == "sdl_clear" {
                    let out = self.alloc_reg();
                    self.emit(Instruction::Call { out, func: "SDL_RenderClear".to_string(), args: arg_regs });
                    Ok(out)
                } else if name == "sdl_present" {
                    let out = self.alloc_reg();
                    self.emit(Instruction::Call { out, func: "SDL_RenderPresent".to_string(), args: arg_regs });
                    Ok(out)
                } else if name == "sdl_poll" {
                    let out = self.alloc_reg();
                    self.emit(Instruction::Call { out, func: "SDL_PollEvent".to_string(), args: arg_regs });
                    Ok(out)
                } else if name == "sdl_delay" {
                    let out = self.alloc_reg();
                    self.emit(Instruction::Call { out, func: "SDL_Delay".to_string(), args: arg_regs });
                    Ok(out)
                } else if name == "sdl_quit" {
                    let out = self.alloc_reg();
                    self.emit(Instruction::Call { out, func: "SDL_Quit".to_string(), args: arg_regs });
                    Ok(out)
                } else if name == "tensor_new_2d" {
                    // Expect 2 args: rows, cols (must be literals for now for TensorCreate)
                    // Wait, lower_expr receives Exprs, but here I already lowered them to Registers (arg_regs).
                    // Instruction::TensorCreate takes Shape (Vec<usize>), which is static.
                    // If args are variables, I can't use TensorCreate with static shape.
                    // BUT, my __hlx_tensor_create accepts dynamic args!
                    // So I should use a CALL to __hlx_tensor_create, not the Instruction::TensorCreate if I want dynamic.
                    // However, Instruction::TensorCreate maps to __hlx_tensor_create in backend.
                    // But Instruction::TensorCreate requires static shape in the struct.
                    
                    // Let's modify Instruction::TensorCreate to take registers for shape?
                    // No, that changes hlx_core deeply.
                    
                    // Easy fix: Expose __hlx_tensor_create as a function call directly.
                    // But I made it Internal linkage.
                    
                    // Actually, let's just make `tensor_new_2d` use `Instruction::TensorCreate` and enforce literals for now.
                    // Getting values from registers at compile time is impossible if they are variables.
                    
                    // Alternative: Map `tensor_new_2d` to a CALL to `__hlx_tensor_create`.
                    // To do this, I need to add `__hlx_tensor_create` to the function map in LoweringContext?
                    // No, `Instruction::Call` just takes a string.
                    
                    // I'll assume __hlx_tensor_create is available to call.
                    // But in backend, I made it Internal linkage. I should change it to External or just allow internal calls.
                    // The JIT can call internal functions if they are in the module.
                    
                    if arg_regs.len() != 2 { return Err(HlxError::ValidationFail { message: "tensor_new_2d takes 2 args".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::Call { out, func: "__hlx_tensor_create".to_string(), args: arg_regs });
                    Ok(out)
                } else if name == "tensor_matmul" {
                    if arg_regs.len() != 2 { return Err(HlxError::ValidationFail { message: "tensor_matmul takes 2 args".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::MatMul { out, lhs: arg_regs[0], rhs: arg_regs[1] });
                    Ok(out)
                } else if name == "tensor_add" {
                    if arg_regs.len() != 2 { return Err(HlxError::ValidationFail { message: "tensor_add takes 2 args".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::Call { out, func: "__hlx_tensor_add".to_string(), args: arg_regs });
                    Ok(out)
                } else if name == "tensor_transpose" {
                    if arg_regs.len() != 1 { return Err(HlxError::ValidationFail { message: "tensor_transpose takes 1 arg".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::Call { out, func: "__hlx_tensor_transpose".to_string(), args: arg_regs });
                    Ok(out)
                } else if name == "alloc_array" {
                    if arg_regs.len() != 1 {
                        return Err(HlxError::ValidationFail { message: "alloc_array takes exactly 1 argument".to_string() });
                    }
                    let out = self.alloc_reg();

                    // Check if we have type information from the expected type
                    let element_type = if let Some(Type::Array(inner)) = &self.expected_type {
                        Self::type_to_dtype(inner)
                    } else {
                        None
                    };

                    self.emit(Instruction::ArrayAlloc { out, size: arg_regs[0], element_type });
                    Ok(out)
                                } else {
                                    let out = self.alloc_reg();
                                    self.emit(Instruction::Call { out, func: name, args: arg_regs });
                                    Ok(out)
                                }
                            }
                            _ => Err(HlxError::ValidationFail { message: format!("Unsupported expression type: {:?}", expr) }),
                        }
                    }
                }
                