//! AST → Bytecode Lowerer
//!
//! Consumes a `Program` AST (from `ast_parser`) and emits `Bytecode` for the VM.
//! This is the bridge between the rich, introspectable AST and executable bytecode.

use crate::ast::{
    AgentDef, BinaryOp, CycleLevel, ExprKind, Expression, Function, Gate, Import, Item, ModuleDef,
    Parameter, Pattern, Program, Statement, StmtKind, UnaryOp,
};
use crate::ast_parser::AstParser;
use crate::resolver::{ImportStyle, ModuleResolver};
use crate::{Bytecode, Opcode, Value};
use std::collections::HashMap;
use std::path::Path;

/// Errors that can occur during lowering
#[derive(Debug, Clone)]
pub struct LowerError {
    pub message: String,
}

impl std::fmt::Display for LowerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Lower error: {}", self.message)
    }
}

impl LowerError {
    fn new(msg: impl Into<String>) -> Self {
        LowerError {
            message: msg.into(),
        }
    }
}

type LowerResult<T> = Result<T, LowerError>;

/// Function to be loaded after main program
#[derive(Debug, Clone)]
struct PendingImport {
    name: String,
    func: Function,
}

/// Lowers a `Program` AST into `Bytecode` + function table.
pub struct Lowerer {
    bytecode: Bytecode,
    strings: HashMap<String, u32>,
    functions: HashMap<String, (u32, u32)>, // name -> (start_pc, param_count)
    variables: HashMap<String, u8>,
    next_var_reg: u8,
    next_tmp_reg: u8,
    /// Forward references to functions not yet compiled
    patch_points: Vec<(usize, String)>,
    /// Loop context for break/continue
    loop_stack: Vec<LoopContext>,
    /// Imports to be loaded after main program
    pending_imports: Vec<PendingImport>,
}

#[derive(Debug, Clone)]
struct LoopContext {
    start_pc: usize,
    /// Positions to patch with the loop exit PC
    break_patches: Vec<usize>,
}

impl Lowerer {
    pub fn new() -> Self {
        Lowerer {
            bytecode: Bytecode::new(),
            strings: HashMap::new(),
            functions: HashMap::new(),
            variables: HashMap::new(),
            next_var_reg: 0,
            next_tmp_reg: 200,
            patch_points: Vec::new(),
            loop_stack: Vec::new(),
            pending_imports: Vec::new(),
        }
    }

    /// Lower with pre-resolved imports (from ModuleResolver)
    /// Main program is lowered first so it starts at PC 0
    pub fn lower_with_imports(
        program: &Program,
        resolver_imports: HashMap<String, Function>,
    ) -> LowerResult<(Bytecode, HashMap<String, (u32, u32)>)> {
        let mut lowerer = Lowerer::new();

        // First, lower the main program so it starts at PC 0
        lowerer.lower_program(program)?;

        // Process any dynamic imports collected during program lowering
        // (e.g., from Item::Import nodes that use lower_import())
        while !lowerer.pending_imports.is_empty() {
            let pending: Vec<_> = std::mem::take(&mut lowerer.pending_imports);
            for import in pending {
                eprintln!(
                    "[lowerer] Loading pending import '{}' at PC {}",
                    import.name,
                    lowerer.current_pc()
                );
                lowerer.lower_imported_function_to_pending(&import.name, &import.func)?;
            }
        }

        // Then, add resolver-imported functions after the main program
        for (name, func) in resolver_imports {
            lowerer.lower_imported_function_to_pending(&name, &func)?;
        }

        // Now patch all forward calls
        lowerer.patch_forward_calls()?;

        Ok((lowerer.bytecode, lowerer.functions))
    }

    /// Lower a single imported function to the pending section
    fn lower_imported_function_to_pending(
        &mut self,
        name: &str,
        func: &Function,
    ) -> LowerResult<()> {
        // Emit jump over the function so top-level execution doesn't run it
        self.emit(Opcode::Jump);
        let skip_pos = self.current_pc();
        self.emit_u32(0);

        // Now the function body starts
        let start_pc = self.current_pc() as u32;
        let param_count = func.parameters.len() as u32;

        self.functions
            .insert(name.to_string(), (start_pc, param_count));

        self.with_scope(|this| {
            this.bind_params(&func.parameters);
            this.lower_body(&func.body)?;
            this.emit(Opcode::Return);
            Ok(())
        })?;

        let end_pc = self.current_pc();
        self.patch_jump(skip_pos, end_pc)?;

        Ok(())
    }

    /// Lower a complete Program AST to bytecode.
    pub fn lower(program: &Program) -> LowerResult<(Bytecode, HashMap<String, (u32, u32)>)> {
        let mut lowerer = Lowerer::new();
        lowerer.lower_program(program)?;
        lowerer.patch_forward_calls()?;
        Ok((lowerer.bytecode, lowerer.functions))
    }

    fn lower_program(&mut self, program: &Program) -> LowerResult<()> {
        for item in &program.items {
            self.lower_item(item)?;
        }
        self.emit(Opcode::Halt);
        // Don't patch forward calls here - caller should do it after all functions are added
        Ok(())
    }

    fn lower_item(&mut self, item: &Item) -> LowerResult<()> {
        match item {
            Item::Function(func) => self.lower_function(func),
            Item::Agent(agent) => self.lower_agent(agent),
            Item::Cluster(cluster) => self.lower_cluster(cluster),
            Item::Module(module) => {
                for sub_item in &module.items {
                    self.lower_item(sub_item)?;
                }
                Ok(())
            }
            Item::Struct(_) => Ok(()), // No bytecode for struct definitions (types only)
            Item::Export(_) => Ok(()), // TODO: implement exports
            Item::Import(import) => self.lower_import(import),
        }
    }

    /// Lower an import: resolve module path, read file, parse, and queue functions for later loading
    fn lower_import(&mut self, import: &Import) -> LowerResult<()> {
        // Resolve the module path to a file
        let resolver = ModuleResolver::new();
        let style = ModuleResolver::detect_style(&import.module);

        let file_path = resolver
            .module_path_to_file(&import.module, style.clone())
            .ok_or_else(|| LowerError::new(format!("Module not found: {}", import.module)))?;

        eprintln!(
            "[import] Queueing module '{}' from {:?}",
            import.module, file_path
        );

        // Read and parse the module file
        let source = std::fs::read_to_string(&file_path).map_err(|e| {
            LowerError::new(format!("Failed to read {}: {}", file_path.display(), e))
        })?;

        let ast = AstParser::parse(&source).map_err(|e| {
            LowerError::new(format!("Parse error in {}: {:?}", file_path.display(), e))
        })?;

        // Queue imported functions for later loading (after main program)
        // This prevents PC corruption in the main program bytecode
        for item in &ast.items {
            if let Item::Function(func) = item {
                let import_name = if import.items.is_empty()
                    || import.items.contains(&crate::ast::ImportItem::Wildcard)
                {
                    // Wildcard import: use function name directly
                    func.name.clone()
                } else {
                    // Specific imports: check if this function is in the list
                    let func_name = &func.name;
                    let should_import = import.items.iter().any(|i| match i {
                        crate::ast::ImportItem::Named(name) => name == func_name,
                        crate::ast::ImportItem::Aliased { name, alias: _ } => name == func_name,
                        crate::ast::ImportItem::Wildcard => true,
                    });
                    if !should_import {
                        continue;
                    }
                    func_name.clone()
                };

                // Queue for later loading (don't emit bytecode now)
                self.pending_imports.push(PendingImport {
                    name: import_name.clone(),
                    func: func.clone(),
                });
                eprintln!(
                    "[import] Queued function '{}' from '{}'",
                    import_name, import.module
                );
            } else if let Item::Module(module) = item {
                // Recursively queue items from module blocks
                for sub_item in &module.items {
                    if let Item::Function(func) = sub_item {
                        self.pending_imports.push(PendingImport {
                            name: func.name.clone(),
                            func: func.clone(),
                        });
                        eprintln!(
                            "[import] Queued function '{}' from module '{}'",
                            func.name, import.module
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// Lower a single imported function (emit immediately - for use after main program)
    fn lower_imported_function(&mut self, name: &str, func: &Function) -> LowerResult<()> {
        // Emit jump over function body (same pattern as lower_function for non-main)
        self.emit(Opcode::Jump);
        let skip_pos = self.current_pc();
        self.emit_u32(0);

        let start_pc = self.current_pc() as u32;
        let param_count = func.parameters.len() as u32;

        self.functions
            .insert(name.to_string(), (start_pc, param_count));

        self.with_scope(|this| {
            this.bind_params(&func.parameters);
            this.lower_body(&func.body)?;
            this.emit(Opcode::Return);
            Ok(())
        })?;

        let end_pc = self.current_pc();
        self.patch_jump(skip_pos, end_pc)?;

        Ok(())
    }

    // ─── Functions ──────────────────────────────────────────────────────

    fn lower_function(&mut self, func: &Function) -> LowerResult<()> {
        let is_main = func.name == "main";
        let param_count = func.parameters.len() as u32;

        if !is_main {
            // Non-main: jump over the body so top-level execution skips it
            self.emit(Opcode::Jump);
            let skip_pos = self.current_pc();
            self.emit_u32(0);

            let start_pc = self.current_pc() as u32;
            self.functions
                .insert(func.name.clone(), (start_pc, param_count));

            self.with_scope(|this| {
                this.bind_params(&func.parameters);
                this.lower_body(&func.body)?;
                this.emit(Opcode::Return);
                Ok(())
            })?;

            let end_pc = self.current_pc();
            self.patch_jump(skip_pos, end_pc)?;
        } else {
            let start_pc = self.current_pc() as u32;
            self.functions
                .insert(func.name.clone(), (start_pc, param_count));

            self.with_scope(|this| {
                this.bind_params(&func.parameters);
                this.lower_body(&func.body)?;
                this.emit(Opcode::Return);
                Ok(())
            })?;
        }

        Ok(())
    }

    fn bind_params(&mut self, params: &[Parameter]) {
        for (i, param) in params.iter().enumerate() {
            self.variables.insert(param.name.clone(), (i + 1) as u8);
        }
    }

    fn lower_body(&mut self, stmts: &[Statement]) -> LowerResult<()> {
        for stmt in stmts {
            self.lower_statement(stmt)?;
        }
        Ok(())
    }

    // ─── Statements ─────────────────────────────────────────────────────

    fn lower_statement(&mut self, stmt: &Statement) -> LowerResult<()> {
        match &stmt.kind {
            StmtKind::Let {
                name, value, ty, ..
            } => {
                let reg = self.alloc_var(name)?;
                // Handle optional value - initialize to 0 if not provided
                if let Some(val) = value {
                    self.lower_expr(val, reg)?;
                } else {
                    // Initialize to 0 (default for primitives)
                    let idx = self.bytecode.add_constant(Value::I64(0));
                    self.emit(Opcode::Const);
                    self.emit_u8(reg);
                    self.emit_u32(idx);
                }
            }
            StmtKind::Assign { target, value } => {
                if let ExprKind::Identifier(name) = &target.kind {
                    let reg = self.resolve_or_alloc_var(name)?;
                    self.lower_expr(value, reg)?;
                } else {
                    // Index assignment, field assignment, etc. — emit to temp
                    let tmp = self.next_tmp_reg;
                    self.lower_expr(value, tmp)?;
                }
            }
            StmtKind::CompoundAssign { target, op, value } => {
                if let ExprKind::Identifier(name) = &target.kind {
                    let reg = self.resolve_or_alloc_var(name)?;
                    let tmp = self.alloc_tmp()?;
                    self.lower_expr(value, tmp)?;
                    self.emit_binop(*op, reg, reg, tmp);
                }
            }
            StmtKind::Return(opt_expr) => {
                if let Some(expr) = opt_expr {
                    self.lower_expr(expr, 0)?;
                }
                self.emit(Opcode::Return);
            }
            StmtKind::Expr(expr) => {
                let tmp = self.next_tmp_reg;
                self.lower_expr(expr, tmp)?;
            }
            StmtKind::If(if_stmt) => {
                // Evaluate condition into register 10
                self.lower_expr(&if_stmt.condition, 10)?;

                self.emit(Opcode::JumpIfNot);
                self.emit_u8(10);
                let else_jump = self.current_pc();
                self.emit_u32(0);

                self.lower_body(&if_stmt.then_body)?;

                self.emit(Opcode::Jump);
                let end_jump = self.current_pc();
                self.emit_u32(0);

                let else_pc = self.current_pc();
                self.patch_jump(else_jump, else_pc)?;

                self.lower_body(&if_stmt.else_body)?;

                let end_pc = self.current_pc();
                self.patch_jump(end_jump, end_pc)?;
            }
            StmtKind::Loop(loop_stmt) => {
                let loop_start = self.current_pc();

                self.lower_expr(&loop_stmt.condition, 10)?;

                self.emit(Opcode::JumpIfNot);
                self.emit_u8(10);
                let exit_jump = self.current_pc();
                self.emit_u32(0);

                self.loop_stack.push(LoopContext {
                    start_pc: loop_start,
                    break_patches: Vec::new(),
                });

                self.lower_body(&loop_stmt.body)?;

                self.emit(Opcode::Jump);
                self.emit_u32(loop_start as u32);

                let exit_pc = self.current_pc();
                self.patch_jump(exit_jump, exit_pc)?;

                // Patch any break statements
                if let Some(ctx) = self.loop_stack.pop() {
                    for bp in ctx.break_patches {
                        self.patch_jump(bp, exit_pc)?;
                    }
                }
            }
            StmtKind::While { condition, body } => {
                let loop_start = self.current_pc();

                self.lower_expr(condition, 10)?;

                self.emit(Opcode::JumpIfNot);
                self.emit_u8(10);
                let exit_jump = self.current_pc();
                self.emit_u32(0);

                self.loop_stack.push(LoopContext {
                    start_pc: loop_start,
                    break_patches: Vec::new(),
                });

                self.lower_body(body)?;

                self.emit(Opcode::Jump);
                self.emit_u32(loop_start as u32);

                let exit_pc = self.current_pc();
                self.patch_jump(exit_jump, exit_pc)?;

                if let Some(ctx) = self.loop_stack.pop() {
                    for bp in ctx.break_patches {
                        self.patch_jump(bp, exit_pc)?;
                    }
                }
            }
            StmtKind::Break => {
                self.emit(Opcode::Jump);
                let bp = self.current_pc();
                self.emit_u32(0);
                if let Some(ctx) = self.loop_stack.last_mut() {
                    ctx.break_patches.push(bp);
                }
            }
            StmtKind::Continue => {
                if let Some(ctx) = self.loop_stack.last() {
                    let start = ctx.start_pc;
                    self.emit(Opcode::Jump);
                    self.emit_u32(start as u32);
                }
            }
            StmtKind::Block(stmts) => {
                self.lower_body(stmts)?;
            }
            StmtKind::Switch(switch_stmt) => {
                // Lower discriminant into register 11
                self.lower_expr(&switch_stmt.discriminant, 11)?;

                let mut end_patches = Vec::new();

                for case in &switch_stmt.cases {
                    // Lower pattern value into register 12
                    match case.pattern {
                        crate::ast::Pattern::Int(n) => {
                            let idx = self.bytecode.add_constant(Value::I64(n));
                            self.emit(Opcode::Const);
                            self.emit_u8(12);
                            self.emit_u32(idx);
                        }
                        crate::ast::Pattern::String(ref s) => {
                            let idx = self.bytecode.add_constant(Value::String(s.clone()));
                            self.emit(Opcode::Const);
                            self.emit_u8(12);
                            self.emit_u32(idx);
                        }
                        _ => {
                            // Wildcard/identifier patterns: always match
                            let idx = self.bytecode.add_constant(Value::Bool(true));
                            self.emit(Opcode::Const);
                            self.emit_u8(12);
                            self.emit_u32(idx);
                        }
                    }

                    // Compare: r10 = (r11 == r12)
                    self.emit(Opcode::Eq);
                    self.emit_u8(10);
                    self.emit_u8(11);
                    self.emit_u8(12);

                    // Skip case body if no match
                    self.emit(Opcode::JumpIfNot);
                    self.emit_u8(10);
                    let skip_pos = self.current_pc();
                    self.emit_u32(0);

                    self.lower_body(&case.body)?;

                    // Jump to end of switch
                    self.emit(Opcode::Jump);
                    end_patches.push(self.current_pc());
                    self.emit_u32(0);

                    let next_case = self.current_pc();
                    self.patch_jump(skip_pos, next_case)?;
                }

                // Default body
                self.lower_body(&switch_stmt.default_body)?;

                let end_pc = self.current_pc();
                for ep in end_patches {
                    self.patch_jump(ep, end_pc)?;
                }
            }
            StmtKind::For {
                pattern,
                iterable,
                body,
            } => {
                // Lower: for item in collection { body }
                // As:
                //   i = 0
                //   len = collection.len()
                // loop_start:
                //   if i >= len goto loop_end
                //   item = collection[i]
                //   [body]
                //   i = i + 1
                //   goto loop_start
                // loop_end:

                // Allocate registers for loop state
                let i_reg = self.alloc_tmp()?;
                let len_reg = self.alloc_tmp()?;
                let item_reg = self.alloc_tmp()?;

                // Initialize i = 0
                let zero_idx = self.bytecode.add_constant(Value::I64(0));
                self.emit(Opcode::Const);
                self.emit_u8(i_reg);
                self.emit_u32(zero_idx);

                // Get collection and its length
                let coll_reg = self.alloc_tmp()?;
                self.lower_expr(iterable, coll_reg)?;

                // len = collection.len()
                self.emit(Opcode::Len);
                self.emit_u8(len_reg);
                self.emit_u8(coll_reg);

                // loop_start label
                let loop_start = self.current_pc();

                // cmp = i >= len
                self.emit(Opcode::Ge);
                self.emit_u8(item_reg); // reuse item_reg for cmp result
                self.emit_u8(i_reg);
                self.emit_u8(len_reg);

                // if cmp goto loop_end
                self.emit(Opcode::JumpIf);
                self.emit_u8(item_reg);
                let exit_jump = self.current_pc();
                self.emit_u32(0); // placeholder

                // item = collection[i]
                self.emit(Opcode::Get);
                self.emit_u8(item_reg);
                self.emit_u8(coll_reg);
                self.emit_u8(i_reg);

                // Bind pattern to item_reg - for simple ident pattern, store in variable
                if let Pattern::Identifier(name) = pattern {
                    self.variables.insert(name.clone(), item_reg);
                }

                // Lower body
                self.lower_body(body)?;

                // i = i + 1
                let one_idx = self.bytecode.add_constant(Value::I64(1));
                let tmp_reg = self.alloc_tmp()?;
                self.emit(Opcode::Const);
                self.emit_u8(tmp_reg);
                self.emit_u32(one_idx);

                self.emit(Opcode::Add);
                self.emit_u8(i_reg);
                self.emit_u8(i_reg);
                self.emit_u8(tmp_reg);

                // goto loop_start
                self.emit(Opcode::Jump);
                self.emit_u32(loop_start as u32);

                // loop_end - patch exit jump
                let loop_end = self.current_pc();
                self.patch_jump(exit_jump, loop_end)?;
            }
            StmtKind::Module(_) | StmtKind::Import(_) | StmtKind::Export(_) => {}
        }
        Ok(())
    }

    // ─── Expressions ────────────────────────────────────────────────────

    fn lower_expr(&mut self, expr: &Expression, dst: u8) -> LowerResult<()> {
        match &expr.kind {
            ExprKind::Int(n) => {
                let idx = self.bytecode.add_constant(Value::I64(*n));
                self.emit(Opcode::Const);
                self.emit_u8(dst);
                self.emit_u32(idx);
            }
            ExprKind::Float(n) => {
                let idx = self.bytecode.add_constant(Value::F64(*n));
                self.emit(Opcode::Const);
                self.emit_u8(dst);
                self.emit_u32(idx);
            }
            ExprKind::String(s) => {
                let idx = self.bytecode.add_constant(Value::String(s.clone()));
                self.emit(Opcode::Const);
                self.emit_u8(dst);
                self.emit_u32(idx);
            }
            ExprKind::Bool(b) => {
                let idx = self.bytecode.add_constant(Value::Bool(*b));
                self.emit(Opcode::Const);
                self.emit_u8(dst);
                self.emit_u32(idx);
            }
            ExprKind::Nil => {
                let idx = self.bytecode.add_constant(Value::Nil);
                self.emit(Opcode::Const);
                self.emit_u8(dst);
                self.emit_u32(idx);
            }
            ExprKind::Void => {
                let idx = self.bytecode.add_constant(Value::Void);
                self.emit(Opcode::Const);
                self.emit_u8(dst);
                self.emit_u32(idx);
            }
            ExprKind::Identifier(name) => {
                if let Some(&src_reg) = self.variables.get(name.as_str()) {
                    self.emit(Opcode::Move);
                    self.emit_u8(dst);
                    self.emit_u8(src_reg);
                } else {
                    // Unknown variable — emit Nil
                    let idx = self.bytecode.add_constant(Value::Nil);
                    self.emit(Opcode::Const);
                    self.emit_u8(dst);
                    self.emit_u32(idx);
                }
            }
            ExprKind::BinaryOp { op, left, right } => {
                self.lower_expr(left, dst)?;
                let rhs = self.alloc_tmp()?;
                self.lower_expr(right, rhs)?;
                self.emit_binop(*op, dst, dst, rhs);
            }
            ExprKind::UnaryOp { op, operand } => {
                self.lower_expr(operand, dst)?;
                match op {
                    UnaryOp::Neg => {
                        self.emit(Opcode::Neg);
                        self.emit_u8(dst);
                        self.emit_u8(dst);
                    }
                    UnaryOp::Not => {
                        self.emit(Opcode::Not);
                        self.emit_u8(dst);
                        self.emit_u8(dst);
                    }
                    UnaryOp::BitNot => {
                        // No dedicated opcode — emit Not as approximation
                        self.emit(Opcode::Not);
                        self.emit_u8(dst);
                        self.emit_u8(dst);
                    }
                }
            }
            ExprKind::Call {
                function,
                arguments,
            } => {
                let arg_base = 150u8;
                for (i, arg) in arguments.iter().enumerate() {
                    self.lower_expr(arg, arg_base + i as u8)?;
                }
                self.emit(Opcode::Call);
                let name_idx = self.get_or_add_string(function);
                self.emit_u32(name_idx);
                self.emit_u8(arguments.len() as u8);
                self.emit_u8(dst);

                if !self.functions.contains_key(function) {
                    let call_site = self.current_pc() - 6; // -6 because emit() writes u16 (2 bytes) + u32 name_idx (4 bytes) = 6 bytes before arg_count/dst
                    self.patch_points.push((call_site, function.clone()));
                }
            }
            ExprKind::Array(elements) => {
                // Push each element, then build array
                for (i, elem) in elements.iter().enumerate() {
                    let reg = 150u8 + i as u8;
                    self.lower_expr(elem, reg)?;
                    self.emit(Opcode::Push);
                    self.emit_u8(reg);
                }
                // Store array length as constant
                let len_idx = self
                    .bytecode
                    .add_constant(Value::I64(elements.len() as i64));
                self.emit(Opcode::Const);
                self.emit_u8(dst);
                self.emit_u32(len_idx);
            }
            ExprKind::Index { array, index } => {
                self.lower_expr(array, dst)?;
                let idx_reg = self.alloc_tmp()?;
                self.lower_expr(index, idx_reg)?;
                self.emit(Opcode::Get);
                self.emit_u8(dst);
                self.emit_u8(dst);
                self.emit_u8(idx_reg);
            }
            ExprKind::FieldAccess { object, field } => {
                self.lower_expr(object, dst)?;
                let field_idx = self.get_or_add_string(field);
                let tmp = self.alloc_tmp()?;
                let idx = self.bytecode.add_constant(Value::String(field.clone()));
                self.emit(Opcode::Const);
                self.emit_u8(tmp);
                self.emit_u32(idx);
                self.emit(Opcode::Get);
                self.emit_u8(dst);
                self.emit_u8(dst);
                self.emit_u8(tmp);
                let _ = field_idx; // used for string table
            }
            ExprKind::MethodCall {
                object,
                method,
                arguments,
            } => {
                // Lower as function call with object as first argument
                self.lower_expr(object, 150)?;
                for (i, arg) in arguments.iter().enumerate() {
                    self.lower_expr(arg, 151 + i as u8)?;
                }
                self.emit(Opcode::Call);
                let name_idx = self.get_or_add_string(method);
                self.emit_u32(name_idx);
                self.emit_u8((arguments.len() + 1) as u8);
                self.emit_u8(dst);
            }
            ExprKind::Collapse(inner) => {
                // Collapse a latent value
                self.lower_expr(inner, dst)?;
            }
            ExprKind::Resolve(inner) => {
                // Resolve a latent handle
                self.lower_expr(inner, dst)?;
            }
            ExprKind::Dict(pairs) => {
                // Create empty map in dst, then insert each key-value pair
                self.emit(Opcode::MapCreate);
                self.emit_u8(dst);
                for (key, val) in pairs {
                    let key_reg = self.alloc_tmp()?;
                    let val_reg = self.alloc_tmp()?;
                    self.lower_expr(key, key_reg)?;
                    self.lower_expr(val, val_reg)?;
                    self.emit(Opcode::MapSet);
                    self.emit_u8(dst);
                    self.emit_u8(key_reg);
                    self.emit_u8(val_reg);
                }
            }
            // Lower match expression as chain of comparisons with jumps
            ExprKind::Match { value, cases } => {
                // Evaluate value first
                let val_reg = self.alloc_tmp()?;
                self.lower_expr(value, val_reg)?;

                // Track patches for end jumps
                let mut end_patches = Vec::new();

                for case in cases {
                    // Skip wildcard - will be handled at end
                    if matches!(case.pattern, Pattern::Wildcard) {
                        // Lower wildcard body directly
                        self.lower_expr(&case.body, dst)?;
                        break;
                    }

                    // Load pattern value into temp register
                    let pat_reg = self.alloc_tmp()?;
                    match &case.pattern {
                        Pattern::Int(n) => {
                            let idx = self.bytecode.add_constant(Value::I64(*n));
                            self.emit(Opcode::Const);
                            self.emit_u8(pat_reg);
                            self.emit_u32(idx);
                        }
                        Pattern::String(s) => {
                            let idx = self.bytecode.add_constant(Value::String(s.clone()));
                            self.emit(Opcode::Const);
                            self.emit_u8(pat_reg);
                            self.emit_u32(idx);
                        }
                        Pattern::Identifier(_) => {
                            // Binding pattern - treat as wildcard for comparison
                            // (always matches, binds value to name)
                            let idx = self.bytecode.add_constant(Value::Bool(true));
                            self.emit(Opcode::Const);
                            self.emit_u8(pat_reg);
                            self.emit_u32(idx);
                        }
                        _ => {
                            // Unsupported pattern - emit nop for now
                            self.emit(Opcode::Nop);
                            continue;
                        }
                    }

                    // Compare value == pattern
                    let cmp_reg = self.alloc_tmp()?;
                    self.emit(Opcode::Eq);
                    self.emit_u8(cmp_reg);
                    self.emit_u8(val_reg);
                    self.emit_u8(pat_reg);

                    // If not equal, jump to next case
                    self.emit(Opcode::JumpIfNot);
                    self.emit_u8(cmp_reg);
                    let next_case_jump = self.current_pc();
                    self.emit_u32(0); // placeholder

                    // Lower case body
                    self.lower_expr(&case.body, dst)?;

                    // Jump to end
                    self.emit(Opcode::Jump);
                    let end_jump = self.current_pc();
                    self.emit_u32(0); // placeholder
                    end_patches.push(end_jump);

                    // Patch next case jump
                    let next_case_pc = self.current_pc();
                    self.patch_jump(next_case_jump, next_case_pc)?;
                }

                // Patch all end jumps
                let end_pc = self.current_pc();
                for ep in end_patches {
                    self.patch_jump(ep, end_pc)?;
                }
            }
            // Unsupported in bytecode — emit Nop placeholders
            ExprKind::Range { .. }
            | ExprKind::Contract { .. }
            | ExprKind::Lambda { .. }
            | ExprKind::Conditional { .. } => {
                self.emit(Opcode::Nop);
            }
        }
        Ok(())
    }

    fn emit_binop(&mut self, op: BinaryOp, dst: u8, left: u8, right: u8) {
        let opcode = match op {
            BinaryOp::Add => Opcode::Add,
            BinaryOp::Sub => Opcode::Sub,
            BinaryOp::Mul => Opcode::Mul,
            BinaryOp::Div => Opcode::Div,
            BinaryOp::Mod => Opcode::Mod,
            BinaryOp::Eq => Opcode::Eq,
            BinaryOp::Ne => Opcode::Ne,
            BinaryOp::Lt => Opcode::Lt,
            BinaryOp::Le => Opcode::Le,
            BinaryOp::Gt => Opcode::Gt,
            BinaryOp::Ge => Opcode::Ge,
            BinaryOp::And => Opcode::And,
            BinaryOp::Or => Opcode::Or,
            // For ops without direct opcodes, use Nop + dst unchanged
            BinaryOp::Pow
            | BinaryOp::BitAnd
            | BinaryOp::BitOr
            | BinaryOp::BitXor
            | BinaryOp::Shl
            | BinaryOp::Shr => {
                self.emit(Opcode::Nop);
                return;
            }
        };
        self.emit(opcode);
        self.emit_u8(dst);
        self.emit_u8(left);
        self.emit_u8(right);
    }

    // ─── Agents ─────────────────────────────────────────────────────────

    fn lower_agent(&mut self, agent: &AgentDef) -> LowerResult<()> {
        let name_idx = self.get_or_add_string(&agent.name);

        // Spawn the agent
        self.emit(Opcode::AgentSpawn);
        self.emit_u32(name_idx);
        self.emit_u32(0); // flags

        // Initialize latent state
        for latent in &agent.latents {
            let latent_name_idx = self.get_or_add_string(&latent.name);
            if let Some(ref init) = latent.initializer {
                self.lower_expr(init, 0)?;
                self.emit(Opcode::LatentSet);
                self.emit_u32(latent_name_idx);
                self.emit_u8(0);
            }
        }

        // Lower governance
        if let Some(ref govern) = agent.govern {
            let effect_name = govern.effect.name();
            let effect_idx = self.get_or_add_string(effect_name);
            self.emit(Opcode::GovernRegister);
            self.emit_u32(effect_idx);

            // Set trust threshold as confidence
            let trust_idx = self
                .bytecode
                .add_constant(Value::F64(govern.trust_threshold));
            self.emit(Opcode::Const);
            self.emit_u8(0);
            self.emit_u32(trust_idx);
            self.emit(Opcode::GovernSetConfidence);
            self.emit_u8(0);

            // Emit governance check
            // VM expects: dst (u8), effect_type (u8), desc_idx (u32)
            let effect_type: u8 = 0; // Modify effect type
            let desc_idx = self.get_or_add_string("governance check");
            self.emit(Opcode::GovernCheck);
            self.emit_u8(0); // result reg
            self.emit_u8(effect_type);
            self.emit_u32(desc_idx);
        }

        // Lower cycles
        for cycle in &agent.cycles {
            let level_str = cycle.level.name();
            let _level_idx = self.get_or_add_string(&level_str);

            self.emit(Opcode::CycleBegin);
            self.emit_u8(match cycle.level {
                CycleLevel::H => 0,
                CycleLevel::L => 1,
                CycleLevel::Custom(n) => n as u8,
            });
            self.emit_u8(cycle.iterations as u8);

            self.lower_body(&cycle.body)?;

            self.emit(Opcode::CycleEnd);
            self.emit_u8(0);
        }

        // Lower main agent body
        self.lower_body(&agent.body)?;

        // Lower modify block
        if let Some(ref modify) = agent.modify {
            for gate in &modify.gates {
                match gate {
                    Gate::Proof { name, .. } => {
                        let gate_idx = self.get_or_add_string(name);
                        let _ = gate_idx;
                    }
                    Gate::Consensus { .. } => {}
                    Gate::Human { .. } => {}
                    Gate::SafetyCheck { .. } => {}
                }
            }
        }

        Ok(())
    }

    fn lower_cluster(&mut self, cluster: &crate::ast::ClusterDef) -> LowerResult<()> {
        let cluster_name_idx = self.get_or_add_string(&cluster.name);
        self.emit(Opcode::ScaleCreate);
        self.emit_u32(cluster_name_idx);

        for agent_ref in &cluster.agents {
            let agent_idx = self.get_or_add_string(&agent_ref.name);
            self.emit(Opcode::ScaleAddAgent);
            self.emit_u32(agent_idx);
        }

        for barrier in &cluster.barriers {
            let barrier_idx = self.get_or_add_string(&barrier.name);
            self.emit(Opcode::BarrierCreate);
            self.emit_u32(barrier_idx);
            self.emit_u8(barrier.expected as u8);
        }

        Ok(())
    }

    // ─── Helpers ────────────────────────────────────────────────────────

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

    fn patch_jump(&mut self, jump_pos: usize, target: usize) -> LowerResult<()> {
        if jump_pos + 4 > self.bytecode.code.len() {
            return Err(LowerError::new(format!(
                "Jump patch position {} out of bounds (code len {})",
                jump_pos,
                self.bytecode.code.len()
            )));
        }
        self.bytecode.code[jump_pos..jump_pos + 4].copy_from_slice(&(target as u32).to_le_bytes());
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

    fn alloc_var(&mut self, name: &str) -> LowerResult<u8> {
        let reg = self.next_var_reg;
        if reg >= 200 {
            // Reserve regs 0-199 for variables, 200+ for temps
            return Err(LowerError::new("Too many variables (max 200)"));
        }
        self.next_var_reg += 1;
        self.variables.insert(name.to_string(), reg);
        Ok(reg)
    }

    fn resolve_or_alloc_var(&mut self, name: &str) -> LowerResult<u8> {
        if let Some(&reg) = self.variables.get(name) {
            Ok(reg)
        } else {
            self.alloc_var(name)
        }
    }

    fn alloc_tmp(&mut self) -> LowerResult<u8> {
        let reg = self.next_tmp_reg;
        if reg >= 230 {
            return Err(LowerError::new(
                "Expression too complex (temp register overflow)",
            ));
        }
        self.next_tmp_reg += 1;
        Ok(reg)
    }

    fn with_scope<F>(&mut self, f: F) -> LowerResult<()>
    where
        F: FnOnce(&mut Self) -> LowerResult<()>,
    {
        let saved_vars = self.variables.clone();
        let saved_var_reg = self.next_var_reg;
        let saved_tmp_reg = self.next_tmp_reg;

        self.variables.clear();
        self.next_var_reg = 0;
        self.next_tmp_reg = 200;

        let result = f(self);

        self.variables = saved_vars;
        self.next_var_reg = saved_var_reg;
        self.next_tmp_reg = saved_tmp_reg;

        result
    }

    fn patch_forward_calls(&mut self) -> LowerResult<()> {
        let patches = std::mem::take(&mut self.patch_points);
        for (call_site, func_name) in patches {
            if let Some(&(start_pc, _)) = self.functions.get(&func_name) {
                // Found the function - convert CALL to CALL_ADDR for direct jump
                // call_site points to the name_idx (4 bytes), which we replace with 16-bit PC
                // Layout was: CALL [1] + name_idx [4] + arg_count [1] + dst [1] = 7 bytes
                // New layout: CALL_ADDR [1] + pc_lo [1] + pc_hi [1] + arg_count [1] + dst [1] = 5 bytes
                // We write: CALL_ADDR opcode, then 16-bit PC, then arg_count, then dst
                // Need to shift bytes due to size difference (7 -> 5)

                // call_site points to name_idx[0] (first byte after the u16 opcode).
                // CALL layout: [op_lo][op_hi][name_idx u32 4 bytes][arg_count u8][dst u8] = 8 bytes
                // CALL_ADDR layout: same 8 bytes — opcode replaced, name_idx slot reused as u32 PC.
                // arg_count and dst stay at call_site+4 and call_site+5 — no shifting needed.
                let call_opcode_pos = call_site - 2; // low byte of the u16 opcode slot
                if call_site + 3 < self.bytecode.code.len() {
                    // Overwrite opcode low byte with CallAddr; high byte is already 0x00
                    self.bytecode.code[call_opcode_pos] = Opcode::CallAddr as u8;

                    // Write 32-bit PC into the name_idx slot (same 4 bytes, no size change)
                    self.bytecode.code[call_site] = (start_pc & 0xFF) as u8;
                    self.bytecode.code[call_site + 1] = ((start_pc >> 8) & 0xFF) as u8;
                    self.bytecode.code[call_site + 2] = ((start_pc >> 16) & 0xFF) as u8;
                    self.bytecode.code[call_site + 3] = ((start_pc >> 24) & 0xFF) as u8;
                    // arg_count at call_site+4 and dst at call_site+5 remain untouched

                    eprintln!("[patch] {} -> CALL_ADDR PC {}", func_name, start_pc);
                }
            }
            // If function not found, leave the patch as-is (could be a built-in)
        }
        Ok(())
    }
}

impl Default for Lowerer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast_parser::AstParser;

    fn lower_source(source: &str) -> LowerResult<(Bytecode, HashMap<String, (u32, u32)>)> {
        let ast = AstParser::parse(source).map_err(|e| LowerError::new(e.message))?;
        Lowerer::lower(&ast)
    }

    #[test]
    fn test_lower_simple_return() {
        let source = r#"
            fn main() -> i64 {
                return 42;
            }
        "#;
        let (bc, funcs) = lower_source(source).expect("Lower failed");
        assert!(funcs.contains_key("main"));
        assert!(!bc.code.is_empty());

        let mut vm = crate::Vm::new();
        vm.load_functions(&funcs);
        let result = vm.run(&bc).unwrap();
        assert_eq!(result, Value::I64(42));
    }

    #[test]
    fn test_lower_arithmetic() {
        let source = r#"
            fn main() -> i64 {
                let x = 10 + 32;
                return x;
            }
        "#;
        let (bc, funcs) = lower_source(source).expect("Lower failed");

        let mut vm = crate::Vm::new();
        vm.load_functions(&funcs);
        let result = vm.run(&bc).unwrap();
        assert_eq!(result, Value::I64(42));
    }

    #[test]
    fn test_lower_if() {
        let source = r#"
            fn main() -> i64 {
                let x = 10;
                if (x > 5) {
                    return 100;
                } else {
                    return 0;
                }
            }
        "#;
        let (bc, funcs) = lower_source(source).expect("Lower failed");

        let mut vm = crate::Vm::new();
        vm.load_functions(&funcs);
        let result = vm.run(&bc).unwrap();
        assert_eq!(result, Value::I64(100));
    }

    #[test]
    fn test_lower_loop() {
        let source = r#"
            fn main() -> i64 {
                let sum = 0;
                let i = 1;
                loop(i < 6) {
                    sum = sum + i;
                    i = i + 1;
                }
                return sum;
            }
        "#;
        let (bc, funcs) = lower_source(source).expect("Lower failed");

        let mut vm = crate::Vm::new().with_max_steps(100000);
        vm.load_functions(&funcs);
        let result = vm.run(&bc).unwrap();
        assert_eq!(result, Value::I64(15));
    }

    #[test]
    fn test_lower_function_call() {
        let source = r#"
            fn add(a: i64, b: i64) -> i64 {
                return a + b;
            }
            fn main() -> i64 {
                let x = add(10, 32);
                return x;
            }
        "#;
        let (bc, funcs) = lower_source(source).expect("Lower failed");

        let mut vm = crate::Vm::new().with_max_steps(10000);
        vm.load_functions(&funcs);
        let result = vm.run(&bc).unwrap();
        assert_eq!(result, Value::I64(42));
    }

    #[test]
    fn test_lower_recursive_fib() {
        let source = r#"
            fn fib(n: i64) -> i64 {
                if (n < 2) {
                    return n;
                }
                return fib(n - 1) + fib(n - 2);
            }
            fn main() -> i64 {
                return fib(10);
            }
        "#;
        let (bc, funcs) = lower_source(source).expect("Lower failed");

        let mut vm = crate::Vm::new().with_max_steps(100000);
        vm.load_functions(&funcs);
        let result = vm.run(&bc).unwrap();
        assert_eq!(result, Value::I64(55));
    }

    #[test]
    fn test_lower_agent_parses() {
        let source = r#"
            recursive agent Counter {
                latent count: i64 = 0;
                cycle H(10) {
                    count = count + 1;
                }
            }
        "#;
        let (bc, _funcs) = lower_source(source).expect("Lower failed");
        assert!(!bc.code.is_empty());
    }

    #[test]
    fn test_lower_agent_with_govern() {
        let source = r#"
            recursive agent SafeAgent {
                latent state: i64 = 0;
                govern {
                    effect: modify;
                    conscience: [no_harm, path_safety];
                    trust: 0.8;
                }
                cycle H(5) {
                    state = state + 1;
                }
            }
        "#;
        let (bc, _funcs) = lower_source(source).expect("Lower failed");
        assert!(!bc.code.is_empty());
    }

    #[test]
    fn test_lower_agent_with_modify() {
        let source = r#"
            recursive agent SelfModifying {
                latent value: f64 = 0.0;
                modify self {
                    gate proof;
                    cooldown: 100;
                }
            }
        "#;
        let (bc, _funcs) = lower_source(source).expect("Lower failed");
        assert!(!bc.code.is_empty());
    }

    #[test]
    fn test_lower_nested_cycles() {
        let source = r#"
            fn main() -> i64 {
                let h = 0;
                let l = 0;
                let result = 0;
                loop(h < 3) {
                    l = 0;
                    loop(l < 6) {
                        result = result + 1;
                        l = l + 1;
                    }
                    h = h + 1;
                }
                return result;
            }
        "#;
        let (bc, funcs) = lower_source(source).expect("Lower failed");

        let mut vm = crate::Vm::new().with_max_steps(100000);
        vm.load_functions(&funcs);
        let result = vm.run(&bc).unwrap();
        assert_eq!(result, Value::I64(18));
    }

    // ─── E2E: Source → AST → Bytecode → VM ─────────────────────────────

    #[test]
    fn test_full_pipeline_e2e() {
        let source = r#"
            fn double(x: i64) -> i64 {
                return x + x;
            }
            fn main() -> i64 {
                let a = double(5);
                let b = double(a);
                return b;
            }
        "#;

        // Stage 1: Source → AST
        let ast = AstParser::parse(source).expect("Parse failed");
        assert!(!ast.name.is_empty());

        // Stage 2: AST → Bytecode
        let (bc, funcs) = Lowerer::lower(&ast).expect("Lower failed");
        assert!(funcs.contains_key("double"));
        assert!(funcs.contains_key("main"));

        // Stage 3: Bytecode → Execution
        let mut vm = crate::Vm::new().with_max_steps(10000);
        vm.load_functions(&funcs);
        let result = vm.run(&bc).expect("VM execution failed");
        assert_eq!(result, Value::I64(20));
    }
}
