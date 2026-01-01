//! Executor
//!
//! The main instruction dispatch engine. Takes LC-B capsules and
//! executes them deterministically against the configured backend.

use hlx_core::{Capsule, Instruction, Value, Result, HlxError};
use crate::config::RuntimeConfig;
use crate::backend::{Backend, create_backend, TensorHandle, DType};
use crate::value_store::ValueStore;
use std::collections::HashMap;

/// The executor runs LC-B capsules
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
    
    /// Run a capsule and return its result
    pub fn run(&self, capsule: &Capsule) -> Result<Value> {
        // Create execution context
        let mut ctx = ExecutionContext::new(&self.config)?;
        
        // Scan for function definitions first
        for inst in capsule.instructions.iter() {
            if let Instruction::FuncDef { name, body } = inst {
                ctx.functions.insert(name.clone(), *body);
            }
        }

        // Execute instructions
        let mut pc = 0;
        while pc < capsule.instructions.len() {
            let inst = &capsule.instructions[pc];
            
            if self.config.debug {
                tracing::debug!("PC {}: {:?}", pc, inst);
            }
            
            match ctx.execute_instruction(inst, pc)? {
                ControlFlow::Continue => pc += 1,
                ControlFlow::Jump(target) => pc = target as usize,
                ControlFlow::Return(val) => return Ok(val),
                ControlFlow::Call(target, ret) => {
                    ctx.call_stack.push(ret);
                    pc = target as usize;
                }
                ControlFlow::Ret => {
                    if let Some(ret_pc) = ctx.call_stack.pop() {
                        pc = ret_pc;
                    } else {
                        return Ok(Value::Null); // End of main
                    }
                }
            }
        }
        
        // If no explicit return, return Null
        Ok(Value::Null)
    }
}

/// Control flow result from instruction execution
enum ControlFlow {
    Continue,
    Jump(u32),
    Call(u32, usize), // target, return_pc
    Ret,
    Return(Value),
}

/// Execution context holding register state
struct ExecutionContext {
    /// Register file (maps register number to value)
    registers: HashMap<u32, Value>,
    
    /// Function table (name -> start_index)
    functions: HashMap<String, u32>,
    
    /// Call stack (return addresses)
    call_stack: Vec<usize>,

    /// Loop counters (PC -> count) for DLB
    loop_counters: HashMap<u32, u32>,
    
    /// Tensor handles for GPU operations
    tensors: HashMap<u32, TensorHandle>,
    
    /// Content-addressed storage for handles
    cas: ValueStore,
    
    /// Backend reference (we'll use a simple approach for now)
    config: RuntimeConfig,
}

impl ExecutionContext {
    fn new(config: &RuntimeConfig) -> Result<Self> {
        Ok(Self {
            registers: HashMap::new(),
            functions: HashMap::new(),
            call_stack: Vec::new(),
            loop_counters: HashMap::new(),
            tensors: HashMap::new(),
            cas: ValueStore::new(),
            config: config.clone(),
        })
    }
    
    fn get_reg(&self, reg: u32) -> Result<&Value> {
        self.registers.get(&reg).ok_or_else(|| HlxError::ValidationFail {
            message: format!("Register {} not defined", reg),
        })
    }
    
    fn set_reg(&mut self, reg: u32, val: Value) {
        self.registers.insert(reg, val);
    }
    
    fn execute_instruction(&mut self, inst: &Instruction, pc: usize) -> Result<ControlFlow> {
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

            Instruction::Loop { cond, body, max_iter } => {
                let condition = self.get_bool(*cond)?;
                
                // Deterministic Loop Bound Check
                let count = self.loop_counters.entry(pc as u32).or_insert(0);
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
                    *count = 0; 
                    return Ok(ControlFlow::Continue);
                }
            }

            Instruction::FuncDef { .. } => {
                // Function definitions are skipped during execution
                return Ok(ControlFlow::Continue);
            }

            Instruction::Call { out, func, args } => {
                // Check if it's a user function
                if let Some(&target) = self.functions.get(func) {
                    return Ok(ControlFlow::Call(target, pc + 1));
                } else {
                    // Built-in function dispatch
                    let result = self.call_builtin(func, args)?;
                    self.set_reg(*out, result);
                    return Ok(ControlFlow::Continue);
                }
            }

            // === Constants ===
            Instruction::Constant { out, val } => {
                self.set_reg(*out, val.clone());
            }

            Instruction::Move { out, src } => {
                let val = self.get_reg(*src)?.clone();
                self.set_reg(*out, val);
            }
            
            // === Arithmetic ===
            Instruction::Add { out, lhs, rhs } => {
                let a = self.get_reg(*lhs)?;
                let b = self.get_reg(*rhs)?;
                let result = self.scalar_add(a, b)?;
                self.set_reg(*out, result);
            }
            
            Instruction::Sub { out, lhs, rhs } => {
                let a = self.get_reg(*lhs)?;
                let b = self.get_reg(*rhs)?;
                let result = self.scalar_sub(a, b)?;
                self.set_reg(*out, result);
            }
            
            Instruction::Mul { out, lhs, rhs } => {
                let a = self.get_reg(*lhs)?;
                let b = self.get_reg(*rhs)?;
                let result = self.scalar_mul(a, b)?;
                self.set_reg(*out, result);
            }
            
            Instruction::Div { out, lhs, rhs } => {
                let a = self.get_reg(*lhs)?;
                let b = self.get_reg(*rhs)?;
                let result = self.scalar_div(a, b)?;
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
                self.set_reg(*out, Value::Boolean(a == b));
            }
            
            Instruction::Ne { out, lhs, rhs } => {
                let a = self.get_reg(*lhs)?;
                let b = self.get_reg(*rhs)?;
                self.set_reg(*out, Value::Boolean(a != b));
            }
            
            Instruction::Lt { out, lhs, rhs } => {
                let a = self.get_reg(*lhs)?;
                let b = self.get_reg(*rhs)?;
                let result = self.compare_lt(a, b)?;
                self.set_reg(*out, Value::Boolean(result));
            }
            
            Instruction::Le { out, lhs, rhs } => {
                let a = self.get_reg(*lhs)?;
                let b = self.get_reg(*rhs)?;
                let result = self.compare_le(a, b)?;
                self.set_reg(*out, Value::Boolean(result));
            }
            
            Instruction::Gt { out, lhs, rhs } => {
                let a = self.get_reg(*lhs)?;
                let b = self.get_reg(*rhs)?;
                let result = self.compare_lt(b, a)?; // Flip operands
                self.set_reg(*out, Value::Boolean(result));
            }
            
            Instruction::Ge { out, lhs, rhs } => {
                let a = self.get_reg(*lhs)?;
                let b = self.get_reg(*rhs)?;
                let result = self.compare_le(b, a)?; // Flip operands
                self.set_reg(*out, Value::Boolean(result));
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
                let result = self.get_reg(*val)?.clone();
                return Ok(ControlFlow::Return(result));
            }
            
            // === I/O ===
            Instruction::Print { val } => {
                let v = self.get_reg(*val)?;
                println!("{}", v);
            }
            
            Instruction::TypeOf { out, val } => {
                let v = self.get_reg(*val)?;
                self.set_reg(*out, Value::String(v.type_name().to_string()));
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
                let snapshot = Value::Object(
                    self.registers.iter()
                        .map(|(k, v)| (format!("r{}", k), v.clone()))
                        .collect()
                );
                let handle = self.cas.store(snapshot)?;
                self.set_reg(*handle_out, Value::Handle(handle));
            }
            
            // === Tensor Operations (CPU fallback) ===
            Instruction::MatMul { out, lhs, rhs } => {
                // For now, scalar multiplication as placeholder
                // Full tensor support requires backend integration
                let a = self.get_reg(*lhs)?;
                let b = self.get_reg(*rhs)?;
                let result = self.scalar_mul(a, b)?;
                self.set_reg(*out, result);
            }
            
            Instruction::Gelu { out, input } => {
                let x = self.get_reg(*input)?;
                let result = self.apply_gelu(x)?;
                self.set_reg(*out, result);
            }
            
            Instruction::Relu { out, input } => {
                let x = self.get_reg(*input)?;
                let result = self.apply_relu(x)?;
                self.set_reg(*out, result);
            }
            
            Instruction::Softmax { out, input, dim: _ } => {
                // Simplified: just return input (full implementation needs backend)
                let x = self.get_reg(*input)?.clone();
                self.set_reg(*out, x);
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
        match self.get_reg(reg)? {
            Value::Boolean(b) => Ok(*b),
            v => Err(HlxError::TypeError {
                expected: "boolean".to_string(),
                got: v.type_name().to_string(),
            }),
        }
    }
    
    fn scalar_add(&self, a: &Value, b: &Value) -> Result<Value> {
        match (a, b) {
            (Value::Integer(x), Value::Integer(y)) => Ok(Value::Integer(x + y)),
            (Value::Float(x), Value::Float(y)) => Value::float(x + y),
            (Value::Integer(x), Value::Float(y)) => Value::float(*x as f64 + y),
            (Value::Float(x), Value::Integer(y)) => Value::float(x + *y as f64),
            (Value::String(x), Value::String(y)) => Ok(Value::String(format!("{}{}", x, y))),
            _ => Err(HlxError::TypeError {
                expected: "numeric or string".to_string(),
                got: format!("{} + {}", a.type_name(), b.type_name()),
            }),
        }
    }
    
    fn scalar_sub(&self, a: &Value, b: &Value) -> Result<Value> {
        match (a, b) {
            (Value::Integer(x), Value::Integer(y)) => Ok(Value::Integer(x - y)),
            (Value::Float(x), Value::Float(y)) => Value::float(x - y),
            (Value::Integer(x), Value::Float(y)) => Value::float(*x as f64 - y),
            (Value::Float(x), Value::Integer(y)) => Value::float(x - *y as f64),
            _ => Err(HlxError::TypeError {
                expected: "numeric".to_string(),
                got: format!("{} - {}", a.type_name(), b.type_name()),
            }),
        }
    }
    
    fn scalar_mul(&self, a: &Value, b: &Value) -> Result<Value> {
        match (a, b) {
            (Value::Integer(x), Value::Integer(y)) => Ok(Value::Integer(x * y)),
            (Value::Float(x), Value::Float(y)) => Value::float(x * y),
            (Value::Integer(x), Value::Float(y)) => Value::float(*x as f64 * y),
            (Value::Float(x), Value::Integer(y)) => Value::float(x * *y as f64),
            _ => Err(HlxError::TypeError {
                expected: "numeric".to_string(),
                got: format!("{} * {}", a.type_name(), b.type_name()),
            }),
        }
    }
    
    fn scalar_div(&self, a: &Value, b: &Value) -> Result<Value> {
        match (a, b) {
            (Value::Integer(x), Value::Integer(y)) => {
                if *y == 0 {
                    return Err(HlxError::ValidationFail {
                        message: "Division by zero".to_string(),
                    });
                }
                // Integer division yields float
                Value::float(*x as f64 / *y as f64)
            }
            (Value::Float(x), Value::Float(y)) => {
                if *y == 0.0 {
                    return Err(HlxError::ValidationFail {
                        message: "Division by zero".to_string(),
                    });
                }
                Value::float(x / y)
            }
            (Value::Integer(x), Value::Float(y)) => {
                if *y == 0.0 {
                    return Err(HlxError::ValidationFail {
                        message: "Division by zero".to_string(),
                    });
                }
                Value::float(*x as f64 / y)
            }
            (Value::Float(x), Value::Integer(y)) => {
                if *y == 0 {
                    return Err(HlxError::ValidationFail {
                        message: "Division by zero".to_string(),
                    });
                }
                Value::float(x / *y as f64)
            }
            _ => Err(HlxError::TypeError {
                expected: "numeric".to_string(),
                got: format!("{} / {}", a.type_name(), b.type_name()),
            }),
        }
    }
    
    fn compare_lt(&self, a: &Value, b: &Value) -> Result<bool> {
        match (a, b) {
            (Value::Integer(x), Value::Integer(y)) => Ok(x < y),
            (Value::Float(x), Value::Float(y)) => Ok(x < y),
            (Value::Integer(x), Value::Float(y)) => Ok((*x as f64) < *y),
            (Value::Float(x), Value::Integer(y)) => Ok(*x < (*y as f64)),
            (Value::String(x), Value::String(y)) => Ok(x < y),
            _ => Err(HlxError::TypeError {
                expected: "comparable".to_string(),
                got: format!("{} < {}", a.type_name(), b.type_name()),
            }),
        }
    }
    
    fn compare_le(&self, a: &Value, b: &Value) -> Result<bool> {
        match (a, b) {
            (Value::Integer(x), Value::Integer(y)) => Ok(x <= y),
            (Value::Float(x), Value::Float(y)) => Ok(x <= y),
            (Value::Integer(x), Value::Float(y)) => Ok((*x as f64) <= *y),
            (Value::Float(x), Value::Integer(y)) => Ok(*x <= (*y as f64)),
            (Value::String(x), Value::String(y)) => Ok(x <= y),
            _ => Err(HlxError::TypeError {
                expected: "comparable".to_string(),
                got: format!("{} <= {}", a.type_name(), b.type_name()),
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
                    message: format!("Key '{}' not found", key),
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
            _ => Err(HlxError::ValidationFail {
                message: format!("Unknown function: {}", func),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arithmetic() {
        let config = RuntimeConfig::default();
        let executor = Executor::new(&config).unwrap();
        
        let capsule = Capsule::new(vec![
            Instruction::Constant { out: 0, val: Value::Integer(10) },
            Instruction::Constant { out: 1, val: Value::Integer(3) },
            Instruction::Add { out: 2, lhs: 0, rhs: 1 },
            Instruction::Mul { out: 3, lhs: 2, rhs: 0 },
            Instruction::Return { val: 3 },
        ]);
        
        let result = executor.run(&capsule).unwrap();
        // (10 + 3) * 10 = 130
        assert_eq!(result, Value::Integer(130));
    }

    #[test]
    fn test_comparison() {
        let config = RuntimeConfig::default();
        let executor = Executor::new(&config).unwrap();
        
        let capsule = Capsule::new(vec![
            Instruction::Constant { out: 0, val: Value::Integer(5) },
            Instruction::Constant { out: 1, val: Value::Integer(10) },
            Instruction::Lt { out: 2, lhs: 0, rhs: 1 },
            Instruction::Return { val: 2 },
        ]);
        
        let result = executor.run(&capsule).unwrap();
        assert_eq!(result, Value::Boolean(true));
    }

    #[test]
    fn test_conditional_if() {
        let config = RuntimeConfig::default();
        let executor = Executor::new(&config).unwrap();
        
        // Logic: r0 = true; if r0 { return 100 } else { return 200 }
        let capsule = Capsule::new(vec![
            Instruction::Constant { out: 0, val: Value::Boolean(true) },
            Instruction::If { cond: 0, then_block: 2, else_block: 3 },
            Instruction::Return { val: 4 }, // Target 2 (Return 100) - wait, target is index
            Instruction::Return { val: 5 }, // Target 3 (Return 200)
            Instruction::Constant { out: 4, val: Value::Integer(100) }, // This is index 4
            Instruction::Constant { out: 5, val: Value::Integer(200) }, // This is index 5
        ]);

        // Corrected logic for flat stream:
        // 0: r0 = true
        // 1: if r0 then jump to 4 else jump to 6
        // 2: (gap)
        // 3: (gap)
        // 4: r1 = 100
        // 5: return r1
        // 6: r2 = 200
        // 7: return r2
        let capsule = Capsule::new(vec![
            Instruction::Constant { out: 0, val: Value::Boolean(true) }, // 0
            Instruction::If { cond: 0, then_block: 3, else_block: 5 },     // 1
            Instruction::Nop,                                            // 2
            Instruction::Constant { out: 1, val: Value::Integer(100) },    // 3
            Instruction::Return { val: 1 },                               // 4
            Instruction::Constant { out: 1, val: Value::Integer(200) },    // 5
            Instruction::Return { val: 1 },                               // 6
        ]);
        
        let result = executor.run(&capsule).unwrap();
        assert_eq!(result, Value::Integer(100));
    }

    #[test]
    fn test_loop_dlb() {
        let config = RuntimeConfig::default();
        let executor = Executor::new(&config).unwrap();
        
        // Logic: 
        // r0 = 0
        // loop (r0 < 5, 10): r0 = r0 + 1
        let capsule = Capsule::new(vec![
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
        
        let result = executor.run(&capsule).unwrap();
        assert_eq!(result, Value::Integer(5));

        // Test DLB Panic:
        // Loop runs forever but max_iter is 10.
        let capsule_panic = Capsule::new(vec![
            Instruction::Constant { out: 0, val: Value::Integer(0) },    // 0
            Instruction::Constant { out: 1, val: Value::Boolean(true) }, // 1
            Instruction::Loop { cond: 1, body: 2, max_iter: 10 },        // 2: Loop point
            Instruction::Jump { target: 2 },                             // 3: Jump back to loop
        ]);
        
        let err = executor.run(&capsule_panic);
        assert!(err.is_err());
        let err_msg = format!("{:?}", err);
        assert!(err_msg.contains("Deterministic Loop Bound exceeded"));
    }
}
