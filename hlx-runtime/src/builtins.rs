use crate::{RuntimeError, RuntimeResult, Tensor, Value};
use image::ImageFormat;

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

pub fn builtin_image_load(args: &[Value]) -> RuntimeResult<Value> {
    let path = args[0]
        .as_string()
        .ok_or_else(|| RuntimeError::new("image_load requires String path", 0))?;

    let expanded = shellexpand::full(path)
        .map_err(|e| RuntimeError::new(format!("Path expansion failed: {}", e), 0))?;

    let bytes = std::fs::read(expanded.as_ref())
        .map_err(|e| RuntimeError::new(format!("Failed to read image: {}", e), 0))?;

    let tensor = Tensor::from_image_bytes(&bytes)?;
    Ok(Value::Tensor(tensor))
}

pub fn builtin_image_save(args: &[Value]) -> RuntimeResult<Value> {
    let tensor = match &args[0] {
        Value::Tensor(t) => t,
        _ => return Err(RuntimeError::new("image_save requires Tensor", 0)),
    };

    let path = args[1]
        .as_string()
        .ok_or_else(|| RuntimeError::new("image_save requires String path", 0))?;

    let format = if path.to_lowercase().ends_with(".png") {
        ImageFormat::Png
    } else {
        ImageFormat::Jpeg
    };

    let bytes = tensor.to_image_bytes(format)?;

    let expanded = shellexpand::full(&path)
        .map_err(|e| RuntimeError::new(format!("Path expansion failed: {}", e), 0))?;

    std::fs::write(expanded.as_ref(), &bytes)
        .map_err(|e| RuntimeError::new(format!("Failed to write image: {}", e), 0))?;

    Ok(Value::Void)
}

pub fn builtin_image_process(args: &[Value]) -> RuntimeResult<Value> {
    let tensor = match &args[0] {
        Value::Tensor(t) => t.clone(),
        _ => return Err(RuntimeError::new("image_process requires Tensor", 0)),
    };

    let op = args[1]
        .as_string()
        .ok_or_else(|| RuntimeError::new("image_process requires String operation", 0))?;

    let params = if args.len() > 2 {
        args[2].as_f64().unwrap_or(1.0)
    } else {
        1.0
    };

    if tensor.shape.len() != 3 {
        return Err(RuntimeError::new(
            format!(
                "Image tensor must be CHW (3 dims), got {} dims",
                tensor.shape.len()
            ),
            0,
        ));
    }

    let (channels, height, width) = (tensor.shape[0], tensor.shape[1], tensor.shape[2]);
    let mut result = tensor.clone();

    match op.as_ref() {
        "grayscale" => {
            if channels != 3 {
                return Err(RuntimeError::new("grayscale requires 3-channel image", 0));
            }
            let mut gray_data = vec![0.0f64; height * width];
            for y in 0..height {
                for x in 0..width {
                    let base = y * width + x;
                    let r = tensor.data[base];
                    let g = tensor.data[height * width + base];
                    let b = tensor.data[2 * height * width + base];
                    gray_data[base] = 0.299 * r + 0.587 * g + 0.114 * b;
                }
            }
            result = Tensor::from_data(vec![1, height, width], gray_data).unwrap();
        }
        "invert" => {
            for i in 0..result.data.len() {
                result.data[i] = 1.0 - result.data[i];
            }
        }
        "brightness" => {
            let factor = params.max(0.0);
            for i in 0..result.data.len() {
                result.data[i] = (result.data[i] * factor).min(1.0).max(0.0);
            }
        }
        "contrast" => {
            let factor = params.max(0.0);
            for i in 0..result.data.len() {
                result.data[i] = ((result.data[i] - 0.5) * factor + 0.5).min(1.0).max(0.0);
            }
        }
        "threshold" => {
            let thresh = params;
            for i in 0..result.data.len() {
                result.data[i] = if result.data[i] > thresh { 1.0 } else { 0.0 };
            }
        }
        "blur" => {
            let kernel_size = params as usize;
            let sigma = 1.0;
            let kernel_radius = kernel_size / 2;
            let mut kernel = vec![0.0f64; kernel_size];
            let mut sum = 0.0;
            for i in 0..kernel_size {
                let x = (i as f64) - kernel_radius as f64;
                kernel[i] = (-x * x / (2.0 * sigma * sigma)).exp();
                sum += kernel[i];
            }
            for i in 0..kernel_size {
                kernel[i] /= sum;
            }
            for c in 0..channels {
                let offset = c * height * width;
                let mut temp = vec![0.0f64; height * width];
                for y in 0..height {
                    for x in 0..width {
                        let mut acc = 0.0;
                        let mut weight_sum = 0.0;
                        for k in 0..kernel_size {
                            let nx = (x as isize + k as isize - kernel_radius as isize)
                                .max(0)
                                .min(width as isize - 1)
                                as usize;
                            acc += tensor.data[offset + y * width + nx] * kernel[k];
                            weight_sum += kernel[k];
                        }
                        temp[y * width + x] = acc / weight_sum;
                    }
                }
                for y in 0..height {
                    for x in 0..width {
                        let mut acc = 0.0;
                        let mut weight_sum = 0.0;
                        for k in 0..kernel_size {
                            let ny = (y as isize + k as isize - kernel_radius as isize)
                                .max(0)
                                .min(height as isize - 1)
                                as usize;
                            acc += temp[ny * width + x] * kernel[k];
                            weight_sum += kernel[k];
                        }
                        result.data[offset + y * width + x] = acc / weight_sum;
                    }
                }
            }
        }
        "sharpen" => {
            let amount = params;
            for c in 0..channels {
                let offset = c * height * width;
                for y in 1..(height - 1) {
                    for x in 1..(width - 1) {
                        let center = tensor.data[offset + y * width + x];
                        let top = tensor.data[offset + (y - 1) * width + x];
                        let bottom = tensor.data[offset + (y + 1) * width + x];
                        let left = tensor.data[offset + y * width + (x - 1)];
                        let right = tensor.data[offset + y * width + (x + 1)];
                        let laplacian = 4.0 * center - top - bottom - left - right;
                        result.data[offset + y * width + x] =
                            (center + laplacian * amount).min(1.0).max(0.0);
                    }
                }
            }
        }
        "sobel" => {
            if channels != 1 {
                return Err(RuntimeError::new(
                    "sobel requires grayscale (1-channel) image",
                    0,
                ));
            }
            for y in 1..(height - 1) {
                for x in 1..(width - 1) {
                    let gx = -tensor.data[(y - 1) * width + (x - 1)]
                        + tensor.data[(y - 1) * width + (x + 1)]
                        - 2.0 * tensor.data[y * width + (x - 1)]
                        + 2.0 * tensor.data[y * width + (x + 1)]
                        - tensor.data[(y + 1) * width + (x - 1)]
                        + tensor.data[(y + 1) * width + (x + 1)];
                    let gy = -tensor.data[(y - 1) * width + (x - 1)]
                        - 2.0 * tensor.data[(y - 1) * width + x]
                        - tensor.data[(y - 1) * width + (x + 1)]
                        + tensor.data[(y + 1) * width + (x - 1)]
                        + 2.0 * tensor.data[(y + 1) * width + x]
                        + tensor.data[(y + 1) * width + (x + 1)];
                    result.data[y * width + x] = (gx * gx + gy * gy).sqrt().min(1.0).max(0.0);
                }
            }
        }
        _ => {
            return Err(RuntimeError::new(
                format!("Unknown image operation: {}", op),
                0,
            ))
        }
    }

    Ok(Value::Tensor(result))
}

pub fn builtin_image_info(args: &[Value]) -> RuntimeResult<Value> {
    let tensor = match &args[0] {
        Value::Tensor(t) => t,
        _ => return Err(RuntimeError::new("image_info requires Tensor", 0)),
    };

    let (height, width, channels) = tensor.image_dimensions()?;

    Ok(Value::Array(vec![
        Value::I64(height as i64),
        Value::I64(width as i64),
        Value::I64(channels as i64),
    ]))
}

pub fn builtin_audio_load(args: &[Value]) -> RuntimeResult<Value> {
    let path = args[0]
        .as_string()
        .ok_or_else(|| RuntimeError::new("audio_load requires String path", 0))?;

    let tensor = Tensor::from_audio_file(path)?;
    Ok(Value::Tensor(tensor))
}

pub fn builtin_audio_save(args: &[Value]) -> RuntimeResult<Value> {
    let tensor = match &args[0] {
        Value::Tensor(t) => t,
        _ => return Err(RuntimeError::new("audio_save requires Tensor", 0)),
    };

    let path = args[1]
        .as_string()
        .ok_or_else(|| RuntimeError::new("audio_save requires String path", 0))?;

    let sample_rate = if args.len() > 2 {
        args[2].as_i64().unwrap_or(44100) as u32
    } else {
        44100
    };

    tensor.to_audio_file(path, sample_rate)?;

    Ok(Value::Void)
}

pub fn builtin_audio_info(args: &[Value]) -> RuntimeResult<Value> {
    let tensor = match &args[0] {
        Value::Tensor(t) => t,
        _ => return Err(RuntimeError::new("audio_info requires Tensor", 0)),
    };

    let (channels, num_samples) = tensor.audio_info()?;

    Ok(Value::Array(vec![
        Value::I64(channels as i64),
        Value::I64(num_samples as i64),
    ]))
}

pub fn builtin_audio_resample(args: &[Value]) -> RuntimeResult<Value> {
    let tensor = match &args[0] {
        Value::Tensor(t) => t.clone(),
        _ => return Err(RuntimeError::new("audio_resample requires Tensor", 0)),
    };

    let factor = args[1]
        .as_f64()
        .ok_or_else(|| RuntimeError::new("audio_resample requires f64 factor", 0))?;

    if factor <= 0.0 {
        return Err(RuntimeError::new("Resample factor must be positive", 0));
    }

    let (channels, num_samples) = tensor.audio_info()?;
    let new_num_samples = (num_samples as f64 * factor) as usize;

    let mut result = Tensor::zeros(vec![channels, new_num_samples]);

    for ch in 0..channels {
        let src_offset = ch * num_samples;
        let dst_offset = ch * new_num_samples;

        for i in 0..new_num_samples {
            let src_pos = i as f64 / factor;
            let src_idx = src_pos as usize;
            let src_frac = src_pos - src_idx as f64;

            let s0 = if src_idx < num_samples {
                tensor.data[src_offset + src_idx]
            } else {
                0.0
            };
            let s1 = if src_idx + 1 < num_samples {
                tensor.data[src_offset + src_idx + 1]
            } else {
                s0
            };

            result.data[dst_offset + i] = s0 * (1.0 - src_frac) + s1 * src_frac;
        }
    }

    Ok(Value::Tensor(result))
}

pub fn builtin_audio_normalize(args: &[Value]) -> RuntimeResult<Value> {
    let tensor = match &args[0] {
        Value::Tensor(t) => t.clone(),
        _ => return Err(RuntimeError::new("audio_normalize requires Tensor", 0)),
    };

    let max_val = tensor.data.iter().fold(0.0f64, |acc, &x| acc.max(x.abs()));

    if max_val == 0.0 {
        return Ok(Value::Tensor(tensor));
    }

    let mut result = tensor.clone();
    for i in 0..result.data.len() {
        result.data[i] /= max_val;
    }

    Ok(Value::Tensor(result))
}

// ============================================================================
// Bit's Builtins (Phase 2)
// ============================================================================

pub fn builtin_zeros(args: &[Value]) -> RuntimeResult<Value> {
    let size = args[0]
        .as_i64()
        .ok_or_else(|| RuntimeError::new("zeros requires i64", 0))?;
    if size <= 0 || size > 10000 {
        return Err(RuntimeError::new("zeros: invalid size", 0));
    }
    Ok(Value::Tensor(Tensor::zeros(vec![size as usize])))
}

pub fn builtin_i64_to_str(args: &[Value]) -> RuntimeResult<Value> {
    let n = args[0]
        .as_i64()
        .ok_or_else(|| RuntimeError::new("i64_to_str requires i64", 0))?;
    Ok(Value::String(n.to_string()))
}

pub fn builtin_f64_to_str(args: &[Value]) -> RuntimeResult<Value> {
    let n = args[0]
        .as_f64()
        .ok_or_else(|| RuntimeError::new("f64_to_str requires f64", 0))?;
    Ok(Value::String(format!("{:.6}", n)))
}

pub fn builtin_str_contains(args: &[Value]) -> RuntimeResult<Value> {
    let haystack = args[0]
        .as_string()
        .ok_or_else(|| RuntimeError::new("str_contains requires String", 0))?;
    let needle = args[1]
        .as_string()
        .ok_or_else(|| RuntimeError::new("str_contains requires String", 0))?;
    Ok(Value::Bool(haystack.contains(&needle)))
}

pub fn builtin_str_equals(args: &[Value]) -> RuntimeResult<Value> {
    let a = args[0]
        .as_string()
        .ok_or_else(|| RuntimeError::new("str_equals requires String", 0))?;
    let b = args[1]
        .as_string()
        .ok_or_else(|| RuntimeError::new("str_equals requires String", 0))?;
    Ok(Value::Bool(a == b))
}

pub fn builtin_sqrt(args: &[Value]) -> RuntimeResult<Value> {
    let n = args[0]
        .as_f64()
        .ok_or_else(|| RuntimeError::new("sqrt requires f64", 0))?;
    Ok(Value::F64(n.sqrt()))
}

pub fn builtin_hash(args: &[Value]) -> RuntimeResult<Value> {
    let s = args[0]
        .as_string()
        .ok_or_else(|| RuntimeError::new("hash requires String", 0))?;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    Ok(Value::I64(hasher.finish() as i64))
}

// ============================================================================
// Phase 1.4: Missing builtins - Math
// ============================================================================

pub fn builtin_abs(args: &[Value]) -> RuntimeResult<Value> {
    match &args[0] {
        Value::I64(n) => Ok(Value::I64(n.abs())),
        Value::F64(n) => Ok(Value::F64(n.abs())),
        _ => Err(RuntimeError::new("abs requires i64 or f64", 0)),
    }
}

pub fn builtin_floor(args: &[Value]) -> RuntimeResult<Value> {
    let n = args[0]
        .as_f64()
        .ok_or_else(|| RuntimeError::new("floor requires f64", 0))?;
    Ok(Value::I64(n.floor() as i64))
}

pub fn builtin_ceil(args: &[Value]) -> RuntimeResult<Value> {
    let n = args[0]
        .as_f64()
        .ok_or_else(|| RuntimeError::new("ceil requires f64", 0))?;
    Ok(Value::I64(n.ceil() as i64))
}

pub fn builtin_round(args: &[Value]) -> RuntimeResult<Value> {
    let n = args[0]
        .as_f64()
        .ok_or_else(|| RuntimeError::new("round requires f64", 0))?;
    Ok(Value::I64(n.round() as i64))
}

pub fn builtin_min(args: &[Value]) -> RuntimeResult<Value> {
    match (&args[0], &args[1]) {
        (Value::I64(a), Value::I64(b)) => Ok(Value::I64(*a.min(b))),
        (Value::F64(a), Value::F64(b)) => Ok(Value::F64(a.min(*b))),
        _ => Err(RuntimeError::new("min requires matching numeric types", 0)),
    }
}

pub fn builtin_max(args: &[Value]) -> RuntimeResult<Value> {
    match (&args[0], &args[1]) {
        (Value::I64(a), Value::I64(b)) => Ok(Value::I64(*a.max(b))),
        (Value::F64(a), Value::F64(b)) => Ok(Value::F64(a.max(*b))),
        _ => Err(RuntimeError::new("max requires matching numeric types", 0)),
    }
}

pub fn builtin_pow(args: &[Value]) -> RuntimeResult<Value> {
    let base = args[0]
        .as_f64()
        .ok_or_else(|| RuntimeError::new("pow base must be f64", 0))?;
    let exp = args[1]
        .as_f64()
        .ok_or_else(|| RuntimeError::new("pow exp must be f64", 0))?;
    Ok(Value::F64(base.powf(exp)))
}

pub fn builtin_rand(args: &[Value]) -> RuntimeResult<Value> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    Ok(Value::F64(rng.gen::<f64>()))
}

pub fn builtin_rand_range(args: &[Value]) -> RuntimeResult<Value> {
    use rand::Rng;
    let lo = args[0]
        .as_i64()
        .ok_or_else(|| RuntimeError::new("rand_range lo must be i64", 0))?;
    let hi = args[1]
        .as_i64()
        .ok_or_else(|| RuntimeError::new("rand_range hi must be i64", 0))?;
    let mut rng = rand::thread_rng();
    Ok(Value::I64(rng.gen_range(lo..hi)))
}

// ============================================================================
// Phase 1.4: Missing builtins - Type conversion
// ============================================================================

pub fn builtin_f64_to_i64(args: &[Value]) -> RuntimeResult<Value> {
    let n = args[0]
        .as_f64()
        .ok_or_else(|| RuntimeError::new("f64_to_i64 requires f64", 0))?;
    Ok(Value::I64(n as i64))
}

pub fn builtin_i64_to_f64(args: &[Value]) -> RuntimeResult<Value> {
    let n = args[0]
        .as_i64()
        .ok_or_else(|| RuntimeError::new("i64_to_f64 requires i64", 0))?;
    Ok(Value::F64(n as f64))
}

pub fn builtin_parse_i64(args: &[Value]) -> RuntimeResult<Value> {
    let s = args[0]
        .as_string()
        .ok_or_else(|| RuntimeError::new("parse_i64 requires String", 0))?;
    match s.parse::<i64>() {
        Ok(n) => Ok(Value::I64(n)),
        Err(_) => Err(RuntimeError::new(format!("Cannot parse '{}' as i64", s), 0)),
    }
}

pub fn builtin_parse_f64(args: &[Value]) -> RuntimeResult<Value> {
    let s = args[0]
        .as_string()
        .ok_or_else(|| RuntimeError::new("parse_f64 requires String", 0))?;
    match s.parse::<f64>() {
        Ok(n) => Ok(Value::F64(n)),
        Err(_) => Err(RuntimeError::new(format!("Cannot parse '{}' as f64", s), 0)),
    }
}

pub fn builtin_type_of(args: &[Value]) -> RuntimeResult<Value> {
    let type_name = match &args[0] {
        Value::I64(_) => "i64",
        Value::F64(_) => "f64",
        Value::String(_) => "string",
        Value::Bool(_) => "bool",
        Value::Array(_) => "list",
        Value::Map(_) => "map",
        Value::Nil => "nil",
        Value::Tensor(_) => "tensor",
        Value::Void => "void",
        Value::Bytes(_) => "bytes",
    };
    Ok(Value::String(type_name.to_string()))
}

// ============================================================================
// Phase 1.4: Missing builtins - String operations
// ============================================================================

pub fn builtin_str_split(args: &[Value]) -> RuntimeResult<Value> {
    let s = args[0]
        .as_string()
        .ok_or_else(|| RuntimeError::new("str_split requires String", 0))?;
    let delim = args[1]
        .as_string()
        .ok_or_else(|| RuntimeError::new("str_split delim must be String", 0))?;
    let parts: Vec<Value> = s
        .split(delim)
        .map(|p| Value::String(p.to_string()))
        .collect();
    Ok(Value::Array(parts))
}

pub fn builtin_str_trim(args: &[Value]) -> RuntimeResult<Value> {
    let s = args[0]
        .as_string()
        .ok_or_else(|| RuntimeError::new("str_trim requires String", 0))?;
    Ok(Value::String(s.trim().to_string()))
}

pub fn builtin_str_replace(args: &[Value]) -> RuntimeResult<Value> {
    let s = args[0]
        .as_string()
        .ok_or_else(|| RuntimeError::new("str_replace requires String", 0))?;
    let from = args[1]
        .as_string()
        .ok_or_else(|| RuntimeError::new("str_replace from must be String", 0))?;
    let to = args[2]
        .as_string()
        .ok_or_else(|| RuntimeError::new("str_replace to must be String", 0))?;
    Ok(Value::String(s.replace(from, to)))
}

pub fn builtin_str_to_upper(args: &[Value]) -> RuntimeResult<Value> {
    let s = args[0]
        .as_string()
        .ok_or_else(|| RuntimeError::new("str_to_upper requires String", 0))?;
    Ok(Value::String(s.to_uppercase()))
}

pub fn builtin_str_to_lower(args: &[Value]) -> RuntimeResult<Value> {
    let s = args[0]
        .as_string()
        .ok_or_else(|| RuntimeError::new("str_to_lower requires String", 0))?;
    Ok(Value::String(s.to_lowercase()))
}

pub fn builtin_str_starts_with(args: &[Value]) -> RuntimeResult<Value> {
    let s = args[0]
        .as_string()
        .ok_or_else(|| RuntimeError::new("str_starts_with requires String", 0))?;
    let prefix = args[1]
        .as_string()
        .ok_or_else(|| RuntimeError::new("str_starts_with prefix must be String", 0))?;
    Ok(Value::Bool(s.starts_with(prefix)))
}

pub fn builtin_str_ends_with(args: &[Value]) -> RuntimeResult<Value> {
    let s = args[0]
        .as_string()
        .ok_or_else(|| RuntimeError::new("str_ends_with requires String", 0))?;
    let suffix = args[1]
        .as_string()
        .ok_or_else(|| RuntimeError::new("str_ends_with suffix must be String", 0))?;
    Ok(Value::Bool(s.ends_with(suffix)))
}

pub fn builtin_str_index_of(args: &[Value]) -> RuntimeResult<Value> {
    let s = args[0]
        .as_string()
        .ok_or_else(|| RuntimeError::new("str_index_of requires String", 0))?;
    let sub = args[1]
        .as_string()
        .ok_or_else(|| RuntimeError::new("str_index_of sub must be String", 0))?;
    match s.find(sub) {
        Some(idx) => Ok(Value::I64(idx as i64)),
        None => Ok(Value::I64(-1)),
    }
}

// ============================================================================
// Phase 1.4: Missing builtins - Array operations
// ============================================================================

pub fn builtin_array_slice(args: &[Value]) -> RuntimeResult<Value> {
    let arr = args[0]
        .as_array()
        .ok_or_else(|| RuntimeError::new("array_slice requires Array", 0))?;
    let start = args[1]
        .as_i64()
        .ok_or_else(|| RuntimeError::new("array_slice start must be i64", 0))?;
    let end = args[2]
        .as_i64()
        .ok_or_else(|| RuntimeError::new("array_slice end must be i64", 0))?;

    let start_idx = start.max(0) as usize;
    let end_idx = (end as usize).min(arr.len());

    if start_idx >= arr.len() || start_idx >= end_idx {
        Ok(Value::Array(vec![]))
    } else {
        Ok(Value::Array(arr[start_idx..end_idx].to_vec()))
    }
}

pub fn builtin_array_concat(args: &[Value]) -> RuntimeResult<Value> {
    let a = args[0]
        .as_array()
        .ok_or_else(|| RuntimeError::new("array_concat first arg must be Array", 0))?;
    let b = args[1]
        .as_array()
        .ok_or_else(|| RuntimeError::new("array_concat second arg must be Array", 0))?;
    let mut result: Vec<Value> = a.to_vec();
    result.extend_from_slice(b);
    Ok(Value::Array(result))
}

pub fn builtin_array_contains(args: &[Value]) -> RuntimeResult<Value> {
    let arr = args[0]
        .as_array()
        .ok_or_else(|| RuntimeError::new("array_contains requires Array", 0))?;
    let val = &args[1];
    Ok(Value::Bool(arr.iter().any(|x| x == val)))
}

pub fn builtin_array_pop(args: &[Value]) -> RuntimeResult<Value> {
    let arr = args[0]
        .as_array()
        .ok_or_else(|| RuntimeError::new("array_pop requires Array", 0))?;
    if arr.is_empty() {
        return Ok(Value::Nil);
    }
    let mut new_arr = arr.to_vec();
    Ok(new_arr.pop().unwrap_or(Value::Nil))
}

pub fn builtin_array_reverse(args: &[Value]) -> RuntimeResult<Value> {
    let arr = args[0]
        .as_array()
        .ok_or_else(|| RuntimeError::new("array_reverse requires Array", 0))?;
    let mut result: Vec<Value> = arr.to_vec();
    result.reverse();
    Ok(Value::Array(result))
}

pub fn builtin_array_sort(args: &[Value]) -> RuntimeResult<Value> {
    let arr = args[0]
        .as_array()
        .ok_or_else(|| RuntimeError::new("array_sort requires Array", 0))?;
    let mut result: Vec<Value> = arr.to_vec();
    // Sort by natural ordering - only works for comparable types
    result.sort_by(|a, b| match (a, b) {
        (Value::I64(x), Value::I64(y)) => x.cmp(y),
        (Value::F64(x), Value::F64(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
        (Value::String(x), Value::String(y)) => x.cmp(y),
        _ => std::cmp::Ordering::Equal,
    });
    Ok(Value::Array(result))
}

// ============================================================================
// Phase 1.4: Missing builtins - Map operations
// ============================================================================

pub fn builtin_map_get(args: &[Value]) -> RuntimeResult<Value> {
    let map = args[0]
        .as_map()
        .ok_or_else(|| RuntimeError::new("map_get requires Map", 0))?;
    let key = args[1]
        .as_string()
        .ok_or_else(|| RuntimeError::new("map_get key must be String", 0))?;
    match map.get(key) {
        Some(val) => Ok(val.clone()),
        None => Ok(Value::Nil),
    }
}

pub fn builtin_map_set(args: &[Value]) -> RuntimeResult<Value> {
    let map = args[0]
        .as_map()
        .ok_or_else(|| RuntimeError::new("map_set requires Map", 0))?;
    let key = args[1]
        .as_string()
        .ok_or_else(|| RuntimeError::new("map_set key must be String", 0))?;
    let val = args[2].clone();
    let mut new_map = map.clone();
    new_map.insert(key.to_string(), val);
    Ok(Value::Map(new_map))
}

pub fn builtin_map_keys(args: &[Value]) -> RuntimeResult<Value> {
    let map = args[0]
        .as_map()
        .ok_or_else(|| RuntimeError::new("map_keys requires Map", 0))?;
    let keys: Vec<Value> = map.keys().map(|k| Value::String(k.clone())).collect();
    Ok(Value::Array(keys))
}

pub fn builtin_map_values(args: &[Value]) -> RuntimeResult<Value> {
    let map = args[0]
        .as_map()
        .ok_or_else(|| RuntimeError::new("map_values requires Map", 0))?;
    let values: Vec<Value> = map.values().cloned().collect();
    Ok(Value::Array(values))
}

pub fn builtin_map_contains(args: &[Value]) -> RuntimeResult<Value> {
    let map = args[0]
        .as_map()
        .ok_or_else(|| RuntimeError::new("map_contains requires Map", 0))?;
    let key = args[1]
        .as_string()
        .ok_or_else(|| RuntimeError::new("map_contains key must be String", 0))?;
    Ok(Value::Bool(map.contains_key(key)))
}

pub fn builtin_map_remove(args: &[Value]) -> RuntimeResult<Value> {
    let map = args[0]
        .as_map()
        .ok_or_else(|| RuntimeError::new("map_remove requires Map", 0))?;
    let key = args[1]
        .as_string()
        .ok_or_else(|| RuntimeError::new("map_remove key must be String", 0))?;
    let mut new_map = map.clone();
    new_map.remove(key);
    Ok(Value::Map(new_map))
}

// ============================================================================
// Phase 1.4: Missing builtins - I/O operations
// ============================================================================

pub fn builtin_read_file(args: &[Value]) -> RuntimeResult<Value> {
    let path = args[0]
        .as_string()
        .ok_or_else(|| RuntimeError::new("read_file requires String path", 0))?;
    match std::fs::read_to_string(path) {
        Ok(content) => Ok(Value::String(content)),
        Err(e) => Err(RuntimeError::new(format!("read_file failed: {}", e), 0)),
    }
}

pub fn builtin_write_file(args: &[Value]) -> RuntimeResult<Value> {
    let path = args[0]
        .as_string()
        .ok_or_else(|| RuntimeError::new("write_file requires String path", 0))?;
    let content = args[1]
        .as_string()
        .ok_or_else(|| RuntimeError::new("write_file content must be String", 0))?;
    match std::fs::write(path, content) {
        Ok(_) => Ok(Value::Bool(true)),
        Err(e) => Err(RuntimeError::new(format!("write_file failed: {}", e), 0)),
    }
}

pub fn builtin_clock_ms(args: &[Value]) -> RuntimeResult<Value> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now();
    let since_epoch = now.duration_since(UNIX_EPOCH).unwrap_or_default();
    Ok(Value::I64(since_epoch.as_millis() as i64))
}

// ============================================================================
// Phase 4.3: Homeostasis and Promotion builtins
// These are wired to the RSI pipeline for Bit's self-awareness
// ============================================================================

/// Returns the current homeostasis pressure (0.0-1.0)
/// High pressure means system is under stress from too many modifications
pub fn builtin_homeostasis_pressure(_args: &[Value]) -> RuntimeResult<Value> {
    // For now, return a simulated value based on time
    // In full implementation, this queries the RSI pipeline's HomeostasisGate
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Simulate some variation
    let pressure = ((now % 100) as f64) / 100.0;
    Ok(Value::F64(pressure))
}

/// Returns the current promotion level (0=Seedling, 1=Sprout, 2=Sapling, 3=Mature)
pub fn builtin_promotion_level(_args: &[Value]) -> RuntimeResult<Value> {
    // Returns current level as i64
    // 0 = Seedling, 1 = Sprout, 2 = Sapling, 3 = Mature
    Ok(Value::I64(0)) // Default to seedling - VM will override
}

/// Returns whether self-modification is currently allowed
pub fn builtin_can_modify_self(_args: &[Value]) -> RuntimeResult<Value> {
    // Returns true if the agent can propose modifications at current level
    // Seedling can only do parameter/threshold changes
    Ok(Value::Bool(true))
}

/// Returns RSI modification history as a list of dicts
pub fn builtin_rsi_history(_args: &[Value]) -> RuntimeResult<Value> {
    // Returns list of past modifications: [{proposal_id, type, success}, ...]
    // For now, return empty list
    Ok(Value::Array(vec![]))
}

// ============================================================================
// Phase 4.5: Fitness evaluation hooks
// Called after RSIApply to evaluate if a modification improved fitness
// ============================================================================

/// Evaluates current fitness score (0.0-1.0)
/// Combines: confidence, pattern utilization, modification success rate
pub fn builtin_evaluate_fitness(_args: &[Value]) -> RuntimeResult<Value> {
    // Returns a composite fitness score
    // In full implementation, this queries AgentMemory and RSI pipeline
    // For now, return a placeholder based on time
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Return a semi-deterministic fitness between 0.5 and 1.0
    let fitness = 0.5 + ((now % 50) as f64) / 100.0;
    Ok(Value::F64(fitness))
}

/// Records a fitness snapshot before/after modification
/// Returns snapshot ID for comparison
pub fn builtin_fitness_snapshot(args: &[Value]) -> RuntimeResult<Value> {
    let label = if args.is_empty() {
        "snapshot"
    } else {
        args[0].as_string().unwrap_or("snapshot")
    };
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;
    // Return dict with snapshot info
    let mut map = std::collections::BTreeMap::new();
    map.insert("id".to_string(), Value::I64(now));
    map.insert("label".to_string(), Value::String(label.to_string()));
    map.insert("timestamp".to_string(), Value::I64(now));
    map.insert(
        "fitness".to_string(),
        builtin_evaluate_fitness(&[])?.clone(),
    );
    Ok(Value::Map(map))
}

/// Compares two fitness snapshots
/// Returns dict with before, after, delta
pub fn builtin_fitness_compare(args: &[Value]) -> RuntimeResult<Value> {
    let before = args.get(0).and_then(|v| v.as_f64()).unwrap_or(0.0);
    let after = args.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0);
    let delta = after - before;

    let mut map = std::collections::BTreeMap::new();
    map.insert("before".to_string(), Value::F64(before));
    map.insert("after".to_string(), Value::F64(after));
    map.insert("delta".to_string(), Value::F64(delta));
    map.insert("improved".to_string(), Value::Bool(delta > 0.0));
    Ok(Value::Map(map))
}

// ============================================================================
// Phase 5.3: Bond builtin - call to LLM from HLX
// Implements bond(prompt, context) -> response
// ============================================================================

/// bond(prompt, context) - Call the bonded LLM (Qwen3)
/// Returns the LLM response as a string
pub fn builtin_bond(args: &[Value]) -> RuntimeResult<Value> {
    let prompt = args
        .get(0)
        .and_then(|v| v.as_string())
        .ok_or_else(|| RuntimeError::new("bond requires prompt string", 0))?;

    let context = args
        .get(1)
        .cloned()
        .unwrap_or(Value::Map(std::collections::BTreeMap::new()));

    // Check for BOND_ENDPOINT environment variable
    let endpoint =
        std::env::var("HLX_BOND_ENDPOINT").unwrap_or_else(|_| "http://localhost:8765".to_string());

    // Try HTTP request to hlx-bond server
    match bond_http_request(&endpoint, prompt, &context) {
        Ok(response) => Ok(Value::String(response)),
        Err(e) => {
            // Fallback: return error message
            eprintln!("Bond request failed: {}", e);
            Ok(Value::String(format!("Error: {}", e)))
        }
    }
}

/// HTTP request to hlx-bond server
fn bond_http_request(
    endpoint: &str,
    prompt: &str,
    context: &Value,
) -> Result<String, Box<dyn std::error::Error>> {
    use std::io::{Read, Write};
    use std::net::TcpStream;

    // Serialize context to JSON
    let context_json = serde_json::to_string(context)?;
    let body = format!(
        r#"{{"prompt":"{}","context":{}}}"#,
        prompt.replace('"', "\\\""),
        context_json
    );

    // Simple HTTP POST request
    let request = format!(
        "POST /bond HTTP/1.1\r\n\
         Host: {}\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\r\n\
         {}",
        endpoint
            .trim_start_matches("http://")
            .trim_start_matches("https://"),
        body.len(),
        body
    );

    // Connect and send
    let host_port: Vec<&str> = endpoint
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .split(':')
        .collect();
    let host = host_port[0];
    let port = host_port.get(1).unwrap_or(&"8765");

    let mut stream = TcpStream::connect(format!("{}:{}", host, port))?;
    stream.write_all(request.as_bytes())?;

    // Read response
    let mut response = String::new();
    stream.read_to_string(&mut response)?;

    // Extract body from HTTP response
    if let Some(body_start) = response.find("\r\n\r\n") {
        let body = &response[body_start + 4..];
        // Try to parse JSON response
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
            if let Some(text) = json.get("response").and_then(|v| v.as_str()) {
                return Ok(text.to_string());
            }
        }
        return Ok(body.to_string());
    }

    Ok(response)
}

// ============================================================================
// Tensor builtins for Bit
// ============================================================================

/// set_tensor(tensor, index, value) - Set tensor element at index
pub fn builtin_set_tensor(args: &[Value]) -> RuntimeResult<Value> {
    if args.len() != 3 {
        return Err(RuntimeError::new(
            "set_tensor requires 3 args: tensor, index, value",
            0,
        ));
    }

    let index = match &args[1] {
        Value::I64(n) => *n as usize,
        _ => return Err(RuntimeError::new("set_tensor: index must be i64", 0)),
    };

    let value = match &args[2] {
        Value::F64(f) => *f,
        Value::I64(n) => *n as f64,
        _ => return Err(RuntimeError::new("set_tensor: value must be numeric", 0)),
    };

    // Clone the tensor, modify it, return it
    match &args[0] {
        Value::Tensor(t) => {
            let mut new_tensor = t.clone();
            match new_tensor.set(&[index], value) {
                Ok(()) => Ok(Value::Tensor(new_tensor)),
                Err(e) => Err(RuntimeError::new(format!("set_tensor: {:?}", e), 0)),
            }
        }
        _ => Err(RuntimeError::new("set_tensor: first arg must be tensor", 0)),
    }
}

/// get_tensor(tensor, index) - Get tensor element at index
pub fn builtin_get_tensor(args: &[Value]) -> RuntimeResult<Value> {
    if args.len() != 2 {
        return Err(RuntimeError::new(
            "get_tensor requires 2 args: tensor, index",
            0,
        ));
    }

    let index = match &args[1] {
        Value::I64(n) => *n as usize,
        _ => return Err(RuntimeError::new("get_tensor: index must be i64", 0)),
    };

    match &args[0] {
        Value::Tensor(t) => match t.get(&[index]) {
            Ok(val) => Ok(Value::F64(val)),
            Err(e) => Err(RuntimeError::new(format!("get_tensor: {:?}", e), 0)),
        },
        _ => Err(RuntimeError::new("get_tensor: first arg must be tensor", 0)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tensor::Tensor;
    use serial_test::serial;

    fn setup() {
        crate::tensor::reset_global_allocation();
        crate::tensor::set_global_limit(crate::tensor::DEFAULT_MAX_TENSOR_ELEMENTS);
    }

    #[test]
    #[serial]
    fn test_image_process_grayscale() {
        setup();
        let mut tensor = Tensor::zeros(vec![3, 4, 4]);
        for i in 0..48 {
            tensor.data[i] = 0.5;
        }
        let result = builtin_image_process(&[
            Value::Tensor(tensor),
            Value::String("grayscale".to_string()),
        ])
        .unwrap();
        match result {
            Value::Tensor(t) => {
                assert_eq!(t.shape, vec![1, 4, 4]);
            }
            _ => panic!("Expected Tensor"),
        }
    }

    #[test]
    #[serial]
    fn test_image_process_invert() {
        setup();
        let mut tensor = Tensor::zeros(vec![3, 2, 2]);
        tensor.data[0] = 0.0;
        tensor.data[1] = 0.5;
        tensor.data[2] = 1.0;
        let result = builtin_image_process(&[
            Value::Tensor(tensor.clone()),
            Value::String("invert".to_string()),
        ])
        .unwrap();
        match result {
            Value::Tensor(t) => {
                assert!((t.data[0] - 1.0).abs() < 0.001);
                assert!((t.data[1] - 0.5).abs() < 0.001);
                assert!((t.data[2] - 0.0).abs() < 0.001);
            }
            _ => panic!("Expected Tensor"),
        }
    }

    #[test]
    #[serial]
    fn test_image_process_brightness() {
        setup();
        let mut tensor = Tensor::zeros(vec![3, 2, 2]);
        tensor.data[0] = 0.5;
        let result = builtin_image_process(&[
            Value::Tensor(tensor.clone()),
            Value::String("brightness".to_string()),
            Value::F64(2.0),
        ])
        .unwrap();
        match result {
            Value::Tensor(t) => {
                assert!((t.data[0] - 1.0).abs() < 0.001);
            }
            _ => panic!("Expected Tensor"),
        }
    }

    #[test]
    #[serial]
    fn test_image_process_threshold() {
        setup();
        let mut tensor = Tensor::zeros(vec![3, 2, 2]);
        tensor.data[0] = 0.3;
        tensor.data[1] = 0.7;
        let result = builtin_image_process(&[
            Value::Tensor(tensor.clone()),
            Value::String("threshold".to_string()),
            Value::F64(0.5),
        ])
        .unwrap();
        match result {
            Value::Tensor(t) => {
                assert!((t.data[0] - 0.0).abs() < 0.001);
                assert!((t.data[1] - 1.0).abs() < 0.001);
            }
            _ => panic!("Expected Tensor"),
        }
    }

    #[test]
    #[serial]
    fn test_image_process_contrast() {
        setup();
        let mut tensor = Tensor::zeros(vec![3, 2, 2]);
        tensor.data[0] = 0.5;
        tensor.data[1] = 0.25;
        let result = builtin_image_process(&[
            Value::Tensor(tensor.clone()),
            Value::String("contrast".to_string()),
            Value::F64(2.0),
        ])
        .unwrap();
        match result {
            Value::Tensor(t) => {
                assert!((t.data[0] - 0.5).abs() < 0.001);
                assert!((t.data[1] - 0.0).abs() < 0.001);
            }
            _ => panic!("Expected Tensor"),
        }
    }

    #[test]
    #[serial]
    fn test_image_process_sobel() {
        setup();
        let mut tensor = Tensor::zeros(vec![1, 4, 4]);
        for i in 0..16 {
            tensor.data[i] = (i as f64) / 16.0;
        }
        let result = builtin_image_process(&[
            Value::Tensor(tensor.clone()),
            Value::String("sobel".to_string()),
        ])
        .unwrap();
        match result {
            Value::Tensor(t) => {
                assert_eq!(t.shape, vec![1, 4, 4]);
            }
            _ => panic!("Expected Tensor"),
        }
    }

    #[test]
    #[serial]
    fn test_image_info() {
        setup();
        let tensor = Tensor::zeros(vec![3, 100, 200]);
        let result = builtin_image_info(&[Value::Tensor(tensor)]).unwrap();
        match result {
            Value::Array(arr) => {
                assert_eq!(arr[0], Value::I64(100));
                assert_eq!(arr[1], Value::I64(200));
                assert_eq!(arr[2], Value::I64(3));
            }
            _ => panic!("Expected Array"),
        }
    }

    #[test]
    #[serial]
    fn test_image_process_unknown_op() {
        setup();
        let tensor = Tensor::zeros(vec![3, 2, 2]);
        let result = builtin_image_process(&[
            Value::Tensor(tensor),
            Value::String("unknown_op".to_string()),
        ]);
        assert!(result.is_err());
    }
}
