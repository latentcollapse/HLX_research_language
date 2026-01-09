use hlx_backend_llvm::CodeGen;
use inkwell::context::Context;

#[test]
fn test_invalid_target_triple() {
    let context = Context::create();
    let result = CodeGen::with_target(&context, "test", Some("invalid-triple-xyz"));
    assert!(result.is_err(), "Should return error for invalid triple");
    if let Err(e) = result {
        let err_msg = e.to_string();
        assert!(err_msg.contains("Target") || err_msg.contains("triple"),
                "Error should mention target/triple: {}", err_msg);
    }
}

#[test]
fn test_valid_backend_creation() {
    let context = Context::create();
    let result = CodeGen::new(&context, "test");
    assert!(result.is_ok(), "Should create backend successfully");
}

#[test]
fn test_missing_function_lookup() {
    let context = Context::create();
    let codegen = CodeGen::new(&context, "test").expect("Should create backend");

    // Try to get a function that doesn't exist
    let result = codegen.get_function("nonexistent_function_xyz");
    assert!(result.is_err(), "Should return error for missing function");
    if let Err(e) = result {
        let err_msg = e.to_string();
        assert!(err_msg.contains("nonexistent_function_xyz"),
                "Error should mention function name: {}", err_msg);
    }
}

#[test]
fn test_missing_block_lookup() {
    let context = Context::create();
    let codegen = CodeGen::new(&context, "test").expect("Should create backend");

    // Try to get a block that doesn't exist
    let result = codegen.get_block(99999);
    assert!(result.is_err(), "Should return error for missing block");
    if let Err(e) = result {
        let err_msg = e.to_string();
        assert!(err_msg.contains("99999") || err_msg.contains("block"),
                "Error should mention block or PC: {}", err_msg);
    }
}
