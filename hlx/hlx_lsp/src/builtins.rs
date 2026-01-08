//! HLX Runtime Builtins Registry
//!
//! Comprehensive list of all runtime-provided builtin functions.
//! Used by the LSP to validate function calls before runtime.

use std::collections::HashMap;

/// Builtin function signature
#[derive(Debug, Clone)]
pub struct BuiltinSignature {
    pub name: &'static str,
    pub min_args: usize,
    pub max_args: Option<usize>, // None = variadic
    pub description: &'static str,
    pub return_type: &'static str,
}

impl BuiltinSignature {
    pub fn fixed(name: &'static str, args: usize, desc: &'static str, ret: &'static str) -> Self {
        Self {
            name,
            min_args: args,
            max_args: Some(args),
            description: desc,
            return_type: ret,
        }
    }

    pub fn variadic(name: &'static str, min: usize, desc: &'static str, ret: &'static str) -> Self {
        Self {
            name,
            min_args: min,
            max_args: None,
            description: desc,
            return_type: ret,
        }
    }

    pub fn range(name: &'static str, min: usize, max: usize, desc: &'static str, ret: &'static str) -> Self {
        Self {
            name,
            min_args: min,
            max_args: Some(max),
            description: desc,
            return_type: ret,
        }
    }
}

/// Registry of all HLX runtime builtins
pub struct BuiltinRegistry {
    builtins: HashMap<&'static str, BuiltinSignature>,
}

impl BuiltinRegistry {
    pub fn new() -> Self {
        let mut builtins = HashMap::new();

        // I/O Functions
        builtins.insert("print", BuiltinSignature::variadic("print", 0, "Print values to stdout", "Null"));
        builtins.insert("read_file", BuiltinSignature::fixed("read_file", 1, "Read file contents as string", "String"));
        builtins.insert("write_file", BuiltinSignature::fixed("write_file", 2, "Write string to file (path, content)", "Null"));
        builtins.insert("file_exists", BuiltinSignature::fixed("file_exists", 1, "Check if file exists", "Bool"));
        builtins.insert("delete_file", BuiltinSignature::fixed("delete_file", 1, "Delete a file", "Null"));
        builtins.insert("list_files", BuiltinSignature::fixed("list_files", 1, "List files in directory", "Array<String>"));
        builtins.insert("create_dir", BuiltinSignature::fixed("create_dir", 1, "Create directory", "Null"));

        // Type Introspection
        builtins.insert("type", BuiltinSignature::fixed("type", 1, "Get type name of value", "String"));
        builtins.insert("len", BuiltinSignature::fixed("len", 1, "Get length of array/string", "Int"));

        // Array Operations
        builtins.insert("slice", BuiltinSignature::range("slice", 2, 3, "Slice array (arr, start, end?)", "Array"));
        builtins.insert("append", BuiltinSignature::fixed("append", 2, "Append element to array", "Array"));
        builtins.insert("arr_pop", BuiltinSignature::fixed("arr_pop", 1, "Remove and return last element", "Any"));
        builtins.insert("arr_slice", BuiltinSignature::fixed("arr_slice", 3, "Slice array (arr, start, end)", "Array"));
        builtins.insert("arr_concat", BuiltinSignature::fixed("arr_concat", 2, "Concatenate two arrays", "Array"));

        // String Operations
        builtins.insert("concat", BuiltinSignature::variadic("concat", 2, "Concatenate strings", "String"));
        builtins.insert("strlen", BuiltinSignature::fixed("strlen", 1, "Get string length", "Int"));
        builtins.insert("substring", BuiltinSignature::fixed("substring", 3, "Get substring (str, start, end)", "String"));
        builtins.insert("index_of", BuiltinSignature::fixed("index_of", 2, "Find substring index", "Int"));
        builtins.insert("to_upper", BuiltinSignature::fixed("to_upper", 1, "Convert to uppercase", "String"));
        builtins.insert("to_lower", BuiltinSignature::fixed("to_lower", 1, "Convert to lowercase", "String"));
        builtins.insert("trim", BuiltinSignature::fixed("trim", 1, "Trim whitespace", "String"));
        builtins.insert("starts_with", BuiltinSignature::fixed("starts_with", 2, "Check if string starts with prefix", "Bool"));
        builtins.insert("ends_with", BuiltinSignature::fixed("ends_with", 2, "Check if string ends with suffix", "Bool"));

        // Type Conversions
        builtins.insert("to_string", BuiltinSignature::fixed("to_string", 1, "Convert value to string", "String"));
        builtins.insert("to_int", BuiltinSignature::fixed("to_int", 1, "Convert value to integer", "Int"));
        builtins.insert("parse_int", BuiltinSignature::fixed("parse_int", 1, "Parse string as integer", "Int"));
        builtins.insert("ord", BuiltinSignature::fixed("ord", 1, "Get ASCII code of first character", "Int"));

        // Math Functions
        builtins.insert("floor", BuiltinSignature::fixed("floor", 1, "Round down to nearest integer", "Float"));
        builtins.insert("ceil", BuiltinSignature::fixed("ceil", 1, "Round up to nearest integer", "Float"));
        builtins.insert("round", BuiltinSignature::fixed("round", 1, "Round to nearest integer", "Float"));
        builtins.insert("sqrt", BuiltinSignature::fixed("sqrt", 1, "Square root", "Float"));
        builtins.insert("sin", BuiltinSignature::fixed("sin", 1, "Sine (radians)", "Float"));
        builtins.insert("cos", BuiltinSignature::fixed("cos", 1, "Cosine (radians)", "Float"));
        builtins.insert("tan", BuiltinSignature::fixed("tan", 1, "Tangent (radians)", "Float"));
        builtins.insert("log", BuiltinSignature::fixed("log", 1, "Natural logarithm", "Float"));
        builtins.insert("exp", BuiltinSignature::fixed("exp", 1, "Exponential (e^x)", "Float"));
        builtins.insert("random", BuiltinSignature::fixed("random", 0, "Random float [0, 1)", "Float"));

        // Object Operations
        builtins.insert("has_key", BuiltinSignature::fixed("has_key", 2, "Check if object has key", "Bool"));

        // JSON Operations
        builtins.insert("json_parse", BuiltinSignature::fixed("json_parse", 1, "Parse JSON string", "Any"));
        builtins.insert("json_stringify", BuiltinSignature::fixed("json_stringify", 1, "Convert value to JSON string", "String"));
        builtins.insert("read_json", BuiltinSignature::fixed("read_json", 1, "Read and parse JSON file", "Any"));
        builtins.insert("write_json", BuiltinSignature::fixed("write_json", 2, "Write value as JSON file", "Null"));

        // HTTP Operations
        builtins.insert("http_request", BuiltinSignature::fixed("http_request", 1, "Make HTTP request (config object)", "Object"));

        // Runtime Introspection
        builtins.insert("snapshot", BuiltinSignature::fixed("snapshot", 0, "Create VM state snapshot", "Object"));
        builtins.insert("export_trace", BuiltinSignature::fixed("export_trace", 0, "Export execution trace", "Array<Object>"));
        builtins.insert("write_snapshot", BuiltinSignature::fixed("write_snapshot", 1, "Write snapshot to file", "Null"));

        // Process Management
        builtins.insert("pipe_open", BuiltinSignature::fixed("pipe_open", 1, "Open subprocess pipe", "Handle"));
        builtins.insert("pipe_write", BuiltinSignature::fixed("pipe_write", 2, "Write to subprocess pipe", "Null"));
        builtins.insert("pipe_close", BuiltinSignature::fixed("pipe_close", 1, "Close subprocess pipe", "Null"));

        // System Operations
        builtins.insert("sleep", BuiltinSignature::fixed("sleep", 1, "Sleep for milliseconds", "Null"));
        builtins.insert("capture_screen", BuiltinSignature::fixed("capture_screen", 1, "Capture screenshot to file", "Null"));

        // Tokenization (for metaprogramming)
        builtins.insert("native_tokenize", BuiltinSignature::fixed("native_tokenize", 1, "Tokenize HLX source code", "Array<Object>"));

        Self { builtins }
    }

    pub fn get(&self, name: &str) -> Option<&BuiltinSignature> {
        self.builtins.get(name)
    }

    pub fn exists(&self, name: &str) -> bool {
        self.builtins.contains_key(name)
    }

    pub fn all(&self) -> impl Iterator<Item = &BuiltinSignature> {
        self.builtins.values()
    }

    /// Check if argument count is valid for builtin
    pub fn validate_args(&self, name: &str, arg_count: usize) -> Result<(), String> {
        if let Some(sig) = self.get(name) {
            if arg_count < sig.min_args {
                return Err(format!(
                    "{}() requires at least {} argument(s), got {}",
                    name, sig.min_args, arg_count
                ));
            }
            if let Some(max) = sig.max_args {
                if arg_count > max {
                    return Err(format!(
                        "{}() accepts at most {} argument(s), got {}",
                        name, max, arg_count
                    ));
                }
            }
            Ok(())
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
        assert!(registry.exists("to_string"));
        assert!(registry.exists("export_trace"));
        assert!(!registry.exists("nonexistent_function"));
    }

    #[test]
    fn test_validate_args() {
        let registry = BuiltinRegistry::new();

        // to_string takes exactly 1 arg
        assert!(registry.validate_args("to_string", 1).is_ok());
        assert!(registry.validate_args("to_string", 0).is_err());
        assert!(registry.validate_args("to_string", 2).is_err());

        // print is variadic (0+)
        assert!(registry.validate_args("print", 0).is_ok());
        assert!(registry.validate_args("print", 1).is_ok());
        assert!(registry.validate_args("print", 100).is_ok());

        // write_file takes exactly 2 args
        assert!(registry.validate_args("write_file", 2).is_ok());
        assert!(registry.validate_args("write_file", 1).is_err());
        assert!(registry.validate_args("write_file", 3).is_err());
    }

    #[test]
    fn test_get_signature() {
        let registry = BuiltinRegistry::new();

        let sig = registry.get("to_string").unwrap();
        assert_eq!(sig.name, "to_string");
        assert_eq!(sig.min_args, 1);
        assert_eq!(sig.max_args, Some(1));
        assert_eq!(sig.return_type, "String");
    }
}
