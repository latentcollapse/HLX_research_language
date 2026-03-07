use hlx::*;
use std::ffi::{CStr, CString};

#[test]
fn test_ffi_smoke() {
    unsafe {
        let h = hlx_open();
        assert!(!h.is_null());

        let source = CString::new("fn add(a: i64, b: i64) -> i64 { return a + b; }").unwrap();
        let res = hlx_compile_source(h, source.as_ptr());
        assert_eq!(res, 1, "Compilation failed: {}", CStr::from_ptr(hlx_errmsg(h)).to_str().unwrap());

        let func = CString::new("add").unwrap();
        let args = CString::new("[{\"type\":\"I64\",\"value\":21},{\"type\":\"I64\",\"value\":21}]").unwrap();
        
        let result_ptr = hlx_call(h, func.as_ptr(), args.as_ptr());
        assert!(!result_ptr.is_null(), "hlx_call failed: {}", CStr::from_ptr(hlx_errmsg(h)).to_str().unwrap());

        let result_str = CStr::from_ptr(result_ptr).to_str().unwrap();
        assert!(result_str.contains("\"type\":\"I64\""));
        assert!(result_str.contains("\"value\":42"));

        hlx_free_string(result_ptr);
        hlx_close(h);
    }
}

#[test]
fn test_ffi_error() {
    unsafe {
        let h = hlx_open();
        let source = CString::new("fn bad { oops }").unwrap();
        let res = hlx_compile_source(h, source.as_ptr());
        assert_eq!(res, 0);
        
        let err_ptr = hlx_errmsg(h);
        let err_str = CStr::from_ptr(err_ptr).to_str().unwrap();
        assert!(!err_str.is_empty());
        assert!(err_str.contains("parse error"));
        
        hlx_close(h);
    }
}
