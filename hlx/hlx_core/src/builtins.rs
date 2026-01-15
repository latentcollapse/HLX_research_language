//! Unified Builtin Function Registry
//!
//! Single source of truth for all HLX intrinsic functions.
//! Used by:
//! - Compiler (lowering, validation)
//! - LSP (type checking, signature help, completions)
//! - Backends (code generation)
//!
//! This eliminates duplication and ensures consistency across all components.

use std::collections::HashMap;

/// Parameter type for builtin functions
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParamType {
    /// Any type accepted
    Any,
    /// Integer
    Int,
    /// Float
    Float,
    /// String
    String,
    /// Boolean
    Bool,
    /// Array of any type
    Array,
    /// Object/map
    Object,
    /// Handle reference
    Handle,
}

impl ParamType {
    pub fn to_string(&self) -> &'static str {
        match self {
            ParamType::Any => "any",
            ParamType::Int => "Int",
            ParamType::Float => "Float",
            ParamType::String => "String",
            ParamType::Bool => "Bool",
            ParamType::Array => "Array",
            ParamType::Object => "Object",
            ParamType::Handle => "Handle",
        }
    }
}

/// Return type for builtin functions
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReturnType {
    /// Null (void)
    Null,
    /// Any type (runtime-determined)
    Any,
    /// Integer
    Int,
    /// Float
    Float,
    /// String
    String,
    /// Boolean
    Bool,
    /// Array
    Array,
    /// Object
    Object,
    /// Handle
    Handle,
}

impl ReturnType {
    pub fn to_string(&self) -> &'static str {
        match self {
            ReturnType::Null => "Null",
            ReturnType::Any => "any",
            ReturnType::Int => "Int",
            ReturnType::Float => "Float",
            ReturnType::String => "String",
            ReturnType::Bool => "Bool",
            ReturnType::Array => "Array",
            ReturnType::Object => "Object",
            ReturnType::Handle => "Handle",
        }
    }
}

/// Backend implementation category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendImpl {
    /// LLVM intrinsic (sqrt, sin, cos, etc.)
    LLVMIntrinsic,
    /// Runtime call (file I/O, HTTP, etc.)
    RuntimeCall,
    /// Special compiler handling (len, type, etc.)
    CompilerSpecial,
    /// Math operation (basic arithmetic)
    Math,
}

/// Builtin function signature
#[derive(Debug, Clone)]
pub struct BuiltinSignature {
    /// Function name
    pub name: &'static str,
    /// Parameter types (empty for variadic starting at 0)
    pub params: Vec<ParamType>,
    /// Minimum argument count
    pub min_args: usize,
    /// Maximum argument count (None = unbounded)
    pub max_args: Option<usize>,
    /// Return type
    pub return_type: ReturnType,
    /// Human-readable description
    pub description: &'static str,
    /// Backend implementation category
    pub backend_impl: BackendImpl,
}

impl BuiltinSignature {
    /// Create a fixed-arity builtin
    pub fn fixed(
        name: &'static str,
        params: Vec<ParamType>,
        return_type: ReturnType,
        description: &'static str,
        backend_impl: BackendImpl,
    ) -> Self {
        let arg_count = params.len();
        Self {
            name,
            params,
            min_args: arg_count,
            max_args: Some(arg_count),
            return_type,
            description,
            backend_impl,
        }
    }

    /// Create a variadic builtin (unlimited args)
    pub fn variadic(
        name: &'static str,
        min_args: usize,
        return_type: ReturnType,
        description: &'static str,
        backend_impl: BackendImpl,
    ) -> Self {
        Self {
            name,
            params: vec![],
            min_args,
            max_args: None,
            return_type,
            description,
            backend_impl,
        }
    }

    /// Create a range-arity builtin (min to max args)
    pub fn range(
        name: &'static str,
        params: Vec<ParamType>,
        min_args: usize,
        max_args: usize,
        return_type: ReturnType,
        description: &'static str,
        backend_impl: BackendImpl,
    ) -> Self {
        Self {
            name,
            params,
            min_args,
            max_args: Some(max_args),
            return_type,
            description,
            backend_impl,
        }
    }

    /// Validate argument count
    pub fn validate_arg_count(&self, count: usize) -> Result<(), String> {
        if count < self.min_args {
            return Err(format!(
                "{}() requires at least {} argument(s), got {}",
                self.name, self.min_args, count
            ));
        }
        if let Some(max) = self.max_args {
            if count > max {
                return Err(format!(
                    "{}() accepts at most {} argument(s), got {}",
                    self.name, max, count
                ));
            }
        }
        Ok(())
    }

    /// Get parameter type at index (for type checking)
    pub fn param_type(&self, index: usize) -> Option<&ParamType> {
        self.params.get(index)
    }
}

/// Registry of all HLX builtin functions
pub struct BuiltinRegistry {
    builtins: HashMap<&'static str, BuiltinSignature>,
}

impl BuiltinRegistry {
    /// Create a new registry with all builtins registered
    pub fn new() -> Self {
        let mut builtins = HashMap::new();

        // === I/O Functions ===
        builtins.insert(
            "print",
            BuiltinSignature::variadic(
                "print",
                0,
                ReturnType::Null,
                "Print values to stdout",
                BackendImpl::RuntimeCall,
            ),
        );

        builtins.insert(
            "read_file",
            BuiltinSignature::fixed(
                "read_file",
                vec![ParamType::String],
                ReturnType::String,
                "Read file contents as string",
                BackendImpl::RuntimeCall,
            ),
        );

        builtins.insert(
            "write_file",
            BuiltinSignature::fixed(
                "write_file",
                vec![ParamType::String, ParamType::String],
                ReturnType::Null,
                "Write string to file (path, content)",
                BackendImpl::RuntimeCall,
            ),
        );

        builtins.insert(
            "file_exists",
            BuiltinSignature::fixed(
                "file_exists",
                vec![ParamType::String],
                ReturnType::Bool,
                "Check if file exists",
                BackendImpl::RuntimeCall,
            ),
        );

        builtins.insert(
            "delete_file",
            BuiltinSignature::fixed(
                "delete_file",
                vec![ParamType::String],
                ReturnType::Null,
                "Delete a file",
                BackendImpl::RuntimeCall,
            ),
        );

        builtins.insert(
            "list_files",
            BuiltinSignature::fixed(
                "list_files",
                vec![ParamType::String],
                ReturnType::Array,
                "List files in directory",
                BackendImpl::RuntimeCall,
            ),
        );

        builtins.insert(
            "create_dir",
            BuiltinSignature::fixed(
                "create_dir",
                vec![ParamType::String],
                ReturnType::Null,
                "Create directory",
                BackendImpl::RuntimeCall,
            ),
        );

        // === Type Introspection ===
        builtins.insert(
            "type",
            BuiltinSignature::fixed(
                "type",
                vec![ParamType::Any],
                ReturnType::String,
                "Get type name of value",
                BackendImpl::CompilerSpecial,
            ),
        );

        builtins.insert(
            "len",
            BuiltinSignature::fixed(
                "len",
                vec![ParamType::Any],
                ReturnType::Int,
                "Get length of array, string, or object",
                BackendImpl::CompilerSpecial,
            ),
        );

        // === Array Operations ===
        builtins.insert(
            "slice",
            BuiltinSignature::range(
                "slice",
                vec![ParamType::Array, ParamType::Int, ParamType::Int],
                2,
                3,
                ReturnType::Array,
                "Slice array (arr, start, end?)",
                BackendImpl::RuntimeCall,
            ),
        );

        builtins.insert(
            "append",
            BuiltinSignature::fixed(
                "append",
                vec![ParamType::Array, ParamType::Any],
                ReturnType::Array,
                "Append element to array (returns new array)",
                BackendImpl::RuntimeCall,
            ),
        );

        builtins.insert(
            "arr_pop",
            BuiltinSignature::fixed(
                "arr_pop",
                vec![ParamType::Array],
                ReturnType::Any,
                "Remove and return last element",
                BackendImpl::RuntimeCall,
            ),
        );

        builtins.insert(
            "arr_slice",
            BuiltinSignature::fixed(
                "arr_slice",
                vec![ParamType::Array, ParamType::Int, ParamType::Int],
                ReturnType::Array,
                "Slice array (arr, start, end)",
                BackendImpl::RuntimeCall,
            ),
        );

        builtins.insert(
            "arr_concat",
            BuiltinSignature::fixed(
                "arr_concat",
                vec![ParamType::Array, ParamType::Array],
                ReturnType::Array,
                "Concatenate two arrays",
                BackendImpl::RuntimeCall,
            ),
        );

        // === String Operations ===
        builtins.insert(
            "concat",
            BuiltinSignature::variadic(
                "concat",
                2,
                ReturnType::String,
                "Concatenate strings",
                BackendImpl::RuntimeCall,
            ),
        );

        builtins.insert(
            "strlen",
            BuiltinSignature::fixed(
                "strlen",
                vec![ParamType::String],
                ReturnType::Int,
                "Get string length",
                BackendImpl::CompilerSpecial,
            ),
        );

        builtins.insert(
            "str_get",
            BuiltinSignature::fixed(
                "str_get",
                vec![ParamType::String, ParamType::Int],
                ReturnType::Int,
                "Get character code at index",
                BackendImpl::CompilerSpecial,
            ),
        );

        builtins.insert(
            "substring",
            BuiltinSignature::fixed(
                "substring",
                vec![ParamType::String, ParamType::Int, ParamType::Int],
                ReturnType::String,
                "Get substring (str, start, end)",
                BackendImpl::RuntimeCall,
            ),
        );

        builtins.insert(
            "index_of",
            BuiltinSignature::fixed(
                "index_of",
                vec![ParamType::String, ParamType::String],
                ReturnType::Int,
                "Find substring index (-1 if not found)",
                BackendImpl::RuntimeCall,
            ),
        );

        builtins.insert(
            "to_upper",
            BuiltinSignature::fixed(
                "to_upper",
                vec![ParamType::String],
                ReturnType::String,
                "Convert to uppercase",
                BackendImpl::RuntimeCall,
            ),
        );

        builtins.insert(
            "to_lower",
            BuiltinSignature::fixed(
                "to_lower",
                vec![ParamType::String],
                ReturnType::String,
                "Convert to lowercase",
                BackendImpl::RuntimeCall,
            ),
        );

        builtins.insert(
            "trim",
            BuiltinSignature::fixed(
                "trim",
                vec![ParamType::String],
                ReturnType::String,
                "Trim whitespace from both ends",
                BackendImpl::RuntimeCall,
            ),
        );

        builtins.insert(
            "starts_with",
            BuiltinSignature::fixed(
                "starts_with",
                vec![ParamType::String, ParamType::String],
                ReturnType::Bool,
                "Check if string starts with prefix",
                BackendImpl::RuntimeCall,
            ),
        );

        builtins.insert(
            "ends_with",
            BuiltinSignature::fixed(
                "ends_with",
                vec![ParamType::String, ParamType::String],
                ReturnType::Bool,
                "Check if string ends with suffix",
                BackendImpl::RuntimeCall,
            ),
        );

        // === Type Conversions ===
        builtins.insert(
            "to_string",
            BuiltinSignature::fixed(
                "to_string",
                vec![ParamType::Any],
                ReturnType::String,
                "Convert value to string",
                BackendImpl::RuntimeCall,
            ),
        );

        builtins.insert(
            "to_int",
            BuiltinSignature::fixed(
                "to_int",
                vec![ParamType::Any],
                ReturnType::Int,
                "Convert value to integer (truncates floats)",
                BackendImpl::CompilerSpecial,
            ),
        );

        builtins.insert(
            "to_float",
            BuiltinSignature::fixed(
                "to_float",
                vec![ParamType::Any],
                ReturnType::Float,
                "Convert value to float",
                BackendImpl::CompilerSpecial,
            ),
        );

        builtins.insert(
            "parse_int",
            BuiltinSignature::fixed(
                "parse_int",
                vec![ParamType::String],
                ReturnType::Int,
                "Parse string as integer",
                BackendImpl::RuntimeCall,
            ),
        );

        builtins.insert(
            "ord",
            BuiltinSignature::fixed(
                "ord",
                vec![ParamType::String],
                ReturnType::Int,
                "Get ASCII code of first character",
                BackendImpl::RuntimeCall,
            ),
        );

        // === Math Functions ===
        builtins.insert(
            "floor",
            BuiltinSignature::fixed(
                "floor",
                vec![ParamType::Float],
                ReturnType::Float,
                "Round down to nearest integer",
                BackendImpl::LLVMIntrinsic,
            ),
        );

        builtins.insert(
            "ceil",
            BuiltinSignature::fixed(
                "ceil",
                vec![ParamType::Float],
                ReturnType::Float,
                "Round up to nearest integer",
                BackendImpl::LLVMIntrinsic,
            ),
        );

        builtins.insert(
            "round",
            BuiltinSignature::fixed(
                "round",
                vec![ParamType::Float],
                ReturnType::Float,
                "Round to nearest integer",
                BackendImpl::LLVMIntrinsic,
            ),
        );

        builtins.insert(
            "sqrt",
            BuiltinSignature::fixed(
                "sqrt",
                vec![ParamType::Float],
                ReturnType::Float,
                "Square root",
                BackendImpl::LLVMIntrinsic,
            ),
        );

        builtins.insert(
            "abs",
            BuiltinSignature::fixed(
                "abs",
                vec![ParamType::Float],
                ReturnType::Float,
                "Absolute value",
                BackendImpl::LLVMIntrinsic,
            ),
        );

        builtins.insert(
            "sin",
            BuiltinSignature::fixed(
                "sin",
                vec![ParamType::Float],
                ReturnType::Float,
                "Sine (radians)",
                BackendImpl::LLVMIntrinsic,
            ),
        );

        builtins.insert(
            "cos",
            BuiltinSignature::fixed(
                "cos",
                vec![ParamType::Float],
                ReturnType::Float,
                "Cosine (radians)",
                BackendImpl::LLVMIntrinsic,
            ),
        );

        builtins.insert(
            "tan",
            BuiltinSignature::fixed(
                "tan",
                vec![ParamType::Float],
                ReturnType::Float,
                "Tangent (radians)",
                BackendImpl::LLVMIntrinsic,
            ),
        );

        builtins.insert(
            "log",
            BuiltinSignature::fixed(
                "log",
                vec![ParamType::Float],
                ReturnType::Float,
                "Natural logarithm",
                BackendImpl::LLVMIntrinsic,
            ),
        );

        builtins.insert(
            "exp",
            BuiltinSignature::fixed(
                "exp",
                vec![ParamType::Float],
                ReturnType::Float,
                "Exponential (e^x)",
                BackendImpl::LLVMIntrinsic,
            ),
        );

        builtins.insert(
            "pow",
            BuiltinSignature::fixed(
                "pow",
                vec![ParamType::Float, ParamType::Float],
                ReturnType::Float,
                "Power (base^exponent)",
                BackendImpl::LLVMIntrinsic,
            ),
        );

        builtins.insert(
            "min",
            BuiltinSignature::fixed(
                "min",
                vec![ParamType::Float, ParamType::Float],
                ReturnType::Float,
                "Minimum of two values",
                BackendImpl::Math,
            ),
        );

        builtins.insert(
            "max",
            BuiltinSignature::fixed(
                "max",
                vec![ParamType::Float, ParamType::Float],
                ReturnType::Float,
                "Maximum of two values",
                BackendImpl::Math,
            ),
        );

        builtins.insert(
            "random",
            BuiltinSignature::fixed(
                "random",
                vec![],
                ReturnType::Float,
                "Random float in [0, 1)",
                BackendImpl::RuntimeCall,
            ),
        );

        // === Object Operations ===
        builtins.insert(
            "has_key",
            BuiltinSignature::fixed(
                "has_key",
                vec![ParamType::Object, ParamType::String],
                ReturnType::Bool,
                "Check if object has key",
                BackendImpl::RuntimeCall,
            ),
        );

        // === JSON Operations ===
        builtins.insert(
            "json_parse",
            BuiltinSignature::fixed(
                "json_parse",
                vec![ParamType::String],
                ReturnType::Any,
                "Parse JSON string",
                BackendImpl::RuntimeCall,
            ),
        );

        builtins.insert(
            "json_stringify",
            BuiltinSignature::fixed(
                "json_stringify",
                vec![ParamType::Any],
                ReturnType::String,
                "Convert value to JSON string",
                BackendImpl::RuntimeCall,
            ),
        );

        builtins.insert(
            "read_json",
            BuiltinSignature::fixed(
                "read_json",
                vec![ParamType::String],
                ReturnType::Any,
                "Read and parse JSON file",
                BackendImpl::RuntimeCall,
            ),
        );

        builtins.insert(
            "write_json",
            BuiltinSignature::fixed(
                "write_json",
                vec![ParamType::String, ParamType::Any],
                ReturnType::Null,
                "Write value as JSON file",
                BackendImpl::RuntimeCall,
            ),
        );

        // === HTTP Operations ===
        builtins.insert(
            "http_request",
            BuiltinSignature::fixed(
                "http_request",
                vec![ParamType::Object],
                ReturnType::Object,
                "Make HTTP request (config object)",
                BackendImpl::RuntimeCall,
            ),
        );

        // === Runtime Introspection ===
        builtins.insert(
            "snapshot",
            BuiltinSignature::fixed(
                "snapshot",
                vec![],
                ReturnType::Object,
                "Create VM state snapshot",
                BackendImpl::RuntimeCall,
            ),
        );

        builtins.insert(
            "export_trace",
            BuiltinSignature::fixed(
                "export_trace",
                vec![],
                ReturnType::Array,
                "Export execution trace",
                BackendImpl::RuntimeCall,
            ),
        );

        builtins.insert(
            "write_snapshot",
            BuiltinSignature::fixed(
                "write_snapshot",
                vec![ParamType::String],
                ReturnType::Null,
                "Write snapshot to file",
                BackendImpl::RuntimeCall,
            ),
        );

        // === Process Management ===
        builtins.insert(
            "pipe_open",
            BuiltinSignature::fixed(
                "pipe_open",
                vec![ParamType::String],
                ReturnType::Handle,
                "Open subprocess pipe",
                BackendImpl::RuntimeCall,
            ),
        );

        builtins.insert(
            "pipe_write",
            BuiltinSignature::fixed(
                "pipe_write",
                vec![ParamType::Handle, ParamType::String],
                ReturnType::Null,
                "Write to subprocess pipe",
                BackendImpl::RuntimeCall,
            ),
        );

        builtins.insert(
            "pipe_close",
            BuiltinSignature::fixed(
                "pipe_close",
                vec![ParamType::Handle],
                ReturnType::Null,
                "Close subprocess pipe",
                BackendImpl::RuntimeCall,
            ),
        );

        // === System Operations ===
        builtins.insert(
            "sleep",
            BuiltinSignature::fixed(
                "sleep",
                vec![ParamType::Int],
                ReturnType::Null,
                "Sleep for milliseconds",
                BackendImpl::RuntimeCall,
            ),
        );

        builtins.insert(
            "capture_screen",
            BuiltinSignature::fixed(
                "capture_screen",
                vec![ParamType::String],
                ReturnType::Null,
                "Capture screenshot to file",
                BackendImpl::RuntimeCall,
            ),
        );

        // === Metaprogramming ===
        builtins.insert(
            "native_tokenize",
            BuiltinSignature::fixed(
                "native_tokenize",
                vec![ParamType::String],
                ReturnType::Array,
                "Tokenize HLX source code",
                BackendImpl::CompilerSpecial,
            ),
        );

        // === Additional Math Functions ===
        builtins.insert("asin", BuiltinSignature::fixed("asin", vec![ParamType::Float], ReturnType::Float, "Arcsine", BackendImpl::LLVMIntrinsic));
        builtins.insert("acos", BuiltinSignature::fixed("acos", vec![ParamType::Float], ReturnType::Float, "Arccosine", BackendImpl::LLVMIntrinsic));
        builtins.insert("atan", BuiltinSignature::fixed("atan", vec![ParamType::Float], ReturnType::Float, "Arctangent", BackendImpl::LLVMIntrinsic));
        builtins.insert("atan2", BuiltinSignature::fixed("atan2", vec![ParamType::Float, ParamType::Float], ReturnType::Float, "Two-argument arctangent", BackendImpl::LLVMIntrinsic));
        builtins.insert("sinh", BuiltinSignature::fixed("sinh", vec![ParamType::Float], ReturnType::Float, "Hyperbolic sine", BackendImpl::LLVMIntrinsic));
        builtins.insert("cosh", BuiltinSignature::fixed("cosh", vec![ParamType::Float], ReturnType::Float, "Hyperbolic cosine", BackendImpl::LLVMIntrinsic));
        builtins.insert("tanh", BuiltinSignature::fixed("tanh", vec![ParamType::Float], ReturnType::Float, "Hyperbolic tangent", BackendImpl::LLVMIntrinsic));
        builtins.insert("log2", BuiltinSignature::fixed("log2", vec![ParamType::Float], ReturnType::Float, "Base-2 logarithm", BackendImpl::LLVMIntrinsic));
        builtins.insert("log10", BuiltinSignature::fixed("log10", vec![ParamType::Float], ReturnType::Float, "Base-10 logarithm", BackendImpl::LLVMIntrinsic));
        builtins.insert("cbrt", BuiltinSignature::fixed("cbrt", vec![ParamType::Float], ReturnType::Float, "Cube root", BackendImpl::LLVMIntrinsic));
        builtins.insert("hypot", BuiltinSignature::fixed("hypot", vec![ParamType::Float, ParamType::Float], ReturnType::Float, "Euclidean distance (sqrt(x²+y²))", BackendImpl::LLVMIntrinsic));
        builtins.insert("trunc", BuiltinSignature::fixed("trunc", vec![ParamType::Float], ReturnType::Float, "Truncate to integer value", BackendImpl::LLVMIntrinsic));
        builtins.insert("sign", BuiltinSignature::fixed("sign", vec![ParamType::Float], ReturnType::Int, "Sign of number (-1, 0, or 1)", BackendImpl::Math));
        builtins.insert("clamp", BuiltinSignature::fixed("clamp", vec![ParamType::Float, ParamType::Float, ParamType::Float], ReturnType::Float, "Clamp value between min and max", BackendImpl::Math));
        builtins.insert("lerp", BuiltinSignature::fixed("lerp", vec![ParamType::Float, ParamType::Float, ParamType::Float], ReturnType::Float, "Linear interpolation", BackendImpl::Math));
        builtins.insert("degrees", BuiltinSignature::fixed("degrees", vec![ParamType::Float], ReturnType::Float, "Radians to degrees", BackendImpl::Math));
        builtins.insert("radians", BuiltinSignature::fixed("radians", vec![ParamType::Float], ReturnType::Float, "Degrees to radians", BackendImpl::Math));
        builtins.insert("gcd", BuiltinSignature::fixed("gcd", vec![ParamType::Int, ParamType::Int], ReturnType::Int, "Greatest common divisor", BackendImpl::Math));
        builtins.insert("lcm", BuiltinSignature::fixed("lcm", vec![ParamType::Int, ParamType::Int], ReturnType::Int, "Least common multiple", BackendImpl::Math));
        builtins.insert("factorial", BuiltinSignature::fixed("factorial", vec![ParamType::Int], ReturnType::Int, "Factorial (n!)", BackendImpl::Math));
        builtins.insert("is_nan", BuiltinSignature::fixed("is_nan", vec![ParamType::Float], ReturnType::Bool, "Check if value is NaN", BackendImpl::Math));
        builtins.insert("is_inf", BuiltinSignature::fixed("is_inf", vec![ParamType::Float], ReturnType::Bool, "Check if value is infinite", BackendImpl::Math));

        // === String Functions (Extended) ===
        builtins.insert("split", BuiltinSignature::fixed("split", vec![ParamType::String, ParamType::String], ReturnType::Array, "Split string by delimiter", BackendImpl::RuntimeCall));
        builtins.insert("join", BuiltinSignature::fixed("join", vec![ParamType::Array, ParamType::String], ReturnType::String, "Join array elements with delimiter", BackendImpl::RuntimeCall));
        builtins.insert("replace", BuiltinSignature::fixed("replace", vec![ParamType::String, ParamType::String, ParamType::String], ReturnType::String, "Replace all occurrences", BackendImpl::RuntimeCall));
        builtins.insert("replace_first", BuiltinSignature::fixed("replace_first", vec![ParamType::String, ParamType::String, ParamType::String], ReturnType::String, "Replace first occurrence", BackendImpl::RuntimeCall));
        builtins.insert("pad_left", BuiltinSignature::fixed("pad_left", vec![ParamType::String, ParamType::Int, ParamType::String], ReturnType::String, "Pad string on left", BackendImpl::RuntimeCall));
        builtins.insert("pad_right", BuiltinSignature::fixed("pad_right", vec![ParamType::String, ParamType::Int, ParamType::String], ReturnType::String, "Pad string on right", BackendImpl::RuntimeCall));
        builtins.insert("repeat", BuiltinSignature::fixed("repeat", vec![ParamType::String, ParamType::Int], ReturnType::String, "Repeat string n times", BackendImpl::RuntimeCall));
        builtins.insert("reverse_str", BuiltinSignature::fixed("reverse_str", vec![ParamType::String], ReturnType::String, "Reverse string", BackendImpl::RuntimeCall));
        builtins.insert("contains", BuiltinSignature::fixed("contains", vec![ParamType::String, ParamType::String], ReturnType::Bool, "Check if string contains substring", BackendImpl::RuntimeCall));
        builtins.insert("char_at", BuiltinSignature::fixed("char_at", vec![ParamType::String, ParamType::Int], ReturnType::String, "Get character at index", BackendImpl::RuntimeCall));
        builtins.insert("is_alpha", BuiltinSignature::fixed("is_alpha", vec![ParamType::String], ReturnType::Bool, "Check if string is alphabetic", BackendImpl::RuntimeCall));
        builtins.insert("is_numeric", BuiltinSignature::fixed("is_numeric", vec![ParamType::String], ReturnType::Bool, "Check if string is numeric", BackendImpl::RuntimeCall));
        builtins.insert("is_alphanumeric", BuiltinSignature::fixed("is_alphanumeric", vec![ParamType::String], ReturnType::Bool, "Check if string is alphanumeric", BackendImpl::RuntimeCall));
        builtins.insert("format", BuiltinSignature::variadic("format", 1, ReturnType::String, "Format string with arguments", BackendImpl::RuntimeCall));
        builtins.insert("match_regex", BuiltinSignature::fixed("match_regex", vec![ParamType::String, ParamType::String], ReturnType::Bool, "Match string against regex", BackendImpl::RuntimeCall));
        builtins.insert("find_regex", BuiltinSignature::fixed("find_regex", vec![ParamType::String, ParamType::String], ReturnType::Array, "Find all regex matches", BackendImpl::RuntimeCall));

        // === Array Functions (Extended) ===
        builtins.insert("map", BuiltinSignature::fixed("map", vec![ParamType::Array, ParamType::String], ReturnType::Array, "Map function over array", BackendImpl::CompilerSpecial));
        builtins.insert("filter", BuiltinSignature::fixed("filter", vec![ParamType::Array, ParamType::String], ReturnType::Array, "Filter array by predicate", BackendImpl::CompilerSpecial));
        builtins.insert("reduce", BuiltinSignature::fixed("reduce", vec![ParamType::Array, ParamType::String, ParamType::Any], ReturnType::Any, "Reduce array to single value", BackendImpl::CompilerSpecial));
        builtins.insert("sort", BuiltinSignature::fixed("sort", vec![ParamType::Array], ReturnType::Array, "Sort array", BackendImpl::RuntimeCall));
        builtins.insert("sort_by", BuiltinSignature::fixed("sort_by", vec![ParamType::Array, ParamType::String], ReturnType::Array, "Sort array by key function", BackendImpl::CompilerSpecial));
        builtins.insert("reverse", BuiltinSignature::fixed("reverse", vec![ParamType::Array], ReturnType::Array, "Reverse array", BackendImpl::RuntimeCall));
        builtins.insert("find", BuiltinSignature::fixed("find", vec![ParamType::Array, ParamType::String], ReturnType::Any, "Find first matching element", BackendImpl::CompilerSpecial));
        builtins.insert("find_index", BuiltinSignature::fixed("find_index", vec![ParamType::Array, ParamType::String], ReturnType::Int, "Find index of first match", BackendImpl::CompilerSpecial));
        builtins.insert("every", BuiltinSignature::fixed("every", vec![ParamType::Array, ParamType::String], ReturnType::Bool, "Check if all elements match", BackendImpl::CompilerSpecial));
        builtins.insert("some", BuiltinSignature::fixed("some", vec![ParamType::Array, ParamType::String], ReturnType::Bool, "Check if any element matches", BackendImpl::CompilerSpecial));
        builtins.insert("flatten", BuiltinSignature::fixed("flatten", vec![ParamType::Array], ReturnType::Array, "Flatten nested array one level", BackendImpl::RuntimeCall));
        builtins.insert("flatten_deep", BuiltinSignature::fixed("flatten_deep", vec![ParamType::Array], ReturnType::Array, "Flatten nested array recursively", BackendImpl::RuntimeCall));
        builtins.insert("unique", BuiltinSignature::fixed("unique", vec![ParamType::Array], ReturnType::Array, "Remove duplicates", BackendImpl::RuntimeCall));
        builtins.insert("zip", BuiltinSignature::fixed("zip", vec![ParamType::Array, ParamType::Array], ReturnType::Array, "Zip two arrays into pairs", BackendImpl::RuntimeCall));
        builtins.insert("unzip", BuiltinSignature::fixed("unzip", vec![ParamType::Array], ReturnType::Array, "Unzip array of pairs", BackendImpl::RuntimeCall));
        builtins.insert("partition", BuiltinSignature::fixed("partition", vec![ParamType::Array, ParamType::String], ReturnType::Array, "Partition array by predicate", BackendImpl::CompilerSpecial));
        builtins.insert("chunk", BuiltinSignature::fixed("chunk", vec![ParamType::Array, ParamType::Int], ReturnType::Array, "Split array into chunks of size n", BackendImpl::RuntimeCall));
        builtins.insert("take", BuiltinSignature::fixed("take", vec![ParamType::Array, ParamType::Int], ReturnType::Array, "Take first n elements", BackendImpl::RuntimeCall));
        builtins.insert("drop", BuiltinSignature::fixed("drop", vec![ParamType::Array, ParamType::Int], ReturnType::Array, "Drop first n elements", BackendImpl::RuntimeCall));
        builtins.insert("take_while", BuiltinSignature::fixed("take_while", vec![ParamType::Array, ParamType::String], ReturnType::Array, "Take elements while predicate is true", BackendImpl::CompilerSpecial));
        builtins.insert("drop_while", BuiltinSignature::fixed("drop_while", vec![ParamType::Array, ParamType::String], ReturnType::Array, "Drop elements while predicate is true", BackendImpl::CompilerSpecial));
        builtins.insert("sum", BuiltinSignature::fixed("sum", vec![ParamType::Array], ReturnType::Float, "Sum array elements", BackendImpl::RuntimeCall));
        builtins.insert("product", BuiltinSignature::fixed("product", vec![ParamType::Array], ReturnType::Float, "Product of array elements", BackendImpl::RuntimeCall));
        builtins.insert("mean", BuiltinSignature::fixed("mean", vec![ParamType::Array], ReturnType::Float, "Mean of array elements", BackendImpl::RuntimeCall));
        builtins.insert("median", BuiltinSignature::fixed("median", vec![ParamType::Array], ReturnType::Float, "Median of array elements", BackendImpl::RuntimeCall));
        builtins.insert("mode", BuiltinSignature::fixed("mode", vec![ParamType::Array], ReturnType::Any, "Mode of array elements", BackendImpl::RuntimeCall));
        builtins.insert("range", BuiltinSignature::range("range", vec![ParamType::Int, ParamType::Int, ParamType::Int], 1, 3, ReturnType::Array, "Generate range of integers", BackendImpl::RuntimeCall));

        // === Object/Map Functions ===
        builtins.insert("keys", BuiltinSignature::fixed("keys", vec![ParamType::Object], ReturnType::Array, "Get object keys", BackendImpl::RuntimeCall));
        builtins.insert("values", BuiltinSignature::fixed("values", vec![ParamType::Object], ReturnType::Array, "Get object values", BackendImpl::RuntimeCall));
        builtins.insert("entries", BuiltinSignature::fixed("entries", vec![ParamType::Object], ReturnType::Array, "Get object entries as [key, value] pairs", BackendImpl::RuntimeCall));
        builtins.insert("merge", BuiltinSignature::variadic("merge", 2, ReturnType::Object, "Merge objects (rightmost wins)", BackendImpl::RuntimeCall));
        builtins.insert("omit", BuiltinSignature::fixed("omit", vec![ParamType::Object, ParamType::Array], ReturnType::Object, "Omit keys from object", BackendImpl::RuntimeCall));
        builtins.insert("pick", BuiltinSignature::fixed("pick", vec![ParamType::Object, ParamType::Array], ReturnType::Object, "Pick keys from object", BackendImpl::RuntimeCall));
        builtins.insert("map_values", BuiltinSignature::fixed("map_values", vec![ParamType::Object, ParamType::String], ReturnType::Object, "Map function over object values", BackendImpl::CompilerSpecial));
        builtins.insert("map_keys", BuiltinSignature::fixed("map_keys", vec![ParamType::Object, ParamType::String], ReturnType::Object, "Map function over object keys", BackendImpl::CompilerSpecial));
        builtins.insert("from_entries", BuiltinSignature::fixed("from_entries", vec![ParamType::Array], ReturnType::Object, "Create object from [key, value] pairs", BackendImpl::RuntimeCall));

        // === Date/Time Functions ===
        builtins.insert("now", BuiltinSignature::fixed("now", vec![], ReturnType::Int, "Current Unix timestamp (milliseconds)", BackendImpl::RuntimeCall));
        builtins.insert("now_micros", BuiltinSignature::fixed("now_micros", vec![], ReturnType::Int, "Current Unix timestamp (microseconds)", BackendImpl::RuntimeCall));
        builtins.insert("format_timestamp", BuiltinSignature::fixed("format_timestamp", vec![ParamType::Int, ParamType::String], ReturnType::String, "Format Unix timestamp", BackendImpl::RuntimeCall));
        builtins.insert("parse_timestamp", BuiltinSignature::fixed("parse_timestamp", vec![ParamType::String, ParamType::String], ReturnType::Int, "Parse timestamp string", BackendImpl::RuntimeCall));
        builtins.insert("year", BuiltinSignature::fixed("year", vec![ParamType::Int], ReturnType::Int, "Extract year from timestamp", BackendImpl::RuntimeCall));
        builtins.insert("month", BuiltinSignature::fixed("month", vec![ParamType::Int], ReturnType::Int, "Extract month from timestamp", BackendImpl::RuntimeCall));
        builtins.insert("day", BuiltinSignature::fixed("day", vec![ParamType::Int], ReturnType::Int, "Extract day from timestamp", BackendImpl::RuntimeCall));
        builtins.insert("hour", BuiltinSignature::fixed("hour", vec![ParamType::Int], ReturnType::Int, "Extract hour from timestamp", BackendImpl::RuntimeCall));
        builtins.insert("minute", BuiltinSignature::fixed("minute", vec![ParamType::Int], ReturnType::Int, "Extract minute from timestamp", BackendImpl::RuntimeCall));
        builtins.insert("second", BuiltinSignature::fixed("second", vec![ParamType::Int], ReturnType::Int, "Extract second from timestamp", BackendImpl::RuntimeCall));

        // === Encoding/Decoding ===
        builtins.insert("base64_encode", BuiltinSignature::fixed("base64_encode", vec![ParamType::String], ReturnType::String, "Base64 encode", BackendImpl::RuntimeCall));
        builtins.insert("base64_decode", BuiltinSignature::fixed("base64_decode", vec![ParamType::String], ReturnType::String, "Base64 decode", BackendImpl::RuntimeCall));
        builtins.insert("hex_encode", BuiltinSignature::fixed("hex_encode", vec![ParamType::String], ReturnType::String, "Hex encode", BackendImpl::RuntimeCall));
        builtins.insert("hex_decode", BuiltinSignature::fixed("hex_decode", vec![ParamType::String], ReturnType::String, "Hex decode", BackendImpl::RuntimeCall));
        builtins.insert("url_encode", BuiltinSignature::fixed("url_encode", vec![ParamType::String], ReturnType::String, "URL encode", BackendImpl::RuntimeCall));
        builtins.insert("url_decode", BuiltinSignature::fixed("url_decode", vec![ParamType::String], ReturnType::String, "URL decode", BackendImpl::RuntimeCall));

        // === Cryptography/Hashing ===
        builtins.insert("sha256", BuiltinSignature::fixed("sha256", vec![ParamType::String], ReturnType::String, "SHA-256 hash", BackendImpl::RuntimeCall));
        builtins.insert("sha512", BuiltinSignature::fixed("sha512", vec![ParamType::String], ReturnType::String, "SHA-512 hash", BackendImpl::RuntimeCall));
        builtins.insert("blake3", BuiltinSignature::fixed("blake3", vec![ParamType::String], ReturnType::String, "BLAKE3 hash", BackendImpl::RuntimeCall));
        builtins.insert("md5", BuiltinSignature::fixed("md5", vec![ParamType::String], ReturnType::String, "MD5 hash (insecure, for compatibility)", BackendImpl::RuntimeCall));
        builtins.insert("hmac_sha256", BuiltinSignature::fixed("hmac_sha256", vec![ParamType::String, ParamType::String], ReturnType::String, "HMAC-SHA256", BackendImpl::RuntimeCall));

        // === Bit Operations ===
        builtins.insert("bit_and", BuiltinSignature::fixed("bit_and", vec![ParamType::Int, ParamType::Int], ReturnType::Int, "Bitwise AND", BackendImpl::Math));
        builtins.insert("bit_or", BuiltinSignature::fixed("bit_or", vec![ParamType::Int, ParamType::Int], ReturnType::Int, "Bitwise OR", BackendImpl::Math));
        builtins.insert("bit_xor", BuiltinSignature::fixed("bit_xor", vec![ParamType::Int, ParamType::Int], ReturnType::Int, "Bitwise XOR", BackendImpl::Math));
        builtins.insert("bit_not", BuiltinSignature::fixed("bit_not", vec![ParamType::Int], ReturnType::Int, "Bitwise NOT", BackendImpl::Math));
        builtins.insert("bit_shl", BuiltinSignature::fixed("bit_shl", vec![ParamType::Int, ParamType::Int], ReturnType::Int, "Bit shift left", BackendImpl::Math));
        builtins.insert("bit_shr", BuiltinSignature::fixed("bit_shr", vec![ParamType::Int, ParamType::Int], ReturnType::Int, "Bit shift right", BackendImpl::Math));
        builtins.insert("bit_count", BuiltinSignature::fixed("bit_count", vec![ParamType::Int], ReturnType::Int, "Count set bits (popcount)", BackendImpl::Math));
        builtins.insert("bit_reverse", BuiltinSignature::fixed("bit_reverse", vec![ParamType::Int], ReturnType::Int, "Reverse bits", BackendImpl::Math));

        // === Utility Functions ===
        builtins.insert("identity", BuiltinSignature::fixed("identity", vec![ParamType::Any], ReturnType::Any, "Return input unchanged", BackendImpl::CompilerSpecial));
        builtins.insert("const", BuiltinSignature::fixed("const", vec![ParamType::Any, ParamType::Any], ReturnType::Any, "Return first argument, ignore second", BackendImpl::CompilerSpecial));
        builtins.insert("noop", BuiltinSignature::fixed("noop", vec![], ReturnType::Null, "No operation", BackendImpl::CompilerSpecial));
        builtins.insert("assert", BuiltinSignature::fixed("assert", vec![ParamType::Bool, ParamType::String], ReturnType::Null, "Assert condition or panic", BackendImpl::RuntimeCall));
        builtins.insert("panic", BuiltinSignature::fixed("panic", vec![ParamType::String], ReturnType::Null, "Panic with message", BackendImpl::RuntimeCall));
        builtins.insert("debug", BuiltinSignature::variadic("debug", 1, ReturnType::Null, "Debug print with location", BackendImpl::RuntimeCall));
        builtins.insert("measure_time", BuiltinSignature::fixed("measure_time", vec![ParamType::String], ReturnType::Int, "Measure function execution time (microseconds)", BackendImpl::CompilerSpecial));
        builtins.insert("typeof_detailed", BuiltinSignature::fixed("typeof_detailed", vec![ParamType::Any], ReturnType::String, "Get detailed type information", BackendImpl::CompilerSpecial));

        Self { builtins }
    }

    /// Get signature for a builtin function
    pub fn get(&self, name: &str) -> Option<&BuiltinSignature> {
        self.builtins.get(name)
    }

    /// Check if a function is a builtin
    pub fn exists(&self, name: &str) -> bool {
        self.builtins.contains_key(name)
    }

    /// Get all builtin signatures
    pub fn all(&self) -> impl Iterator<Item = &BuiltinSignature> {
        self.builtins.values()
    }

    /// Get all builtin names
    pub fn names(&self) -> impl Iterator<Item = &&'static str> {
        self.builtins.keys()
    }

    /// Validate argument count for a builtin call
    pub fn validate_args(&self, name: &str, arg_count: usize) -> Result<(), String> {
        if let Some(sig) = self.get(name) {
            sig.validate_arg_count(arg_count)
        } else {
            Err(format!("Unknown builtin function: {}", name))
        }
    }
}

impl Default for BuiltinRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_exists() {
        let registry = BuiltinRegistry::new();
        assert!(registry.exists("print"));
        assert!(registry.exists("len"));
        assert!(registry.exists("sqrt"));
        assert!(registry.exists("str_get"));
        assert!(!registry.exists("nonexistent_function"));
    }

    #[test]
    fn test_validate_args() {
        let registry = BuiltinRegistry::new();

        // len takes exactly 1 arg
        assert!(registry.validate_args("len", 1).is_ok());
        assert!(registry.validate_args("len", 0).is_err());
        assert!(registry.validate_args("len", 2).is_err());

        // print is variadic (0+)
        assert!(registry.validate_args("print", 0).is_ok());
        assert!(registry.validate_args("print", 1).is_ok());
        assert!(registry.validate_args("print", 100).is_ok());

        // str_get takes exactly 2 args
        assert!(registry.validate_args("str_get", 2).is_ok());
        assert!(registry.validate_args("str_get", 1).is_err());
        assert!(registry.validate_args("str_get", 3).is_err());
    }

    #[test]
    fn test_get_signature() {
        let registry = BuiltinRegistry::new();

        let sig = registry.get("sqrt").unwrap();
        assert_eq!(sig.name, "sqrt");
        assert_eq!(sig.min_args, 1);
        assert_eq!(sig.max_args, Some(1));
        assert_eq!(sig.return_type, ReturnType::Float);
        assert_eq!(sig.backend_impl, BackendImpl::LLVMIntrinsic);
    }

    #[test]
    fn test_param_types() {
        let registry = BuiltinRegistry::new();

        let sig = registry.get("str_get").unwrap();
        assert_eq!(sig.param_type(0), Some(&ParamType::String));
        assert_eq!(sig.param_type(1), Some(&ParamType::Int));
        assert_eq!(sig.param_type(2), None);
    }

    #[test]
    fn test_backend_categories() {
        let registry = BuiltinRegistry::new();

        assert_eq!(
            registry.get("sqrt").unwrap().backend_impl,
            BackendImpl::LLVMIntrinsic
        );
        assert_eq!(
            registry.get("len").unwrap().backend_impl,
            BackendImpl::CompilerSpecial
        );
        assert_eq!(
            registry.get("read_file").unwrap().backend_impl,
            BackendImpl::RuntimeCall
        );
    }
}
