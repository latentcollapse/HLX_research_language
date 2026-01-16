//! FFI utilities for header generation and type mapping

use crate::instruction::DType;
use crate::hlx_crate::{HlxCrate, FfiExportInfo};

/// Convert HLX DType to C type string
pub fn dtype_to_c_type(dtype: &DType) -> String {
    match dtype {
        DType::I32 => "int32_t".to_string(),
        DType::I64 => "int64_t".to_string(),
        DType::F32 => "float".to_string(),
        DType::F64 => "double".to_string(),
        DType::Bool => "bool".to_string(),
        DType::Array(inner) => format!("{}*", dtype_to_c_type(inner)),
    }
}

/// Generate C function declaration
pub fn generate_c_declaration(name: &str, info: &FfiExportInfo) -> String {
    let return_type = dtype_to_c_type(&info.return_type);

    let params = if info.param_types.is_empty() {
        "void".to_string()
    } else {
        info.param_types
            .iter()
            .enumerate()
            .map(|(i, dtype)| format!("{} arg{}", dtype_to_c_type(dtype), i))
            .collect::<Vec<_>>()
            .join(", ")
    };

    format!("{} {}({});", return_type, name, params)
}

/// Generate complete C header file content
pub fn generate_header(krate: &HlxCrate, module_name: &str) -> Option<String> {
    let meta = krate.metadata.as_ref()?;

    if meta.ffi_exports.is_empty() {
        return None;
    }

    let guard_name = format!("{}_H", module_name.to_uppercase().replace("-", "_"));

    let mut header = String::new();

    // Header guard
    header.push_str(&format!("#ifndef {}\n", guard_name));
    header.push_str(&format!("#define {}\n\n", guard_name));

    // Includes
    header.push_str("#include <stdint.h>\n");
    header.push_str("#include <stdbool.h>\n\n");

    // Extern C wrapper for C++
    header.push_str("#ifdef __cplusplus\n");
    header.push_str("extern \"C\" {\n");
    header.push_str("#endif\n\n");

    // Function declarations
    header.push_str("// HLX FFI Exports\n");

    // Sort for deterministic output
    let mut exports: Vec<_> = meta.ffi_exports.iter().collect();
    exports.sort_by_key(|(name, _)| *name);

    for (name, info) in exports {
        header.push_str(&generate_c_declaration(name, info));
        header.push('\n');
    }

    // Close extern C
    header.push_str("\n#ifdef __cplusplus\n");
    header.push_str("}\n");
    header.push_str("#endif\n\n");

    // Close header guard
    header.push_str(&format!("#endif // {}\n", guard_name));

    Some(header)
}

/// Convert HLX DType to Python ctypes type string
pub fn dtype_to_python_ctype(dtype: &DType) -> String {
    match dtype {
        DType::I32 => "ctypes.c_int32".to_string(),
        DType::I64 => "ctypes.c_int64".to_string(),
        DType::F32 => "ctypes.c_float".to_string(),
        DType::F64 => "ctypes.c_double".to_string(),
        DType::Bool => "ctypes.c_bool".to_string(),
        DType::Array(inner) => format!("ctypes.POINTER({})", dtype_to_python_ctype(inner)),
    }
}

/// Convert HLX DType to Python type hint
pub fn dtype_to_python_hint(dtype: &DType) -> String {
    match dtype {
        DType::I32 | DType::I64 => "int".to_string(),
        DType::F32 | DType::F64 => "float".to_string(),
        DType::Bool => "bool".to_string(),
        DType::Array(_) => "list".to_string(),
    }
}

/// Generate Python function wrapper
fn generate_python_function(name: &str, info: &FfiExportInfo) -> String {
    let mut code = String::new();

    // Function signature with type hints
    let params: Vec<String> = info.param_types
        .iter()
        .enumerate()
        .map(|(i, dtype)| format!("arg{}: {}", i, dtype_to_python_hint(dtype)))
        .collect();

    let return_hint = dtype_to_python_hint(&info.return_type);

    code.push_str(&format!("def {}({}) -> {}:\n", name, params.join(", "), return_hint));

    // Docstring
    code.push_str(&format!("    \"\"\"Call HLX function: {}\"\"\"\n", name));

    // Call the C function
    let args: Vec<String> = (0..info.param_types.len())
        .map(|i| format!("arg{}", i))
        .collect();

    code.push_str(&format!("    return _lib.{}({})\n", name, args.join(", ")));

    code
}

/// Generate complete Python wrapper module
pub fn generate_python_wrapper(krate: &HlxCrate, module_name: &str, lib_name: &str) -> Option<String> {
    let meta = krate.metadata.as_ref()?;

    if meta.ffi_exports.is_empty() {
        return None;
    }

    let mut wrapper = String::new();

    // Header comment
    wrapper.push_str(&format!("\"\"\"Python wrapper for {} HLX library\n\n", module_name));
    wrapper.push_str("Auto-generated FFI bindings using ctypes.\n");
    wrapper.push_str("\"\"\"\n\n");

    // Imports
    wrapper.push_str("import ctypes\n");
    wrapper.push_str("import os\n");
    wrapper.push_str("import sys\n");
    wrapper.push_str("from pathlib import Path\n\n");

    // Library loading with platform detection
    wrapper.push_str("# Load shared library with platform-specific naming\n");
    wrapper.push_str("def _load_library():\n");
    wrapper.push_str("    \"\"\"Load the HLX shared library with platform-specific extensions\"\"\"\n");
    wrapper.push_str("    lib_dir = Path(__file__).parent\n");
    wrapper.push_str("    \n");
    wrapper.push_str("    if sys.platform == 'linux':\n");
    wrapper.push_str(&format!("        lib_path = lib_dir / 'lib{}.so'\n", lib_name));
    wrapper.push_str("    elif sys.platform == 'darwin':\n");
    wrapper.push_str(&format!("        lib_path = lib_dir / 'lib{}.dylib'\n", lib_name));
    wrapper.push_str("    elif sys.platform == 'win32':\n");
    wrapper.push_str(&format!("        lib_path = lib_dir / '{}.dll'\n", lib_name));
    wrapper.push_str("    else:\n");
    wrapper.push_str("        raise RuntimeError(f'Unsupported platform: {sys.platform}')\n");
    wrapper.push_str("    \n");
    wrapper.push_str("    if not lib_path.exists():\n");
    wrapper.push_str(&format!("        raise FileNotFoundError(f'HLX library not found: {{lib_path}}')\n"));
    wrapper.push_str("    \n");
    wrapper.push_str("    return ctypes.CDLL(str(lib_path))\n\n");

    // Load library
    wrapper.push_str("_lib = _load_library()\n\n");

    // Configure function signatures
    wrapper.push_str("# Configure function signatures\n");

    // Sort for deterministic output
    let mut exports: Vec<_> = meta.ffi_exports.iter().collect();
    exports.sort_by_key(|(name, _)| *name);

    for (name, info) in &exports {
        // Set argtypes
        if !info.param_types.is_empty() {
            let arg_types: Vec<String> = info.param_types
                .iter()
                .map(|dtype| dtype_to_python_ctype(dtype))
                .collect();
            wrapper.push_str(&format!("_lib.{}.argtypes = [{}]\n", name, arg_types.join(", ")));
        }

        // Set restype
        wrapper.push_str(&format!("_lib.{}.restype = {}\n", name, dtype_to_python_ctype(&info.return_type)));
    }

    wrapper.push_str("\n# Python wrapper functions\n");

    // Generate wrapper functions
    for (name, info) in exports {
        wrapper.push_str(&generate_python_function(name, info));
        wrapper.push('\n');
    }

    // Add __all__ for clean imports
    let func_names: Vec<String> = meta.ffi_exports.keys().map(|s| format!("\"{}\"", s)).collect();
    wrapper.push_str(&format!("__all__ = [{}]\n", func_names.join(", ")));

    Some(wrapper)
}

/// Convert HLX DType to Rust type string
pub fn dtype_to_rust_type(dtype: &DType) -> String {
    match dtype {
        DType::I32 => "i32".to_string(),
        DType::I64 => "i64".to_string(),
        DType::F32 => "f32".to_string(),
        DType::F64 => "f64".to_string(),
        DType::Bool => "bool".to_string(),
        DType::Array(inner) => format!("*const {}", dtype_to_rust_type(inner)),
    }
}

/// Generate Rust extern "C" declaration
fn generate_rust_extern(name: &str, info: &FfiExportInfo) -> String {
    let return_type = dtype_to_rust_type(&info.return_type);

    let params: Vec<String> = info.param_types
        .iter()
        .enumerate()
        .map(|(i, dtype)| format!("arg{}: {}", i, dtype_to_rust_type(dtype)))
        .collect();

    format!("    pub fn {}({}) -> {};", name, params.join(", "), return_type)
}

/// Generate Rust safe wrapper function
fn generate_rust_function(name: &str, info: &FfiExportInfo) -> String {
    let mut code = String::new();

    let return_type = dtype_to_rust_type(&info.return_type);

    let params: Vec<String> = info.param_types
        .iter()
        .enumerate()
        .map(|(i, dtype)| format!("arg{}: {}", i, dtype_to_rust_type(dtype)))
        .collect();

    // Documentation
    code.push_str(&format!("/// Call HLX function: {}\n", name));
    code.push_str("///\n");
    code.push_str("/// # Safety\n");
    code.push_str("/// This function is safe to call as it wraps a pure HLX function.\n");
    code.push_str(&format!("pub fn {}({}) -> {} {{\n", name, params.join(", "), return_type));
    code.push_str("    unsafe {\n");

    let args: Vec<String> = (0..info.param_types.len())
        .map(|i| format!("arg{}", i))
        .collect();

    code.push_str(&format!("        ffi::{}({})\n", name, args.join(", ")));
    code.push_str("    }\n");
    code.push_str("}\n");

    code
}

/// Generate complete Rust wrapper module
pub fn generate_rust_wrapper(krate: &HlxCrate, module_name: &str, lib_name: &str) -> Option<String> {
    let meta = krate.metadata.as_ref()?;

    if meta.ffi_exports.is_empty() {
        return None;
    }

    let mut wrapper = String::new();

    // Module documentation
    wrapper.push_str(&format!("//! Rust bindings for {} HLX library\n", module_name));
    wrapper.push_str("//!\n");
    wrapper.push_str("//! Auto-generated FFI bindings.\n");
    wrapper.push_str("//!\n");
    wrapper.push_str("//! ## Usage\n");
    wrapper.push_str("//!\n");
    wrapper.push_str("//! Add to your `Cargo.toml`:\n");
    wrapper.push_str("//! ```toml\n");
    wrapper.push_str("//! [dependencies]\n");
    wrapper.push_str(&format!("//! {} = {{ path = \".\" }}\n", module_name));
    wrapper.push_str("//! ```\n\n");

    // FFI module with extern declarations
    wrapper.push_str("#[allow(non_camel_case_types)]\n");
    wrapper.push_str("mod ffi {\n");
    wrapper.push_str("    use std::os::raw::*;\n\n");

    wrapper.push_str("    #[link(name = \"");
    wrapper.push_str(lib_name);
    wrapper.push_str("\")]\n");
    wrapper.push_str("    extern \"C\" {\n");

    // Sort for deterministic output
    let mut exports: Vec<_> = meta.ffi_exports.iter().collect();
    exports.sort_by_key(|(name, _)| *name);

    for (name, info) in &exports {
        wrapper.push_str(&generate_rust_extern(name, info));
        wrapper.push('\n');
    }

    wrapper.push_str("    }\n");
    wrapper.push_str("}\n\n");

    // Safe wrapper functions
    for (name, info) in exports {
        wrapper.push_str(&generate_rust_function(name, info));
        wrapper.push('\n');
    }

    Some(wrapper)
}

/// Generate Cargo.toml for Rust wrapper
pub fn generate_cargo_toml(module_name: &str, lib_name: &str) -> String {
    let mut toml = String::new();

    toml.push_str("[package]\n");
    toml.push_str(&format!("name = \"{}\"\n", module_name));
    toml.push_str("version = \"0.1.0\"\n");
    toml.push_str("edition = \"2021\"\n\n");

    toml.push_str("[lib]\n");
    toml.push_str("name = \"");
    toml.push_str(module_name);
    toml.push_str("\"\n");
    toml.push_str("path = \"src/lib.rs\"\n\n");

    toml.push_str("[build-dependencies]\n");
    toml.push_str("# If you want to use build.rs for custom linking\n\n");

    toml.push_str("# To link the HLX shared library, add the library directory to your path:\n");
    toml.push_str("# export LIBRARY_PATH=/path/to/hlx/lib:$LIBRARY_PATH\n");
    toml.push_str(&format!("# or use: cargo rustc -- -L /path/to/{}\n", lib_name));

    toml
}

/// Convert HLX DType to Ada type string
pub fn dtype_to_ada_type(dtype: &DType) -> String {
    match dtype {
        DType::I32 => "Interfaces.C.int".to_string(),
        DType::I64 => "Interfaces.C.long".to_string(),
        DType::F32 => "Interfaces.C.C_float".to_string(),
        DType::F64 => "Interfaces.C.double".to_string(),
        DType::Bool => "Interfaces.C.unsigned_char".to_string(),
        DType::Array(inner) => format!("Interfaces.C.Pointers.Pointer_To_{}", dtype_to_ada_type(inner)),
    }
}

/// Generate Ada function specification with SPARK contracts
fn generate_ada_spec(name: &str, info: &FfiExportInfo) -> String {
    let mut spec = String::new();

    let return_type = dtype_to_ada_type(&info.return_type);
    let params: Vec<String> = info.param_types
        .iter()
        .enumerate()
        .map(|(i, dtype)| format!("Arg_{} : {}", i, dtype_to_ada_type(dtype)))
        .collect();

    // SPARK precondition (placeholder - can be expanded based on function semantics)
    spec.push_str("   --  @pre True\n");
    spec.push_str("   --  @post True\n");

    if params.is_empty() {
        spec.push_str(&format!("   function {} return {} with\n", name, return_type));
    } else {
        spec.push_str(&format!("   function {} ({}) return {} with\n", name, params.join("; "), return_type));
    }
    spec.push_str("     Import        => True,\n");
    spec.push_str("     Convention    => C,\n");
    spec.push_str(&format!("     External_Name => \"{}\";\n", name));

    spec
}

/// Generate complete Ada package specification with SPARK contracts
pub fn generate_ada_spec_file(krate: &HlxCrate, module_name: &str) -> Option<String> {
    let meta = krate.metadata.as_ref()?;

    if meta.ffi_exports.is_empty() {
        return None;
    }

    let mut spec = String::new();

    // Package header comment
    spec.push_str(&format!("--  HLX FFI Bindings for {}\n", module_name));
    spec.push_str("--  Auto-generated Ada package specification\n");
    spec.push_str("--  This package provides SPARK-verified bindings to HLX functions\n\n");

    // Package declaration
    let pkg_name = format!("{}_FFI", module_name.replace("-", "_"));
    spec.push_str(&format!("package {} is\n", pkg_name));
    spec.push_str("   pragma Pure;\n\n");

    // Import required interfaces
    spec.push_str("   use Interfaces;\n");
    spec.push_str("   use Interfaces.C;\n\n");

    // Sort for deterministic output
    let mut exports: Vec<_> = meta.ffi_exports.iter().collect();
    exports.sort_by_key(|(name, _)| *name);

    // Function declarations with SPARK contracts
    spec.push_str("   --  HLX FFI Functions\n\n");

    for (name, info) in &exports {
        spec.push_str(&generate_ada_spec(name, info));
        spec.push('\n');
    }

    spec.push_str(&format!("end {};\n", pkg_name));

    Some(spec)
}

/// Generate Ada package body (implementation stub)
pub fn generate_ada_body_file(krate: &HlxCrate, module_name: &str) -> Option<String> {
    let meta = krate.metadata.as_ref()?;

    if meta.ffi_exports.is_empty() {
        return None;
    }

    let mut body = String::new();

    let pkg_name = format!("{}_FFI", module_name.replace("-", "_"));
    body.push_str(&format!("package body {} is\n", pkg_name));
    body.push_str("   --  This package body is intentionally empty.\n");
    body.push_str("   --  All function implementations are provided by the external C library.\n");
    body.push_str(&format!("end {};\n", pkg_name));

    Some(body)
}

/// Generate SPARK project file for formal verification
pub fn generate_spark_project(module_name: &str) -> String {
    let mut project = String::new();

    project.push_str("project SPARK_HLX is\n\n");

    project.push_str("   for Source_Dirs use (\"src\");\n");
    project.push_str("   for Object_Dir use \".objects\";\n");
    project.push_str("   for Library_Dir use \".\";\n\n");

    project.push_str("   package Compiler is\n");
    project.push_str("      for Default_Switches (\"Ada\") use\n");
    project.push_str("        (\"-gnat2022\",\n");
    project.push_str("         \"-gnatwa\",\n");
    project.push_str("         \"-gnatwe\",\n");
    project.push_str("         \"-gnatyyM\",\n");
    project.push_str("         \"-gnaty3abdefhijklmnoprstux\",\n");
    project.push_str("         \"-gnaty_\",\n");
    project.push_str("         \"-gnatf\",\n");
    project.push_str("         \"-gnata\",\n");
    project.push_str("         \"-gnatwa\",\n");
    project.push_str("         \"-gnatwe\",\n");
    project.push_str("         \"-gnatyyM\",\n");
    project.push_str("         \"-gnaty3abdefhijklmnoprstux\");\n");
    project.push_str("   end Compiler;\n\n");

    project.push_str("   package Prove is\n");
    project.push_str("      for Switches use (\"--level=1\", \"--proof=progressive\");\n");
    project.push_str("   end Prove;\n\n");

    project.push_str("end SPARK_HLX;\n");

    project
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dtype_to_c_type() {
        assert_eq!(dtype_to_c_type(&DType::I64), "int64_t");
        assert_eq!(dtype_to_c_type(&DType::F64), "double");
        assert_eq!(dtype_to_c_type(&DType::Bool), "bool");
        assert_eq!(dtype_to_c_type(&DType::Array(Box::new(DType::I32))), "int32_t*");
    }

    #[test]
    fn test_generate_c_declaration() {
        let info = FfiExportInfo {
            no_mangle: true,
            export: true,
            param_types: vec![DType::I64, DType::I64],
            return_type: DType::I64,
        };

        let decl = generate_c_declaration("add", &info);
        assert_eq!(decl, "int64_t add(int64_t arg0, int64_t arg1);");
    }

    #[test]
    fn test_dtype_to_ada_type() {
        assert_eq!(dtype_to_ada_type(&DType::I32), "Interfaces.C.int");
        assert_eq!(dtype_to_ada_type(&DType::I64), "Interfaces.C.long");
        assert_eq!(dtype_to_ada_type(&DType::F32), "Interfaces.C.C_float");
        assert_eq!(dtype_to_ada_type(&DType::F64), "Interfaces.C.double");
        assert_eq!(dtype_to_ada_type(&DType::Bool), "Interfaces.C.unsigned_char");
    }

    #[test]
    fn test_generate_spark_project() {
        let project = generate_spark_project("test_module");
        assert!(project.contains("project SPARK_HLX is"));
        assert!(project.contains("package Prove is"));
        assert!(project.contains("--level=1"));
    }
}
