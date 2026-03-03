//! AST → Bytecode Lowerer
//!
//! Consumes a `Program` AST (from `ast_parser`) and emits `Bytecode` for the VM.
//! This is the bridge between the rich, introspectable AST and executable bytecode.

use crate::ast::{
    AgentDef, BinaryOp, CycleLevel, ExprKind, Expression, Function, Gate, Import, Item,
    ModificationKind, ModuleDef, Parameter, Pattern, Program, Statement, StmtKind, UnaryOp,
};
use crate::ast_parser::AstParser;
use crate::resolver::{ImportStyle, ModuleResolver};
use crate::tensor::Tensor;
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
    /// Whether we're currently inside __top_level__ (module-level init)
    in_top_level: bool,
    /// Current source line for line table generation
    current_line: u32,
    /// Counter for generating unique lambda names
    lambda_counter: u32,
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
            in_top_level: false,
            current_line: 0,
            lambda_counter: 0,
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
                log::debug!(
                    target: "lowerer",
                    "Loading pending import '{}' at PC {}",
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
            Item::Export(export) => {
                // Lower the exported item (typically a function)
                self.lower_item(&export.item)
            }
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

        log::debug!(
            target: "lowerer",
            "Queueing module '{}' from {:?}",
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
                log::debug!(
                    target: "lowerer",
                    "Queued function '{}' from '{}'",
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
                        log::debug!(
                            target: "lowerer",
                            "Queued function '{}' from module '{}'",
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
        let is_top_level = func.name == "__top_level__";
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
                this.in_top_level = is_top_level;
                this.bind_params(&func.parameters);
                this.lower_body(&func.body)?;
                this.emit(Opcode::Return);
                this.in_top_level = false;
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

    /// Reserved register layout:
    ///   0       = return value
    ///   1..N    = function parameters (loaded by VM call_function)
    ///   10      = condition register (if/loop/while conditions)
    ///   20+     = local variables (safe zone)
    ///   150+    = call argument base
    ///   200+    = temp registers
    const LOCAL_VAR_BASE: u8 = 20;

    fn bind_params(&mut self, params: &[Parameter]) {
        for (i, param) in params.iter().enumerate() {
            self.variables.insert(param.name.clone(), (i + 1) as u8);
        }
        // Local variables start at LOCAL_VAR_BASE to avoid colliding with
        // register 0 (return), registers 1-N (parameters), and register 10 (conditions).
        if self.next_var_reg < Self::LOCAL_VAR_BASE {
            self.next_var_reg = Self::LOCAL_VAR_BASE;
        }
    }

    fn lower_body(&mut self, stmts: &[Statement]) -> LowerResult<()> {
        for stmt in stmts {
            self.lower_statement(stmt)?;
            // Reset temp registers after each statement to prevent overflow
            self.next_tmp_reg = 200;
        }
        Ok(())
    }

    // ─── Statements ─────────────────────────────────────────────────────

    fn lower_statement(&mut self, stmt: &Statement) -> LowerResult<()> {
        if stmt.span.start_line > 0 {
            let pc = self.current_pc() as u32;
            self.bytecode.line_table.push((pc, stmt.span.start_line));
        }
        match &stmt.kind {
            StmtKind::Let {
                name, value, ty, ..
            } => {
                let reg = self.alloc_var(name)?;
                if let Some(val) = value {
                    self.lower_expr(val, reg)?;
                } else {
                    // Auto-initialize based on type annotation
                    let default_val = if let Some(ann) = ty {
                        if ann.name.starts_with("Tensor[") {
                            let size: usize = ann
                                .name
                                .trim_start_matches("Tensor[")
                                .trim_end_matches(']')
                                .parse()
                                .unwrap_or(64);
                            Value::Tensor(Tensor::zeros(vec![size]))
                        } else if ann.name.starts_with('[') && ann.name.contains(';') {
                            // Array type: [Type; N] — e.g. "[i64; 10]", "[f64; 10]"
                            let parts: Vec<&str> = ann
                                .name
                                .trim_matches(|c| c == '[' || c == ']')
                                .split(';')
                                .collect();
                            let size: usize = parts
                                .get(1)
                                .and_then(|s| s.trim().parse().ok())
                                .unwrap_or(0);
                            let elem_type = parts.first().map(|s| s.trim()).unwrap_or("");
                            let default_elem = match elem_type {
                                "f64" => Value::F64(0.0),
                                "String" => Value::String(String::new()),
                                _ => Value::I64(0),
                            };
                            Value::Array(vec![default_elem; size])
                        } else if ann.name == "String" {
                            Value::String(String::new())
                        } else if ann.name == "f64" {
                            Value::F64(0.0)
                        } else if ann.name == "dict" {
                            Value::Map(std::collections::BTreeMap::new())
                        } else {
                            Value::I64(0)
                        }
                    } else {
                        Value::I64(0)
                    };
                    let idx = self.bytecode.add_constant(default_val);
                    self.emit(Opcode::Const);
                    self.emit_u8(reg);
                    self.emit_u32(idx);
                }
                // In __top_level__, persist to latent state so other functions can read it
                if self.in_top_level {
                    let name_idx = self.get_or_add_string(name) as u32;
                    self.emit(Opcode::LatentSet);
                    self.emit_u32(name_idx);
                    self.emit_u8(reg);
                }
            }
            StmtKind::Assign { target, value } => {
                if let ExprKind::Identifier(name) = &target.kind {
                    if let Some(&reg) = self.variables.get(name.as_str()) {
                        // Local variable — write to its register
                        self.lower_expr(value, reg)?;
                    } else {
                        // Not a local variable — treat as latent (module-level)
                        let tmp = self.alloc_tmp()?;
                        self.lower_expr(value, tmp)?;
                        let name_idx = self.get_or_add_string(name) as u32;
                        self.emit(Opcode::LatentSet);
                        self.emit_u32(name_idx);
                        self.emit_u8(tmp);
                    }
                } else if let ExprKind::Index { array, index } = &target.kind {
                    // Index assignment: arr[idx] = value
                    if let ExprKind::Identifier(arr_name) = &array.kind {
                        if let Some(&arr_reg) = self.variables.get(arr_name.as_str()) {
                            // Local array — emit Set opcode
                            let idx_reg = self.alloc_tmp()?;
                            self.lower_expr(index, idx_reg)?;
                            let val_reg = self.alloc_tmp()?;
                            self.lower_expr(value, val_reg)?;
                            self.emit(Opcode::Set);
                            self.emit_u8(arr_reg);
                            self.emit_u8(idx_reg);
                            self.emit_u8(val_reg);
                        } else {
                            // Latent array — read, modify, write back
                            let arr_reg = self.alloc_tmp()?;
                            let name_idx = self.get_or_add_string(arr_name) as u32;
                            self.emit(Opcode::LatentGet);
                            self.emit_u8(arr_reg);
                            self.emit_u32(name_idx);
                            let idx_reg = self.alloc_tmp()?;
                            self.lower_expr(index, idx_reg)?;
                            let val_reg = self.alloc_tmp()?;
                            self.lower_expr(value, val_reg)?;
                            self.emit(Opcode::Set);
                            self.emit_u8(arr_reg);
                            self.emit_u8(idx_reg);
                            self.emit_u8(val_reg);
                            self.emit(Opcode::LatentSet);
                            self.emit_u32(name_idx);
                            self.emit_u8(arr_reg);
                        }
                    } else {
                        // Complex array expression — evaluate array to temp
                        let arr_reg = self.alloc_tmp()?;
                        self.lower_expr(array, arr_reg)?;
                        let idx_reg = self.alloc_tmp()?;
                        self.lower_expr(index, idx_reg)?;
                        let val_reg = self.alloc_tmp()?;
                        self.lower_expr(value, val_reg)?;
                        self.emit(Opcode::Set);
                        self.emit_u8(arr_reg);
                        self.emit_u8(idx_reg);
                        self.emit_u8(val_reg);
                    }
                } else if let ExprKind::FieldAccess { object, field } = &target.kind {
                    // Field assignment: obj.field = value
                    if let ExprKind::Identifier(obj_name) = &object.kind {
                        if let Some(&obj_reg) = self.variables.get(obj_name.as_str()) {
                            // Local object — obj_reg is in saved range (0-149), safe across calls.
                            // Evaluate value first so any function call doesn't clobber field_reg.
                            let val_reg = self.alloc_tmp()?;
                            self.lower_expr(value, val_reg)?;
                            let field_reg = self.alloc_tmp()?;
                            let field_const =
                                self.bytecode.add_constant(Value::String(field.clone()));
                            self.emit(Opcode::Const);
                            self.emit_u8(field_reg);
                            self.emit_u32(field_const);
                            self.emit(Opcode::Set);
                            self.emit_u8(obj_reg);
                            self.emit_u8(field_reg);
                            self.emit_u8(val_reg);
                        } else {
                            // Latent object — obj_reg is a temp (200+), not saved across calls.
                            // Evaluate value first, then LatentGet, then load field name.
                            let name_idx = self.get_or_add_string(obj_name) as u32;
                            let val_reg = self.alloc_tmp()?;
                            self.lower_expr(value, val_reg)?;
                            let obj_reg = self.alloc_tmp()?;
                            self.emit(Opcode::LatentGet);
                            self.emit_u8(obj_reg);
                            self.emit_u32(name_idx);
                            let field_reg = self.alloc_tmp()?;
                            let field_const =
                                self.bytecode.add_constant(Value::String(field.clone()));
                            self.emit(Opcode::Const);
                            self.emit_u8(field_reg);
                            self.emit_u32(field_const);
                            self.emit(Opcode::Set);
                            self.emit_u8(obj_reg);
                            self.emit_u8(field_reg);
                            self.emit_u8(val_reg);
                            self.emit(Opcode::LatentSet);
                            self.emit_u32(name_idx);
                            self.emit_u8(obj_reg);
                        }
                    } else if let ExprKind::Index { array, index } = &object.kind {
                        // arr[idx].field = value — read-modify-write for indexed struct fields
                        if let ExprKind::Identifier(arr_name) = &array.kind {
                            let val_reg = self.alloc_tmp()?;
                            self.lower_expr(value, val_reg)?;
                            let idx_reg = self.alloc_tmp()?;
                            self.lower_expr(index, idx_reg)?;
                            if let Some(&arr_reg) = self.variables.get(arr_name.as_str()) {
                                // Local array of structs: read element, modify field, write back
                                let obj_reg = self.alloc_tmp()?;
                                self.emit(Opcode::Get);
                                self.emit_u8(obj_reg);
                                self.emit_u8(arr_reg);
                                self.emit_u8(idx_reg);
                                let field_reg = self.alloc_tmp()?;
                                let field_const =
                                    self.bytecode.add_constant(Value::String(field.clone()));
                                self.emit(Opcode::Const);
                                self.emit_u8(field_reg);
                                self.emit_u32(field_const);
                                self.emit(Opcode::Set);
                                self.emit_u8(obj_reg);
                                self.emit_u8(field_reg);
                                self.emit_u8(val_reg);
                                self.emit(Opcode::Set);
                                self.emit_u8(arr_reg);
                                self.emit_u8(idx_reg);
                                self.emit_u8(obj_reg);
                            } else {
                                // Latent array of structs: LatentGet → Get element → modify → Set element → LatentSet
                                let arr_name_idx = self.get_or_add_string(arr_name) as u32;
                                let arr_reg = self.alloc_tmp()?;
                                self.emit(Opcode::LatentGet);
                                self.emit_u8(arr_reg);
                                self.emit_u32(arr_name_idx);
                                let obj_reg = self.alloc_tmp()?;
                                self.emit(Opcode::Get);
                                self.emit_u8(obj_reg);
                                self.emit_u8(arr_reg);
                                self.emit_u8(idx_reg);
                                let field_reg = self.alloc_tmp()?;
                                let field_const =
                                    self.bytecode.add_constant(Value::String(field.clone()));
                                self.emit(Opcode::Const);
                                self.emit_u8(field_reg);
                                self.emit_u32(field_const);
                                self.emit(Opcode::Set);
                                self.emit_u8(obj_reg);
                                self.emit_u8(field_reg);
                                self.emit_u8(val_reg);
                                self.emit(Opcode::Set);
                                self.emit_u8(arr_reg);
                                self.emit_u8(idx_reg);
                                self.emit_u8(obj_reg);
                                self.emit(Opcode::LatentSet);
                                self.emit_u32(arr_name_idx);
                                self.emit_u8(arr_reg);
                            }
                        }
                    } else {
                        // Generic complex object — evaluate value first, then object, then field.
                        // NOTE: no writeback — for simple cases without compound objects.
                        let val_reg = self.alloc_tmp()?;
                        self.lower_expr(value, val_reg)?;
                        let obj_reg = self.alloc_tmp()?;
                        self.lower_expr(object, obj_reg)?;
                        let field_reg = self.alloc_tmp()?;
                        let field_const = self.bytecode.add_constant(Value::String(field.clone()));
                        self.emit(Opcode::Const);
                        self.emit_u8(field_reg);
                        self.emit_u32(field_const);
                        self.emit(Opcode::Set);
                        self.emit_u8(obj_reg);
                        self.emit_u8(field_reg);
                        self.emit_u8(val_reg);
                    }
                } else {
                    // Unknown assignment target — evaluate value to temp (no-op)
                    let tmp = self.alloc_tmp()?;
                    self.lower_expr(value, tmp)?;
                }
            }
            StmtKind::CompoundAssign { target, op, value } => {
                if let ExprKind::Identifier(name) = &target.kind {
                    if let Some(&reg) = self.variables.get(name.as_str()) {
                        // Local variable
                        let tmp = self.alloc_tmp()?;
                        self.lower_expr(value, tmp)?;
                        self.emit_binop(*op, reg, reg, tmp);
                    } else {
                        // Latent variable — read, modify, write back
                        let reg = self.alloc_tmp()?;
                        let name_idx = self.get_or_add_string(name) as u32;
                        self.emit(Opcode::LatentGet);
                        self.emit_u8(reg);
                        self.emit_u32(name_idx);
                        let tmp = self.alloc_tmp()?;
                        self.lower_expr(value, tmp)?;
                        self.emit_binop(*op, reg, reg, tmp);
                        self.emit(Opcode::LatentSet);
                        self.emit_u32(name_idx);
                        self.emit_u8(reg);
                    }
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
                    // Wildcard/Identifier patterns always match — skip comparison entirely
                    let is_wildcard = matches!(
                        case.pattern,
                        crate::ast::Pattern::Wildcard | crate::ast::Pattern::Identifier(_)
                    );

                    let skip_pos = if is_wildcard {
                        // No pattern comparison needed — fall through to body
                        // (guard still applies if present)
                        None
                    } else {
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
                            crate::ast::Pattern::Range { start, end, inclusive } => {
                                // r10 = (r11 >= start) && (r11 <= end or r11 < end)
                                let start_idx = self.bytecode.add_constant(Value::I64(start));
                                self.emit(Opcode::Const); self.emit_u8(12); self.emit_u32(start_idx);
                                self.emit(Opcode::Ge); self.emit_u8(10); self.emit_u8(11); self.emit_u8(12);
                                let end_val = if inclusive { end } else { end - 1 };
                                let end_idx = self.bytecode.add_constant(Value::I64(end_val));
                                self.emit(Opcode::Const); self.emit_u8(12); self.emit_u32(end_idx);
                                self.emit(Opcode::Le); self.emit_u8(12); self.emit_u8(11); self.emit_u8(12);
                                self.emit(Opcode::And); self.emit_u8(10); self.emit_u8(10); self.emit_u8(12);
                                // skip_pos handled below via JumpIfNot on r10
                                self.emit(Opcode::JumpIfNot);
                                self.emit_u8(10);
                                let pos = self.current_pc();
                                self.emit_u32(0);
                                // Handle guard then body
                                if let Some(ref guard_expr) = case.guard {
                                    self.lower_expr(guard_expr, 10)?;
                                    self.emit(Opcode::JumpIfNot);
                                    self.emit_u8(10);
                                    let guard_skip = self.current_pc();
                                    self.emit_u32(0);
                                    self.lower_body(&case.body)?;
                                    self.emit(Opcode::Jump);
                                    end_patches.push(self.current_pc());
                                    self.emit_u32(0);
                                    let next_case = self.current_pc();
                                    self.patch_jump(guard_skip, next_case)?;
                                } else {
                                    self.lower_body(&case.body)?;
                                    self.emit(Opcode::Jump);
                                    end_patches.push(self.current_pc());
                                    self.emit_u32(0);
                                }
                                let next_case = self.current_pc();
                                self.patch_jump(pos, next_case)?;
                                continue;
                            }
                            _ => {
                                // Unknown pattern variant — treat as wildcard
                                None::<()>;
                                // Fall through to body without comparison
                                self.lower_body(&case.body)?;
                                self.emit(Opcode::Jump);
                                end_patches.push(self.current_pc());
                                self.emit_u32(0);
                                continue;
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
                        let pos = self.current_pc();
                        self.emit_u32(0);
                        Some(pos)
                    };

                    // Guard expression (if any): skip body if guard is false
                    if let Some(ref guard_expr) = case.guard {
                        self.lower_expr(guard_expr, 10)?;
                        self.emit(Opcode::JumpIfNot);
                        self.emit_u8(10);
                        let guard_skip = self.current_pc();
                        self.emit_u32(0);
                        self.lower_body(&case.body)?;
                        self.emit(Opcode::Jump);
                        end_patches.push(self.current_pc());
                        self.emit_u32(0);
                        let next_case = self.current_pc();
                        self.patch_jump(guard_skip, next_case)?;
                        if let Some(pos) = skip_pos {
                            self.patch_jump(pos, next_case)?;
                        }
                    } else {
                        self.lower_body(&case.body)?;
                        self.emit(Opcode::Jump);
                        end_patches.push(self.current_pc());
                        self.emit_u32(0);
                        let next_case = self.current_pc();
                        if let Some(pos) = skip_pos {
                            self.patch_jump(pos, next_case)?;
                        }
                    }
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
                // CRITICAL FIX: Use high reserved registers (240-243) for loop state
                // These won't be touched by alloc_var (0-199) or alloc_tmp (200-229)
                const FOR_I_REG: u8 = 240;
                const FOR_LEN_REG: u8 = 241;
                const FOR_ITEM_REG: u8 = 242;
                const FOR_COLL_REG: u8 = 243;

                // Initialize i = 0
                let zero_idx = self.bytecode.add_constant(Value::I64(0));
                self.emit(Opcode::Const);
                self.emit_u8(FOR_I_REG);
                self.emit_u32(zero_idx);

                // Get collection
                self.lower_expr(iterable, FOR_COLL_REG)?;

                // len = collection.len()
                self.emit(Opcode::Len);
                self.emit_u8(FOR_LEN_REG);
                self.emit_u8(FOR_COLL_REG);

                // loop_start label
                let loop_start = self.current_pc();

                // Check if i >= len
                let cmp_reg = self.alloc_tmp()?;
                self.emit(Opcode::Ge);
                self.emit_u8(cmp_reg);
                self.emit_u8(FOR_I_REG);
                self.emit_u8(FOR_LEN_REG);

                // Exit if done
                self.emit(Opcode::JumpIf);
                self.emit_u8(cmp_reg);
                let exit_jump = self.current_pc();
                self.emit_u32(0);

                // Get item = collection[i]
                self.emit(Opcode::Get);
                self.emit_u8(FOR_ITEM_REG);
                self.emit_u8(FOR_COLL_REG);
                self.emit_u8(FOR_I_REG);

                // Bind the pattern variable to FOR_ITEM_REG
                if let Pattern::Identifier(name) = pattern {
                    self.variables.insert(name.clone(), FOR_ITEM_REG);
                }

                // Lower the body
                self.lower_body(body)?;

                // Increment i
                let one_idx = self.bytecode.add_constant(Value::I64(1));
                let one_reg = self.alloc_tmp()?;
                self.emit(Opcode::Const);
                self.emit_u8(one_reg);
                self.emit_u32(one_idx);

                self.emit(Opcode::Add);
                self.emit_u8(FOR_I_REG);
                self.emit_u8(FOR_I_REG);
                self.emit_u8(one_reg);

                // Jump back
                self.emit(Opcode::Jump);
                self.emit_u32(loop_start as u32);

                // Patch exit
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
                    // Not a local variable — try latent state (module-level variable)
                    let name_idx = self.get_or_add_string(name) as u32;
                    self.emit(Opcode::LatentGet);
                    self.emit_u8(dst);
                    self.emit_u32(name_idx);
                }
            }
            ExprKind::BinaryOp { op, left, right } => {
                match op {
                    BinaryOp::And => {
                        // Short-circuit: if left is false, skip right (dst retains falsy value)
                        self.lower_expr(left, dst)?;
                        self.emit(Opcode::JumpIfNot);
                        self.emit_u8(dst);
                        let skip_pos = self.current_pc();
                        self.emit_u32(0);
                        self.lower_expr(right, dst)?;
                        let end_pc = self.current_pc();
                        self.patch_jump(skip_pos, end_pc)?;
                    }
                    BinaryOp::Or => {
                        // Short-circuit: if left is true, skip right (dst retains truthy value)
                        self.lower_expr(left, dst)?;
                        self.emit(Opcode::JumpIf);
                        self.emit_u8(dst);
                        let skip_pos = self.current_pc();
                        self.emit_u32(0);
                        self.lower_expr(right, dst)?;
                        let end_pc = self.current_pc();
                        self.patch_jump(skip_pos, end_pc)?;
                    }
                    _ => {
                        self.lower_expr(left, dst)?;
                        let rhs = self.alloc_tmp()?;
                        self.lower_expr(right, rhs)?;
                        self.emit_binop(*op, dst, dst, rhs);
                    }
                }
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
                // DEBT-007: Fix concat() register collision
                // Use fresh temp registers for each argument instead of hardcoded 150+
                // This prevents nested calls from overwriting outer call arguments
                let mut arg_regs = Vec::with_capacity(arguments.len());
                for arg in arguments.iter() {
                    let reg = self.alloc_tmp()?;
                    self.lower_expr(arg, reg)?;
                    arg_regs.push(reg);
                }

                // Emit Move instructions to pack args into consecutive registers (150, 151, ...)
                // VM expects arguments in consecutive registers starting at 150
                for (i, &reg) in arg_regs.iter().enumerate() {
                    if reg != 150 + i as u8 {
                        self.emit(Opcode::Move);
                        self.emit_u8(150 + i as u8);
                        self.emit_u8(reg);
                    }
                }

                // If the function name resolves to a local variable (not a top-level
                // function definition), it may hold a Value::Function — use CallDyn.
                if self.variables.contains_key(function.as_str())
                    && !self.functions.contains_key(function.as_str())
                {
                    let var_reg = self.variables[function.as_str()];
                    self.emit(Opcode::CallDyn);
                    self.emit_u8(var_reg);
                    self.emit_u8(arguments.len() as u8);
                    self.emit_u8(dst);
                } else {
                    self.emit(Opcode::Call);
                    let name_idx = self.get_or_add_string(function);
                    self.emit_u32(name_idx);
                    self.emit_u8(arguments.len() as u8);
                    self.emit_u8(dst);

                    if !self.functions.contains_key(function) {
                        let call_site = self.current_pc() - 6;
                        self.patch_points.push((call_site, function.clone()));
                    }
                }
            }
            ExprKind::Array(elements) => {
                // Initialize dst with an empty array, then push each element.
                // Push opcode: dst(u8) = array reg, val(u8) = element reg.
                let empty_idx = self.bytecode.add_constant(Value::Array(Vec::new()));
                self.emit(Opcode::Const);
                self.emit_u8(dst);
                self.emit_u32(empty_idx);
                for elem in elements.iter() {
                    let elem_reg = self.alloc_tmp()?;
                    self.lower_expr(elem, elem_reg)?;
                    self.emit(Opcode::Push);
                    self.emit_u8(dst);
                    self.emit_u8(elem_reg);
                }
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
            // Ternary conditional: cond ? then : else
            ExprKind::Conditional {
                condition,
                then_expr,
                else_expr,
            } => {
                // Evaluate condition
                self.lower_expr(condition, 10)?;

                // Jump to else if condition is false
                self.emit(Opcode::JumpIfNot);
                self.emit_u8(10);
                let else_jump = self.current_pc();
                self.emit_u32(0);

                // Then branch - evaluate and store in dst
                self.lower_expr(then_expr, dst)?;

                // Jump over else
                self.emit(Opcode::Jump);
                let end_jump = self.current_pc();
                self.emit_u32(0);

                // Else branch
                let else_pc = self.current_pc();
                self.patch_jump(else_jump, else_pc)?;
                self.lower_expr(else_expr, dst)?;

                // End
                let end_pc = self.current_pc();
                self.patch_jump(end_jump, end_pc)?;
            }
            ExprKind::Lambda { parameters, body } => {
                // Compile the lambda body as a named anonymous function __lambda_N__
                let lambda_name = format!("__lambda_{}__", self.lambda_counter);
                self.lambda_counter += 1;

                // Jump over the lambda body so top-level execution skips it
                self.emit(Opcode::Jump);
                let skip_pos = self.current_pc();
                self.emit_u32(0);

                let start_pc = self.current_pc() as u32;
                let param_count = parameters.len() as u32;
                self.functions
                    .insert(lambda_name.clone(), (start_pc, param_count));

                // Build fake Parameter list from the lambda's plain-string param names
                let fake_params: Vec<Parameter> = parameters
                    .iter()
                    .map(|name| Parameter::new(name.clone()))
                    .collect();

                self.with_scope(|this| {
                    this.bind_params(&fake_params);
                    // Lambda body is a single expression — evaluate into reg 0 (return)
                    this.lower_expr(body, 0)?;
                    this.emit(Opcode::Return);
                    Ok(())
                })?;

                let end_pc = self.current_pc();
                self.patch_jump(skip_pos, end_pc)?;

                // Emit a Value::Function constant so the lambda is a first-class value
                let func_val_idx = self.bytecode.add_constant(Value::Function(lambda_name));
                self.emit(Opcode::Const);
                self.emit_u8(dst);
                self.emit_u32(func_val_idx);
            }
            // Type cast: expr as Type
            ExprKind::Cast {
                expr: cast_expr,
                target_type,
            } => {
                // Lower the expression first
                self.lower_expr(cast_expr, dst)?;

                // Emit appropriate conversion based on target type
                match target_type.as_str() {
                    "f64" => {
                        // Emit i64_to_f64 call
                        let func_idx = self.bytecode.add_string("i64_to_f64".to_string());
                        self.emit(Opcode::Call);
                        self.emit_u32(func_idx);
                        self.emit_u8(1); // 1 argument
                        self.emit_u8(dst);
                    }
                    "i64" => {
                        // Emit f64_to_i64 call
                        let func_idx = self.bytecode.add_string("f64_to_i64".to_string());
                        self.emit(Opcode::Call);
                        self.emit_u32(func_idx);
                        self.emit_u8(1); // 1 argument
                        self.emit_u8(dst);
                    }
                    _ => {
                        // Unknown cast, just keep the value as-is
                    }
                }
            }
            // Unsupported in bytecode — emit Nop placeholders
            ExprKind::Range { .. } | ExprKind::Contract { .. } => {
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

        // Lower modify block - Phase 4.2: Full RSI proposal flow
        if let Some(ref modify) = agent.modify {
            // Emit governance registration for the agent
            self.emit(Opcode::GovernRegister);

            // Set confidence threshold from gates
            let mut confidence_threshold: u8 = 80; // default 80%
            let mut cooldown: u8 = 5; // default 5 steps

            for gate in &modify.gates {
                match gate {
                    Gate::Proof { name, .. } => {
                        let gate_idx = self.get_or_add_string(name);
                        let _ = gate_idx;
                        // Parse confidence from proof gate if specified
                        if name.contains("confidence=") {
                            if let Some(start) = name.find("confidence=") {
                                if let Some(val) = name[start + 11..].split_whitespace().next() {
                                    if let Ok(conf) = val.parse::<f64>() {
                                        confidence_threshold = (conf * 100.0) as u8;
                                    }
                                }
                            }
                        }
                    }
                    Gate::Consensus { threshold, .. } => {
                        // Use consensus threshold if specified
                        confidence_threshold = (*threshold * 100.0) as u8;
                    }
                    Gate::Human { .. } => {}
                    Gate::SafetyCheck { name, passed, .. } => {
                        // Track safety check - name indicates the check type
                        let _ = name;
                        let _ = passed;
                    }
                }
            }

            // Set confidence threshold
            self.emit(Opcode::GovernSetConfidence);
            self.emit_u8(confidence_threshold);

            // Set cooldown steps
            cooldown = modify.cooldown_steps.max(1).min(255) as u8;
            let _ = cooldown;

            // Lower each modification proposal through full RSI flow
            for proposal in &modify.proposals {
                // Map ModificationKind to ModificationType (0-8)
                let mod_type_code: u8 = match proposal.kind {
                    ModificationKind::ParameterChange { .. } => 0,
                    ModificationKind::CycleChange { .. } => 1,
                    ModificationKind::AddBehavior { .. } => 2,
                    ModificationKind::RemoveBehavior { .. } => 3,
                    ModificationKind::ThresholdChange { .. } => 4,
                    ModificationKind::WeightUpdate { .. } => 5,
                    ModificationKind::RuleAdd { .. } => 6,
                    ModificationKind::RuleRemove { .. } => 7,
                    ModificationKind::RuleUpdate { .. } => 8,
                    _ => 0, // Default to ParameterChange
                };

                // Emit RSIPropose opcode
                // Format: RSIPropose dst mod_type confidence [...mod_data]
                self.emit(Opcode::RSIPropose);
                self.emit_u8(0); // dst register for proposal ID
                self.emit_u8(mod_type_code);
                self.emit_u8((proposal.confidence * 100.0) as u8);

                // Emit modification-specific data
                match &proposal.kind {
                    ModificationKind::ParameterChange {
                        name,
                        old_value,
                        new_value,
                    } => {
                        let name_idx = self.get_or_add_string(name);
                        self.emit_u32(name_idx);
                        self.emit_u8((*old_value * 100.0) as u8);
                        self.emit_u8((*new_value * 100.0) as u8);
                    }
                    ModificationKind::CycleChange { h_cycles, l_cycles } => {
                        self.emit_u8(*h_cycles as u8);
                        self.emit_u8(*l_cycles as u8);
                    }
                    ModificationKind::ThresholdChange {
                        name,
                        old_value,
                        new_value,
                    } => {
                        let name_idx = self.get_or_add_string(name);
                        self.emit_u32(name_idx);
                        self.emit_u8((*old_value * 100.0) as u8);
                        self.emit_u8((*new_value * 100.0) as u8);
                    }
                    ModificationKind::RuleAdd {
                        name,
                        description,
                        confidence: conf,
                    }
                    | ModificationKind::RuleUpdate {
                        name,
                        description,
                        confidence: conf,
                    } => {
                        let name_idx = self.get_or_add_string(name);
                        let desc_idx = self.get_or_add_string(description);
                        self.emit_u32(name_idx);
                        self.emit_u32(desc_idx);
                        self.emit_u8((*conf * 100.0) as u8);
                    }
                    ModificationKind::RuleRemove { name } => {
                        let name_idx = self.get_or_add_string(name);
                        self.emit_u32(name_idx);
                    }
                    ModificationKind::AddBehavior { pattern, response } => {
                        self.emit_u8(pattern.len().min(255) as u8);
                        for val in pattern.iter().take(255) {
                            self.emit_f64(*val);
                        }
                        self.emit_u8(response.len().min(255) as u8);
                        for val in response.iter().take(255) {
                            self.emit_f64(*val);
                        }
                    }
                    ModificationKind::RemoveBehavior { index } => {
                        self.emit_u32(*index as u32);
                    }
                    ModificationKind::WeightUpdate { layer, deltas } => {
                        self.emit_u32(*layer as u32);
                        self.emit_u8(deltas.len().min(255) as u8);
                        for val in deltas.iter().take(255) {
                            self.emit_f64(*val);
                        }
                    }
                    _ => {}
                }

                // Emit RSIValidate to check proposal against governance
                self.emit(Opcode::RSIValidate);
                self.emit_u8(1); // dst register for valid flag
                self.emit_u32(0); // proposal_id (will be patched)

                // Emit conditional: if valid, apply; else rollback
                // Note: In full implementation, this would branch on the valid flag
                // For now, we emit both paths and the VM will handle it

                // Emit RSIApply
                self.emit(Opcode::RSIApply);
                self.emit_u32(0); // proposal_id
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

    fn emit_f64(&mut self, v: f64) {
        // Emit f64 as 8 little-endian bytes
        self.bytecode.code.extend_from_slice(&v.to_le_bytes());
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
        self.next_var_reg = Self::LOCAL_VAR_BASE;
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

                    log::debug!(target: "lowerer", "[patch] {} -> CALL_ADDR PC {}", func_name, start_pc);
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
