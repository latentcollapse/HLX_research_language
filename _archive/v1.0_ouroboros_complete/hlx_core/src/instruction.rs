//! HLX Instruction Set
//!
//! The IR for HLX computation. Instructions operate on registers
//! and produce deterministic results.
//!
//! ## Design Principles
//! - All operations are deterministic
//! - Tensor operations map directly to GPU kernels
//! - Control flow is structured (no arbitrary jumps)

use serde::{Deserialize, Serialize};
use crate::value::Value;

/// Register identifier (virtual register, SSA-like)
pub type Register = u32;

/// Shape of a tensor (dimensions)
pub type TensorShape = Vec<usize>;

/// Data type for tensor elements
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DType {
    /// 32-bit float (GPU default)
    F32,
    /// 64-bit float (high precision)
    F64,
    /// 32-bit signed integer
    I32,
    /// 64-bit signed integer
    I64,
    /// Boolean
    Bool,
}

impl Default for DType {
    fn default() -> Self { DType::F32 }
}

/// HLX IR Instructions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Instruction {
    // === Value Operations ===
    
    /// Load a constant value into a register
    Constant {
        out: Register,
        val: Value,
    },

    /// Copy a register to another
    Move {
        out: Register,
        src: Register,
    },

    // === Arithmetic Operations ===
    
    /// Add two values: out = lhs + rhs
    Add {
        out: Register,
        lhs: Register,
        rhs: Register,
    },

    /// Subtract: out = lhs - rhs
    Sub {
        out: Register,
        lhs: Register,
        rhs: Register,
    },

    /// Multiply: out = lhs * rhs
    Mul {
        out: Register,
        lhs: Register,
        rhs: Register,
    },

    /// Divide: out = lhs / rhs
    Div {
        out: Register,
        lhs: Register,
        rhs: Register,
    },

    /// Negate: out = -src
    Neg {
        out: Register,
        src: Register,
    },

    // === Comparison Operations ===
    
    /// Equal: out = lhs == rhs
    Eq {
        out: Register,
        lhs: Register,
        rhs: Register,
    },

    /// Not equal: out = lhs != rhs
    Ne {
        out: Register,
        lhs: Register,
        rhs: Register,
    },

    /// Less than: out = lhs < rhs
    Lt {
        out: Register,
        lhs: Register,
        rhs: Register,
    },

    /// Less or equal: out = lhs <= rhs
    Le {
        out: Register,
        lhs: Register,
        rhs: Register,
    },

    /// Greater than: out = lhs > rhs
    Gt {
        out: Register,
        lhs: Register,
        rhs: Register,
    },

    /// Greater or equal: out = lhs >= rhs
    Ge {
        out: Register,
        lhs: Register,
        rhs: Register,
    },

    // === Logical Operations ===
    
    /// Logical AND: out = lhs && rhs
    And {
        out: Register,
        lhs: Register,
        rhs: Register,
    },

    /// Logical OR: out = lhs || rhs
    Or {
        out: Register,
        lhs: Register,
        rhs: Register,
    },

    /// Logical NOT: out = !src
    Not {
        out: Register,
        src: Register,
    },

    // === Tensor Operations ===
    
    /// Matrix multiplication: out = lhs @ rhs
    /// Maps to gemm.glsl kernel
    MatMul {
        out: Register,
        lhs: Register,
        rhs: Register,
    },

    /// Matrix multiplication with bias: out = lhs @ rhs + bias
    MatMulBias {
        out: Register,
        lhs: Register,
        rhs: Register,
        bias: Register,
    },

    /// Create a tensor with specified shape and dtype
    TensorCreate {
        out: Register,
        shape: TensorShape,
        dtype: DType,
        fill: Option<f64>, // Optional fill value
    },

    /// Reshape tensor (must preserve total elements)
    Reshape {
        out: Register,
        src: Register,
        shape: TensorShape,
    },

    /// Transpose tensor (swap dimensions)
    Transpose {
        out: Register,
        src: Register,
        dim0: usize,
        dim1: usize,
    },

    // === Neural Network Layers ===
    
    /// Layer normalization
    /// Maps to layernorm_forward.glsl
    LayerNorm {
        out: Register,
        input: Register,
        gamma: Register,
        beta: Register,
        eps: f64,
    },

    /// Softmax activation
    /// Maps to softmax_forward.glsl
    Softmax {
        out: Register,
        input: Register,
        dim: i32, // Dimension to normalize over
    },

    /// GELU activation
    /// Maps to gelu_forward.glsl
    Gelu {
        out: Register,
        input: Register,
    },

    /// ReLU activation
    Relu {
        out: Register,
        input: Register,
    },

    // === Attention Operations ===
    
    /// Scaled dot-product attention
    /// Combines attention_scores + softmax + attention_output
    Attention {
        out: Register,
        query: Register,
        key: Register,
        value: Register,
        mask: Option<Register>,
        scale: f64,
    },

    // === Loss Functions ===
    
    /// Cross-entropy loss
    /// Maps to cross_entropy_forward.glsl
    CrossEntropy {
        loss_out: Register,
        probs_out: Register, // Softmax probabilities (for backward)
        logits: Register,
        targets: Register,
        ignore_index: Option<u32>,
    },

    // === Reduction Operations ===
    
    /// Sum reduction
    /// Maps to reduce_sum.glsl + reduce_final.glsl
    ReduceSum {
        out: Register,
        input: Register,
        dim: Option<i32>, // None = reduce all
        keepdim: bool,
    },

    /// Mean reduction
    ReduceMean {
        out: Register,
        input: Register,
        dim: Option<i32>,
        keepdim: bool,
    },

    /// Max reduction
    ReduceMax {
        out: Register,
        input: Register,
        dim: Option<i32>,
        keepdim: bool,
    },

    // === Embedding Operations ===
    
    /// Embedding lookup
    /// Maps to embedding_forward.glsl
    Embedding {
        out: Register,
        indices: Register,
        weight: Register,
    },

    // === Optimizer Operations ===
    
    /// Adam optimizer step
    /// Maps to adam_update.glsl
    AdamUpdate {
        param: Register,
        grad: Register,
        m: Register,     // First moment
        v: Register,     // Second moment
        lr: f64,
        beta1: f64,
        beta2: f64,
        eps: f64,
        step: u64,       // For bias correction
    },

    // === Control Flow ===
    
    /// Conditional branch
    /// If cond is true, execute then_block (capsule index), else execute else_block (capsule index)
    If {
        cond: Register,
        then_block: u32,
        else_block: u32,
    },

    /// Unconditional jump to instruction index
    Jump {
        target: u32,
    },

    /// Bounded loop
    /// While cond is true, execute body (capsule index) up to max_iter times
    Loop {
        cond: Register,
        body: u32,
        exit: u32,
        max_iter: u32,
    },

    /// Define a function (metadata instruction, usually at top level)
    FuncDef {
        name: String,
        params: Vec<Register>,
        body: u32, // Capsule index
    },

    /// Call a function by name
    Call {
        out: Register,
        func: String,
        args: Vec<Register>,
    },

    /// Return a value
    Return {
        val: Register,
    },

    /// Break out of current loop
    Break,

    /// Continue to next iteration of current loop
    Continue,

    // === Memory Operations ===
    
    /// Get length of array or string
    ArrayLen {
        out: Register,
        array: Register,
    },

    /// Load from array/object by index/key
    Index {
        out: Register,
        container: Register,
        index: Register,
    },

    /// Create an array from a list of registers
    ArrayCreate {
        out: Register,
        elements: Vec<Register>,
    },

    /// Allocate an empty array of dynamic size
    ArrayAlloc {
        out: Register,
        size: Register,
    },

    /// Create an object from keys and value registers
    ObjectCreate {
        out: Register,
        keys: Vec<String>,
        values: Vec<Register>,
    },

    /// Store to array/object by index/key
    Store {
        container: Register,
        index: Register,
        value: Register,
    },

    // === Debug/Introspection ===
    
    /// Print value (deterministic output)
    Print {
        val: Register,
    },

    /// Print string (null-terminated char*)
    PrintStr {
        val: Register,
    },

    /// Get type of value
    TypeOf {
        out: Register,
        val: Register,
    },

    // === Latent Space Operations ===
    
    /// Collapse value to handle (CAS store)
    Collapse {
        handle_out: Register,
        val: Register,
    },

    /// Resolve handle to value (CAS retrieve)
    Resolve {
        val_out: Register,
        handle: Register,
    },

    /// Take snapshot of current state
    Snapshot {
        handle_out: Register,
    },

    // === No-op (for padding/alignment) ===
    Nop,
}

impl Instruction {
    /// Get the output register (if any)
    pub fn output_register(&self) -> Option<Register> {
        match self {
            Instruction::Constant { out, .. } => Some(*out),
            Instruction::Move { out, .. } => Some(*out),
            Instruction::Add { out, .. } => Some(*out),
            Instruction::Sub { out, .. } => Some(*out),
            Instruction::Mul { out, .. } => Some(*out),
            Instruction::Div { out, .. } => Some(*out),
            Instruction::Neg { out, .. } => Some(*out),
            Instruction::Eq { out, .. } => Some(*out),
            Instruction::Ne { out, .. } => Some(*out),
            Instruction::Lt { out, .. } => Some(*out),
            Instruction::Le { out, .. } => Some(*out),
            Instruction::Gt { out, .. } => Some(*out),
            Instruction::Ge { out, .. } => Some(*out),
            Instruction::And { out, .. } => Some(*out),
            Instruction::Or { out, .. } => Some(*out),
            Instruction::Not { out, .. } => Some(*out),
            Instruction::MatMul { out, .. } => Some(*out),
            Instruction::MatMulBias { out, .. } => Some(*out),
            Instruction::TensorCreate { out, .. } => Some(*out),
            Instruction::Reshape { out, .. } => Some(*out),
            Instruction::Transpose { out, .. } => Some(*out),
            Instruction::LayerNorm { out, .. } => Some(*out),
            Instruction::Softmax { out, .. } => Some(*out),
            Instruction::Gelu { out, .. } => Some(*out),
            Instruction::Relu { out, .. } => Some(*out),
            Instruction::Attention { out, .. } => Some(*out),
            Instruction::CrossEntropy { loss_out, .. } => Some(*loss_out),
            Instruction::ReduceSum { out, .. } => Some(*out),
            Instruction::ReduceMean { out, .. } => Some(*out),
            Instruction::ReduceMax { out, .. } => Some(*out),
            Instruction::Embedding { out, .. } => Some(*out),
            Instruction::Call { out, .. } => Some(*out),
            Instruction::ArrayLen { out, .. } => Some(*out),
            Instruction::Index { out, .. } => Some(*out),
            Instruction::ArrayCreate { out, .. } => Some(*out),
            Instruction::ArrayAlloc { out, .. } => Some(*out),
            Instruction::ObjectCreate { out, .. } => Some(*out),
            Instruction::TypeOf { out, .. } => Some(*out),
            Instruction::Collapse { handle_out, .. } => Some(*handle_out),
            Instruction::Resolve { val_out, .. } => Some(*val_out),
            Instruction::Snapshot { handle_out } => Some(*handle_out),
            // These don't produce output registers
            Instruction::If { .. } => None,
            Instruction::Jump { .. } => None,
            Instruction::Loop { .. } => None,
            Instruction::FuncDef { .. } => None,
            Instruction::AdamUpdate { .. } => None,
            Instruction::Return { .. } => None,
            Instruction::Break => None,
            Instruction::Continue => None,
            Instruction::Store { .. } => None,
            Instruction::Print { .. } => None,
            Instruction::PrintStr { .. } => None,
            Instruction::Nop => None,
        }
    }

    /// Get all input registers used by this instruction
    pub fn input_registers(&self) -> Vec<Register> {
        match self {
            Instruction::Constant { .. } => vec![],
            Instruction::Move { src, .. } => vec![*src],
            Instruction::Add { lhs, rhs, .. } => vec![*lhs, *rhs],
            Instruction::Sub { lhs, rhs, .. } => vec![*lhs, *rhs],
            Instruction::Mul { lhs, rhs, .. } => vec![*lhs, *rhs],
            Instruction::Div { lhs, rhs, .. } => vec![*lhs, *rhs],
            Instruction::Neg { src, .. } => vec![*src],
            Instruction::Eq { lhs, rhs, .. } => vec![*lhs, *rhs],
            Instruction::Ne { lhs, rhs, .. } => vec![*lhs, *rhs],
            Instruction::Lt { lhs, rhs, .. } => vec![*lhs, *rhs],
            Instruction::Le { lhs, rhs, .. } => vec![*lhs, *rhs],
            Instruction::Gt { lhs, rhs, .. } => vec![*lhs, *rhs],
            Instruction::Ge { lhs, rhs, .. } => vec![*lhs, *rhs],
            Instruction::And { lhs, rhs, .. } => vec![*lhs, *rhs],
            Instruction::Or { lhs, rhs, .. } => vec![*lhs, *rhs],
            Instruction::Not { src, .. } => vec![*src],
            Instruction::MatMul { lhs, rhs, .. } => vec![*lhs, *rhs],
            Instruction::MatMulBias { lhs, rhs, bias, .. } => vec![*lhs, *rhs, *bias],
            Instruction::TensorCreate { .. } => vec![],
            Instruction::Reshape { src, .. } => vec![*src],
            Instruction::Transpose { src, .. } => vec![*src],
            Instruction::LayerNorm { input, gamma, beta, .. } => vec![*input, *gamma, *beta],
            Instruction::Softmax { input, .. } => vec![*input],
            Instruction::Gelu { input, .. } => vec![*input],
            Instruction::Relu { input, .. } => vec![*input],
            Instruction::Attention { query, key, value, mask, .. } => {
                let mut regs = vec![*query, *key, *value];
                if let Some(m) = mask { regs.push(*m); }
                regs
            }
            Instruction::CrossEntropy { logits, targets, .. } => vec![*logits, *targets],
            Instruction::ReduceSum { input, .. } => vec![*input],
            Instruction::ReduceMean { input, .. } => vec![*input],
            Instruction::ReduceMax { input, .. } => vec![*input],
            Instruction::Embedding { indices, weight, .. } => vec![*indices, *weight],
            Instruction::AdamUpdate { param, grad, m, v, .. } => vec![*param, *grad, *m, *v],
            Instruction::If { cond, .. } => vec![*cond],
            Instruction::Jump { .. } => vec![],
            Instruction::Loop { cond, .. } => vec![*cond],
            Instruction::FuncDef { .. } => vec![],
            Instruction::Call { args, .. } => args.clone(),
            Instruction::Return { val } => vec![*val],
            Instruction::Break => vec![],
            Instruction::Continue => vec![],
            Instruction::ArrayLen { array, .. } => vec![*array],
            Instruction::Index { container, index, .. } => vec![*container, *index],
            Instruction::ArrayCreate { elements, .. } => elements.clone(),
            Instruction::ArrayAlloc { size, .. } => vec![*size],
            Instruction::ObjectCreate { values, .. } => values.clone(),
            Instruction::Store { container, index, value } => vec![*container, *index, *value],
            Instruction::Print { val } => vec![*val],
            Instruction::PrintStr { val } => vec![*val],
            Instruction::TypeOf { val, .. } => vec![*val],
            Instruction::Collapse { val, .. } => vec![*val],
            Instruction::Resolve { handle, .. } => vec![*handle],
            Instruction::Snapshot { .. } => vec![],
            Instruction::Nop => vec![],
        }
    }

    /// Check if this instruction has side effects
    pub fn has_side_effects(&self) -> bool {
        matches!(self,
            Instruction::Store { .. } |
            Instruction::Print { .. } |
            Instruction::AdamUpdate { .. } |
            Instruction::Collapse { .. } |
            Instruction::Snapshot { .. } |
            Instruction::Call { .. } |
            Instruction::Return { .. } |
            Instruction::Break |
            Instruction::Continue |
            Instruction::If { .. } |
            Instruction::Loop { .. } |
            Instruction::Jump { .. }
        )
    }

    /// Check if this is a tensor operation (maps to GPU kernel)
    pub fn is_tensor_op(&self) -> bool {
        matches!(self,
            Instruction::MatMul { .. } |
            Instruction::MatMulBias { .. } |
            Instruction::TensorCreate { .. } |
            Instruction::Reshape { .. } |
            Instruction::Transpose { .. } |
            Instruction::LayerNorm { .. } |
            Instruction::Softmax { .. } |
            Instruction::Gelu { .. } |
            Instruction::Relu { .. } |
            Instruction::Attention { .. } |
            Instruction::CrossEntropy { .. } |
            Instruction::ReduceSum { .. } |
            Instruction::ReduceMean { .. } |
            Instruction::ReduceMax { .. } |
            Instruction::Embedding { .. } |
            Instruction::AdamUpdate { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instruction_output_register() {
        let inst = Instruction::Add { out: 5, lhs: 1, rhs: 2 };
        assert_eq!(inst.output_register(), Some(5));

        let inst = Instruction::Nop;
        assert_eq!(inst.output_register(), None);
    }

    #[test]
    fn test_instruction_input_registers() {
        let inst = Instruction::MatMul { out: 3, lhs: 1, rhs: 2 };
        assert_eq!(inst.input_registers(), vec![1, 2]);
    }
}
