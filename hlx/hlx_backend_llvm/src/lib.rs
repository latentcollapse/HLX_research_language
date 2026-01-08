//! HLX LLVM Backend (Iron)
//! 
//! Compiles HLX IR (LC-B) to Native Machine Code via LLVM.

use hlx_core::{HlxCrate, Instruction, Value, Register};
use inkwell::context::Context;
use inkwell::builder::Builder;
use inkwell::module::{Module, Linkage};
use inkwell::values::{FunctionValue, IntValue, FloatValue, BasicValueEnum, PointerValue};
use inkwell::basic_block::BasicBlock;
use inkwell::{IntPredicate, FloatPredicate};
use inkwell::OptimizationLevel;
use inkwell::targets::{Target, InitializationConfig, TargetMachine};
use std::collections::{HashMap, HashSet};
use anyhow::{Result, anyhow};

/// Runtime type of a value
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValueType {
    Int,
    Float,
    Pointer,
}

pub struct CodeGen<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    functions: HashMap<String, FunctionValue<'ctx>>,
    reg_map: HashMap<Register, PointerValue<'ctx>>,
    reg_types: HashMap<Register, ValueType>,  // Track register types
    block_map: HashMap<u32, BasicBlock<'ctx>>,
}

use std::env;

impl<'ctx> CodeGen<'ctx> {
    /// Create a new code generator with default (host) target
    pub fn new(context: &'ctx Context, module_name: &str) -> Self {
        Self::with_target(context, module_name, None)
    }

    /// Create a new code generator with a specific target triple
    ///
    /// # Examples
    /// - `None` - Use host target (default)
    /// - `Some("x86_64-unknown-none-elf")` - Bare metal x86_64
    /// - `Some("aarch64-unknown-none")` - Bare metal ARM64
    /// - `Some("riscv64gc-unknown-none-elf")` - Bare metal RISC-V
    pub fn with_target(context: &'ctx Context, module_name: &str, target_triple: Option<&str>) -> Self {
        Target::initialize_native(&InitializationConfig::default()).unwrap();

        // Load symbols from current executable so JIT can find printf, malloc, etc.
        // (Only for hosted targets)
        if target_triple.is_none() || !target_triple.unwrap().contains("none") {
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

        let target = Target::from_triple(&triple).unwrap();

        // For bare metal targets, use generic CPU features
        let (cpu, features) = if target_triple.map_or(false, |t| t.contains("none")) {
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
            inkwell::targets::RelocMode::Default,
            inkwell::targets::CodeModel::Default,
        ).unwrap();

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

        // Math (libm)
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

        // SDL2
        let sdl_init = module.add_function("SDL_Init", i32_type.fn_type(&[i32_type.into()], false), Some(Linkage::External));
        let sdl_create_window = module.add_function("SDL_CreateWindow", ptr_type.fn_type(&[ptr_type.into(), i32_type.into(), i32_type.into(), i32_type.into(), i32_type.into(), i32_type.into()], false), Some(Linkage::External));
        let sdl_create_renderer = module.add_function("SDL_CreateRenderer", ptr_type.fn_type(&[ptr_type.into(), i32_type.into(), i32_type.into()], false), Some(Linkage::External));
        let sdl_set_color = module.add_function("SDL_SetRenderDrawColor", i32_type.fn_type(&[ptr_type.into(), context.i8_type().into(), context.i8_type().into(), context.i8_type().into(), context.i8_type().into()], false), Some(Linkage::External));
        let sdl_clear = module.add_function("SDL_RenderClear", i32_type.fn_type(&[ptr_type.into()], false), Some(Linkage::External));
        let sdl_present = module.add_function("SDL_RenderPresent", context.void_type().fn_type(&[ptr_type.into()], false), Some(Linkage::External));
        let sdl_poll = module.add_function("SDL_PollEvent", i32_type.fn_type(&[ptr_type.into()], false), Some(Linkage::External));
        let sdl_delay = module.add_function("SDL_Delay", context.void_type().fn_type(&[i32_type.into()], false), Some(Linkage::External));
        let sdl_quit = module.add_function("SDL_Quit", context.void_type().fn_type(&[], false), Some(Linkage::External));

        // Register both PascalCase and snake_case variants
        functions.insert("SDL_Init".to_string(), sdl_init);
        functions.insert("sdl_init".to_string(), sdl_init);
        functions.insert("SDL_CreateWindow".to_string(), sdl_create_window);
        functions.insert("sdl_create_window".to_string(), sdl_create_window);
        functions.insert("SDL_CreateRenderer".to_string(), sdl_create_renderer);
        functions.insert("sdl_create_renderer".to_string(), sdl_create_renderer);
        functions.insert("SDL_SetRenderDrawColor".to_string(), sdl_set_color);
        functions.insert("sdl_set_color".to_string(), sdl_set_color);
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

        let mut codegen = Self {
            context,
            module,
            builder,
            functions,
            reg_map: HashMap::new(),
            reg_types: HashMap::new(),
            block_map: HashMap::new(),
        };
        
        codegen.define_tensor_utils().unwrap();
        
        let helpers = ["__hlx_tensor_create", "__hlx_matmul", "__hlx_tensor_add", "__hlx_tensor_transpose"];
        for h in helpers {
            codegen.functions.insert(h.to_string(), codegen.module.get_function(h).unwrap());
        }
        codegen
    }
    
    fn define_tensor_utils(&mut self) -> Result<()> {
        let i64_type = self.context.i64_type();
        let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
        let malloc = *self.functions.get("malloc").unwrap();

        let create_func = self.module.add_function("__hlx_tensor_create", ptr_type.fn_type(&[i64_type.into(), i64_type.into()], false), Some(Linkage::Internal));
        let bb = self.context.append_basic_block(create_func, "entry");
        self.builder.position_at_end(bb);
        let rows = create_func.get_nth_param(0).unwrap().into_int_value();
        let cols = create_func.get_nth_param(1).unwrap().into_int_value();
        let s_ptr = self.builder.build_call(malloc, &[i64_type.const_int(24, false).into()], "s_ptr")?.try_as_basic_value().left().unwrap().into_pointer_value();
        let count = self.builder.build_int_mul(rows, cols, "cnt")?;
        let d_ptr = self.builder.build_call(malloc, &[self.builder.build_int_mul(count, i64_type.const_int(8, false), "db")?.into()], "d_ptr")?.try_as_basic_value().left().unwrap().into_pointer_value();
        unsafe {
            self.builder.build_store(self.builder.build_gep(i64_type, s_ptr, &[i64_type.const_int(0, false)], "r")?, rows)?;
            self.builder.build_store(self.builder.build_gep(i64_type, s_ptr, &[i64_type.const_int(1, false)], "c")?, cols)?;
            self.builder.build_store(self.builder.build_gep(i64_type, s_ptr, &[i64_type.const_int(2, false)], "d")?, self.builder.build_ptr_to_int(d_ptr, i64_type, "di")?)?;
        }
        self.builder.build_return(Some(&s_ptr))?;
        
        let matmul_func = self.module.add_function("__hlx_matmul", ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false), Some(Linkage::Internal));
        let bb = self.context.append_basic_block(matmul_func, "entry");
        self.builder.position_at_end(bb);
        let a_ptr = matmul_func.get_nth_param(0).unwrap().into_pointer_value();
        let b_ptr = matmul_func.get_nth_param(1).unwrap().into_pointer_value();
        unsafe {
            let m = self.builder.build_load(i64_type, self.builder.build_gep(i64_type, a_ptr, &[i64_type.const_int(0, false)], "ar")?, "m")?.into_int_value();
            let k = self.builder.build_load(i64_type, self.builder.build_gep(i64_type, a_ptr, &[i64_type.const_int(1, false)], "ac")?, "k")?.into_int_value();
            let ad_i = self.builder.build_load(i64_type, self.builder.build_gep(i64_type, a_ptr, &[i64_type.const_int(2, false)], "ad")?, "adi")?.into_int_value();
            let ad = self.builder.build_int_to_ptr(ad_i, ptr_type, "ad")?;
            let n = self.builder.build_load(i64_type, self.builder.build_gep(i64_type, b_ptr, &[i64_type.const_int(1, false)], "bc")?, "n")?.into_int_value();
            let bd_i = self.builder.build_load(i64_type, self.builder.build_gep(i64_type, b_ptr, &[i64_type.const_int(2, false)], "bd")?, "bdi")?.into_int_value();
            let bd = self.builder.build_int_to_ptr(bd_i, ptr_type, "bd")?;
            let c_ptr = self.builder.build_call(create_func, &[m.into(), n.into()], "cp")?.try_as_basic_value().left().unwrap().into_pointer_value();
            let cd_i = self.builder.build_load(i64_type, self.builder.build_gep(i64_type, c_ptr, &[i64_type.const_int(2, false)], "cd")?, "cdi")?.into_int_value();
            let cd = self.builder.build_int_to_ptr(cd_i, ptr_type, "cd")?;

            let i_a = self.builder.build_alloca(i64_type, "i")?; self.builder.build_store(i_a, i64_type.const_int(0, false))?;
            let c_i = self.context.append_basic_block(matmul_func, "ci");
            let b_i = self.context.append_basic_block(matmul_func, "bi");
            let e_i = self.context.append_basic_block(matmul_func, "ei");
            self.builder.build_unconditional_branch(c_i)?;
            self.builder.position_at_end(c_i);
            let i_v = self.builder.build_load(i64_type, i_a, "iv")?.into_int_value();
            self.builder.build_conditional_branch(self.builder.build_int_compare(IntPredicate::SLT, i_v, m, "cmpi")?, b_i, e_i)?;
            self.builder.position_at_end(b_i);
            let j_a = self.builder.build_alloca(i64_type, "j")?; self.builder.build_store(j_a, i64_type.const_int(0, false))?;
            let c_j = self.context.append_basic_block(matmul_func, "cj");
            let b_j = self.context.append_basic_block(matmul_func, "bj");
            let e_j = self.context.append_basic_block(matmul_func, "ej");
            self.builder.build_unconditional_branch(c_j)?;
            self.builder.position_at_end(c_j);
            let j_v = self.builder.build_load(i64_type, j_a, "jv")?.into_int_value();
            self.builder.build_conditional_branch(self.builder.build_int_compare(IntPredicate::SLT, j_v, n, "cmpj")?, b_j, e_j)?;
            self.builder.position_at_end(b_j);
            let s_a = self.builder.build_alloca(i64_type, "s")?; self.builder.build_store(s_a, i64_type.const_int(0, false))?;
            let k_a = self.builder.build_alloca(i64_type, "k")?; self.builder.build_store(k_a, i64_type.const_int(0, false))?;
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
        let a_ptr = add_func.get_nth_param(0).unwrap().into_pointer_value();
        let b_ptr = add_func.get_nth_param(1).unwrap().into_pointer_value();
        unsafe {
            let r = self.builder.build_load(i64_type, self.builder.build_gep(i64_type, a_ptr, &[i64_type.const_int(0, false)], "r")?, "rows")?.into_int_value();
            let c = self.builder.build_load(i64_type, self.builder.build_gep(i64_type, a_ptr, &[i64_type.const_int(1, false)], "c")?, "cols")?.into_int_value();
            let ad = self.builder.build_int_to_ptr(self.builder.build_load(i64_type, self.builder.build_gep(i64_type, a_ptr, &[i64_type.const_int(2, false)], "ad")?, "adi")?.into_int_value(), ptr_type, "ad")?;
            let bd = self.builder.build_int_to_ptr(self.builder.build_load(i64_type, self.builder.build_gep(i64_type, b_ptr, &[i64_type.const_int(2, false)], "bd")?, "bdi")?.into_int_value(), ptr_type, "bd")?;
            let c_ptr = self.builder.build_call(create_func, &[r.into(), c.into()], "cp")?.try_as_basic_value().left().unwrap().into_pointer_value();
            let cd = self.builder.build_int_to_ptr(self.builder.build_load(i64_type, self.builder.build_gep(i64_type, c_ptr, &[i64_type.const_int(2, false)], "cd")?, "cdi")?.into_int_value(), ptr_type, "cd")?;
            let tot = self.builder.build_int_mul(r, c, "tot")?;
            let i_a = self.builder.build_alloca(i64_type, "i")?; self.builder.build_store(i_a, i64_type.const_int(0, false))?;
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
        let a_ptr = trans_func.get_nth_param(0).unwrap().into_pointer_value();
        unsafe {
            let m = self.builder.build_load(i64_type, self.builder.build_gep(i64_type, a_ptr, &[i64_type.const_int(0, false)], "r")?, "m")?.into_int_value();
            let n = self.builder.build_load(i64_type, self.builder.build_gep(i64_type, a_ptr, &[i64_type.const_int(1, false)], "c")?, "n")?.into_int_value();
            let ad = self.builder.build_int_to_ptr(self.builder.build_load(i64_type, self.builder.build_gep(i64_type, a_ptr, &[i64_type.const_int(2, false)], "ad")?, "adi")?.into_int_value(), ptr_type, "ad")?;
            let c_ptr = self.builder.build_call(create_func, &[n.into(), m.into()], "cp")?.try_as_basic_value().left().unwrap().into_pointer_value();
            let cd = self.builder.build_int_to_ptr(self.builder.build_load(i64_type, self.builder.build_gep(i64_type, c_ptr, &[i64_type.const_int(2, false)], "cd")?, "cdi")?.into_int_value(), ptr_type, "cd")?;
            let i_a = self.builder.build_alloca(i64_type, "i")?; self.builder.build_store(i_a, i64_type.const_int(0, false))?;
            let c_i = self.context.append_basic_block(trans_func, "ci");
            let b_i = self.context.append_basic_block(trans_func, "bi");
            let end_i = self.context.append_basic_block(trans_func, "ei");
            self.builder.build_unconditional_branch(c_i)?;
            self.builder.position_at_end(c_i);
            let i_v = self.builder.build_load(i64_type, i_a, "iv")?.into_int_value();
            self.builder.build_conditional_branch(self.builder.build_int_compare(IntPredicate::SLT, i_v, m, "cmpi")?, b_i, end_i)?;
            self.builder.position_at_end(b_i);
            let j_a = self.builder.build_alloca(i64_type, "j")?; self.builder.build_store(j_a, i64_type.const_int(0, false))?;
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
        let i64_type = self.context.i64_type();
        for inst in &krate.instructions {
            if let Instruction::FuncDef { name, params, .. } = inst {
                if self.functions.contains_key(name) { continue; }
                let param_types = vec![i64_type.into(); params.len()];
                let function = self.module.add_function(name, i64_type.fn_type(&param_types, false), None);
                self.functions.insert(name.clone(), function);
            }
        }
        let mut targets = HashSet::new();
        for inst in &krate.instructions {
            match inst {
                Instruction::Jump { target } => { targets.insert(*target); },
                Instruction::If { then_block, else_block, .. } => {
                    targets.insert(*then_block);
                    targets.insert(*else_block);
                },
                Instruction::Loop { body, .. } => { targets.insert(*body); },
                _ => {} 
            }
        }
        for inst in &krate.instructions {
            if let Instruction::FuncDef { name, params, body } = inst {
                self.compile_function(name, params, *body, &krate.instructions, &targets)?;
            }
        }
        Ok(())
    }

    fn compile_function(&mut self, name: &str, params: &[Register], start_pc: u32, instructions: &[Instruction], all_targets: &HashSet<u32>) -> Result<()> {
        let function = *self.functions.get(name).ok_or_else(|| anyhow!("Fn missing"))?;
        self.reg_map.clear();
        self.reg_types.clear();
        self.block_map.clear();
        let entry_bb = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry_bb);
        let mut pc = start_pc as usize;
        let mut used_regs = HashSet::new();
        for &r in params { used_regs.insert(r); }
        while pc < instructions.len() {
            let inst = &instructions[pc];
            if pc > start_pc as usize && matches!(inst, Instruction::FuncDef { .. }) { break; }
            if let Some(r) = inst.output_register() { used_regs.insert(r); }
            for &r in &inst.input_registers() { used_regs.insert(r); }
            if all_targets.contains(&(pc as u32)) || pc == start_pc as usize {
                let bb = self.context.append_basic_block(function, &format!("pc_{}", pc));
                self.block_map.insert(pc as u32, bb);
            }
            pc += 1;
        }
        for &r in &used_regs {
            let ptr = self.builder.build_alloca(self.context.i64_type(), &format!("r{}", r))?;
            self.reg_map.insert(r, ptr);
        }
        for (i, &reg) in params.iter().enumerate() {
            let val = function.get_nth_param(i as u32).unwrap();
            // Params are assumed to be integers for now, matching the old behavior
            // but we wrap them in the new typed store_reg
            self.store_reg(reg, val, ValueType::Int)?;
        }
        self.builder.build_unconditional_branch(*self.block_map.get(&start_pc).unwrap())?;
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
                if self.builder.get_insert_block().unwrap() != *bb {
                    if self.builder.get_insert_block().unwrap().get_terminator().is_none() {
                        self.builder.build_unconditional_branch(*bb)?;
                    }
                    self.builder.position_at_end(*bb);
                }
            }
            self.compile_inst(inst)?;
            pc += 1;
        }
        if self.builder.get_insert_block().unwrap().get_terminator().is_none() {
            self.builder.build_return(Some(&self.context.i64_type().const_int(0, false)))?;
        }
        Ok(())
    }
    
    fn compile_inst(&mut self, inst: &Instruction) -> Result<()> {
        let i64_t = self.context.i64_type();
        let ptr_t = self.context.ptr_type(inkwell::AddressSpace::default());
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
                    _ => return Err(anyhow!("Type mismatch in Eq: {:?} == {:?}", l_type, r_type)),
                };

                let ext = self.builder.build_int_z_extend(res, self.context.i64_type(), "bool_ext")?;
                self.store_reg(*out, ext.into(), ValueType::Int)?;
            }
            Instruction::Call { out, func, args } => {
                // Builtin intrinsics
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
                if func == "to_string" {
                    let (val, val_type) = self.load_reg(args[0])?;
                    let malloc = *self.functions.get("malloc").unwrap();
                    let sprintf = *self.functions.get("sprintf").ok_or(anyhow!("sprintf missing"))?;
                    
                    // Allocate 32 bytes for the string
                    let buf = self.builder.build_call(malloc, &[self.context.i64_type().const_int(32, false).into()], "buf")?
                        .try_as_basic_value().left().unwrap().into_pointer_value();
                    
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
                self.builder.build_return(Some(&v))?;
            }
            Instruction::Jump { target } => {
                let bb = self.get_block(*target)?;
                self.builder.build_unconditional_branch(bb)?;
            }
            Instruction::If { cond, then_block, else_block } => {
                let (c, _c_type) = self.load_reg(*cond)?;
                let c_int = c.into_int_value();
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
            Instruction::ArrayCreate { out, elements } => {
                let malloc = *self.functions.get("malloc").unwrap();
                let ptr_val = self.builder.build_call(malloc, &[i64_t.const_int(elements.len() as u64 * 8, false).into()], "malloc_call")?
                    .try_as_basic_value().left().unwrap();
                let ptr = ptr_val.into_pointer_value();
                for (i, &reg) in elements.iter().enumerate() {
                    let (val, _v_type) = self.load_reg(reg)?;
                    let idx = self.context.i64_type().const_int(i as u64, false);
                    unsafe {
                        let elem_ptr = self.builder.build_gep(self.context.i64_type(), ptr, &[idx], "gep")?;
                        self.builder.build_store(elem_ptr, val)?;
                    }
                }
                self.store_reg(*out, ptr_val, ValueType::Pointer)?;
            }
            Instruction::ArrayAlloc { out, size } => {
                let (size_val, _) = self.load_reg(*size)?;
                let bytes = self.builder.build_int_mul(size_val.into_int_value(), i64_t.const_int(8, false), "bytes")?;
                let malloc = *self.functions.get("malloc").unwrap();
                let ptr_val = self.builder.build_call(malloc, &[bytes.into()], "malloc_call")?
                    .try_as_basic_value().left().unwrap();
                self.store_reg(*out, ptr_val, ValueType::Pointer)?;
            }
            Instruction::Index { out, container, index } => {
                let (ptr_val, _) = self.load_reg(*container)?;
                let (idx_val, _) = self.load_reg(*index)?;
                let ptr = ptr_val.into_pointer_value();
                let idx = idx_val.into_int_value();
                unsafe {
                    let elem_ptr = self.builder.build_gep(self.context.i64_type(), ptr, &[idx], "gep")?;
                    let val = self.builder.build_load(self.context.i64_type(), elem_ptr, "elem_load")?;
                    self.store_reg(*out, val, ValueType::Int)?;
                }
            }
            Instruction::Store { container, index, value } => {
                let (ptr_val, _) = self.load_reg(*container)?;
                let (idx_val, _) = self.load_reg(*index)?;
                let (val, _) = self.load_reg(*value)?;
                let ptr = ptr_val.into_pointer_value();
                let idx = idx_val.into_int_value();
                unsafe {
                    let elem_ptr = self.builder.build_gep(self.context.i64_type(), ptr, &[idx], "gep")?;
                    self.builder.build_store(elem_ptr, val)?;
                }
            }
            Instruction::TensorCreate { out, shape, .. } => {
                let ptr_val = self.builder.build_call(self.module.get_function("__hlx_tensor_create").unwrap(), &[i64_t.const_int(shape[0] as u64, false).into(), i64_t.const_int(shape[1] as u64, false).into()], "ta")?.try_as_basic_value().left().unwrap();
                self.store_reg(*out, ptr_val, ValueType::Pointer)?;
            }
            Instruction::MatMul { out, lhs, rhs } => {
                let (a, _) = self.load_reg(*lhs)?;
                let (b, _) = self.load_reg(*rhs)?;
                let res_val = self.builder.build_call(self.module.get_function("__hlx_matmul").unwrap(), &[a.into(), b.into()], "mm")?.try_as_basic_value().left().unwrap();
                self.store_reg(*out, res_val, ValueType::Pointer)?;
            }
            Instruction::Print { val } => {
                let (v, _) = self.load_reg(*val)?;
                let f = *self.functions.get("printf").unwrap();
                let fmt = self.context.const_string(b"%lld\n\0", false);
                let g = self.module.add_global(fmt.get_type(), Some(inkwell::AddressSpace::default()), "fi");
                g.set_initializer(&fmt); g.set_constant(true); g.set_linkage(Linkage::Internal);
                let gep = unsafe { self.builder.build_gep(fmt.get_type(), g.as_pointer_value(), &[self.context.i32_type().const_int(0, false), self.context.i32_type().const_int(0, false)], "fg")? };
                self.builder.build_call(f, &[gep.into(), v.into()], "pc")?;
            }
            Instruction::PrintStr { val } => {
                let (v, _) = self.load_reg(*val)?;
                let f = *self.functions.get("printf").unwrap();
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
            _ => Err(anyhow!("Unsupported constant: {:?}", val)),
        }
    }
    
    fn load_reg(&self, reg: Register) -> Result<(BasicValueEnum<'ctx>, ValueType)> {
        let ptr = self.reg_map.get(&reg).ok_or_else(|| anyhow!("Reg missing"))?;
        let val_type = *self.reg_types.get(&reg).ok_or_else(|| {
            // Detailed error for debugging
            let known_regs: Vec<_> = self.reg_types.keys().copied().collect();
            anyhow!(
                "Reg type missing for r{}.\n\
                \n\
                This means the compiler tried to load from a register that was never stored to.\n\
                Known typed registers: {:?}\n\
                \n\
                Possible causes:\n\
                1. User code has uninitialized variable (LSP should catch this)\n\
                2. Compiler bug: IR generator emitted Load before Store\n\
                3. Backend bug: Instruction didn't call store_reg() for output\n\
                \n\
                See DEBUGGING_REG_TYPE_MISSING.md for investigation steps.",
                reg, known_regs
            )
        })?;

        // Registers are allocated as i64, bitcast to f64 if needed
        let val = match val_type {
            ValueType::Int | ValueType::Pointer => {
                self.builder.build_load(self.context.i64_type(), *ptr, "reg_load")?
            }
            ValueType::Float => {
                let as_int = self.builder.build_load(self.context.i64_type(), *ptr, "reg_load_int")?;
                self.builder.build_bit_cast(as_int, self.context.f64_type(), "int_to_float")?
            }
        };

        Ok((val, val_type))
    }

    fn store_reg(&mut self, reg: Register, val: BasicValueEnum<'ctx>, val_type: ValueType) -> Result<()> {
        let ptr = self.reg_map.get(&reg).ok_or_else(|| anyhow!("Reg missing"))?;

        // If storing float, bitcast to i64 first (all registers are i64 allocas)
        let to_store = match val_type {
            ValueType::Int | ValueType::Pointer => val,
            ValueType::Float => {
                self.builder.build_bit_cast(val, self.context.i64_type(), "float_to_int")?
            }
        };

        self.builder.build_store(*ptr, to_store)?;
        self.reg_types.insert(reg, val_type);
        Ok(())
    }
    
    fn get_block(&self, target: u32) -> Result<BasicBlock<'ctx>> {
        self.block_map.get(&target).copied().ok_or_else(|| anyhow!("Target missing"))
    }

    pub fn print_ir(&self) { self.module.print_to_stderr(); }

    pub fn run_jit(&self) -> Result<i64> {
        let ee = self.module.create_jit_execution_engine(OptimizationLevel::None)
            .map_err(|e| anyhow!("Failed to create JIT: {:?}", e))?;
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

        let triple_str = triple.as_str().to_str().unwrap();
        let is_bare_metal = triple_str.contains("none");

        let cpu_name = TargetMachine::get_host_cpu_name();
        let cpu_features = TargetMachine::get_host_cpu_features();

        let cpu = if is_bare_metal {
            "generic"
        } else {
            cpu_name.to_str().unwrap()
        };

        let features = if is_bare_metal {
            ""
        } else {
            cpu_features.to_str().unwrap()
        };

        let target_machine = target.create_target_machine(
            &triple,
            cpu,
            features,
            OptimizationLevel::Default,
            inkwell::targets::RelocMode::Default,
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

        let triple_str = triple.as_str().to_str().unwrap();
        let is_bare_metal = triple_str.contains("none");

        let cpu_name = TargetMachine::get_host_cpu_name();
        let cpu_features = TargetMachine::get_host_cpu_features();

        let cpu = if is_bare_metal {
            "generic"
        } else {
            cpu_name.to_str().unwrap()
        };

        let features = if is_bare_metal {
            ""
        } else {
            cpu_features.to_str().unwrap()
        };

        let target_machine = target.create_target_machine(
            &triple,
            cpu,
            features,
            OptimizationLevel::Default,
            inkwell::targets::RelocMode::Default,
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

            // NOTABLY MISSING: Math functions (sin, cos, tan, log, exp)
            // These require libc linking which isn't set up yet
            // "sin" - ❌ NOT IN LLVM BACKEND YET
            // "cos" - ❌ NOT IN LLVM BACKEND YET
            // "tan" - ❌ NOT IN LLVM BACKEND YET
            // "log" - ❌ NOT IN LLVM BACKEND YET
            // "exp" - ❌ NOT IN LLVM BACKEND YET

            // JSON operations - may need external lib
            // "json_parse" - ❌ NOT IN LLVM BACKEND YET
            // "json_stringify" - ❌ NOT IN LLVM BACKEND YET

            // HTTP operations - definitely needs external lib
            // "http_request" - ❌ NOT IN LLVM BACKEND YET
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
}