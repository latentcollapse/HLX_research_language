//! Module system integration tests

use hlx_compiler::ModuleResolver;

#[test]
fn test_stdlib_module_resolution() {
    let mut resolver = ModuleResolver::new();

    // Resolve stdlib math module
    let program = resolver.resolve("std.math").expect("Failed to resolve std.math");

    // Should have parsed successfully
    assert!(!program.modules.is_empty());
}

#[test]
fn test_stdlib_array_module() {
    let mut resolver = ModuleResolver::new();

    // Resolve stdlib array module
    let program = resolver.resolve("std.array").expect("Failed to resolve std.array");
    assert!(!program.modules.is_empty());
}

#[test]
fn test_stdlib_string_module() {
    let mut resolver = ModuleResolver::new();

    // Resolve stdlib string module
    let program = resolver.resolve("std.string").expect("Failed to resolve std.string");
    assert!(!program.modules.is_empty());
}

#[test]
fn test_module_not_found() {
    let mut resolver = ModuleResolver::new();

    // Should fail to resolve non-existent module
    let result = resolver.resolve("nonexistent.module");
    assert!(result.is_err());
}

#[test]
fn test_hlx_path_env() {
    // This test verifies HLX_PATH is respected
    // Note: In real usage, set HLX_PATH environment variable
    let resolver = ModuleResolver::new();

    // Resolver should have default search paths
    // This is a smoke test that construction works
    assert!(!resolver.search_paths.is_empty());
}
