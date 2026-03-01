//! C FFI — SQLite-style embedding API
//!
//! Exposes Axiom as a C-compatible shared/static library so any language
//! can embed it: Python (ctypes/cffi), Go (cgo), Node.js (ffi-napi),
//! Ruby, Swift, Zig, etc.
//!
//! The design mirrors SQLite: open → verify → close.
//!
//! # C example
//! ```c
//! #include "axiom.h"
//!
//! axiom_engine_t *eng = axiom_engine_open("security.axm");
//! if (!eng) { fprintf(stderr, "open failed\n"); return 1; }
//!
//! const char *keys[] = { "path" };
//! const char *vals[] = { "/tmp/out.txt" };
//! int rc = axiom_verify(eng, "WriteFile", keys, vals, 1);
//! if (rc == 1)      puts("allowed");
//! else if (rc == 0) printf("denied: %s\n", axiom_denied_reason(eng));
//! else              printf("error: %s\n",  axiom_errmsg(eng));
//!
//! axiom_engine_close(eng);
//! ```

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::ptr;

use crate::AxiomEngine;

/// Internal handle — the pointer the C caller holds.
pub struct AxiomHandle {
    engine: Option<AxiomEngine>,
    last_error: CString,
    last_reason: CString,
}

impl AxiomHandle {
    fn ok(engine: AxiomEngine) -> Self {
        AxiomHandle {
            engine: Some(engine),
            last_error: CString::new("").unwrap(),
            last_reason: CString::new("").unwrap(),
        }
    }

    fn with_error(msg: &str) -> Self {
        AxiomHandle {
            engine: None,
            last_error: CString::new(sanitize(msg)).unwrap_or_default(),
            last_reason: CString::new("").unwrap(),
        }
    }

    fn set_error(&mut self, msg: &str) {
        self.last_error = CString::new(sanitize(msg)).unwrap_or_default();
        self.last_reason = CString::new("").unwrap();
    }

    fn set_denied(&mut self, reason: &str) {
        self.last_error = CString::new("").unwrap();
        self.last_reason = CString::new(sanitize(reason)).unwrap_or_default();
    }
}

/// Strip interior NUL bytes so CString::new never panics.
fn sanitize(s: &str) -> String {
    s.replace('\0', "\\0")
}

// ─── Open / Close ────────────────────────────────────────────────────────────

/// Open an Axiom engine from a `.axm` policy file.
///
/// Returns a heap-allocated handle, or NULL on error (check `axiom_errmsg`).
/// The caller must eventually call `axiom_engine_close`.
#[no_mangle]
pub extern "C" fn axiom_engine_open(path: *const c_char) -> *mut AxiomHandle {
    if path.is_null() {
        return ptr::null_mut();
    }
    let path_str = unsafe {
        match CStr::from_ptr(path).to_str() {
            Ok(s) => s,
            Err(_) => return Box::into_raw(Box::new(AxiomHandle::with_error("path is not valid UTF-8"))),
        }
    };
    let handle = match AxiomEngine::from_file(path_str) {
        Ok(e) => AxiomHandle::ok(e),
        Err(e) => AxiomHandle::with_error(&e.to_string()),
    };
    Box::into_raw(Box::new(handle))
}

/// Open an Axiom engine from an in-memory policy source string.
///
/// Returns a heap-allocated handle, or NULL on error (check `axiom_errmsg`).
/// The caller must eventually call `axiom_engine_close`.
#[no_mangle]
pub extern "C" fn axiom_engine_open_source(source: *const c_char) -> *mut AxiomHandle {
    if source.is_null() {
        return ptr::null_mut();
    }
    let src = unsafe {
        match CStr::from_ptr(source).to_str() {
            Ok(s) => s,
            Err(_) => return Box::into_raw(Box::new(AxiomHandle::with_error("source is not valid UTF-8"))),
        }
    };
    let handle = match AxiomEngine::from_source(src) {
        Ok(e) => AxiomHandle::ok(e),
        Err(e) => AxiomHandle::with_error(&e.to_string()),
    };
    Box::into_raw(Box::new(handle))
}

/// Close and free an Axiom engine handle.
///
/// Safe to call with NULL.
#[no_mangle]
pub extern "C" fn axiom_engine_close(handle: *mut AxiomHandle) {
    if !handle.is_null() {
        unsafe { drop(Box::from_raw(handle)) };
    }
}

// ─── Verification ─────────────────────────────────────────────────────────────

/// Verify an intent against the loaded policy.
///
/// Parameters
/// ----------
/// handle  — engine handle from `axiom_engine_open`
/// intent  — intent name, e.g. `"WriteFile"`
/// keys    — array of `n` NUL-terminated key strings  (may be NULL if n == 0)
/// values  — array of `n` NUL-terminated value strings (may be NULL if n == 0)
/// n       — number of key-value field pairs
///
/// Returns
/// -------
///  1  allowed
///  0  denied  (call `axiom_denied_reason` for the policy guidance)
/// -1  error   (call `axiom_errmsg` for the error message)
#[no_mangle]
pub extern "C" fn axiom_verify(
    handle: *mut AxiomHandle,
    intent: *const c_char,
    keys: *const *const c_char,
    values: *const *const c_char,
    n: c_int,
) -> c_int {
    if handle.is_null() || intent.is_null() {
        return -1;
    }
    let h = unsafe { &mut *handle };

    let engine = match &h.engine {
        Some(e) => e,
        None => {
            h.set_error("engine not initialized (open failed)");
            return -1;
        }
    };

    let intent_str = unsafe {
        match CStr::from_ptr(intent).to_str() {
            Ok(s) => s,
            Err(_) => {
                h.set_error("intent name is not valid UTF-8");
                return -1;
            }
        }
    };

    // Build field pairs
    let mut owned: Vec<(String, String)> = Vec::with_capacity(n.max(0) as usize);
    if n > 0 && !keys.is_null() && !values.is_null() {
        for i in 0..n as usize {
            let k = unsafe {
                let ptr = *keys.add(i);
                if ptr.is_null() { break; }
                match CStr::from_ptr(ptr).to_str() {
                    Ok(s) => s.to_owned(),
                    Err(_) => { h.set_error("field key is not valid UTF-8"); return -1; }
                }
            };
            let v = unsafe {
                let ptr = *values.add(i);
                if ptr.is_null() { break; }
                match CStr::from_ptr(ptr).to_str() {
                    Ok(s) => s.to_owned(),
                    Err(_) => { h.set_error("field value is not valid UTF-8"); return -1; }
                }
            };
            owned.push((k, v));
        }
    }

    let fields: Vec<(&str, &str)> = owned.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();

    match engine.verify(intent_str, &fields) {
        Ok(verdict) => {
            if verdict.allowed() {
                h.last_error = CString::new("").unwrap();
                h.last_reason = CString::new("").unwrap();
                1
            } else {
                h.set_denied(verdict.reason().unwrap_or(verdict.guidance()));
                0
            }
        }
        Err(e) => {
            h.set_error(&e.to_string());
            -1
        }
    }
}

// ─── Diagnostics ─────────────────────────────────────────────────────────────

/// Return the last error message (empty string if no error).
///
/// The pointer is valid until the next `axiom_*` call on this handle.
#[no_mangle]
pub extern "C" fn axiom_errmsg(handle: *const AxiomHandle) -> *const c_char {
    if handle.is_null() {
        return b"\0".as_ptr() as *const c_char;
    }
    unsafe { (*handle).last_error.as_ptr() }
}

/// Return the denial reason from the last `axiom_verify` call (empty if allowed).
///
/// The pointer is valid until the next `axiom_*` call on this handle.
#[no_mangle]
pub extern "C" fn axiom_denied_reason(handle: *const AxiomHandle) -> *const c_char {
    if handle.is_null() {
        return b"\0".as_ptr() as *const c_char;
    }
    unsafe { (*handle).last_reason.as_ptr() }
}

// ─── Metadata ────────────────────────────────────────────────────────────────

/// Return the Axiom library version as a NUL-terminated C string.
#[no_mangle]
pub extern "C" fn axiom_version() -> *const c_char {
    // env!() is evaluated at compile time; the trailing \0 makes it C-safe.
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr() as *const c_char
}
