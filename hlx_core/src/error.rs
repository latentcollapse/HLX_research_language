//! Error types for HLX operations
//!
//! All errors are deterministic - same input always produces same error.

use thiserror::Error;

/// Result type for HLX operations
pub type Result<T> = std::result::Result<T, HlxError>;

/// Errors that can occur in HLX operations
#[derive(Error, Debug, Clone, PartialEq)]
pub enum HlxError {
    // === Parsing Errors ===
    #[error("E_PARSE_ERROR: {message}")]
    ParseError { message: String },

    #[error("E_LC_PARSE: Invalid LC-T syntax at position {position}")]
    LcParseError { position: usize },

    #[error("E_LC_BINARY_DECODE: Invalid LC-B encoding: {reason}")]
    LcBinaryDecode { reason: String },

    // === Type Errors ===
    #[error("E_TYPE_ERROR: Expected {expected}, got {got}")]
    TypeError { expected: String, got: String },

    #[error("E_FLOAT_SPECIAL: NaN or Infinity not allowed")]
    FloatSpecial,

    // === Structure Errors ===
    #[error("E_DEPTH_EXCEEDED: Nesting depth {depth} exceeds maximum {max}")]
    DepthExceeded { depth: usize, max: usize },

    #[error("E_FIELD_ORDER: Fields must be in ascending order by index")]
    FieldOrder,

    #[error("E_CONTRACT_STRUCTURE: Invalid contract structure: {reason}")]
    ContractStructure { reason: String },

    // === Handle/CAS Errors ===
    #[error("E_HANDLE_INVALID: Invalid handle format: {handle}")]
    HandleInvalid { handle: String },

    #[error("E_HANDLE_UNRESOLVED: Handle requires runtime resolution: {handle}")]
    HandleUnresolved { handle: String },

    #[error("E_HANDLE_NOT_FOUND: Handle not found: {handle}")]
    HandleNotFound { handle: String },

    // === Capsule Errors ===
    #[error("E_CAPSULE_INVALID: Capsule integrity check failed")]
    CapsuleInvalid,

    #[error("E_CAPSULE_VERSION: Unsupported capsule version {version}")]
    CapsuleVersion { version: u8 },

    // === Validation Errors ===
    #[error("E_VALIDATION_FAIL: {message}")]
    ValidationFail { message: String },

    #[error("E_CANONICALIZATION_FAIL: Cannot canonicalize: {reason}")]
    CanonicalizationFail { reason: String },

    // === Runtime Errors ===
    #[error("E_REGISTER_INVALID: Invalid register {reg}")]
    RegisterInvalid { reg: u32 },

    #[error("E_DIVISION_BY_ZERO: Division by zero")]
    DivisionByZero,

    #[error("E_INDEX_OUT_OF_BOUNDS: Index {index} out of bounds for length {len}")]
    IndexOutOfBounds { index: usize, len: usize },

    // === Envelope Errors ===
    #[error("E_ENV_PAYLOAD_HASH_MISMATCH: Merkle root mismatch")]
    EnvPayloadHashMismatch,

    #[error("E_ENV_MANIFEST_INVALID: Invalid LC_12 manifest")]
    EnvManifestInvalid,
}

impl HlxError {
    /// Create a parse error with message
    pub fn parse(msg: impl Into<String>) -> Self {
        Self::ParseError { message: msg.into() }
    }

    /// Create a type error with message
    pub fn type_err(expected: impl Into<String>, got: impl Into<String>) -> Self {
        Self::TypeError { 
            expected: expected.into(),
            got: got.into()
        }
    }

    /// Create a validation failure
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::ValidationFail { message: msg.into() }
    }
}
