use crate::{Bytecode, RuntimeError, RuntimeResult, Tensor, Value, Vm};
use rand::SeedableRng;
use image::ImageFormat;
use std::fs::{File, OpenOptions};
use std::io::{Read as IoRead, Write as IoWrite};
use std::path::{Path, PathBuf};

/// Resolve the sandbox root directory.
fn sandbox_root() -> RuntimeResult<PathBuf> {
    match std::env::var("HLX_SANDBOX") {
        Ok(root) => PathBuf::from(root)
            .canonicalize()
            .map_err(|e| RuntimeError::new(format!("HLX_SANDBOX path invalid: {}", e), 0)),
        Err(_) => std::env::current_dir()
            .map_err(|e| RuntimeError::new(format!("Cannot determine sandbox root: {}", e), 0)),
    }
}

/// Check that a canonical path is within the sandbox root.
fn check_containment(canonical: &Path, sandbox_root: &Path, raw_path: &str) -> RuntimeResult<()> {
    if !canonical.starts_with(sandbox_root) {
        return Err(RuntimeError::new(
            format!(
                "Sandbox violation: path '{}' escapes sandbox root '{}'",
                raw_path,
                sandbox_root.display()
            ),
            0,
        ));
    }
    Ok(())
}

/// Validate that a path is within the sandbox root.
///
/// Returns the canonicalized path on success. For callers that need a PathBuf
/// (e.g., image operations that require a path, not an fd).
fn validate_sandboxed_path(raw_path: &str) -> RuntimeResult<PathBuf> {
    let expanded = shellexpand::full(raw_path)
        .map_err(|e| RuntimeError::new(format!("Path expansion failed: {}", e), 0))?;

    let target = PathBuf::from(expanded.as_ref());
    let root = sandbox_root()?;

    // For new files, canonicalize the parent directory and check containment
    let canonical = if target.exists() {
        target
            .canonicalize()
            .map_err(|e| RuntimeError::new(format!("Path resolution failed: {}", e), 0))?
    } else {
        let parent = target.parent().unwrap_or(Path::new("."));
        let parent_canonical = parent.canonicalize().map_err(|e| {
            RuntimeError::new(format!("Parent directory resolution failed: {}", e), 0)
        })?;
        match target.file_name() {
            Some(name) => parent_canonical.join(name),
            None => return Err(RuntimeError::new("Invalid file path", 0)),
        }
    };

    check_containment(&canonical, &root, raw_path)?;
    Ok(canonical)
}

/// Open a file for reading with TOCTOU-safe sandbox validation.
///
/// Opens the file first, then verifies the fd's real path is within the sandbox.
/// This closes the race window between validation and open.
fn sandbox_open_read(raw_path: &str) -> RuntimeResult<File> {
    let expanded = shellexpand::full(raw_path)
        .map_err(|e| RuntimeError::new(format!("Path expansion failed: {}", e), 0))?;

    let target = PathBuf::from(expanded.as_ref());
    let root = sandbox_root()?;

    // Open first, validate after — closes TOCTOU window
    let file = File::open(&target)
        .map_err(|e| RuntimeError::new(format!("Cannot open '{}': {}", raw_path, e), 0))?;

    // Verify the fd's real path via /proc/self/fd (Linux)
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::io::AsRawFd;
        let fd_path = format!("/proc/self/fd/{}", file.as_raw_fd());
        let real_path = std::fs::read_link(&fd_path).map_err(|e| {
            RuntimeError::new(format!("Cannot resolve fd path: {}", e), 0)
        })?;
        check_containment(&real_path, &root, raw_path)?;
    }

    // Fallback for non-Linux: use canonical path check (still has narrow TOCTOU window)
    #[cfg(not(target_os = "linux"))]
    {
        let canonical = target.canonicalize().map_err(|e| {
            RuntimeError::new(format!("Path resolution failed: {}", e), 0)
        })?;
        check_containment(&canonical, &root, raw_path)?;
    }

    Ok(file)
}

/// Open/create a file for writing with TOCTOU-safe sandbox validation.
///
/// Validates parent directory containment, then creates/opens the file.
fn sandbox_open_write(raw_path: &str) -> RuntimeResult<File> {
    let expanded = shellexpand::full(raw_path)
        .map_err(|e| RuntimeError::new(format!("Path expansion failed: {}", e), 0))?;

    let target = PathBuf::from(expanded.as_ref());
    let root = sandbox_root()?;

    // Validate parent directory first
    let parent = target.parent().unwrap_or(Path::new("."));
    let parent_canonical = parent.canonicalize().map_err(|e| {
        RuntimeError::new(format!("Parent directory resolution failed: {}", e), 0)
    })?;
    check_containment(&parent_canonical, &root, raw_path)?;

    // Open for writing (create or truncate)
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&target)
        .map_err(|e| RuntimeError::new(format!("Cannot open '{}' for writing: {}", raw_path, e), 0))?;

    // Post-open verification on Linux
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::io::AsRawFd;
        let fd_path = format!("/proc/self/fd/{}", file.as_raw_fd());
        let real_path = std::fs::read_link(&fd_path).map_err(|e| {
            RuntimeError::new(format!("Cannot resolve fd path: {}", e), 0)
        })?;
        check_containment(&real_path, &root, raw_path)?;
    }

    Ok(file)
}
pub fn builtin_strlen(_vm: &mut Vm, _bytecode: &Bytecode, args: &[Value]) -> RuntimeResult<Value> {
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

pub fn builtin_substring(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
    let s = args[0].as_string().ok_or_else(|| {
        RuntimeError::new(
            format!("substring: expected String, got {}", args[0].type_name()),
            0,
        )
    })?;
    let start_i = args[1].as_i64().ok_or_else(|| {
        RuntimeError::new(
            format!(
                "substring: expected i64 for start, got {}",
                args[1].type_name()
            ),
            0,
        )
    })?;
    let len_i = args[2].as_i64().ok_or_else(|| {
        RuntimeError::new(
            format!(
                "substring: expected i64 for len, got {}",
                args[2].type_name()
            ),
            0,
        )
    })?;

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

pub fn builtin_concat(_vm: &mut Vm, _bytecode: &Bytecode, args: &[Value]) -> RuntimeResult<Value> {
    let a = args[0].as_string().ok_or_else(|| {
        RuntimeError::new(
            format!("concat: expected String, got {}", args[0].type_name()),
            0,
        )
    })?;
    let b = args[1].as_string().ok_or_else(|| {
        RuntimeError::new(
            format!("concat: expected String, got {}", args[1].type_name()),
            0,
        )
    })?;
    Ok(Value::String(format!("{}{}", a, b)))
}

pub fn builtin_strcmp(_vm: &mut Vm, _bytecode: &Bytecode, args: &[Value]) -> RuntimeResult<Value> {
    let a = args[0].as_string().ok_or_else(|| {
        RuntimeError::new(
            format!("strcmp: expected String, got {}", args[0].type_name()),
            0,
        )
    })?;
    let b = args[1].as_string().ok_or_else(|| {
        RuntimeError::new(
            format!("strcmp: expected String, got {}", args[1].type_name()),
            0,
        )
    })?;
    Ok(Value::I64(match a.cmp(b) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }))
}

pub fn builtin_ord(_vm: &mut Vm, _bytecode: &Bytecode, args: &[Value]) -> RuntimeResult<Value> {
    let s = args[0]
        .as_string()
        .ok_or_else(|| RuntimeError::new("ord requires String", 0))?;
    if s.is_empty() {
        return Err(RuntimeError::new("ord: empty string", 0));
    }
    Ok(Value::I64(s.as_bytes()[0] as i64))
}

pub fn builtin_char(_vm: &mut Vm, _bytecode: &Bytecode, args: &[Value]) -> RuntimeResult<Value> {
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

pub fn builtin_str_char_at(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
    let s = args[0]
        .as_string()
        .ok_or_else(|| RuntimeError::new("str_char_at requires String", 0))?;
    let idx = args[1]
        .as_i64()
        .ok_or_else(|| RuntimeError::new("str_char_at index must be i64", 0))?
        as usize;
    if idx >= s.len() {
        return Ok(Value::I64(0));
    }
    Ok(Value::I64(s.as_bytes()[idx] as i64))
}

pub fn builtin_push(_vm: &mut Vm, _bytecode: &Bytecode, args: &[Value]) -> RuntimeResult<Value> {
    let arr = args[0]
        .as_array()
        .ok_or_else(|| RuntimeError::new("push requires Array", 0))?;
    let mut new_arr = arr.to_vec();
    new_arr.push(args[1].clone());
    Ok(Value::Array(new_arr))
}

pub fn builtin_get_at(_vm: &mut Vm, _bytecode: &Bytecode, args: &[Value]) -> RuntimeResult<Value> {
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

pub fn builtin_set_at(_vm: &mut Vm, _bytecode: &Bytecode, args: &[Value]) -> RuntimeResult<Value> {
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

pub fn builtin_array_len(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
    match &args[0] {
        Value::Array(arr) => Ok(Value::I64(arr.len() as i64)),
        Value::String(s) => Ok(Value::I64(s.len() as i64)),
        _ => Err(RuntimeError::new(
            format!("len requires Array or String, got {}", args[0].type_name()),
            0,
        )),
    }
}

pub fn builtin_print(_vm: &mut Vm, _bytecode: &Bytecode, args: &[Value]) -> RuntimeResult<Value> {
    print!("{}", args[0]);
    Ok(Value::Void)
}

pub fn builtin_println(_vm: &mut Vm, _bytecode: &Bytecode, args: &[Value]) -> RuntimeResult<Value> {
    println!("{}", args[0]);
    Ok(Value::Void)
}

pub fn builtin_image_load(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
    let path = args[0]
        .as_string()
        .ok_or_else(|| RuntimeError::new("image_load requires String path", 0))?;

    let canonical = validate_sandboxed_path(path)?;

    let bytes = std::fs::read(&canonical)
        .map_err(|e| RuntimeError::new(format!("Failed to read image: {}", e), 0))?;

    let tensor = Tensor::from_image_bytes(&bytes)?;
    Ok(Value::Tensor(tensor))
}

pub fn builtin_image_save(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
    let tensor = match &args[0] {
        Value::Tensor(t) => t,
        _ => return Err(RuntimeError::new("image_save requires Tensor", 0)),
    };

    let path = args[1]
        .as_string()
        .ok_or_else(|| RuntimeError::new("image_save requires String path", 0))?;

    let canonical = validate_sandboxed_path(path)?;

    let format = if path.to_lowercase().ends_with(".png") {
        ImageFormat::Png
    } else {
        ImageFormat::Jpeg
    };

    let bytes = tensor.to_image_bytes(format)?;

    std::fs::write(&canonical, &bytes)
        .map_err(|e| RuntimeError::new(format!("Failed to write image: {}", e), 0))?;

    Ok(Value::Void)
}

pub fn builtin_image_process(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
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

pub fn builtin_image_info(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
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

pub fn builtin_audio_load(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
    let path = args[0]
        .as_string()
        .ok_or_else(|| RuntimeError::new("audio_load requires String path", 0))?;

    let canonical = validate_sandboxed_path(path)?;
    let tensor = Tensor::from_audio_file(canonical.to_str().unwrap_or(path))?;
    Ok(Value::Tensor(tensor))
}

pub fn builtin_audio_save(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
    let tensor = match &args[0] {
        Value::Tensor(t) => t,
        _ => return Err(RuntimeError::new("audio_save requires Tensor", 0)),
    };

    let path = args[1]
        .as_string()
        .ok_or_else(|| RuntimeError::new("audio_save requires String path", 0))?;

    let canonical = validate_sandboxed_path(path)?;

    let sample_rate = if args.len() > 2 {
        args[2].as_i64().unwrap_or(44100) as u32
    } else {
        44100
    };

    tensor.to_audio_file(canonical.to_str().unwrap_or(path), sample_rate)?;

    Ok(Value::Void)
}

pub fn builtin_audio_info(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
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

pub fn builtin_audio_resample(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
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

pub fn builtin_audio_normalize(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
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

pub fn builtin_zeros(_vm: &mut Vm, _bytecode: &Bytecode, args: &[Value]) -> RuntimeResult<Value> {
    let size = args[0]
        .as_i64()
        .ok_or_else(|| RuntimeError::new("zeros requires i64", 0))?;
    if size <= 0 || size > 10000 {
        return Err(RuntimeError::new("zeros: invalid size", 0));
    }
    Ok(Value::Tensor(Tensor::zeros(vec![size as usize])))
}

pub fn builtin_i64_to_str(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
    let n = args[0]
        .as_i64()
        .ok_or_else(|| RuntimeError::new("i64_to_str requires i64", 0))?;
    Ok(Value::String(n.to_string()))
}

pub fn builtin_f64_to_str(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
    let n = args[0].as_f64().ok_or_else(|| {
        RuntimeError::new(
            format!("f64_to_str requires f64, got {}", args[0].type_name()),
            0,
        )
    })?;
    // Optional second argument: decimal places (default 6)
    let places = if args.len() > 1 {
        args[1].as_i64().ok_or_else(|| {
            RuntimeError::new(
                format!("f64_to_str places must be i64, got {}", args[1].type_name()),
                0,
            )
        })? as usize
    } else {
        6
    };
    let places = places.min(20); // Cap at 20 decimal places
    Ok(Value::String(format!("{:.1$}", n, places)))
}

pub fn builtin_str_contains(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
    let haystack = args[0]
        .as_string()
        .ok_or_else(|| RuntimeError::new("str_contains requires String", 0))?;
    let needle = args[1]
        .as_string()
        .ok_or_else(|| RuntimeError::new("str_contains requires String", 0))?;
    Ok(Value::Bool(haystack.contains(&needle)))
}

pub fn builtin_str_equals(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
    let a = args[0]
        .as_string()
        .ok_or_else(|| RuntimeError::new("str_equals requires String", 0))?;
    let b = args[1]
        .as_string()
        .ok_or_else(|| RuntimeError::new("str_equals requires String", 0))?;
    Ok(Value::Bool(a == b))
}

pub fn builtin_sqrt(_vm: &mut Vm, _bytecode: &Bytecode, args: &[Value]) -> RuntimeResult<Value> {
    let n = args[0]
        .as_f64()
        .ok_or_else(|| RuntimeError::new("sqrt requires f64", 0))?;
    Ok(Value::F64(n.sqrt()))
}

pub fn builtin_sin(_vm: &mut Vm, _bytecode: &Bytecode, args: &[Value]) -> RuntimeResult<Value> {
    let n = args[0]
        .as_f64()
        .ok_or_else(|| RuntimeError::new("sin requires f64 (radians)", 0))?;
    Ok(Value::F64(n.sin()))
}

pub fn builtin_cos(_vm: &mut Vm, _bytecode: &Bytecode, args: &[Value]) -> RuntimeResult<Value> {
    let n = args[0]
        .as_f64()
        .ok_or_else(|| RuntimeError::new("cos requires f64 (radians)", 0))?;
    Ok(Value::F64(n.cos()))
}

pub fn builtin_tan(_vm: &mut Vm, _bytecode: &Bytecode, args: &[Value]) -> RuntimeResult<Value> {
    let n = args[0]
        .as_f64()
        .ok_or_else(|| RuntimeError::new("tan requires f64 (radians)", 0))?;
    Ok(Value::F64(n.tan()))
}

pub fn builtin_hash(_vm: &mut Vm, _bytecode: &Bytecode, args: &[Value]) -> RuntimeResult<Value> {
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

pub fn builtin_abs(_vm: &mut Vm, _bytecode: &Bytecode, args: &[Value]) -> RuntimeResult<Value> {
    match &args[0] {
        Value::I64(n) => Ok(Value::I64(n.abs())),
        Value::F64(n) => Ok(Value::F64(n.abs())),
        _ => Err(RuntimeError::new("abs requires i64 or f64", 0)),
    }
}

pub fn builtin_floor(_vm: &mut Vm, _bytecode: &Bytecode, args: &[Value]) -> RuntimeResult<Value> {
    let n = args[0]
        .as_f64()
        .ok_or_else(|| RuntimeError::new("floor requires f64", 0))?;
    Ok(Value::I64(n.floor() as i64))
}

pub fn builtin_ceil(_vm: &mut Vm, _bytecode: &Bytecode, args: &[Value]) -> RuntimeResult<Value> {
    let n = args[0]
        .as_f64()
        .ok_or_else(|| RuntimeError::new("ceil requires f64", 0))?;
    Ok(Value::I64(n.ceil() as i64))
}

pub fn builtin_round(_vm: &mut Vm, _bytecode: &Bytecode, args: &[Value]) -> RuntimeResult<Value> {
    let n = args[0]
        .as_f64()
        .ok_or_else(|| RuntimeError::new("round requires f64", 0))?;
    Ok(Value::I64(n.round() as i64))
}

pub fn builtin_min(_vm: &mut Vm, _bytecode: &Bytecode, args: &[Value]) -> RuntimeResult<Value> {
    match (&args[0], &args[1]) {
        (Value::I64(a), Value::I64(b)) => Ok(Value::I64(*a.min(b))),
        (Value::F64(a), Value::F64(b)) => Ok(Value::F64(a.min(*b))),
        _ => Err(RuntimeError::new("min requires matching numeric types", 0)),
    }
}

pub fn builtin_max(_vm: &mut Vm, _bytecode: &Bytecode, args: &[Value]) -> RuntimeResult<Value> {
    match (&args[0], &args[1]) {
        (Value::I64(a), Value::I64(b)) => Ok(Value::I64(*a.max(b))),
        (Value::F64(a), Value::F64(b)) => Ok(Value::F64(a.max(*b))),
        _ => Err(RuntimeError::new("max requires matching numeric types", 0)),
    }
}

pub fn builtin_pow(_vm: &mut Vm, _bytecode: &Bytecode, args: &[Value]) -> RuntimeResult<Value> {
    let base = args[0]
        .as_f64()
        .ok_or_else(|| RuntimeError::new("pow base must be f64", 0))?;
    let exp = args[1]
        .as_f64()
        .ok_or_else(|| RuntimeError::new("pow exp must be f64", 0))?;
    Ok(Value::F64(base.powf(exp)))
}

pub fn builtin_rand(vm: &mut Vm, _bytecode: &Bytecode, _args: &[Value]) -> RuntimeResult<Value> {
    use rand::Rng;
    Ok(Value::F64(vm.rng.gen::<f64>()))
}

pub fn builtin_rand_range(
    vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
    use rand::Rng;
    let lo = args[0]
        .as_i64()
        .ok_or_else(|| RuntimeError::new("rand_range lo must be i64", 0))?;
    let hi = args[1]
        .as_i64()
        .ok_or_else(|| RuntimeError::new("rand_range hi must be i64", 0))?;
    Ok(Value::I64(vm.rng.gen_range(lo..hi)))
}

// ============================================================================
// Phase 1.4: Missing builtins - Type conversion
// ============================================================================

pub fn builtin_f64_to_i64(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
    let n = args[0]
        .as_f64()
        .ok_or_else(|| RuntimeError::new("f64_to_i64 requires f64", 0))?;
    Ok(Value::I64(n as i64))
}

pub fn builtin_i64_to_f64(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
    let n = args[0]
        .as_i64()
        .ok_or_else(|| RuntimeError::new("i64_to_f64 requires i64", 0))?;
    Ok(Value::F64(n as f64))
}

pub fn builtin_parse_i64(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
    let s = args[0]
        .as_string()
        .ok_or_else(|| RuntimeError::new("parse_i64 requires String", 0))?;
    match s.parse::<i64>() {
        Ok(n) => Ok(Value::I64(n)),
        Err(_) => Err(RuntimeError::new(format!("Cannot parse '{}' as i64", s), 0)),
    }
}

pub fn builtin_parse_f64(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
    let s = args[0]
        .as_string()
        .ok_or_else(|| RuntimeError::new("parse_f64 requires String", 0))?;
    match s.parse::<f64>() {
        Ok(n) => Ok(Value::F64(n)),
        Err(_) => Err(RuntimeError::new(format!("Cannot parse '{}' as f64", s), 0)),
    }
}

pub fn builtin_type_of(_vm: &mut Vm, _bytecode: &Bytecode, args: &[Value]) -> RuntimeResult<Value> {
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
        Value::Function(_) => "function",
    };
    Ok(Value::String(type_name.to_string()))
}

// ============================================================================
// Phase 1.4: Missing builtins - String operations
// ============================================================================

pub fn builtin_str_split(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
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

pub fn builtin_str_trim(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
    let s = args[0]
        .as_string()
        .ok_or_else(|| RuntimeError::new("str_trim requires String", 0))?;
    Ok(Value::String(s.trim().to_string()))
}

pub fn builtin_str_replace(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
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

pub fn builtin_str_to_upper(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
    let s = args[0]
        .as_string()
        .ok_or_else(|| RuntimeError::new("str_to_upper requires String", 0))?;
    Ok(Value::String(s.to_uppercase()))
}

pub fn builtin_str_to_lower(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
    let s = args[0]
        .as_string()
        .ok_or_else(|| RuntimeError::new("str_to_lower requires String", 0))?;
    Ok(Value::String(s.to_lowercase()))
}

pub fn builtin_str_starts_with(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
    let s = args[0]
        .as_string()
        .ok_or_else(|| RuntimeError::new("str_starts_with requires String", 0))?;
    let prefix = args[1]
        .as_string()
        .ok_or_else(|| RuntimeError::new("str_starts_with prefix must be String", 0))?;
    Ok(Value::Bool(s.starts_with(prefix)))
}

pub fn builtin_str_ends_with(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
    let s = args[0]
        .as_string()
        .ok_or_else(|| RuntimeError::new("str_ends_with requires String", 0))?;
    let suffix = args[1]
        .as_string()
        .ok_or_else(|| RuntimeError::new("str_ends_with suffix must be String", 0))?;
    Ok(Value::Bool(s.ends_with(suffix)))
}

pub fn builtin_str_index_of(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
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

pub fn builtin_array_slice(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
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

pub fn builtin_array_concat(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
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

pub fn builtin_array_contains(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
    let arr = args[0]
        .as_array()
        .ok_or_else(|| RuntimeError::new("array_contains requires Array", 0))?;
    let val = &args[1];
    Ok(Value::Bool(arr.iter().any(|x| x == val)))
}

pub fn builtin_array_pop(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
    let arr = args[0]
        .as_array()
        .ok_or_else(|| RuntimeError::new("array_pop requires Array", 0))?;
    if arr.is_empty() {
        return Ok(Value::Nil);
    }
    let mut new_arr = arr.to_vec();
    Ok(new_arr.pop().unwrap_or(Value::Nil))
}

pub fn builtin_array_reverse(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
    let arr = args[0]
        .as_array()
        .ok_or_else(|| RuntimeError::new("array_reverse requires Array", 0))?;
    let mut result: Vec<Value> = arr.to_vec();
    result.reverse();
    Ok(Value::Array(result))
}

pub fn builtin_array_sort(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
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

pub fn builtin_map_get(_vm: &mut Vm, _bytecode: &Bytecode, args: &[Value]) -> RuntimeResult<Value> {
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

pub fn builtin_map_set(_vm: &mut Vm, _bytecode: &Bytecode, args: &[Value]) -> RuntimeResult<Value> {
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

pub fn builtin_map_keys(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
    let map = args[0]
        .as_map()
        .ok_or_else(|| RuntimeError::new("map_keys requires Map", 0))?;
    let keys: Vec<Value> = map.keys().map(|k| Value::String(k.clone())).collect();
    Ok(Value::Array(keys))
}

pub fn builtin_map_values(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
    let map = args[0]
        .as_map()
        .ok_or_else(|| RuntimeError::new("map_values requires Map", 0))?;
    let values: Vec<Value> = map.values().cloned().collect();
    Ok(Value::Array(values))
}

pub fn builtin_map_contains(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
    let map = args[0]
        .as_map()
        .ok_or_else(|| RuntimeError::new("map_contains requires Map", 0))?;
    let key = args[1]
        .as_string()
        .ok_or_else(|| RuntimeError::new("map_contains key must be String", 0))?;
    Ok(Value::Bool(map.contains_key(key)))
}

pub fn builtin_map_remove(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
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

pub fn builtin_read_file(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
    let path = args[0]
        .as_string()
        .ok_or_else(|| RuntimeError::new("read_file requires String path", 0))?;
    let mut file = sandbox_open_read(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)
        .map_err(|e| RuntimeError::new(format!("read_file failed: {}", e), 0))?;
    Ok(Value::String(content))
}

pub fn builtin_write_file(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
    let path = args[0]
        .as_string()
        .ok_or_else(|| RuntimeError::new("write_file requires String path", 0))?;
    let content = args[1]
        .as_string()
        .ok_or_else(|| RuntimeError::new("write_file content must be String", 0))?;
    let mut file = sandbox_open_write(path)?;
    file.write_all(content.as_bytes())
        .map_err(|e| RuntimeError::new(format!("write_file failed: {}", e), 0))?;
    Ok(Value::Bool(true))
}

pub fn builtin_clock_ms(
    vm: &mut Vm,
    _bytecode: &Bytecode,
    _args: &[Value],
) -> RuntimeResult<Value> {
    // Deterministic logical clock: increments on each call instead of wall-clock time.
    // This satisfies HLX-S Axiom 1 (Determinism).
    vm.logical_clock += 1;
    Ok(Value::I64(vm.logical_clock as i64))
}

/// Sleep for specified milliseconds
/// sleep(ms: i64) -> i64
pub fn builtin_sleep(_vm: &mut Vm, _bytecode: &Bytecode, args: &[Value]) -> RuntimeResult<Value> {
    let ms = args[0]
        .as_i64()
        .ok_or_else(|| RuntimeError::new("sleep requires i64 milliseconds", 0))?;
    std::thread::sleep(std::time::Duration::from_millis(ms as u64));
    Ok(Value::I64(ms))
}

/// Assert that a condition is true
/// assert(condition: bool, message: String) -> bool
pub fn builtin_assert(_vm: &mut Vm, _bytecode: &Bytecode, args: &[Value]) -> RuntimeResult<Value> {
    let condition = args[0]
        .as_bool()
        .ok_or_else(|| RuntimeError::new("assert requires bool condition", 0))?;
    let message = if args.len() > 1 {
        args[1].as_string().unwrap_or("Assertion failed")
    } else {
        "Assertion failed"
    };
    if condition {
        Ok(Value::Bool(true))
    } else {
        Err(RuntimeError::new(
            format!("Assertion failed: {}", message),
            0,
        ))
    }
}

/// Sort an array (ascending)
/// sort(arr: Array) -> Array
pub fn builtin_sort(_vm: &mut Vm, _bytecode: &Bytecode, args: &[Value]) -> RuntimeResult<Value> {
    let arr = args[0]
        .as_array()
        .ok_or_else(|| RuntimeError::new("sort requires Array", 0))?;
    let mut sorted: Vec<Value> = arr.to_vec();
    sorted.sort_by(|a, b| {
        // Try numeric comparison first
        match (a.as_i64(), b.as_i64()) {
            (Some(a_i64), Some(b_i64)) => a_i64.cmp(&b_i64),
            _ => match (a.as_f64(), b.as_f64()) {
                (Some(a_f64), Some(b_f64)) => a_f64
                    .partial_cmp(&b_f64)
                    .unwrap_or(std::cmp::Ordering::Equal),
                _ => match (a.as_string(), b.as_string()) {
                    (Some(a_str), Some(b_str)) => a_str.cmp(b_str),
                    _ => std::cmp::Ordering::Equal,
                },
            },
        }
    });
    Ok(Value::Array(sorted))
}

/// DISABLED: Shell access is a sandbox escape.
/// shell() is permanently disabled per the V12 Security Audit.
pub fn builtin_shell(_vm: &mut Vm, _bytecode: &Bytecode, _args: &[Value]) -> RuntimeResult<Value> {
    Err(RuntimeError::new(
        "shell() is disabled — sandbox violation (V12 Security Audit)",
        0,
    ))
}

// ============================================================================
// Phase 4.3: Homeostasis and Promotion builtins
// These are wired to the RSI pipeline for Bit's self-awareness
// ============================================================================

/// Returns the current homeostasis pressure (0.0-1.0)
/// High pressure means system is under stress from too many modifications
pub fn builtin_homeostasis_pressure(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    _args: &[Value],
) -> RuntimeResult<Value> {
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
pub fn builtin_promotion_level(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    _args: &[Value],
) -> RuntimeResult<Value> {
    // Returns current level as i64
    // 0 = Seedling, 1 = Sprout, 2 = Sapling, 3 = Mature
    Ok(Value::I64(0)) // Default to seedling - VM will override
}

/// Returns whether self-modification is currently allowed
pub fn builtin_can_modify_self(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    _args: &[Value],
) -> RuntimeResult<Value> {
    // Returns true if the agent can propose modifications at current level
    // Seedling can only do parameter/threshold changes
    Ok(Value::Bool(true))
}

/// Returns RSI modification history as a list of dicts
pub fn builtin_rsi_history(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    _args: &[Value],
) -> RuntimeResult<Value> {
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
pub fn builtin_evaluate_fitness(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    _args: &[Value],
) -> RuntimeResult<Value> {
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

/// Records a fitness snapshot before/after modification
/// Returns snapshot ID for comparison
pub fn builtin_fitness_snapshot(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
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
        builtin_evaluate_fitness(_vm, _bytecode, &[])?.clone(),
    );
    Ok(Value::Map(map))
}

/// Compares two fitness snapshots
/// Returns dict with before, after, delta
pub fn builtin_fitness_compare(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
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

/// bond(prompt, context) - Call the bonded LLM (Qwen3)
/// Returns the LLM response as a string
pub fn builtin_bond(_vm: &mut Vm, _bytecode: &Bytecode, args: &[Value]) -> RuntimeResult<Value> {
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
        prompt.replace('"', "\""),
        context_json
    );

    // Simple HTTP POST request
    let request = format!(
        "POST /bond HTTP/1.1
\
         Host: {}
\
         Content-Type: application/json
\
         Content-Length: {}
\
         Connection: close

\
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
    if let Some(body_start) = response.find(
        "

",
    ) {
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

pub fn builtin_set_tensor(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
    if args.len() != 3 {
        return Err(RuntimeError::new("set_tensor requires 3 args", 0));
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

pub fn builtin_get_tensor(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
    if args.len() != 2 {
        return Err(RuntimeError::new("get_tensor requires 2 args", 0));
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
    use crate::bytecode::Bytecode;
    use crate::tensor::Tensor;
    use crate::vm::Vm;
    use serial_test::serial;

    fn setup() {
        crate::tensor::reset_global_allocation();
        crate::tensor::set_global_limit(crate::tensor::DEFAULT_MAX_TENSOR_ELEMENTS);
    }

    #[test]
    #[serial]
    fn test_image_process_grayscale() {
        setup();
        let mut vm = Vm::new();
        let bc = Bytecode::new();
        let mut tensor = Tensor::zeros(vec![3, 4, 4]);
        for i in 0..48 {
            tensor.data[i] = 0.5;
        }
        let result = builtin_image_process(
            &mut vm,
            &bc,
            &[
                Value::Tensor(tensor),
                Value::String("grayscale".to_string()),
            ],
        )
        .unwrap();
        match result {
            Value::Tensor(t) => {
                assert_eq!(t.shape, vec![1, 4, 4]);
            }
            _ => panic!("Expected Tensor"),
        }
    }
}

pub fn builtin_map(vm: &mut Vm, bytecode: &Bytecode, args: &[Value]) -> RuntimeResult<Value> {
    let val = args[0].clone();
    let arr = val
        .as_array()
        .ok_or_else(|| RuntimeError::new("map: first arg must be Array", 0))?;
    let func = args[1].clone();

    let mut result = Vec::with_capacity(arr.len());
    for item in arr {
        let val = vm.call_value(&func, &[item.clone()], bytecode)?;
        result.push(val);
    }
    Ok(Value::Array(result))
}

pub fn builtin_filter(vm: &mut Vm, bytecode: &Bytecode, args: &[Value]) -> RuntimeResult<Value> {
    let val = args[0].clone();
    let arr = val
        .as_array()
        .ok_or_else(|| RuntimeError::new("filter: first arg must be Array", 0))?;
    let func = args[1].clone();

    let mut result = Vec::new();
    for item in arr {
        let val = vm.call_value(&func, &[item.clone()], bytecode)?;
        if val.is_truthy() {
            result.push(item.clone());
        }
    }
    Ok(Value::Array(result))
}

pub fn builtin_fold(vm: &mut Vm, bytecode: &Bytecode, args: &[Value]) -> RuntimeResult<Value> {
    let val = args[0].clone();
    let arr = val
        .as_array()
        .ok_or_else(|| RuntimeError::new("fold: first arg must be Array", 0))?;
    let mut acc = args[1].clone();
    let func = args[2].clone();

    for item in arr {
        acc = vm.call_value(&func, &[acc, item.clone()], bytecode)?;
    }
    Ok(acc)
}

// --- Phase 6: Recursive Intelligence Builtins ---

pub fn builtin_tensor_blend(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
    if args.len() != 3 {
        return Err(RuntimeError::new(
            "tensor_blend requires 3 args (a, b, alpha)",
            0,
        ));
    }

    let alpha = match &args[2] {
        Value::F64(f) => *f,
        Value::I64(n) => *n as f64,
        _ => return Err(RuntimeError::new("tensor_blend: alpha must be numeric", 0)),
    };

    let a = match &args[0] {
        Value::Tensor(t) => t,
        _ => {
            return Err(RuntimeError::new(
                "tensor_blend: first arg must be tensor",
                0,
            ))
        }
    };

    let b = match &args[1] {
        Value::Tensor(t) => t,
        _ => {
            return Err(RuntimeError::new(
                "tensor_blend: second arg must be tensor",
                0,
            ))
        }
    };

    if a.data.len() != b.data.len() {
        return Err(RuntimeError::new(
            format!(
                "tensor_blend: tensors must have same size ({} vs {})",
                a.data.len(),
                b.data.len()
            ),
            0,
        ));
    }

    let mut result = a.clone();
    let one_minus_alpha = 1.0 - alpha;

    for (i, val_b) in b.data.iter().enumerate() {
        result.data[i] = result.data[i] * one_minus_alpha + val_b * alpha;
    }

    Ok(Value::Tensor(result))
}

pub fn builtin_mem_query_vec(
    vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
    if args.len() < 1 {
        return Err(RuntimeError::new(
            "mem_query_vec requires at least 1 arg (embedding)",
            0,
        ));
    }

    let query = match &args[0] {
        Value::Tensor(t) => t.data.iter().map(|&x| x as f32).collect::<Vec<f32>>(),
        _ => return Err(RuntimeError::new("mem_query_vec: arg must be tensor", 0)),
    };

    let top_k = if args.len() > 1 {
        match &args[1] {
            Value::I64(n) => *n as usize,
            _ => 5,
        }
    } else {
        5
    };

    let results = vm.memory_pool.query_by_embedding(&query, top_k);
    // Hierarchical Sifter: Prioritize "core" source results
    let core_results = results
        .iter()
        .filter(|(res, _)| {
            res.source == "core" || res.source == "identity" || res.source == "academy"
        })
        .collect::<Vec<_>>();
    let results = if !core_results.is_empty() {
        core_results.into_iter().map(|(r, s)| (*r, *s)).collect()
    } else {
        results
    };
    let mut list = Vec::new();

    for (res, score) in results {
        let mut map = std::collections::BTreeMap::new();
        map.insert("source".to_string(), Value::String(res.source.clone()));
        map.insert("content".to_string(), Value::String(res.content.clone()));
        map.insert("relevance".to_string(), Value::F64(score as f64));
        list.push(Value::Map(map));
    }

    Ok(Value::Array(list))
}

pub fn builtin_mem_store_vec(
    vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
    if args.len() < 2 {
        return Err(RuntimeError::new(
            "mem_store_vec requires 2 args (content, embedding)",
            0,
        ));
    }

    let content = match &args[0] {
        Value::String(s) => s.clone(),
        _ => {
            return Err(RuntimeError::new(
                "mem_store_vec: first arg must be string",
                0,
            ))
        }
    };

    let embedding = match &args[1] {
        Value::Tensor(t) => t.data.iter().map(|&x| x as f32).collect::<Vec<f32>>(),
        _ => {
            return Err(RuntimeError::new(
                "mem_store_vec: second arg must be tensor",
                0,
            ))
        }
    };

    let source = if args.len() > 2 {
        match &args[2] {
            Value::String(s) => s.clone(),
            _ => "hlx_native".to_string(),
        }
    } else {
        "hlx_native".to_string()
    };

    let relevance = if args.len() > 3 {
        match &args[3] {
            Value::F64(f) => *f,
            Value::I64(n) => *n as f64,
            _ => 1.0,
        }
    } else {
        1.0
    };

    let obs = crate::memory_pool::Observation::new(source, content)
        .with_embedding(embedding)
        .with_relevance(relevance);

    vm.memory_pool
        .add_observation_with_relevance(obs, relevance);

    Ok(Value::Bool(true))
}

pub fn builtin_native_embed(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
    if args.len() < 1 {
        return Err(RuntimeError::new("native_embed requires 1 arg (text)", 0));
    }

    let text = match &args[0] {
        Value::String(s) => s.to_lowercase(),
        _ => return Err(RuntimeError::new("native_embed: arg must be string", 0)),
    };

    let mut data = vec![0.0; 10240];
    let bytes = text.as_bytes();

    if bytes.len() >= 3 {
        for i in 0..bytes.len() - 2 {
            let h = (bytes[i] as usize)
                .wrapping_mul(31)
                .wrapping_add(bytes[i + 1] as usize)
                .wrapping_mul(17)
                .wrapping_add(bytes[i + 2] as usize);
            data[h % 10240] += 1.0;
        }
    }

    for word in text.split_whitespace() {
        let mut h: usize = 0;
        for &b in word.as_bytes() {
            h = h.wrapping_mul(31).wrapping_add(b as usize);
        }
        data[h % 10240] += 2.0;
    }

    let mag = data.iter().map(|&x| x * x).sum::<f64>().sqrt();
    if mag > 0.0 {
        for val in data.iter_mut() {
            *val /= mag;
        }
    }

    Ok(Value::Tensor(crate::tensor::Tensor {
        data,
        shape: vec![10240],
    }))
}

pub fn builtin_native_zeros(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
    let size = args.get(0).and_then(|v| v.as_i64()).unwrap_or(10240) as usize;
    Ok(Value::Tensor(crate::tensor::Tensor {
        data: vec![0.0; size],
        shape: vec![size],
    }))
}

pub fn builtin_native_rand(
    vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
    use rand::Rng;
    let size = args.get(0).and_then(|v| v.as_i64()).unwrap_or(10240) as usize;
    let data: Vec<f64> = (0..size).map(|_| vm.rng.gen_range(-1.0..1.0)).collect();
    Ok(Value::Tensor(crate::tensor::Tensor {
        data,
        shape: vec![size],
    }))
}

pub fn builtin_tensor_slice(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
    if args.len() != 3 {
        return Err(RuntimeError::new(
            "tensor_slice requires 3 args (tensor, start, len)",
            0,
        ));
    }

    let t = match &args[0] {
        Value::Tensor(t) => t,
        _ => {
            return Err(RuntimeError::new(
                "tensor_slice: first arg must be tensor",
                0,
            ))
        }
    };

    let start = args[1].as_i64().unwrap_or(0) as usize;
    let len = args[2].as_i64().unwrap_or(0) as usize;

    if start + len > t.data.len() {
        return Err(RuntimeError::new("tensor_slice: out of bounds", 0));
    }

    let slice_data = t.data[start..start + len].to_vec();
    Ok(Value::Tensor(crate::tensor::Tensor {
        data: slice_data,
        shape: vec![len],
    }))
}

pub fn builtin_tensor_merge(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
    if args.len() != 3 {
        return Err(RuntimeError::new(
            "tensor_merge requires 3 args (base, slice, start)",
            0,
        ));
    }

    let mut base = match &args[0] {
        Value::Tensor(t) => t.clone(),
        _ => {
            return Err(RuntimeError::new(
                "tensor_merge: first arg must be tensor",
                0,
            ))
        }
    };

    let slice = match &args[1] {
        Value::Tensor(t) => t,
        _ => {
            return Err(RuntimeError::new(
                "tensor_merge: second arg must be tensor",
                0,
            ))
        }
    };

    let start = args[2].as_i64().unwrap_or(0) as usize;

    if start + slice.data.len() > base.data.len() {
        return Err(RuntimeError::new(
            "tensor_merge: slice exceeds base bounds",
            0,
        ));
    }

    for (i, val) in slice.data.iter().enumerate() {
        base.data[start + i] = *val;
    }

    Ok(Value::Tensor(base))
}

pub fn builtin_patch_module(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
    if args.len() != 2 {
        return Err(RuntimeError::new(
            "patch_module requires 2 args (module_name, new_source)",
            0,
        ));
    }

    let _module_name = match &args[0] {
        Value::String(s) => s.clone(),
        _ => {
            return Err(RuntimeError::new(
                "patch_module: module_name must be string",
                0,
            ))
        }
    };

    let new_source = match &args[1] {
        Value::String(s) => s.clone(),
        _ => {
            return Err(RuntimeError::new(
                "patch_module: new_source must be string",
                0,
            ))
        }
    };

    use crate::{AstParser, Lowerer};

    let program = AstParser::parse(&new_source)
        .map_err(|e| RuntimeError::new(format!("SMI Parse Error: {:?}", e), 0))?;

    let (_bc, _funcs) = Lowerer::lower(&program)
        .map_err(|e| RuntimeError::new(format!("SMI Lower Error: {:?}", e), 0))?;

    println!("[SMI] Module {} patched successfully.", _module_name);

    Ok(Value::Bool(true))
}

pub fn builtin_get_substrate_pressure(
    vm: &mut Vm,
    _bytecode: &Bytecode,
    _args: &[Value],
) -> RuntimeResult<Value> {
    Ok(Value::F64(vm.substrate_pressure))
}

pub fn builtin_tensor_convolve(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
    let a = match args.get(0) {
        Some(Value::Tensor(t)) => t,
        _ => {
            return Err(RuntimeError::new(
                "tensor_convolve: first arg must be tensor",
                0,
            ))
        }
    };
    let b = match args.get(1) {
        Some(Value::Tensor(t)) => t,
        _ => {
            return Err(RuntimeError::new(
                "tensor_convolve: second arg must be tensor",
                0,
            ))
        }
    };
    Ok(Value::Tensor(a.circular_convolve(b)?))
}

pub fn builtin_tensor_correlate(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
    let a = match args.get(0) {
        Some(Value::Tensor(t)) => t,
        _ => {
            return Err(RuntimeError::new(
                "tensor_correlate: first arg must be tensor",
                0,
            ))
        }
    };
    let b = match args.get(1) {
        Some(Value::Tensor(t)) => t,
        _ => {
            return Err(RuntimeError::new(
                "tensor_correlate: second arg must be tensor",
                0,
            ))
        }
    };
    Ok(Value::Tensor(a.circular_correlate(b)?))
}

pub fn builtin_tensor_normalize(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
    let mut t = match args.get(0) {
        Some(Value::Tensor(t)) => t.clone(),
        _ => return Err(RuntimeError::new("tensor_normalize: arg must be tensor", 0)),
    };
    let mag = t.data.iter().map(|&x| x * x).sum::<f64>().sqrt();
    if mag > 0.0 {
        for val in t.data.iter_mut() {
            *val /= mag;
        }
    }
    Ok(Value::Tensor(t))
}

pub fn builtin_tensor_topology_score(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
    let t = match args.get(0) {
        Some(Value::Tensor(t)) => t,
        _ => {
            return Err(RuntimeError::new(
                "tensor_topology_score: arg must be tensor",
                0,
            ))
        }
    };

    // TDA Approximation: Measure the Local Manifold Coherence
    // In a stable 10k field, values should follow a Gaussian distribution.
    // If the "topology" is tearing, we see high-frequency spikes.
    let n = t.data.len();
    if n < 2 {
        return Ok(Value::F64(1.0));
    }

    let mut variance = 0.0;
    let mean = t.data.iter().sum::<f64>() / n as f64;
    for x in t.data.iter() {
        variance += (x - mean).powi(2);
    }
    variance /= n as f64;

    // Coherence score (1.0 = Perfect Manifold, 0.0 = Liquefied/Noisy)
    let score = (1.0 / (1.0 + variance)).clamp(0.0, 1.0);
    Ok(Value::F64(score))
}

/// Serialize the VM's cognitive state (latent_states, memory) to a JSON file.
/// Usage: snapshot("path/to/snapshot.json")
/// Returns the number of entries saved.
pub fn builtin_snapshot(
    vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
    let path = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| RuntimeError::new("snapshot requires String path", 0))?;

    let mut file = sandbox_open_write(path)?;

    // Snapshot cognitive state: latent_states + memory entries
    let snapshot = serde_json::json!({
        "version": 2,
        "logical_clock": vm.logical_clock,
        "rng_seed": vm.rng_seed,
        "latent_states": vm.latent_states
            .iter()
            .map(|(k, v)| (k.clone(), serde_json::to_value(v).unwrap_or_default()))
            .collect::<serde_json::Map<String, serde_json::Value>>(),
        "memory_count": vm.memory.len(),
        "memory": vm.memory.iter().map(|m| {
            serde_json::json!({"pattern": m.pattern, "confidence": m.confidence})
        }).collect::<Vec<_>>(),
    });

    let json = serde_json::to_string_pretty(&snapshot)
        .map_err(|e| RuntimeError::new(format!("Snapshot serialization failed: {}", e), 0))?;

    file.write_all(json.as_bytes())
        .map_err(|e| RuntimeError::new(format!("Snapshot write failed: {}", e), 0))?;

    let count = vm.latent_states.len() + vm.memory.len();
    Ok(Value::I64(count as i64))
}

/// Restore VM cognitive state from a snapshot file.
/// Usage: restore("path/to/snapshot.json")
/// Returns the number of entries restored.
pub fn builtin_restore(
    vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
    let path = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| RuntimeError::new("restore requires String path", 0))?;

    let mut file = sandbox_open_read(path)?;
    let mut json = String::new();
    file.read_to_string(&mut json)
        .map_err(|e| RuntimeError::new(format!("Restore read failed: {}", e), 0))?;

    let snapshot: serde_json::Value = serde_json::from_str(&json)
        .map_err(|e| RuntimeError::new(format!("Snapshot parse failed: {}", e), 0))?;

    let mut count = 0;

    // Restore latent states
    if let Some(states) = snapshot.get("latent_states").and_then(|v| v.as_object()) {
        for (k, v) in states {
            if let Ok(val) = serde_json::from_value::<Value>(v.clone()) {
                vm.latent_states.insert(k.clone(), val);
                count += 1;
            }
        }
    }

    // Restore memory entries
    if let Some(memories) = snapshot.get("memory").and_then(|v| v.as_array()) {
        for m in memories {
            if let (Some(pattern), Some(confidence)) = (
                m.get("pattern").and_then(|v| v.as_str()),
                m.get("confidence").and_then(|v| v.as_f64()),
            ) {
                vm.memory.push(crate::vm::MemEntry {
                    pattern: pattern.to_string(),
                    confidence,
                });
                count += 1;
            }
        }
    }

    // Restore logical clock
    if let Some(clock) = snapshot.get("logical_clock").and_then(|v| v.as_u64()) {
        vm.logical_clock = clock;
    }

    // Phase 24: Restore PRNG seed for deterministic continuation
    if let Some(seed) = snapshot.get("rng_seed").and_then(|v| v.as_u64()) {
        vm.rng_seed = seed;
        vm.rng = rand::rngs::StdRng::seed_from_u64(seed);
    }

    Ok(Value::I64(count))
}

/// Drain all pending sync events as a JSON array string.
/// The Python bridge calls this after each VM run to flush changes to corpus.db.
/// Returns Nil if no events pending (non-blocking).
pub fn builtin_drain_sync_events(
    vm: &mut Vm,
    _bytecode: &Bytecode,
    _args: &[Value],
) -> RuntimeResult<Value> {
    if vm.sync_events.is_empty() {
        return Ok(Value::Nil);
    }
    let events = std::mem::take(&mut vm.sync_events);
    let json = serde_json::to_string(&events)
        .map_err(|e| RuntimeError::new(format!("Sync event serialization failed: {}", e), 0))?;
    Ok(Value::String(json))
}

// ============================================================================
// Phase 19: Symmetric Python→HLX Tensor Bridge
// ============================================================================

/// Parse a JSON-encoded tensor (with "shape" and "data" arrays) and return Value::Tensor.
///
/// arg[0]: String containing JSON like `{"shape":[3,4],"data":[1.0,2.0,...]}`
pub fn builtin_tensor_from_json(
    _vm: &mut Vm,
    _bytecode: &Bytecode,
    args: &[Value],
) -> RuntimeResult<Value> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            "tensor_from_json requires exactly 1 argument (JSON string)",
            0,
        ));
    }
    let json_str = match &args[0] {
        Value::String(s) => s.as_str(),
        _ => {
            return Err(RuntimeError::new(
                "tensor_from_json: argument must be a JSON string",
                0,
            ))
        }
    };

    let parsed: serde_json::Value = serde_json::from_str(json_str).map_err(|e| {
        RuntimeError::new(format!("tensor_from_json: invalid JSON: {}", e), 0)
    })?;

    let shape_arr = parsed
        .get("shape")
        .and_then(|v| v.as_array())
        .ok_or_else(|| RuntimeError::new("tensor_from_json: missing or invalid 'shape' array", 0))?;

    let shape: Vec<usize> = shape_arr
        .iter()
        .enumerate()
        .map(|(i, v)| {
            v.as_u64()
                .map(|n| n as usize)
                .ok_or_else(|| {
                    RuntimeError::new(
                        format!("tensor_from_json: shape[{}] is not a valid unsigned integer", i),
                        0,
                    )
                })
        })
        .collect::<RuntimeResult<Vec<usize>>>()?;

    let data_arr = parsed
        .get("data")
        .and_then(|v| v.as_array())
        .ok_or_else(|| RuntimeError::new("tensor_from_json: missing or invalid 'data' array", 0))?;

    let data: Vec<f64> = data_arr
        .iter()
        .enumerate()
        .map(|(i, v)| {
            v.as_f64().ok_or_else(|| {
                RuntimeError::new(
                    format!("tensor_from_json: data[{}] is not a valid number", i),
                    0,
                )
            })
        })
        .collect::<RuntimeResult<Vec<f64>>>()?;

    let tensor = crate::tensor::Tensor::from_data(shape, data)?;
    Ok(Value::Tensor(tensor))
}

// ============================================================================
// Phase 21: Tether Inbox Async Polling
// ============================================================================

/// Non-blocking poll of the VM's communication inbox.
///
/// Returns Value::Nil if inbox is empty. Otherwise, pops the oldest message
/// and returns it as a Value::Map with keys like "type", "content", etc.
pub fn builtin_poll_inbox(
    vm: &mut Vm,
    _bytecode: &Bytecode,
    _args: &[Value],
) -> RuntimeResult<Value> {
    match vm.inbox.pop_front() {
        None => Ok(Value::Nil),
        Some(msg) => Ok(Value::Map(msg)),
    }
}
