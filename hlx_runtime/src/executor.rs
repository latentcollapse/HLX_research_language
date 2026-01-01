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
        
        // Execute instructions
        for (idx, inst) in capsule.instructions.iter().enumerate() {
            if self.config.debug {
                tracing::debug!("Executing instruction {}: {:?}", idx, inst);
            }
            
            match ctx.execute_instruction(inst)? {
                ControlFlow::Continue => continue,
                ControlFlow::Return(val) => return Ok(val),
            }
        }
        
        // If no explicit return, return Null
        Ok(Value::Null)
    }
}

/// Control flow result from instruction execution
enum ControlFlow {
    Continue,
    Return(Value),
}

/// Execution context holding register state
struct ExecutionContext {
    /// Register file (maps register number to value)
    registers: HashMap<u32, Value>,
    
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
    
    fn execute_instruction(&mut self, inst: &Instruction) -> Result<ControlFlow> {
        match inst {
            // === Constants ===
            Instruction::Constant { out, val } => {
                self.set_reg(*out, val.clone());
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
            
            // === Function Calls ===
            Instruction::Call { out, func, args } => {
                // Built-in function dispatch
                let result = self.call_builtin(func, args)?;
                self.set_reg(*out, result);
            }
            
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
    fn test_cas_roundtrip() {
        let config = RuntimeConfig::default();
        let executor = Executor::new(&config).unwrap();
        
        let capsule = Capsule::new(vec![
            Instruction::Constant { out: 0, val: Value::String("test data".to_string()) },
            Instruction::Collapse { handle_out: 1, val: 0 },
            Instruction::Resolve { val_out: 2, handle: 1 },
            Instruction::Return { val: 2 },
        ]);
        
        let result = executor.run(&capsule).unwrap();
        assert_eq!(result, Value::String("test data".to_string()));
    }
}
