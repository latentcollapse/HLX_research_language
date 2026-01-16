//! CPU Backend
//!
//! Reference implementation using ndarray for tensor operations.
//! Prioritizes correctness and determinism over performance.

use hlx_core::{Value, Result, HlxError};
use crate::backend::{Backend, TensorHandle, TensorMeta, DType};
use crate::config::RuntimeConfig;
use ndarray::{ArrayD, IxDyn};
use std::collections::HashMap;

/// CPU backend using ndarray
pub struct CpuBackend {
    /// Next tensor handle to allocate
    next_handle: u64,
    
    /// Tensor storage
    tensors: HashMap<TensorHandle, TensorStorage>,

    /// Configuration (planned for future use)
    #[allow(dead_code)]
    config: RuntimeConfig,
}

/// Storage for a tensor
enum TensorStorage {
    F32(ArrayD<f32>),
    F64(ArrayD<f64>),
    I32(ArrayD<i32>),
    I64(ArrayD<i64>),
    Bool(ArrayD<bool>),
}

impl TensorStorage {
    fn dtype(&self) -> DType {
        match self {
            TensorStorage::F32(_) => DType::F32,
            TensorStorage::F64(_) => DType::F64,
            TensorStorage::I32(_) => DType::I32,
            TensorStorage::I64(_) => DType::I64,
            TensorStorage::Bool(_) => DType::Bool,
        }
    }
    
    fn shape(&self) -> Vec<usize> {
        match self {
            TensorStorage::F32(a) => a.shape().to_vec(),
            TensorStorage::F64(a) => a.shape().to_vec(),
            TensorStorage::I32(a) => a.shape().to_vec(),
            TensorStorage::I64(a) => a.shape().to_vec(),
            TensorStorage::Bool(a) => a.shape().to_vec(),
        }
    }
    
    fn size_bytes(&self) -> usize {
        let elements: usize = self.shape().iter().product();
        elements * self.dtype().size_bytes()
    }
}

impl CpuBackend {
    /// Create a new CPU backend
    pub fn new(config: &RuntimeConfig) -> Result<Self> {
        Ok(Self {
            next_handle: 0,
            tensors: HashMap::new(),
            config: config.clone(),
        })
    }
    
    fn alloc_handle(&mut self) -> TensorHandle {
        let handle = TensorHandle(self.next_handle);
        self.next_handle += 1;
        handle
    }
    
    fn get_tensor(&self, handle: TensorHandle) -> Result<&TensorStorage> {
        self.tensors.get(&handle).ok_or_else(|| HlxError::ValidationFail {
            message: format!("Tensor handle {:?} not got", handle),
        })
    }
    
    fn get_tensor_mut(&mut self, handle: TensorHandle) -> Result<&mut TensorStorage> {
        self.tensors.get_mut(&handle).ok_or_else(|| HlxError::ValidationFail {
            message: format!("Tensor handle {:?} not got", handle),
        })
    }
    
    fn get_f32(&self, handle: TensorHandle) -> Result<&ArrayD<f32>> {
        match self.get_tensor(handle)? {
            TensorStorage::F32(a) => Ok(a),
            _ => Err(HlxError::TypeError {
                expected: "f32 tensor".to_string(),
                got: "other dtype".to_string(),
            }),
        }
    }
    
    fn get_f32_mut(&mut self, handle: TensorHandle) -> Result<&mut ArrayD<f32>> {
        match self.get_tensor_mut(handle)? {
            TensorStorage::F32(a) => Ok(a),
            _ => Err(HlxError::TypeError {
                expected: "f32 tensor".to_string(),
                got: "other dtype".to_string(),
            }),
        }
    }
}

impl Backend for CpuBackend {
    fn name(&self) -> &'static str {
        "cpu"
    }
    
    fn is_available(&self) -> bool {
        true // CPU is always available
    }
    
    fn alloc_tensor(&mut self, shape: &[usize], dtype: DType) -> Result<TensorHandle> {
        let handle = self.alloc_handle();
        
        let storage = match dtype {
            DType::F32 => TensorStorage::F32(ArrayD::zeros(IxDyn(shape))),
            DType::F64 => TensorStorage::F64(ArrayD::zeros(IxDyn(shape))),
            DType::I32 => TensorStorage::I32(ArrayD::zeros(IxDyn(shape))),
            DType::I64 => TensorStorage::I64(ArrayD::zeros(IxDyn(shape))),
            DType::Bool => TensorStorage::Bool(ArrayD::default(IxDyn(shape))),
        };
        
        self.tensors.insert(handle, storage);
        Ok(handle)
    }
    
    fn free_tensor(&mut self, handle: TensorHandle) -> Result<()> {
        self.tensors.remove(&handle);
        Ok(())
    }
    
    fn tensor_meta(&self, handle: TensorHandle) -> Result<TensorMeta> {
        let tensor = self.get_tensor(handle)?;
        Ok(TensorMeta {
            shape: tensor.shape(),
            dtype: tensor.dtype(),
            size_bytes: tensor.size_bytes(),
        })
    }
    
    fn write_tensor(&mut self, handle: TensorHandle, data: &[u8]) -> Result<()> {
        let tensor = self.get_tensor_mut(handle)?;
        
        match tensor {
            TensorStorage::F32(arr) => {
                let floats: Vec<f32> = data.chunks_exact(4)
                    .map(|chunk| f32::from_le_bytes(chunk.try_into().unwrap()))
                    .collect();
                for (dst, src) in arr.iter_mut().zip(floats.iter()) {
                    *dst = *src;
                }
            }
            TensorStorage::F64(arr) => {
                let floats: Vec<f64> = data.chunks_exact(8)
                    .map(|chunk| f64::from_le_bytes(chunk.try_into().unwrap()))
                    .collect();
                for (dst, src) in arr.iter_mut().zip(floats.iter()) {
                    *dst = *src;
                }
            }
            TensorStorage::I32(arr) => {
                let ints: Vec<i32> = data.chunks_exact(4)
                    .map(|chunk| i32::from_le_bytes(chunk.try_into().unwrap()))
                    .collect();
                for (dst, src) in arr.iter_mut().zip(ints.iter()) {
                    *dst = *src;
                }
            }
            TensorStorage::I64(arr) => {
                let ints: Vec<i64> = data.chunks_exact(8)
                    .map(|chunk| i64::from_le_bytes(chunk.try_into().unwrap()))
                    .collect();
                for (dst, src) in arr.iter_mut().zip(ints.iter()) {
                    *dst = *src;
                }
            }
            TensorStorage::Bool(arr) => {
                for (dst, src) in arr.iter_mut().zip(data.iter()) {
                    *dst = *src != 0;
                }
            }
        }
        
        Ok(())
    }
    
    fn read_tensor(&self, handle: TensorHandle) -> Result<Vec<u8>> {
        let tensor = self.get_tensor(handle)?;
        
        let bytes = match tensor {
            TensorStorage::F32(arr) => {
                arr.iter().flat_map(|f| f.to_le_bytes()).collect()
            }
            TensorStorage::F64(arr) => {
                arr.iter().flat_map(|f| f.to_le_bytes()).collect()
            }
            TensorStorage::I32(arr) => {
                arr.iter().flat_map(|i| i.to_le_bytes()).collect()
            }
            TensorStorage::I64(arr) => {
                arr.iter().flat_map(|i| i.to_le_bytes()).collect()
            }
            TensorStorage::Bool(arr) => {
                arr.iter().map(|b| if *b { 1u8 } else { 0u8 }).collect()
            }
        };
        
        Ok(bytes)
    }
    
    // === Scalar Operations ===
    
    fn scalar_add(&mut self, a: &Value, b: &Value) -> Result<Value> { a.add(b) }
    fn scalar_sub(&mut self, a: &Value, b: &Value) -> Result<Value> { a.sub(b) }
    fn scalar_mul(&mut self, a: &Value, b: &Value) -> Result<Value> { a.mul(b) }
    fn scalar_div(&mut self, a: &Value, b: &Value) -> Result<Value> { a.div(b) }
    fn scalar_mod(&mut self, a: &Value, b: &Value) -> Result<Value> { a.rem(b) }

    // === Math Functions ===

    fn scalar_sqrt(&mut self, a: &Value) -> Result<Value> {
        match a {
            Value::Float(f) => Ok(Value::Float((*f as f64).sqrt())),
            Value::Integer(i) => Ok(Value::Float((*i as f64).sqrt())),
            _ => Err(hlx_core::HlxError::TypeError { expected: "number".to_string(), got: a.type_name().to_string() }),
        }
    }

    fn scalar_pow(&mut self, base: &Value, exp: &Value) -> Result<Value> {
        match (base, exp) {
            (Value::Float(b), Value::Float(e)) => Ok(Value::Float((*b as f64).powf(*e as f64))),
            (Value::Float(b), Value::Integer(e)) => Ok(Value::Float((*b as f64).powi(*e as i32))),
            (Value::Integer(b), Value::Integer(e)) => {
                if *e >= 0 {
                    Ok(Value::Integer((*b as i64).pow(*e as u32)))
                } else {
                    Ok(Value::Float((*b as f64).powf(*e as f64)))
                }
            }
            (Value::Integer(b), Value::Float(e)) => Ok(Value::Float((*b as f64).powf(*e as f64))),
            _ => Err(hlx_core::HlxError::TypeError { expected: "number".to_string(), got: base.type_name().to_string() }),
        }
    }

    fn scalar_sin(&mut self, a: &Value) -> Result<Value> {
        match a {
            Value::Float(f) => Ok(Value::Float((*f as f64).sin())),
            Value::Integer(i) => Ok(Value::Float((*i as f64).sin())),
            _ => Err(hlx_core::HlxError::TypeError { expected: "number".to_string(), got: a.type_name().to_string() }),
        }
    }

    fn scalar_cos(&mut self, a: &Value) -> Result<Value> {
        match a {
            Value::Float(f) => Ok(Value::Float((*f as f64).cos())),
            Value::Integer(i) => Ok(Value::Float((*i as f64).cos())),
            _ => Err(hlx_core::HlxError::TypeError { expected: "number".to_string(), got: a.type_name().to_string() }),
        }
    }

    fn scalar_tan(&mut self, a: &Value) -> Result<Value> {
        match a {
            Value::Float(f) => Ok(Value::Float((*f as f64).tan())),
            Value::Integer(i) => Ok(Value::Float((*i as f64).tan())),
            _ => Err(hlx_core::HlxError::TypeError { expected: "number".to_string(), got: a.type_name().to_string() }),
        }
    }

    fn scalar_log(&mut self, a: &Value) -> Result<Value> {
        match a {
            Value::Float(f) => Ok(Value::Float((*f as f64).ln())),
            Value::Integer(i) => Ok(Value::Float((*i as f64).ln())),
            _ => Err(hlx_core::HlxError::TypeError { expected: "number".to_string(), got: a.type_name().to_string() }),
        }
    }

    fn scalar_exp(&mut self, a: &Value) -> Result<Value> {
        match a {
            Value::Float(f) => Ok(Value::Float((*f as f64).exp())),
            Value::Integer(i) => Ok(Value::Float((*i as f64).exp())),
            _ => Err(hlx_core::HlxError::TypeError { expected: "number".to_string(), got: a.type_name().to_string() }),
        }
    }

    fn scalar_floor(&mut self, a: &Value) -> Result<Value> {
        match a {
            Value::Float(f) => Ok(Value::Integer((*f as f64).floor() as i64)),
            Value::Integer(i) => Ok(Value::Integer(*i)),
            _ => Err(hlx_core::HlxError::TypeError { expected: "number".to_string(), got: a.type_name().to_string() }),
        }
    }

    fn scalar_ceil(&mut self, a: &Value) -> Result<Value> {
        match a {
            Value::Float(f) => Ok(Value::Integer((*f as f64).ceil() as i64)),
            Value::Integer(i) => Ok(Value::Integer(*i)),
            _ => Err(hlx_core::HlxError::TypeError { expected: "number".to_string(), got: a.type_name().to_string() }),
        }
    }

    fn scalar_round(&mut self, a: &Value) -> Result<Value> {
        match a {
            Value::Float(f) => Ok(Value::Integer((*f as f64).round() as i64)),
            Value::Integer(i) => Ok(Value::Integer(*i)),
            _ => Err(hlx_core::HlxError::TypeError { expected: "number".to_string(), got: a.type_name().to_string() }),
        }
    }

    fn scalar_abs(&mut self, a: &Value) -> Result<Value> {
        match a {
            Value::Float(f) => Ok(Value::Float((*f as f64).abs())),
            Value::Integer(i) => Ok(Value::Integer((*i as i64).abs())),
            _ => Err(hlx_core::HlxError::TypeError { expected: "number".to_string(), got: a.type_name().to_string() }),
        }
    }

    // === Comparison Operations ===
    
    fn scalar_eq(&mut self, a: &Value, b: &Value) -> Result<Value> { Ok(Value::Boolean(a == b)) }
    fn scalar_ne(&mut self, a: &Value, b: &Value) -> Result<Value> { Ok(Value::Boolean(a != b)) }
    fn scalar_lt(&mut self, a: &Value, b: &Value) -> Result<Value> { Ok(Value::Boolean(a.lt(b)?)) }
    fn scalar_le(&mut self, a: &Value, b: &Value) -> Result<Value> { Ok(Value::Boolean(a.le(b)?)) }
    fn scalar_gt(&mut self, a: &Value, b: &Value) -> Result<Value> { Ok(Value::Boolean(!a.le(b)?)) }
    fn scalar_ge(&mut self, a: &Value, b: &Value) -> Result<Value> { Ok(Value::Boolean(!a.lt(b)?)) }
    
    // === Tensor Operations ===

    fn pointwise_add(
        &mut self,
        a: TensorHandle,
        b: TensorHandle,
        out: TensorHandle,
    ) -> Result<()> {
        let a_arr = self.get_f32(a)?.clone();
        let b_arr = self.get_f32(b)?.clone();
        let out_arr = self.get_f32_mut(out)?;
        
        if a_arr.shape() != b_arr.shape() || a_arr.shape() != out_arr.shape() {
            return Err(HlxError::ValidationFail {
                message: "Pointwise add shape mismatch".to_string(),
            });
        }

        for ((o, a), b) in out_arr.iter_mut().zip(a_arr.iter()).zip(b_arr.iter()) {
            *o = *a + *b;
        }
        
        Ok(())
    }
    
    fn matmul(
        &mut self,
        a: TensorHandle,
        b: TensorHandle,
        out: TensorHandle,
    ) -> Result<()> {
        let a_arr = self.get_f32(a)?.clone();
        let b_arr = self.get_f32(b)?.clone();
        
        // Simple 2D matrix multiply
        let a_shape = a_arr.shape();
        let b_shape = b_arr.shape();
        
        if a_shape.len() != 2 || b_shape.len() != 2 {
            return Err(HlxError::ValidationFail {
                message: "matmul requires 2D tensors".to_string(),
            });
        }
        
        if a_shape[1] != b_shape[0] {
            return Err(HlxError::ValidationFail {
                message: format!(
                    "matmul dimension mismatch: ({}, {}) @ ({}, {})",
                    a_shape[0], a_shape[1], b_shape[0], b_shape[1]
                ),
            });
        }
        
        let m = a_shape[0];
        let k = a_shape[1];
        let n = b_shape[1];
        
        let out_arr = self.get_f32_mut(out)?;
        
        // Deterministic matrix multiply (fixed iteration order)
        for i in 0..m {
            for j in 0..n {
                let mut sum = 0.0f32;
                for l in 0..k {
                    sum += a_arr[[i, l]] * b_arr[[l, j]];
                }
                out_arr[[i, j]] = sum;
            }
        }
        
        Ok(())
    }
    
    fn matmul_bias(
        &mut self,
        a: TensorHandle,
        b: TensorHandle,
        bias: TensorHandle,
        out: TensorHandle,
    ) -> Result<()> {
        self.matmul(a, b, out)?;
        
        let bias_arr = self.get_f32(bias)?.clone();
        let out_arr = self.get_f32_mut(out)?;
        
        let out_shape = out_arr.shape().to_vec();
        let n = out_shape[1];
        
        for i in 0..out_shape[0] {
            for j in 0..n {
                out_arr[[i, j]] += bias_arr[j];
            }
        }
        
        Ok(())
    }
    
    fn layer_norm(
        &mut self,
        input: TensorHandle,
        gamma: TensorHandle,
        beta: TensorHandle,
        out: TensorHandle,
        eps: f64,
    ) -> Result<()> {
        let input_arr = self.get_f32(input)?.clone();
        let gamma_arr = self.get_f32(gamma)?.clone();
        let beta_arr = self.get_f32(beta)?.clone();
        let out_arr = self.get_f32_mut(out)?;
        
        let shape = input_arr.shape();
        let d_model = shape[shape.len() - 1];
        
        // For each position, compute mean and variance
        let batch_size: usize = shape.iter().take(shape.len() - 1).product();
        
        for b in 0..batch_size {
            // Compute mean
            let mut sum = 0.0f64;
            for i in 0..d_model {
                sum += input_arr.as_slice().unwrap()[b * d_model + i] as f64;
            }
            let mean = sum / d_model as f64;
            
            // Compute variance
            let mut var_sum = 0.0f64;
            for i in 0..d_model {
                let diff = input_arr.as_slice().unwrap()[b * d_model + i] as f64 - mean;
                var_sum += diff * diff;
            }
            let variance = var_sum / d_model as f64;
            let inv_std = 1.0 / (variance + eps).sqrt();
            
            // Normalize and apply affine transform
            for i in 0..d_model {
                let x = input_arr.as_slice().unwrap()[b * d_model + i] as f64;
                let normalized = (x - mean) * inv_std;
                let out_val = normalized * gamma_arr[i] as f64 + beta_arr[i] as f64;
                out_arr.as_slice_mut().unwrap()[b * d_model + i] = out_val as f32;
            }
        }
        
        Ok(())
    }
    
    fn softmax(
        &mut self,
        input: TensorHandle,
        out: TensorHandle,
        dim: i32,
    ) -> Result<()> {
        let input_arr = self.get_f32(input)?.clone();
        let out_arr = self.get_f32_mut(out)?;
        
        let shape = input_arr.shape();
        let axis = if dim < 0 { shape.len() as i32 + dim } else { dim } as usize;
        
        // Simple softmax along last dimension
        if axis != shape.len() - 1 {
            return Err(HlxError::ValidationFail {
                message: "Only softmax along last dimension supported".to_string(),
            });
        }
        
        let row_size = shape[axis];
        let num_rows: usize = shape.iter().take(axis).product();
        
        for r in 0..num_rows.max(1) {
            // Find max for numerical stability
            let mut max_val = f32::NEG_INFINITY;
            for i in 0..row_size {
                let idx = r * row_size + i;
                if input_arr.as_slice().unwrap()[idx] > max_val {
                    max_val = input_arr.as_slice().unwrap()[idx];
                }
            }
            
            // Compute exp(x - max) and sum
            let mut sum = 0.0f32;
            for i in 0..row_size {
                let idx = r * row_size + i;
                let exp_val = (input_arr.as_slice().unwrap()[idx] - max_val).exp();
                out_arr.as_slice_mut().unwrap()[idx] = exp_val;
                sum += exp_val;
            }
            
            // Normalize
            for i in 0..row_size {
                let idx = r * row_size + i;
                out_arr.as_slice_mut().unwrap()[idx] /= sum;
            }
        }
        
        Ok(())
    }
    
    fn gelu(&mut self, input: TensorHandle, out: TensorHandle) -> Result<()> {
        let input_arr = self.get_f32(input)?.clone();
        let out_arr = self.get_f32_mut(out)?;
        
        let sqrt_2_over_pi = 0.7978845608028654_f32;
        let coef = 0.044715_f32;
        
        for (x, y) in input_arr.iter().zip(out_arr.iter_mut()) {
            let x3 = x * x * x;
            let inner = sqrt_2_over_pi * (x + coef * x3);
            *y = x * 0.5 * (1.0 + inner.tanh());
        }
        
        Ok(())
    }
    
    fn relu(&mut self, input: TensorHandle, out: TensorHandle) -> Result<()> {
        let input_arr = self.get_f32(input)?.clone();
        let out_arr = self.get_f32_mut(out)?;
        
        for (x, y) in input_arr.iter().zip(out_arr.iter_mut()) {
            *y = x.max(0.0);
        }
        
        Ok(())
    }
    
    fn attention(
        &mut self,
        q: TensorHandle,
        k: TensorHandle,
        v: TensorHandle,
        out: TensorHandle,
        _mask: Option<TensorHandle>,
        scale: f64,
    ) -> Result<()> {
        // Simplified attention: softmax(Q @ K^T * scale) @ V
        // Full implementation would handle batching, masking, etc.
        
        let q_arr = self.get_f32(q)?.clone();
        let k_arr = self.get_f32(k)?.clone();
        let v_arr = self.get_f32(v)?.clone();
        
        let seq_len = q_arr.shape()[0];
        let head_dim = q_arr.shape()[1];
        
        // Q @ K^T
        let mut scores = ArrayD::<f32>::zeros(IxDyn(&[seq_len, seq_len]));
        for i in 0..seq_len {
            for j in 0..seq_len {
                let mut dot = 0.0f32;
                for d in 0..head_dim {
                    dot += q_arr[[i, d]] * k_arr[[j, d]];
                }
                scores[[i, j]] = dot * scale as f32;
            }
        }
        
        // Softmax
        for i in 0..seq_len {
            let mut max_val = f32::NEG_INFINITY;
            for j in 0..seq_len {
                max_val = max_val.max(scores[[i, j]]);
            }
            
            let mut sum = 0.0f32;
            for j in 0..seq_len {
                scores[[i, j]] = (scores[[i, j]] - max_val).exp();
                sum += scores[[i, j]];
            }
            
            for j in 0..seq_len {
                scores[[i, j]] /= sum;
            }
        }
        
        // Attention @ V
        let out_arr = self.get_f32_mut(out)?;
        for i in 0..seq_len {
            for d in 0..head_dim {
                let mut sum = 0.0f32;
                for j in 0..seq_len {
                    sum += scores[[i, j]] * v_arr[[j, d]];
                }
                out_arr[[i, d]] = sum;
            }
        }
        
        Ok(())
    }
    
    fn cross_entropy(
        &mut self,
        logits: TensorHandle,
        targets: TensorHandle,
        loss_out: TensorHandle,
        probs_out: TensorHandle,
    ) -> Result<()> {
        // Simplified cross-entropy for single batch
        let logits_arr = self.get_f32(logits)?.clone();
        let shape = logits_arr.shape();
        let vocab_size = shape[shape.len() - 1];
        
        // Softmax
        self.softmax(logits, probs_out, -1)?;
        let probs_arr = self.get_f32(probs_out)?.clone(); // Clone to avoid borrow conflict
        
        // Clone targets to avoid borrow conflict
        let targets_storage = self.get_tensor(targets)?;
        let targets_vec: Vec<i64> = match targets_storage {
            TensorStorage::I64(t) => t.iter().cloned().collect(),
            TensorStorage::I32(t) => t.iter().map(|&x| x as i64).collect(),
            _ => return Err(HlxError::TypeError {
                expected: "integer targets".to_string(),
                got: "other dtype".to_string(),
            }),
        };
        
        let loss_arr = self.get_f32_mut(loss_out)?;
        
        for (i, &target) in targets_vec.iter().enumerate() {
            let prob = probs_arr.as_slice().unwrap()[i * vocab_size + target as usize];
            loss_arr.as_slice_mut().unwrap()[i] = -prob.ln();
        }
        
        Ok(())
    }

    fn reduce_sum(
        &mut self,
        input: TensorHandle,
        out: TensorHandle,
        _dim: Option<i32>,
    ) -> Result<()> {
        // Calculate sum first to avoid borrow conflict
        let sum: f32 = self.get_f32(input)?.iter().sum();
        
        let out_arr = self.get_f32_mut(out)?;
        out_arr.as_slice_mut().unwrap()[0] = sum;
        
        Ok(())
    }
    
    fn embedding(
        &mut self,
        indices: TensorHandle,
        weight: TensorHandle,
        out: TensorHandle,
    ) -> Result<()> {
        let weight_arr = self.get_f32(weight)?.clone();
        let d_model = weight_arr.shape()[1];
        
        // Clone indices to avoid borrow conflict
        let indices_storage = self.get_tensor(indices)?;
        let indices_vec: Vec<usize> = match indices_storage {
            TensorStorage::I64(idx) => idx.iter().map(|&x| x as usize).collect(),
            TensorStorage::I32(idx) => idx.iter().map(|&x| x as usize).collect(),
            _ => return Err(HlxError::TypeError {
                expected: "integer indices".to_string(),
                got: "other dtype".to_string(),
            }),
        };
        
        let out_arr = self.get_f32_mut(out)?;
        
        for (i, &token_id) in indices_vec.iter().enumerate() {
            for d in 0..d_model {
                out_arr.as_slice_mut().unwrap()[i * d_model + d] = 
                    weight_arr[[token_id, d]];
            }
        }
        
        Ok(())
    }
    
    fn adam_update(
        &mut self,
        param: TensorHandle,
        grad: TensorHandle,
        m: TensorHandle,
        v: TensorHandle,
        lr: f64,
        beta1: f64,
        beta2: f64,
        eps: f64,
        step: u64,
    ) -> Result<()> {
        let grad_arr = self.get_f32(grad)?.clone();
        
        // Clone state to avoid multiple mutable borrows
        let mut param_arr = self.get_f32(param)?.clone();
        let mut m_arr = self.get_f32(m)?.clone();
        let mut v_arr = self.get_f32(v)?.clone();
        
        // Bias correction
        let beta1_t = beta1.powi(step as i32);
        let beta2_t = beta2.powi(step as i32);
        
        for i in 0..param_arr.len() {
            let g = grad_arr.as_slice().unwrap()[i] as f64;
            
            // Update moments
            let m_new = beta1 * m_arr.as_slice().unwrap()[i] as f64 + (1.0 - beta1) * g;
            let v_new = beta2 * v_arr.as_slice().unwrap()[i] as f64 + (1.0 - beta2) * g * g;
            
            m_arr.as_slice_mut().unwrap()[i] = m_new as f32;
            v_arr.as_slice_mut().unwrap()[i] = v_new as f32;
            
            // Bias-corrected estimates
            let m_hat = m_new / (1.0 - beta1_t);
            let v_hat = v_new / (1.0 - beta2_t);
            
            // Update parameter
            let update = lr * m_hat / (v_hat.sqrt() + eps);
            param_arr.as_slice_mut().unwrap()[i] -= update as f32;
        }
        
        // Write back updated tensors
        self.tensors.insert(m, TensorStorage::F32(m_arr));
        self.tensors.insert(v, TensorStorage::F32(v_arr));
        self.tensors.insert(param, TensorStorage::F32(param_arr));
        
        Ok(())
    }
    
    fn sync(&mut self) -> Result<()> {
        // CPU is synchronous, nothing to do
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tensor_alloc() {
        let config = RuntimeConfig::default();
        let mut backend = CpuBackend::new(&config).unwrap();
        
        let handle = backend.alloc_tensor(&[2, 3], DType::F32).unwrap();
        let meta = backend.tensor_meta(handle).unwrap();
        
        assert_eq!(meta.shape, vec![2, 3]);
        assert_eq!(meta.dtype, DType::F32);
        assert_eq!(meta.size_bytes, 24); // 2 * 3 * 4
    }

    #[test]
    fn test_scalar_ops() {
        let config = RuntimeConfig::default();
        let mut backend = CpuBackend::new(&config).unwrap();
        
        let a = Value::Integer(10);
        let b = Value::Integer(3);
        
        assert_eq!(backend.scalar_add(&a, &b).unwrap(), Value::Integer(13));
        assert_eq!(backend.scalar_sub(&a, &b).unwrap(), Value::Integer(7));
        assert_eq!(backend.scalar_mul(&a, &b).unwrap(), Value::Integer(30));
    }

    #[test]
    fn test_matmul() {
        let config = RuntimeConfig::default();
        let mut backend = CpuBackend::new(&config).unwrap();
        
        let a = backend.alloc_tensor(&[2, 3], DType::F32).unwrap();
        let b = backend.alloc_tensor(&[3, 2], DType::F32).unwrap();
        let out = backend.alloc_tensor(&[2, 2], DType::F32).unwrap();
        
        // Write identity-like data
        let a_data: Vec<u8> = [1.0f32, 0.0, 0.0, 0.0, 1.0, 0.0]
            .iter().flat_map(|f| f.to_le_bytes()).collect();
        let b_data: Vec<u8> = [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0]
            .iter().flat_map(|f| f.to_le_bytes()).collect();
        
        backend.write_tensor(a, &a_data).unwrap();
        backend.write_tensor(b, &b_data).unwrap();
        
        backend.matmul(a, b, out).unwrap();
        
        let result = backend.read_tensor(out).unwrap();
        let floats: Vec<f32> = result.chunks_exact(4)
            .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
            .collect();
        
        // First row of A is [1, 0, 0], so result[0,0] = 1*1 + 0*3 + 0*5 = 1
        assert_eq!(floats[0], 1.0);
    }
}
