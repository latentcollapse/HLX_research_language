//! Executor
//!
//! The main instruction dispatch engine. Takes LC-B crates and
//! executes them deterministically against the configured backend.

use hlx_core::{HlxCrate, Instruction, Value, Result, HlxError};
use crate::config::RuntimeConfig;
use crate::backend::{Backend, create_backend, TensorHandle};
use crate::value_store::ValueStore;
use crate::speculation::{SpeculationCoordinator, SpeculationConfig};
use std::collections::HashMap;
use im::{Vector, OrdMap};
use std::process::{Child, Command, Stdio};
use std::io::Write;
use xcap::Monitor;

/// The executor runs LC-B crates
pub struct Executor {
    config: RuntimeConfig,
    backend: Box<dyn Backend>,
}

impl Executor {
    /// Create a new executor with the given configuration
    pub fn new(config: &RuntimeConfig) -> Result<Self> {
        let backend = create_backend(config)?;
        
        Ok(Self {
            config: config.clone(),
            backend,
        })
    }
    
    /// Run a crate and return its result
    pub fn run(&mut self, krate: &HlxCrate) -> Result<Value> {
        // Check if speculation is disabled (set by speculation coordinator to prevent recursion)
        let speculation_disabled = crate::is_speculation_disabled();

        // Check if main() has @swarm pragma (Phase 1B: main-only speculation)
        if !speculation_disabled {
            if let Some(metadata) = &krate.metadata {
                if let Some(main_info) = metadata.hlx_scale_substrates.get("main") {
                    if main_info.enable_speculation && main_info.agent_count > 1 {
                    let log_enabled = self.config.debug || std::env::var("RUST_LOG").is_ok();

                    if log_enabled {
                        println!("[HLX-SCALE] main() has @swarm(size={}), enabling speculation",
                                main_info.agent_count);
                        println!("[HLX-SCALE] Substrate: {}, Barriers: {}",
                                main_info.substrate, main_info.barrier_count);
                    }

                    // Route to speculation coordinator
                    let spec_config = SpeculationConfig::default()
                        .with_agent_count(main_info.agent_count)
                        .with_max(1024);  // Default max from Grok feedback

                    let spec_config = SpeculationConfig {
                        debug: self.config.debug,
                        strict_verification: true,
                        ..spec_config
                    };

                        let mut coordinator = SpeculationCoordinator::new(spec_config);
                        return coordinator.execute_speculative(krate);
                    }
                }
            }
        }

        // No speculation needed, run normally (serial execution)
        // Create execution context
        let mut ctx = ExecutionContext::new(&self.config);

        // Scan for function definitions first
        for inst in krate.instructions.iter() {
            if let Instruction::FuncDef { name, params, body } = inst {
                if self.config.debug { println!("Registered function: {}", name); }
                ctx.functions.insert(name.clone(), (*body, params.clone()));
            }
        }

        // Execute instructions - setup main entry point
        let mut pc = if let Some((start, param_regs)) = ctx.functions.get("main").cloned() {
            // If main_input is provided and main takes a parameter, set it up
            if let Some(input) = &self.config.main_input {
                if !param_regs.is_empty() {
                    let mut new_registers = HashMap::new();
                    new_registers.insert(param_regs[0], Value::String(input.clone()));
                    ctx.call_stack.push(StackFrame {
                        return_pc: usize::MAX, // No return - this is entry point
                        out_register: 0,
                        registers: new_registers,
                        loop_counters: HashMap::new(),
                    });
                }
            }
            start as usize
        } else {
            0
        };
        
        while pc < krate.instructions.len() {
            let inst = &krate.instructions[pc];
            ctx.trace_buffer.push_back((pc, format!("{:?}", inst)));
            if !self.config.debug && ctx.trace_buffer.len() > 50 { 
                ctx.trace_buffer.pop_front(); 
            }
            
            match ctx.execute_instruction(inst, pc, &mut *self.backend) {
                Ok(flow) => match flow {
                    ControlFlow::Continue => pc += 1,
                    ControlFlow::Jump(target) => {
                        if target == u32::MAX { break; }
                        pc = target as usize;
                    }
                    ControlFlow::Break => { break; }
                    ControlFlow::ContinueIter => { pc += 1; }
                },
                Err(e) => {
                    println!("\n=== EXECUTION CRASH DUMP ===");
                    println!("Last 50 instructions executed:");
                    for (trace_pc, op_str) in &ctx.trace_buffer {
                         println!("  PC {:04}: {}", trace_pc, op_str);
                    }
                    println!("============================\n");
                    return Err(e);
                }
            }
        }
        
        Ok(ctx.return_value)
    }
}

/// Control flow result from instruction execution
enum ControlFlow {
    Continue,
    Jump(u32),
    #[allow(dead_code)] // Reserved for future Break instruction
    Break,
    #[allow(dead_code)] // Reserved for future Continue instruction
    ContinueIter,
}

/// A frame on the call stack
struct StackFrame {
    return_pc: usize,
    out_register: u32,
    registers: HashMap<u32, Value>,
    /// Loop counters (PC -> count) for DLB within this frame
    loop_counters: HashMap<u32, u32>,
}

use std::collections::VecDeque;

/// Execution context holding state
struct ExecutionContext {
    /// Function table (name -> (start_index, params))
    functions: HashMap<String, (u32, Vec<u32>)>,
    
    /// Call stack
    call_stack: Vec<StackFrame>,

    /// Tensor handles for GPU operations
    tensors: HashMap<u32, TensorHandle>,
    
    /// Content-addressed storage for handles
    cas: ValueStore,
    
    /// Loop stack for break/continue tracking: (loop_entry_pc, loop_exit_pc)
    loop_stack: Vec<(u32, u32)>,

    /// Final return value
    return_value: Value,
    
    /// Backend reference (we'll use a simple approach for now)
    config: RuntimeConfig,

    /// Flight recorder: Last 50 PCs (PC, Instruction String)
    trace_buffer: VecDeque<(usize, String)>,

    /// Subprocess pipes (handle_id -> Child)
    pipes: HashMap<u32, Child>,
    next_pipe_id: u32,

    // SDL2
    sdl_context: Option<sdl2::Sdl>,
    video_subsystem: Option<sdl2::VideoSubsystem>,
    event_pump: Option<sdl2::EventPump>,
    windows: HashMap<u32, sdl2::video::Window>,
    renderers: HashMap<u32, sdl2::render::Canvas<sdl2::video::Window>>,
    next_window_id: u32,
}

impl ExecutionContext {
    fn new(config: &RuntimeConfig) -> Self {
        ExecutionContext {
            functions: HashMap::new(),
            call_stack: vec![StackFrame {
                return_pc: usize::MAX,
                out_register: 0,
                registers: HashMap::new(),
                loop_counters: HashMap::new(),
            }],
            tensors: HashMap::new(),
            cas: ValueStore::new(),
            loop_stack: Vec::new(),
            return_value: Value::Null,
            config: config.clone(),
            trace_buffer: VecDeque::with_capacity(50),
            pipes: HashMap::new(),
            next_pipe_id: 1,
            sdl_context: None,
            video_subsystem: None,
            event_pump: None,
            windows: HashMap::new(),
            renderers: HashMap::new(),
            next_window_id: 1,
        }
    }
    
    fn set_reg(&mut self, reg: u32, val: Value) {
        if let Some(frame) = self.call_stack.last_mut() {
            frame.registers.insert(reg, val);
        }
    }

    fn get_reg(&self, reg: u32) -> Result<&Value> {
        if let Some(frame) = self.call_stack.last() {
            frame.registers.get(&reg).ok_or_else(|| HlxError::ValidationFail {
                message: format!("Register {} not defined in current frame", reg),
            })
        } else {
            Err(HlxError::ValidationFail { message: "Empty call stack during register access".to_string() })
        }
    }

    #[allow(dead_code)]
    fn take_reg(&mut self, reg: u32) -> Result<Value> {
        if let Some(frame) = self.call_stack.last_mut() {
            frame.registers.remove(&reg).ok_or_else(|| HlxError::ValidationFail {
                message: format!("Register {} not defined in current frame", reg),
            })
        } else {
            Err(HlxError::ValidationFail { message: "Empty call stack during register access".to_string() })
        }
    }
    
    fn execute_instruction(&mut self, inst: &Instruction, pc: usize, backend: &mut dyn Backend) -> Result<ControlFlow> {
        if self.config.debug {
            // println!("  [{:4}] {:?}", pc, inst);
        }
        match inst {
            // === Control Flow ===
            Instruction::If { cond, then_block, else_block } => {
                let condition = self.get_bool(*cond)?;
                if condition {
                    return Ok(ControlFlow::Jump(*then_block));
                } else {
                    return Ok(ControlFlow::Jump(*else_block));
                }
            }

            Instruction::Jump { target } => {
                return Ok(ControlFlow::Jump(*target));
            }

            Instruction::Loop { cond, body, exit, max_iter } => {
                let condition = self.get_bool(*cond)?;
                
                // Deterministic Loop Bound Check
                let pc_u32 = pc as u32;
                
                // Maintain loop stack for break/continue
                // Check if this is the first entry or a re-entry from Jump back
                let is_first_entry = if let Some(&(entry, _)) = self.loop_stack.last() {
                    entry != pc_u32
                } else {
                    true
                };

                if is_first_entry {
                    self.loop_stack.push((pc_u32, *exit));
                }

                let count = if let Some(frame) = self.call_stack.last_mut() {
                    frame.loop_counters.entry(pc_u32).or_insert(0)
                } else {
                    return Err(HlxError::ValidationFail { message: "Loop instruction outside of frame".to_string() });
                };

                if *count >= *max_iter {
                    return Err(HlxError::ValidationFail {
                        message: format!("Deterministic Loop Bound exceeded (max: {})", max_iter),
                    });
                }
                *count += 1;

                if condition {
                    return Ok(ControlFlow::Jump(*body));
                } else {
                    // Reset counter when loop exits
                    if let Some(frame) = self.call_stack.last_mut() {
                        frame.loop_counters.insert(pc_u32, 0);
                    }
                    // Pop from loop stack
                    self.loop_stack.pop();
                    return Ok(ControlFlow::Jump(*exit));
                }
            }

            Instruction::FuncDef { .. } => {
                // Function definitions are skipped during execution
                return Ok(ControlFlow::Continue);
            }

            Instruction::Call { out, func, args } => {
                if self.config.debug {
                    // println!("    Call {} with {} args", func, args.len());
                }
                if let Some((start_pc, param_regs)) = self.functions.get(func) {
                    if args.len() != param_regs.len() {
                        return Err(HlxError::ValidationFail {
                            message: format!("Function {} expects {} args, got {}", func, param_regs.len(), args.len()),
                        });
                    }
                    
                    let mut arg_values = Vec::new();
                    for &arg_reg in args {
                        arg_values.push(self.get_reg(arg_reg)?.clone());
                    }
                    
                    let mut new_registers = HashMap::new();
                    for (val, &param_reg) in arg_values.into_iter().zip(param_regs.iter()) {
                        new_registers.insert(param_reg, val);
                    }
                    
                    self.call_stack.push(StackFrame {
                        return_pc: pc + 1,
                        out_register: *out,
                        registers: new_registers,
                        loop_counters: HashMap::new(),
                    });
                    
                    return Ok(ControlFlow::Jump(*start_pc));
                } else {
                    let res = self.call_builtin(func, args, backend)?;
                    self.set_reg(*out, res);
                    return Ok(ControlFlow::Continue);
                }
            }

            Instruction::Break => {
                if let Some(&(_, exit_pc)) = self.loop_stack.last() {
                    return Ok(ControlFlow::Jump(exit_pc));
                } else {
                    return Err(HlxError::ValidationFail { message: "Break instruction outside of loop".to_string() });
                }
            }
            
            Instruction::Continue => {
                if let Some(&(entry_pc, _)) = self.loop_stack.last() {
                    return Ok(ControlFlow::Jump(entry_pc));
                } else {
                    return Err(HlxError::ValidationFail { message: "Continue instruction outside of loop".to_string() });
                }
            }

            Instruction::Barrier { name } => {
                // In serial execution, barrier is a no-op
                // In parallel execution (speculation), this triggers hash verification
                if self.config.debug {
                    if let Some(barrier_name) = name {
                        println!("[BARRIER] '{}'", barrier_name);
                    } else {
                        println!("[BARRIER] (unnamed)");
                    }
                }
                return Ok(ControlFlow::Continue);
            }

            // === Memory Operations ===
            Instruction::Constant { out, val } => {
                self.set_reg(*out, val.clone());
            }

            Instruction::Move { out, src } => {
                let val = self.get_reg(*src)?.clone();
                self.set_reg(*out, val);
            }
            
            // === Arithmetic ===
            Instruction::Add { out, lhs, rhs } => {
                if let (Some(&h_a), Some(&h_b)) = (self.tensors.get(lhs), self.tensors.get(rhs)) {
                    // Tensor addition
                    let meta_a = backend.tensor_meta(h_a)?;
                    let h_out = backend.alloc_tensor(&meta_a.shape, meta_a.dtype)?;
                    backend.pointwise_add(h_a, h_b, h_out)?;
                    self.tensors.insert(*out, h_out);
                } else {
                    // Scalar addition
                    let a = self.get_reg(*lhs)?;
                    let b = self.get_reg(*rhs)?;
                    let result = backend.scalar_add(a, b)?;
                    self.set_reg(*out, result);
                }
            }
            
            Instruction::Sub { out, lhs, rhs } => {
                let a = self.get_reg(*lhs)?;
                let b = self.get_reg(*rhs)?;
                let result = backend.scalar_sub(a, b)?;
                self.set_reg(*out, result);
            }
            
            Instruction::Mul { out, lhs, rhs } => {
                let a = self.get_reg(*lhs)?;
                let b = self.get_reg(*rhs)?;
                let result = backend.scalar_mul(a, b)?;
                self.set_reg(*out, result);
            }
            
            Instruction::Div { out, lhs, rhs } => {
                let a = self.get_reg(*lhs)?;
                let b = self.get_reg(*rhs)?;
                let result = backend.scalar_div(a, b)?;
                self.set_reg(*out, result);
            }

            Instruction::Mod { out, lhs, rhs } => {
                let a = self.get_reg(*lhs)?;
                let b = self.get_reg(*rhs)?;
                let result = backend.scalar_mod(a, b)?;
                self.set_reg(*out, result);
            }

            Instruction::Neg { out, src } => {
                let a = self.get_reg(*src)?;
                let result = match a {
                    Value::Integer(i) => Value::Integer(-i),
                    Value::Float(f) => Value::float(-f)?,
                    _ => return Err(HlxError::TypeError {
                        expected: "numeric".to_string(),
                        got: a.type_name().to_string(),
                    }),
                };
                self.set_reg(*out, result);
            }
            
            // === Comparison ===
            Instruction::Eq { out, lhs, rhs } => {
                let a = self.get_reg(*lhs)?;
                let b = self.get_reg(*rhs)?;
                let result = backend.scalar_eq(a, b)?;
                self.set_reg(*out, result);
            }
            
            Instruction::Ne { out, lhs, rhs } => {
                let a = self.get_reg(*lhs)?;
                let b = self.get_reg(*rhs)?;
                let result = backend.scalar_ne(a, b)?;
                self.set_reg(*out, result);
            }
            
            Instruction::Lt { out, lhs, rhs } => {
                let a = self.get_reg(*lhs)?;
                let b = self.get_reg(*rhs)?;
                let result = backend.scalar_lt(a, b)?;
                self.set_reg(*out, result);
            }
            
            Instruction::Le { out, lhs, rhs } => {
                let a = self.get_reg(*lhs)?;
                let b = self.get_reg(*rhs)?;
                let result = backend.scalar_le(a, b)?;
                self.set_reg(*out, result);
            }
            
            Instruction::Gt { out, lhs, rhs } => {
                let a = self.get_reg(*lhs)?;
                let b = self.get_reg(*rhs)?;
                let result = backend.scalar_gt(a, b)?;
                self.set_reg(*out, result);
            }
            
            Instruction::Ge { out, lhs, rhs } => {
                let a = self.get_reg(*lhs)?;
                let b = self.get_reg(*rhs)?;
                let result = backend.scalar_ge(a, b)?;
                self.set_reg(*out, result);
            }
            
            // === Logical ===
            Instruction::And { out, lhs, rhs } => {
                let a = self.get_bool(*lhs)?;
                let b = self.get_bool(*rhs)?;
                self.set_reg(*out, Value::Boolean(a && b));
            }
            
            Instruction::Or { out, lhs, rhs } => {
                let a = self.get_bool(*lhs)?;
                let b = self.get_bool(*rhs)?;
                self.set_reg(*out, Value::Boolean(a || b));
            }
            
            Instruction::Not { out, src } => {
                let a = self.get_bool(*src)?;
                self.set_reg(*out, Value::Boolean(!a));
            }
            
            // === Control Flow ===
            Instruction::Return { val } => {
                let return_val = self.get_reg(*val)?.clone();
                
                if let Some(frame) = self.call_stack.pop() {
                    if frame.return_pc == usize::MAX {
                        // Main returned
                        self.return_value = return_val;
                        return Ok(ControlFlow::Jump(u32::MAX)); // Terminate
                    }
                    
                    // Write result to caller's register (caller is now on top)
                    self.set_reg(frame.out_register, return_val);
                    return Ok(ControlFlow::Jump(frame.return_pc as u32));
                } else {
                    return Err(HlxError::ValidationFail { message: "Return from empty stack".to_string() });
                }
            }
            
            Instruction::TypeOf { out, val } => {
                let v = self.get_reg(*val)?;
                self.set_reg(*out, Value::String(v.type_name().to_string()));
            }

            // === I/O ===
            Instruction::Print { val } => {
                let v = self.get_reg(*val)?;
                println!("{}", v);
            }
            
            // === Collection Operations ===
            Instruction::Index { out, container, index } => {
                let c = self.get_reg(*container)?;
                let i = self.get_reg(*index)?;
                let result = self.index_into(c, i)?;
                self.set_reg(*out, result);
            }
            
            Instruction::ArrayLen { out, array } => {
                let arr = self.get_reg(*array)?;
                match arr {
                    Value::Array(a) => self.set_reg(*out, Value::Integer(a.len() as i64)),
                    _ => return Err(HlxError::TypeError {
                        expected: "array".to_string(),
                        got: arr.type_name().to_string(),
                    }),
                }
            }
            
            Instruction::ArrayCreate { out, elements, element_type: _ } => {
                // Runtime is dynamically typed, so we ignore element_type hint
                let mut vals = Vector::new();
                for &reg in elements {
                    vals.push_back(self.get_reg(reg)?.clone());
                }
                self.set_reg(*out, Value::Array(vals));
            }
            
            Instruction::ArrayAlloc { out, size, element_type: _ } => {
                let size_val = match self.get_reg(*size)? {
                    Value::Integer(i) => *i as usize,
                    v => return Err(HlxError::TypeError { expected: "integer".to_string(), got: v.type_name().to_string() }),
                };
                let mut vals = Vector::new();
                for _ in 0..size_val {
                    vals.push_back(Value::Null);
                }
                self.set_reg(*out, Value::Array(vals));
            }
            
            Instruction::ObjectCreate { out, keys, values } => {
                let mut map = OrdMap::new();
                for (key, &reg) in keys.iter().zip(values.iter()) {
                    map.insert(key.clone(), self.get_reg(reg)?.clone());
                }
                
                self.set_reg(*out, Value::Object(map));
            }

            Instruction::Store { container, index, value } => {
                // Persistent Store:
                // Since we use im::Vector and im::OrdMap, update() returns a new version efficiently (O(log n)).
                // We update the register with this new version.
                
                let container_val = self.get_reg(*container)?;
                let index_val = self.get_reg(*index)?;
                let val = self.get_reg(*value)?.clone();
                
                let new_container_val = match (container_val, index_val) {
                    (Value::Array(arr), Value::Integer(idx)) => {
                        let i = *idx as usize;
                        if i < arr.len() {
                            Value::Array(arr.update(i, val))
                        } else if i == arr.len() {
                            let mut new_arr = arr.clone();
                            new_arr.push_back(val);
                            Value::Array(new_arr)
                        } else {
                            return Err(HlxError::IndexOutOfBounds { index: i, len: arr.len() });
                        }
                    }
                    (Value::Object(obj), Value::String(key)) => {
                        Value::Object(obj.update(key.clone(), val))
                    }
                    (v, _) => return Err(HlxError::TypeError { 
                        expected: "mutable container (array/object)".to_string(), 
                        got: v.type_name().to_string() 
                    }),
                };
                
                // Write back the mutated (persistent) value to the register
                self.set_reg(*container, new_container_val);
            }
            
            // === Latent Space Operations ===
            Instruction::Collapse { handle_out, val } => {
                let v = self.get_reg(*val)?.clone();
                let handle = self.cas.store(v)?;
                self.set_reg(*handle_out, Value::Handle(handle));
            }
            
            Instruction::Resolve { val_out, handle } => {
                let h = self.get_reg(*handle)?;
                match h {
                    Value::Handle(handle_str) => {
                        let v = self.cas.retrieve(handle_str)?;
                        self.set_reg(*val_out, v);
                    }
                    _ => return Err(HlxError::TypeError {
                        expected: "handle".to_string(),
                        got: h.type_name().to_string(),
                    }),
                }
            }
            
            Instruction::Snapshot { handle_out } => {
                // Create a full system snapshot (Digital Twin)
                let mut snap_obj = OrdMap::new();
                
                // 1. Instruction Pointer
                snap_obj.insert("pc".to_string(), Value::Integer(pc as i64));
                
                // 2. Call Stack
                let mut stack_arr = im::Vector::new();
                for frame in &self.call_stack {
                    let mut frame_obj = OrdMap::new();
                    frame_obj.insert("return_pc".to_string(), if frame.return_pc == usize::MAX { Value::Null } else { Value::Integer(frame.return_pc as i64) });
                    frame_obj.insert("out_reg".to_string(), Value::Integer(frame.out_register as i64));
                    
                    let mut regs = OrdMap::new();
                    for (k, v) in &frame.registers {
                        regs.insert(format!("r{}", k), v.clone());
                    }
                    frame_obj.insert("registers".to_string(), Value::Object(regs));
                    stack_arr.push_back(Value::Object(frame_obj));
                }
                snap_obj.insert("call_stack".to_string(), Value::Array(stack_arr));
                
                // 3. Loop Stack
                let mut loop_arr = im::Vector::new();
                for (entry, exit) in &self.loop_stack {
                    let mut l = im::Vector::new();
                    l.push_back(Value::Integer(*entry as i64));
                    l.push_back(Value::Integer(*exit as i64));
                    loop_arr.push_back(Value::Array(l));
                }
                snap_obj.insert("loop_stack".to_string(), Value::Array(loop_arr));

                // 4. Metadata
                snap_obj.insert("timestamp".to_string(), Value::Integer(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64));

                let snapshot = Value::Object(snap_obj);
                let handle = self.cas.store(snapshot)?;
                self.set_reg(*handle_out, Value::Handle(handle));
            }
            
            // === Tensor Operations (CPU fallback) ===
            Instruction::MatMul { out, lhs, rhs } => {
                if let (Some(&h_a), Some(&h_b)) = (self.tensors.get(lhs), self.tensors.get(rhs)) {
                    let meta_a = backend.tensor_meta(h_a)?;
                    let meta_b = backend.tensor_meta(h_b)?;
                    let out_shape = vec![meta_a.shape[0], meta_b.shape[1]];
                    let h_out = backend.alloc_tensor(&out_shape, meta_a.dtype)?;
                    backend.matmul(h_a, h_b, h_out)?;
                    self.tensors.insert(*out, h_out);
                } else {
                    // Scalar fallback
                    let a = self.get_reg(*lhs)?;
                    let b = self.get_reg(*rhs)?;
                    let result = backend.scalar_mul(a, b)?;
                    self.set_reg(*out, result);
                }
            }
            
            Instruction::Gelu { out, input } => {
                if let Some(&h_in) = self.tensors.get(input) {
                    let meta = backend.tensor_meta(h_in)?;
                    let h_out = backend.alloc_tensor(&meta.shape, meta.dtype)?;
                    backend.gelu(h_in, h_out)?;
                    self.tensors.insert(*out, h_out);
                } else {
                    let x = self.get_reg(*input)?;
                    let result = self.apply_gelu(x)?;
                    self.set_reg(*out, result);
                }
            }
            
            Instruction::Relu { out, input } => {
                if let Some(&h_in) = self.tensors.get(input) {
                    let meta = backend.tensor_meta(h_in)?;
                    let h_out = backend.alloc_tensor(&meta.shape, meta.dtype)?;
                    backend.relu(h_in, h_out)?;
                    self.tensors.insert(*out, h_out);
                } else {
                    let x = self.get_reg(*input)?;
                    let result = self.apply_relu(x)?;
                    self.set_reg(*out, result);
                }
            }
            
            Instruction::Softmax { out, input, dim } => {
                if let Some(&h_in) = self.tensors.get(input) {
                    let meta = backend.tensor_meta(h_in)?;
                    let h_out = backend.alloc_tensor(&meta.shape, meta.dtype)?;
                    backend.softmax(h_in, h_out, *dim)?;
                    self.tensors.insert(*out, h_out);
                } else {
                    let x = self.get_reg(*input)?.clone();
                    self.set_reg(*out, x);
                }
            }
            
            // === Function Calls handled above ===
            
            // Default: unimplemented instructions
            _ => {
                if self.config.debug {
                    tracing::warn!("Unimplemented instruction: {:?}", inst);
                }
            }
        }
        
        Ok(ControlFlow::Continue)
    }
    
    // === Helper Methods ===
    
    fn get_bool(&self, reg: u32) -> Result<bool> {
        let v = self.get_reg(reg)?;
        match v {
            Value::Boolean(b) => Ok(*b),
            _ => Err(HlxError::TypeError {
                expected: "boolean".to_string(),
                got: format!("{} (value: {})", v.type_name(), v),
            }),
        }
    }
    
    fn index_into(&self, container: &Value, index: &Value) -> Result<Value> {
        match (container, index) {
            (Value::Array(arr), Value::Integer(i)) => {
                let idx = *i as usize;
                arr.get(idx).cloned().ok_or_else(|| HlxError::ValidationFail {
                    message: format!("Index {} out of bounds", i),
                })
            }
            (Value::Object(obj), Value::String(key)) => {
                obj.get(key).cloned().ok_or_else(|| HlxError::ValidationFail {
                    message: format!("Key '{}' not found in object with keys: {:?}", key, obj.keys().collect::<Vec<_>>()),
                })
            }
            (Value::String(s), Value::Integer(i)) => {
                let idx = *i as usize;
                s.chars().nth(idx).map(|c| Value::String(c.to_string())).ok_or_else(|| HlxError::IndexOutOfBounds {
                    index: idx,
                    len: s.len(),
                })
            }
            _ => Err(HlxError::TypeError {
                expected: "indexable".to_string(),
                got: format!("{}[{}]", container.type_name(), index.type_name()),
            }),
        }
    }
    
    fn apply_gelu(&self, x: &Value) -> Result<Value> {
        match x {
            Value::Float(f) => {
                // GELU(x) = x * 0.5 * (1 + tanh(sqrt(2/π) * (x + 0.044715 * x³)))
                let sqrt_2_over_pi = 0.7978845608028654_f64;
                let coef = 0.044715_f64;
                let x3 = f * f * f;
                let inner = sqrt_2_over_pi * (f + coef * x3);
                let result = f * 0.5 * (1.0 + inner.tanh());
                Value::float(result)
            }
            Value::Integer(i) => {
                let f = *i as f64;
                self.apply_gelu(&Value::Float(f))
            }
            _ => Err(HlxError::TypeError {
                expected: "numeric".to_string(),
                got: x.type_name().to_string(),
            }),
        }
    }
    
    fn apply_relu(&self, x: &Value) -> Result<Value> {
        match x {
            Value::Float(f) => Value::float(f.max(0.0)),
            Value::Integer(i) => Ok(Value::Integer((*i).max(0))),
            _ => Err(HlxError::TypeError {
                expected: "numeric".to_string(),
                got: x.type_name().to_string(),
            }),
        }
    }
    
    fn call_builtin(&mut self, func: &str, args: &[u32], backend: &mut dyn Backend) -> Result<Value> {
        match func {
            "DEFAULT_MAX_ITER" => {
                Ok(Value::Integer(1000000))
            }
            "snapshot" => {
                // Return a full system snapshot (Digital Twin) handle
                let mut snap_obj = OrdMap::new();
                snap_obj.insert("pc".to_string(), Value::Integer(0)); // Builtin call context
                
                let mut stack_arr = im::Vector::new();
                for frame in &self.call_stack {
                    let mut frame_obj = OrdMap::new();
                    frame_obj.insert("return_pc".to_string(), if frame.return_pc == usize::MAX { Value::Null } else { Value::Integer(frame.return_pc as i64) });
                    frame_obj.insert("out_reg".to_string(), Value::Integer(frame.out_register as i64));
                    
                    let mut regs = OrdMap::new();
                    for (k, v) in &frame.registers {
                        regs.insert(format!("r{}", k), v.clone());
                    }
                    frame_obj.insert("registers".to_string(), Value::Object(regs));
                    stack_arr.push_back(Value::Object(frame_obj));
                }
                snap_obj.insert("call_stack".to_string(), Value::Array(stack_arr));
                
                let mut loop_arr = im::Vector::new();
                for (entry, exit) in &self.loop_stack {
                    let mut l = im::Vector::new();
                    l.push_back(Value::Integer(*entry as i64));
                    l.push_back(Value::Integer(*exit as i64));
                    loop_arr.push_back(Value::Array(l));
                }
                snap_obj.insert("loop_stack".to_string(), Value::Array(loop_arr));
                snap_obj.insert("timestamp".to_string(), Value::Integer(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64));

                let snapshot = Value::Object(snap_obj);
                let handle = self.cas.store(snapshot)?;
                Ok(Value::Handle(handle))
            }
            "export_trace" => {
                let mut trace_arr = im::Vector::new();
                for (pc, op_str) in &self.trace_buffer {
                    let mut obj = im::OrdMap::new();
                    obj.insert("pc".to_string(), Value::Integer(*pc as i64));
                    obj.insert("op".to_string(), Value::String(op_str.clone()));
                    trace_arr.push_back(Value::Object(obj));
                }
                Ok(Value::Array(trace_arr))
            }
            "print" => {
                for arg in args {
                    let v = self.get_reg(*arg)?;
                    println!("{}", v);
                }
                Ok(Value::Null)
            }
            "type" => {
                if args.len() != 1 {
                    return Err(HlxError::ValidationFail {
                        message: "type() takes exactly 1 argument".to_string(),
                    });
                }
                let v = self.get_reg(args[0])?;
                Ok(Value::String(v.type_name().to_string()))
            }
            "len" => {
                if args.len() != 1 {
                    return Err(HlxError::ValidationFail {
                        message: "len() takes exactly 1 argument".to_string(),
                    });
                }
                let v = self.get_reg(args[0])?;
                match v {
                    Value::Array(a) => Ok(Value::Integer(a.len() as i64)),
                    Value::String(s) => Ok(Value::Integer(s.len() as i64)),
                    Value::Object(o) => Ok(Value::Integer(o.len() as i64)),
                    _ => Err(HlxError::TypeError {
                        expected: "array, string, or object".to_string(),
                        got: v.type_name().to_string(),
                    }),
                }
            }
            "slice" => {
                if args.len() != 3 {
                    return Err(HlxError::ValidationFail {
                        message: "slice() takes exactly 3 arguments (array, start, len)".to_string(),
                    });
                }
                let arr_val = self.get_reg(args[0])?.clone();
                let start_val = self.get_reg(args[1])?.clone();
                let len_val = self.get_reg(args[2])?.clone();
                
                let start = match start_val { Value::Integer(i) => i as usize, _ => return Err(HlxError::TypeError { expected: "integer".to_string(), got: start_val.type_name().to_string() }) };
                let length = match len_val { Value::Integer(i) => i as usize, _ => return Err(HlxError::TypeError { expected: "integer".to_string(), got: len_val.type_name().to_string() }) };
                
                if let Value::Array(arr) = arr_val {
                    if start > arr.len() {
                        Ok(Value::Array(im::Vector::new()))
                    } else {
                        let (_, tail) = arr.split_at(start);
                        let effective_len = std::cmp::min(length, tail.len());
                        let (slice, _) = tail.split_at(effective_len);
                        Ok(Value::Array(slice))
                    }
                } else {
                     Err(HlxError::TypeError {
                        expected: "array".to_string(),
                        got: arr_val.type_name().to_string(),
                    })
                }
            }
            "append" => {
                if args.len() != 2 {
                    return Err(HlxError::ValidationFail {
                        message: "append() takes exactly 2 arguments (array, item)".to_string(),
                    });
                }
                let arr_reg = args[0];
                let item = self.get_reg(args[1])?.clone();
                let arr_val = self.get_reg(arr_reg)?.clone();
                
                if let Value::Array(arr) = arr_val {
                    let mut new_arr = arr.clone();
                    new_arr.push_back(item);
                    Ok(Value::Array(new_arr))
                } else {
                     Err(HlxError::TypeError {
                        expected: "array".to_string(),
                        got: arr_val.type_name().to_string(),
                    })
                }
            }
            "ord" => {
                if args.len() != 1 {
                    return Err(HlxError::ValidationFail {
                        message: "ord() takes exactly 1 argument".to_string(),
                    });
                }
                let v = self.get_reg(args[0])?;
                match v {
                    Value::String(s) => {
                        if s.len() != 1 {
                            return Err(HlxError::ValidationFail {
                                message: "ord() requires a single-character string".to_string(),
                            });
                        }
                        let code = s.chars().next().unwrap() as i64;
                        Ok(Value::Integer(code))
                    }
                    _ => Err(HlxError::TypeError {
                        expected: "string".to_string(),
                        got: v.type_name().to_string(),
                    }),
                }
            }
            "parse_int" => {
                if args.len() != 1 {
                    return Err(HlxError::ValidationFail {
                        message: "parse_int() takes exactly 1 argument".to_string(),
                    });
                }
                let v = self.get_reg(args[0])?;
                match v {
                    Value::String(s) => {
                        match s.parse::<i64>() {
                            Ok(n) => Ok(Value::Integer(n)),
                            Err(_) => Err(HlxError::ValidationFail {
                                message: format!("parse_int() cannot parse '{}' as integer", s),
                            }),
                        }
                    }
                    Value::Integer(n) => Ok(Value::Integer(*n)), // Already an integer
                    _ => Err(HlxError::TypeError {
                        expected: "string or integer".to_string(),
                        got: v.type_name().to_string(),
                    }),
                }
            }
            "to_int" => {
                if args.len() != 1 {
                    return Err(HlxError::ValidationFail { message: "to_int() takes exactly 1 argument".to_string() });
                }
                let v = self.get_reg(args[0])?;
                match v {
                    Value::Integer(i) => Ok(Value::Integer(*i)),
                    Value::Float(f) => Ok(Value::Integer(*f as i64)),
                    Value::String(s) => match s.parse::<i64>() {
                        Ok(i) => Ok(Value::Integer(i)),
                        Err(_) => Err(HlxError::ValidationFail { message: format!("to_int() cannot parse '{}' as integer", s) }),
                    },
                    _ => Err(HlxError::TypeError { expected: "numeric or string".to_string(), got: v.type_name().to_string() }),
                }
            }
            "to_string" => {
                if args.len() != 1 {
                    return Err(HlxError::ValidationFail { message: "to_string() takes exactly 1 argument".to_string() });
                }
                let v = self.get_reg(args[0])?;
                match v {
                    Value::Integer(i) => Ok(Value::String(i.to_string())),
                    Value::Float(f) => Ok(Value::String(f.to_string())),
                    Value::String(s) => Ok(Value::String(s.clone())),
                    Value::Boolean(b) => Ok(Value::String(b.to_string())),
                    Value::Null => Ok(Value::String("null".to_string())),
                    _ => Ok(Value::String(format!("[{}]", v.type_name()))),
                }
            }
            "to_float" => {
                if args.len() != 1 {
                    return Err(HlxError::ValidationFail { message: "to_float() takes exactly 1 argument".to_string() });
                }
                let v = self.get_reg(args[0])?;
                match v {
                    Value::Integer(i) => Ok(Value::Float(*i as f64)),
                    Value::Float(f) => Ok(Value::Float(*f)),
                    Value::String(s) => match s.parse::<f64>() {
                        Ok(f) => Ok(Value::Float(f)),
                        Err(_) => Err(HlxError::ValidationFail { message: format!("to_float() cannot parse '{}' as float", s) }),
                    },
                    _ => Err(HlxError::TypeError { expected: "numeric or string".to_string(), got: v.type_name().to_string() }),
                }
            }
            "floor" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "floor() takes 1 arg".to_string() }); }
                match self.get_reg(args[0])? {
                    Value::Float(f) => Ok(Value::Integer(f.floor() as i64)),
                    Value::Integer(i) => Ok(Value::Integer(*i)),
                    v => Err(HlxError::TypeError { expected: "numeric".to_string(), got: v.type_name().to_string() }),
                }
            }
            "ceil" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "ceil() takes 1 arg".to_string() }); }
                match self.get_reg(args[0])? {
                    Value::Float(f) => Ok(Value::Integer(f.ceil() as i64)),
                    Value::Integer(i) => Ok(Value::Integer(*i)),
                    v => Err(HlxError::TypeError { expected: "numeric".to_string(), got: v.type_name().to_string() }),
                }
            }
            "round" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "round() takes 1 arg".to_string() }); }
                match self.get_reg(args[0])? {
                    Value::Float(f) => Ok(Value::Integer(f.round() as i64)),
                    Value::Integer(i) => Ok(Value::Integer(*i)),
                    v => Err(HlxError::TypeError { expected: "numeric".to_string(), got: v.type_name().to_string() }),
                }
            }
            "sqrt" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "sqrt() takes 1 arg".to_string() }); }
                match self.get_reg(args[0])? {
                    Value::Float(f) => Ok(Value::Float(f.sqrt())),
                    Value::Integer(i) => Ok(Value::Float((*i as f64).sqrt())),
                    v => Err(HlxError::TypeError { expected: "numeric".to_string(), got: v.type_name().to_string() }),
                }
            }
            "sin" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "sin() takes 1 arg".to_string() }); }
                match self.get_reg(args[0])? {
                    Value::Float(f) => Ok(Value::Float(f.sin())),
                    Value::Integer(i) => Ok(Value::Float((*i as f64).sin())),
                    v => Err(HlxError::TypeError { expected: "numeric".to_string(), got: v.type_name().to_string() }),
                }
            }
            "cos" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "cos() takes 1 arg".to_string() }); }
                match self.get_reg(args[0])? {
                    Value::Float(f) => Ok(Value::Float(f.cos())),
                    Value::Integer(i) => Ok(Value::Float((*i as f64).cos())),
                    v => Err(HlxError::TypeError { expected: "numeric".to_string(), got: v.type_name().to_string() }),
                }
            }
            "tan" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "tan() takes 1 arg".to_string() }); }
                match self.get_reg(args[0])? {
                    Value::Float(f) => Ok(Value::Float(f.tan())),
                    Value::Integer(i) => Ok(Value::Float((*i as f64).tan())),
                    v => Err(HlxError::TypeError { expected: "numeric".to_string(), got: v.type_name().to_string() }),
                }
            }
            "log" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "log() takes 1 arg".to_string() }); }
                match self.get_reg(args[0])? {
                    Value::Float(f) => Ok(Value::Float(f.ln())),
                    Value::Integer(i) => Ok(Value::Float((*i as f64).ln())),
                    v => Err(HlxError::TypeError { expected: "numeric".to_string(), got: v.type_name().to_string() }),
                }
            }
            "exp" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "exp() takes 1 arg".to_string() }); }
                match self.get_reg(args[0])? {
                    Value::Float(f) => Ok(Value::Float(f.exp())),
                    Value::Integer(i) => Ok(Value::Float((*i as f64).exp())),
                    v => Err(HlxError::TypeError { expected: "numeric".to_string(), got: v.type_name().to_string() }),
                }
            }
            "random" => {
                let seed = if args.len() > 0 {
                    match self.get_reg(args[0])? {
                        Value::Integer(i) => *i as u64,
                        _ => 42,
                    }
                } else {
                    42
                };
                let res = ((seed.wrapping_mul(1103515245).wrapping_add(12345)) & 0x7fffffff) as f64 / 2147483647.0;
                Ok(Value::Float(res))
            }
            "has_key" => {
                if args.len() != 2 {
                    return Err(HlxError::ValidationFail {
                        message: "has_key() takes exactly 2 arguments (object, key)".to_string(),
                    });
                }
                let obj_val = self.get_reg(args[0])?;
                let key_val = self.get_reg(args[1])?;
                
                match obj_val {
                    Value::Object(map) => {
                        let key = match key_val {
                            Value::String(s) => s.clone(),
                            _ => return Err(HlxError::TypeError { expected: "string".to_string(), got: key_val.type_name().to_string() }),
                        };
                        Ok(Value::Boolean(map.contains_key(&key)))
                    }
                    _ => Err(HlxError::TypeError {
                        expected: "object".to_string(),
                        got: obj_val.type_name().to_string(),
                    }),
                }
            }
            "concat" => {
                if args.len() != 2 { return Err(HlxError::ValidationFail { message: "concat() takes 2 args".to_string() }); }
                let lhs = self.get_reg(args[0])?.to_string();
                let rhs = self.get_reg(args[1])?.to_string();
                Ok(Value::String(format!("{}{}", lhs, rhs)))
            }
            "strlen" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "strlen() takes 1 arg".to_string() }); }
                match self.get_reg(args[0])? {
                    Value::String(s) => Ok(Value::Integer(s.len() as i64)),
                    v => Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }),
                }
            }
            "substring" => {
                if args.len() != 3 { return Err(HlxError::ValidationFail { message: "substring(s, start, len) takes 3 args".to_string() }); }
                let s = match self.get_reg(args[0])? { Value::String(s) => s.as_str(), v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }) };
                let start = match self.get_reg(args[1])? { Value::Integer(i) => *i as usize, v => return Err(HlxError::TypeError { expected: "integer".to_string(), got: v.type_name().to_string() }) };
                let length = match self.get_reg(args[2])? { Value::Integer(i) => *i as usize, v => return Err(HlxError::TypeError { expected: "integer".to_string(), got: v.type_name().to_string() }) };
                
                let end = (start + length).min(s.len());
                if start > s.len() { return Ok(Value::String("".to_string())); }
                Ok(Value::String(s[start..end].to_string()))
            }
            "index_of" => {
                if args.len() != 2 { return Err(HlxError::ValidationFail { message: "index_of(haystack, needle) takes 2 args".to_string() }); }
                let haystack = match self.get_reg(args[0])? { Value::String(s) => s.as_str(), v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }) };
                let needle = match self.get_reg(args[1])? { Value::String(s) => s.as_str(), v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }) };
                match haystack.find(needle) {
                    Some(i) => Ok(Value::Integer(i as i64)),
                    None => Ok(Value::Integer(-1)),
                }
            }
            "to_upper" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "to_upper() takes 1 arg".to_string() }); }
                let s = self.get_reg(args[0])?.to_string();
                Ok(Value::String(s.to_uppercase()))
            }
            "to_lower" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "to_lower() takes 1 arg".to_string() }); }
                let s = self.get_reg(args[0])?.to_string();
                Ok(Value::String(s.to_lowercase()))
            }
            "trim" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "trim() takes 1 arg".to_string() }); }
                let s = match self.get_reg(args[0])? { Value::String(s) => s.trim().to_string(), _ => self.get_reg(args[0])?.to_string().trim().to_string() };
                Ok(Value::String(s))
            }
            "starts_with" => {
                if args.len() != 2 { return Err(HlxError::ValidationFail { message: "starts_with(s, prefix) takes 2 args".to_string() }); }
                let s = self.get_reg(args[0])?.to_string();
                let prefix = self.get_reg(args[1])?.to_string();
                Ok(Value::Boolean(s.starts_with(&prefix)))
            }
            "ends_with" => {
                if args.len() != 2 { return Err(HlxError::ValidationFail { message: "ends_with(s, suffix) takes 2 args".to_string() }); }
                let s = self.get_reg(args[0])?.to_string();
                let suffix = self.get_reg(args[1])?.to_string();
                Ok(Value::Boolean(s.ends_with(&suffix)))
            }
            "json_parse" => {
                if args.len() != 1 {
                    return Err(HlxError::ValidationFail {
                        message: "json_parse() takes exactly 1 argument (string)".to_string(),
                    });
                }
                let json_str = match self.get_reg(args[0])? {
                    Value::String(s) => s.as_str(),
                    v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }),
                };
                let sjv: serde_json::Value = serde_json::from_str(json_str).map_err(|e| HlxError::BackendError {
                    message: format!("JSON parse error: {}", e),
                })?;
                Ok(Value::from_json(sjv)?)
            }
            "json_stringify" => {
                if args.len() != 1 {
                    return Err(HlxError::ValidationFail {
                        message: "json_stringify() takes exactly 1 argument (value)".to_string(),
                    });
                }
                let val = self.get_reg(args[0])?;
                let sjv = val.to_json()?;
                let s = serde_json::to_string(&sjv).map_err(|e| HlxError::BackendError {
                    message: format!("JSON stringify error: {}", e),
                })?;
                Ok(Value::String(s))
            }
            "http_request" => {
                // http_request(method, url, body, headers)
                if args.len() < 2 {
                    return Err(HlxError::ValidationFail {
                        message: "http_request() takes at least 2 arguments (method, url)".to_string(),
                    });
                }
                let method_str = match self.get_reg(args[0])? {
                    Value::String(s) => s.to_uppercase(),
                    v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }),
                };
                let url = match self.get_reg(args[1])? {
                    Value::String(s) => s.as_str(),
                    v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }),
                };
                
                let client = reqwest::blocking::Client::new();
                let method = match method_str.as_str() {
                    "GET" => reqwest::Method::GET,
                    "POST" => reqwest::Method::POST,
                    "PUT" => reqwest::Method::PUT,
                    "DELETE" => reqwest::Method::DELETE,
                    _ => return Err(HlxError::ValidationFail { message: format!("Invalid HTTP method: {}", method_str) }),
                };
                
                let mut rb = client.request(method, url);
                
                if args.len() >= 3 {
                    let body_val = self.get_reg(args[2])?;
                    match body_val {
                        Value::String(s) => { rb = rb.body(s.clone()); }
                        Value::Null => {}
                        _ => {
                            let sjv = body_val.to_json()?;
                            let json_body = serde_json::to_string(&sjv).map_err(|e| HlxError::BackendError {
                                message: format!("JSON stringify error for body: {}", e),
                            })?;
                            rb = rb.body(json_body).header("Content-Type", "application/json");
                        }
                    }
                }
                
                if args.len() >= 4 {
                    let headers_val = self.get_reg(args[3])?;
                    if let Value::Object(headers) = headers_val {
                        for (k, v) in headers {
                            if let Value::String(vs) = v {
                                rb = rb.header(k, vs);
                            }
                        }
                    }
                }
                
                let resp = rb.send().map_err(|e| HlxError::BackendError {
                    message: format!("HTTP request failed: {}", e),
                })?;
                
                let status = resp.status().as_u16() as i64;
                let text = resp.text().map_err(|e| HlxError::BackendError {
                    message: format!("Failed to read HTTP response: {}", e),
                })?;
                
                let mut res_obj = im::OrdMap::new();
                res_obj.insert("status".to_string(), Value::Integer(status));
                res_obj.insert("body".to_string(), Value::String(text));
                
                Ok(Value::Object(res_obj))
            }
            "read_file" => {
                if args.len() != 1 {
                    return Err(HlxError::ValidationFail {
                        message: "read_file() takes exactly 1 argument (path)".to_string(),
                    });
                }
                let path_val = self.get_reg(args[0])?;
                let path = match path_val {
                    Value::String(s) => s.as_str(),
                    _ => return Err(HlxError::TypeError { 
                        expected: "string".to_string(), 
                        got: path_val.type_name().to_string() 
                    }),
                };
                let content = std::fs::read_to_string(path).map_err(|e| HlxError::BackendError { 
                    message: format!("Failed to read file {}: {}", path, e) 
                })?;
                Ok(Value::String(content))
            }
            "write_file" => {
                if args.len() != 2 {
                    return Err(HlxError::ValidationFail {
                        message: "write_file() takes exactly 2 arguments (path, data)".to_string(),
                    });
                }
                let path = match self.get_reg(args[0])? {
                    Value::String(s) => s.clone(),
                    v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }),
                };
                let data = match self.get_reg(args[1])? {
                    Value::String(s) => s.clone(),
                    v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }),
                };
                
                std::fs::write(&path, data).map_err(|e| HlxError::BackendError { 
                    message: format!("Failed to write file {}: {}", path, e) 
                })?;
                Ok(Value::Boolean(true))
            }
            "file_exists" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "file_exists() takes 1 arg".to_string() }); }
                let path = self.get_reg(args[0])?.to_string();
                Ok(Value::Boolean(std::path::Path::new(&path).exists()))
            }
            "delete_file" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "delete_file() takes 1 arg".to_string() }); }
                let path = self.get_reg(args[0])?.to_string();
                std::fs::remove_file(path).map_err(|e| HlxError::BackendError { message: e.to_string() })?;
                Ok(Value::Boolean(true))
            }
            "list_files" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "list_files() takes 1 arg".to_string() }); }
                let path = self.get_reg(args[0])?.to_string();
                let entries = std::fs::read_dir(path).map_err(|e| HlxError::BackendError { message: e.to_string() })?;
                let mut files = im::Vector::new();
                for entry in entries {
                    if let Ok(entry) = entry {
                        files.push_back(Value::String(entry.file_name().to_string_lossy().to_string()));
                    }
                }
                Ok(Value::Array(files))
            }
            "create_dir" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "create_dir() takes 1 arg".to_string() }); }
                let path = self.get_reg(args[0])?.to_string();
                std::fs::create_dir_all(path).map_err(|e| HlxError::BackendError { message: e.to_string() })?;
                Ok(Value::Boolean(true))
            }
            "arr_pop" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "arr_pop() takes 1 arg".to_string() }); }
                let val = self.get_reg(args[0])?;
                match val {
                    Value::Array(arr) => {
                        let mut new_arr = arr.clone();
                        new_arr.pop_back();
                        Ok(Value::Array(new_arr))
                    }
                    _ => Err(HlxError::TypeError { expected: "array".to_string(), got: val.type_name().to_string() }),
                }
            }
            "arr_slice" => {
                if args.len() != 3 { return Err(HlxError::ValidationFail { message: "arr_slice(arr, start, len) takes 3 args".to_string() }); }
                let val = self.get_reg(args[0])?;
                let start = match self.get_reg(args[1])? { Value::Integer(i) => *i as usize, v => return Err(HlxError::TypeError { expected: "integer".to_string(), got: v.type_name().to_string() }) };
                let length = match self.get_reg(args[2])? { Value::Integer(i) => *i as usize, v => return Err(HlxError::TypeError { expected: "integer".to_string(), got: v.type_name().to_string() }) };
                
                match val {
                    Value::Array(arr) => {
                        let end = (start + length).min(arr.len());
                        if start > arr.len() { return Ok(Value::Array(im::Vector::new())); }
                        Ok(Value::Array(arr.clone().slice(start..end)))
                    }
                    _ => Err(HlxError::TypeError { expected: "array".to_string(), got: val.type_name().to_string() }),
                }
            }
            "arr_concat" => {
                if args.len() != 2 { return Err(HlxError::ValidationFail { message: "arr_concat() takes 2 args".to_string() }); }
                let lhs = self.get_reg(args[0])?;
                let rhs = self.get_reg(args[1])?;
                match (lhs, rhs) {
                    (Value::Array(a), Value::Array(b)) => {
                        let mut res = a.clone();
                        res.append(b.clone());
                        Ok(Value::Array(res))
                    }
                    _ => Err(HlxError::TypeError { expected: "two arrays".to_string(), got: format!("{}, {}", lhs.type_name(), rhs.type_name()) }),
                }
            }
            "read_json" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "read_json() takes 1 arg".to_string() }); }
                let path = self.get_reg(args[0])?.to_string();
                let content = std::fs::read_to_string(path).map_err(|e| HlxError::BackendError { message: e.to_string() })?;
                let sjv: serde_json::Value = serde_json::from_str(&content).map_err(|e| HlxError::BackendError { message: e.to_string() })?;
                Ok(Value::from_json(sjv)?)
            }
            "write_json" => {
                if args.len() != 2 { return Err(HlxError::ValidationFail { message: "write_json() takes 2 args".to_string() }); }
                let path = self.get_reg(args[0])?.to_string();
                let val = self.get_reg(args[1])?;
                let sjv = val.to_json()?;
                let content = serde_json::to_string_pretty(&sjv).map_err(|e| HlxError::BackendError { message: e.to_string() })?;
                std::fs::write(path, content).map_err(|e| HlxError::BackendError { message: e.to_string() })?;
                Ok(Value::Boolean(true))
            }
            "write_snapshot" => {
                // write_snapshot(path, handle)
                if args.len() != 2 {
                    return Err(HlxError::ValidationFail {
                        message: "write_snapshot() takes exactly 2 arguments (path, handle)".to_string(),
                    });
                }
                let path = match self.get_reg(args[0])? {
                    Value::String(s) => s.clone(),
                    v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }),
                };
                let handle = match self.get_reg(args[1])? {
                    Value::Handle(h) => h.clone(),
                    v => return Err(HlxError::TypeError { expected: "handle".to_string(), got: v.type_name().to_string() }),
                };
                
                let snapshot_val = self.cas.retrieve(&handle)?;
                let sjv = snapshot_val.to_json()?;
                let json_str = serde_json::to_string_pretty(&sjv).map_err(|e| HlxError::BackendError {
                    message: format!("Failed to serialize snapshot: {}", e),
                })?;
                
                std::fs::write(&path, json_str).map_err(|e| HlxError::BackendError {
                    message: format!("Failed to write snapshot to {}: {}", path, e),
                })?;
                
                Ok(Value::Boolean(true))
            }
            "SDL_Init" => {
                if self.sdl_context.is_none() {
                    let sdl_context = sdl2::init().map_err(|e| HlxError::BackendError { message: e })?;
                    let video_subsystem = sdl_context.video().map_err(|e| HlxError::BackendError { message: e })?;
                    let event_pump = sdl_context.event_pump().map_err(|e| HlxError::BackendError { message: e })?;
                    self.sdl_context = Some(sdl_context);
                    self.video_subsystem = Some(video_subsystem);
                    self.event_pump = Some(event_pump);
                }
                Ok(Value::Integer(0))
            }
            "SDL_CreateWindow" => {
                // (title, x, y, w, h, flags)
                // We'll ignore x, y, flags detail for simplicity or map them
                let title = self.get_reg(args[0])?.to_string();
                let width = match self.get_reg(args[3])? { Value::Integer(i) => *i as u32, _ => 800 };
                let height = match self.get_reg(args[4])? { Value::Integer(i) => *i as u32, _ => 600 };
                
                let video_subsystem = self.video_subsystem.as_ref().ok_or(HlxError::BackendError { message: "SDL not initialized".to_string() })?;
                let window = video_subsystem.window(&title, width, height)
                    .position_centered()
                    .build()
                    .map_err(|e| HlxError::BackendError { message: e.to_string() })?;
                
                let id = self.next_window_id;
                self.next_window_id += 1;
                self.windows.insert(id, window);
                Ok(Value::Integer(id as i64))
            }
            "SDL_CreateRenderer" => {
                let window_id = match self.get_reg(args[0])? { Value::Integer(i) => *i as u32, _ => 0 };
                let window = self.windows.remove(&window_id).ok_or(HlxError::ValidationFail { message: "Invalid window ID".to_string() })?;
                let canvas = window.into_canvas().build().map_err(|e| HlxError::BackendError { message: e.to_string() })?;
                self.renderers.insert(window_id, canvas); // Re-use ID for renderer-canvas map
                Ok(Value::Integer(window_id as i64))
            }
            "SDL_SetRenderDrawColor" => {
                let id = match self.get_reg(args[0])? { Value::Integer(i) => *i as u32, _ => 0 };
                let r = match self.get_reg(args[1])? { Value::Integer(i) => *i as u8, _ => 0 };
                let g = match self.get_reg(args[2])? { Value::Integer(i) => *i as u8, _ => 0 };
                let b = match self.get_reg(args[3])? { Value::Integer(i) => *i as u8, _ => 0 };
                let a = match self.get_reg(args[4])? { Value::Integer(i) => *i as u8, _ => 255 };
                
                if let Some(canvas) = self.renderers.get_mut(&id) {
                    canvas.set_draw_color(sdl2::pixels::Color::RGBA(r, g, b, a));
                }
                Ok(Value::Integer(0))
            }
            "SDL_RenderClear" => {
                let id = match self.get_reg(args[0])? { Value::Integer(i) => *i as u32, _ => 0 };
                if let Some(canvas) = self.renderers.get_mut(&id) {
                    canvas.clear();
                }
                Ok(Value::Integer(0))
            }
            "SDL_RenderPresent" => {
                let id = match self.get_reg(args[0])? { Value::Integer(i) => *i as u32, _ => 0 };
                if let Some(canvas) = self.renderers.get_mut(&id) {
                    canvas.present();
                }
                Ok(Value::Integer(0))
            }
            "SDL_PollEvent" => {
                let event_pump = self.event_pump.as_mut().ok_or(HlxError::BackendError { message: "SDL not initialized".to_string() })?;
                if let Some(event) = event_pump.poll_event() {
                     // For now, return 1 if event exists, 0 otherwise
                     // To make it usable, we should return event type or struct
                     match event {
                        sdl2::event::Event::Quit {..} => Ok(Value::Integer(256)), // Quit
                        _ => Ok(Value::Integer(1)),
                     }
                } else {
                    Ok(Value::Integer(0))
                }
            }
            "SDL_Delay" => {
                let ms = match self.get_reg(args[0])? { Value::Integer(i) => *i as u32, _ => 10 };
                std::thread::sleep(std::time::Duration::from_millis(ms as u64));
                Ok(Value::Integer(0))
            }
            "SDL_Quit" => {
                self.renderers.clear();
                self.windows.clear();
                self.video_subsystem = None;
                self.sdl_context = None;
                Ok(Value::Integer(0))
            }
            "SDL_DestroyWindow" => {
                 let id = match self.get_reg(args[0])? { Value::Integer(i) => *i as u32, _ => 0 };
                 self.renderers.remove(&id);
                 self.windows.remove(&id);
                 Ok(Value::Integer(0))
            }
            "malloc" => {
                // Emulate malloc by allocating a byte array
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "malloc() takes 1 arg".to_string() }); }
                let _size = match self.get_reg(args[0])? { Value::Integer(i) => *i as usize, _ => 0 };
                // In interpreter, we can't return a raw pointer easily that SDL understands if SDL expects *real* pointers.
                // However, our SDL builtins don't use the pointer! They ignore it or handle it internally.
                // Wait, SDL_PollEvent(event_ptr).
                // If I implemented SDL_PollEvent to use internal event pump, it ignores the argument!
                // So I can just return a dummy integer.
                Ok(Value::Integer(0xDEADBEEF))
            }
            "free" => {
                Ok(Value::Integer(0))
            }
            "pipe_open" => {
                if args.len() != 1 {
                    return Err(HlxError::ValidationFail {
                        message: "pipe_open() takes exactly 1 argument (command_string)".to_string(),
                    });
                }
                let cmd_str = match self.get_reg(args[0])? {
                    Value::String(s) => s.clone(),
                    v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }),
                };
                
                let child = if cfg!(target_os = "windows") {
                    Command::new("cmd").args(["/C", &cmd_str]).stdin(Stdio::piped()).spawn()
                } else {
                    Command::new("sh").args(["-c", &cmd_str]).stdin(Stdio::piped()).spawn()
                }.map_err(|e| HlxError::BackendError { message: format!("Failed to spawn pipe: {}", e) })?;
                
                let id = self.next_pipe_id;
                self.next_pipe_id += 1;
                self.pipes.insert(id, child);
                
                Ok(Value::Integer(id as i64))
            }
            "pipe_write" => {
                if args.len() != 2 {
                    return Err(HlxError::ValidationFail {
                        message: "pipe_write() takes 2 arguments (pipe_id, tensor_handle)".to_string(),
                    });
                }
                let pipe_id = match self.get_reg(args[0])? {
                    Value::Integer(i) => *i as u32,
                    v => return Err(HlxError::TypeError { expected: "integer".to_string(), got: v.type_name().to_string() }),
                };
                let handle = match self.get_reg(args[1])? {
                    Value::Handle(h) => h.clone(),
                    v => return Err(HlxError::TypeError { expected: "handle".to_string(), got: v.type_name().to_string() }),
                };
                
                // Get raw tensor data
                let tensor_handle = crate::backend::TensorHandle(handle.parse::<u64>().map_err(|_| HlxError::ValidationFail { message: "Invalid tensor handle".to_string() })?);
                let data = backend.read_tensor(tensor_handle)?;
                let i32_data: &[i32] = bytemuck::cast_slice(&data);
                
                // Convert i32 back to u8 for the pipe
                let mut u8_data = Vec::with_capacity(i32_data.len());
                for &val in i32_data {
                    u8_data.push(val.clamp(0, 255) as u8);
                }
                
                if let Some(child) = self.pipes.get_mut(&pipe_id) {
                    if let Some(stdin) = child.stdin.as_mut() {
                        stdin.write_all(&u8_data).map_err(|e| HlxError::BackendError { message: format!("Pipe write failed: {}", e) })?;
                        stdin.flush().ok();
                    }
                }
                
                Ok(Value::Boolean(true))
            }
            "pipe_close" => {
                if args.len() != 1 {
                    return Err(HlxError::ValidationFail {
                        message: "pipe_close() takes 1 argument (pipe_id)".to_string(),
                    });
                }
                let pipe_id = match self.get_reg(args[0])? {
                    Value::Integer(i) => *i as u32,
                    v => return Err(HlxError::TypeError { expected: "integer".to_string(), got: v.type_name().to_string() }),
                };
                
                if let Some(mut child) = self.pipes.remove(&pipe_id) {
                    child.kill().ok();
                }
                
                Ok(Value::Boolean(true))
            }
            "capture_screen" => {
                if args.len() != 1 {
                    return Err(HlxError::ValidationFail {
                        message: "capture_screen() takes 1 argument (monitor_index)".to_string(),
                    });
                }
                let monitor_idx = match self.get_reg(args[0])? {
                    Value::Integer(i) => *i as usize,
                    v => return Err(HlxError::TypeError { expected: "integer".to_string(), got: v.type_name().to_string() }),
                };
                
                let monitors = Monitor::all().map_err(|e| HlxError::BackendError { message: format!("Failed to list monitors: {}", e) })?;
                let monitor = monitors.get(monitor_idx).ok_or_else(|| HlxError::ValidationFail { message: format!("Monitor index {} not found", monitor_idx) })?;
                
                let image = monitor.capture_image().map_err(|e| HlxError::BackendError { message: format!("Capture failed: {}", e) })?;
                let width = image.width() as usize;
                let height = image.height() as usize;
                let rgba_data = image.into_raw();
                
                // Convert u8 to i32 for the tensor
                let mut i32_data = Vec::with_capacity(rgba_data.len());
                for b in rgba_data {
                    i32_data.push(b as i32);
                }
                let raw_bytes: &[u8] = bytemuck::cast_slice(&i32_data);
                
                // Allocate tensor in backend (H, W, 4)
                let shape = vec![height, width, 4];
                let h_tensor = backend.alloc_tensor(&shape, crate::backend::DType::I32)?;
                backend.write_tensor(h_tensor, raw_bytes)?;
                
                Ok(Value::Handle(h_tensor.0.to_string()))
            }
            "sleep" => {
                if args.len() != 1 {
                    return Err(HlxError::ValidationFail {
                        message: "sleep() takes 1 argument (ms)".to_string(),
                    });
                }
                let ms = match self.get_reg(args[0])? {
                    Value::Integer(i) => *i as u64,
                    v => return Err(HlxError::TypeError { expected: "integer".to_string(), got: v.type_name().to_string() }),
                };
                std::thread::sleep(std::time::Duration::from_millis(ms));
                Ok(Value::Null)
            }
            "native_tokenize" => {
                // Native tokenizer - runs in Rust to bypass O(n²) interpreted string ops
                if args.len() != 1 {
                    return Err(HlxError::ValidationFail {
                        message: "native_tokenize() takes exactly 1 argument (source)".to_string(),
                    });
                }
                let source = match self.get_reg(args[0])? {
                    Value::String(s) => s.clone(),
                    v => return Err(HlxError::TypeError {
                        expected: "string".to_string(),
                        got: v.type_name().to_string(),
                    }),
                };

                let tokens = native_tokenize(&source)?;
                Ok(tokens)
            }
            "split" => {
                if args.len() != 2 { return Err(HlxError::ValidationFail { message: "split(string, delimiter) takes 2 args".to_string() }); }
                let s = match self.get_reg(args[0])? { Value::String(s) => s.as_str(), v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }) };
                let delimiter = match self.get_reg(args[1])? { Value::String(s) => s.as_str(), v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }) };
                let parts: im::Vector<Value> = s.split(delimiter).map(|p| Value::String(p.to_string())).collect();
                Ok(Value::Array(parts))
            }
            "join" => {
                if args.len() != 2 { return Err(HlxError::ValidationFail { message: "join(array, delimiter) takes 2 args".to_string() }); }
                let arr = match self.get_reg(args[0])? { Value::Array(a) => a, v => return Err(HlxError::TypeError { expected: "array".to_string(), got: v.type_name().to_string() }) };
                let delimiter = match self.get_reg(args[1])? { Value::String(s) => s.as_str(), v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }) };
                let strings: Vec<String> = arr.iter().map(|v| v.to_string()).collect();
                Ok(Value::String(strings.join(delimiter)))
            }
            "replace" => {
                if args.len() != 3 { return Err(HlxError::ValidationFail { message: "replace(string, from, to) takes 3 args".to_string() }); }
                let s = match self.get_reg(args[0])? { Value::String(s) => s.as_str(), v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }) };
                let from = match self.get_reg(args[1])? { Value::String(s) => s.as_str(), v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }) };
                let to = match self.get_reg(args[2])? { Value::String(s) => s.as_str(), v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }) };
                Ok(Value::String(s.replace(from, to)))
            }
            "replace_first" => {
                if args.len() != 3 { return Err(HlxError::ValidationFail { message: "replace_first(string, from, to) takes 3 args".to_string() }); }
                let s = match self.get_reg(args[0])? { Value::String(s) => s.as_str(), v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }) };
                let from = match self.get_reg(args[1])? { Value::String(s) => s.as_str(), v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }) };
                let to = match self.get_reg(args[2])? { Value::String(s) => s.as_str(), v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }) };
                Ok(Value::String(s.replacen(from, to, 1)))
            }
            "pad_left" => {
                if args.len() != 3 { return Err(HlxError::ValidationFail { message: "pad_left(string, width, pad_char) takes 3 args".to_string() }); }
                let s = match self.get_reg(args[0])? { Value::String(s) => s.as_str(), v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }) };
                let width = match self.get_reg(args[1])? { Value::Integer(i) => *i as usize, v => return Err(HlxError::TypeError { expected: "integer".to_string(), got: v.type_name().to_string() }) };
                let pad = match self.get_reg(args[2])? { Value::String(s) => s.chars().next().unwrap_or(' '), v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }) };
                if s.len() >= width {
                    Ok(Value::String(s.to_string()))
                } else {
                    Ok(Value::String(format!("{:>width$}", s, width = width).replace(' ', &pad.to_string())))
                }
            }
            "pad_right" => {
                if args.len() != 3 { return Err(HlxError::ValidationFail { message: "pad_right(string, width, pad_char) takes 3 args".to_string() }); }
                let s = match self.get_reg(args[0])? { Value::String(s) => s.as_str(), v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }) };
                let width = match self.get_reg(args[1])? { Value::Integer(i) => *i as usize, v => return Err(HlxError::TypeError { expected: "integer".to_string(), got: v.type_name().to_string() }) };
                let pad = match self.get_reg(args[2])? { Value::String(s) => s.chars().next().unwrap_or(' '), v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }) };
                if s.len() >= width {
                    Ok(Value::String(s.to_string()))
                } else {
                    Ok(Value::String(format!("{:<width$}", s, width = width).replace(' ', &pad.to_string())))
                }
            }
            "repeat" => {
                if args.len() != 2 { return Err(HlxError::ValidationFail { message: "repeat(string, count) takes 2 args".to_string() }); }
                let s = match self.get_reg(args[0])? { Value::String(s) => s.as_str(), v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }) };
                let count = match self.get_reg(args[1])? { Value::Integer(i) => *i as usize, v => return Err(HlxError::TypeError { expected: "integer".to_string(), got: v.type_name().to_string() }) };
                Ok(Value::String(s.repeat(count)))
            }
            "reverse_str" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "reverse_str() takes 1 arg".to_string() }); }
                let s = match self.get_reg(args[0])? { Value::String(s) => s.as_str(), v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }) };
                Ok(Value::String(s.chars().rev().collect()))
            }
            "char_at" => {
                if args.len() != 2 { return Err(HlxError::ValidationFail { message: "char_at(string, index) takes 2 args".to_string() }); }
                let s = match self.get_reg(args[0])? { Value::String(s) => s.as_str(), v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }) };
                let index = match self.get_reg(args[1])? { Value::Integer(i) => *i as usize, v => return Err(HlxError::TypeError { expected: "integer".to_string(), got: v.type_name().to_string() }) };
                match s.chars().nth(index) {
                    Some(c) => Ok(Value::String(c.to_string())),
                    None => Err(HlxError::ValidationFail { message: format!("Index {} out of bounds for string of length {}", index, s.len()) }),
                }
            }
            "contains" => {
                if args.len() != 2 { return Err(HlxError::ValidationFail { message: "contains(haystack, needle) takes 2 args".to_string() }); }
                let haystack = match self.get_reg(args[0])? { Value::String(s) => s.as_str(), v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }) };
                let needle = match self.get_reg(args[1])? { Value::String(s) => s.as_str(), v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }) };
                Ok(Value::Boolean(haystack.contains(needle)))
            }
            "is_alpha" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "is_alpha() takes 1 arg".to_string() }); }
                let s = match self.get_reg(args[0])? { Value::String(s) => s.as_str(), v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }) };
                Ok(Value::Boolean(!s.is_empty() && s.chars().all(|c| c.is_alphabetic())))
            }
            "is_numeric" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "is_numeric() takes 1 arg".to_string() }); }
                let s = match self.get_reg(args[0])? { Value::String(s) => s.as_str(), v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }) };
                Ok(Value::Boolean(!s.is_empty() && s.chars().all(|c| c.is_numeric())))
            }
            "is_alphanumeric" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "is_alphanumeric() takes 1 arg".to_string() }); }
                let s = match self.get_reg(args[0])? { Value::String(s) => s.as_str(), v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }) };
                Ok(Value::Boolean(!s.is_empty() && s.chars().all(|c| c.is_alphanumeric())))
            }
            "format" => {
                // Simple format implementation - takes format string and variadic args
                if args.is_empty() { return Err(HlxError::ValidationFail { message: "format() requires at least 1 argument".to_string() }); }
                let fmt = match self.get_reg(args[0])? { Value::String(s) => s.as_str(), v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }) };
                let mut result = fmt.to_string();
                for (i, arg_reg) in args.iter().skip(1).enumerate() {
                    let val = self.get_reg(*arg_reg)?;
                    result = result.replace(&format!("{{{}}}", i), &val.to_string());
                }
                Ok(Value::String(result))
            }
            "match_regex" => {
                if args.len() != 2 { return Err(HlxError::ValidationFail { message: "match_regex(string, pattern) takes 2 args".to_string() }); }
                let s = match self.get_reg(args[0])? { Value::String(s) => s.as_str(), v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }) };
                let pattern = match self.get_reg(args[1])? { Value::String(s) => s.as_str(), v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }) };
                match regex::Regex::new(pattern) {
                    Ok(re) => Ok(Value::Boolean(re.is_match(s))),
                    Err(e) => Err(HlxError::ValidationFail { message: format!("Invalid regex pattern: {}", e) }),
                }
            }
            "find_regex" => {
                if args.len() != 2 { return Err(HlxError::ValidationFail { message: "find_regex(string, pattern) takes 2 args".to_string() }); }
                let s = match self.get_reg(args[0])? { Value::String(s) => s.as_str(), v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }) };
                let pattern = match self.get_reg(args[1])? { Value::String(s) => s.as_str(), v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }) };
                match regex::Regex::new(pattern) {
                    Ok(re) => {
                        let matches: im::Vector<Value> = re.find_iter(s)
                            .map(|m| Value::String(m.as_str().to_string()))
                            .collect();
                        Ok(Value::Array(matches))
                    }
                    Err(e) => Err(HlxError::ValidationFail { message: format!("Invalid regex pattern: {}", e) }),
                }
            }
            "sort" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "sort() takes 1 arg".to_string() }); }
                let arr = match self.get_reg(args[0])? { Value::Array(a) => a.clone(), v => return Err(HlxError::TypeError { expected: "array".to_string(), got: v.type_name().to_string() }) };
                let mut vec: Vec<Value> = arr.iter().cloned().collect();
                vec.sort_by(|a, b| {
                    match (a, b) {
                        (Value::Integer(x), Value::Integer(y)) => x.cmp(y),
                        (Value::Float(x), Value::Float(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
                        (Value::String(x), Value::String(y)) => x.cmp(y),
                        (Value::Boolean(x), Value::Boolean(y)) => x.cmp(y),
                        (Value::Integer(x), Value::Float(y)) => (*x as f64).partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
                        (Value::Float(x), Value::Integer(y)) => x.partial_cmp(&(*y as f64)).unwrap_or(std::cmp::Ordering::Equal),
                        _ => std::cmp::Ordering::Equal,
                    }
                });
                Ok(Value::Array(vec.into_iter().collect()))
            }
            "reverse" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "reverse() takes 1 arg".to_string() }); }
                let arr = match self.get_reg(args[0])? { Value::Array(a) => a, v => return Err(HlxError::TypeError { expected: "array".to_string(), got: v.type_name().to_string() }) };
                let reversed: im::Vector<Value> = arr.iter().rev().cloned().collect();
                Ok(Value::Array(reversed))
            }
            "flatten" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "flatten() takes 1 arg".to_string() }); }
                let arr = match self.get_reg(args[0])? { Value::Array(a) => a, v => return Err(HlxError::TypeError { expected: "array".to_string(), got: v.type_name().to_string() }) };
                let mut result = im::Vector::new();
                for item in arr.iter() {
                    if let Value::Array(inner) = item {
                        result.append(inner.clone());
                    } else {
                        result.push_back(item.clone());
                    }
                }
                Ok(Value::Array(result))
            }
            "flatten_deep" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "flatten_deep() takes 1 arg".to_string() }); }
                fn flatten_recursive(arr: &im::Vector<Value>) -> im::Vector<Value> {
                    let mut result = im::Vector::new();
                    for item in arr.iter() {
                        if let Value::Array(inner) = item {
                            result.append(flatten_recursive(inner));
                        } else {
                            result.push_back(item.clone());
                        }
                    }
                    result
                }
                let arr = match self.get_reg(args[0])? { Value::Array(a) => a, v => return Err(HlxError::TypeError { expected: "array".to_string(), got: v.type_name().to_string() }) };
                Ok(Value::Array(flatten_recursive(arr)))
            }
            "unique" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "unique() takes 1 arg".to_string() }); }
                let arr = match self.get_reg(args[0])? { Value::Array(a) => a, v => return Err(HlxError::TypeError { expected: "array".to_string(), got: v.type_name().to_string() }) };
                let mut seen = std::collections::HashSet::new();
                let mut result = im::Vector::new();
                for item in arr.iter() {
                    let key = format!("{:?}", item);
                    if seen.insert(key) {
                        result.push_back(item.clone());
                    }
                }
                Ok(Value::Array(result))
            }
            "zip" => {
                if args.len() != 2 { return Err(HlxError::ValidationFail { message: "zip(arr1, arr2) takes 2 args".to_string() }); }
                let arr1 = match self.get_reg(args[0])? { Value::Array(a) => a, v => return Err(HlxError::TypeError { expected: "array".to_string(), got: v.type_name().to_string() }) };
                let arr2 = match self.get_reg(args[1])? { Value::Array(a) => a, v => return Err(HlxError::TypeError { expected: "array".to_string(), got: v.type_name().to_string() }) };
                let result: im::Vector<Value> = arr1.iter().zip(arr2.iter())
                    .map(|(a, b)| Value::Array(vec![a.clone(), b.clone()].into_iter().collect()))
                    .collect();
                Ok(Value::Array(result))
            }
            "unzip" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "unzip() takes 1 arg".to_string() }); }
                let arr = match self.get_reg(args[0])? { Value::Array(a) => a, v => return Err(HlxError::TypeError { expected: "array".to_string(), got: v.type_name().to_string() }) };
                let mut left = im::Vector::new();
                let mut right = im::Vector::new();
                for item in arr.iter() {
                    if let Value::Array(pair) = item {
                        if pair.len() >= 2 {
                            left.push_back(pair[0].clone());
                            right.push_back(pair[1].clone());
                        }
                    }
                }
                Ok(Value::Array(vec![Value::Array(left), Value::Array(right)].into_iter().collect()))
            }
            "chunk" => {
                if args.len() != 2 { return Err(HlxError::ValidationFail { message: "chunk(array, size) takes 2 args".to_string() }); }
                let arr = match self.get_reg(args[0])? { Value::Array(a) => a, v => return Err(HlxError::TypeError { expected: "array".to_string(), got: v.type_name().to_string() }) };
                let size = match self.get_reg(args[1])? { Value::Integer(i) => *i as usize, v => return Err(HlxError::TypeError { expected: "integer".to_string(), got: v.type_name().to_string() }) };
                if size == 0 { return Err(HlxError::ValidationFail { message: "chunk size must be > 0".to_string() }); }
                let mut result = im::Vector::new();
                let mut chunk = im::Vector::new();
                for (i, item) in arr.iter().enumerate() {
                    chunk.push_back(item.clone());
                    if (i + 1) % size == 0 {
                        result.push_back(Value::Array(chunk.clone()));
                        chunk.clear();
                    }
                }
                if !chunk.is_empty() {
                    result.push_back(Value::Array(chunk));
                }
                Ok(Value::Array(result))
            }
            "take" => {
                if args.len() != 2 { return Err(HlxError::ValidationFail { message: "take(array, n) takes 2 args".to_string() }); }
                let arr = match self.get_reg(args[0])? { Value::Array(a) => a, v => return Err(HlxError::TypeError { expected: "array".to_string(), got: v.type_name().to_string() }) };
                let n = match self.get_reg(args[1])? { Value::Integer(i) => *i as usize, v => return Err(HlxError::TypeError { expected: "integer".to_string(), got: v.type_name().to_string() }) };
                let result: im::Vector<Value> = arr.iter().take(n).cloned().collect();
                Ok(Value::Array(result))
            }
            "drop" => {
                if args.len() != 2 { return Err(HlxError::ValidationFail { message: "drop(array, n) takes 2 args".to_string() }); }
                let arr = match self.get_reg(args[0])? { Value::Array(a) => a, v => return Err(HlxError::TypeError { expected: "array".to_string(), got: v.type_name().to_string() }) };
                let n = match self.get_reg(args[1])? { Value::Integer(i) => *i as usize, v => return Err(HlxError::TypeError { expected: "integer".to_string(), got: v.type_name().to_string() }) };
                let result: im::Vector<Value> = arr.iter().skip(n).cloned().collect();
                Ok(Value::Array(result))
            }
            "sum" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "sum() takes 1 arg".to_string() }); }
                let arr = match self.get_reg(args[0])? { Value::Array(a) => a, v => return Err(HlxError::TypeError { expected: "array".to_string(), got: v.type_name().to_string() }) };
                let mut total = 0.0;
                for item in arr.iter() {
                    match item {
                        Value::Integer(i) => total += *i as f64,
                        Value::Float(f) => total += *f,
                        _ => return Err(HlxError::TypeError { expected: "numeric array".to_string(), got: "mixed types".to_string() }),
                    }
                }
                Ok(Value::Float(total))
            }
            "product" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "product() takes 1 arg".to_string() }); }
                let arr = match self.get_reg(args[0])? { Value::Array(a) => a, v => return Err(HlxError::TypeError { expected: "array".to_string(), got: v.type_name().to_string() }) };
                let mut result = 1.0;
                for item in arr.iter() {
                    match item {
                        Value::Integer(i) => result *= *i as f64,
                        Value::Float(f) => result *= *f,
                        _ => return Err(HlxError::TypeError { expected: "numeric array".to_string(), got: "mixed types".to_string() }),
                    }
                }
                Ok(Value::Float(result))
            }
            "mean" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "mean() takes 1 arg".to_string() }); }
                let arr = match self.get_reg(args[0])? { Value::Array(a) => a, v => return Err(HlxError::TypeError { expected: "array".to_string(), got: v.type_name().to_string() }) };
                if arr.is_empty() { return Ok(Value::Float(0.0)); }
                let mut total = 0.0;
                for item in arr.iter() {
                    match item {
                        Value::Integer(i) => total += *i as f64,
                        Value::Float(f) => total += *f,
                        _ => return Err(HlxError::TypeError { expected: "numeric array".to_string(), got: "mixed types".to_string() }),
                    }
                }
                Ok(Value::Float(total / arr.len() as f64))
            }
            "median" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "median() takes 1 arg".to_string() }); }
                let arr = match self.get_reg(args[0])? { Value::Array(a) => a, v => return Err(HlxError::TypeError { expected: "array".to_string(), got: v.type_name().to_string() }) };
                if arr.is_empty() { return Ok(Value::Float(0.0)); }
                let mut nums: Vec<f64> = Vec::new();
                for item in arr.iter() {
                    match item {
                        Value::Integer(i) => nums.push(*i as f64),
                        Value::Float(f) => nums.push(*f),
                        _ => return Err(HlxError::TypeError { expected: "numeric array".to_string(), got: "mixed types".to_string() }),
                    }
                }
                nums.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let mid = nums.len() / 2;
                let median = if nums.len() % 2 == 0 {
                    (nums[mid - 1] + nums[mid]) / 2.0
                } else {
                    nums[mid]
                };
                Ok(Value::Float(median))
            }
            "mode" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "mode() takes 1 arg".to_string() }); }
                let arr = match self.get_reg(args[0])? { Value::Array(a) => a, v => return Err(HlxError::TypeError { expected: "array".to_string(), got: v.type_name().to_string() }) };
                if arr.is_empty() { return Ok(Value::Null); }
                let mut counts: std::collections::HashMap<String, (usize, Value)> = std::collections::HashMap::new();
                for item in arr.iter() {
                    let key = format!("{:?}", item);
                    let entry = counts.entry(key).or_insert((0, item.clone()));
                    entry.0 += 1;
                }
                let max_count = counts.values().map(|(c, _)| *c).max().unwrap_or(0);
                for (_, (count, val)) in counts {
                    if count == max_count {
                        return Ok(val);
                    }
                }
                Ok(Value::Null)
            }
            "range" => {
                let (start, end, step) = match args.len() {
                    1 => {
                        let end = match self.get_reg(args[0])? { Value::Integer(i) => *i, v => return Err(HlxError::TypeError { expected: "integer".to_string(), got: v.type_name().to_string() }) };
                        (0, end, 1)
                    }
                    2 => {
                        let start = match self.get_reg(args[0])? { Value::Integer(i) => *i, v => return Err(HlxError::TypeError { expected: "integer".to_string(), got: v.type_name().to_string() }) };
                        let end = match self.get_reg(args[1])? { Value::Integer(i) => *i, v => return Err(HlxError::TypeError { expected: "integer".to_string(), got: v.type_name().to_string() }) };
                        (start, end, 1)
                    }
                    3 => {
                        let start = match self.get_reg(args[0])? { Value::Integer(i) => *i, v => return Err(HlxError::TypeError { expected: "integer".to_string(), got: v.type_name().to_string() }) };
                        let end = match self.get_reg(args[1])? { Value::Integer(i) => *i, v => return Err(HlxError::TypeError { expected: "integer".to_string(), got: v.type_name().to_string() }) };
                        let step = match self.get_reg(args[2])? { Value::Integer(i) => *i, v => return Err(HlxError::TypeError { expected: "integer".to_string(), got: v.type_name().to_string() }) };
                        (start, end, step)
                    }
                    _ => return Err(HlxError::ValidationFail { message: "range() takes 1, 2, or 3 args".to_string() }),
                };
                if step == 0 { return Err(HlxError::ValidationFail { message: "range step cannot be 0".to_string() }); }
                let mut result = im::Vector::new();
                if step > 0 {
                    let mut i = start;
                    while i < end {
                        result.push_back(Value::Integer(i));
                        i += step;
                    }
                } else {
                    let mut i = start;
                    while i > end {
                        result.push_back(Value::Integer(i));
                        i += step;
                    }
                }
                Ok(Value::Array(result))
            }
            "keys" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "keys() takes 1 arg".to_string() }); }
                let obj = match self.get_reg(args[0])? { Value::Object(o) => o, v => return Err(HlxError::TypeError { expected: "object".to_string(), got: v.type_name().to_string() }) };
                let keys: im::Vector<Value> = obj.keys().map(|k| Value::String(k.clone())).collect();
                Ok(Value::Array(keys))
            }
            "values" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "values() takes 1 arg".to_string() }); }
                let obj = match self.get_reg(args[0])? { Value::Object(o) => o, v => return Err(HlxError::TypeError { expected: "object".to_string(), got: v.type_name().to_string() }) };
                let values: im::Vector<Value> = obj.values().cloned().collect();
                Ok(Value::Array(values))
            }
            "entries" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "entries() takes 1 arg".to_string() }); }
                let obj = match self.get_reg(args[0])? { Value::Object(o) => o, v => return Err(HlxError::TypeError { expected: "object".to_string(), got: v.type_name().to_string() }) };
                let entries: im::Vector<Value> = obj.iter()
                    .map(|(k, v)| Value::Array(vec![Value::String(k.clone()), v.clone()].into_iter().collect()))
                    .collect();
                Ok(Value::Array(entries))
            }
            "merge" => {
                if args.len() < 2 { return Err(HlxError::ValidationFail { message: "merge() requires at least 2 objects".to_string() }); }
                let mut result = im::OrdMap::new();
                for arg_reg in args {
                    let obj = match self.get_reg(*arg_reg)? { Value::Object(o) => o, v => return Err(HlxError::TypeError { expected: "object".to_string(), got: v.type_name().to_string() }) };
                    for (k, v) in obj {
                        result.insert(k.clone(), v.clone());
                    }
                }
                Ok(Value::Object(result))
            }
            "omit" => {
                if args.len() != 2 { return Err(HlxError::ValidationFail { message: "omit(object, keys) takes 2 args".to_string() }); }
                let obj = match self.get_reg(args[0])? { Value::Object(o) => o, v => return Err(HlxError::TypeError { expected: "object".to_string(), got: v.type_name().to_string() }) };
                let keys_arr = match self.get_reg(args[1])? { Value::Array(a) => a, v => return Err(HlxError::TypeError { expected: "array".to_string(), got: v.type_name().to_string() }) };
                let keys_to_omit: std::collections::HashSet<String> = keys_arr.iter()
                    .filter_map(|v| if let Value::String(s) = v { Some(s.clone()) } else { None })
                    .collect();
                let mut result = im::OrdMap::new();
                for (k, v) in obj {
                    if !keys_to_omit.contains(k) {
                        result.insert(k.clone(), v.clone());
                    }
                }
                Ok(Value::Object(result))
            }
            "pick" => {
                if args.len() != 2 { return Err(HlxError::ValidationFail { message: "pick(object, keys) takes 2 args".to_string() }); }
                let obj = match self.get_reg(args[0])? { Value::Object(o) => o, v => return Err(HlxError::TypeError { expected: "object".to_string(), got: v.type_name().to_string() }) };
                let keys_arr = match self.get_reg(args[1])? { Value::Array(a) => a, v => return Err(HlxError::TypeError { expected: "array".to_string(), got: v.type_name().to_string() }) };
                let mut result = im::OrdMap::new();
                for key_val in keys_arr.iter() {
                    if let Value::String(key) = key_val {
                        if let Some(val) = obj.get(key) {
                            result.insert(key.clone(), val.clone());
                        }
                    }
                }
                Ok(Value::Object(result))
            }
            "from_entries" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "from_entries() takes 1 arg".to_string() }); }
                let arr = match self.get_reg(args[0])? { Value::Array(a) => a, v => return Err(HlxError::TypeError { expected: "array".to_string(), got: v.type_name().to_string() }) };
                let mut result = im::OrdMap::new();
                for entry in arr.iter() {
                    if let Value::Array(pair) = entry {
                        if pair.len() >= 2 {
                            if let Value::String(key) = &pair[0] {
                                result.insert(key.clone(), pair[1].clone());
                            }
                        }
                    }
                }
                Ok(Value::Object(result))
            }
            "sha256" => {
                use sha2::{Sha256, Digest};
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "sha256() takes 1 arg".to_string() }); }
                let data = match self.get_reg(args[0])? { Value::String(s) => s.as_bytes(), v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }) };
                let mut hasher = Sha256::new();
                hasher.update(data);
                let result = hasher.finalize();
                Ok(Value::String(hex::encode(result)))
            }
            "sha512" => {
                use sha2::{Sha512, Digest};
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "sha512() takes 1 arg".to_string() }); }
                let data = match self.get_reg(args[0])? { Value::String(s) => s.as_bytes(), v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }) };
                let mut hasher = Sha512::new();
                hasher.update(data);
                let result = hasher.finalize();
                Ok(Value::String(hex::encode(result)))
            }
            "blake3" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "blake3() takes 1 arg".to_string() }); }
                let data = match self.get_reg(args[0])? { Value::String(s) => s.as_bytes(), v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }) };
                let hash = blake3::hash(data);
                Ok(Value::String(hash.to_hex().to_string()))
            }
            "md5" => {
                use md5::{Md5, Digest};
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "md5() takes 1 arg".to_string() }); }
                let data = match self.get_reg(args[0])? { Value::String(s) => s.as_bytes(), v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }) };
                let mut hasher = Md5::new();
                hasher.update(data);
                let result = hasher.finalize();
                Ok(Value::String(hex::encode(result)))
            }
            "hmac_sha256" => {
                use hmac::{Hmac, Mac};
                use sha2::Sha256;
                type HmacSha256 = Hmac<Sha256>;
                if args.len() != 2 { return Err(HlxError::ValidationFail { message: "hmac_sha256(key, message) takes 2 args".to_string() }); }
                let key = match self.get_reg(args[0])? { Value::String(s) => s.as_bytes(), v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }) };
                let message = match self.get_reg(args[1])? { Value::String(s) => s.as_bytes(), v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }) };
                let mut mac = HmacSha256::new_from_slice(key).map_err(|e| HlxError::BackendError { message: format!("HMAC error: {}", e) })?;
                mac.update(message);
                let result = mac.finalize();
                Ok(Value::String(hex::encode(result.into_bytes())))
            }
            "base64_encode" => {
                use base64::{Engine as _, engine::general_purpose};
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "base64_encode() takes 1 arg".to_string() }); }
                let data = match self.get_reg(args[0])? { Value::String(s) => s.as_bytes(), v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }) };
                Ok(Value::String(general_purpose::STANDARD.encode(data)))
            }
            "base64_decode" => {
                use base64::{Engine as _, engine::general_purpose};
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "base64_decode() takes 1 arg".to_string() }); }
                let encoded = match self.get_reg(args[0])? { Value::String(s) => s.as_str(), v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }) };
                let decoded = general_purpose::STANDARD.decode(encoded).map_err(|e| HlxError::ValidationFail { message: format!("Base64 decode error: {}", e) })?;
                let result = String::from_utf8(decoded).map_err(|e| HlxError::ValidationFail { message: format!("Invalid UTF-8 in decoded data: {}", e) })?;
                Ok(Value::String(result))
            }
            "hex_encode" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "hex_encode() takes 1 arg".to_string() }); }
                let data = match self.get_reg(args[0])? { Value::String(s) => s.as_bytes(), v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }) };
                Ok(Value::String(hex::encode(data)))
            }
            "hex_decode" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "hex_decode() takes 1 arg".to_string() }); }
                let encoded = match self.get_reg(args[0])? { Value::String(s) => s.as_str(), v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }) };
                let decoded = hex::decode(encoded).map_err(|e| HlxError::ValidationFail { message: format!("Hex decode error: {}", e) })?;
                let result = String::from_utf8(decoded).map_err(|e| HlxError::ValidationFail { message: format!("Invalid UTF-8 in decoded data: {}", e) })?;
                Ok(Value::String(result))
            }
            "url_encode" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "url_encode() takes 1 arg".to_string() }); }
                let data = match self.get_reg(args[0])? { Value::String(s) => s.as_str(), v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }) };
                Ok(Value::String(urlencoding::encode(data).into_owned()))
            }
            "url_decode" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "url_decode() takes 1 arg".to_string() }); }
                let encoded = match self.get_reg(args[0])? { Value::String(s) => s.as_str(), v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }) };
                let decoded = urlencoding::decode(encoded).map_err(|e| HlxError::ValidationFail { message: format!("URL decode error: {}", e) })?;
                Ok(Value::String(decoded.into_owned()))
            }
            "now" => {
                if args.len() != 0 { return Err(HlxError::ValidationFail { message: "now() takes no arguments".to_string() }); }
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(|e| HlxError::BackendError { message: format!("Time error: {}", e) })?;
                Ok(Value::Integer(now.as_millis() as i64))
            }
            "now_micros" => {
                if args.len() != 0 { return Err(HlxError::ValidationFail { message: "now_micros() takes no arguments".to_string() }); }
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(|e| HlxError::BackendError { message: format!("Time error: {}", e) })?;
                Ok(Value::Integer(now.as_micros() as i64))
            }
            "format_timestamp" => {
                use chrono::{DateTime, Utc, TimeZone};
                if args.len() != 2 { return Err(HlxError::ValidationFail { message: "format_timestamp(timestamp, format) takes 2 args".to_string() }); }
                let timestamp = match self.get_reg(args[0])? { Value::Integer(i) => *i, v => return Err(HlxError::TypeError { expected: "integer".to_string(), got: v.type_name().to_string() }) };
                let format = match self.get_reg(args[1])? { Value::String(s) => s.as_str(), v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }) };
                let dt = Utc.timestamp_millis_opt(timestamp).single()
                    .ok_or_else(|| HlxError::ValidationFail { message: "Invalid timestamp".to_string() })?;
                Ok(Value::String(dt.format(format).to_string()))
            }
            "parse_timestamp" => {
                use chrono::{DateTime, Utc, NaiveDateTime};
                if args.len() != 2 { return Err(HlxError::ValidationFail { message: "parse_timestamp(date_string, format) takes 2 args".to_string() }); }
                let date_str = match self.get_reg(args[0])? { Value::String(s) => s.as_str(), v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }) };
                let format = match self.get_reg(args[1])? { Value::String(s) => s.as_str(), v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }) };
                let naive = NaiveDateTime::parse_from_str(date_str, format)
                    .map_err(|e| HlxError::ValidationFail { message: format!("Failed to parse timestamp: {}", e) })?;
                let dt: DateTime<Utc> = DateTime::from_naive_utc_and_offset(naive, Utc);
                Ok(Value::Integer(dt.timestamp_millis()))
            }
            "year" => {
                use chrono::{DateTime, Utc, TimeZone, Datelike};
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "year() takes 1 arg".to_string() }); }
                let timestamp = match self.get_reg(args[0])? { Value::Integer(i) => *i, v => return Err(HlxError::TypeError { expected: "integer".to_string(), got: v.type_name().to_string() }) };
                let dt = Utc.timestamp_millis_opt(timestamp).single()
                    .ok_or_else(|| HlxError::ValidationFail { message: "Invalid timestamp".to_string() })?;
                Ok(Value::Integer(dt.year() as i64))
            }
            "month" => {
                use chrono::{DateTime, Utc, TimeZone, Datelike};
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "month() takes 1 arg".to_string() }); }
                let timestamp = match self.get_reg(args[0])? { Value::Integer(i) => *i, v => return Err(HlxError::TypeError { expected: "integer".to_string(), got: v.type_name().to_string() }) };
                let dt = Utc.timestamp_millis_opt(timestamp).single()
                    .ok_or_else(|| HlxError::ValidationFail { message: "Invalid timestamp".to_string() })?;
                Ok(Value::Integer(dt.month() as i64))
            }
            "day" => {
                use chrono::{DateTime, Utc, TimeZone, Datelike};
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "day() takes 1 arg".to_string() }); }
                let timestamp = match self.get_reg(args[0])? { Value::Integer(i) => *i, v => return Err(HlxError::TypeError { expected: "integer".to_string(), got: v.type_name().to_string() }) };
                let dt = Utc.timestamp_millis_opt(timestamp).single()
                    .ok_or_else(|| HlxError::ValidationFail { message: "Invalid timestamp".to_string() })?;
                Ok(Value::Integer(dt.day() as i64))
            }
            "hour" => {
                use chrono::{DateTime, Utc, TimeZone, Timelike};
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "hour() takes 1 arg".to_string() }); }
                let timestamp = match self.get_reg(args[0])? { Value::Integer(i) => *i, v => return Err(HlxError::TypeError { expected: "integer".to_string(), got: v.type_name().to_string() }) };
                let dt = Utc.timestamp_millis_opt(timestamp).single()
                    .ok_or_else(|| HlxError::ValidationFail { message: "Invalid timestamp".to_string() })?;
                Ok(Value::Integer(dt.hour() as i64))
            }
            "minute" => {
                use chrono::{DateTime, Utc, TimeZone, Timelike};
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "minute() takes 1 arg".to_string() }); }
                let timestamp = match self.get_reg(args[0])? { Value::Integer(i) => *i, v => return Err(HlxError::TypeError { expected: "integer".to_string(), got: v.type_name().to_string() }) };
                let dt = Utc.timestamp_millis_opt(timestamp).single()
                    .ok_or_else(|| HlxError::ValidationFail { message: "Invalid timestamp".to_string() })?;
                Ok(Value::Integer(dt.minute() as i64))
            }
            "second" => {
                use chrono::{DateTime, Utc, TimeZone, Timelike};
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "second() takes 1 arg".to_string() }); }
                let timestamp = match self.get_reg(args[0])? { Value::Integer(i) => *i, v => return Err(HlxError::TypeError { expected: "integer".to_string(), got: v.type_name().to_string() }) };
                let dt = Utc.timestamp_millis_opt(timestamp).single()
                    .ok_or_else(|| HlxError::ValidationFail { message: "Invalid timestamp".to_string() })?;
                Ok(Value::Integer(dt.second() as i64))
            }
            "bit_and" => {
                if args.len() != 2 { return Err(HlxError::ValidationFail { message: "bit_and(a, b) takes 2 args".to_string() }); }
                let a = match self.get_reg(args[0])? { Value::Integer(i) => *i, v => return Err(HlxError::TypeError { expected: "integer".to_string(), got: v.type_name().to_string() }) };
                let b = match self.get_reg(args[1])? { Value::Integer(i) => *i, v => return Err(HlxError::TypeError { expected: "integer".to_string(), got: v.type_name().to_string() }) };
                Ok(Value::Integer(a & b))
            }
            "bit_or" => {
                if args.len() != 2 { return Err(HlxError::ValidationFail { message: "bit_or(a, b) takes 2 args".to_string() }); }
                let a = match self.get_reg(args[0])? { Value::Integer(i) => *i, v => return Err(HlxError::TypeError { expected: "integer".to_string(), got: v.type_name().to_string() }) };
                let b = match self.get_reg(args[1])? { Value::Integer(i) => *i, v => return Err(HlxError::TypeError { expected: "integer".to_string(), got: v.type_name().to_string() }) };
                Ok(Value::Integer(a | b))
            }
            "bit_xor" => {
                if args.len() != 2 { return Err(HlxError::ValidationFail { message: "bit_xor(a, b) takes 2 args".to_string() }); }
                let a = match self.get_reg(args[0])? { Value::Integer(i) => *i, v => return Err(HlxError::TypeError { expected: "integer".to_string(), got: v.type_name().to_string() }) };
                let b = match self.get_reg(args[1])? { Value::Integer(i) => *i, v => return Err(HlxError::TypeError { expected: "integer".to_string(), got: v.type_name().to_string() }) };
                Ok(Value::Integer(a ^ b))
            }
            "bit_not" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "bit_not() takes 1 arg".to_string() }); }
                let a = match self.get_reg(args[0])? { Value::Integer(i) => *i, v => return Err(HlxError::TypeError { expected: "integer".to_string(), got: v.type_name().to_string() }) };
                Ok(Value::Integer(!a))
            }
            "bit_shl" => {
                if args.len() != 2 { return Err(HlxError::ValidationFail { message: "bit_shl(value, shift) takes 2 args".to_string() }); }
                let value = match self.get_reg(args[0])? { Value::Integer(i) => *i, v => return Err(HlxError::TypeError { expected: "integer".to_string(), got: v.type_name().to_string() }) };
                let shift = match self.get_reg(args[1])? { Value::Integer(i) => *i as u32, v => return Err(HlxError::TypeError { expected: "integer".to_string(), got: v.type_name().to_string() }) };
                Ok(Value::Integer(value << shift))
            }
            "bit_shr" => {
                if args.len() != 2 { return Err(HlxError::ValidationFail { message: "bit_shr(value, shift) takes 2 args".to_string() }); }
                let value = match self.get_reg(args[0])? { Value::Integer(i) => *i, v => return Err(HlxError::TypeError { expected: "integer".to_string(), got: v.type_name().to_string() }) };
                let shift = match self.get_reg(args[1])? { Value::Integer(i) => *i as u32, v => return Err(HlxError::TypeError { expected: "integer".to_string(), got: v.type_name().to_string() }) };
                Ok(Value::Integer(value >> shift))
            }
            "bit_count" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "bit_count() takes 1 arg".to_string() }); }
                let value = match self.get_reg(args[0])? { Value::Integer(i) => *i, v => return Err(HlxError::TypeError { expected: "integer".to_string(), got: v.type_name().to_string() }) };
                Ok(Value::Integer(value.count_ones() as i64))
            }
            "bit_reverse" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "bit_reverse() takes 1 arg".to_string() }); }
                let value = match self.get_reg(args[0])? { Value::Integer(i) => *i, v => return Err(HlxError::TypeError { expected: "integer".to_string(), got: v.type_name().to_string() }) };
                Ok(Value::Integer(value.reverse_bits()))
            }
            "sign" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "sign() takes 1 arg".to_string() }); }
                let num = match self.get_reg(args[0])? {
                    Value::Integer(i) => *i as f64,
                    Value::Float(f) => *f,
                    v => return Err(HlxError::TypeError { expected: "numeric".to_string(), got: v.type_name().to_string() }),
                };
                let sign = if num > 0.0 { 1 } else if num < 0.0 { -1 } else { 0 };
                Ok(Value::Integer(sign))
            }
            "clamp" => {
                if args.len() != 3 { return Err(HlxError::ValidationFail { message: "clamp(value, min, max) takes 3 args".to_string() }); }
                let value = match self.get_reg(args[0])? { Value::Float(f) => *f, Value::Integer(i) => *i as f64, v => return Err(HlxError::TypeError { expected: "numeric".to_string(), got: v.type_name().to_string() }) };
                let min = match self.get_reg(args[1])? { Value::Float(f) => *f, Value::Integer(i) => *i as f64, v => return Err(HlxError::TypeError { expected: "numeric".to_string(), got: v.type_name().to_string() }) };
                let max = match self.get_reg(args[2])? { Value::Float(f) => *f, Value::Integer(i) => *i as f64, v => return Err(HlxError::TypeError { expected: "numeric".to_string(), got: v.type_name().to_string() }) };
                Ok(Value::Float(value.max(min).min(max)))
            }
            "lerp" => {
                if args.len() != 3 { return Err(HlxError::ValidationFail { message: "lerp(a, b, t) takes 3 args".to_string() }); }
                let a = match self.get_reg(args[0])? { Value::Float(f) => *f, Value::Integer(i) => *i as f64, v => return Err(HlxError::TypeError { expected: "numeric".to_string(), got: v.type_name().to_string() }) };
                let b = match self.get_reg(args[1])? { Value::Float(f) => *f, Value::Integer(i) => *i as f64, v => return Err(HlxError::TypeError { expected: "numeric".to_string(), got: v.type_name().to_string() }) };
                let t = match self.get_reg(args[2])? { Value::Float(f) => *f, Value::Integer(i) => *i as f64, v => return Err(HlxError::TypeError { expected: "numeric".to_string(), got: v.type_name().to_string() }) };
                Ok(Value::Float(a + (b - a) * t))
            }
            "degrees" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "degrees() takes 1 arg".to_string() }); }
                let radians = match self.get_reg(args[0])? { Value::Float(f) => *f, Value::Integer(i) => *i as f64, v => return Err(HlxError::TypeError { expected: "numeric".to_string(), got: v.type_name().to_string() }) };
                Ok(Value::Float(radians.to_degrees()))
            }
            "radians" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "radians() takes 1 arg".to_string() }); }
                let degrees = match self.get_reg(args[0])? { Value::Float(f) => *f, Value::Integer(i) => *i as f64, v => return Err(HlxError::TypeError { expected: "numeric".to_string(), got: v.type_name().to_string() }) };
                Ok(Value::Float(degrees.to_radians()))
            }
            "gcd" => {
                if args.len() != 2 { return Err(HlxError::ValidationFail { message: "gcd(a, b) takes 2 args".to_string() }); }
                let mut a = match self.get_reg(args[0])? { Value::Integer(i) => i.abs(), v => return Err(HlxError::TypeError { expected: "integer".to_string(), got: v.type_name().to_string() }) };
                let mut b = match self.get_reg(args[1])? { Value::Integer(i) => i.abs(), v => return Err(HlxError::TypeError { expected: "integer".to_string(), got: v.type_name().to_string() }) };
                while b != 0 {
                    let temp = b;
                    b = a % b;
                    a = temp;
                }
                Ok(Value::Integer(a))
            }
            "lcm" => {
                if args.len() != 2 { return Err(HlxError::ValidationFail { message: "lcm(a, b) takes 2 args".to_string() }); }
                let a = match self.get_reg(args[0])? { Value::Integer(i) => i.abs(), v => return Err(HlxError::TypeError { expected: "integer".to_string(), got: v.type_name().to_string() }) };
                let b = match self.get_reg(args[1])? { Value::Integer(i) => i.abs(), v => return Err(HlxError::TypeError { expected: "integer".to_string(), got: v.type_name().to_string() }) };
                if a == 0 || b == 0 { return Ok(Value::Integer(0)); }
                // Use gcd to compute lcm: lcm(a,b) = |a*b| / gcd(a,b)
                let mut gcd_a = a;
                let mut gcd_b = b;
                while gcd_b != 0 {
                    let temp = gcd_b;
                    gcd_b = gcd_a % gcd_b;
                    gcd_a = temp;
                }
                Ok(Value::Integer((a * b) / gcd_a))
            }
            "factorial" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "factorial() takes 1 arg".to_string() }); }
                let n = match self.get_reg(args[0])? { Value::Integer(i) => *i, v => return Err(HlxError::TypeError { expected: "integer".to_string(), got: v.type_name().to_string() }) };
                if n < 0 { return Err(HlxError::ValidationFail { message: "factorial requires non-negative integer".to_string() }); }
                let mut result: i64 = 1;
                for i in 2..=n {
                    result = result.checked_mul(i).ok_or_else(|| HlxError::ValidationFail { message: "factorial overflow".to_string() })?;
                }
                Ok(Value::Integer(result))
            }
            "is_nan" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "is_nan() takes 1 arg".to_string() }); }
                match self.get_reg(args[0])? {
                    Value::Float(f) => Ok(Value::Boolean(f.is_nan())),
                    Value::Integer(_) => Ok(Value::Boolean(false)),
                    v => Err(HlxError::TypeError { expected: "numeric".to_string(), got: v.type_name().to_string() }),
                }
            }
            "is_inf" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "is_inf() takes 1 arg".to_string() }); }
                match self.get_reg(args[0])? {
                    Value::Float(f) => Ok(Value::Boolean(f.is_infinite())),
                    Value::Integer(_) => Ok(Value::Boolean(false)),
                    v => Err(HlxError::TypeError { expected: "numeric".to_string(), got: v.type_name().to_string() }),
                }
            }
            "hypot" => {
                if args.len() != 2 { return Err(HlxError::ValidationFail { message: "hypot(x, y) takes 2 args".to_string() }); }
                let x = match self.get_reg(args[0])? { Value::Float(f) => *f, Value::Integer(i) => *i as f64, v => return Err(HlxError::TypeError { expected: "numeric".to_string(), got: v.type_name().to_string() }) };
                let y = match self.get_reg(args[1])? { Value::Float(f) => *f, Value::Integer(i) => *i as f64, v => return Err(HlxError::TypeError { expected: "numeric".to_string(), got: v.type_name().to_string() }) };
                Ok(Value::Float(x.hypot(y)))
            }
            "trunc" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "trunc() takes 1 arg".to_string() }); }
                match self.get_reg(args[0])? {
                    Value::Float(f) => Ok(Value::Float(f.trunc())),
                    Value::Integer(i) => Ok(Value::Float(*i as f64)),
                    v => Err(HlxError::TypeError { expected: "numeric".to_string(), got: v.type_name().to_string() }),
                }
            }
            "assert" => {
                if args.len() != 2 { return Err(HlxError::ValidationFail { message: "assert(condition, message) takes 2 args".to_string() }); }
                let condition = match self.get_reg(args[0])? { Value::Boolean(b) => *b, v => return Err(HlxError::TypeError { expected: "boolean".to_string(), got: v.type_name().to_string() }) };
                let message = match self.get_reg(args[1])? { Value::String(s) => s.as_str(), v => return Err(HlxError::TypeError { expected: "string".to_string(), got: v.type_name().to_string() }) };
                if !condition {
                    return Err(HlxError::ValidationFail { message: format!("Assertion failed: {}", message) });
                }
                Ok(Value::Null)
            }
            "debug" => {
                // Variadic debug print
                for arg_reg in args {
                    let val = self.get_reg(*arg_reg)?;
                    eprintln!("[DEBUG] {}", val);
                }
                Ok(Value::Null)
            }
            "asin" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "asin() takes 1 arg".to_string() }); }
                match self.get_reg(args[0])? {
                    Value::Float(f) => Ok(Value::Float(f.asin())),
                    Value::Integer(i) => Ok(Value::Float((*i as f64).asin())),
                    v => Err(HlxError::TypeError { expected: "numeric".to_string(), got: v.type_name().to_string() }),
                }
            }
            "acos" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "acos() takes 1 arg".to_string() }); }
                match self.get_reg(args[0])? {
                    Value::Float(f) => Ok(Value::Float(f.acos())),
                    Value::Integer(i) => Ok(Value::Float((*i as f64).acos())),
                    v => Err(HlxError::TypeError { expected: "numeric".to_string(), got: v.type_name().to_string() }),
                }
            }
            "atan" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "atan() takes 1 arg".to_string() }); }
                match self.get_reg(args[0])? {
                    Value::Float(f) => Ok(Value::Float(f.atan())),
                    Value::Integer(i) => Ok(Value::Float((*i as f64).atan())),
                    v => Err(HlxError::TypeError { expected: "numeric".to_string(), got: v.type_name().to_string() }),
                }
            }
            "atan2" => {
                if args.len() != 2 { return Err(HlxError::ValidationFail { message: "atan2(y, x) takes 2 args".to_string() }); }
                let y = match self.get_reg(args[0])? {
                    Value::Float(f) => *f,
                    Value::Integer(i) => *i as f64,
                    v => return Err(HlxError::TypeError { expected: "numeric".to_string(), got: v.type_name().to_string() }),
                };
                let x = match self.get_reg(args[1])? {
                    Value::Float(f) => *f,
                    Value::Integer(i) => *i as f64,
                    v => return Err(HlxError::TypeError { expected: "numeric".to_string(), got: v.type_name().to_string() }),
                };
                Ok(Value::Float(y.atan2(x)))
            }
            "sinh" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "sinh() takes 1 arg".to_string() }); }
                match self.get_reg(args[0])? {
                    Value::Float(f) => Ok(Value::Float(f.sinh())),
                    Value::Integer(i) => Ok(Value::Float((*i as f64).sinh())),
                    v => Err(HlxError::TypeError { expected: "numeric".to_string(), got: v.type_name().to_string() }),
                }
            }
            "cosh" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "cosh() takes 1 arg".to_string() }); }
                match self.get_reg(args[0])? {
                    Value::Float(f) => Ok(Value::Float(f.cosh())),
                    Value::Integer(i) => Ok(Value::Float((*i as f64).cosh())),
                    v => Err(HlxError::TypeError { expected: "numeric".to_string(), got: v.type_name().to_string() }),
                }
            }
            "tanh" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "tanh() takes 1 arg".to_string() }); }
                match self.get_reg(args[0])? {
                    Value::Float(f) => Ok(Value::Float(f.tanh())),
                    Value::Integer(i) => Ok(Value::Float((*i as f64).tanh())),
                    v => Err(HlxError::TypeError { expected: "numeric".to_string(), got: v.type_name().to_string() }),
                }
            }
            "asinh" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "asinh() takes 1 arg".to_string() }); }
                match self.get_reg(args[0])? {
                    Value::Float(f) => Ok(Value::Float(f.asinh())),
                    Value::Integer(i) => Ok(Value::Float((*i as f64).asinh())),
                    v => Err(HlxError::TypeError { expected: "numeric".to_string(), got: v.type_name().to_string() }),
                }
            }
            "acosh" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "acosh() takes 1 arg".to_string() }); }
                match self.get_reg(args[0])? {
                    Value::Float(f) => Ok(Value::Float(f.acosh())),
                    Value::Integer(i) => Ok(Value::Float((*i as f64).acosh())),
                    v => Err(HlxError::TypeError { expected: "numeric".to_string(), got: v.type_name().to_string() }),
                }
            }
            "atanh" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "atanh() takes 1 arg".to_string() }); }
                match self.get_reg(args[0])? {
                    Value::Float(f) => Ok(Value::Float(f.atanh())),
                    Value::Integer(i) => Ok(Value::Float((*i as f64).atanh())),
                    v => Err(HlxError::TypeError { expected: "numeric".to_string(), got: v.type_name().to_string() }),
                }
            }
            "cbrt" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "cbrt() takes 1 arg".to_string() }); }
                match self.get_reg(args[0])? {
                    Value::Float(f) => Ok(Value::Float(f.cbrt())),
                    Value::Integer(i) => Ok(Value::Float((*i as f64).cbrt())),
                    v => Err(HlxError::TypeError { expected: "numeric".to_string(), got: v.type_name().to_string() }),
                }
            }
            "pow" => {
                if args.len() != 2 { return Err(HlxError::ValidationFail { message: "pow(base, exponent) takes 2 args".to_string() }); }
                let base = match self.get_reg(args[0])? {
                    Value::Float(f) => *f,
                    Value::Integer(i) => *i as f64,
                    v => return Err(HlxError::TypeError { expected: "numeric".to_string(), got: v.type_name().to_string() }),
                };
                let exponent = match self.get_reg(args[1])? {
                    Value::Float(f) => *f,
                    Value::Integer(i) => *i as f64,
                    v => return Err(HlxError::TypeError { expected: "numeric".to_string(), got: v.type_name().to_string() }),
                };
                Ok(Value::Float(base.powf(exponent)))
            }
            "exp2" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "exp2() takes 1 arg".to_string() }); }
                match self.get_reg(args[0])? {
                    Value::Float(f) => Ok(Value::Float(f.exp2())),
                    Value::Integer(i) => Ok(Value::Float((*i as f64).exp2())),
                    v => Err(HlxError::TypeError { expected: "numeric".to_string(), got: v.type_name().to_string() }),
                }
            }
            "log2" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "log2() takes 1 arg".to_string() }); }
                match self.get_reg(args[0])? {
                    Value::Float(f) => Ok(Value::Float(f.log2())),
                    Value::Integer(i) => Ok(Value::Float((*i as f64).log2())),
                    v => Err(HlxError::TypeError { expected: "numeric".to_string(), got: v.type_name().to_string() }),
                }
            }
            "log10" => {
                if args.len() != 1 { return Err(HlxError::ValidationFail { message: "log10() takes 1 arg".to_string() }); }
                match self.get_reg(args[0])? {
                    Value::Float(f) => Ok(Value::Float(f.log10())),
                    Value::Integer(i) => Ok(Value::Float((*i as f64).log10())),
                    v => Err(HlxError::TypeError { expected: "numeric".to_string(), got: v.type_name().to_string() }),
                }
            }
            _ => Err(HlxError::ValidationFail {
                message: format!("Unknown function: {}", func),
            }),
        }
    }
}

/// Native tokenizer - replicates compiler.hlxc tokenize() in Rust for O(n) performance
fn native_tokenize(source: &str) -> Result<Value> {
    let chars: Vec<char> = source.chars().collect();
    let len = chars.len();
    let mut tokens = Vector::new();
    let mut i = 0;
    
    while i < len {
        let c = chars[i];
        
        // Skip whitespace
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        
        // Skip comments
        if c == '/' && i + 1 < len && chars[i + 1] == '/' {
            while i < len && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        
        // Identifiers and keywords
        if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let ident: String = chars[start..i].iter().collect();
            
            let token_type = match ident.as_str() {
                "fn" => "KW_FN",
                "let" => "KW_LET",
                "return" => "KW_RETURN",
                "if" => "KW_IF",
                "else" => "KW_ELSE",
                "loop" => "KW_LOOP",
                "break" => "KW_BREAK",
                "continue" => "KW_CONT",
                "and" => "OP_AND",
                "or" => "OP_OR",
                "true" => "LIT_BOOL",
                "false" => "LIT_BOOL",
                "null" => "LIT_NULL",
                _ => "IDENT",
            };
            
            let mut obj = OrdMap::new();
            obj.insert("type".to_string(), Value::String(token_type.to_string()));
            obj.insert("val".to_string(), Value::String(ident));
            // println!("DEBUG TOKEN: {} ({})", token_type, ident);
            tokens.push_back(Value::Object(obj));
            continue;
        }
        
        // Numbers
        if c.is_ascii_digit() {
            let start = i;
            while i < len && chars[i].is_ascii_digit() {
                i += 1;
            }
            let num: String = chars[start..i].iter().collect();
            let mut obj = OrdMap::new();
            obj.insert("type".to_string(), Value::String("LIT_INT".to_string()));
            obj.insert("val".to_string(), Value::String(num));
            tokens.push_back(Value::Object(obj));
            continue;
        }
        
        // String literals
        if c == '"' {
            i += 1;
            let mut s = String::new();
            while i < len && chars[i] != '"' {
                if chars[i] == '\\' && i + 1 < len {
                    match chars[i + 1] {
                        '"' => { s.push('"'); i += 2; }
                        '\\' => { s.push('\\'); i += 2; }
                        'n' => { s.push('\n'); i += 2; }
                        't' => { s.push('\t'); i += 2; }
                        'r' => { s.push('\r'); i += 2; }
                        _ => { s.push(chars[i]); i += 1; }
                    }
                } else {
                    s.push(chars[i]);
                    i += 1;
                }
            }
            i += 1; // Skip closing quote
            let mut obj = OrdMap::new();
            obj.insert("type".to_string(), Value::String("LIT_STR".to_string()));
            obj.insert("val".to_string(), Value::String(s));
            tokens.push_back(Value::Object(obj));
            continue;
        }
        
        if c == '-' {
            if i + 1 < len && chars[i + 1] == '>' {
                let mut obj = OrdMap::new();
                obj.insert("type".to_string(), Value::String("OP_ARROW".to_string()));
                obj.insert("val".to_string(), Value::String("->".to_string()));
                tokens.push_back(Value::Object(obj));
                i += 2;
                continue;
            }
        }

        // Single-character tokens
        let single_token = match c {
            '{' => Some("LBRACE"),
            '}' => Some("RBRACE"),
            '[' => Some("LBRACK"),
            ']' => Some("RBRACK"),
            '(' => Some("LPAREN"),
            ')' => Some("RPAREN"),
            ';' => Some("SEMI"),
            ':' => Some("COLON"),
            ',' => Some("COMMA"),
            '.' => Some("DOT"),
            '+' => Some("OP_ADD"),
            '-' => Some("OP_SUB"),
            '*' => Some("OP_MUL"),
            '/' => Some("OP_DIV"),
            _ => None,
        };
        
        if let Some(token_type) = single_token {
            let mut obj = OrdMap::new();
            obj.insert("type".to_string(), Value::String(token_type.to_string()));
            obj.insert("val".to_string(), Value::String(c.to_string()));
            tokens.push_back(Value::Object(obj));
            i += 1;
            continue;
        }
        
        // Two-character operators
        if c == '=' {
            if i + 1 < len && chars[i + 1] == '=' {
                let mut obj = OrdMap::new();
                obj.insert("type".to_string(), Value::String("OP_EQ".to_string()));
                obj.insert("val".to_string(), Value::String("==".to_string()));
                tokens.push_back(Value::Object(obj));
                i += 2;
            } else {
                let mut obj = OrdMap::new();
                obj.insert("type".to_string(), Value::String("OP_ASSIGN".to_string()));
                obj.insert("val".to_string(), Value::String("=".to_string()));
                tokens.push_back(Value::Object(obj));
                i += 1;
            }
            continue;
        }
        
        if c == '!' && i + 1 < len && chars[i + 1] == '=' {
            let mut obj = OrdMap::new();
            obj.insert("type".to_string(), Value::String("OP_NE".to_string()));
            obj.insert("val".to_string(), Value::String("!=".to_string()));
            tokens.push_back(Value::Object(obj));
            i += 2;
            continue;
        }
        
        if c == '<' {
            if i + 1 < len && chars[i + 1] == '=' {
                let mut obj = OrdMap::new();
                obj.insert("type".to_string(), Value::String("OP_LE".to_string()));
                obj.insert("val".to_string(), Value::String("<=".to_string()));
                tokens.push_back(Value::Object(obj));
                i += 2;
            } else {
                let mut obj = OrdMap::new();
                obj.insert("type".to_string(), Value::String("OP_LT".to_string()));
                obj.insert("val".to_string(), Value::String("<".to_string()));
                tokens.push_back(Value::Object(obj));
                i += 1;
            }
            continue;
        }
        
        if c == '>' {
            if i + 1 < len && chars[i + 1] == '=' {
                let mut obj = OrdMap::new();
                obj.insert("type".to_string(), Value::String("OP_GE".to_string()));
                obj.insert("val".to_string(), Value::String(">=".to_string()));
                tokens.push_back(Value::Object(obj));
                i += 2;
            } else {
                let mut obj = OrdMap::new();
                obj.insert("type".to_string(), Value::String("OP_GT".to_string()));
                obj.insert("val".to_string(), Value::String(">".to_string()));
                tokens.push_back(Value::Object(obj));
                i += 1;
            }
            continue;
        }
        
        // Skip unknown characters
        i += 1;
    }
    
    // println!("DEBUG: Native tokenization complete. Generated {} tokens.", tokens.len());
    // if let Some(last) = tokens.back() {
    //     println!("DEBUG: Last token: {:?}", last);
    // }

    Ok(Value::Array(tokens))
}

// Backend Capability Implementation for Interpreter
impl crate::backend::BackendCapability for Executor {
    fn supported_contracts(&self) -> Vec<String> {
        // Interpreter is the reference implementation - supports ALL contracts
        vec!["*".to_string()]
    }

    fn supported_builtins(&self) -> Vec<String> {
        // Return all builtin functions implemented in execute_builtin()
        vec![
            // I/O
            "print".to_string(),
            "read_file".to_string(),
            "write_file".to_string(),
            "file_exists".to_string(),
            "delete_file".to_string(),
            "list_files".to_string(),
            "create_dir".to_string(),

            // Type introspection
            "type".to_string(),
            "len".to_string(),

            // Array operations
            "slice".to_string(),
            "append".to_string(),
            "arr_pop".to_string(),
            "arr_slice".to_string(),
            "arr_concat".to_string(),

            // String operations
            "concat".to_string(),
            "strlen".to_string(),
            "substring".to_string(),
            "index_of".to_string(),
            "to_upper".to_string(),
            "to_lower".to_string(),
            "trim".to_string(),
            "starts_with".to_string(),
            "ends_with".to_string(),

            // Type conversions
            "to_string".to_string(),
            "to_int".to_string(),
            "parse_int".to_string(),
            "ord".to_string(),

            // Math functions (ALL SUPPORTED IN INTERPRETER)
            "floor".to_string(),
            "ceil".to_string(),
            "round".to_string(),
            "sqrt".to_string(),
            "sin".to_string(),      // ← Interpreter has this
            "cos".to_string(),      // ← Interpreter has this
            "tan".to_string(),      // ← Interpreter has this
            "log".to_string(),      // ← Interpreter has this
            "exp".to_string(),      // ← Interpreter has this
            "random".to_string(),

            // Object operations
            "has_key".to_string(),

            // JSON operations
            "json_parse".to_string(),
            "json_stringify".to_string(),
            "read_json".to_string(),
            "write_json".to_string(),

            // HTTP
            "http_request".to_string(),

            // Runtime introspection
            "snapshot".to_string(),
            "export_trace".to_string(),
            "write_snapshot".to_string(),

            // Process management
            "pipe_open".to_string(),
            "pipe_write".to_string(),
            "pipe_close".to_string(),

            // System operations
            "sleep".to_string(),
            "capture_screen".to_string(),

            // Metaprogramming
            "native_tokenize".to_string(),
        ]
    }

    fn backend_name(&self) -> &'static str {
        "Interpreter (JIT)"
    }
}

impl Executor {
    /// Static capability query - can be called without Executor instance
    ///
    /// Used by LSP to detect backend compatibility issues
    /// Interpreter is reference implementation with full stdlib
    pub fn static_supported_builtins() -> Vec<String> {
        vec![
            // I/O
            "print".to_string(),
            "read_file".to_string(),
            "write_file".to_string(),
            "file_exists".to_string(),
            "delete_file".to_string(),
            "list_files".to_string(),
            "create_dir".to_string(),
            // Type introspection
            "type".to_string(),
            "len".to_string(),
            // Array operations
            "slice".to_string(),
            "append".to_string(),
            "arr_pop".to_string(),
            "arr_slice".to_string(),
            "arr_concat".to_string(),
            // String operations
            "concat".to_string(),
            "strlen".to_string(),
            "substring".to_string(),
            "index_of".to_string(),
            "to_upper".to_string(),
            "to_lower".to_string(),
            "trim".to_string(),
            "starts_with".to_string(),
            "ends_with".to_string(),
            // Type conversions
            "to_string".to_string(),
            "to_int".to_string(),
            "parse_int".to_string(),
            "ord".to_string(),
            // Math functions (ALL SUPPORTED)
            "floor".to_string(),
            "ceil".to_string(),
            "round".to_string(),
            "sqrt".to_string(),
            "sin".to_string(),
            "cos".to_string(),
            "tan".to_string(),
            "log".to_string(),
            "exp".to_string(),
            "random".to_string(),
            // Object operations
            "has_key".to_string(),
            // JSON operations
            "json_parse".to_string(),
            "json_stringify".to_string(),
            "read_json".to_string(),
            "write_json".to_string(),
            // HTTP
            "http_request".to_string(),
            // Runtime introspection
            "snapshot".to_string(),
            "export_trace".to_string(),
            "write_snapshot".to_string(),
            // Process management
            "pipe_open".to_string(),
            "pipe_write".to_string(),
            "pipe_close".to_string(),
            // System operations
            "sleep".to_string(),
            "capture_screen".to_string(),
            // Metaprogramming
            "native_tokenize".to_string(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arithmetic() {
        let config = RuntimeConfig::default();
        let mut executor = Executor::new(&config).unwrap();
        
        let krate = HlxCrate::new(vec![
            Instruction::Constant { out: 0, val: Value::Integer(10) },
            Instruction::Constant { out: 1, val: Value::Integer(3) },
            Instruction::Add { out: 2, lhs: 0, rhs: 1 },
            Instruction::Mul { out: 3, lhs: 2, rhs: 0 },
            Instruction::Return { val: 3 },
        ]);
        
        let result = executor.run(&krate).unwrap();
        // (10 + 3) * 10 = 130
        assert_eq!(result, Value::Integer(130));
    }

    #[test]
    fn test_comparison() {
        let config = RuntimeConfig::default();
        let mut executor = Executor::new(&config).unwrap();
        
        let krate = HlxCrate::new(vec![
            Instruction::Constant { out: 0, val: Value::Integer(5) },
            Instruction::Constant { out: 1, val: Value::Integer(10) },
            Instruction::Lt { out: 2, lhs: 0, rhs: 1 },
            Instruction::Return { val: 2 },
        ]);
        
        let result = executor.run(&krate).unwrap();
        assert_eq!(result, Value::Boolean(true));
    }

    #[test]
    fn test_conditional_if() {
        let config = RuntimeConfig::default();
        let mut executor = Executor::new(&config).unwrap();
        
        // Logic: r0 = true; if r0 { return 100 } else { return 200 }
        let krate = HlxCrate::new(vec![
            Instruction::Constant { out: 0, val: Value::Boolean(true) }, // 0
            Instruction::If { cond: 0, then_block: 3, else_block: 5 },     // 1
            Instruction::Nop,                                            // 2
            Instruction::Constant { out: 1, val: Value::Integer(100) },    // 3
            Instruction::Return { val: 1 },                               // 4
            Instruction::Constant { out: 1, val: Value::Integer(200) },    // 5
            Instruction::Return { val: 1 },                               // 6
        ]);
        
        let result = executor.run(&krate).unwrap();
        assert_eq!(result, Value::Integer(100));
    }

    #[test]
    fn test_loop_dlb() {
        let config = RuntimeConfig::default();
        let mut executor = Executor::new(&config).unwrap();
        
        // Logic: 
        // r0 = 0
        // loop (r0 < 5, 10): r0 = r0 + 1
        let krate = HlxCrate::new(vec![
            Instruction::Constant { out: 0, val: Value::Integer(0) },    // 0
            Instruction::Constant { out: 1, val: Value::Integer(5) },    // 1
            Instruction::Constant { out: 2, val: Value::Integer(1) },    // 2
            Instruction::Lt { out: 3, lhs: 0, rhs: 1 },                  // 3: r3 = r0 < 5
            Instruction::Loop { cond: 3, body: 6, exit: 8, max_iter: 10 },        // 4: if r3 jump 6 else exit to 8
            Instruction::Jump { target: 8 },                             // 5: EXIT
            Instruction::Add { out: 0, lhs: 0, rhs: 2 },                 // 6: r0 = r0 + 1
            Instruction::Jump { target: 3 },                             // 7: jump back to condition
            Instruction::Return { val: 0 },                               // 8
        ]);
        
        let result = executor.run(&krate).unwrap();
        assert_eq!(result, Value::Integer(5));

        // Test DLB Panic:
        // Loop runs forever but max_iter is 10.
        let krate_panic = HlxCrate::new(vec![
            Instruction::Constant { out: 0, val: Value::Integer(0) },    // 0
            Instruction::Constant { out: 1, val: Value::Boolean(true) }, // 1
            Instruction::Loop { cond: 1, body: 2, exit: 4, max_iter: 10 },        // 2: Loop point, exit at 4
            Instruction::Jump { target: 2 },                             // 3: Jump back to loop
        ]);
        
        let err = executor.run(&krate_panic);
        assert!(err.is_err());
        let err_msg = format!("{:?}", err);
        assert!(err_msg.contains("Deterministic Loop Bound exceeded"));
    }
}
