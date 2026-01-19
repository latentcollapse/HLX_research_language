//! Lowering Pass: AST → Instructions → Crate

use crate::ast::*;
use crate::substrate_inference::SubstrateInference;
use hlx_core::{
    hlx_crate::{HlxCrate, CrateMetadata, HlxScaleInfo},
    instruction::{Instruction, Register},
    value::Value,
    Result, HlxError,
};
use std::collections::HashMap;

const DEFAULT_MAX_DEPTH: u32 = 1000;

/// Extract max_depth from block attributes
fn extract_max_depth(attributes: &[String]) -> u32 {
    for attr in attributes {
        if attr.starts_with("max_depth(") && attr.ends_with(")") {
            let inner = &attr[10..attr.len()-1];  // Extract "N" from "max_depth(N)"
            if let Ok(depth) = inner.parse::<u32>() {
                return depth;
            }
        }
    }
    DEFAULT_MAX_DEPTH
}

/// Lower an AST Program to a Crate
pub fn lower_to_crate(program: &Program) -> Result<HlxCrate> {
    let mut ctx = LoweringContext::new();
    let mut signatures = HashMap::new();
    let mut ffi_exports = HashMap::new();

    // Populate function_depths from block attributes
    for block in &program.blocks {
        ctx.function_depths.insert(block.name.clone(), extract_max_depth(&block.attributes));
    }
    for module in &program.modules {
        for block in &module.blocks {
            ctx.function_depths.insert(block.name.clone(), extract_max_depth(&block.attributes));
        }
    }

    // First pass: collect signatures and FFI attributes for blocks
    for block in &program.blocks {
        let mut param_dtypes = Vec::new();
        for (_, _span, typ_opt) in &block.params {
            if let Some((t, _t_span)) = typ_opt {
                param_dtypes.push(LoweringContext::type_to_dtype(t).unwrap_or(hlx_core::instruction::DType::I64));
            } else {
                param_dtypes.push(hlx_core::instruction::DType::I64);
            }
        }
        signatures.insert(block.name.clone(), param_dtypes.clone());

        // Extract FFI attributes
        let has_no_mangle = block.attributes.iter().any(|attr| attr == "no_mangle");
        let has_export = block.attributes.iter().any(|attr| attr == "export");

        if has_no_mangle || has_export {
            let return_dtype = block.return_type.as_ref()
                .and_then(|t| LoweringContext::type_to_dtype(t))
                .unwrap_or(hlx_core::instruction::DType::I64);

            ffi_exports.insert(block.name.clone(), hlx_core::hlx_crate::FfiExportInfo {
                no_mangle: has_no_mangle,
                export: has_export,
                param_types: param_dtypes.clone(),
                return_type: return_dtype,
            });
        }
    }

    // Also collect signatures and FFI attributes for module blocks
    for module in &program.modules {
        for block in &module.blocks {
            let mut param_dtypes = Vec::new();
            for (_, _span, typ_opt) in &block.params {
                if let Some((t, _t_span)) = typ_opt {
                    param_dtypes.push(LoweringContext::type_to_dtype(t).unwrap_or(hlx_core::instruction::DType::I64));
                } else {
                    param_dtypes.push(hlx_core::instruction::DType::I64);
                }
            }
            signatures.insert(block.name.clone(), param_dtypes.clone());

            // Extract FFI attributes for module blocks
            let has_no_mangle = block.attributes.iter().any(|attr| attr == "no_mangle");
            let has_export = block.attributes.iter().any(|attr| attr == "export");

            if has_no_mangle || has_export {
                let return_dtype = block.return_type.as_ref()
                    .and_then(|t| LoweringContext::type_to_dtype(t))
                    .unwrap_or(hlx_core::instruction::DType::I64);

                ffi_exports.insert(block.name.clone(), hlx_core::hlx_crate::FfiExportInfo {
                    no_mangle: has_no_mangle,
                    export: has_export,
                    param_types: param_dtypes.clone(),
                    return_type: return_dtype,
                });
            }
        }
    }

    // Second pass: lower modules and their contents
    for module in &program.modules {
        let mut module_constants = Vec::new();
        for constant in &module.constants {
            let const_reg = ctx.lower_expr(&constant.value.node)?;
            module_constants.push((constant.name.clone(), LoweringContext::type_to_dtype(&constant.typ).unwrap_or(hlx_core::instruction::DType::I64), const_reg));
        }

        let mut module_structs = Vec::new();
        for struct_def in &module.structs {
            let mut struct_fields = Vec::new();
            for (field_name, field_type) in &struct_def.fields {
                struct_fields.push((field_name.clone(), LoweringContext::type_to_dtype(field_type).unwrap_or(hlx_core::instruction::DType::I64)));
            }
            module_structs.push((struct_def.name.clone(), struct_fields));
        }

        let mut module_blocks = Vec::new();
        for block in &module.blocks {
            let func_start = ctx.instructions.len() as u32;
            let params = ctx.lower_block(block)?;
            module_blocks.push((block.name.clone(), params, func_start));
        }

        ctx.instructions.push(Instruction::ModuleDef {
            name: module.name.clone(),
            capabilities: module.capabilities.clone(),
            constants: module_constants,
            structs: module_structs,
            blocks: module_blocks,
        });
    }

    // Third pass: lower top-level functions
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

    // Run HLX-Scale substrate inference
    let mut inference = SubstrateInference::new();
    let substrate_results = inference.infer_program(program);

    // Validate @scale usage: only main() can have it (MVP restriction)
    let swarm_functions: Vec<_> = substrate_results.iter()
        .filter(|(_, info)| info.agent_count.is_some() && info.agent_count.unwrap() > 1)
        .map(|(name, _)| name.clone())
        .collect();

    if swarm_functions.len() > 1 {
        return Err(HlxError::validation(format!(
            "Multiple @scale functions not supported in MVP: {}. Only main() can use @scale.",
            swarm_functions.join(", ")
        )));
    }

    if swarm_functions.len() == 1 && swarm_functions[0] != "main" {
        return Err(HlxError::validation(format!(
            "@scale on '{}' not supported. Only main() can use @scale in MVP (flat execution model).",
            swarm_functions[0]
        )));
    }

    // Convert substrate info to runtime format (only for main if @scale present)
    let mut hlx_scale_substrates = HashMap::new();
    for (func_name, info) in substrate_results {
        // Only add substrate info if it suggests parallel execution AND it's main()
        if func_name == "main" {
            if let Some(agent_count) = info.agent_count {
                if agent_count > 1 {
                    hlx_scale_substrates.insert(func_name.clone(), HlxScaleInfo {
                        enable_speculation: true,
                        agent_count: agent_count as usize,
                        substrate: info.substrate.to_str().to_string(),
                        barrier_count: info.barrier_count,
                    });
                }
            }
        }
    }

    // Build crate with metadata
    let metadata = CrateMetadata {
        source_file: Some(format!("{}.hlxl", program.name)),
        compiler_version: Some("0.1.0".to_string()),
        register_count: Some(ctx.next_reg),
        function_signatures: signatures,
        debug_symbols,
        hlx_scale_substrates,
        ffi_exports,
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
    /// Track max_depth per function
    function_depths: HashMap<String, u32>,
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
            function_depths: HashMap::new(),
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
        for (param, _span, _typ) in &block.params {
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
            Statement::Let { name, type_annotation, value, .. } => {
                // Set expected type before lowering the value expression
                if let Some(typ) = type_annotation {
                    self.expected_type = Some(typ.clone());
                }

                let val_reg = self.lower_expr(&value.node)?;

                // Clear expected type after lowering
                self.expected_type = None;

                // Allocate a fresh register and copy the value to avoid aliasing
                // Without this, `let x = y` would make x and y aliases to the same register
                let new_reg = self.alloc_reg();
                self.emit(Instruction::Move { out: new_reg, src: val_reg });
                self.bind(name, new_reg);

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
            Statement::Return { value, .. } => {
                let reg = self.lower_expr(&value.node)?;
                self.emit_with_span(Instruction::Return { val: reg }, span);
            }
            Statement::If { condition, then_branch, else_branch, .. } => {
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
            Statement::While { condition, body, max_iter, .. } => {
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
            Statement::Barrier { name, .. } => {
                self.emit_with_span(Instruction::Barrier { name: name.clone() }, span);
            }
            Statement::Asm { template, outputs, inputs, clobbers } => {
                // Build constraints string from outputs, inputs, and clobbers
                let mut constraints_parts = Vec::new();
                
                // Output constraints
                for (constraint, _var) in outputs {
                    constraints_parts.push(format!("={}", constraint));
                }
                
                // Input constraints
                for (constraint, _expr) in inputs {
                    constraints_parts.push(constraint.clone());
                }
                
                // Clobbers
                for clobber in clobbers {
                    constraints_parts.push(format!("~{{{}}}", clobber));
                }
                
                let constraints = constraints_parts.join(",");
                
                // Determine output register (if any)
                let out = if !outputs.is_empty() {
                    Some(self.alloc_reg())
                } else {
                    None
                };
                
                self.emit_with_span(Instruction::Asm {
                    out,
                    template: template.clone(),
                    constraints,
                    side_effects: !clobbers.is_empty() || clobbers.iter().any(|c| c == "memory"),
                }, span);
            }
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
                            BinOp::BitAnd => Instruction::BitAnd { out, lhs: l, rhs: r },
                            BinOp::BitOr => Instruction::BitOr { out, lhs: l, rhs: r },
                            BinOp::BitXor => Instruction::BitXor { out, lhs: l, rhs: r },
                            BinOp::Shl => Instruction::Shl { out, lhs: l, rhs: r },
                            BinOp::Shr => Instruction::Shr { out, lhs: l, rhs: r },
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
                    self.emit(Instruction::Call { out, func: "strlen".to_string(), args: arg_regs, max_depth: DEFAULT_MAX_DEPTH });
                    Ok(out)
                } else if name == "str" || name == "to_string" {
                    if arg_regs.len() != 1 {
                        return Err(HlxError::ValidationFail { message: "str/to_string takes exactly 1 argument".to_string() });
                    }
                    let out = self.alloc_reg();
                    self.emit(Instruction::Call { out, func: "str".to_string(), args: arg_regs, max_depth: DEFAULT_MAX_DEPTH });
                    Ok(out)
                } else if name == "verify_parity" {
                    if arg_regs.len() != 1 {
                        return Err(HlxError::ValidationFail { message: "verify_parity takes exactly 1 argument".to_string() });
                    }
                    let out = self.alloc_reg();
                    self.emit(Instruction::Call { out, func: "verify_parity".to_string(), args: arg_regs, max_depth: DEFAULT_MAX_DEPTH });
                    Ok(out)
                } else if name == "hash_logic" {
                    if arg_regs.len() != 1 {
                        return Err(HlxError::ValidationFail { message: "hash_logic takes exactly 1 argument".to_string() });
                    }
                    let out = self.alloc_reg();
                    self.emit(Instruction::Call { out, func: "hash_logic".to_string(), args: arg_regs, max_depth: DEFAULT_MAX_DEPTH });
                    Ok(out)
                } else if name == "fopen" {
                    if arg_regs.len() != 2 {
                        return Err(HlxError::ValidationFail { message: "fopen takes 2 arguments".to_string() });
                    }
                    let out = self.alloc_reg();
                    self.emit(Instruction::Call { out, func: "fopen".to_string(), args: arg_regs, max_depth: DEFAULT_MAX_DEPTH });
                    Ok(out)
                } else if name == "fclose" {
                    if arg_regs.len() != 1 {
                        return Err(HlxError::ValidationFail { message: "fclose takes 1 argument".to_string() });
                    }
                    let out = self.alloc_reg();
                    self.emit(Instruction::Call { out, func: "fclose".to_string(), args: arg_regs, max_depth: DEFAULT_MAX_DEPTH });
                    Ok(out)
                } else if name == "fwrite" {
                    if arg_regs.len() != 4 {
                        return Err(HlxError::ValidationFail { message: "fwrite takes 4 arguments".to_string() });
                    }
                    let out = self.alloc_reg();
                    self.emit(Instruction::Call { out, func: "fwrite".to_string(), args: arg_regs, max_depth: DEFAULT_MAX_DEPTH });
                    Ok(out)
                } else if name == "fread" {
                    if arg_regs.len() != 4 {
                        return Err(HlxError::ValidationFail { message: "fread takes 4 arguments".to_string() });
                    }
                    let out = self.alloc_reg();
                    self.emit(Instruction::Call { out, func: "fread".to_string(), args: arg_regs, max_depth: DEFAULT_MAX_DEPTH });
                    Ok(out)
                } else if name == "fseek" {
                    if arg_regs.len() != 3 {
                        return Err(HlxError::ValidationFail { message: "fseek takes 3 arguments".to_string() });
                    }
                    let out = self.alloc_reg();
                    self.emit(Instruction::Call { out, func: "fseek".to_string(), args: arg_regs, max_depth: DEFAULT_MAX_DEPTH });
                    Ok(out)
                } else if name == "ftell" {
                    if arg_regs.len() != 1 {
                        return Err(HlxError::ValidationFail { message: "ftell takes 1 argument".to_string() });
                    }
                    let out = self.alloc_reg();
                    self.emit(Instruction::Call { out, func: "ftell".to_string(), args: arg_regs, max_depth: DEFAULT_MAX_DEPTH });
                    Ok(out)
                } else if name == "sdl_init" {
                    let out = self.alloc_reg();
                    self.emit(Instruction::Call { out, func: "SDL_Init".to_string(), args: arg_regs, max_depth: DEFAULT_MAX_DEPTH });
                    Ok(out)
                } else if name == "sdl_create_window" {
                    let out = self.alloc_reg();
                    self.emit(Instruction::Call { out, func: "SDL_CreateWindow".to_string(), args: arg_regs, max_depth: DEFAULT_MAX_DEPTH });
                    Ok(out)
                } else if name == "sdl_create_renderer" {
                    let out = self.alloc_reg();
                    self.emit(Instruction::Call { out, func: "SDL_CreateRenderer".to_string(), args: arg_regs, max_depth: DEFAULT_MAX_DEPTH });
                    Ok(out)
                } else if name == "sdl_set_color" {
                    let out = self.alloc_reg();
                    self.emit(Instruction::Call { out, func: "SDL_SetRenderDrawColor".to_string(), args: arg_regs, max_depth: DEFAULT_MAX_DEPTH });
                    Ok(out)
                } else if name == "sdl_clear" {
                    let out = self.alloc_reg();
                    self.emit(Instruction::Call { out, func: "SDL_RenderClear".to_string(), args: arg_regs, max_depth: DEFAULT_MAX_DEPTH });
                    Ok(out)
                } else if name == "sdl_present" {
                    let out = self.alloc_reg();
                    self.emit(Instruction::Call { out, func: "SDL_RenderPresent".to_string(), args: arg_regs, max_depth: DEFAULT_MAX_DEPTH });
                    Ok(out)
                } else if name == "sdl_poll" {
                    let out = self.alloc_reg();
                    self.emit(Instruction::Call { out, func: "SDL_PollEvent".to_string(), args: arg_regs, max_depth: DEFAULT_MAX_DEPTH });
                    Ok(out)
                } else if name == "sdl_delay" {
                    let out = self.alloc_reg();
                    self.emit(Instruction::Call { out, func: "SDL_Delay".to_string(), args: arg_regs, max_depth: DEFAULT_MAX_DEPTH });
                    Ok(out)
                } else if name == "sdl_quit" {
                    let out = self.alloc_reg();
                    self.emit(Instruction::Call { out, func: "SDL_Quit".to_string(), args: arg_regs, max_depth: DEFAULT_MAX_DEPTH });
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
                    self.emit(Instruction::Call { out, func: "__hlx_tensor_create".to_string(), args: arg_regs, max_depth: DEFAULT_MAX_DEPTH });
                    Ok(out)
                } else if name == "tensor_matmul" {
                    if arg_regs.len() != 2 { return Err(HlxError::ValidationFail { message: "tensor_matmul takes 2 args".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::MatMul { out, lhs: arg_regs[0], rhs: arg_regs[1] });
                    Ok(out)
                } else if name == "tensor_add" {
                    if arg_regs.len() != 2 { return Err(HlxError::ValidationFail { message: "tensor_add takes 2 args".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::Call { out, func: "__hlx_tensor_add".to_string(), args: arg_regs, max_depth: DEFAULT_MAX_DEPTH });
                    Ok(out)
                } else if name == "tensor_transpose" {
                    if arg_regs.len() != 1 { return Err(HlxError::ValidationFail { message: "tensor_transpose takes 1 arg".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::Call { out, func: "__hlx_tensor_transpose".to_string(), args: arg_regs, max_depth: DEFAULT_MAX_DEPTH });
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
                // Math function builtins
                } else if name == "sqrt" {
                    if arg_regs.len() != 1 { return Err(HlxError::ValidationFail { message: "sqrt takes 1 argument".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::Sqrt { out, src: arg_regs[0] });
                    Ok(out)
                } else if name == "pow" {
                    if arg_regs.len() != 2 { return Err(HlxError::ValidationFail { message: "pow takes 2 arguments".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::Pow { out, base: arg_regs[0], exp: arg_regs[1] });
                    Ok(out)
                } else if name == "sin" {
                    if arg_regs.len() != 1 { return Err(HlxError::ValidationFail { message: "sin takes 1 argument".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::Sin { out, src: arg_regs[0] });
                    Ok(out)
                } else if name == "cos" {
                    if arg_regs.len() != 1 { return Err(HlxError::ValidationFail { message: "cos takes 1 argument".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::Cos { out, src: arg_regs[0] });
                    Ok(out)
                } else if name == "tan" {
                    if arg_regs.len() != 1 { return Err(HlxError::ValidationFail { message: "tan takes 1 argument".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::Tan { out, src: arg_regs[0] });
                    Ok(out)
                } else if name == "log" {
                    if arg_regs.len() != 1 { return Err(HlxError::ValidationFail { message: "log takes 1 argument".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::Log { out, src: arg_regs[0] });
                    Ok(out)
                } else if name == "exp" {
                    if arg_regs.len() != 1 { return Err(HlxError::ValidationFail { message: "exp takes 1 argument".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::Exp { out, src: arg_regs[0] });
                    Ok(out)
                } else if name == "floor" {
                    if arg_regs.len() != 1 { return Err(HlxError::ValidationFail { message: "floor takes 1 argument".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::Floor { out, src: arg_regs[0] });
                    Ok(out)
                } else if name == "ceil" {
                    if arg_regs.len() != 1 { return Err(HlxError::ValidationFail { message: "ceil takes 1 argument".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::Ceil { out, src: arg_regs[0] });
                    Ok(out)
                } else if name == "round" {
                    if arg_regs.len() != 1 { return Err(HlxError::ValidationFail { message: "round takes 1 argument".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::Round { out, src: arg_regs[0] });
                    Ok(out)
                } else if name == "abs" {
                    if arg_regs.len() != 1 { return Err(HlxError::ValidationFail { message: "abs takes 1 argument".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::Abs { out, src: arg_regs[0] });
                    Ok(out)
                // String operation builtins
                } else if name == "str_concat" || name == "concat" {
                    if arg_regs.len() != 2 { return Err(HlxError::ValidationFail { message: "concat takes 2 arguments".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::StrConcat { out, lhs: arg_regs[0], rhs: arg_regs[1] });
                    Ok(out)
                } else if name == "str_len" || name == "strlen" {
                    if arg_regs.len() != 1 { return Err(HlxError::ValidationFail { message: "strlen takes 1 argument".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::StrLen { out, src: arg_regs[0] });
                    Ok(out)
                } else if name == "substring" {
                    if arg_regs.len() != 3 { return Err(HlxError::ValidationFail { message: "substring takes 3 arguments".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::Substring { out, src: arg_regs[0], start: arg_regs[1], length: arg_regs[2] });
                    Ok(out)
                } else if name == "index_of" {
                    if arg_regs.len() != 2 { return Err(HlxError::ValidationFail { message: "index_of takes 2 arguments".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::IndexOf { out, haystack: arg_regs[0], needle: arg_regs[1] });
                    Ok(out)
                } else if name == "str_replace" || name == "replace" {
                    if arg_regs.len() != 3 { return Err(HlxError::ValidationFail { message: "replace takes 3 arguments".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::StrReplace { out, src: arg_regs[0], from: arg_regs[1], to: arg_regs[2] });
                    Ok(out)
                } else if name == "split" {
                    if arg_regs.len() != 2 { return Err(HlxError::ValidationFail { message: "split takes 2 arguments".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::StrSplit { out, src: arg_regs[0], delimiter: arg_regs[1] });
                    Ok(out)
                } else if name == "join" {
                    if arg_regs.len() != 2 { return Err(HlxError::ValidationFail { message: "join takes 2 arguments".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::StrJoin { out, array: arg_regs[0], separator: arg_regs[1] });
                    Ok(out)
                } else if name == "to_upper" {
                    if arg_regs.len() != 1 { return Err(HlxError::ValidationFail { message: "to_upper takes 1 argument".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::ToUpper { out, src: arg_regs[0] });
                    Ok(out)
                } else if name == "to_lower" {
                    if arg_regs.len() != 1 { return Err(HlxError::ValidationFail { message: "to_lower takes 1 argument".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::ToLower { out, src: arg_regs[0] });
                    Ok(out)
                } else if name == "trim" {
                    if arg_regs.len() != 1 { return Err(HlxError::ValidationFail { message: "trim takes 1 argument".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::StrTrim { out, src: arg_regs[0] });
                    Ok(out)
                } else if name == "starts_with" {
                    if arg_regs.len() != 2 { return Err(HlxError::ValidationFail { message: "starts_with takes 2 arguments".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::StartsWith { out, src: arg_regs[0], prefix: arg_regs[1] });
                    Ok(out)
                } else if name == "ends_with" {
                    if arg_regs.len() != 2 { return Err(HlxError::ValidationFail { message: "ends_with takes 2 arguments".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::EndsWith { out, src: arg_regs[0], suffix: arg_regs[1] });
                    Ok(out)
                } else if name == "str_repeat" || name == "repeat" {
                    if arg_regs.len() != 2 { return Err(HlxError::ValidationFail { message: "repeat takes 2 arguments".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::StrRepeat { out, src: arg_regs[0], count: arg_regs[1] });
                    Ok(out)
                } else if name == "str_reverse" {
                    if arg_regs.len() != 1 { return Err(HlxError::ValidationFail { message: "str_reverse takes 1 argument".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::StrReverse { out, src: arg_regs[0] });
                    Ok(out)
                } else if name == "char_at" {
                    if arg_regs.len() != 2 { return Err(HlxError::ValidationFail { message: "char_at takes 2 arguments".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::CharAt { out, src: arg_regs[0], index: arg_regs[1] });
                    Ok(out)
                // Array operation builtins
                } else if name == "arr_push" || name == "push" {
                    if arg_regs.len() != 2 { return Err(HlxError::ValidationFail { message: "push takes 2 arguments".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::ArrayPush { out, array: arg_regs[0], element: arg_regs[1] });
                    Ok(out)
                } else if name == "arr_pop" || name == "pop" {
                    if arg_regs.len() != 1 { return Err(HlxError::ValidationFail { message: "pop takes 1 argument".to_string() }); }
                    let array_out = self.alloc_reg();
                    let element_out = self.alloc_reg();
                    self.emit(Instruction::ArrayPop { array_out, element_out, array: arg_regs[0] });
                    // Return array with element as second element
                    let out = self.alloc_reg();
                    self.emit(Instruction::ArrayCreate { out, elements: vec![array_out, element_out], element_type: None });
                    Ok(out)
                } else if name == "arr_shift" || name == "shift" {
                    if arg_regs.len() != 1 { return Err(HlxError::ValidationFail { message: "shift takes 1 argument".to_string() }); }
                    let array_out = self.alloc_reg();
                    let element_out = self.alloc_reg();
                    self.emit(Instruction::ArrayShift { array_out, element_out, array: arg_regs[0] });
                    // Return array with element as second element
                    let out = self.alloc_reg();
                    self.emit(Instruction::ArrayCreate { out, elements: vec![array_out, element_out], element_type: None });
                    Ok(out)
                } else if name == "arr_unshift" || name == "unshift" {
                    if arg_regs.len() != 2 { return Err(HlxError::ValidationFail { message: "unshift takes 2 arguments".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::ArrayUnshift { out, array: arg_regs[0], element: arg_regs[1] });
                    Ok(out)
                } else if name == "arr_slice" || name == "slice" {
                    if arg_regs.len() != 3 { return Err(HlxError::ValidationFail { message: "slice takes 3 arguments".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::ArraySlice { out, array: arg_regs[0], start: arg_regs[1], length: arg_regs[2] });
                    Ok(out)
                } else if name == "arr_concat" {
                    if arg_regs.len() != 2 { return Err(HlxError::ValidationFail { message: "arr_concat takes 2 arguments".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::ArrayConcat { out, lhs: arg_regs[0], rhs: arg_regs[1] });
                    Ok(out)
                } else if name == "arr_reverse" || name == "reverse" {
                    if arg_regs.len() != 1 { return Err(HlxError::ValidationFail { message: "reverse takes 1 argument".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::ArrayReverse { out, array: arg_regs[0] });
                    Ok(out)
                } else if name == "arr_sort" || name == "sort" {
                    if arg_regs.len() != 1 { return Err(HlxError::ValidationFail { message: "sort takes 1 argument".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::ArraySort { out, array: arg_regs[0] });
                    Ok(out)
                } else if name == "arr_find" || name == "find" {
                    if arg_regs.len() != 2 { return Err(HlxError::ValidationFail { message: "find takes 2 arguments".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::ArrayFind { out, array: arg_regs[0], element: arg_regs[1] });
                    Ok(out)

                // Image processing builtins
                } else if name == "gaussian_blur" || name == "blur" {
                    if arg_regs.len() != 2 { return Err(HlxError::ValidationFail { message: "gaussian_blur takes 2 arguments (image, sigma)".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::GaussianBlur { out, input: arg_regs[0], sigma: arg_regs[1] });
                    Ok(out)
                } else if name == "sobel_edges" || name == "edge_detect" {
                    if arg_regs.len() != 2 { return Err(HlxError::ValidationFail { message: "sobel_edges takes 2 arguments (image, threshold)".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::SobelEdges { out, input: arg_regs[0], threshold: arg_regs[1] });
                    Ok(out)
                } else if name == "grayscale" {
                    if arg_regs.len() != 1 { return Err(HlxError::ValidationFail { message: "grayscale takes 1 argument".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::Grayscale { out, input: arg_regs[0] });
                    Ok(out)
                } else if name == "threshold" {
                    if arg_regs.len() != 2 { return Err(HlxError::ValidationFail { message: "threshold takes 2 arguments (image, value)".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::Threshold { out, input: arg_regs[0], value: arg_regs[1] });
                    Ok(out)
                } else if name == "brightness" {
                    if arg_regs.len() != 2 { return Err(HlxError::ValidationFail { message: "brightness takes 2 arguments (image, factor)".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::Brightness { out, input: arg_regs[0], factor: arg_regs[1] });
                    Ok(out)
                } else if name == "contrast" {
                    if arg_regs.len() != 2 { return Err(HlxError::ValidationFail { message: "contrast takes 2 arguments (image, factor)".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::Contrast { out, input: arg_regs[0], factor: arg_regs[1] });
                    Ok(out)
                } else if name == "invert_colors" || name == "invert" {
                    if arg_regs.len() != 1 { return Err(HlxError::ValidationFail { message: "invert_colors takes 1 argument".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::InvertColors { out, input: arg_regs[0] });
                    Ok(out)
                } else if name == "sharpen" {
                    if arg_regs.len() != 1 { return Err(HlxError::ValidationFail { message: "sharpen takes 1 argument".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::Sharpen { out, input: arg_regs[0] });
                    Ok(out)

                // Parsing builtins
                } else if name == "parse_int" || name == "to_int" {
                    if arg_regs.len() != 1 { return Err(HlxError::ValidationFail { message: "parse_int takes 1 argument".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::ParseInt { out, input: arg_regs[0] });
                    Ok(out)
                } else if name == "parse_float" || name == "to_float" {
                    if arg_regs.len() != 1 { return Err(HlxError::ValidationFail { message: "parse_float takes 1 argument".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::ParseFloat { out, input: arg_regs[0] });
                    Ok(out)
                } else if name == "json_serialize" || name == "to_json" {
                    if arg_regs.len() != 1 { return Err(HlxError::ValidationFail { message: "json_serialize takes 1 argument".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::JsonSerialize { out, input: arg_regs[0] });
                    Ok(out)
                } else if name == "csv_parse" || name == "parse_csv" {
                    if arg_regs.len() != 2 { return Err(HlxError::ValidationFail { message: "csv_parse takes 2 arguments (string, delimiter)".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::CsvParse { out, input: arg_regs[0], delimiter: arg_regs[1] });
                    Ok(out)
                } else if name == "format" || name == "format_string" {
                    if arg_regs.is_empty() { return Err(HlxError::ValidationFail { message: "format takes at least 1 argument".to_string() }); }
                    let out = self.alloc_reg();
                    let format = arg_regs[0];
                    let args = arg_regs[1..].to_vec();
                    self.emit(Instruction::FormatString { out, format, args });
                    Ok(out)
                } else if name == "regex_match" {
                    if arg_regs.len() != 2 { return Err(HlxError::ValidationFail { message: "regex_match takes 2 arguments (string, pattern)".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::RegexMatch { out, input: arg_regs[0], pattern: arg_regs[1] });
                    Ok(out)
                } else if name == "regex_replace" {
                    if arg_regs.len() != 3 { return Err(HlxError::ValidationFail { message: "regex_replace takes 3 arguments (string, pattern, replacement)".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::RegexReplace { out, input: arg_regs[0], pattern: arg_regs[1], replacement: arg_regs[2] });
                    Ok(out)

                // File I/O builtins
                } else if name == "read_line" || name == "readline" {
                    if !arg_regs.is_empty() { return Err(HlxError::ValidationFail { message: "read_line takes no arguments".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::ReadLine { out });
                    Ok(out)
                } else if name == "append_file" {
                    if arg_regs.len() != 2 { return Err(HlxError::ValidationFail { message: "append_file takes 2 arguments (path, content)".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::AppendFile { out, path: arg_regs[0], content: arg_regs[1] });
                    Ok(out)
                } else if name == "file_exists" || name == "exists" {
                    if arg_regs.len() != 1 { return Err(HlxError::ValidationFail { message: "file_exists takes 1 argument".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::FileExists { out, path: arg_regs[0] });
                    Ok(out)
                } else if name == "delete_file" || name == "remove_file" {
                    if arg_regs.len() != 1 { return Err(HlxError::ValidationFail { message: "delete_file takes 1 argument".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::DeleteFile { out, path: arg_regs[0] });
                    Ok(out)
                } else if name == "list_files" || name == "list_dir" {
                    if arg_regs.len() != 1 { return Err(HlxError::ValidationFail { message: "list_files takes 1 argument".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::ListFiles { out, path: arg_regs[0] });
                    Ok(out)
                } else if name == "create_dir" || name == "mkdir" {
                    if arg_regs.len() != 1 { return Err(HlxError::ValidationFail { message: "create_dir takes 1 argument".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::CreateDir { out, path: arg_regs[0] });
                    Ok(out)
                } else if name == "delete_dir" || name == "remove_dir" || name == "rmdir" {
                    if arg_regs.len() != 1 { return Err(HlxError::ValidationFail { message: "delete_dir takes 1 argument".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::DeleteDir { out, path: arg_regs[0] });
                    Ok(out)
                } else if name == "read_json" {
                    if arg_regs.len() != 1 { return Err(HlxError::ValidationFail { message: "read_json takes 1 argument".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::ReadJson { out, path: arg_regs[0] });
                    Ok(out)
                } else if name == "write_json" {
                    if arg_regs.len() != 2 { return Err(HlxError::ValidationFail { message: "write_json takes 2 arguments (path, value)".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::WriteJson { out, path: arg_regs[0], value: arg_regs[1] });
                    Ok(out)
                } else if name == "read_csv" {
                    if arg_regs.len() != 2 { return Err(HlxError::ValidationFail { message: "read_csv takes 2 arguments (path, delimiter)".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::ReadCsv { out, path: arg_regs[0], delimiter: arg_regs[1] });
                    Ok(out)
                } else if name == "write_csv" {
                    if arg_regs.len() != 3 { return Err(HlxError::ValidationFail { message: "write_csv takes 3 arguments (path, data, delimiter)".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::WriteCsv { out, path: arg_regs[0], data: arg_regs[1], delimiter: arg_regs[2] });
                    Ok(out)

                // Image I/O builtins
                } else if name == "load_image" {
                    if arg_regs.len() != 1 { return Err(HlxError::ValidationFail { message: "load_image takes 1 argument (path)".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::LoadImage { out, path: arg_regs[0] });
                    Ok(out)
                } else if name == "save_image" {
                    if arg_regs.len() != 2 { return Err(HlxError::ValidationFail { message: "save_image takes 2 arguments (tensor, path)".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::SaveImage { out, tensor: arg_regs[0], path: arg_regs[1] });
                    Ok(out)

                // Tensor creation builtin
                } else if name == "tensor" {
                    if arg_regs.len() != 2 { return Err(HlxError::ValidationFail { message: "tensor takes 2 arguments (data, shape)".to_string() }); }
                    let out = self.alloc_reg();
                    self.emit(Instruction::TensorFromData { out, data: arg_regs[0], shape: arg_regs[1] });
                    Ok(out)
                                } else {
                                    let out = self.alloc_reg();
                                    let max_depth = self.function_depths
                                        .get(&name)
                                        .copied()
                                        .unwrap_or(DEFAULT_MAX_DEPTH);
                                    self.emit(Instruction::Call { out, func: name, args: arg_regs, max_depth });
                                    Ok(out)
                                }
                            }
                            _ => Err(HlxError::ValidationFail { message: format!("Unsupported expression type: {:?}", expr) }),
                        }
                    }
                }
                