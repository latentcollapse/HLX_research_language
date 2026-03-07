use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;
use std::ptr;

use hlx_runtime::{AstParser, Bytecode, Lowerer, ModuleResolver, Value, Vm};

/// An opaque handle representing an HLX runtime instance.
pub struct HlxHandle {
    bytecode: Option<Bytecode>,
    functions: HashMap<String, (u32, u32)>,
    vm: Vm,
    search_path: Option<PathBuf>,
    last_error: CString,
}

impl HlxHandle {
    fn new() -> Self {
        HlxHandle {
            bytecode: None,
            functions: HashMap::new(),
            vm: Vm::new(),
            search_path: None,
            last_error: CString::new("").unwrap(),
        }
    }

    fn set_error(&mut self, msg: &str) {
        self.last_error =
            CString::new(sanitize(msg)).unwrap_or_else(|_| CString::new("unknown error").unwrap());
    }

    fn clear_error(&mut self) {
        self.last_error = CString::new("").unwrap();
    }

    fn compile(&mut self, source: &str) -> bool {
        self.bytecode = None;
        self.functions = HashMap::new();
        let program = match AstParser::parse(source) {
            Ok(p) => p,
            Err(e) => {
                self.set_error(&format!("parse error: {:?}", e));
                return false;
            }
        };
        let mut resolver = ModuleResolver::new();
        if let Some(ref path) = self.search_path {
            resolver.add_search_path(path);
        }
        let imported_functions = match resolver.resolve_program(&program) {
            Ok(f) => f,
            Err(e) => {
                self.set_error(&format!("import error: {}", e));
                return false;
            }
        };
        match Lowerer::lower_with_imports(&program, imported_functions) {
            Ok((bc, funcs)) => {
                self.bytecode = Some(bc);
                self.functions = funcs.clone();
                for (name, &(start_pc, params)) in &funcs {
                    self.vm
                        .register_function(name, start_pc as usize, params as usize);
                }
                // Reset execution state so the VM is ready for re-execution,
                // but preserve persistent state (memory, latent_states).
                self.vm.reset_execution_state();
                self.clear_error();
                true
            }
            Err(e) => {
                self.set_error(&format!("lower error: {:?}", e));
                false
            }
        }
    }
}

fn sanitize(s: &str) -> String {
    s.replace('\0', "\\0")
}

// ── Lifecycle ─────────────────────────────────────────────────────────────

/// Create a new HLX runtime instance.
///
/// Returns a handle that must be freed with `hlx_close`.
#[no_mangle]
pub extern "C" fn hlx_open() -> *mut HlxHandle {
    Box::into_raw(Box::new(HlxHandle::new()))
}

/// Free an HLX handle. Safe to call with NULL.
#[no_mangle]
pub extern "C" fn hlx_close(handle: *mut HlxHandle) {
    if !handle.is_null() {
        unsafe { drop(Box::from_raw(handle)) };
    }
}

/// Reset the VM to a fresh state, discarding all persistent memory.
///
/// Use this when you explicitly want a clean slate. Unlike recompilation
/// (which preserves memory), this wipes everything.
/// Returns 1 on success, 0 if handle is NULL.
#[no_mangle]
pub extern "C" fn hlx_reset(handle: *mut HlxHandle) -> c_int {
    if handle.is_null() {
        return 0;
    }
    let h = unsafe { &mut *handle };
    h.vm = Vm::new();
    // Re-register any compiled functions on the fresh VM
    for (name, &(start_pc, params)) in &h.functions {
        h.vm.register_function(name, start_pc as usize, params as usize);
    }
    h.clear_error();
    1
}

// ── Configuration ─────────────────────────────────────────────────────────────

/// Set the directory searched for `use` imports during compilation.
///
/// Call this before `hlx_compile_*`. Passing NULL clears any previously set path.
/// Returns 1 on success, 0 if the path is not valid UTF-8.
#[no_mangle]
pub extern "C" fn hlx_set_search_path(handle: *mut HlxHandle, path: *const c_char) -> c_int {
    if handle.is_null() {
        return 0;
    }
    let h = unsafe { &mut *handle };

    if path.is_null() {
        h.search_path = None;
        return 1;
    }

    let path_str = unsafe {
        match CStr::from_ptr(path).to_str() {
            Ok(s) => s,
            Err(_) => {
                h.set_error("search path is not valid UTF-8");
                return 0;
            }
        }
    };

    h.search_path = Some(PathBuf::from(path_str));
    1
}

// ── Compile ───────────────────────────────────────────────────────────────────

/// Compile HLX source code held in memory.
///
/// Returns 1 on success, 0 on error — call `hlx_errmsg` for details.
/// Must be called before `hlx_run` or `hlx_call`.
#[no_mangle]
pub extern "C" fn hlx_compile_source(handle: *mut HlxHandle, source: *const c_char) -> c_int {
    if handle.is_null() || source.is_null() {
        return 0;
    }
    let h = unsafe { &mut *handle };

    let src = unsafe {
        match CStr::from_ptr(source).to_str() {
            Ok(s) => s,
            Err(_) => {
                h.set_error("source is not valid UTF-8");
                return 0;
            }
        }
    };

    if h.compile(src) {
        1
    } else {
        0
    }
}

/// Compile HLX source from a file on disk.
///
/// Returns 1 on success, 0 on error — call `hlx_errmsg` for details.
#[no_mangle]
pub extern "C" fn hlx_compile_file(handle: *mut HlxHandle, path: *const c_char) -> c_int {
    if handle.is_null() || path.is_null() {
        return 0;
    }
    let h = unsafe { &mut *handle };

    let path_str = unsafe {
        match CStr::from_ptr(path).to_str() {
            Ok(s) => s,
            Err(_) => {
                h.set_error("path is not valid UTF-8");
                return 0;
            }
        }
    };

    // Automatically set the search path to the file's parent directory
    // so relative `use` imports work out of the box.
    let pb = PathBuf::from(path_str);
    if let Some(parent) = pb.parent() {
        h.search_path = Some(parent.to_path_buf());
    }

    let source = match std::fs::read_to_string(&pb) {
        Ok(s) => s,
        Err(e) => {
            h.set_error(&format!("read error: {e}"));
            return 0;
        }
    };

    if h.compile(&source) {
        1
    } else {
        0
    }
}

// ── Execute ───────────────────────────────────────────────────────────────────

/// Run the top-level code in the compiled program.
///
/// Returns a heap-allocated JSON string representing the final value, or NULL
/// on error. The caller must free the string with `hlx_free_string`.
///
/// Most programs have no meaningful top-level return value — use `hlx_call`
/// to invoke named functions instead.
#[no_mangle]
pub extern "C" fn hlx_run(handle: *mut HlxHandle) -> *mut c_char {
    if handle.is_null() {
        return ptr::null_mut();
    }
    let h = unsafe { &mut *handle };

    let bytecode = match &h.bytecode {
        Some(bc) => bc.clone(),
        None => {
            h.set_error("no compiled program — call hlx_compile_* first");
            return ptr::null_mut();
        }
    };

    // Use the persistent VM — no more Brain Wipe.
    h.vm.reset_execution_state();
    match h.vm.run(&bytecode) {
        Ok(val) => {
            h.clear_error();
            value_to_cstring(val, h)
        }
        Err(e) => {
            h.set_error(&format!("{}: {}", e.message, e.pc));
            ptr::null_mut()
        }
    }
}

/// Call a named function in the compiled program.
///
/// `func_name` — NUL-terminated name of the exported function.
/// `args_json`  — JSON array of `Value` objects, e.g.
///                `[{"type":"I64","value":42},{"type":"String","value":"hi"}]`
///                Pass NULL or an empty array `[]` for zero-argument functions.
///
/// Returns a heap-allocated JSON string of the return `Value`, or NULL on
/// error. The caller must free the string with `hlx_free_string`.
#[no_mangle]
pub extern "C" fn hlx_call(
    handle: *mut HlxHandle,
    func_name: *const c_char,
    args_json: *const c_char,
) -> *mut c_char {
    if handle.is_null() || func_name.is_null() {
        return ptr::null_mut();
    }
    let h = unsafe { &mut *handle };

    // Clean the Bleed: reset execution state between turns to prevent
    // stale call stacks, registers, and halted flags from corrupting the next call.
    h.vm.reset_execution_state();

    let bytecode = match &h.bytecode {
        Some(bc) => bc,
        None => {
            h.set_error("no compiled program — call hlx_compile_* first");
            return ptr::null_mut();
        }
    };

    let name = unsafe {
        match CStr::from_ptr(func_name).to_str() {
            Ok(s) => s,
            Err(_) => {
                h.set_error("func_name is not valid UTF-8");
                return ptr::null_mut();
            }
        }
    };

    // Parse JSON args (null pointer or empty → no args)
    let args: Vec<Value> = if args_json.is_null() {
        vec![]
    } else {
        let json_str = unsafe {
            match CStr::from_ptr(args_json).to_str() {
                Ok(s) => s,
                Err(_) => {
                    h.set_error("args_json is not valid UTF-8");
                    return ptr::null_mut();
                }
            }
        };
        if json_str.trim().is_empty() || json_str.trim() == "[]" {
            vec![]
        } else {
            match serde_json::from_str::<Vec<Value>>(json_str) {
                Ok(v) => v,
                Err(e) => {
                    h.set_error(&format!("args_json parse error: {e}"));
                    return ptr::null_mut();
                }
            }
        }
    };

    // Clone bytecode to satisfy borrow checker
    let bc = bytecode.clone();
    match h.vm.call_function(&bc, name, &args) {
        Ok(val) => {
            h.clear_error();
            value_to_cstring(val, h)
        }
        Err(e) => {
            h.set_error(&format!("{}: {}", e.message, e.pc));
            ptr::null_mut()
        }
    }
}

// ── Memory Management ─────────────────────────────────────────────────────────

/// Free a string returned by `hlx_run` or `hlx_call`.
///
/// Calling this with NULL is safe and a no-op.
/// Do NOT use the standard C `free()` — use this function, because the string
/// was allocated by Rust.
#[no_mangle]
pub extern "C" fn hlx_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe { drop(CString::from_raw(s)) };
    }
}

// ── Introspection ─────────────────────────────────────────────────────────────

/// Return the last error message as a NUL-terminated C string.
///
/// Returns an empty string (not NULL) when there is no error.
/// The pointer is valid until the next `hlx_*` call on this handle.
/// Do NOT free this pointer.
#[no_mangle]
pub extern "C" fn hlx_errmsg(handle: *const HlxHandle) -> *const c_char {
    if handle.is_null() {
        return b"\0".as_ptr() as *const c_char;
    }
    unsafe { (*handle).last_error.as_ptr() }
}

/// Return the names of all compiled functions as a NUL-terminated
/// comma-separated C string (e.g. `"main,greet,add"`).
///
/// Returns a heap-allocated string — the caller must free it with
/// `hlx_free_string`. Returns NULL if no program has been compiled yet.
#[no_mangle]
pub extern "C" fn hlx_list_functions(handle: *mut HlxHandle) -> *mut c_char {
    if handle.is_null() {
        return ptr::null_mut();
    }
    let h = unsafe { &mut *handle };

    if h.bytecode.is_none() {
        h.set_error("no compiled program — call hlx_compile_* first");
        return ptr::null_mut();
    }

    let mut names: Vec<&str> = h.functions.keys().map(|s| s.as_str()).collect();
    names.sort_unstable();
    let joined = names.join(",");

    match CString::new(joined) {
        Ok(cs) => cs.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

/// Return the HLX runtime version as a NUL-terminated C string.
/// The pointer is to static memory — do NOT free it.
#[no_mangle]
pub extern "C" fn hlx_version() -> *const c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr() as *const c_char
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn value_to_cstring(val: Value, h: &mut HlxHandle) -> *mut c_char {
    match serde_json::to_string(&val) {
        Ok(json) => match CString::new(sanitize(&json)) {
            Ok(cs) => cs.into_raw(),
            Err(_) => {
                h.set_error("return value JSON contained an interior NUL byte");
                ptr::null_mut()
            }
        },
        Err(e) => {
            h.set_error(&format!("failed to serialize return value: {e}"));
            ptr::null_mut()
        }
    }
}

// ── Binary ABI (Phase 15: Zero-Copy) ──────────────────────────────────────────
//
// Wire format: [u32 type_tag][u32 data_len][...data...]
// Tags: 0=nil, 1=i64(8 bytes LE), 2=f64(8 bytes LE), 3=string(UTF-8 bytes),
//       4=tensor(u32 ndim + ndim*u64 shape + n*f64 data), 5=array, 6=map

/// C-ABI tensor handle for zero-copy passing
#[repr(C)]
pub struct HlxTensor {
    pub data: *mut f64,
    pub data_len: usize,
    pub shape: *mut usize,
    pub ndim: usize,
}

/// Create a tensor from raw pointers. Data is COPIED into Rust-owned memory.
/// Returns NULL on invalid input. Caller must free with `hlx_tensor_free`.
#[no_mangle]
pub extern "C" fn hlx_tensor_create_from_ptr(
    data: *const f64,
    data_len: usize,
    shape: *const usize,
    ndim: usize,
) -> *mut HlxTensor {
    if data.is_null() || shape.is_null() || data_len == 0 || ndim == 0 {
        return ptr::null_mut();
    }

    let data_slice = unsafe { std::slice::from_raw_parts(data, data_len) };
    let shape_slice = unsafe { std::slice::from_raw_parts(shape, ndim) };

    // Validate shape matches data length
    let expected: usize = shape_slice.iter().product();
    if expected != data_len {
        return ptr::null_mut();
    }

    let mut data_vec = data_slice.to_vec();
    let mut shape_vec = shape_slice.to_vec();

    let tensor = Box::new(HlxTensor {
        data: data_vec.as_mut_ptr(),
        data_len,
        shape: shape_vec.as_mut_ptr(),
        ndim,
    });

    // Prevent deallocation — ownership transfers to the HlxTensor
    std::mem::forget(data_vec);
    std::mem::forget(shape_vec);

    Box::into_raw(tensor)
}

/// Get pointer to tensor data. Valid until `hlx_tensor_free`.
#[no_mangle]
pub extern "C" fn hlx_tensor_get_data(tensor: *const HlxTensor) -> *const f64 {
    if tensor.is_null() {
        return ptr::null();
    }
    unsafe { (*tensor).data }
}

/// Get tensor data length (number of f64 elements).
#[no_mangle]
pub extern "C" fn hlx_tensor_get_data_len(tensor: *const HlxTensor) -> usize {
    if tensor.is_null() {
        return 0;
    }
    unsafe { (*tensor).data_len }
}

/// Get pointer to tensor shape array.
#[no_mangle]
pub extern "C" fn hlx_tensor_get_shape(tensor: *const HlxTensor) -> *const usize {
    if tensor.is_null() {
        return ptr::null();
    }
    unsafe { (*tensor).shape }
}

/// Get tensor number of dimensions.
#[no_mangle]
pub extern "C" fn hlx_tensor_get_ndim(tensor: *const HlxTensor) -> usize {
    if tensor.is_null() {
        return 0;
    }
    unsafe { (*tensor).ndim }
}

/// Free a tensor created by `hlx_tensor_create_from_ptr`.
#[no_mangle]
pub extern "C" fn hlx_tensor_free(tensor: *mut HlxTensor) {
    if tensor.is_null() {
        return;
    }
    unsafe {
        let t = Box::from_raw(tensor);
        // Reconstruct the Vecs so they get properly deallocated
        if !t.data.is_null() && t.data_len > 0 {
            drop(Vec::from_raw_parts(t.data, t.data_len, t.data_len));
        }
        if !t.shape.is_null() && t.ndim > 0 {
            drop(Vec::from_raw_parts(t.shape, t.ndim, t.ndim));
        }
    }
}

const BIN_TAG_NIL: u32 = 0;
const BIN_TAG_I64: u32 = 1;
const BIN_TAG_F64: u32 = 2;
const BIN_TAG_STRING: u32 = 3;
const BIN_TAG_BOOL: u32 = 7;

/// Encode a Value into the binary wire format.
fn binary_encode(val: &Value) -> Vec<u8> {
    let mut buf = Vec::new();
    match val {
        Value::Nil => {
            buf.extend_from_slice(&BIN_TAG_NIL.to_le_bytes());
            buf.extend_from_slice(&0u32.to_le_bytes());
        }
        Value::I64(v) => {
            buf.extend_from_slice(&BIN_TAG_I64.to_le_bytes());
            buf.extend_from_slice(&8u32.to_le_bytes());
            buf.extend_from_slice(&v.to_le_bytes());
        }
        Value::F64(v) => {
            buf.extend_from_slice(&BIN_TAG_F64.to_le_bytes());
            buf.extend_from_slice(&8u32.to_le_bytes());
            buf.extend_from_slice(&v.to_le_bytes());
        }
        Value::String(s) => {
            let bytes = s.as_bytes();
            buf.extend_from_slice(&BIN_TAG_STRING.to_le_bytes());
            buf.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
            buf.extend_from_slice(bytes);
        }
        Value::Bool(b) => {
            buf.extend_from_slice(&BIN_TAG_BOOL.to_le_bytes());
            buf.extend_from_slice(&1u32.to_le_bytes());
            buf.push(if *b { 1 } else { 0 });
        }
        // Complex types fall back to JSON encoding
        _ => {
            let json = serde_json::to_string(val).unwrap_or_default();
            let bytes = json.as_bytes();
            buf.extend_from_slice(&BIN_TAG_STRING.to_le_bytes());
            buf.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
            buf.extend_from_slice(bytes);
        }
    }
    buf
}

/// Decode a Value from the binary wire format. Returns (Value, bytes_consumed).
fn binary_decode(data: &[u8]) -> Option<(Value, usize)> {
    if data.len() < 8 {
        return None;
    }
    let tag = u32::from_le_bytes(data[0..4].try_into().ok()?);
    let data_len = u32::from_le_bytes(data[4..8].try_into().ok()?) as usize;

    if data.len() < 8 + data_len {
        return None;
    }

    let payload = &data[8..8 + data_len];
    let consumed = 8 + data_len;

    match tag {
        BIN_TAG_NIL => Some((Value::Nil, consumed)),
        BIN_TAG_I64 => {
            if data_len != 8 {
                return None;
            }
            let v = i64::from_le_bytes(payload.try_into().ok()?);
            Some((Value::I64(v), consumed))
        }
        BIN_TAG_F64 => {
            if data_len != 8 {
                return None;
            }
            let v = f64::from_le_bytes(payload.try_into().ok()?);
            Some((Value::F64(v), consumed))
        }
        BIN_TAG_STRING => {
            let s = std::str::from_utf8(payload).ok()?;
            Some((Value::String(s.to_string()), consumed))
        }
        BIN_TAG_BOOL => {
            if data_len != 1 {
                return None;
            }
            Some((Value::Bool(payload[0] != 0), consumed))
        }
        _ => None,
    }
}

/// Binary ABI: Call a function with binary-encoded arguments.
///
/// `args_ptr` points to concatenated binary-encoded values.
/// `args_len` is the total byte length.
/// Returns a heap-allocated binary-encoded result.
/// `out_len` receives the result length.
/// Caller must free with `hlx_free_binary`.
#[no_mangle]
pub extern "C" fn hlx_call_binary(
    handle: *mut HlxHandle,
    func_name: *const c_char,
    args_ptr: *const u8,
    args_len: usize,
    out_len: *mut usize,
) -> *mut u8 {
    if handle.is_null() || func_name.is_null() || out_len.is_null() {
        return ptr::null_mut();
    }
    let h = unsafe { &mut *handle };
    h.vm.reset_execution_state();

    let bytecode = match &h.bytecode {
        Some(bc) => bc.clone(),
        None => {
            h.set_error("no compiled program");
            return ptr::null_mut();
        }
    };

    let name = unsafe {
        match CStr::from_ptr(func_name).to_str() {
            Ok(s) => s,
            Err(_) => {
                h.set_error("func_name is not valid UTF-8");
                return ptr::null_mut();
            }
        }
    };

    // Decode binary args
    let mut args = Vec::new();
    if !args_ptr.is_null() && args_len > 0 {
        let data = unsafe { std::slice::from_raw_parts(args_ptr, args_len) };
        let mut offset = 0;
        while offset < data.len() {
            match binary_decode(&data[offset..]) {
                Some((val, consumed)) => {
                    args.push(val);
                    offset += consumed;
                }
                None => {
                    h.set_error("failed to decode binary argument");
                    return ptr::null_mut();
                }
            }
        }
    }

    match h.vm.call_function(&bytecode, name, &args) {
        Ok(val) => {
            h.clear_error();
            let encoded = binary_encode(&val);
            let len = encoded.len();
            unsafe { *out_len = len };
            let mut boxed = encoded.into_boxed_slice();
            let ptr = boxed.as_mut_ptr();
            std::mem::forget(boxed);
            ptr
        }
        Err(e) => {
            h.set_error(&format!("{}: {}", e.message, e.pc));
            ptr::null_mut()
        }
    }
}

/// Free a binary buffer returned by `hlx_call_binary`.
#[no_mangle]
pub extern "C" fn hlx_free_binary(ptr: *mut u8, len: usize) {
    if !ptr.is_null() && len > 0 {
        unsafe {
            drop(Vec::from_raw_parts(ptr, len, len));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smoke() {
        unsafe {
            let h = hlx_open();
            assert!(!h.is_null());

            let source = CString::new("fn add(a: i64, b: i64) -> i64 { return a + b; }").unwrap();
            let res = hlx_compile_source(h, source.as_ptr());
            assert_eq!(res, 1);

            let func = CString::new("add").unwrap();
            let args =
                CString::new("[{\"type\":\"I64\",\"value\":21},{\"type\":\"I64\",\"value\":21}]")
                    .unwrap();

            let result_ptr = hlx_call(h, func.as_ptr(), args.as_ptr());
            if result_ptr.is_null() {
                let err = CStr::from_ptr(hlx_errmsg(h)).to_str().unwrap();
                panic!("hlx_call failed: {}", err);
            }

            let result_str = CStr::from_ptr(result_ptr).to_str().unwrap();
            assert!(result_str.contains("\"type\":\"I64\""));
            assert!(result_str.contains("\"value\":42"));

            hlx_free_string(result_ptr);
            hlx_close(h);
        }
    }

    #[test]
    fn test_errmsg() {
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

    #[test]
    fn test_binary_encode_decode_i64() {
        let val = Value::I64(42);
        let encoded = binary_encode(&val);
        let (decoded, consumed) = binary_decode(&encoded).unwrap();
        assert_eq!(decoded, Value::I64(42));
        assert_eq!(consumed, encoded.len());
    }

    #[test]
    fn test_binary_encode_decode_f64() {
        let val = Value::F64(3.14);
        let encoded = binary_encode(&val);
        let (decoded, consumed) = binary_decode(&encoded).unwrap();
        assert_eq!(consumed, encoded.len());
        match decoded {
            Value::F64(v) => assert!((v - 3.14).abs() < 1e-10),
            _ => panic!("expected F64"),
        }
    }

    #[test]
    fn test_binary_encode_decode_string() {
        let val = Value::String("hello".to_string());
        let encoded = binary_encode(&val);
        let (decoded, consumed) = binary_decode(&encoded).unwrap();
        assert_eq!(decoded, Value::String("hello".to_string()));
        assert_eq!(consumed, encoded.len());
    }

    #[test]
    fn test_binary_encode_decode_nil() {
        let val = Value::Nil;
        let encoded = binary_encode(&val);
        let (decoded, consumed) = binary_decode(&encoded).unwrap();
        assert_eq!(decoded, Value::Nil);
        assert_eq!(consumed, 8);
    }

    #[test]
    fn test_binary_encode_decode_bool() {
        let val = Value::Bool(true);
        let encoded = binary_encode(&val);
        let (decoded, _) = binary_decode(&encoded).unwrap();
        assert_eq!(decoded, Value::Bool(true));
    }

    #[test]
    fn test_binary_multi_value_concat() {
        let mut buf = Vec::new();
        buf.extend(binary_encode(&Value::I64(1)));
        buf.extend(binary_encode(&Value::I64(2)));

        let (v1, c1) = binary_decode(&buf).unwrap();
        let (v2, c2) = binary_decode(&buf[c1..]).unwrap();
        assert_eq!(v1, Value::I64(1));
        assert_eq!(v2, Value::I64(2));
        assert_eq!(c1 + c2, buf.len());
    }

    #[test]
    fn test_binary_call_add() {
        unsafe {
            let h = hlx_open();
            let source = CString::new("fn add(a: i64, b: i64) -> i64 { return a + b; }").unwrap();
            assert_eq!(hlx_compile_source(h, source.as_ptr()), 1);

            // Encode two i64 args in binary
            let mut args_buf = Vec::new();
            args_buf.extend(binary_encode(&Value::I64(21)));
            args_buf.extend(binary_encode(&Value::I64(21)));

            let func = CString::new("add").unwrap();
            let mut out_len: usize = 0;
            let result_ptr = hlx_call_binary(
                h,
                func.as_ptr(),
                args_buf.as_ptr(),
                args_buf.len(),
                &mut out_len,
            );

            assert!(!result_ptr.is_null());
            let result_data = std::slice::from_raw_parts(result_ptr, out_len);
            let (val, _) = binary_decode(result_data).unwrap();
            assert_eq!(val, Value::I64(42));

            hlx_free_binary(result_ptr, out_len);
            hlx_close(h);
        }
    }

    #[test]
    fn test_tensor_create_and_read() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let shape = vec![2usize, 3usize];

        let tensor = hlx_tensor_create_from_ptr(
            data.as_ptr(),
            data.len(),
            shape.as_ptr(),
            shape.len(),
        );
        assert!(!tensor.is_null());

        unsafe {
            assert_eq!(hlx_tensor_get_data_len(tensor), 6);
            assert_eq!(hlx_tensor_get_ndim(tensor), 2);

            let data_ptr = hlx_tensor_get_data(tensor);
            let read_data = std::slice::from_raw_parts(data_ptr, 6);
            assert_eq!(read_data, &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);

            let shape_ptr = hlx_tensor_get_shape(tensor);
            let read_shape = std::slice::from_raw_parts(shape_ptr, 2);
            assert_eq!(read_shape, &[2, 3]);

            hlx_tensor_free(tensor);
        }
    }

    #[test]
    fn test_tensor_shape_mismatch_returns_null() {
        let data = vec![1.0, 2.0, 3.0];
        let shape = vec![2usize, 3usize]; // expects 6 elements, got 3
        let tensor = hlx_tensor_create_from_ptr(
            data.as_ptr(),
            data.len(),
            shape.as_ptr(),
            shape.len(),
        );
        assert!(tensor.is_null());
    }
}
