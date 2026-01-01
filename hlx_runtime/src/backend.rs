//! Backend Trait
//!
//! Defines the interface between the executor and compute backends.
//! Implementations provide CPU (ndarray) or GPU (Vulkan/SPIR-V) execution.

use hlx_core::{Value, Result, Instruction};
use crate::config::RuntimeConfig;

/// Handle to a tensor stored in the backend
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TensorHandle(pub u64);

/// Tensor metadata
#[derive(Debug, Clone)]
pub struct TensorMeta {
    pub shape: Vec<usize>,
    pub dtype: DType,
    pub size_bytes: usize,
}

/// Data type for tensors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DType {
    F32,
    F64,
    I32,
    I64,
    Bool,
}

impl DType {
    pub fn size_bytes(&self) -> usize {
        match self {
            DType::F32 => 4,
            DType::F64 => 8,
            DType::I32 => 4,
            DType::I64 => 8,
            DType::Bool => 1,
        }
    }
}

/// Backend trait for compute operations
pub trait Backend: Send + Sync {
    /// Get backend name
    fn name(&self) -> &'static str;
    
    /// Check if backend is available and initialized
    fn is_available(&self) -> bool;
    
    // === Tensor Management ===
    
    /// Allocate a new tensor
    fn alloc_tensor(&mut self, shape: &[usize], dtype: DType) -> Result<TensorHandle>;
    
    /// Free a tensor
    fn free_tensor(&mut self, handle: TensorHandle) -> Result<()>;
    
    /// Get tensor metadata
    fn tensor_meta(&self, handle: TensorHandle) -> Result<TensorMeta>;
    
    /// Write data to tensor
    fn write_tensor(&mut self, handle: TensorHandle, data: &[u8]) -> Result<()>;
    
    /// Read data from tensor
    fn read_tensor(&self, handle: TensorHandle) -> Result<Vec<u8>>;
    
    // === Scalar Operations ===
    
    /// Add two scalars
    fn scalar_add(&mut self, a: &Value, b: &Value) -> Result<Value>;
    
    /// Subtract two scalars
    fn scalar_sub(&mut self, a: &Value, b: &Value) -> Result<Value>;
    
    /// Multiply two scalars
    fn scalar_mul(&mut self, a: &Value, b: &Value) -> Result<Value>;
    
    /// Divide two scalars
    fn scalar_div(&mut self, a: &Value, b: &Value) -> Result<Value>;
    
    // === Comparison Operations ===
    
    fn scalar_eq(&mut self, a: &Value, b: &Value) -> Result<Value>;
    fn scalar_ne(&mut self, a: &Value, b: &Value) -> Result<Value>;
    fn scalar_lt(&mut self, a: &Value, b: &Value) -> Result<Value>;
    fn scalar_le(&mut self, a: &Value, b: &Value) -> Result<Value>;
    fn scalar_gt(&mut self, a: &Value, b: &Value) -> Result<Value>;
    fn scalar_ge(&mut self, a: &Value, b: &Value) -> Result<Value>;
    
    // === Tensor Operations ===
    
    /// Matrix multiplication: C = A @ B
    fn matmul(
        &mut self,
        a: TensorHandle,
        b: TensorHandle,
        out: TensorHandle,
    ) -> Result<()>;
    
    /// Matrix multiplication with bias: C = A @ B + bias
    fn matmul_bias(
        &mut self,
        a: TensorHandle,
        b: TensorHandle,
        bias: TensorHandle,
        out: TensorHandle,
    ) -> Result<()>;
    
    /// Layer normalization
    fn layer_norm(
        &mut self,
        input: TensorHandle,
        gamma: TensorHandle,
        beta: TensorHandle,
        out: TensorHandle,
        eps: f64,
    ) -> Result<()>;
    
    /// Softmax activation
    fn softmax(
        &mut self,
        input: TensorHandle,
        out: TensorHandle,
        dim: i32,
    ) -> Result<()>;
    
    /// GELU activation
    fn gelu(
        &mut self,
        input: TensorHandle,
        out: TensorHandle,
    ) -> Result<()>;
    
    /// ReLU activation
    fn relu(
        &mut self,
        input: TensorHandle,
        out: TensorHandle,
    ) -> Result<()>;
    
    /// Attention: softmax(Q @ K^T / sqrt(d)) @ V
    fn attention(
        &mut self,
        q: TensorHandle,
        k: TensorHandle,
        v: TensorHandle,
        out: TensorHandle,
        mask: Option<TensorHandle>,
        scale: f64,
    ) -> Result<()>;
    
    /// Cross-entropy loss
    fn cross_entropy(
        &mut self,
        logits: TensorHandle,
        targets: TensorHandle,
        loss_out: TensorHandle,
        probs_out: TensorHandle,
    ) -> Result<()>;
    
    /// Sum reduction
    fn reduce_sum(
        &mut self,
        input: TensorHandle,
        out: TensorHandle,
        dim: Option<i32>,
    ) -> Result<()>;
    
    /// Embedding lookup
    fn embedding(
        &mut self,
        indices: TensorHandle,
        weight: TensorHandle,
        out: TensorHandle,
    ) -> Result<()>;
    
    /// Adam optimizer update
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
    ) -> Result<()>;
    
    // === Synchronization ===
    
    /// Synchronize all pending operations
    fn sync(&mut self) -> Result<()>;
}

/// Create a backend based on configuration
pub fn create_backend(config: &RuntimeConfig) -> Result<Box<dyn Backend>> {
    use crate::config::BackendType;
    
    match config.backend {
        BackendType::Cpu => {
            #[cfg(feature = "cpu")]
            {
                Ok(Box::new(crate::backends::cpu::CpuBackend::new(config)?))
            }
            #[cfg(not(feature = "cpu"))]
            {
                Err(hlx_core::HlxError::ValidationFail {
                    message: "CPU backend not compiled".to_string(),
                })
            }
        }
        BackendType::Vulkan => {
            #[cfg(feature = "vulkan")]
            {
                Ok(Box::new(crate::backends::vulkan::VulkanBackend::new(config)?))
            }
            #[cfg(not(feature = "vulkan"))]
            {
                Err(hlx_core::HlxError::ValidationFail {
                    message: "Vulkan backend not compiled".to_string(),
                })
            }
        }
        BackendType::Auto => {
            // Try Vulkan first, fall back to CPU
            #[cfg(feature = "vulkan")]
            {
                if let Ok(backend) = crate::backends::vulkan::VulkanBackend::new(config) {
                    return Ok(Box::new(backend));
                }
            }
            
            #[cfg(feature = "cpu")]
            {
                Ok(Box::new(crate::backends::cpu::CpuBackend::new(config)?))
            }
            
            #[cfg(not(any(feature = "cpu", feature = "vulkan")))]
            {
                Err(hlx_core::HlxError::ValidationFail {
                    message: "No backend available".to_string(),
                })
            }
        }
    }
}
