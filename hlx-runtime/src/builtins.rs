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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tensor::Tensor;
    use image::ImageFormat;
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
