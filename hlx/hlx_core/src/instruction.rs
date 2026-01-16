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

/// Data type for tensor elements and array elements
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    /// Array of elements with inner type (for nested arrays)
    Array(Box<DType>),
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

    /// Modulo: out = lhs % rhs
    Mod {
        out: Register,
        lhs: Register,
        rhs: Register,
    },

    /// Negate: out = -src
    Neg {
        out: Register,
        src: Register,
    },

    // === Math Functions ===

    /// Square root: out = sqrt(src)
    Sqrt {
        out: Register,
        src: Register,
    },

    /// Power: out = base^exp
    Pow {
        out: Register,
        base: Register,
        exp: Register,
    },

    /// Sine: out = sin(src)
    Sin {
        out: Register,
        src: Register,
    },

    /// Cosine: out = cos(src)
    Cos {
        out: Register,
        src: Register,
    },

    /// Tangent: out = tan(src)
    Tan {
        out: Register,
        src: Register,
    },

    /// Natural logarithm: out = ln(src)
    Log {
        out: Register,
        src: Register,
    },

    /// Exponential: out = e^src
    Exp {
        out: Register,
        src: Register,
    },

    /// Floor: out = floor(src)
    Floor {
        out: Register,
        src: Register,
    },

    /// Ceiling: out = ceil(src)
    Ceil {
        out: Register,
        src: Register,
    },

    /// Round: out = round(src)
    Round {
        out: Register,
        src: Register,
    },

    /// Absolute value: out = abs(src)
    Abs {
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

    // === Bitwise Operations ===
    
    /// Bitwise AND: out = lhs & rhs
    BitAnd {
        out: Register,
        lhs: Register,
        rhs: Register,
    },

    /// Bitwise OR: out = lhs | rhs
    BitOr {
        out: Register,
        lhs: Register,
        rhs: Register,
    },

    /// Bitwise XOR: out = lhs ^ rhs
    BitXor {
        out: Register,
        lhs: Register,
        rhs: Register,
    },

    /// Shift left: out = lhs << rhs
    Shl {
        out: Register,
        lhs: Register,
        rhs: Register,
    },

    /// Shift right: out = lhs >> rhs
    Shr {
        out: Register,
        lhs: Register,
        rhs: Register,
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

    // === Image Processing Operations ===

    /// Gaussian blur filter
    /// Maps to gaussian_blur.comp shader
    GaussianBlur {
        out: Register,
        input: Register,
        sigma: Register,  // Blur strength
    },

    /// Sobel edge detection
    /// Maps to sobel.comp shader
    SobelEdges {
        out: Register,
        input: Register,
        threshold: Register,  // Edge threshold (0.0 to 1.0)
    },

    /// Convert RGB to grayscale
    Grayscale {
        out: Register,
        input: Register,
    },

    /// Binary threshold
    /// Output: 0 if pixel < threshold, 1 if pixel >= threshold
    Threshold {
        out: Register,
        input: Register,
        value: Register,
    },

    /// Adjust brightness
    /// Output: input * factor
    Brightness {
        out: Register,
        input: Register,
        factor: Register,
    },

    /// Adjust contrast
    /// Output: (input - 0.5) * factor + 0.5
    Contrast {
        out: Register,
        input: Register,
        factor: Register,
    },

    /// Invert colors
    /// Output: 1.0 - input
    InvertColors {
        out: Register,
        input: Register,
    },

    /// Sharpen filter
    Sharpen {
        out: Register,
        input: Register,
    },

    // === Parsing Operations ===

    /// Parse string to integer
    ParseInt {
        out: Register,
        input: Register,  // String to parse
    },

    /// Parse string to float
    ParseFloat {
        out: Register,
        input: Register,  // String to parse
    },

    /// Serialize value to JSON string
    JsonSerialize {
        out: Register,
        input: Register,  // Value to serialize
    },

    /// Parse CSV string to array of arrays
    CsvParse {
        out: Register,
        input: Register,      // CSV string
        delimiter: Register,  // Delimiter (e.g., ",")
    },

    /// Format string with arguments
    /// Supports "{}" placeholders
    FormatString {
        out: Register,
        format: Register,  // Format string
        args: Vec<Register>,  // Arguments to interpolate
    },

    /// Match regex pattern against string
    /// Returns array of matches or empty array
    RegexMatch {
        out: Register,
        input: Register,    // String to match
        pattern: Register,  // Regex pattern
    },

    /// Replace regex matches in string
    RegexReplace {
        out: Register,
        input: Register,        // String to process
        pattern: Register,      // Regex pattern
        replacement: Register,  // Replacement string
    },

    // === File I/O Operations ===

    /// Read line from stdin
    ReadLine {
        out: Register,
    },

    /// Append string to file
    AppendFile {
        out: Register,      // Success boolean
        path: Register,     // File path
        content: Register,  // Content to append
    },

    /// Check if file exists
    FileExists {
        out: Register,
        path: Register,
    },

    /// Delete file
    DeleteFile {
        out: Register,  // Success boolean
        path: Register,
    },

    /// List files in directory
    /// Returns array of file names
    ListFiles {
        out: Register,
        path: Register,  // Directory path
    },

    /// Create directory
    CreateDir {
        out: Register,  // Success boolean
        path: Register,
    },

    /// Delete directory (must be empty)
    DeleteDir {
        out: Register,  // Success boolean
        path: Register,
    },

    /// Read and parse JSON file
    ReadJson {
        out: Register,
        path: Register,
    },

    /// Write value as JSON to file
    WriteJson {
        out: Register,    // Success boolean
        path: Register,   // File path
        value: Register,  // Value to serialize
    },

    /// Read CSV file
    /// Returns array of arrays
    ReadCsv {
        out: Register,
        path: Register,
        delimiter: Register,  // Optional delimiter (defaults to ",")
    },

    /// Write CSV file
    WriteCsv {
        out: Register,    // Success boolean
        path: Register,   // File path
        data: Register,   // Array of arrays
        delimiter: Register,  // Optional delimiter (defaults to ",")
    },

    // === Image I/O Operations ===

    /// Load image from file as tensor
    /// Returns tensor with shape [height, width, channels]
    /// Channels: 3 for RGB, 4 for RGBA
    LoadImage {
        out: Register,   // Tensor handle
        path: Register,  // File path (PNG, JPEG, etc.)
    },

    /// Save tensor as image file
    /// Tensor shape should be [height, width, channels]
    SaveImage {
        out: Register,    // Success boolean
        tensor: Register, // Tensor handle
        path: Register,   // File path
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

    /// Define a module (metadata instruction, usually at top level)
    ModuleDef {
        name: String,
        capabilities: Vec<String>,
        constants: Vec<(String, DType, Register)>,
        structs: Vec<(String, Vec<(String, DType)>)>,
        blocks: Vec<(String, Vec<Register>, u32)>,
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

    /// Barrier synchronization for HLX-Scale parallel execution
    /// All agents must reach this point before any can continue
    /// Runtime performs hash verification of agent states at this point
    Barrier {
        /// Optional barrier name for debugging and profiling
        name: Option<String>,
    },

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
        /// Element type (None means untyped/dynamic for backwards compat)
        element_type: Option<DType>,
    },

    /// Allocate an empty array of dynamic size
    ArrayAlloc {
        out: Register,
        size: Register,
        /// Element type (None means untyped/dynamic for backwards compat)
        element_type: Option<DType>,
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

    // === String Operations ===

    /// String concatenation: out = lhs + rhs
    StrConcat {
        out: Register,
        lhs: Register,
        rhs: Register,
    },

    /// String length
    StrLen {
        out: Register,
        src: Register,
    },

    /// Substring extraction: out = src[start..start+len]
    Substring {
        out: Register,
        src: Register,
        start: Register,
        length: Register,
    },

    /// Find index of substring: out = haystack.find(needle) or -1
    IndexOf {
        out: Register,
        haystack: Register,
        needle: Register,
    },

    /// Replace all occurrences: out = src.replace(from, to)
    StrReplace {
        out: Register,
        src: Register,
        from: Register,
        to: Register,
    },

    /// Split string by delimiter: out = src.split(delimiter)
    StrSplit {
        out: Register,
        src: Register,
        delimiter: Register,
    },

    /// Join array of strings: out = arr.join(separator)
    StrJoin {
        out: Register,
        array: Register,
        separator: Register,
    },

    /// To uppercase
    ToUpper {
        out: Register,
        src: Register,
    },

    /// To lowercase
    ToLower {
        out: Register,
        src: Register,
    },

    /// Trim whitespace
    StrTrim {
        out: Register,
        src: Register,
    },

    /// Check if starts with prefix
    StartsWith {
        out: Register,
        src: Register,
        prefix: Register,
    },

    /// Check if ends with suffix
    EndsWith {
        out: Register,
        src: Register,
        suffix: Register,
    },

    /// Repeat string n times
    StrRepeat {
        out: Register,
        src: Register,
        count: Register,
    },

    /// Reverse string
    StrReverse {
        out: Register,
        src: Register,
    },

    /// Get character at index
    CharAt {
        out: Register,
        src: Register,
        index: Register,
    },

    // === Array Operations ===

    /// Push element to end of array (returns new array)
    ArrayPush {
        out: Register,
        array: Register,
        element: Register,
    },

    /// Pop element from end of array (returns [new_array, element])
    ArrayPop {
        array_out: Register,
        element_out: Register,
        array: Register,
    },

    /// Remove first element (returns [new_array, element])
    ArrayShift {
        array_out: Register,
        element_out: Register,
        array: Register,
    },

    /// Prepend element to start of array (returns new array)
    ArrayUnshift {
        out: Register,
        array: Register,
        element: Register,
    },

    /// Slice array: out = arr[start..start+len]
    ArraySlice {
        out: Register,
        array: Register,
        start: Register,
        length: Register,
    },

    /// Concatenate arrays: out = lhs + rhs
    ArrayConcat {
        out: Register,
        lhs: Register,
        rhs: Register,
    },

    /// Reverse array
    ArrayReverse {
        out: Register,
        array: Register,
    },

    /// Sort array (in-place, persistent)
    ArraySort {
        out: Register,
        array: Register,
    },

    /// Find element in array (returns index or -1)
    ArrayFind {
        out: Register,
        array: Register,
        element: Register,
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

    /// Inline Assembly (for kernel/bare metal)
    Asm {
        out: Option<Register>,
        template: String,
        constraints: String,
        side_effects: bool,
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
            Instruction::Mod { out, .. } => Some(*out),
            Instruction::Neg { out, .. } => Some(*out),
            Instruction::Sqrt { out, .. } => Some(*out),
            Instruction::Pow { out, .. } => Some(*out),
            Instruction::Sin { out, .. } => Some(*out),
            Instruction::Cos { out, .. } => Some(*out),
            Instruction::Tan { out, .. } => Some(*out),
            Instruction::Log { out, .. } => Some(*out),
            Instruction::Exp { out, .. } => Some(*out),
            Instruction::Floor { out, .. } => Some(*out),
            Instruction::Ceil { out, .. } => Some(*out),
            Instruction::Round { out, .. } => Some(*out),
            Instruction::Abs { out, .. } => Some(*out),
            Instruction::StrConcat { out, .. } => Some(*out),
            Instruction::StrLen { out, .. } => Some(*out),
            Instruction::Substring { out, .. } => Some(*out),
            Instruction::IndexOf { out, .. } => Some(*out),
            Instruction::StrReplace { out, .. } => Some(*out),
            Instruction::StrSplit { out, .. } => Some(*out),
            Instruction::StrJoin { out, .. } => Some(*out),
            Instruction::ToUpper { out, .. } => Some(*out),
            Instruction::ToLower { out, .. } => Some(*out),
            Instruction::StrTrim { out, .. } => Some(*out),
            Instruction::StartsWith { out, .. } => Some(*out),
            Instruction::EndsWith { out, .. } => Some(*out),
            Instruction::StrRepeat { out, .. } => Some(*out),
            Instruction::StrReverse { out, .. } => Some(*out),
            Instruction::CharAt { out, .. } => Some(*out),
            Instruction::ArrayPush { out, .. } => Some(*out),
            Instruction::ArrayPop { array_out, .. } => Some(*array_out),
            Instruction::ArrayShift { array_out, .. } => Some(*array_out),
            Instruction::ArrayUnshift { out, .. } => Some(*out),
            Instruction::ArraySlice { out, .. } => Some(*out),
            Instruction::ArrayConcat { out, .. } => Some(*out),
            Instruction::ArrayReverse { out, .. } => Some(*out),
            Instruction::ArraySort { out, .. } => Some(*out),
            Instruction::ArrayFind { out, .. } => Some(*out),
            Instruction::Eq { out, .. } => Some(*out),
            Instruction::Ne { out, .. } => Some(*out),
            Instruction::Lt { out, .. } => Some(*out),
            Instruction::Le { out, .. } => Some(*out),
            Instruction::Gt { out, .. } => Some(*out),
            Instruction::Ge { out, .. } => Some(*out),
            Instruction::And { out, .. } => Some(*out),
            Instruction::Or { out, .. } => Some(*out),
            Instruction::Not { out, .. } => Some(*out),
            Instruction::BitAnd { out, .. } => Some(*out),
            Instruction::BitOr { out, .. } => Some(*out),
            Instruction::BitXor { out, .. } => Some(*out),
            Instruction::Shl { out, .. } => Some(*out),
            Instruction::Shr { out, .. } => Some(*out),
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
            Instruction::Asm { out, .. } => *out,
            Instruction::ModuleDef { .. } => None,
            // These don't produce output registers
            Instruction::If { .. } => None,
            Instruction::Jump { .. } => None,
            Instruction::Loop { .. } => None,
            Instruction::FuncDef { .. } => None,
            Instruction::AdamUpdate { .. } => None,
            Instruction::GaussianBlur { out, .. } => Some(*out),
            Instruction::SobelEdges { out, .. } => Some(*out),
            Instruction::Grayscale { out, .. } => Some(*out),
            Instruction::Threshold { out, .. } => Some(*out),
            Instruction::Brightness { out, .. } => Some(*out),
            Instruction::Contrast { out, .. } => Some(*out),
            Instruction::InvertColors { out, .. } => Some(*out),
            Instruction::Sharpen { out, .. } => Some(*out),
            Instruction::ParseInt { out, .. } => Some(*out),
            Instruction::ParseFloat { out, .. } => Some(*out),
            Instruction::JsonSerialize { out, .. } => Some(*out),
            Instruction::CsvParse { out, .. } => Some(*out),
            Instruction::FormatString { out, .. } => Some(*out),
            Instruction::RegexMatch { out, .. } => Some(*out),
            Instruction::RegexReplace { out, .. } => Some(*out),
            Instruction::ReadLine { out } => Some(*out),
            Instruction::AppendFile { out, .. } => Some(*out),
            Instruction::FileExists { out, .. } => Some(*out),
            Instruction::DeleteFile { out, .. } => Some(*out),
            Instruction::ListFiles { out, .. } => Some(*out),
            Instruction::CreateDir { out, .. } => Some(*out),
            Instruction::DeleteDir { out, .. } => Some(*out),
            Instruction::ReadJson { out, .. } => Some(*out),
            Instruction::WriteJson { out, .. } => Some(*out),
            Instruction::ReadCsv { out, .. } => Some(*out),
            Instruction::WriteCsv { out, .. } => Some(*out),
            Instruction::LoadImage { out, .. } => Some(*out),
            Instruction::SaveImage { out, .. } => Some(*out),
            Instruction::Return { .. } => None,
            Instruction::Break => None,
            Instruction::Continue => None,
            Instruction::Barrier { .. } => None,
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
            Instruction::Mod { lhs, rhs, .. } => vec![*lhs, *rhs],
            Instruction::Neg { src, .. } => vec![*src],
            Instruction::Sqrt { src, .. } => vec![*src],
            Instruction::Pow { base, exp, .. } => vec![*base, *exp],
            Instruction::Sin { src, .. } => vec![*src],
            Instruction::Cos { src, .. } => vec![*src],
            Instruction::Tan { src, .. } => vec![*src],
            Instruction::Log { src, .. } => vec![*src],
            Instruction::Exp { src, .. } => vec![*src],
            Instruction::Floor { src, .. } => vec![*src],
            Instruction::Ceil { src, .. } => vec![*src],
            Instruction::Round { src, .. } => vec![*src],
            Instruction::Abs { src, .. } => vec![*src],
            Instruction::StrConcat { lhs, rhs, .. } => vec![*lhs, *rhs],
            Instruction::StrLen { src, .. } => vec![*src],
            Instruction::Substring { src, start, length, .. } => vec![*src, *start, *length],
            Instruction::IndexOf { haystack, needle, .. } => vec![*haystack, *needle],
            Instruction::StrReplace { src, from, to, .. } => vec![*src, *from, *to],
            Instruction::StrSplit { src, delimiter, .. } => vec![*src, *delimiter],
            Instruction::StrJoin { array, separator, .. } => vec![*array, *separator],
            Instruction::ToUpper { src, .. } => vec![*src],
            Instruction::ToLower { src, .. } => vec![*src],
            Instruction::StrTrim { src, .. } => vec![*src],
            Instruction::StartsWith { src, prefix, .. } => vec![*src, *prefix],
            Instruction::EndsWith { src, suffix, .. } => vec![*src, *suffix],
            Instruction::StrRepeat { src, count, .. } => vec![*src, *count],
            Instruction::StrReverse { src, .. } => vec![*src],
            Instruction::CharAt { src, index, .. } => vec![*src, *index],
            Instruction::ArrayPush { array, element, .. } => vec![*array, *element],
            Instruction::ArrayPop { array, .. } => vec![*array],
            Instruction::ArrayShift { array, .. } => vec![*array],
            Instruction::ArrayUnshift { array, element, .. } => vec![*array, *element],
            Instruction::ArraySlice { array, start, length, .. } => vec![*array, *start, *length],
            Instruction::ArrayConcat { lhs, rhs, .. } => vec![*lhs, *rhs],
            Instruction::ArrayReverse { array, .. } => vec![*array],
            Instruction::ArraySort { array, .. } => vec![*array],
            Instruction::ArrayFind { array, element, .. } => vec![*array, *element],
            Instruction::Eq { lhs, rhs, .. } => vec![*lhs, *rhs],
            Instruction::Ne { lhs, rhs, .. } => vec![*lhs, *rhs],
            Instruction::Lt { lhs, rhs, .. } => vec![*lhs, *rhs],
            Instruction::Le { lhs, rhs, .. } => vec![*lhs, *rhs],
            Instruction::Gt { lhs, rhs, .. } => vec![*lhs, *rhs],
            Instruction::Ge { lhs, rhs, .. } => vec![*lhs, *rhs],
            Instruction::And { lhs, rhs, .. } => vec![*lhs, *rhs],
            Instruction::Or { lhs, rhs, .. } => vec![*lhs, *rhs],
            Instruction::Not { src, .. } => vec![*src],
            Instruction::BitAnd { lhs, rhs, .. } => vec![*lhs, *rhs],
            Instruction::BitOr { lhs, rhs, .. } => vec![*lhs, *rhs],
            Instruction::BitXor { lhs, rhs, .. } => vec![*lhs, *rhs],
            Instruction::Shl { lhs, rhs, .. } => vec![*lhs, *rhs],
            Instruction::Shr { lhs, rhs, .. } => vec![*lhs, *rhs],
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
            Instruction::GaussianBlur { input, sigma, .. } => vec![*input, *sigma],
            Instruction::SobelEdges { input, threshold, .. } => vec![*input, *threshold],
            Instruction::Grayscale { input, .. } => vec![*input],
            Instruction::Threshold { input, value, .. } => vec![*input, *value],
            Instruction::Brightness { input, factor, .. } => vec![*input, *factor],
            Instruction::Contrast { input, factor, .. } => vec![*input, *factor],
            Instruction::InvertColors { input, .. } => vec![*input],
            Instruction::Sharpen { input, .. } => vec![*input],
            Instruction::ParseInt { input, .. } => vec![*input],
            Instruction::ParseFloat { input, .. } => vec![*input],
            Instruction::JsonSerialize { input, .. } => vec![*input],
            Instruction::CsvParse { input, delimiter, .. } => vec![*input, *delimiter],
            Instruction::FormatString { format, args, .. } => {
                let mut regs = vec![*format];
                regs.extend(args);
                regs
            },
            Instruction::RegexMatch { input, pattern, .. } => vec![*input, *pattern],
            Instruction::RegexReplace { input, pattern, replacement, .. } => vec![*input, *pattern, *replacement],
            Instruction::ReadLine { .. } => vec![],
            Instruction::AppendFile { path, content, .. } => vec![*path, *content],
            Instruction::FileExists { path, .. } => vec![*path],
            Instruction::DeleteFile { path, .. } => vec![*path],
            Instruction::ListFiles { path, .. } => vec![*path],
            Instruction::CreateDir { path, .. } => vec![*path],
            Instruction::DeleteDir { path, .. } => vec![*path],
            Instruction::ReadJson { path, .. } => vec![*path],
            Instruction::WriteJson { path, value, .. } => vec![*path, *value],
            Instruction::ReadCsv { path, delimiter, .. } => vec![*path, *delimiter],
            Instruction::WriteCsv { path, data, delimiter, .. } => vec![*path, *data, *delimiter],
            Instruction::LoadImage { path, .. } => vec![*path],
            Instruction::SaveImage { tensor, path, .. } => vec![*tensor, *path],
            Instruction::If { cond, .. } => vec![*cond],
            Instruction::Jump { .. } => vec![],
            Instruction::Loop { cond, .. } => vec![*cond],
            Instruction::FuncDef { .. } => vec![],
            Instruction::Call { args, .. } => args.clone(),
            Instruction::Return { val } => vec![*val],
            Instruction::Break => vec![],
            Instruction::Continue => vec![],
            Instruction::Barrier { .. } => vec![],
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
            Instruction::Asm { .. } => vec![],
            Instruction::ModuleDef { constants, .. } => {
                let mut regs = Vec::new();
                for (_, _, reg) in constants {
                    regs.push(*reg);
                }
                regs
            },
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
            Instruction::Asm { side_effects: true, .. } |
            Instruction::Call { .. } |
            Instruction::Return { .. } |
            Instruction::Break |
            Instruction::Continue |
            Instruction::Barrier { .. } |
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
