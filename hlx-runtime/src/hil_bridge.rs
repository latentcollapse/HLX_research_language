//! HIL (HLX Intermediate Language) Bridge
//!
//! This module provides native function implementations for the HIL stdlib.
//! These are verified implementations that satisfy the type signatures expected
//! by HLX code importing from `hlx/hil`.

use crate::{Value, Vm};

/// Register all HIL native functions with the VM
pub fn register_hil_functions(vm: &mut Vm) {
    let hil_functions: Vec<(&str, fn(&mut Vm, Vec<Value>) -> Value)> = vec![
        ("hil::print", hil_print),
        ("hil::debug", hil_debug),
        ("hil::assert", hil_assert),
        ("hil::is_nil", hil_is_nil),
        ("hil::is_int", hil_is_int),
        ("hil::is_float", hil_is_float),
        ("hil::is_string", hil_is_string),
        ("hil::is_array", hil_is_array),
        ("hil::is_map", hil_is_map),
        ("hil::len", hil_len),
        ("hil::to_string", hil_to_string),
        ("hil::to_int", hil_to_int),
        ("hil::to_float", hil_to_float),
        ("hil::array_get", hil_array_get),
        ("hil::array_set", hil_array_set),
        ("hil::array_push", hil_array_push),
        ("hil::array_pop", hil_array_pop),
        ("hil::map_get", hil_map_get),
        ("hil::map_set", hil_map_set),
        ("hil::map_has", hil_map_has),
        ("hil::map_keys", hil_map_keys),
        ("hil::string_concat", hil_string_concat),
        ("hil::string_slice", hil_string_slice),
        ("hil::string_contains", hil_string_contains),
        ("hil::range", hil_range),
        ("hil::min", hil_min),
        ("hil::max", hil_max),
        ("hil::abs", hil_abs),
        ("hil::floor", hil_floor),
        ("hil::ceil", hil_ceil),
        ("hil::round", hil_round),
        ("hil::sqrt", hil_sqrt),
        ("hil::pow", hil_pow),
        ("hil::mem_query_vec", hil_mem_query_vec),
    ];

    for (name, func) in hil_functions {
        vm.register_native(name, func);
    }
}

// ─── I/O Functions ─────────────────────────────────────────────────────────

fn hil_print(_vm: &mut Vm, args: Vec<Value>) -> Value {
    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            print!(" ");
        }
        print!("{}", format_value(arg));
    }
    println!();
    Value::Nil
}

fn hil_debug(_vm: &mut Vm, args: Vec<Value>) -> Value {
    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            eprint!(" ");
        }
        eprint!("{:?}", arg);
    }
    eprintln!();
    Value::Nil
}

fn hil_assert(_vm: &mut Vm, args: Vec<Value>) -> Value {
    if args.is_empty() {
        return Value::Bool(true);
    }
    match &args[0] {
        Value::Bool(true) => Value::Bool(true),
        Value::Bool(false) => {
            let msg = args.get(1).map(|m| format_value(m)).unwrap_or_default();
            panic!("Assertion failed: {}", msg);
        }
        _ => {
            let is_truthy = !matches!(args[0], Value::Nil | Value::Bool(false));
            Value::Bool(is_truthy)
        }
    }
}

// ─── Type Checking Functions ────────────────────────────────────────────────

fn hil_is_nil(_vm: &mut Vm, args: Vec<Value>) -> Value {
    Value::Bool(matches!(args.get(0), Some(Value::Nil)))
}

fn hil_is_int(_vm: &mut Vm, args: Vec<Value>) -> Value {
    Value::Bool(matches!(args.get(0), Some(Value::I64(_))))
}

fn hil_is_float(_vm: &mut Vm, args: Vec<Value>) -> Value {
    Value::Bool(matches!(args.get(0), Some(Value::F64(_))))
}

fn hil_is_string(_vm: &mut Vm, args: Vec<Value>) -> Value {
    Value::Bool(matches!(args.get(0), Some(Value::String(_))))
}

fn hil_is_array(_vm: &mut Vm, args: Vec<Value>) -> Value {
    Value::Bool(matches!(args.get(0), Some(Value::Array(_))))
}

fn hil_is_map(_vm: &mut Vm, args: Vec<Value>) -> Value {
    Value::Bool(matches!(args.get(0), Some(Value::Map(_))))
}

// ─── Collection Functions ───────────────────────────────────────────────────

fn hil_len(_vm: &mut Vm, args: Vec<Value>) -> Value {
    match args.get(0) {
        Some(Value::Array(arr)) => Value::I64(arr.len() as i64),
        Some(Value::Map(map)) => Value::I64(map.len() as i64),
        Some(Value::String(s)) => Value::I64(s.len() as i64),
        _ => Value::I64(0),
    }
}

fn hil_array_get(_vm: &mut Vm, args: Vec<Value>) -> Value {
    if args.len() < 2 {
        return Value::Nil;
    }
    match (&args[0], &args[1]) {
        (Value::Array(arr), Value::I64(idx)) => {
            let i = *idx as usize;
            arr.get(i).cloned().unwrap_or(Value::Nil)
        }
        _ => Value::Nil,
    }
}

fn hil_array_set(_vm: &mut Vm, args: Vec<Value>) -> Value {
    if args.len() < 3 {
        return Value::Nil;
    }
    if let (Value::Array(arr), Value::I64(idx)) = (&args[0], &args[1]) {
        let mut new_arr = arr.clone();
        let i = *idx as usize;
        if i < new_arr.len() {
            new_arr[i] = args[2].clone();
            return Value::Array(new_arr);
        }
    }
    Value::Nil
}

fn hil_array_push(_vm: &mut Vm, args: Vec<Value>) -> Value {
    if args.len() < 2 {
        return Value::Nil;
    }
    if let Value::Array(arr) = &args[0] {
        let mut new_arr = arr.clone();
        new_arr.push(args[1].clone());
        return Value::Array(new_arr);
    }
    Value::Nil
}

fn hil_array_pop(_vm: &mut Vm, args: Vec<Value>) -> Value {
    if args.is_empty() {
        return Value::Nil;
    }
    if let Value::Array(arr) = &args[0] {
        let mut new_arr = arr.clone();
        // hil_array_pop in many languages returns the popped value,
        // but here we are using copy-on-write arrays.
        // For HIL bridge we return the new array or we'd need to return a pair.
        // Standard HIL likely expects the popped value.
        return new_arr.pop().unwrap_or(Value::Nil);
    }
    Value::Nil
}

fn hil_map_get(_vm: &mut Vm, args: Vec<Value>) -> Value {
    if args.len() < 2 {
        return Value::Nil;
    }
    match (&args[0], &args[1]) {
        (Value::Map(map), key) => map.get(&format_value(key)).cloned().unwrap_or(Value::Nil),
        _ => Value::Nil,
    }
}

fn hil_map_set(_vm: &mut Vm, args: Vec<Value>) -> Value {
    if args.len() < 3 {
        return Value::Nil;
    }
    if let Value::Map(map) = &args[0] {
        let mut new_map = map.clone();
        new_map.insert(format_value(&args[1]), args[2].clone());
        return Value::Map(new_map);
    }
    Value::Nil
}

fn hil_map_has(_vm: &mut Vm, args: Vec<Value>) -> Value {
    if args.len() < 2 {
        return Value::Bool(false);
    }
    match (&args[0], &args[1]) {
        (Value::Map(map), key) => Value::Bool(map.contains_key(&format_value(key))),
        _ => Value::Bool(false),
    }
}

fn hil_map_keys(_vm: &mut Vm, args: Vec<Value>) -> Value {
    match args.get(0) {
        Some(Value::Map(map)) => {
            let keys: Vec<Value> = map.keys().map(|k| Value::String(k.clone())).collect();
            Value::Array(keys)
        }
        _ => Value::Array(vec![]),
    }
}

// ─── Conversion Functions ───────────────────────────────────────────────────

fn hil_to_string(_vm: &mut Vm, args: Vec<Value>) -> Value {
    match args.get(0) {
        Some(val) => Value::String(format_value(val)),
        None => Value::String(String::new()),
    }
}

fn hil_to_int(_vm: &mut Vm, args: Vec<Value>) -> Value {
    match args.get(0) {
        Some(Value::I64(n)) => Value::I64(*n),
        Some(Value::F64(f)) => Value::I64(*f as i64),
        Some(Value::String(s)) => Value::I64(s.parse().unwrap_or(0)),
        Some(Value::Bool(true)) => Value::I64(1),
        Some(Value::Bool(false)) => Value::I64(0),
        _ => Value::I64(0),
    }
}

fn hil_to_float(_vm: &mut Vm, args: Vec<Value>) -> Value {
    match args.get(0) {
        Some(Value::I64(n)) => Value::F64(*n as f64),
        Some(Value::F64(f)) => Value::F64(*f),
        Some(Value::String(s)) => Value::F64(s.parse().unwrap_or(0.0)),
        _ => Value::F64(0.0),
    }
}

// ─── String Functions ───────────────────────────────────────────────────────

fn hil_string_concat(_vm: &mut Vm, args: Vec<Value>) -> Value {
    let mut result = String::new();
    for arg in args {
        result.push_str(&format_value(&arg));
    }
    Value::String(result)
}

fn hil_string_slice(_vm: &mut Vm, args: Vec<Value>) -> Value {
    if args.len() < 3 {
        return Value::String(String::new());
    }
    match (&args[0], &args[1], &args[2]) {
        (Value::String(s), Value::I64(start), Value::I64(end)) => {
            let start_idx = (*start as usize).min(s.len());
            let end_idx = (*end as usize).min(s.len());
            if start_idx <= end_idx {
                Value::String(s[start_idx..end_idx].to_string())
            } else {
                Value::String(String::new())
            }
        }
        _ => Value::String(String::new()),
    }
}

fn hil_string_contains(_vm: &mut Vm, args: Vec<Value>) -> Value {
    if args.len() < 2 {
        return Value::Bool(false);
    }
    match (&args[0], &args[1]) {
        (Value::String(s), Value::String(sub)) => Value::Bool(s.contains(sub)),
        _ => Value::Bool(false),
    }
}

// ─── Math Functions ─────────────────────────────────────────────────────────

fn hil_range(_vm: &mut Vm, args: Vec<Value>) -> Value {
    let start = match args.get(0) {
        Some(Value::I64(n)) => *n,
        _ => 0,
    };
    let end = match args.get(1) {
        Some(Value::I64(n)) => *n,
        _ => start,
    };
    let arr: Vec<Value> = (start..end).map(|i| Value::I64(i)).collect();
    Value::Array(arr)
}

fn hil_min(_vm: &mut Vm, args: Vec<Value>) -> Value {
    if args.len() < 2 {
        return args.get(0).cloned().unwrap_or(Value::Nil);
    }
    match (&args[0], &args[1]) {
        (Value::I64(a), Value::I64(b)) => Value::I64(*a.min(b)),
        (Value::F64(a), Value::F64(b)) => Value::F64(a.min(*b)),
        _ => args[0].clone(),
    }
}

fn hil_max(_vm: &mut Vm, args: Vec<Value>) -> Value {
    if args.len() < 2 {
        return args.get(0).cloned().unwrap_or(Value::Nil);
    }
    match (&args[0], &args[1]) {
        (Value::I64(a), Value::I64(b)) => Value::I64(*a.max(b)),
        (Value::F64(a), Value::F64(b)) => Value::F64(a.max(*b)),
        _ => args[0].clone(),
    }
}

fn hil_abs(_vm: &mut Vm, args: Vec<Value>) -> Value {
    match args.get(0) {
        Some(Value::I64(n)) => Value::I64(n.abs()),
        Some(Value::F64(f)) => Value::F64(f.abs()),
        other => other.cloned().unwrap_or(Value::Nil),
    }
}

fn hil_floor(_vm: &mut Vm, args: Vec<Value>) -> Value {
    match args.get(0) {
        Some(Value::F64(f)) => Value::F64(f.floor()),
        Some(Value::I64(n)) => Value::I64(*n),
        other => other.cloned().unwrap_or(Value::Nil),
    }
}

fn hil_ceil(_vm: &mut Vm, args: Vec<Value>) -> Value {
    match args.get(0) {
        Some(Value::F64(f)) => Value::F64(f.ceil()),
        Some(Value::I64(n)) => Value::I64(*n),
        other => other.cloned().unwrap_or(Value::Nil),
    }
}

fn hil_round(_vm: &mut Vm, args: Vec<Value>) -> Value {
    match args.get(0) {
        Some(Value::F64(f)) => Value::F64(f.round()),
        Some(Value::I64(n)) => Value::I64(*n),
        other => other.cloned().unwrap_or(Value::Nil),
    }
}

fn hil_sqrt(_vm: &mut Vm, args: Vec<Value>) -> Value {
    match args.get(0) {
        Some(Value::F64(f)) => Value::F64(f.sqrt()),
        Some(Value::I64(n)) => Value::F64((*n as f64).sqrt()),
        _ => Value::F64(0.0),
    }
}

fn hil_pow(_vm: &mut Vm, args: Vec<Value>) -> Value {
    if args.len() < 2 {
        return Value::F64(0.0);
    }
    match (&args[0], &args[1]) {
        (Value::F64(base), Value::F64(exp)) => Value::F64(base.powf(*exp)),
        (Value::I64(base), Value::I64(exp)) => Value::I64(base.pow(*exp as u32)),
        _ => Value::F64(0.0),
    }
}

// ─── Memory Functions ───────────────────────────────────────────────────────

/// Vector similarity search for memory queries
/// Args: query_embedding (Vec<f32>), top_k (i64)
/// Returns: Array of matching observations
fn hil_mem_query_vec(vm: &mut Vm, args: Vec<Value>) -> Value {
    if args.len() < 2 {
        return Value::Array(vec![]);
    }

    // Extract query embedding from array
    let query_embedding: Vec<f32> = match &args[0] {
        Value::Array(arr) => arr
            .iter()
            .filter_map(|v| match v {
                Value::F64(f) => Some(*f as f32),
                Value::I64(i) => Some(*i as f32),
                _ => None,
            })
            .collect(),
        _ => return Value::Array(vec![]),
    };

    let top_k = match &args[1] {
        Value::I64(n) => *n as usize,
        _ => 5,
    };

    // Get the memory pool and query by embedding
    let results = vm.mem_query_vec(&query_embedding, top_k);

    // Convert results to array of observations
    Value::Array(
        results
            .into_iter()
            .map(|content| Value::String(content))
            .collect(),
    )
}

// ─── Helper Functions ───────────────────────────────────────────────────────

fn format_value(val: &Value) -> String {
    match val {
        Value::I64(n) => n.to_string(),
        Value::F64(f) => f.to_string(),
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Nil => "nil".to_string(),
        Value::Void => "void".to_string(),
        Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(format_value).collect();
            format!("[{}]", items.join(", "))
        }
        Value::Map(map) => {
            let items: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("{}: {}", k, format_value(v)))
                .collect();
            format!("{{{}}}", items.join(", "))
        }
        Value::Agent(id) => format!("<agent:{}>", id),
        Value::Tensor(_) => "<tensor>".to_string(),
        Value::Function(name) => format!("<function:{}>", name),
        Value::Bytes(b) => format!("<bytes:{}>", b.len()),
    }
}
