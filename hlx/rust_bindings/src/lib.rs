//! Rust bindings for test_ffi_lib HLX library
//!
//! Auto-generated FFI bindings.
//!
//! ## Usage
//!
//! Add to your `Cargo.toml`:
//! ```toml
//! [dependencies]
//! test_ffi_lib = { path = "." }
//! ```

#[allow(non_camel_case_types)]
mod ffi {
    use std::os::raw::*;

    #[link(name = "hlx_test_ffi")]
    extern "C" {
    pub fn add(arg0: i64, arg1: i64) -> i64;
    pub fn multiply(arg0: i64, arg1: i64) -> i64;
    }
}

/// Call HLX function: add
///
/// # Safety
/// This function is safe to call as it wraps a pure HLX function.
pub fn add(arg0: i64, arg1: i64) -> i64 {
    unsafe {
        ffi::add(arg0, arg1)
    }
}

/// Call HLX function: multiply
///
/// # Safety
/// This function is safe to call as it wraps a pure HLX function.
pub fn multiply(arg0: i64, arg1: i64) -> i64 {
    unsafe {
        ffi::multiply(arg0, arg1)
    }
}

