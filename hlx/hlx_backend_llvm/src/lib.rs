//! HLX LLVM Backend (Iron)
//!
//! Compiles HLX IR (LC-B) to Native Machine Code via LLVM.

use hlx_core::{HlxCrate, Instruction, Value, Register};
use inkwell::context::Context;
use inkwell::builder::Builder;
use inkwell::module::{Module, Linkage};
use inkwell::values::{FunctionValue, BasicValueEnum, PointerValue};
use inkwell::basic_block::BasicBlock;
use inkwell::{IntPredicate, FloatPredicate};
use inkwell::types::BasicType;
use inkwell::OptimizationLevel;
use inkwell::targets::{Target, InitializationConfig, TargetMachine};
use inkwell::debug_info::{AsDIScope, DebugInfoBuilder, DICompileUnit, DIFile, DWARFSourceLanguage};
use std::collections::{HashMap, HashSet};
use anyhow::{Result, anyhow};

/// Runtime type of a value
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValueType {
    Int,
    Float,
    Pointer,
}

/// A basic block in the control flow graph
#[derive(Debug, Clone)]
struct CfgBlock {
    /// Program counter where this block starts
    #[allow(dead_code)] // Used for debugging and future optimizations
    start_pc: u32,
    /// Program counter where this block ends (inclusive)
    end_pc: u32,
    /// PCs of successor blocks (blocks that can follow this one)
    successors: Vec<u32>,
    /// PCs of predecessor blocks (blocks that can jump to this one)
    predecessors: Vec<u32>,
}

/// Control Flow Graph for a function
#[derive(Debug, Clone)]
struct ControlFlowGraph {
    /// Map from start_pc to CfgBlock
    blocks: HashMap<u32, CfgBlock>,
    /// Entry point PC
    entry_pc: u32,
}

impl ControlFlowGraph {
    /// Build a CFG from a list of instructions starting at start_pc
    fn build(start_pc: u32, instructions: &[Instruction]) -> Result<Self> {
        // STEP 1: Identify all block leaders
        let mut leaders = HashSet::new();

        // First instruction is always a leader
        leaders.insert(start_pc);

        let mut pc = start_pc as usize;
        while pc < instructions.len() {
            let inst = &instructions[pc];

            // Stop at next function definition
            if pc > start_pc as usize && matches!(inst, Instruction::FuncDef { .. }) {
                break;
            }

            // Targets of jumps are leaders
            match inst {
                Instruction::Jump { target } => {
                    leaders.insert(*target);
                    // Instruction after unconditional jump is also a leader (unreachable code boundary)
                    if pc + 1 < instructions.len() {
                        leaders.insert((pc + 1) as u32);
                    }
                }
                Instruction::If { then_block, else_block, .. } => {
                    leaders.insert(*then_block);
                    leaders.insert(*else_block);
                    // Instruction after If is also a leader (fallthrough from branches)
                    if pc + 1 < instructions.len() {
                        leaders.insert((pc + 1) as u32);
                    }
                }
                Instruction::Loop { body, exit, .. } => {
                    leaders.insert(*body);
                    leaders.insert(*exit);
                    // Instruction after Loop is also a leader
                    if pc + 1 < instructions.len() {
                        leaders.insert((pc + 1) as u32);
                    }
                }
                Instruction::Return { .. } => {
                    // Instruction after Return is a leader (unreachable code boundary)
                    if pc + 1 < instructions.len() {
                        leaders.insert((pc + 1) as u32);
                    }
                }
                _ => {}
            }

            pc += 1;
        }

        // STEP 2: Build basic blocks
        let mut sorted_leaders: Vec<u32> = leaders.into_iter().collect();
        sorted_leaders.sort();

        let mut blocks = HashMap::new();
        for i in 0..sorted_leaders.len() {
            let start = sorted_leaders[i];
            let end = if i + 1 < sorted_leaders.len() {
                sorted_leaders[i + 1] - 1
            } else {
                // Find end of this function
                let mut end_pc = start;
                while (end_pc as usize) < instructions.len() {
                    if end_pc > start && matches!(instructions[end_pc as usize], Instruction::FuncDef { .. }) {
                        break;
                    }
                    end_pc += 1;
                }
                end_pc - 1
            };

            blocks.insert(start, CfgBlock {
                start_pc: start,
                end_pc: end,
                successors: Vec::new(),
                predecessors: Vec::new(),
            });
        }

        // STEP 3: Connect blocks (build successor/predecessor links)
        // First, compute successors for each block
        let mut successors_map: HashMap<u32, Vec<u32>> = HashMap::new();

        for (&start_pc, block) in &blocks {
            let end_inst = &instructions[block.end_pc as usize];
            let mut succs = Vec::new();

            match end_inst {
                Instruction::Jump { target } => {
                    // Unconditional jump - single successor
                    succs.push(*target);
                }
                Instruction::If { then_block, else_block, .. } => {
                    // Conditional branch - two successors
                    succs.push(*then_block);
                    succs.push(*else_block);
                }
                Instruction::Loop { body, exit, .. } => {
                    // Loop - two successors (body and exit)
                    succs.push(*body);
                    succs.push(*exit);
                }
                Instruction::Return { .. } => {
                    // No successors
                }
                _ => {
                    // Fallthrough to next block
                    let next_pc = block.end_pc + 1;
                    if (next_pc as usize) < instructions.len() && blocks.contains_key(&next_pc) {
                        succs.push(next_pc);
                    }
                }
            }

            successors_map.insert(start_pc, succs);
        }

        // Now update blocks with computed successors
        for (start_pc, succs) in successors_map.iter() {
            if let Some(block) = blocks.get_mut(start_pc) {
                block.successors = succs.clone();
            }
        }

        // Build predecessor links
        let successor_map: HashMap<u32, Vec<u32>> = blocks.iter()
            .map(|(start, block)| (*start, block.successors.clone()))
            .collect();

        for (&start, successors) in &successor_map {
            for &succ in successors {
                if let Some(succ_block) = blocks.get_mut(&succ) {
                    succ_block.predecessors.push(start);
                }
            }
        }

        Ok(ControlFlowGraph {
            blocks,
            entry_pc: start_pc,
        })
    }

    /// Get all block leader PCs (for LLVM BasicBlock creation)
    fn block_leaders(&self) -> HashSet<u32> {
        self.blocks.keys().copied().collect()
    }

    /// Validate that all instructions are reachable from entry
    fn validate_reachability(&self) -> Result<()> {
        let mut reachable = HashSet::new();
        let mut worklist = vec![self.entry_pc];

        while let Some(pc) = worklist.pop() {
            if reachable.contains(&pc) {
                continue;
            }
            reachable.insert(pc);

            if let Some(block) = self.blocks.get(&pc) {
                for &succ in &block.successors {
                    worklist.push(succ);
                }
            }
        }

        // Check if any blocks are unreachable
        let unreachable: Vec<u32> = self.blocks.keys()
            .filter(|pc| !reachable.contains(pc))
            .copied()
            .collect();

        if !unreachable.is_empty() {
            eprintln!("WARNING: Unreachable blocks detected at PCs: {:?}", unreachable);
        }

        Ok(())
    }
}

pub struct CodeGen<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    functions: HashMap<String, FunctionValue<'ctx>>,
    reg_map: HashMap<Register, PointerValue<'ctx>>,
    reg_types: HashMap<Register, ValueType>,  // Track register types
    block_map: HashMap<u32, BasicBlock<'ctx>>,
    /// Track element types for array registers (supports recursion!)
    array_element_types: HashMap<Register, hlx_core::instruction::DType>,
    /// Global function signatures from crate metadata
    function_signatures: HashMap<String, Vec<hlx_core::instruction::DType>>,
    /// FFI export information from crate metadata
    ffi_exports: HashMap<String, hlx_core::hlx_crate::FfiExportInfo>,
    /// Debug info builder
    debug_builder: Option<DebugInfoBuilder<'ctx>>,
    /// Debug compile unit
    debug_compile_unit: Option<DICompileUnit<'ctx>>,
    /// Debug file
    debug_file: Option<DIFile<'ctx>>,
    /// Map from instruction index to (line, col) for debug locations
    debug_symbols: HashMap<usize, (u32, u32)>,
}

use std::env;

impl<'ctx> CodeGen<'ctx> {
    /// Helper: Get a function from the module by name
    pub fn get_function(&self, name: &str) -> Result<FunctionValue<'ctx>> {
        self.module.get_function(name)
            .ok_or_else(|| anyhow!("Function '{}' not found in module", name))
    }

    /// Helper: Get a function parameter by index
    pub fn get_param(&self, func: FunctionValue<'ctx>, index: u32, param_name: &str) -> Result<BasicValueEnum<'ctx>> {
        func.get_nth_param(index)
            .ok_or_else(|| anyhow!("Parameter {} (index {}) not found in function '{}'",
                param_name, index, func.get_name().to_string_lossy()))
    }

    /// Helper: Extract pointer value from call result
    pub fn call_result_to_ptr(&self, result: BasicValueEnum<'ctx>, _call_name: &str) -> Result<PointerValue<'ctx>> {
        result.into_pointer_value();
        Ok(result.into_pointer_value())
    }

    /// Helper: Get current insert block
    pub fn current_block(&self) -> Result<BasicBlock<'ctx>> {
        self.builder.get_insert_block()
            .ok_or_else(|| anyhow!("No current basic block in builder"))
    }

    /// Helper: Get a basic block by PC (for testing)
    pub fn get_block(&self, pc: u32) -> Result<BasicBlock<'ctx>> {
        self.block_map.get(&pc)
            .copied()
            .ok_or_else(|| anyhow!("Basic block for PC {} not found", pc))
    }

    /// Create a new code generator with default (host) target
    pub fn new(context: &'ctx Context, module_name: &str) -> Result<Self> {
        Self::with_target(context, module_name, None)
    }

    /// Create a new code generator with a specific target triple
    ///
    /// # Examples
    /// - `None` - Use host target (default)
    /// - `Some("x86_64-unknown-none-elf")` - Bare metal x86_64
    /// - `Some("aarch64-unknown-none")` - Bare metal ARM64
    /// - `Some("riscv64gc-unknown-none-elf")` - Bare metal RISC-V
    pub fn with_target(context: &'ctx Context, module_name: &str, target_triple: Option<&str>) -> Result<Self> {
        Target::initialize_native(&InitializationConfig::default())
            .map_err(|e| anyhow!("Failed to initialize LLVM target: {}", e))?;

        // Load symbols from current executable so JIT can find printf, malloc, etc.
        // (Only for hosted targets)
        let is_bare_metal = target_triple.map_or(false, |t| t.contains("none"));
        if !is_bare_metal {
            if let Ok(exe) = env::current_exe() {
                let _ = inkwell::support::load_library_permanently(&exe);
            }

            // Load SDL2 library for graphics support
            let _ = inkwell::support::load_library_permanently(std::path::Path::new("/usr/lib/libSDL2.so"));
        }

        let module = context.create_module(module_name);
        let builder = context.create_builder();

        // Use provided target triple or default to host
        let triple = if let Some(target_str) = target_triple {
            inkwell::targets::TargetTriple::create(target_str)
        } else {
            TargetMachine::get_default_triple()
        };

        let target = Target::from_triple(&triple)
            .map_err(|e| anyhow!("Failed to create LLVM target from triple '{}': {}", triple, e))?;

        // For bare metal targets, use generic CPU features
        let (cpu, features) = if is_bare_metal {
            ("generic".to_string(), "".to_string())
        } else {
            (TargetMachine::get_host_cpu_name().to_string(),
             TargetMachine::get_host_cpu_features().to_string())
        };

        let target_machine = target.create_target_machine(
            &triple,
            &cpu,
            &features,
            OptimizationLevel::Default,
            inkwell::targets::RelocMode::PIC, // Position Independent Code for PIE compatibility
            inkwell::targets::CodeModel::Default,
        ).ok_or_else(|| anyhow!("Failed to create LLVM target machine for triple '{}'", triple))?;

        module.set_data_layout(&target_machine.get_target_data().get_data_layout());
        module.set_triple(&triple);
        
        let i64_type = context.i64_type();
        let i32_type = context.i32_type();
        let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
        
        let mut functions = HashMap::new();

        // libc
        functions.insert("malloc".to_string(), module.add_function("malloc", ptr_type.fn_type(&[i64_type.into()], false), Some(Linkage::External)));
        functions.insert("free".to_string(), module.add_function("free", context.void_type().fn_type(&[ptr_type.into()], false), Some(Linkage::External)));
        functions.insert("printf".to_string(), module.add_function("printf", i32_type.fn_type(&[ptr_type.into()], true), Some(Linkage::External)));
        functions.insert("sprintf".to_string(), module.add_function("sprintf", i32_type.fn_type(&[ptr_type.into(), ptr_type.into()], true), Some(Linkage::External)));
        functions.insert("strlen".to_string(), module.add_function("strlen", i64_type.fn_type(&[ptr_type.into()], false), Some(Linkage::External)));
        functions.insert("strcpy".to_string(), module.add_function("strcpy", ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false), Some(Linkage::External)));
        functions.insert("strcat".to_string(), module.add_function("strcat", ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false), Some(Linkage::External)));
        functions.insert("strcmp".to_string(), module.add_function("strcmp", i32_type.fn_type(&[ptr_type.into(), ptr_type.into()], false), Some(Linkage::External)));
        functions.insert("fopen".to_string(), module.add_function("fopen", ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false), Some(Linkage::External)));
        functions.insert("fclose".to_string(), module.add_function("fclose", i32_type.fn_type(&[ptr_type.into()], false), Some(Linkage::External)));
        functions.insert("fread".to_string(), module.add_function("fread", i64_type.fn_type(&[ptr_type.into(), i64_type.into(), i64_type.into(), ptr_type.into()], false), Some(Linkage::External)));
        functions.insert("fwrite".to_string(), module.add_function("fwrite", i64_type.fn_type(&[ptr_type.into(), i64_type.into(), i64_type.into(), ptr_type.into()], false), Some(Linkage::External)));
        functions.insert("fseek".to_string(), module.add_function("fseek", i32_type.fn_type(&[ptr_type.into(), i64_type.into(), i32_type.into()], false), Some(Linkage::External)));
        functions.insert("ftell".to_string(), module.add_function("ftell", i64_type.fn_type(&[ptr_type.into()], false), Some(Linkage::External)));
        functions.insert("memcpy".to_string(), module.add_function("memcpy", ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), i64_type.into()], false), Some(Linkage::External)));
        functions.insert("atoi".to_string(), module.add_function("atoi", i32_type.fn_type(&[ptr_type.into()], false), Some(Linkage::External)));

        // Math (libm) - LLVM Intrinsics
        // These correspond to BackendImpl::LLVMIntrinsic in the unified BuiltinRegistry
        let f64_type = context.f64_type();
        functions.insert("sin".to_string(), module.add_function("sin", f64_type.fn_type(&[f64_type.into()], false), Some(Linkage::External)));
        functions.insert("cos".to_string(), module.add_function("cos", f64_type.fn_type(&[f64_type.into()], false), Some(Linkage::External)));
        functions.insert("tan".to_string(), module.add_function("tan", f64_type.fn_type(&[f64_type.into()], false), Some(Linkage::External)));
        functions.insert("sqrt".to_string(), module.add_function("sqrt", f64_type.fn_type(&[f64_type.into()], false), Some(Linkage::External)));
        functions.insert("floor".to_string(), module.add_function("floor", f64_type.fn_type(&[f64_type.into()], false), Some(Linkage::External)));
        functions.insert("ceil".to_string(), module.add_function("ceil", f64_type.fn_type(&[f64_type.into()], false), Some(Linkage::External)));
        functions.insert("round".to_string(), module.add_function("round", f64_type.fn_type(&[f64_type.into()], false), Some(Linkage::External)));
        functions.insert("log".to_string(), module.add_function("log", f64_type.fn_type(&[f64_type.into()], false), Some(Linkage::External)));
        functions.insert("exp".to_string(), module.add_function("exp", f64_type.fn_type(&[f64_type.into()], false), Some(Linkage::External)));
        functions.insert("pow".to_string(), module.add_function("pow", f64_type.fn_type(&[f64_type.into(), f64_type.into()], false), Some(Linkage::External)));
        functions.insert("abs".to_string(), module.add_function("fabs", f64_type.fn_type(&[f64_type.into()], false), Some(Linkage::External)));

        // SDL2
        let sdl_init = module.add_function("SDL_Init", i32_type.fn_type(&[i32_type.into()], false), Some(Linkage::External));
        let sdl_create_window = module.add_function("SDL_CreateWindow", ptr_type.fn_type(&[ptr_type.into(), i32_type.into(), i32_type.into(), i32_type.into(), i32_type.into(), i32_type.into()], false), Some(Linkage::External));
        let sdl_create_renderer = module.add_function("SDL_CreateRenderer", ptr_type.fn_type(&[ptr_type.into(), i32_type.into(), i32_type.into()], false), Some(Linkage::External));
        let sdl_set_color = module.add_function("SDL_SetRenderDrawColor", i32_type.fn_type(&[ptr_type.into(), context.i8_type().into(), context.i8_type().into(), context.i8_type().into(), context.i8_type().into()], false), Some(Linkage::External));
        let sdl_draw_line = module.add_function("SDL_RenderDrawLine", i32_type.fn_type(&[ptr_type.into(), i32_type.into(), i32_type.into(), i32_type.into(), i32_type.into()], false), Some(Linkage::External));
        let sdl_draw_point = module.add_function("SDL_RenderDrawPoint", i32_type.fn_type(&[ptr_type.into(), i32_type.into(), i32_type.into()], false), Some(Linkage::External));
        let sdl_clear = module.add_function("SDL_RenderClear", i32_type.fn_type(&[ptr_type.into()], false), Some(Linkage::External));
        let sdl_present = module.add_function("SDL_RenderPresent", context.void_type().fn_type(&[ptr_type.into()], false), Some(Linkage::External));
        let sdl_poll = module.add_function("SDL_PollEvent", i32_type.fn_type(&[ptr_type.into()], false), Some(Linkage::External));
        let sdl_delay = module.add_function("SDL_Delay", context.void_type().fn_type(&[i32_type.into()], false), Some(Linkage::External));
        let sdl_quit = module.add_function("SDL_Quit", context.void_type().fn_type(&[], false), Some(Linkage::External));
        let sdl_destroy_window = module.add_function("SDL_DestroyWindow", context.void_type().fn_type(&[ptr_type.into()], false), Some(Linkage::External));

        // Register both PascalCase and snake_case variants
        functions.insert("SDL_Init".to_string(), sdl_init);
        functions.insert("sdl_init".to_string(), sdl_init);
        functions.insert("SDL_CreateWindow".to_string(), sdl_create_window);
        functions.insert("sdl_create_window".to_string(), sdl_create_window);
        functions.insert("SDL_CreateRenderer".to_string(), sdl_create_renderer);
        functions.insert("sdl_create_renderer".to_string(), sdl_create_renderer);
        functions.insert("SDL_SetRenderDrawColor".to_string(), sdl_set_color);
        functions.insert("sdl_set_color".to_string(), sdl_set_color);
        functions.insert("SDL_RenderDrawLine".to_string(), sdl_draw_line);
        functions.insert("sdl_render_draw_line".to_string(), sdl_draw_line);
        functions.insert("SDL_RenderDrawPoint".to_string(), sdl_draw_point);
        functions.insert("sdl_render_draw_point".to_string(), sdl_draw_point);
        functions.insert("SDL_RenderClear".to_string(), sdl_clear);
        functions.insert("sdl_clear".to_string(), sdl_clear);
        functions.insert("SDL_RenderPresent".to_string(), sdl_present);
        functions.insert("sdl_present".to_string(), sdl_present);
        functions.insert("SDL_PollEvent".to_string(), sdl_poll);
        functions.insert("sdl_poll".to_string(), sdl_poll);
        functions.insert("SDL_Delay".to_string(), sdl_delay);
        functions.insert("sdl_delay".to_string(), sdl_delay);
        functions.insert("SDL_Quit".to_string(), sdl_quit);
        functions.insert("sdl_quit".to_string(), sdl_quit);
        functions.insert("SDL_DestroyWindow".to_string(), sdl_destroy_window);
        functions.insert("sdl_destroy_window".to_string(), sdl_destroy_window);

        // Create debug info builder
        let (debug_builder, debug_compile_unit) = module.create_debug_info_builder(
            true,  // allow_unresolved
            DWARFSourceLanguage::C,  // Use C as the closest match for HLX
            "unknown.hlxl",  // filename (will be updated when we compile a crate)
            ".",  // directory
            "hlx-compiler v0.1",  // producer
            false,  // is_optimized
            "",  // flags
            0,  // runtime_version
            "",  // split_name
            inkwell::debug_info::DWARFEmissionKind::Full,  // kind
            0,  // dwo_id
            false,  // split_debug_inlining
            false,  // debug_info_for_profiling
            "",  // sysroot
            "",  // sdk
        );

        let debug_file = debug_compile_unit.get_file();

        let mut codegen = Self {
            context,
            module,
            builder,
            functions,
            reg_map: HashMap::new(),
            reg_types: HashMap::new(),
            block_map: HashMap::new(),
            array_element_types: HashMap::new(),
            function_signatures: HashMap::new(),
            ffi_exports: HashMap::new(),
            debug_builder: Some(debug_builder),
            debug_compile_unit: Some(debug_compile_unit),
            debug_file: Some(debug_file),
            debug_symbols: HashMap::new(),
        };

        codegen.define_tensor_utils()?;

        let helpers = ["__hlx_tensor_create", "__hlx_matmul", "__hlx_tensor_add", "__hlx_tensor_transpose"];
        for h in helpers {
            let func = codegen.get_function(h)?;
            codegen.functions.insert(h.to_string(), func);
        }
        Ok(codegen)
    }
    
    fn define_tensor_utils(&mut self) -> Result<()> {
        let i64_type = self.context.i64_type();
        let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
        let malloc = self.get_function("malloc")?;

        let create_func = self.module.add_function("__hlx_tensor_create", ptr_type.fn_type(&[i64_type.into(), i64_type.into()], false), Some(Linkage::Internal));
        let bb = self.context.append_basic_block(create_func, "entry");
        self.builder.position_at_end(bb);
        let rows = self.get_param(create_func, 0, "rows")?.into_int_value();
        let cols = self.get_param(create_func, 1, "cols")?.into_int_value();
        let s_ptr_result = self.builder.build_call(malloc, &[i64_type.const_int(24, false).into()], "s_ptr")?
            .try_as_basic_value().left()
            .ok_or_else(|| anyhow!("malloc call for tensor struct returned void"))?;
        let s_ptr = self.call_result_to_ptr(s_ptr_result, "malloc(struct)")?;
        let count = self.builder.build_int_mul(rows, cols, "cnt")?;
        let d_ptr_result = self.builder.build_call(malloc, &[self.builder.build_int_mul(count, i64_type.const_int(8, false), "db")?.into()], "d_ptr")?
            .try_as_basic_value().left()
            .ok_or_else(|| anyhow!("malloc call for tensor data returned void"))?;
        let d_ptr = self.call_result_to_ptr(d_ptr_result, "malloc(data)")?;
        unsafe {
            self.builder.build_store(self.builder.build_gep(i64_type, s_ptr, &[i64_type.const_int(0, false)], "r")?, rows)?;
            self.builder.build_store(self.builder.build_gep(i64_type, s_ptr, &[i64_type.const_int(1, false)], "c")?, cols)?;
            self.builder.build_store(self.builder.build_gep(i64_type, s_ptr, &[i64_type.const_int(2, false)], "d")?, self.builder.build_ptr_to_int(d_ptr, i64_type, "di")?)?;
        }
        self.builder.build_return(Some(&s_ptr))?;
        
        let matmul_func = self.module.add_function("__hlx_matmul", ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false), Some(Linkage::Internal));
        let bb = self.context.append_basic_block(matmul_func, "entry");
        self.builder.position_at_end(bb);
        let a_ptr = self.get_param(matmul_func, 0, "a")?.into_pointer_value();
        let b_ptr = self.get_param(matmul_func, 1, "b")?.into_pointer_value();
        unsafe {
            let m = self.builder.build_load(i64_type, self.builder.build_gep(i64_type, a_ptr, &[i64_type.const_int(0, false)], "ar")?, "m")?.into_int_value();
            let k = self.builder.build_load(i64_type, self.builder.build_gep(i64_type, a_ptr, &[i64_type.const_int(1, false)], "ac")?, "k")?.into_int_value();
            let ad_i = self.builder.build_load(i64_type, self.builder.build_gep(i64_type, a_ptr, &[i64_type.const_int(2, false)], "ad")?, "adi")?.into_int_value();
            let ad = self.builder.build_int_to_ptr(ad_i, ptr_type, "ad")?;
            let n = self.builder.build_load(i64_type, self.builder.build_gep(i64_type, b_ptr, &[i64_type.const_int(1, false)], "bc")?, "n")?.into_int_value();
            let bd_i = self.builder.build_load(i64_type, self.builder.build_gep(i64_type, b_ptr, &[i64_type.const_int(2, false)], "bd")?, "bdi")?.into_int_value();
            let bd = self.builder.build_int_to_ptr(bd_i, ptr_type, "bd")?;
            let c_ptr_result = self.builder.build_call(create_func, &[m.into(), n.into()], "cp")?
                .try_as_basic_value().left()
                .ok_or_else(|| anyhow!("__hlx_tensor_create call returned void"))?;
            let c_ptr = self.call_result_to_ptr(c_ptr_result, "__hlx_tensor_create")?;
            let cd_i = self.builder.build_load(i64_type, self.builder.build_gep(i64_type, c_ptr, &[i64_type.const_int(2, false)], "cd")?, "cdi")?.into_int_value();
            let cd = self.builder.build_int_to_ptr(cd_i, ptr_type, "cd")?;

            let i_a = self.builder.build_alloca(i64_type, "i")?;
            self.builder.build_store(i_a, i64_type.const_int(0, false))?;
            let c_i = self.context.append_basic_block(matmul_func, "ci");
            let b_i = self.context.append_basic_block(matmul_func, "bi");
            let e_i = self.context.append_basic_block(matmul_func, "ei");
            self.builder.build_unconditional_branch(c_i)?;
            self.builder.position_at_end(c_i);
            let i_v = self.builder.build_load(i64_type, i_a, "iv")?.into_int_value();
            self.builder.build_conditional_branch(self.builder.build_int_compare(IntPredicate::SLT, i_v, m, "cmpi")?, b_i, e_i)?;
            self.builder.position_at_end(b_i);
            let j_a = self.builder.build_alloca(i64_type, "j")?;
            self.builder.build_store(j_a, i64_type.const_int(0, false))?;
            let c_j = self.context.append_basic_block(matmul_func, "cj");
            let b_j = self.context.append_basic_block(matmul_func, "bj");
            let e_j = self.context.append_basic_block(matmul_func, "ej");
            self.builder.build_unconditional_branch(c_j)?;
            self.builder.position_at_end(c_j);
            let j_v = self.builder.build_load(i64_type, j_a, "jv")?.into_int_value();
            self.builder.build_conditional_branch(self.builder.build_int_compare(IntPredicate::SLT, j_v, n, "cmpj")?, b_j, e_j)?;
            self.builder.position_at_end(b_j);
            let s_a = self.builder.build_alloca(i64_type, "s")?;
            self.builder.build_store(s_a, i64_type.const_int(0, false))?;
            let k_a = self.builder.build_alloca(i64_type, "k")?;
            self.builder.build_store(k_a, i64_type.const_int(0, false))?;
            let c_k = self.context.append_basic_block(matmul_func, "ck");
            let b_k = self.context.append_basic_block(matmul_func, "bk");
            let e_k = self.context.append_basic_block(matmul_func, "ek");
            self.builder.build_unconditional_branch(c_k)?;
            self.builder.position_at_end(c_k);
            let k_v = self.builder.build_load(i64_type, k_a, "kv")?.into_int_value();
            self.builder.build_conditional_branch(self.builder.build_int_compare(IntPredicate::SLT, k_v, k, "cmpk")?, b_k, e_k)?;
            self.builder.position_at_end(b_k);
            let av = self.builder.build_load(i64_type, self.builder.build_gep(i64_type, ad, &[self.builder.build_int_add(self.builder.build_int_mul(i_v, k, "ik")?, k_v, "aidx")?], "aep")?, "av")?.into_int_value();
            let bv = self.builder.build_load(i64_type, self.builder.build_gep(i64_type, bd, &[self.builder.build_int_add(self.builder.build_int_mul(k_v, n, "kn")?, j_v, "bidx")?], "bep")?, "bv")?.into_int_value();
            self.builder.build_store(s_a, self.builder.build_int_add(self.builder.build_load(i64_type, s_a, "sv")?.into_int_value(), self.builder.build_int_mul(av, bv, "pd")?, "ns")?)?;
            self.builder.build_store(k_a, self.builder.build_int_add(k_v, i64_type.const_int(1, false), "ki")?)?;
            self.builder.build_unconditional_branch(c_k)?;
            self.builder.position_at_end(e_k);
            self.builder.build_store(self.builder.build_gep(i64_type, cd, &[self.builder.build_int_add(self.builder.build_int_mul(i_v, n, "in")?, j_v, "cidx")?], "cep")?, self.builder.build_load(i64_type, s_a, "fs")?)?;
            self.builder.build_store(j_a, self.builder.build_int_add(j_v, i64_type.const_int(1, false), "ji")?)?;
            self.builder.build_unconditional_branch(c_j)?;
            self.builder.position_at_end(e_j);
            self.builder.build_store(i_a, self.builder.build_int_add(i_v, i64_type.const_int(1, false), "ii")?)?;
            self.builder.build_unconditional_branch(c_i)?;
            self.builder.position_at_end(e_i);
            self.builder.build_return(Some(&c_ptr))?;
        }

        let add_func = self.module.add_function("__hlx_tensor_add", ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false), Some(Linkage::Internal));
        let bb = self.context.append_basic_block(add_func, "entry");
        self.builder.position_at_end(bb);
        let a_ptr = self.get_param(add_func, 0, "a")?.into_pointer_value();
        let b_ptr = self.get_param(add_func, 1, "b")?.into_pointer_value();
        unsafe {
            let r = self.builder.build_load(i64_type, self.builder.build_gep(i64_type, a_ptr, &[i64_type.const_int(0, false)], "r")?, "rows")?.into_int_value();
            let c = self.builder.build_load(i64_type, self.builder.build_gep(i64_type, a_ptr, &[i64_type.const_int(1, false)], "c")?, "cols")?.into_int_value();
            let ad = self.builder.build_int_to_ptr(self.builder.build_load(i64_type, self.builder.build_gep(i64_type, a_ptr, &[i64_type.const_int(2, false)], "ad")?, "adi")?.into_int_value(), ptr_type, "ad")?;
            let bd = self.builder.build_int_to_ptr(self.builder.build_load(i64_type, self.builder.build_gep(i64_type, b_ptr, &[i64_type.const_int(2, false)], "bd")?, "bdi")?.into_int_value(), ptr_type, "bd")?;
            let c_ptr_result = self.builder.build_call(create_func, &[r.into(), c.into()], "cp")?
                .try_as_basic_value().left()
                .ok_or_else(|| anyhow!("__hlx_tensor_create call in tensor_add returned void"))?;
            let c_ptr = self.call_result_to_ptr(c_ptr_result, "__hlx_tensor_create(add)")?;
            let cd = self.builder.build_int_to_ptr(self.builder.build_load(i64_type, self.builder.build_gep(i64_type, c_ptr, &[i64_type.const_int(2, false)], "cd")?, "cdi")?.into_int_value(), ptr_type, "cd")?;
            let tot = self.builder.build_int_mul(r, c, "tot")?;
            let i_a = self.builder.build_alloca(i64_type, "i")?;
            self.builder.build_store(i_a, i64_type.const_int(0, false))?;
            let cond_bb = self.context.append_basic_block(add_func, "c");
            let body_bb = self.context.append_basic_block(add_func, "b");
            let end_bb = self.context.append_basic_block(add_func, "e");
            self.builder.build_unconditional_branch(cond_bb)?;
            self.builder.position_at_end(cond_bb);
            let i_v = self.builder.build_load(i64_type, i_a, "iv")?.into_int_value();
            self.builder.build_conditional_branch(self.builder.build_int_compare(IntPredicate::SLT, i_v, tot, "cmp")?, body_bb, end_bb)?;
            self.builder.position_at_end(body_bb);
            self.builder.build_store(self.builder.build_gep(i64_type, cd, &[i_v], "cep")?, self.builder.build_int_add(self.builder.build_load(i64_type, self.builder.build_gep(i64_type, ad, &[i_v], "aep")?, "av")?.into_int_value(), self.builder.build_load(i64_type, self.builder.build_gep(i64_type, bd, &[i_v], "bep")?, "bv")?.into_int_value(), "s")?)?;
            self.builder.build_store(i_a, self.builder.build_int_add(i_v, i64_type.const_int(1, false), "ii")?)?;
            self.builder.build_unconditional_branch(cond_bb)?;
            self.builder.position_at_end(end_bb);
            self.builder.build_return(Some(&c_ptr))?;
        }

        let trans_func = self.module.add_function("__hlx_tensor_transpose", ptr_type.fn_type(&[ptr_type.into()], false), Some(Linkage::Internal));
        let bb = self.context.append_basic_block(trans_func, "entry");
        self.builder.position_at_end(bb);
        let a_ptr = self.get_param(trans_func, 0, "a")?.into_pointer_value();
        unsafe {
            let m = self.builder.build_load(i64_type, self.builder.build_gep(i64_type, a_ptr, &[i64_type.const_int(0, false)], "r")?, "m")?.into_int_value();
            let n = self.builder.build_load(i64_type, self.builder.build_gep(i64_type, a_ptr, &[i64_type.const_int(1, false)], "c")?, "n")?.into_int_value();
            let ad = self.builder.build_int_to_ptr(self.builder.build_load(i64_type, self.builder.build_gep(i64_type, a_ptr, &[i64_type.const_int(2, false)], "ad")?, "adi")?.into_int_value(), ptr_type, "ad")?;
            let c_ptr_result = self.builder.build_call(create_func, &[n.into(), m.into()], "cp")?
                .try_as_basic_value().left()
                .ok_or_else(|| anyhow!("__hlx_tensor_create call in tensor_transpose returned void"))?;
            let c_ptr = self.call_result_to_ptr(c_ptr_result, "__hlx_tensor_create(transpose)")?;
            let cd = self.builder.build_int_to_ptr(self.builder.build_load(i64_type, self.builder.build_gep(i64_type, c_ptr, &[i64_type.const_int(2, false)], "cd")?, "cdi")?.into_int_value(), ptr_type, "cd")?;
            let i_a = self.builder.build_alloca(i64_type, "i")?;
            self.builder.build_store(i_a, i64_type.const_int(0, false))?;
            let c_i = self.context.append_basic_block(trans_func, "ci");
            let b_i = self.context.append_basic_block(trans_func, "bi");
            let end_i = self.context.append_basic_block(trans_func, "ei");
            self.builder.build_unconditional_branch(c_i)?;
            self.builder.position_at_end(c_i);
            let i_v = self.builder.build_load(i64_type, i_a, "iv")?.into_int_value();
            self.builder.build_conditional_branch(self.builder.build_int_compare(IntPredicate::SLT, i_v, m, "cmpi")?, b_i, end_i)?;
            self.builder.position_at_end(b_i);
            let j_a = self.builder.build_alloca(i64_type, "j")?;
            self.builder.build_store(j_a, i64_type.const_int(0, false))?;
            let c_j = self.context.append_basic_block(trans_func, "cj");
            let b_j = self.context.append_basic_block(trans_func, "bj");
            let end_j = self.context.append_basic_block(trans_func, "ej");
            self.builder.build_unconditional_branch(c_j)?;
            self.builder.position_at_end(c_j);
            let j_v = self.builder.build_load(i64_type, j_a, "jv")?.into_int_value();
            self.builder.build_conditional_branch(self.builder.build_int_compare(IntPredicate::SLT, j_v, n, "cmpj")?, b_j, end_j)?;
            self.builder.position_at_end(b_j);
            let val = self.builder.build_load(i64_type, self.builder.build_gep(i64_type, ad, &[self.builder.build_int_add(self.builder.build_int_mul(i_v, n, "in")?, j_v, "sidx")?], "sp")?, "v")?.into_int_value();
            self.builder.build_store(self.builder.build_gep(i64_type, cd, &[self.builder.build_int_add(self.builder.build_int_mul(j_v, m, "jm")?, i_v, "didx")?], "dp")?, val)?;
            self.builder.build_store(j_a, self.builder.build_int_add(j_v, i64_type.const_int(1, false), "ji")?)?;
            self.builder.build_unconditional_branch(c_j)?;
            self.builder.position_at_end(end_j);
            self.builder.build_store(i_a, self.builder.build_int_add(i_v, i64_type.const_int(1, false), "ii")?)?;
            self.builder.build_unconditional_branch(c_i)?;
            self.builder.position_at_end(end_i);
            self.builder.build_return(Some(&c_ptr))?;
        }
        Ok(())
    }
    
    pub fn compile_crate(&mut self, krate: &HlxCrate) -> Result<()> {
        // 1. Build global signature map, FFI exports, and debug symbols from metadata
        if let Some(meta) = &krate.metadata {
            for (name, sig) in &meta.function_signatures {
                self.function_signatures.insert(name.clone(), sig.clone());
            }

            // Extract FFI export information
            for (name, info) in &meta.ffi_exports {
                self.ffi_exports.insert(name.clone(), info.clone());
            }

            // Extract debug symbols (instruction_idx → (line, col))
            for sym in &meta.debug_symbols {
                self.debug_symbols.insert(sym.inst_idx, (sym.line, sym.col));
            }

            // Update debug file with real source filename
            if let Some(source_file) = &meta.source_file {
                if let (Some(debug_builder), Some(_)) = (&self.debug_builder, &self.debug_compile_unit) {
                    self.debug_file = Some(debug_builder.create_file(source_file, "."));
                }
            }
        }

        // 2. PRE-SCAN: Declare all user functions in the LLVM module
        // This resolves the "Fn missing" error for forward-references and recursion.
        let i64_type = self.context.i64_type();
        let f64_type = self.context.f64_type();
        let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());

        for inst in &krate.instructions {
            if let Instruction::FuncDef { name, params, .. } = inst {
                let mut param_types = Vec::new();
                
                if let Some(sig) = self.function_signatures.get(name) {
                    for dtype in sig {
                        match dtype {
                            hlx_core::instruction::DType::F32 | hlx_core::instruction::DType::F64 => {
                                param_types.push(f64_type.into());
                            }
                            hlx_core::instruction::DType::Array(_) => {
                                param_types.push(ptr_type.into());
                            }
                            _ => {
                                param_types.push(i64_type.into());
                            }
                        }
                    }
                } else {
                    // Default to i64 if no signature (shouldn't happen with metadata)
                    for _ in params {
                        param_types.push(i64_type.into());
                    }
                }

                let fn_type = i64_type.fn_type(&param_types, false);

                // Check if this function should be exported with C linkage
                let linkage = if self.ffi_exports.contains_key(name) {
                    Some(Linkage::External)
                } else {
                    None
                };

                let func = self.module.add_function(name, fn_type, linkage);
                self.functions.insert(name.clone(), func);
            }
        }

        // 3. COMPILE: Build CFG and compile each function
        for inst in &krate.instructions {
            if let Instruction::FuncDef { name, params, body, .. } = inst {
                self.compile_function(name, params, *body, &krate.instructions)?;
            }
        }

        // 4. FINALIZE: Finalize debug info
        if let Some(debug_builder) = &self.debug_builder {
            debug_builder.finalize();
        }

        Ok(())
    }

    /// Get the base element type from a potentially nested DType
    fn get_base_dtype(dtype: &hlx_core::instruction::DType) -> hlx_core::instruction::DType {
        match dtype {
            hlx_core::instruction::DType::Array(inner) => Self::get_base_dtype(inner),
            other => other.clone(),
        }
    }

    /// Get the inner type of an array DType (one level of nesting)
    fn get_inner_dtype(dtype: &hlx_core::instruction::DType) -> Option<hlx_core::instruction::DType> {
        match dtype {
            hlx_core::instruction::DType::Array(inner) => Some((**inner).clone()),
            _ => None,
        }
    }

    /// Pre-populate array element types before type inference
    /// This ensures Index operations can correctly infer their output types
    fn populate_array_types(&mut self, name: &str, params: &[Register], start_pc: u32, instructions: &[Instruction]) {
        // First, populate from function signature parameters
        if let Some(sig) = self.function_signatures.get(name).cloned() {
            for (i, &reg) in params.iter().enumerate() {
                if let Some(dtype) = sig.get(i) {
                    if let hlx_core::instruction::DType::Array(inner) = dtype {
                        self.array_element_types.insert(reg, (**inner).clone());
                    }
                }
            }
        }

        // Then scan all instructions to build a complete type map and infer array types
        let mut register_values: HashMap<Register, hlx_core::value::Value> = HashMap::new();
        let mut pc = start_pc as usize;

        // First pass: collect constant values
        while pc < instructions.len() {
            let inst = &instructions[pc];
            if pc > start_pc as usize && matches!(inst, Instruction::FuncDef { .. }) {
                break;
            }

            match inst {
                Instruction::Constant { out, val } => {
                    register_values.insert(*out, val.clone());
                }
                Instruction::Neg { out, src } => {
                    // Propagate type through negation
                    if let Some(val) = register_values.get(src) {
                        let negated = match val {
                            hlx_core::value::Value::Integer(i) => hlx_core::value::Value::Integer(-i),
                            hlx_core::value::Value::Float(f) => hlx_core::value::Value::Float(-f),
                            _ => val.clone(),
                        };
                        register_values.insert(*out, negated);
                    }
                }
                _ => {}
            }

            pc += 1;
        }

        // Second pass: handle arrays and allocations
        pc = start_pc as usize;
        while pc < instructions.len() {
            let inst = &instructions[pc];
            if pc > start_pc as usize && matches!(inst, Instruction::FuncDef { .. }) {
                break;
            }

            match inst {
                Instruction::ArrayCreate { out, elements, element_type } => {
                    // If element_type is explicitly specified, use it
                    if let Some(dtype) = element_type {
                        self.array_element_types.insert(*out, dtype.clone());
                    } else if !elements.is_empty() {
                        // Infer element type from first element
                        if let Some(first_val) = register_values.get(&elements[0]) {
                            let inferred_dtype = match first_val {
                                hlx_core::value::Value::Float(_) => hlx_core::instruction::DType::F64,
                                hlx_core::value::Value::Integer(_) => hlx_core::instruction::DType::I64,
                                hlx_core::value::Value::Boolean(_) => hlx_core::instruction::DType::Bool,
                                _ => hlx_core::instruction::DType::I64, // default
                            };
                            self.array_element_types.insert(*out, inferred_dtype);
                        }
                    }
                }
                Instruction::ArrayAlloc { out, element_type, .. } => {
                    if let Some(dtype) = element_type {
                        self.array_element_types.insert(*out, dtype.clone());
                    }
                }
                _ => {}
            }

            pc += 1;
        }
    }

    /// Infer the LLVM type for each register based on how it's defined
    fn infer_register_types(&self, start_pc: u32, instructions: &[Instruction]) -> HashMap<Register, ValueType> {
        let mut types = HashMap::new();
        let mut pc = start_pc as usize;

        // First pass: Identify registers that are used as array containers (indexed into)
        let mut used_as_arrays = HashSet::new();
        let mut temp_pc = start_pc as usize;
        while temp_pc < instructions.len() {
            let inst = &instructions[temp_pc];
            if temp_pc > start_pc as usize && matches!(inst, Instruction::FuncDef { .. }) { break; }

            // If a register is used as container in Index, it's an array
            if let Instruction::Index { container, .. } = inst {
                used_as_arrays.insert(*container);
            }
            temp_pc += 1;
        }

        while pc < instructions.len() {
            let inst = &instructions[pc];
            if pc > start_pc as usize && matches!(inst, Instruction::FuncDef { .. }) { break; }

            // Determine output register type based on instruction
            if let Some(out_reg) = inst.output_register() {
                let reg_type = match inst {
                    // Float operations produce floats
                    Instruction::Constant { val, .. } => {
                        match val {
                            Value::Float(_) => ValueType::Float,
                            Value::String(_) => ValueType::Pointer,
                            _ => ValueType::Int,
                        }
                    }

                    Instruction::Add { lhs, rhs, .. } | Instruction::Sub { lhs, rhs, .. } |
                    Instruction::Mul { lhs, rhs, .. } | Instruction::Div { lhs, rhs, .. } |
                    Instruction::Mod { lhs, rhs, .. } => {
                        let l_type = types.get(lhs).cloned().unwrap_or(ValueType::Int);
                        let r_type = types.get(rhs).cloned().unwrap_or(ValueType::Int);
                        if l_type == ValueType::Float || r_type == ValueType::Float {
                            ValueType::Float
                        } else {
                            ValueType::Int
                        }
                    }

                    Instruction::Neg { src, .. } => {
                        types.get(src).cloned().unwrap_or(ValueType::Int)
                    }

                    // Math intrinsics that return floats
                    Instruction::Call { func, .. } if matches!(func.as_str(),
                        "sin" | "cos" | "tan" | "sqrt" | "floor" | "ceil" | "abs" | "pow" | "log" | "exp") => {
                        ValueType::Float
                    }

                    // Conversion intrinsics
                    Instruction::Call { func, .. } if func == "to_float" || func == "as_float" => ValueType::Float,
                    Instruction::Call { func, .. } if func == "to_int" || func == "as_int" => ValueType::Int,
                    Instruction::Call { func, .. } if func == "to_string" => ValueType::Pointer,

                    // Memory allocation returns pointers
                    Instruction::ArrayAlloc { .. } | Instruction::ArrayCreate { .. } |
                    Instruction::ObjectCreate { .. } | Instruction::TensorCreate { .. } => ValueType::Pointer,

                    Instruction::Call { func, .. } if func.contains("malloc") ||
                                                       func.contains("alloc") ||
                                                       func.contains("SDL_") => ValueType::Pointer,

                    // Move preserves type
                    Instruction::Move { src, .. } => {
                        types.get(src).cloned().unwrap_or(ValueType::Int)
                    }

                    // Index loads from typed arrays
                    Instruction::Index { container, out, .. } => {
                        // Check if we know this array's element type
                        if let Some(dtype) = self.array_element_types.get(container) {
                            match dtype {
                                hlx_core::instruction::DType::F32 | hlx_core::instruction::DType::F64 => ValueType::Float,
                                _ => {
                                    // If the output is used as an array later, keep it as Pointer
                                    if used_as_arrays.contains(out) {
                                        ValueType::Pointer
                                    } else {
                                        ValueType::Int
                                    }
                                }
                            }
                        } else {
                            // For untyped arrays, if the output is used as array, it's Pointer
                            if used_as_arrays.contains(out) {
                                ValueType::Pointer
                            } else {
                                ValueType::Int
                            }
                        }
                    }

                    // Comparison operators produce Int (boolean)
                    Instruction::Eq { .. } | Instruction::Ne { .. } |
                    Instruction::Lt { .. } | Instruction::Gt { .. } |
                    Instruction::Le { .. } | Instruction::Ge { .. } => {
                        ValueType::Int
                    }

                    // Default to int for everything else
                    _ => ValueType::Int,
                };

                types.insert(out_reg, reg_type);
            }

            pc += 1;
        }

        types
    }

    fn compile_function(&mut self, name: &str, params: &[Register], start_pc: u32, instructions: &[Instruction]) -> Result<()> {
        let function = *self.functions.get(name).ok_or_else(|| anyhow!("Fn missing: {}", name))?;
        self.reg_map.clear();
        self.reg_types.clear();
        self.block_map.clear();
        let entry_bb = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry_bb);

        // Create debug subprogram for this function
        if let (Some(debug_builder), Some(debug_file), Some(_debug_compile_unit)) =
            (&self.debug_builder, &self.debug_file, &self.debug_compile_unit) {
            // Get line number for function start (if available from debug symbols)
            let func_line = self.debug_symbols.get(&(start_pc as usize))
                .map(|(line, _)| *line)
                .unwrap_or(1);

            let di_type = debug_builder.create_subroutine_type(
                *debug_file,
                None,  // return_type - simplified
                &[],  // params - simplified for now
                0,  // flags
            );

            // Create DISubprogram for this function
            let di_subprogram = debug_builder.create_function(
                (*debug_file).as_debug_info_scope(),  // dereference then convert to scope
                name,
                None,  // linkage_name
                *debug_file,
                func_line,
                di_type,
                true,  // is_local_to_unit
                true,  // is_definition
                func_line,  // scope_line
                0,  // flags
                false,  // is_optimized
            );

            function.set_subprogram(di_subprogram);
        }

        // STEP 1: Build Control Flow Graph
        let cfg = ControlFlowGraph::build(start_pc, instructions)
            .map_err(|e| anyhow!("CFG construction failed for '{}': {}", name, e))?;

        // Validate CFG (warns about unreachable blocks)
        cfg.validate_reachability()?;

        // STEP 2: Pre-populate array element types before type inference
        // This fixes the bug where Index operations couldn't determine float array element types
        self.populate_array_types(name, params, start_pc, instructions);

        // STEP 3: Infer types for all registers
        let _register_types = self.infer_register_types(start_pc, instructions);

        // STEP 4: Collect used registers and create LLVM BasicBlocks for all block leaders
        let mut pc = start_pc as usize;
        let mut used_regs = HashSet::new();
        for &r in params { used_regs.insert(r); }

        let block_leaders = cfg.block_leaders();

        while pc < instructions.len() {
            let inst = &instructions[pc];
            if pc > start_pc as usize && matches!(inst, Instruction::FuncDef { .. }) { break; }
            if let Some(r) = inst.output_register() { used_regs.insert(r); }
            for &r in &inst.input_registers() { used_regs.insert(r); }

            // Create LLVM BasicBlock for each block leader
            if block_leaders.contains(&(pc as u32)) {
                let bb = self.context.append_basic_block(function, &format!("bb_{}", pc));
                self.block_map.insert(pc as u32, bb);
            }
            pc += 1;
        }

        // STEP 5: Allocate registers with their inferred types
        for &r in &used_regs {
            let ptr = self.builder.build_alloca(self.context.i64_type(), &format!("r{}", r))?;
            self.reg_map.insert(r, ptr);
        }

        // Initialize function parameters
        for (i, &reg) in params.iter().enumerate() {
            let val = self.get_param(function, i as u32, &format!("param_{}", i))?;

            // Determine ValueType from signature
            let v_type = if let Some(sig) = self.function_signatures.get(name) {
                if let Some(dtype) = sig.get(i) {
                    match dtype {
                        hlx_core::instruction::DType::F32 | hlx_core::instruction::DType::F64 => ValueType::Float,
                        hlx_core::instruction::DType::Array(_) => ValueType::Pointer,
                        _ => ValueType::Int,
                    }
                } else { ValueType::Int }
            } else { ValueType::Int };

            self.store_reg(reg, val, v_type)?;
        }
        let start_block = self.get_block(start_pc)?;
        self.builder.build_unconditional_branch(start_block)?;
        pc = start_pc as usize;
        while pc < instructions.len() {
            if let Some(bb) = self.builder.get_insert_block() {
                if bb.get_terminator().is_some() {
                    if let Some(new_bb) = self.block_map.get(&(pc as u32)) {
                        self.builder.position_at_end(*new_bb);
                    } else {
                        pc += 1;
                        if pc >= instructions.len() { break; }
                        let inst = &instructions[pc];
                        if matches!(inst, Instruction::FuncDef { .. }) { break; }
                        continue;
                    }
                }
            }
            let inst = &instructions[pc];
            if pc > start_pc as usize && matches!(inst, Instruction::FuncDef { .. }) { break; }
            if let Some(bb) = self.block_map.get(&(pc as u32)) {
                let current = self.current_block()?;
                if current != *bb {
                    if current.get_terminator().is_none() {
                        self.builder.build_unconditional_branch(*bb)?;
                    }
                    self.builder.position_at_end(*bb);
                }
            }

            // Set debug location for this instruction
            if let Some((line, col)) = self.debug_symbols.get(&pc) {
                if let (Some(debug_builder), Some(debug_file)) =
                    (&self.debug_builder, &self.debug_file) {
                    let loc = debug_builder.create_debug_location(
                        self.context,
                        *line,
                        *col,
                        (*debug_file).as_debug_info_scope(),  // dereference then convert
                        None,
                    );
                    self.builder.set_current_debug_location(loc);
                }
            }

            self.compile_inst(inst)?;
            pc += 1;
        }
        let current_block = self.current_block()?;
        if current_block.get_terminator().is_none() {
            self.builder.build_return(Some(&self.context.i64_type().const_int(0, false)))?;
        }
        Ok(())
    }
    
    fn compile_inst(&mut self, inst: &Instruction) -> Result<()> {
        // eprintln!("DEBUG: Compiling inst {:?}", inst);
        let i64_t = self.context.i64_type();
        let _ptr_t = self.context.ptr_type(inkwell::AddressSpace::default());
        match inst {
            Instruction::Constant { out, val } => {
                let (v, v_type) = self.compile_constant(val)?;
                self.store_reg(*out, v, v_type)?;
            }
            Instruction::Add { out, lhs, rhs } => {
                let (l, l_type) = self.load_reg(*lhs)?;
                let (r, r_type) = self.load_reg(*rhs)?;

                // Dispatch based on operand types
                let (res, res_type) = match (l_type, r_type) {
                    (ValueType::Int, ValueType::Int) => {
                        let res = self.builder.build_int_add(l.into_int_value(), r.into_int_value(), "add")?;
                        (res.into(), ValueType::Int)
                    }
                    (ValueType::Float, ValueType::Float) => {
                        let res = self.builder.build_float_add(l.into_float_value(), r.into_float_value(), "fadd")?;
                        (res.into(), ValueType::Float)
                    }
                    (ValueType::Pointer, ValueType::Pointer) => {
                        // String concatenation
                        let malloc = self.get_function("malloc")?;
                        let strlen = self.get_function("strlen")?;
                        let strcpy = self.get_function("strcpy")?;
                        let strcat = self.get_function("strcat")?;

                        let l_len_result = self.builder.build_call(strlen, &[l.into()], "l_len")?
                            .try_as_basic_value().left()
                            .ok_or_else(|| anyhow!("strlen call returned void"))?;
                        let l_len = l_len_result.into_int_value();

                        let r_len_result = self.builder.build_call(strlen, &[r.into()], "r_len")?
                            .try_as_basic_value().left()
                            .ok_or_else(|| anyhow!("strlen call returned void"))?;
                        let r_len = r_len_result.into_int_value();

                        let total_len = self.builder.build_int_add(l_len, r_len, "sum_len")?;
                        let total_len_plus_1 = self.builder.build_int_add(total_len, self.context.i64_type().const_int(1, false), "len_plus_1")?;

                        let new_str = self.builder.build_call(malloc, &[total_len_plus_1.into()], "new_str")?
                            .try_as_basic_value().left()
                            .ok_or_else(|| anyhow!("malloc call for string concatenation returned void"))?;

                        self.builder.build_call(strcpy, &[new_str.into(), l.into()], "copy_l")?;
                        self.builder.build_call(strcat, &[new_str.into(), r.into()], "cat_r")?;

                        (new_str, ValueType::Pointer)
                    }
                    _ => return Err(anyhow!("Type mismatch in Add: {:?} + {:?}", l_type, r_type)),
                };

                self.store_reg(*out, res, res_type)?;
            }
            Instruction::Sub { out, lhs, rhs } => {
                let (l, l_type) = self.load_reg(*lhs)?;
                let (r, r_type) = self.load_reg(*rhs)?;

                let (res, res_type) = match (l_type, r_type) {
                    (ValueType::Int, ValueType::Int) => {
                        let res = self.builder.build_int_sub(l.into_int_value(), r.into_int_value(), "sub")?;
                        (res.into(), ValueType::Int)
                    }
                    (ValueType::Float, ValueType::Float) => {
                        let res = self.builder.build_float_sub(l.into_float_value(), r.into_float_value(), "fsub")?;
                        (res.into(), ValueType::Float)
                    }
                    _ => return Err(anyhow!("Type mismatch in Sub: {:?} - {:?}", l_type, r_type)),
                };

                self.store_reg(*out, res, res_type)?;
            }
            Instruction::Mul { out, lhs, rhs } => {
                let (l, l_type) = self.load_reg(*lhs)?;
                let (r, r_type) = self.load_reg(*rhs)?;

                let (res, res_type) = match (l_type, r_type) {
                    (ValueType::Int, ValueType::Int) => {
                        let res = self.builder.build_int_mul(l.into_int_value(), r.into_int_value(), "mul")?;
                        (res.into(), ValueType::Int)
                    }
                    (ValueType::Float, ValueType::Float) => {
                        let res = self.builder.build_float_mul(l.into_float_value(), r.into_float_value(), "fmul")?;
                        (res.into(), ValueType::Float)
                    }
                    _ => return Err(anyhow!("Type mismatch in Mul: {:?} * {:?}", l_type, r_type)),
                };

                self.store_reg(*out, res, res_type)?;
            }
            Instruction::Div { out, lhs, rhs } => {
                let (l, l_type) = self.load_reg(*lhs)?;
                let (r, r_type) = self.load_reg(*rhs)?;

                let (res, res_type) = match (l_type, r_type) {
                    (ValueType::Int, ValueType::Int) => {
                        let res = self.builder.build_int_signed_div(l.into_int_value(), r.into_int_value(), "div")?;
                        (res.into(), ValueType::Int)
                    }
                    (ValueType::Float, ValueType::Float) => {
                        let res = self.builder.build_float_div(l.into_float_value(), r.into_float_value(), "fdiv")?;
                        (res.into(), ValueType::Float)
                    }
                    _ => return Err(anyhow!("Type mismatch in Div: {:?} / {:?}", l_type, r_type)),
                };

                self.store_reg(*out, res, res_type)?;
            }
            Instruction::Mod { out, lhs, rhs } => {
                let (l, l_type) = self.load_reg(*lhs)?;
                let (r, r_type) = self.load_reg(*rhs)?;

                let (res, res_type) = match (l_type, r_type) {
                    (ValueType::Int, ValueType::Int) => {
                        let res = self.builder.build_int_signed_rem(l.into_int_value(), r.into_int_value(), "mod")?;
                        (res.into(), ValueType::Int)
                    }
                    (ValueType::Float, ValueType::Float) => {
                        let res = self.builder.build_float_rem(l.into_float_value(), r.into_float_value(), "fmod")?;
                        (res.into(), ValueType::Float)
                    }
                    _ => return Err(anyhow!("Type mismatch in Mod: {:?} % {:?}", l_type, r_type)),
                };

                self.store_reg(*out, res, res_type)?;
            }
            Instruction::Neg { out, src } => {
                let (val, val_type) = self.load_reg(*src)?;

                let (res, res_type) = match val_type {
                    ValueType::Int => {
                        let res = self.builder.build_int_neg(val.into_int_value(), "neg")?;
                        (res.into(), ValueType::Int)
                    }
                    ValueType::Float => {
                        let res = self.builder.build_float_neg(val.into_float_value(), "fneg")?;
                        (res.into(), ValueType::Float)
                    }
                    ValueType::Pointer => return Err(anyhow!("Cannot negate pointer")),
                };

                self.store_reg(*out, res, res_type)?;
            }
            Instruction::Lt { out, lhs, rhs } => {
                let (l, l_type) = self.load_reg(*lhs)?;
                let (r, r_type) = self.load_reg(*rhs)?;

                let res = match (l_type, r_type) {
                    (ValueType::Int, ValueType::Int) => {
                        self.builder.build_int_compare(IntPredicate::SLT, l.into_int_value(), r.into_int_value(), "lt")?
                    }
                    (ValueType::Float, ValueType::Float) => {
                        self.builder.build_float_compare(FloatPredicate::OLT, l.into_float_value(), r.into_float_value(), "flt")?
                    }
                    (ValueType::Int, ValueType::Float) => {
                        let l_float = self.builder.build_signed_int_to_float(l.into_int_value(), self.context.f64_type(), "i2f")?;
                        self.builder.build_float_compare(FloatPredicate::OLT, l_float, r.into_float_value(), "flt_mix")?
                    }
                    (ValueType::Float, ValueType::Int) => {
                        let r_float = self.builder.build_signed_int_to_float(r.into_int_value(), self.context.f64_type(), "i2f")?;
                        self.builder.build_float_compare(FloatPredicate::OLT, l.into_float_value(), r_float, "flt_mix")?
                    }
                    _ => return Err(anyhow!("Type mismatch in Lt: {:?} < {:?}", l_type, r_type)),
                };

                let ext = self.builder.build_int_z_extend(res, self.context.i64_type(), "bool_ext")?;
                self.store_reg(*out, ext.into(), ValueType::Int)?;
            }
            Instruction::Gt { out, lhs, rhs } => {
                let (l, l_type) = self.load_reg(*lhs)?;
                let (r, r_type) = self.load_reg(*rhs)?;

                let res = match (l_type, r_type) {
                    (ValueType::Int, ValueType::Int) => {
                        self.builder.build_int_compare(IntPredicate::SGT, l.into_int_value(), r.into_int_value(), "gt")?
                    }
                    (ValueType::Float, ValueType::Float) => {
                        self.builder.build_float_compare(FloatPredicate::OGT, l.into_float_value(), r.into_float_value(), "fgt")?
                    }
                    (ValueType::Int, ValueType::Float) => {
                        let l_float = self.builder.build_signed_int_to_float(l.into_int_value(), self.context.f64_type(), "i2f")?;
                        self.builder.build_float_compare(FloatPredicate::OGT, l_float, r.into_float_value(), "fgt_mix")?
                    }
                    (ValueType::Float, ValueType::Int) => {
                        let r_float = self.builder.build_signed_int_to_float(r.into_int_value(), self.context.f64_type(), "i2f")?;
                        self.builder.build_float_compare(FloatPredicate::OGT, l.into_float_value(), r_float, "fgt_mix")?
                    }
                    _ => return Err(anyhow!("Type mismatch in Gt: {:?} > {:?}", l_type, r_type)),
                };

                let ext = self.builder.build_int_z_extend(res, self.context.i64_type(), "bool_ext")?;
                self.store_reg(*out, ext.into(), ValueType::Int)?;
            }
            Instruction::Eq { out, lhs, rhs } => {
                let (l, l_type) = self.load_reg(*lhs)?;
                let (r, r_type) = self.load_reg(*rhs)?;

                let res = match (l_type, r_type) {
                    (ValueType::Int, ValueType::Int) => {
                        self.builder.build_int_compare(IntPredicate::EQ, l.into_int_value(), r.into_int_value(), "eq")?
                    }
                    (ValueType::Float, ValueType::Float) => {
                        self.builder.build_float_compare(FloatPredicate::OEQ, l.into_float_value(), r.into_float_value(), "feq")?
                    }
                    (ValueType::Int, ValueType::Float) => {
                        let l_float = self.builder.build_signed_int_to_float(l.into_int_value(), self.context.f64_type(), "i2f")?;
                        self.builder.build_float_compare(FloatPredicate::OEQ, l_float, r.into_float_value(), "feq_mix")?
                    }
                    (ValueType::Float, ValueType::Int) => {
                        let r_float = self.builder.build_signed_int_to_float(r.into_int_value(), self.context.f64_type(), "i2f")?;
                        self.builder.build_float_compare(FloatPredicate::OEQ, l.into_float_value(), r_float, "feq_mix")?
                    }
                    _ => return Err(anyhow!("Type mismatch in Eq: {:?} == {:?}", l_type, r_type)),
                };

                let ext = self.builder.build_int_z_extend(res, self.context.i64_type(), "bool_ext")?;
                self.store_reg(*out, ext.into(), ValueType::Int)?;
            }
            Instruction::Ne { out, lhs, rhs } => {
                let (l, l_type) = self.load_reg(*lhs)?;
                let (r, r_type) = self.load_reg(*rhs)?;

                let res = match (l_type, r_type) {
                    (ValueType::Int, ValueType::Int) => {
                        self.builder.build_int_compare(IntPredicate::NE, l.into_int_value(), r.into_int_value(), "ne")?
                    }
                    (ValueType::Float, ValueType::Float) => {
                        self.builder.build_float_compare(FloatPredicate::ONE, l.into_float_value(), r.into_float_value(), "fne")?
                    }
                    (ValueType::Int, ValueType::Float) => {
                        let l_float = self.builder.build_signed_int_to_float(l.into_int_value(), self.context.f64_type(), "i2f")?;
                        self.builder.build_float_compare(FloatPredicate::ONE, l_float, r.into_float_value(), "fne_mix")?
                    }
                    (ValueType::Float, ValueType::Int) => {
                        let r_float = self.builder.build_signed_int_to_float(r.into_int_value(), self.context.f64_type(), "i2f")?;
                        self.builder.build_float_compare(FloatPredicate::ONE, l.into_float_value(), r_float, "fne_mix")?
                    }
                    _ => return Err(anyhow!("Type mismatch in Ne: {:?} != {:?}", l_type, r_type)),
                };

                let ext = self.builder.build_int_z_extend(res, self.context.i64_type(), "bool_ext")?;
                self.store_reg(*out, ext.into(), ValueType::Int)?;
            }
            Instruction::Le { out, lhs, rhs } => {
                let (l, l_type) = self.load_reg(*lhs)?;
                let (r, r_type) = self.load_reg(*rhs)?;

                let res = match (l_type, r_type) {
                    (ValueType::Int, ValueType::Int) => {
                        self.builder.build_int_compare(IntPredicate::SLE, l.into_int_value(), r.into_int_value(), "le")?
                    }
                    (ValueType::Float, ValueType::Float) => {
                        self.builder.build_float_compare(FloatPredicate::OLE, l.into_float_value(), r.into_float_value(), "fle")?
                    }
                    (ValueType::Int, ValueType::Float) => {
                        let l_float = self.builder.build_signed_int_to_float(l.into_int_value(), self.context.f64_type(), "i2f")?;
                        self.builder.build_float_compare(FloatPredicate::OLE, l_float, r.into_float_value(), "fle_mix")?
                    }
                    (ValueType::Float, ValueType::Int) => {
                        let r_float = self.builder.build_signed_int_to_float(r.into_int_value(), self.context.f64_type(), "i2f")?;
                        self.builder.build_float_compare(FloatPredicate::OLE, l.into_float_value(), r_float, "fle_mix")?
                    }
                    _ => return Err(anyhow!("Type mismatch in Le: {:?} (reg {}) <= {:?} (reg {})", l_type, lhs, r_type, rhs)),
                };

                let ext = self.builder.build_int_z_extend(res, self.context.i64_type(), "bool_ext")?;
                self.store_reg(*out, ext.into(), ValueType::Int)?;
            }
            Instruction::Ge { out, lhs, rhs } => {
                let (l, l_type) = self.load_reg(*lhs)?;
                let (r, r_type) = self.load_reg(*rhs)?;

                let res = match (l_type, r_type) {
                    (ValueType::Int, ValueType::Int) => {
                        self.builder.build_int_compare(IntPredicate::SGE, l.into_int_value(), r.into_int_value(), "ge")?
                    }
                    (ValueType::Float, ValueType::Float) => {
                        self.builder.build_float_compare(FloatPredicate::OGE, l.into_float_value(), r.into_float_value(), "fge")?
                    }
                    (ValueType::Int, ValueType::Float) => {
                        let l_float = self.builder.build_signed_int_to_float(l.into_int_value(), self.context.f64_type(), "i2f")?;
                        self.builder.build_float_compare(FloatPredicate::OGE, l_float, r.into_float_value(), "fge_mix")?
                    }
                    (ValueType::Float, ValueType::Int) => {
                        let r_float = self.builder.build_signed_int_to_float(r.into_int_value(), self.context.f64_type(), "i2f")?;
                        self.builder.build_float_compare(FloatPredicate::OGE, l.into_float_value(), r_float, "fge_mix")?
                    }
                    _ => return Err(anyhow!("Type mismatch in Ge: {:?} >= {:?}", l_type, r_type)),
                };

                let ext = self.builder.build_int_z_extend(res, self.context.i64_type(), "bool_ext")?;
                self.store_reg(*out, ext.into(), ValueType::Int)?;
            }
            Instruction::Call { out, func, args, max_depth: _ } => {
                // DEBUG: Check function name
                if func.contains("to_int") {
                     println!("DEBUG: Call func='{}' len={} out={:?}", func, func.len(), out);
                }

                // === Builtin Intrinsics ===
                //
                // All builtin functions are defined in hlx_core::BuiltinRegistry.
                // The registry categorizes builtins by their backend implementation:
                //   - CompilerSpecial: Handled inline below (len, str_get, type conversions)
                //   - LLVMIntrinsic: Math functions (sin, cos, sqrt, etc.) - handled via LLVM
                //   - RuntimeCall: I/O, JSON, HTTP, etc. - not yet implemented here
                //   - Math: Basic operations (min, max) - handled below
                //
                // TODO: Validate all CompilerSpecial and LLVMIntrinsic builtins have implementations

                // Type conversions (CompilerSpecial)
                if func == "to_int" {
                    let (val, _) = self.load_reg(args[0])?;
                    let res = if val.is_float_value() {
                        self.builder.build_float_to_signed_int(val.into_float_value(), self.context.i64_type(), "fptosi")?.into()
                    } else if val.is_pointer_value() {
                        self.builder.build_ptr_to_int(val.into_pointer_value(), self.context.i64_type(), "ptrtoint")?.into()
                    } else {
                        val
                    };
                    self.store_reg(*out, res, ValueType::Int)?;
                    return Ok(());
                }
                if func == "to_float" {
                    let (val, _) = self.load_reg(args[0])?;
                    let res = if val.is_int_value() {
                        self.builder.build_signed_int_to_float(val.into_int_value(), self.context.f64_type(), "sitofp")?.into()
                    } else {
                        val
                    };
                    self.store_reg(*out, res, ValueType::Float)?;
                    return Ok(());
                }
                if func == "as_float" {
                    let (val, _) = self.load_reg(args[0])?;
                    let res = if val.is_int_value() {
                        self.builder.build_bit_cast(val.into_int_value(), self.context.f64_type(), "i_to_f_bits")?.into()
                    } else {
                        val
                    };
                    self.store_reg(*out, res, ValueType::Float)?;
                    return Ok(());
                }
                if func == "as_int" {
                    let (val, _) = self.load_reg(args[0])?;
                    let res = if val.is_float_value() {
                        self.builder.build_bit_cast(val.into_float_value(), self.context.i64_type(), "f_to_i_bits")?.into()
                    } else {
                        val
                    };
                    self.store_reg(*out, res, ValueType::Int)?;
                    return Ok(());
                }
                if func == "str_get" {
                    let (s, _) = self.load_reg(args[0])?;
                    let (idx, _) = self.load_reg(args[1])?;
                    
                    let ptr = if s.is_pointer_value() {
                        s.into_pointer_value()
                    } else {
                        self.builder.build_int_to_ptr(s.into_int_value(), self.context.ptr_type(inkwell::AddressSpace::default()), "ptr_cast")?
                    };
                    
                    let idx_val = idx.into_int_value();
                    let byte_ptr = unsafe { self.builder.build_gep(self.context.i8_type(), ptr, &[idx_val], "byte_gep")? };
                    let byte_val = self.builder.build_load(self.context.i8_type(), byte_ptr, "byte_load")?.into_int_value();
                    let res = self.builder.build_int_z_extend(byte_val, self.context.i64_type(), "char_ext")?;
                    
                    self.store_reg(*out, res.into(), ValueType::Int)?;
                    return Ok(());
                }

                if func == "len" {
                    let (val, _val_type) = self.load_reg(args[0])?;
                    let ptr = if val.is_pointer_value() {
                        val.into_pointer_value()
                    } else {
                        self.builder.build_int_to_ptr(val.into_int_value(), self.context.ptr_type(inkwell::AddressSpace::default()), "ptr_cast")?
                    };
                    
                    let strlen = self.get_function("strlen")?;
                    let res = self.builder.build_call(strlen, &[ptr.into()], "len")?
                        .try_as_basic_value().left()
                        .ok_or_else(|| anyhow!("strlen returned void"))?;
                    self.store_reg(*out, res, ValueType::Int)?;
                    return Ok(());
                }

                if func == "slice" {
                    let (s, _) = self.load_reg(args[0])?;
                    let (start, _) = self.load_reg(args[1])?;
                    let (end, _) = self.load_reg(args[2])?;

                    let start_val = start.into_int_value();
                    let end_val = end.into_int_value();
                    let len_val = self.builder.build_int_sub(end_val, start_val, "slice_len")?;
                    
                    // malloc(len + 1)
                    let size_plus_1 = self.builder.build_int_add(len_val, self.context.i64_type().const_int(1, false), "size_p1")?;
                    let malloc = self.get_function("malloc")?;
                    let new_ptr_res = self.builder.build_call(malloc, &[size_plus_1.into()], "slice_malloc")?
                        .try_as_basic_value().left()
                        .ok_or_else(|| anyhow!("malloc returned void"))?;
                    let new_ptr = self.call_result_to_ptr(new_ptr_res, "malloc(slice)")?;

                    // src_ptr = s + start
                    let s_ptr = if s.is_pointer_value() { s.into_pointer_value() } else { self.builder.build_int_to_ptr(s.into_int_value(), self.context.ptr_type(inkwell::AddressSpace::default()), "s_ptr")? };
                    let src_ptr = unsafe { self.builder.build_gep(self.context.i8_type(), s_ptr, &[start_val], "src_offset")? };

                    // memcpy(dest, src, len)
                    let memcpy = self.get_function("memcpy")?;
                    self.builder.build_call(memcpy, &[new_ptr.into(), src_ptr.into(), len_val.into()], "memcpy_slice")?;

                    // null terminate: new_ptr[len] = 0
                    let end_ptr = unsafe { self.builder.build_gep(self.context.i8_type(), new_ptr, &[len_val], "end_ptr")? };
                    self.builder.build_store(end_ptr, self.context.i8_type().const_int(0, false))?;

                    self.store_reg(*out, new_ptr.into(), ValueType::Pointer)?;
                    return Ok(());
                }

                if func == "to_string" {
                    let (val, val_type) = self.load_reg(args[0])?;
                    let malloc = self.get_function("malloc")?;
                    let sprintf = self.get_function("sprintf")?;

                    // Allocate 32 bytes for the string
                    let buf_result = self.builder.build_call(malloc, &[self.context.i64_type().const_int(32, false).into()], "buf")?
                        .try_as_basic_value().left()
                        .ok_or_else(|| anyhow!("malloc call for to_string buffer returned void"))?;
                    let buf = self.call_result_to_ptr(buf_result, "malloc(to_string)")?;
                    
                    let fmt_str = match val_type {
                        ValueType::Float => "%f\0",
                        ValueType::Pointer => "%p\0",
                        _ => "%lld\0",
                    };
                    let fmt = self.context.const_string(fmt_str.as_bytes(), false);
                    let g = self.module.add_global(fmt.get_type(), Some(inkwell::AddressSpace::default()), "tsf");
                    g.set_initializer(&fmt); g.set_constant(true); g.set_linkage(Linkage::Internal);
                    let gep = unsafe { self.builder.build_gep(fmt.get_type(), g.as_pointer_value(), &[self.context.i32_type().const_int(0, false), self.context.i32_type().const_int(0, false)], "tsg")? };
                    
                    self.builder.build_call(sprintf, &[buf.into(), gep.into(), val.into()], "spr")?;
                    
                    self.store_reg(*out, buf.into(), ValueType::Pointer)?;
                    return Ok(());
                }
                if func == "chr" {
                    // chr(int) - Convert ASCII code to single-character string
                    let (val, _) = self.load_reg(args[0])?;
                    let malloc = self.get_function("malloc")?;

                    // Allocate 2 bytes (char + null terminator)
                    let buf_result = self.builder.build_call(malloc, &[self.context.i64_type().const_int(2, false).into()], "chr_buf")?
                        .try_as_basic_value().left()
                        .ok_or_else(|| anyhow!("malloc call for chr buffer returned void"))?;
                    let buf = self.call_result_to_ptr(buf_result, "malloc(chr)")?;

                    // Convert i64 to i8 (truncate to byte)
                    let int_val = if val.is_int_value() {
                        val.into_int_value()
                    } else {
                        self.builder.build_ptr_to_int(val.into_pointer_value(), self.context.i64_type(), "chr_ptrtoint")?
                    };
                    let char_val = self.builder.build_int_truncate(int_val, self.context.i8_type(), "chr_trunc")?;

                    // Store character at buf[0]
                    let char_ptr = unsafe {
                        self.builder.build_gep(self.context.i8_type(), buf, &[self.context.i32_type().const_int(0, false)], "chr_gep0")?
                    };
                    self.builder.build_store(char_ptr, char_val)?;

                    // Store null terminator at buf[1]
                    let null_ptr = unsafe {
                        self.builder.build_gep(self.context.i8_type(), buf, &[self.context.i32_type().const_int(1, false)], "chr_gep1")?
                    };
                    self.builder.build_store(null_ptr, self.context.i8_type().const_int(0, false))?;

                    self.store_reg(*out, buf.into(), ValueType::Pointer)?;
                    return Ok(());
                }

                let f = *self.functions.get(func).ok_or_else(|| anyhow!("Fn missing: {}", func))?;
                let mut llvm_args = Vec::new();
                let param_types = f.get_type().get_param_types();
                for (i, &a) in args.iter().enumerate() {
                    let (val, val_type) = self.load_reg(a)?;
                    let param_type = param_types.get(i).ok_or_else(|| anyhow!("Too many args"))?;

                    if param_type.is_pointer_type() {
                        let ptr = if val.is_pointer_value() {
                            val.into_pointer_value()
                        } else {
                            self.builder.build_int_to_ptr(val.into_int_value(), param_type.into_pointer_type(), "ptr_cast")?
                        };
                        llvm_args.push(ptr.into());
                    } else if param_type.is_int_type() {
                        let int_val = if val.is_int_value() {
                            val.into_int_value()
                        } else {
                            self.builder.build_ptr_to_int(val.into_pointer_value(), self.context.i64_type(), "int_cast")?
                        };
                        let target_width = param_type.into_int_type().get_bit_width();
                        if target_width < 64 {
                            let trunc = self.builder.build_int_truncate(int_val, param_type.into_int_type(), "int_trunc")?;
                            llvm_args.push(trunc.into());
                        } else {
                            llvm_args.push(int_val.into());
                        }
                    } else if param_type.is_float_type() {
                        // Handle float parameters (for math functions like sin, cos, etc.)
                        let float_val = if val.is_float_value() {
                            val.into_float_value()
                        } else if val_type == ValueType::Int {
                            // Convert int to float if needed
                            self.builder.build_signed_int_to_float(val.into_int_value(), self.context.f64_type(), "int_to_float")?
                        } else {
                            return Err(anyhow!("Cannot convert {:?} to float for call", val_type));
                        };
                        llvm_args.push(float_val.into());
                    } else {
                        llvm_args.push(val.into());
                    }
                }
                let call = self.builder.build_call(f, &llvm_args, "call_tmp")?;
                if let Some(res) = call.try_as_basic_value().left() {
                    // Detect return type
                    let res_type = if res.is_pointer_value() {
                        ValueType::Pointer
                    } else if res.is_float_value() {
                        ValueType::Float
                    } else {
                        ValueType::Int
                    };
                    self.store_reg(*out, res, res_type)?;
                }
            }
            Instruction::Return { val } => {
                let (v, _v_type) = self.load_reg(*val)?;
                let ret_val = if v.is_pointer_value() {
                    self.builder.build_ptr_to_int(v.into_pointer_value(), self.context.i64_type(), "ret_cast")?.into()
                } else if v.is_float_value() {
                    self.builder.build_bit_cast(v.into_float_value(), self.context.i64_type(), "ret_cast")?.into()
                } else {
                    v
                };
                self.builder.build_return(Some(&ret_val))?;
            }
            Instruction::Jump { target } => {
                let bb = self.get_block(*target)?;
                self.builder.build_unconditional_branch(bb)?;
            }
            Instruction::If { cond, then_block, else_block } => {
                let (c, _c_type) = self.load_reg(*cond)?;
                let c_int = if c.is_pointer_value() {
                    self.builder.build_ptr_to_int(c.into_pointer_value(), self.context.i64_type(), "ptr_check")?
                } else {
                    c.into_int_value()
                };
                let zero = self.context.i64_type().const_int(0, false);
                let is_nonzero = self.builder.build_int_compare(IntPredicate::NE, c_int, zero, "is_nonzero")?;
                let t = self.get_block(*then_block)?;
                let e = self.get_block(*else_block)?;
                self.builder.build_conditional_branch(is_nonzero, t, e)?;
            }
            Instruction::Move { out, src } => {
                let (v, v_type) = self.load_reg(*src)?;
                self.store_reg(*out, v, v_type)?;
            }
            Instruction::ArrayCreate { out, elements, element_type } => {
                // Track element type for later Index/Store operations (supports nesting!)
                if let Some(dtype) = element_type {
                    self.array_element_types.insert(*out, dtype.clone());
                }

                // Determine element size and LLVM type based on element_type
                let (elem_size, llvm_type) = if let Some(dtype) = element_type {
                    let base_dtype = Self::get_base_dtype(dtype);
                    match base_dtype {
                        hlx_core::instruction::DType::F32 => (4, self.context.f32_type().as_basic_type_enum()),
                        hlx_core::instruction::DType::F64 => (8, self.context.f64_type().as_basic_type_enum()),
                        hlx_core::instruction::DType::I32 => (4, self.context.i32_type().as_basic_type_enum()),
                        hlx_core::instruction::DType::I64 => (8, self.context.i64_type().as_basic_type_enum()),
                        hlx_core::instruction::DType::Bool => (1, self.context.bool_type().as_basic_type_enum()),
                        hlx_core::instruction::DType::Array(_) => (8, self.context.i64_type().as_basic_type_enum()), // Nested arrays are pointers
                    }
                } else {
                    // Untyped arrays fall back to i64 storage for backwards compatibility
                    (8, self.context.i64_type().as_basic_type_enum())
                };

                let malloc = self.get_function("malloc")?;
                let ptr_val = self.builder.build_call(
                    malloc,
                    &[i64_t.const_int(elements.len() as u64 * elem_size, false).into()],
                    "malloc_call"
                )?
                    .try_as_basic_value().left()
                    .ok_or_else(|| anyhow!("malloc call for ArrayCreate returned void"))?;
                let ptr = ptr_val.into_pointer_value();

                // Store elements with proper typing
                for (i, &reg) in elements.iter().enumerate() {
                    let (val, _v_type) = self.load_reg(reg)?;
                    let idx = self.context.i64_type().const_int(i as u64, false);

                    if element_type.is_some() {
                        // Typed array: store values directly with their proper type
                        unsafe {
                            let elem_ptr = self.builder.build_gep(llvm_type, ptr, &[idx], "gep")?;

                            // Type conversion if needed
                            let val_to_store = match llvm_type {
                                t if t.is_float_type() => {
                                    if val.is_float_value() {
                                        val
                                    } else if val.is_int_value() {
                                        // Convert int to float
                                        self.builder.build_signed_int_to_float(
                                            val.into_int_value(),
                                            llvm_type.into_float_type(),
                                            "int_to_float"
                                        )?.into()
                                    } else {
                                        val
                                    }
                                }
                                t if t.is_int_type() => {
                                    if val.is_int_value() {
                                        let int_val = val.into_int_value();
                                        let target_width = llvm_type.into_int_type().get_bit_width();
                                        let current_width = int_val.get_type().get_bit_width();

                                        if target_width < current_width {
                                            // Truncate if target is smaller
                                            self.builder.build_int_truncate(
                                                int_val,
                                                llvm_type.into_int_type(),
                                                "int_trunc"
                                            )?.into()
                                        } else if target_width > current_width {
                                            // Sign extend if target is larger
                                            self.builder.build_int_s_extend(
                                                int_val,
                                                llvm_type.into_int_type(),
                                                "int_sext"
                                            )?.into()
                                        } else {
                                            val
                                        }
                                    } else if val.is_float_value() {
                                        // Convert float to int
                                        self.builder.build_float_to_signed_int(
                                            val.into_float_value(),
                                            llvm_type.into_int_type(),
                                            "float_to_int"
                                        )?.into()
                                    } else if val.is_pointer_value() {
                                        // Pointer to int
                                        self.builder.build_ptr_to_int(
                                            val.into_pointer_value(),
                                            llvm_type.into_int_type(),
                                            "ptr_to_int"
                                        )?.into()
                                    } else {
                                        val
                                    }
                                }
                                _ => val,
                            };

                            self.builder.build_store(elem_ptr, val_to_store)?;
                        }
                    } else {
                        // Untyped array: bitcast to i64 for backwards compatibility
                        let val_to_store = if val.is_float_value() {
                            self.builder.build_bit_cast(val.into_float_value(), self.context.i64_type(), "f_to_i")?.into()
                        } else if val.is_pointer_value() {
                            self.builder.build_ptr_to_int(val.into_pointer_value(), self.context.i64_type(), "p_to_i")?.into()
                        } else {
                            val
                        };
                        unsafe {
                            let elem_ptr = self.builder.build_gep(self.context.i64_type(), ptr, &[idx], "gep")?;
                            self.builder.build_store(elem_ptr, val_to_store)?;
                        }
                    }
                }
                self.store_reg(*out, ptr_val, ValueType::Pointer)?;
            }
            Instruction::ArrayAlloc { out, size, element_type } => {
                let (size_val, _) = self.load_reg(*size)?;

                // Track element type for later Index/Store operations (supports nesting!)
                if let Some(dtype) = element_type {
                    self.array_element_types.insert(*out, dtype.clone());
                }

                // Determine element size based on base type
                let elem_size = if let Some(dtype) = element_type {
                    // For nested arrays, we store pointers (8 bytes)
                    // For primitives, we store the actual size
                    match Self::get_base_dtype(dtype) {
                        hlx_core::instruction::DType::F32 => 4,
                        hlx_core::instruction::DType::F64 => 8,
                        hlx_core::instruction::DType::I32 => 4,
                        hlx_core::instruction::DType::I64 => 8,
                        hlx_core::instruction::DType::Bool => 1,
                        hlx_core::instruction::DType::Array(_) => 8, // Nested arrays are pointers
                    }
                } else {
                    // Untyped arrays default to 8-byte elements (i64/pointer)
                    8
                };

                let bytes = self.builder.build_int_mul(
                    size_val.into_int_value(),
                    i64_t.const_int(elem_size, false),
                    "bytes"
                )?;
                let malloc = self.get_function("malloc")?;
                let ptr_val = self.builder.build_call(malloc, &[bytes.into()], "malloc_call")?
                    .try_as_basic_value().left()
                    .ok_or_else(|| anyhow!("malloc call for ArrayAlloc returned void"))?;
                self.store_reg(*out, ptr_val, ValueType::Pointer)?;
            }
            Instruction::Index { out, container, index } => {
                let (ptr_val, _) = self.load_reg(*container)?;
                let (idx_val, _) = self.load_reg(*index)?;
                let ptr = if ptr_val.is_pointer_value() {
                    ptr_val.into_pointer_value()
                } else {
                    self.builder.build_int_to_ptr(ptr_val.into_int_value(), self.context.ptr_type(inkwell::AddressSpace::default()), "int_to_ptr")? 
                };
                let idx = idx_val.into_int_value();

                // Check if we know the element type of this array
                if let Some(dtype) = self.array_element_types.get(container).cloned() {
                    // Check if this is a nested array (Array<Array<T>> or deeper)
                    if let Some(inner_dtype) = Self::get_inner_dtype(&dtype) {
                        // This is an array-of-arrays! Load pointer and propagate inner type
                        unsafe {
                            let elem_ptr = self.builder.build_gep(self.context.i64_type(), ptr, &[idx], "gep")?;
                            let val = self.builder.build_load(self.context.i64_type(), elem_ptr, "elem_load")?;
                            self.store_reg(*out, val, ValueType::Pointer)?;
                            // Propagate the inner array's element type (clean, no HashMap!)
                            self.array_element_types.insert(*out, inner_dtype);
                        }
                    } else {
                        // Regular typed array (primitives)
                        let (llvm_type, value_type) = match dtype {
                            hlx_core::instruction::DType::F32 => (self.context.f32_type().as_basic_type_enum(), ValueType::Float),
                            hlx_core::instruction::DType::F64 => (self.context.f64_type().as_basic_type_enum(), ValueType::Float),
                            hlx_core::instruction::DType::I32 => (self.context.i32_type().as_basic_type_enum(), ValueType::Int),
                            hlx_core::instruction::DType::I64 => (self.context.i64_type().as_basic_type_enum(), ValueType::Int),
                            hlx_core::instruction::DType::Bool => (self.context.bool_type().as_basic_type_enum(), ValueType::Int),
                            hlx_core::instruction::DType::Array(_) => unreachable!("Array case handled above"),
                        };
                        unsafe {
                            let elem_ptr = self.builder.build_gep(llvm_type, ptr, &[idx], "gep")?;
                            let val = self.builder.build_load(llvm_type, elem_ptr, "elem_load")?;
                            self.store_reg(*out, val, value_type)?;
                        }
                    }
                } else {
                    // Fall back to i64 for untyped arrays
                    unsafe {
                        let elem_ptr = self.builder.build_gep(self.context.i64_type(), ptr, &[idx], "gep")?;
                        let val = self.builder.build_load(self.context.i64_type(), elem_ptr, "elem_load")?;
                        self.store_reg(*out, val, ValueType::Int)?;
                    }
                }
            }
            Instruction::Store { container, index, value } => {
                let (ptr_val, _) = self.load_reg(*container)?;
                let (idx_val, _) = self.load_reg(*index)?;
                let (val, _) = self.load_reg(*value)?;
                let ptr = if ptr_val.is_pointer_value() {
                    ptr_val.into_pointer_value()
                } else {
                    self.builder.build_int_to_ptr(ptr_val.into_int_value(), self.context.ptr_type(inkwell::AddressSpace::default()), "int_to_ptr")? 
                };
                let idx = idx_val.into_int_value();

                // Check if we know the element type of this array
                if let Some(dtype) = self.array_element_types.get(container).cloned() {
                    // Handle nested arrays (store as pointers) vs primitives
                    if let hlx_core::instruction::DType::Array(_) = &dtype {
                        // Storing into array-of-arrays: store pointer as i64
                        let val_to_store = if val.is_pointer_value() {
                            self.builder.build_ptr_to_int(val.into_pointer_value(), self.context.i64_type(), "p_to_i")?.into()
                        } else {
                            val
                        };
                        unsafe {
                            let elem_ptr = self.builder.build_gep(self.context.i64_type(), ptr, &[idx], "gep")?;
                            self.builder.build_store(elem_ptr, val_to_store)?;
                        }
                    } else {
                        // Storing primitives: use typed store - NO BITCASTING!
                        let llvm_type = match dtype {
                            hlx_core::instruction::DType::F32 => self.context.f32_type().as_basic_type_enum(),
                            hlx_core::instruction::DType::F64 => self.context.f64_type().as_basic_type_enum(),
                            hlx_core::instruction::DType::I32 => self.context.i32_type().as_basic_type_enum(),
                            hlx_core::instruction::DType::I64 => self.context.i64_type().as_basic_type_enum(),
                            hlx_core::instruction::DType::Bool => self.context.bool_type().as_basic_type_enum(),
                            hlx_core::instruction::DType::Array(_) => unreachable!("Array case handled above"),
                        };
                        unsafe {
                            let elem_ptr = self.builder.build_gep(llvm_type, ptr, &[idx], "gep")?;
                            self.builder.build_store(elem_ptr, val)?;
                        }
                    }
                } else {
                    // Fall back to i64 storage with bitcasting for untyped arrays
                    let val_to_store = if val.is_float_value() {
                        self.builder.build_bit_cast(val.into_float_value(), self.context.i64_type(), "f_to_i")?.into()
                    } else if val.is_pointer_value() {
                        self.builder.build_ptr_to_int(val.into_pointer_value(), self.context.i64_type(), "p_to_i")?.into()
                    } else {
                        val
                    };
                    unsafe {
                        let elem_ptr = self.builder.build_gep(self.context.i64_type(), ptr, &[idx], "gep")?;
                        self.builder.build_store(elem_ptr, val_to_store)?;
                    }
                }
            }
            Instruction::TensorCreate { out, shape, .. } => {
                let create_func = self.get_function("__hlx_tensor_create")?;
                let ptr_val = self.builder.build_call(create_func, &[i64_t.const_int(shape[0] as u64, false).into(), i64_t.const_int(shape[1] as u64, false).into()], "ta")?
                    .try_as_basic_value().left()
                    .ok_or_else(|| anyhow!("__hlx_tensor_create call returned void"))?;
                self.store_reg(*out, ptr_val, ValueType::Pointer)?;
            }
            Instruction::MatMul { out, lhs, rhs } => {
                let (a, _) = self.load_reg(*lhs)?;
                let (b, _) = self.load_reg(*rhs)?;
                let matmul_func = self.get_function("__hlx_matmul")?;
                let res_val = self.builder.build_call(matmul_func, &[a.into(), b.into()], "mm")?
                    .try_as_basic_value().left()
                    .ok_or_else(|| anyhow!("__hlx_matmul call returned void"))?;
                self.store_reg(*out, res_val, ValueType::Pointer)?;
            }
            Instruction::Print { val } => {
                let (v, v_type) = self.load_reg(*val)?;
                let f = self.get_function("printf")?;
                
                let fmt_str = if v_type == ValueType::Pointer {
                    b"%s\n\0" as &[u8]
                } else if v_type == ValueType::Float {
                    b"%f\n\0" as &[u8]
                } else {
                    b"%lld\n\0" as &[u8]
                };

                let fmt = self.context.const_string(fmt_str, false);
                let g = self.module.add_global(fmt.get_type(), Some(inkwell::AddressSpace::default()), "fi");
                g.set_initializer(&fmt); g.set_constant(true); g.set_linkage(Linkage::Internal);
                let gep = unsafe { self.builder.build_gep(fmt.get_type(), g.as_pointer_value(), &[self.context.i32_type().const_int(0, false), self.context.i32_type().const_int(0, false)], "fg")? };
                self.builder.build_call(f, &[gep.into(), v.into()], "pc")?;
            }
            Instruction::PrintStr { val } => {
                let (v, _) = self.load_reg(*val)?;
                let f = self.get_function("printf")?;
                let fmt = self.context.const_string(b"%s\n\0", false);
                let g = self.module.add_global(fmt.get_type(), Some(inkwell::AddressSpace::default()), "fs");
                g.set_initializer(&fmt); g.set_constant(true); g.set_linkage(Linkage::Internal);
                let gep = unsafe { self.builder.build_gep(fmt.get_type(), g.as_pointer_value(), &[self.context.i32_type().const_int(0, false), self.context.i32_type().const_int(0, false)], "fg")? };
                self.builder.build_call(f, &[gep.into(), v.into()], "pc")?;
            }
            Instruction::Asm { out, template, constraints, side_effects } => {
                // Build inline assembly using inkwell Context API
                let asm_type = if out.is_some() {
                    // If there's an output, assembly returns i64
                    i64_t.fn_type(&[], false)
                } else {
                    // Otherwise, it's void
                    self.context.void_type().fn_type(&[], false)
                };

                let asm_fn_ptr = self.context.create_inline_asm(
                    asm_type,
                    template.clone(),
                    constraints.clone(),
                    *side_effects,
                    false,  // align_stack
                    None,   // dialect (None = AT&T default)
                    false   // can_throw
                );

                // Call the inline assembly using indirect call
                let result = self.builder.build_indirect_call(asm_type, asm_fn_ptr, &[], "asm_result")?;

                // If there's an output register, store the result
                if let Some(out_reg) = out {
                    if let Some(val) = result.try_as_basic_value().left() {
                        self.store_reg(*out_reg, val, ValueType::Int)?;
                    }
                }
            }
            Instruction::ParseInt { out, input } => {
                // Load the input string pointer
                let (str_val, _) = self.load_reg(*input)?;

                // Convert to pointer if needed
                let str_ptr = if str_val.is_pointer_value() {
                    str_val.into_pointer_value()
                } else {
                    self.builder.build_int_to_ptr(
                        str_val.into_int_value(),
                        self.context.ptr_type(inkwell::AddressSpace::default()),
                        "int_to_ptr"
                    )?
                };

                // Call atoi(str)
                let atoi = self.get_function("atoi")?;
                let result = self.builder.build_call(atoi, &[str_ptr.into()], "atoi_call")?
                    .try_as_basic_value().left()
                    .ok_or_else(|| anyhow!("atoi call returned void"))?;

                // atoi returns i32, need to extend to i64
                let result_i64 = self.builder.build_int_s_extend(
                    result.into_int_value(),
                    self.context.i64_type(),
                    "sext_i32_to_i64"
                )?;

                // Store the result
                self.store_reg(*out, result_i64.into(), ValueType::Int)?;
            }
            Instruction::ArraySlice { out, array, start, length } => {
                // Determine element size and type
                let (elem_size, llvm_type) = if let Some(dtype) = self.array_element_types.get(array).cloned() {
                    let base_dtype = Self::get_base_dtype(&dtype);
                    let size_and_type = match base_dtype {
                        hlx_core::instruction::DType::F32 => (4, self.context.f32_type().as_basic_type_enum()),
                        hlx_core::instruction::DType::F64 => (8, self.context.f64_type().as_basic_type_enum()),
                        hlx_core::instruction::DType::I32 => (4, self.context.i32_type().as_basic_type_enum()),
                        hlx_core::instruction::DType::I64 => (8, self.context.i64_type().as_basic_type_enum()),
                        hlx_core::instruction::DType::Bool => (1, self.context.bool_type().as_basic_type_enum()),
                        hlx_core::instruction::DType::Array(_) => (8, self.context.i64_type().as_basic_type_enum()),
                    };
                    // Propagate element type to output
                    self.array_element_types.insert(*out, dtype);
                    size_and_type
                } else {
                    // Untyped array defaults to i64
                    (8, self.context.i64_type().as_basic_type_enum())
                };

                // Load array pointer, start index, and length
                let (arr_val, _) = self.load_reg(*array)?;
                let (start_val, _) = self.load_reg(*start)?;
                let (len_val, _) = self.load_reg(*length)?;

                let arr_ptr = if arr_val.is_pointer_value() {
                    arr_val.into_pointer_value()
                } else {
                    self.builder.build_int_to_ptr(
                        arr_val.into_int_value(),
                        self.context.ptr_type(inkwell::AddressSpace::default()),
                        "arr_int_to_ptr"
                    )?
                };

                let start_idx = start_val.into_int_value();
                let length_val = len_val.into_int_value();

                // Calculate byte offsets
                let elem_size_val = i64_t.const_int(elem_size, false);
                let start_bytes = self.builder.build_int_mul(start_idx, elem_size_val, "start_bytes")?;
                let total_bytes = self.builder.build_int_mul(length_val, elem_size_val, "total_bytes")?;

                // Allocate new array
                let malloc = self.get_function("malloc")?;
                let new_arr = self.builder.build_call(malloc, &[total_bytes.into()], "slice_malloc")?
                    .try_as_basic_value().left()
                    .ok_or_else(|| anyhow!("malloc for ArraySlice returned void"))?;
                let new_arr_ptr = new_arr.into_pointer_value();

                // Calculate source pointer (array + start * elem_size)
                let src_ptr = unsafe {
                    self.builder.build_gep(
                        llvm_type,
                        arr_ptr,
                        &[start_idx],
                        "src_gep"
                    )?
                };

                // memcpy(new_arr, src_ptr, total_bytes)
                let memcpy = self.get_function("memcpy")?;
                self.builder.build_call(
                    memcpy,
                    &[new_arr_ptr.into(), src_ptr.into(), total_bytes.into()],
                    "slice_memcpy"
                )?;

                // Store result
                self.store_reg(*out, new_arr_ptr.into(), ValueType::Pointer)?;
            }
            _ => {}
        }
        Ok(())
    }
    
    fn compile_constant(&self, val: &Value) -> Result<(BasicValueEnum<'ctx>, ValueType)> {
        let i64_t = self.context.i64_type();
        let f64_t = self.context.f64_type();
        match val {
            Value::Integer(i) => {
                Ok((i64_t.const_int(*i as u64, true).into(), ValueType::Int))
            }
            Value::Float(f) => {
                Ok((f64_t.const_float(*f).into(), ValueType::Float))
            }
            Value::Boolean(b) => {
                Ok((i64_t.const_int(if *b { 1 } else { 0 }, false).into(), ValueType::Int))
            }
            Value::Null => {
                Ok((i64_t.const_int(0, false).into(), ValueType::Int))
            }
            Value::String(s) => {
                let fmt = self.context.const_string(format!("{}\0", s).as_bytes(), false);
                let g = self.module.add_global(fmt.get_type(), Some(inkwell::AddressSpace::default()), "sl");
                g.set_initializer(&fmt); g.set_constant(true); g.set_linkage(Linkage::Internal);
                let gep = unsafe { self.builder.build_gep(fmt.get_type(), g.as_pointer_value(), &[self.context.i32_type().const_int(0, false), self.context.i32_type().const_int(0, false)], "sg")? };
                let ptr_as_int = self.builder.build_ptr_to_int(gep, i64_t, "pi")?;
                Ok((ptr_as_int.into(), ValueType::Pointer))
            }
            _ => Err(anyhow!("Unsupported constant: {}", val)),
        }
    }
    
    fn load_reg(&self, reg: Register) -> Result<(BasicValueEnum<'ctx>, ValueType)> {
        // eprintln!("DEBUG: load_reg r{}", reg);
        let ptr = self.reg_map.get(&reg).ok_or_else(|| anyhow!("Reg missing"))?;
        let val_type = *self.reg_types.get(&reg).ok_or_else(|| {
            // Detailed error for debugging
            let known_regs: Vec<_> = self.reg_types.keys().copied().collect();
            anyhow!(
                "Reg type missing for r{}.\n\nThis means the compiler tried to load from a register that was never stored to.\nKnown typed registers: {:?}\n\nPossible causes:\n1. User code has uninitialized variable (LSP should catch this)\n2. Compiler bug: IR generator emitted Load before Store\n3. Backend bug: Instruction didn't call store_reg() for output\n\nSee DEBUGGING_REG_TYPE_MISSING.md for investigation steps.",
                reg, known_regs
            )
        })?;

        // Always load as i64 (storage type)
        let i64_val = self.builder.build_load(self.context.i64_type(), *ptr, "reg_load")?.into_int_value();

        // Cast back to val_type
        let val = match val_type {
            ValueType::Int => i64_val.into(),
            ValueType::Float => self.builder.build_bit_cast(i64_val, self.context.f64_type(), "i2f")?.into(),
            ValueType::Pointer => self.builder.build_int_to_ptr(i64_val, self.context.ptr_type(inkwell::AddressSpace::default()), "i2p")?.into(),
        };

        Ok((val, val_type))
    }

    fn store_reg(&mut self, reg: Register, val: BasicValueEnum<'ctx>, val_type: ValueType) -> Result<()> {
        // eprintln!("DEBUG: store_reg r{} = {:?}", reg, val_type);
        let ptr = self.reg_map.get(&reg).ok_or_else(|| anyhow!("Reg missing"))?;

        // Cast to i64 (storage type)
        let val_to_store = if val.is_pointer_value() {
            self.builder.build_ptr_to_int(val.into_pointer_value(), self.context.i64_type(), "p2i")?.into()
        } else if val.is_float_value() {
            self.builder.build_bit_cast(val.into_float_value(), self.context.i64_type(), "f2i")?.into()
        } else {
            val
        };

        self.builder.build_store(*ptr, val_to_store)?;
        self.reg_types.insert(reg, val_type);
        Ok(())
    }

    pub fn print_ir(&self) { self.module.print_to_stderr(); }

    pub fn run_jit(&self) -> Result<i64> {
        let ee = self.module.create_jit_execution_engine(OptimizationLevel::None)
            .map_err(|e| anyhow!("Failed to create JIT: {}", e))?;
        unsafe {
            let func = ee.get_function::<unsafe extern "C" fn() -> i64>("main")?;
            Ok(func.call())
        }
    }

    /// Emit native object file (.o)
    pub fn emit_object(&self, output_path: &std::path::Path) -> Result<()> {
        let triple = self.module.get_triple();
        let target = Target::from_triple(&triple)
            .map_err(|e| anyhow!("Target error: {}", e))?;

        let triple_str = triple.as_str().to_str()
            .map_err(|_| anyhow!("Target triple contains invalid UTF-8"))?;
        let is_bare_metal = triple_str.contains("none");

        let cpu_name = TargetMachine::get_host_cpu_name();
        let cpu_features = TargetMachine::get_host_cpu_features();

        let cpu = if is_bare_metal {
            "generic"
        } else {
            cpu_name.to_str()
                .map_err(|_| anyhow!("CPU name contains invalid UTF-8"))?
        };

        let features = if is_bare_metal {
            ""
        } else {
            cpu_features.to_str()
                .map_err(|_| anyhow!("CPU features string contains invalid UTF-8"))?
        };

        let target_machine = target.create_target_machine(
            &triple,
            cpu,
            features,
            OptimizationLevel::Default,
            inkwell::targets::RelocMode::PIC, // Position Independent Code for PIE compatibility
            inkwell::targets::CodeModel::Default,
        ).ok_or_else(|| anyhow!("Failed to create target machine"))?;

        target_machine.write_to_file(&self.module, inkwell::targets::FileType::Object, output_path)
            .map_err(|e| anyhow!("Failed to write object file: {}", e))?;

        Ok(())
    }

    /// Emit native assembly (.s)
    pub fn emit_assembly(&self, output_path: &std::path::Path) -> Result<()> {
        let triple = self.module.get_triple();
        let target = Target::from_triple(&triple)
            .map_err(|e| anyhow!("Target error: {}", e))?;

        let triple_str = triple.as_str().to_str()
            .map_err(|_| anyhow!("Target triple contains invalid UTF-8"))?;
        let is_bare_metal = triple_str.contains("none");

        let cpu_name = TargetMachine::get_host_cpu_name();
        let cpu_features = TargetMachine::get_host_cpu_features();

        let cpu = if is_bare_metal {
            "generic"
        } else {
            cpu_name.to_str()
                .map_err(|_| anyhow!("CPU name contains invalid UTF-8"))?
        };

        let features = if is_bare_metal {
            ""
        } else {
            cpu_features.to_str()
                .map_err(|_| anyhow!("CPU features string contains invalid UTF-8"))?
        };

        let target_machine = target.create_target_machine(
            &triple,
            cpu,
            features,
            OptimizationLevel::Default,
            inkwell::targets::RelocMode::PIC, // Position Independent Code for PIE compatibility
            inkwell::targets::CodeModel::Default,
        ).ok_or_else(|| anyhow!("Failed to create target machine"))?;

        target_machine.write_to_file(&self.module, inkwell::targets::FileType::Assembly, output_path)
            .map_err(|e| anyhow!("Failed to write assembly file: {}", e))?;

        Ok(())
    }
}

// Backend Capability Implementation for LLVM Native Compiler
impl<'ctx> hlx_runtime::backend::BackendCapability for CodeGen<'ctx> {
    fn supported_contracts(&self) -> Vec<String> {
        // LLVM backend supports core T0 contracts
        // Expand this list as more contracts are implemented
        vec![
            // T0 Core Language contracts
            "100".to_string(), // Example T0 contract
            "101".to_string(),
            "102".to_string(),
            // Add more as they're implemented in LLVM backend
        ]
    }

    fn supported_builtins(&self) -> Vec<String> {
        // LLVM backend supports basic builtins that don't require libc math
        vec![
            // I/O (basic, no advanced features yet)
            "print".to_string(),
            "read_file".to_string(),
            "write_file".to_string(),

            // Type operations
            "type".to_string(),
            "len".to_string(),

            // Type conversions
            "to_string".to_string(),
            "to_int".to_string(),

            // String operations (basic)
            "concat".to_string(),

            // Array operations (basic)
            "append".to_string(),
            "slice".to_string(),

            // Math functions (NOW SUPPORTED!)
            "sin".to_string(),
            "cos".to_string(),
            "tan".to_string(),
            "sqrt".to_string(),
            "log".to_string(),
            "exp".to_string(),
            "floor".to_string(),
            "ceil".to_string(),
            "round".to_string(),
        ]
    }

    fn backend_name(&self) -> &'static str {
        "LLVM Native (AOT)"
    }

    fn unsupported_contracts(&self) -> Vec<String> {
        // Note: unsupported_builtins removed - not part of trait
        // Missing builtins are implicitly: everything not in supported_builtins()
        // Notably missing: sin, cos, tan, log, exp, sqrt (no libc linking yet)
        Vec::new()
    }
}

impl<'ctx> CodeGen<'ctx> {
    /// Helper: Check if a builtin is supported
    pub fn is_builtin_supported(&self, name: &str) -> bool {
        use hlx_runtime::backend::BackendCapability;
        self.supported_builtins().contains(&name.to_string())
    }

    /// Static capability query - can be called without CodeGen instance
    ///
    /// Used by LSP to detect backend compatibility issues
    pub fn static_supported_builtins() -> Vec<String> {
        vec![
            // I/O
            "print".to_string(),
            "read_file".to_string(),
            "write_file".to_string(),
            // Type operations
            "type".to_string(),
            "len".to_string(),
            // Type conversions
            "to_string".to_string(),
            "to_int".to_string(),
            // String/Array operations
            "concat".to_string(),
            "append".to_string(),
            "slice".to_string(),
            // Math functions (NOW SUPPORTED!)
            "sin".to_string(),
            "cos".to_string(),
            "tan".to_string(),
            "sqrt".to_string(),
            "log".to_string(),
            "exp".to_string(),
            "floor".to_string(),
            "ceil".to_string(),
            "round".to_string(),
        ]
    }

    /// Emit shared library (.so on Linux, .dylib on macOS, .dll on Windows)
    pub fn emit_shared(&self, output_path: &std::path::Path) -> Result<()> {
        use std::process::Command;

        // First, emit an object file to a temporary location
        let obj_path = output_path.with_extension("o");
        self.emit_object(&obj_path)?;

        // Determine the linker command based on OS
        let os = std::env::consts::OS;
        let (linker, args, extension) = match os {
            "linux" => ("gcc", vec!["-shared", "-fPIC"], "so"),
            "macos" => ("clang", vec!["-dynamiclib", "-fPIC"], "dylib"),
            "windows" => ("link.exe", vec!["/DLL"], "dll"),
            _ => return Err(anyhow!("Unsupported platform for shared library: {}", os)),
        };

        // Build final output path with correct extension
        let final_output = if output_path.extension().is_some() {
            output_path.to_path_buf()
        } else {
            output_path.with_extension(extension)
        };

        // Invoke the linker
        let mut cmd = Command::new(linker);
        cmd.args(&args);
        cmd.arg(&obj_path);
        cmd.arg("-o");
        cmd.arg(&final_output);

        let output = cmd.output()
            .map_err(|e| anyhow!("Failed to invoke linker '{}': {}", linker, e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Linker failed:\n{}", stderr));
        }

        // Clean up temporary object file
        let _ = std::fs::remove_file(&obj_path);

        Ok(())
    }
}
