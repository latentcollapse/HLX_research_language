use crate::{RuntimeError, RuntimeResult, Value};

pub fn builtin_strlen(args: &[Value]) -> RuntimeResult<Value> {
    match &args[0] {
        Value::String(s) => Ok(Value::I64(s.len() as i64)),
        Value::Array(arr) => Ok(Value::I64(arr.len() as i64)),
        _ => Err(RuntimeError::new(
            format!(
                "strlen requires String or Array, got {}",
                args[0].type_name()
            ),
            0,
        )),
    }
}

pub fn builtin_substring(args: &[Value]) -> RuntimeResult<Value> {
    let s = args[0]
        .as_string()
        .ok_or_else(|| RuntimeError::new("substring requires String", 0))?;
    let start_i = args[1]
        .as_i64()
        .ok_or_else(|| RuntimeError::new("substring start must be i64", 0))?;
    let len_i = args[2]
        .as_i64()
        .ok_or_else(|| RuntimeError::new("substring len must be i64", 0))?;

    if start_i < 0 || len_i < 0 {
        return Err(RuntimeError::new(
            "substring: negative index not allowed",
            0,
        ));
    }

    let start = (start_i as usize).min(s.len());
    let end = (start + len_i as usize).min(s.len());
    Ok(Value::String(s[start..end].to_string()))
}

pub fn builtin_concat(args: &[Value]) -> RuntimeResult<Value> {
    let a = args[0]
        .as_string()
        .ok_or_else(|| RuntimeError::new("concat requires String", 0))?;
    let b = args[1]
        .as_string()
        .ok_or_else(|| RuntimeError::new("concat requires String", 0))?;
    Ok(Value::String(format!("{}{}", a, b)))
}

pub fn builtin_strcmp(args: &[Value]) -> RuntimeResult<Value> {
    let a = args[0]
        .as_string()
        .ok_or_else(|| RuntimeError::new("strcmp requires String", 0))?;
    let b = args[1]
        .as_string()
        .ok_or_else(|| RuntimeError::new("strcmp requires String", 0))?;
    Ok(Value::I64(match a.cmp(b) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }))
}

pub fn builtin_ord(args: &[Value]) -> RuntimeResult<Value> {
    let s = args[0]
        .as_string()
        .ok_or_else(|| RuntimeError::new("ord requires String", 0))?;
    if s.is_empty() {
        return Err(RuntimeError::new("ord: empty string", 0));
    }
    Ok(Value::I64(s.as_bytes()[0] as i64))
}

pub fn builtin_char(args: &[Value]) -> RuntimeResult<Value> {
    let code_i = args[0]
        .as_i64()
        .ok_or_else(|| RuntimeError::new("char requires i64", 0))?;
    if code_i < 0 || code_i > 127 {
        return Err(RuntimeError::new(
            format!("char: code {} out of ASCII range (0-127)", code_i),
            0,
        ));
    }
    Ok(Value::String((code_i as u8 as char).to_string()))
}

pub fn builtin_push(args: &[Value]) -> RuntimeResult<Value> {
    let arr = args[0]
        .as_array()
        .ok_or_else(|| RuntimeError::new("push requires Array", 0))?;
    let mut new_arr = arr.to_vec();
    new_arr.push(args[1].clone());
    Ok(Value::Array(new_arr))
}

pub fn builtin_get_at(args: &[Value]) -> RuntimeResult<Value> {
    let arr = args[0]
        .as_array()
        .ok_or_else(|| RuntimeError::new("get_at requires Array", 0))?;
    let idx = args[1]
        .as_i64()
        .ok_or_else(|| RuntimeError::new("get_at index must be i64", 0))? as usize;
    arr.get(idx)
        .cloned()
        .ok_or_else(|| RuntimeError::new(format!("get_at: index {} out of bounds", idx), 0))
}

pub fn builtin_set_at(args: &[Value]) -> RuntimeResult<Value> {
    let arr = args[0]
        .as_array()
        .ok_or_else(|| RuntimeError::new("set_at requires Array", 0))?;
    let idx = args[1]
        .as_i64()
        .ok_or_else(|| RuntimeError::new("set_at index must be i64", 0))? as usize;
    if idx >= arr.len() {
        return Err(RuntimeError::new(
            format!("set_at: index {} out of bounds", idx),
            0,
        ));
    }
    let mut new_arr = arr.to_vec();
    new_arr[idx] = args[2].clone();
    Ok(Value::Array(new_arr))
}

pub fn builtin_array_len(args: &[Value]) -> RuntimeResult<Value> {
    match &args[0] {
        Value::Array(arr) => Ok(Value::I64(arr.len() as i64)),
        Value::String(s) => Ok(Value::I64(s.len() as i64)),
        _ => Err(RuntimeError::new(
            format!("len requires Array or String, got {}", args[0].type_name()),
            0,
        )),
    }
}

pub fn builtin_print(args: &[Value]) -> RuntimeResult<Value> {
    print!("{}", args[0]);
    Ok(Value::Void)
}

pub fn builtin_println(args: &[Value]) -> RuntimeResult<Value> {
    println!("{}", args[0]);
    Ok(Value::Void)
}
