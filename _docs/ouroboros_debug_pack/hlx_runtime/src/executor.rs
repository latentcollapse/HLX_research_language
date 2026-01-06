//! Executor
//!
//! The main instruction dispatch engine. Takes LC-B crates and
//! executes them deterministically against the configured backend.

use hlx_core::{HlxCrate, Instruction, Value, Result, HlxError};
use crate::config::RuntimeConfig;
use crate::backend::{Backend, create_backend, TensorHandle};
use crate::value_store::ValueStore;
use std::collections::HashMap;
use im::{Vector, OrdMap};

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
        // Create execution context
        let mut ctx = ExecutionContext::new(&self.config);
        
        // Scan for function definitions first
        for inst in krate.instructions.iter() {
            if let Instruction::FuncDef { name, params, body } = inst {
                if self.config.debug { println!("Registered function: {}", name); }
                ctx.functions.insert(name.clone(), (*body, params.clone()));
            }
        }

        // Execute instructions
        let mut pc = if let Some((start, _)) = ctx.functions.get("main") {
            *start as usize
        } else {
            0
        };
        
        while pc < krate.instructions.len() {
            let inst = &krate.instructions[pc];
            match ctx.execute_instruction(inst, pc, &mut *self.backend)? {
                ControlFlow::Continue => pc += 1,
                ControlFlow::Jump(target) => {
                    if target == u32::MAX {
                        break; 
                    }
                    pc = target as usize;
                }
                ControlFlow::Break => {
                    break;
                }
                ControlFlow::ContinueIter => {
                    pc += 1;
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
    Break,
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
                    let res = self.call_builtin(func, args)?;
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
            
            Instruction::ArrayCreate { out, elements } => {
                let mut vals = Vector::new();
                for &reg in elements {
                    vals.push_back(self.get_reg(reg)?.clone());
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
                // Create a snapshot of all registers
                let regs_map = if let Some(frame) = self.call_stack.last() {
                    frame.registers.clone()
                } else {
                    HashMap::new()
                };
                
                let mut map = OrdMap::new();
                for (k, v) in regs_map {
                    map.insert(format!("r{}", k), v);
                }

                let snapshot = Value::Object(map);
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
    
    fn call_builtin(&mut self, func: &str, args: &[u32]) -> Result<Value> {
        match func {
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
    
    Ok(Value::Array(tokens))
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
            Instruction::Loop { cond: 3, body: 6, max_iter: 10 },        // 4: if r3 jump 6 else continue
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
            Instruction::Loop { cond: 1, body: 2, max_iter: 10 },        // 2: Loop point
            Instruction::Jump { target: 2 },                             // 3: Jump back to loop
        ]);
        
        let err = executor.run(&krate_panic);
        assert!(err.is_err());
        let err_msg = format!("{:?}", err);
        assert!(err_msg.contains("Deterministic Loop Bound exceeded"));
    }
}
