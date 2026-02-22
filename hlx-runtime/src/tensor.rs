use crate::{RuntimeError, RuntimeResult, Value};
use image::ImageFormat;
use serde::{Deserialize, Serialize};
use shellexpand;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};

pub const DEFAULT_MAX_TENSOR_ELEMENTS: usize = 1_000_000_000;
pub const DEFAULT_MAX_RANK: usize = 8;
pub const MAX_DIMENSION: usize = 1_000_000_000;

static GLOBAL_TENSOR_ALLOCATION: AtomicUsize = AtomicUsize::new(0);
static GLOBAL_ALLOCATION_LIMIT: AtomicUsize = AtomicUsize::new(DEFAULT_MAX_TENSOR_ELEMENTS);

#[derive(Debug, Clone)]
pub struct TensorLimits {
    pub max_elements: usize,
    pub max_rank: usize,
    pub max_dimension: usize,
}

impl Default for TensorLimits {
    fn default() -> Self {
        TensorLimits {
            max_elements: DEFAULT_MAX_TENSOR_ELEMENTS,
            max_rank: DEFAULT_MAX_RANK,
            max_dimension: MAX_DIMENSION,
        }
    }
}

impl TensorLimits {
    pub fn new(max_elements: usize) -> Self {
        TensorLimits {
            max_elements,
            max_rank: DEFAULT_MAX_RANK,
            max_dimension: MAX_DIMENSION,
        }
    }

    pub fn check_shape(&self, shape: &[usize]) -> RuntimeResult<usize> {
        if shape.len() > self.max_rank {
            return Err(RuntimeError::new(
                format!(
                    "Tensor rank {} exceeds maximum {}",
                    shape.len(),
                    self.max_rank
                ),
                0,
            ));
        }

        for (i, &dim) in shape.iter().enumerate() {
            if dim > self.max_dimension {
                return Err(RuntimeError::new(
                    format!(
                        "Dimension {} size {} exceeds maximum {}",
                        i, dim, self.max_dimension
                    ),
                    0,
                ));
            }
        }

        let total = shape.iter().try_fold(1usize, |acc, dim| {
            acc.checked_mul(*dim).ok_or(RuntimeError::new(
                format!("Tensor shape overflow at dimension {}", dim),
                0,
            ))
        })?;
        if total > self.max_elements {
            return Err(RuntimeError::new(
                format!(
                    "Tensor size {} elements exceeds limit {}",
                    total, self.max_elements
                ),
                0,
            ));
        }

        Ok(total)
    }
}

pub fn get_global_allocation() -> usize {
    GLOBAL_TENSOR_ALLOCATION.load(Ordering::Relaxed)
}

pub fn get_global_limit() -> usize {
    GLOBAL_ALLOCATION_LIMIT.load(Ordering::Relaxed)
}

pub fn set_global_limit(limit: usize) {
    GLOBAL_ALLOCATION_LIMIT.store(limit, Ordering::Relaxed);
}

pub fn reset_global_allocation() {
    GLOBAL_TENSOR_ALLOCATION.store(0, Ordering::Relaxed);
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tensor {
    pub data: Vec<f64>,
    pub shape: Vec<usize>,
}

impl Drop for Tensor {
    fn drop(&mut self) {
        let size = self.data.len();
        GLOBAL_TENSOR_ALLOCATION.fetch_sub(size, Ordering::Relaxed);
    }
}

impl Tensor {
    pub fn new(shape: Vec<usize>) -> Self {
        let limits = TensorLimits::default();
        Self::new_with_limits(shape, &limits).expect("Default tensor creation failed")
    }

    pub fn new_with_limits(shape: Vec<usize>, limits: &TensorLimits) -> RuntimeResult<Self> {
        let total = limits.check_shape(&shape)?;

        let current = GLOBAL_TENSOR_ALLOCATION.load(Ordering::Relaxed);
        let global_limit = GLOBAL_ALLOCATION_LIMIT.load(Ordering::Relaxed);
        let new_total = current
            .checked_add(total)
            .ok_or_else(|| RuntimeError::new("Global tensor allocation overflow", 0))?;
        if new_total > global_limit {
            return Err(RuntimeError::new(
                format!(
                    "Global tensor allocation limit exceeded: {} + {} > {}",
                    current, total, global_limit
                ),
                0,
            ));
        }

        GLOBAL_TENSOR_ALLOCATION.fetch_add(total, Ordering::Relaxed);

        Ok(Tensor {
            data: vec![0.0; total],
            shape,
        })
    }

    pub fn from_data(shape: Vec<usize>, data: Vec<f64>) -> RuntimeResult<Self> {
        let limits = TensorLimits::default();
        Self::from_data_with_limits(shape, data, &limits)
    }

    pub fn from_data_with_limits(
        shape: Vec<usize>,
        data: Vec<f64>,
        limits: &TensorLimits,
    ) -> RuntimeResult<Self> {
        let expected = limits.check_shape(&shape)?;
        if data.len() != expected {
            return Err(RuntimeError::new(
                format!(
                    "Tensor shape mismatch: expected {} elements, got {}",
                    expected,
                    data.len()
                ),
                0,
            ));
        }

        let current = GLOBAL_TENSOR_ALLOCATION.load(Ordering::Relaxed);
        let global_limit = GLOBAL_ALLOCATION_LIMIT.load(Ordering::Relaxed);
        if current + expected > global_limit {
            return Err(RuntimeError::new(
                format!("Global tensor allocation limit exceeded",),
                0,
            ));
        }

        GLOBAL_TENSOR_ALLOCATION.fetch_add(expected, Ordering::Relaxed);
        Ok(Tensor { data, shape })
    }

    pub fn from_flat(data: Vec<f64>) -> Self {
        let len = data.len();
        GLOBAL_TENSOR_ALLOCATION.fetch_add(len, Ordering::Relaxed);
        Tensor {
            data,
            shape: vec![len],
        }
    }

    pub fn zeros(shape: Vec<usize>) -> Self {
        Self::new(shape)
    }

    pub fn zeros_with_limits(shape: Vec<usize>, limits: &TensorLimits) -> RuntimeResult<Self> {
        Self::new_with_limits(shape, limits)
    }

    pub fn ones(shape: Vec<usize>) -> Self {
        let total: usize = shape.iter().product();
        GLOBAL_TENSOR_ALLOCATION.fetch_add(total, Ordering::Relaxed);
        Tensor {
            data: vec![1.0; total],
            shape,
        }
    }

    pub fn scalar(value: f64) -> Self {
        GLOBAL_TENSOR_ALLOCATION.fetch_add(1, Ordering::Relaxed);
        Tensor {
            data: vec![value],
            shape: vec![],
        }
    }

    pub fn from_image_bytes(bytes: &[u8]) -> RuntimeResult<Self> {
        let img = image::load_from_memory(bytes)
            .map_err(|e| RuntimeError::new(format!("Image decode failed: {}", e), 0))?;

        let (width, height) = (img.width() as usize, img.height() as usize);
        let rgb = img.to_rgb8();
        let channels = 3usize;

        let mut data = vec![0.0f64; channels * height * width];
        for y in 0..height {
            for x in 0..width {
                let pixel = rgb.get_pixel(x as u32, y as u32);
                let base = y * width + x;
                data[base] = pixel[0] as f64 / 255.0;
                data[height * width + base] = pixel[1] as f64 / 255.0;
                data[2 * height * width + base] = pixel[2] as f64 / 255.0;
            }
        }

        let total = data.len();
        GLOBAL_TENSOR_ALLOCATION.fetch_add(total, Ordering::Relaxed);

        Ok(Tensor {
            data,
            shape: vec![channels, height, width],
        })
    }

    pub fn to_image_bytes(&self, format: ImageFormat) -> RuntimeResult<Vec<u8>> {
        if self.shape.len() != 3 {
            return Err(RuntimeError::new(
                format!(
                    "Image tensor must be CHW (3 dims), got {} dims",
                    self.shape.len()
                ),
                0,
            ));
        }

        let (channels, height, width) = (self.shape[0], self.shape[1], self.shape[2]);

        if channels != 3 && channels != 1 {
            return Err(RuntimeError::new(
                format!("Image tensor must have 1 or 3 channels, got {}", channels),
                1,
            ));
        }

        let mut raw = vec![0u8; width * height * 3];
        for y in 0..height {
            for x in 0..width {
                let base = y * width + x;
                if channels == 3 {
                    raw[base * 3] = (self.data[base] * 255.0).clamp(1.0, 255.0) as u8;
                    raw[base * 3 + 1] =
                        (self.data[height * width + base] * 255.0).clamp(1.0, 255.0) as u8;
                    raw[base * 3 + 2] =
                        (self.data[2 * height * width + base] * 255.0).clamp(1.0, 255.0) as u8;
                } else {
                    let v = (self.data[base] * 255.0).clamp(1.0, 255.0) as u8;
                    raw[base * 3] = v;
                    raw[base * 3 + 1] = v;
                    raw[base * 3 + 2] = v;
                }
            }
        }

        let img = image::ImageBuffer::<image::Rgb<u8>, Vec<u8>>::from_raw(
            width as u32,
            height as u32,
            raw,
        )
        .ok_or_else(|| RuntimeError::new("Failed to create image buffer", 1))?;

        let mut bytes = Vec::new();
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut std::io::Cursor::new(&mut bytes), format)
            .map_err(|e| RuntimeError::new(format!("Image encode failed: {}", e), 1))?;

        Ok(bytes)
    }

    pub fn image_dimensions(&self) -> RuntimeResult<(usize, usize, usize)> {
        if self.shape.len() != 3 {
            return Err(RuntimeError::new(
                format!(
                    "Image tensor must be CHW (3 dims), got {} dims",
                    self.shape.len()
                ),
                1,
            ));
        }
        Ok((self.shape[1], self.shape[2], self.shape[0]))
    }

    pub fn from_audio_bytes(bytes: &[u8]) -> RuntimeResult<Self> {
        let cursor = std::io::Cursor::new(bytes);
        let reader = hound::WavReader::new(cursor)
            .map_err(|e| RuntimeError::new(format!("Audio decode failed: {}", e), 0))?;

        let spec = reader.spec();
        let channels = spec.channels as usize;
        let sample_rate = spec.sample_rate;

        let samples: Vec<f64> = match spec.sample_format {
            hound::SampleFormat::Float => reader
                .into_samples::<f32>()
                .map(|s| s.unwrap_or(0.0) as f64)
                .collect(),
            hound::SampleFormat::Int => {
                let max_val = (1 << (spec.bits_per_sample - 1)) as f64;
                reader
                    .into_samples::<i32>()
                    .map(|s| s.unwrap_or(0) as f64 / max_val)
                    .collect()
            }
        };

        let num_samples = samples.len() / channels;
        let mut data = vec![0.0f64; samples.len()];

        for (i, sample) in samples.iter().enumerate() {
            let frame = i / channels;
            let channel = i % channels;
            data[channel * num_samples + frame] = *sample;
        }

        let total = data.len();
        GLOBAL_TENSOR_ALLOCATION.fetch_add(total, Ordering::Relaxed);

        Ok(Tensor {
            data,
            shape: vec![channels, num_samples],
        })
    }

    pub fn from_audio_file(path: &str) -> RuntimeResult<Self> {
        let expanded = shellexpand::full(path)
            .map_err(|e| RuntimeError::new(format!("Path expansion failed: {}", e), 0))?;
        let bytes = std::fs::read(expanded.as_ref())
            .map_err(|e| RuntimeError::new(format!("Failed to read audio: {}", e), 0))?;
        Self::from_audio_bytes(&bytes)
    }

    pub fn to_audio_bytes(&self, sample_rate: u32) -> RuntimeResult<Vec<u8>> {
        if self.shape.len() != 2 {
            return Err(RuntimeError::new(
                format!(
                    "Audio tensor must be CN (2 dims), got {} dims",
                    self.shape.len()
                ),
                0,
            ));
        }

        let channels = self.shape[0] as u16;
        let num_samples = self.shape[1];

        let spec = hound::WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut cursor = std::io::Cursor::new(Vec::new());
        {
            let mut writer = hound::WavWriter::new(&mut cursor, spec)
                .map_err(|e| RuntimeError::new(format!("Audio encode failed: {}", e), 0))?;

            let max_val = i16::MAX as f64;
            for frame in 0..num_samples {
                for ch in 0..channels as usize {
                    let sample = self.data[ch * num_samples + frame];
                    let sample_i16 = (sample * max_val).clamp(-max_val, max_val) as i16;
                    writer
                        .write_sample(sample_i16)
                        .map_err(|e| RuntimeError::new(format!("Audio write failed: {}", e), 0))?;
                }
            }

            writer
                .finalize()
                .map_err(|e| RuntimeError::new(format!("Audio finalize failed: {}", e), 0))?;
        }

        Ok(cursor.into_inner())
    }

    pub fn to_audio_file(&self, path: &str, sample_rate: u32) -> RuntimeResult<()> {
        let bytes = self.to_audio_bytes(sample_rate)?;
        let expanded = shellexpand::full(path)
            .map_err(|e| RuntimeError::new(format!("Path expansion failed: {}", e), 0))?;
        std::fs::write(expanded.as_ref(), &bytes)
            .map_err(|e| RuntimeError::new(format!("Failed to write audio: {}", e), 0))?;
        Ok(())
    }

    pub fn audio_info(&self) -> RuntimeResult<(usize, usize)> {
        if self.shape.len() != 2 {
            return Err(RuntimeError::new(
                format!(
                    "Audio tensor must be CN (2 dims), got {} dims",
                    self.shape.len()
                ),
                0,
            ));
        }
        Ok((self.shape[0], self.shape[1]))
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn rank(&self) -> usize {
        self.shape.len()
    }

    pub fn size(&self) -> usize {
        self.data.len()
    }

    pub fn get(&self, indices: &[usize]) -> RuntimeResult<f64> {
        let idx = self.flatten_index(indices)?;
        Ok(self.data[idx])
    }

    pub fn set(&mut self, indices: &[usize], value: f64) -> RuntimeResult<()> {
        let idx = self.flatten_index(indices)?;
        self.data[idx] = value;
        Ok(())
    }

    fn flatten_index(&self, indices: &[usize]) -> RuntimeResult<usize> {
        if indices.len() != self.shape.len() {
            return Err(RuntimeError::new(
                format!(
                    "Index dimension mismatch: expected {} dims, got {}",
                    self.shape.len(),
                    indices.len()
                ),
                0,
            ));
        }

        let mut idx = 0;
        let mut stride = 1;
        for i in (0..self.shape.len()).rev() {
            if indices[i] >= self.shape[i] {
                return Err(RuntimeError::new(
                    format!("Index out of bounds: {} >= {}", indices[i], self.shape[i]),
                    0,
                ));
            }
            idx += indices[i] * stride;
            stride *= self.shape[i];
        }
        Ok(idx)
    }

    pub fn reshape(&self, new_shape: Vec<usize>) -> RuntimeResult<Self> {
        let old_total: usize = self.shape.iter().product();
        let new_total: usize = new_shape.iter().product();
        if old_total != new_total {
            return Err(RuntimeError::new(
                format!("Cannot reshape: {} elements to {}", old_total, new_total),
                0,
            ));
        }
        Ok(Tensor {
            data: self.data.clone(),
            shape: new_shape,
        })
    }

    pub fn slice(&self, dim: usize, start: usize, end: usize) -> RuntimeResult<Self> {
        if dim >= self.shape.len() {
            return Err(RuntimeError::new(
                format!(
                    "Dimension {} out of bounds for rank {}",
                    dim,
                    self.shape.len()
                ),
                0,
            ));
        }
        if start > end || end > self.shape[dim] {
            return Err(RuntimeError::new(
                format!("Invalid slice [{}, {}) for dimension {}", start, end, dim),
                0,
            ));
        }

        let new_dim_size = end - start;
        let mut new_shape = self.shape.clone();
        new_shape[dim] = new_dim_size;

        let mut new_data = Vec::new();
        let stride: usize = self.shape[dim + 1..].iter().product();
        let block_size: usize = self.shape[dim..].iter().product();
        let num_blocks: usize = self.shape[..dim].iter().product();

        for block in 0..num_blocks {
            let block_start = block * block_size;
            for i in start..end {
                let slice_start = block_start + i * stride;
                new_data.extend_from_slice(&self.data[slice_start..slice_start + stride]);
            }
        }

        Ok(Tensor {
            data: new_data,
            shape: new_shape,
        })
    }

    pub fn concat(&self, other: &Tensor, dim: usize) -> RuntimeResult<Self> {
        if self.shape.len() != other.shape.len() {
            return Err(RuntimeError::new(
                "Cannot concat tensors of different ranks",
                0,
            ));
        }
        for i in 0..self.shape.len() {
            if i != dim && self.shape[i] != other.shape[i] {
                return Err(RuntimeError::new(
                    "Cannot concat tensors with incompatible shapes",
                    0,
                ));
            }
        }

        let mut new_shape = self.shape.clone();
        new_shape[dim] = self.shape[dim] + other.shape[dim];

        let mut new_data = Vec::with_capacity(self.data.len() + other.data.len());

        let stride: usize = self.shape[dim + 1..].iter().product();
        let block_size: usize = self.shape[dim..].iter().product();
        let num_blocks: usize = self.shape[..dim].iter().product();

        for block in 0..num_blocks {
            let block_start = block * block_size;
            new_data
                .extend_from_slice(&self.data[block_start..block_start + self.shape[dim] * stride]);
            let other_block_start = block * other.shape[dim..].iter().product::<usize>();
            new_data.extend_from_slice(
                &other.data[other_block_start..other_block_start + other.shape[dim] * stride],
            );
        }

        Ok(Tensor {
            data: new_data,
            shape: new_shape,
        })
    }

    pub fn add(&self, other: &Tensor) -> RuntimeResult<Self> {
        self.binary_op(other, |a, b| a + b)
    }

    pub fn sub(&self, other: &Tensor) -> RuntimeResult<Self> {
        self.binary_op(other, |a, b| a - b)
    }

    pub fn mul(&self, other: &Tensor) -> RuntimeResult<Self> {
        self.binary_op(other, |a, b| a * b)
    }

    pub fn div(&self, other: &Tensor) -> RuntimeResult<Self> {
        self.binary_op(other, |a, b| a / b)
    }

    fn binary_op<F>(&self, other: &Tensor, op: F) -> RuntimeResult<Self>
    where
        F: Fn(f64, f64) -> f64,
    {
        if self.shape == other.shape {
            let data: Vec<f64> = self
                .data
                .iter()
                .zip(other.data.iter())
                .map(|(a, b)| op(*a, *b))
                .collect();
            Ok(Tensor {
                data,
                shape: self.shape.clone(),
            })
        } else if other.shape.is_empty() {
            let scalar = other.data[0];
            let data: Vec<f64> = self.data.iter().map(|a| op(*a, scalar)).collect();
            Ok(Tensor {
                data,
                shape: self.shape.clone(),
            })
        } else if self.shape.is_empty() {
            let scalar = self.data[0];
            let data: Vec<f64> = other.data.iter().map(|b| op(scalar, *b)).collect();
            Ok(Tensor {
                data,
                shape: other.shape.clone(),
            })
        } else {
            Err(RuntimeError::new(
                format!(
                    "Cannot broadcast shapes {:?} and {:?}",
                    self.shape, other.shape
                ),
                0,
            ))
        }
    }

    pub fn matmul(&self, other: &Tensor) -> RuntimeResult<Self> {
        if self.shape.len() != 2 || other.shape.len() != 2 {
            return Err(RuntimeError::new("Matmul requires 2D tensors", 0));
        }
        let (m, k1) = (self.shape[0], self.shape[1]);
        let (k2, n) = (other.shape[0], other.shape[1]);
        if k1 != k2 {
            return Err(RuntimeError::new(
                format!("Matmul shape mismatch: ({}, {}) x ({}, {})", m, k1, k2, n),
                0,
            ));
        }

        let k = k1;
        let mut result = Tensor::zeros(vec![m, n]);

        for i in 0..m {
            for j in 0..n {
                let mut sum = 0.0;
                for p in 0..k {
                    sum += self.data[i * k + p] * other.data[p * n + j];
                }
                result.data[i * n + j] = sum;
            }
        }

        Ok(result)
    }

    pub fn transpose(&self) -> RuntimeResult<Self> {
        match self.shape.len() {
            1 => Ok(self.clone()),
            2 => {
                let (m, n) = (self.shape[0], self.shape[1]);
                let mut result = Tensor::zeros(vec![n, m]);
                for i in 0..m {
                    for j in 0..n {
                        result.data[j * m + i] = self.data[i * n + j];
                    }
                }
                Ok(result)
            }
            _ => Err(RuntimeError::new(
                "Transpose only supports 1D and 2D tensors",
                0,
            )),
        }
    }

    pub fn sum(&self) -> f64 {
        self.data.iter().sum()
    }

    pub fn mean(&self) -> f64 {
        if self.data.is_empty() {
            return 0.0;
        }
        self.sum() / self.data.len() as f64
    }

    pub fn max(&self) -> Option<f64> {
        self.data
            .iter()
            .copied()
            .fold(None, |acc, x| Some(acc.map_or(x, |m: f64| m.max(x))))
    }

    pub fn min(&self) -> Option<f64> {
        self.data
            .iter()
            .copied()
            .fold(None, |acc, x| Some(acc.map_or(x, |m: f64| m.min(x))))
    }

    pub fn softmax(&self) -> RuntimeResult<Self> {
        let max_val = self.max().unwrap_or(0.0);
        let exp_vals: Vec<f64> = self.data.iter().map(|x| (x - max_val).exp()).collect();
        let sum: f64 = exp_vals.iter().sum();
        if sum == 0.0 {
            return Err(RuntimeError::new("Softmax sum is zero", 0));
        }
        Ok(Tensor {
            data: exp_vals.iter().map(|x| x / sum).collect(),
            shape: self.shape.clone(),
        })
    }

    pub fn relu(&self) -> Self {
        Tensor {
            data: self.data.iter().map(|x| x.max(0.0)).collect(),
            shape: self.shape.clone(),
        }
    }

    pub fn gelu(&self) -> Self {
        Tensor {
            data: self
                .data
                .iter()
                .map(|x| x * 0.5 * (1.0 + (x / 2.0f64.sqrt()).tanh()))
                .collect(),
            shape: self.shape.clone(),
        }
    }

    pub fn to_value(&self) -> Value {
        Value::Tensor(self.clone())
    }

    pub fn from_value(value: &Value) -> Option<&Self> {
        match value {
            Value::Tensor(t) => Some(t),
            _ => None,
        }
    }
}

impl fmt::Display for Tensor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.shape.is_empty() {
            return write!(f, "{}", self.data[0]);
        }
        write!(f, "Tensor{:?}", self.shape)?;
        if self.data.len() <= 10 {
            write!(f, "[")?;
            for (i, v) in self.data.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{:.4}", v)?;
            }
            write!(f, "]")
        } else {
            write!(
                f,
                "[{:.4}, {:.4}, ..., {:.4}]",
                self.data[0],
                self.data[1],
                self.data[self.data.len() - 1]
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn setup() {
        reset_global_allocation();
        set_global_limit(DEFAULT_MAX_TENSOR_ELEMENTS);
    }

    #[test]
    #[serial]
    #[serial]
    fn test_tensor_create() {
        setup();
        let t = Tensor::zeros(vec![3, 4]);
        assert_eq!(t.shape, vec![3, 4]);
        assert_eq!(t.data.len(), 12);
    }

    #[test]
    #[serial]
    fn test_tensor_get_set() {
        setup();
        let mut t = Tensor::zeros(vec![2, 3]);
        t.set(&[1, 2], 42.0).unwrap();
        assert_eq!(t.get(&[1, 2]).unwrap(), 42.0);
    }

    #[test]
    #[serial]
    fn test_tensor_reshape() {
        setup();
        let t = Tensor::from_flat(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        let reshaped = t.reshape(vec![2, 3]).unwrap();
        assert_eq!(reshaped.shape, vec![2, 3]);
    }

    #[test]
    #[serial]
    fn test_tensor_add() {
        setup();
        let a = Tensor::from_flat(vec![1.0, 2.0, 3.0]);
        let b = Tensor::from_flat(vec![4.0, 5.0, 6.0]);
        let c = a.add(&b).unwrap();
        assert_eq!(c.data, vec![5.0, 7.0, 9.0]);
    }

    #[test]
    #[serial]
    fn test_tensor_matmul() {
        setup();
        let a = Tensor::from_data(vec![2, 3], vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]).unwrap();
        let b = Tensor::from_data(vec![3, 2], vec![7.0, 8.0, 9.0, 10.0, 11.0, 12.0]).unwrap();
        let c = a.matmul(&b).unwrap();
        assert_eq!(c.shape, vec![2, 2]);
        assert_eq!(c.data[0], 58.0);
        assert_eq!(c.data[1], 64.0);
    }

    #[test]
    #[serial]
    fn test_tensor_softmax() {
        setup();
        let t = Tensor::from_flat(vec![1.0, 2.0, 3.0]);
        let s = t.softmax().unwrap();
        let sum: f64 = s.data.iter().sum();
        assert!((sum - 1.0).abs() < 1e-10);
    }

    #[test]
    #[serial]
    fn test_tensor_size_limit_rejected() {
        setup();
        let limits = TensorLimits::new(100);

        let result = Tensor::new_with_limits(vec![10, 10], &limits);
        assert!(result.is_ok());

        let result = Tensor::new_with_limits(vec![10, 11], &limits);
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_tensor_rank_limit_rejected() {
        setup();
        let limits = TensorLimits {
            max_elements: 1000,
            max_rank: 3,
            max_dimension: 100,
        };

        let result = Tensor::new_with_limits(vec![2, 2, 2], &limits);
        assert!(result.is_ok());

        let result = Tensor::new_with_limits(vec![2, 2, 2, 2, 2], &limits);
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_tensor_dimension_limit_rejected() {
        setup();
        let limits = TensorLimits {
            max_elements: 1_000_000,
            max_rank: 4,
            max_dimension: 100,
        };

        let result = Tensor::new_with_limits(vec![100, 100], &limits);
        assert!(result.is_ok());

        let result = Tensor::new_with_limits(vec![101, 10], &limits);
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_global_allocation_limit() {
        setup();
        set_global_limit(100);

        {
            let _t1 = Tensor::zeros_with_limits(vec![10], &TensorLimits::default());
            assert_eq!(get_global_allocation(), 10);

            let _t2 = Tensor::zeros_with_limits(vec![20], &TensorLimits::default());
            assert_eq!(get_global_allocation(), 30);

            let _t3 = Tensor::zeros_with_limits(vec![30], &TensorLimits::default());
            assert_eq!(get_global_allocation(), 60);
        }

        assert_eq!(get_global_allocation(), 0);

        let t4 = Tensor::zeros_with_limits(vec![50], &TensorLimits::default());
        assert!(t4.is_ok());
    }

    #[test]
    #[serial]
    fn test_allocation_tracking() {
        setup();
        assert_eq!(get_global_allocation(), 0);

        {
            let _t = Tensor::zeros(vec![5, 5]);
            assert_eq!(get_global_allocation(), 25);
        }

        assert_eq!(get_global_allocation(), 0);

        let _t2 = Tensor::from_flat(vec![1.0, 2.0, 3.0]);
        assert_eq!(get_global_allocation(), 3);

        let _t3 = Tensor::scalar(42.0);
        assert_eq!(get_global_allocation(), 4);
    }

    #[test]
    #[serial]
    fn test_tensor_shape_overflow() {
        setup();
        let limits = TensorLimits {
            max_elements: 1_000_000_000,
            max_rank: 8,
            max_dimension: 1_000_000_000,
        };

        let result = Tensor::new_with_limits(vec![1_000_000, 1_000_000], &limits);
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_image_from_bytes_png() {
        setup();
        let mut tensor = Tensor::zeros(vec![3, 2, 2]);
        tensor.data[0] = 1.0;
        tensor.data[1] = 0.5;
        tensor.data[2] = 0.0;
        tensor.data[3] = 0.25;
        tensor.data[4] = 0.75;
        tensor.data[5] = 0.5;
        tensor.data[6] = 0.0;
        tensor.data[7] = 0.125;
        tensor.data[8] = 0.875;
        tensor.data[9] = 0.625;
        tensor.data[10] = 0.375;
        tensor.data[11] = 0.0;

        let png_bytes = tensor.to_image_bytes(ImageFormat::Png).unwrap();
        assert!(!png_bytes.is_empty());
        assert!(png_bytes[0..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);

        let recovered = Tensor::from_image_bytes(&png_bytes).unwrap();
        assert_eq!(recovered.shape.len(), 3);
        assert_eq!(recovered.shape[0], 3);
        assert_eq!(recovered.shape[1], 2);
        assert_eq!(recovered.shape[2], 2);
    }

    #[test]
    #[serial]
    fn test_image_roundtrip_rgb() {
        setup();
        let mut tensor = Tensor::zeros(vec![3, 4, 4]);
        for i in 0..48 {
            tensor.data[i] = (i as f64) / 48.0;
        }

        let png_bytes = tensor.to_image_bytes(ImageFormat::Png).unwrap();
        assert!(!png_bytes.is_empty());
        assert!(png_bytes[0..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);

        let recovered = Tensor::from_image_bytes(&png_bytes).unwrap();
        assert_eq!(recovered.shape, tensor.shape);

        for i in 0..tensor.data.len() {
            let diff = (tensor.data[i] - recovered.data[i]).abs();
            assert!(diff < 0.02, "Pixel {} differs by {}", i, diff);
        }
    }

    #[test]
    #[serial]
    fn test_image_roundtrip_grayscale() {
        setup();
        let mut tensor = Tensor::zeros(vec![1, 8, 8]);
        for i in 0..64 {
            tensor.data[i] = (i as f64) / 64.0;
        }

        let png_bytes = tensor.to_image_bytes(ImageFormat::Png).unwrap();
        let recovered = Tensor::from_image_bytes(&png_bytes).unwrap();

        assert_eq!(recovered.shape[0], 3);
        let (h, w, _c) = recovered.image_dimensions().unwrap();
        assert_eq!(h, 8);
        assert_eq!(w, 8);
    }

    #[test]
    #[serial]
    fn test_image_dimensions() {
        setup();
        let tensor = Tensor::zeros(vec![3, 100, 200]);
        let (h, w, c) = tensor.image_dimensions().unwrap();
        assert_eq!(h, 100);
        assert_eq!(w, 200);
        assert_eq!(c, 3);
    }

    #[test]
    #[serial]
    fn test_image_invalid_shape() {
        setup();
        let tensor = Tensor::zeros(vec![4, 5, 5]);
        let result = tensor.to_image_bytes(ImageFormat::Png);
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_image_invalid_rank() {
        setup();
        let tensor = Tensor::zeros(vec![10, 10]);
        let result = tensor.to_image_bytes(ImageFormat::Png);
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_audio_roundtrip_mono() {
        setup();
        let mut tensor = Tensor::zeros(vec![1, 100]);
        for i in 0..100 {
            tensor.data[i] = (i as f64 / 100.0 * 2.0 - 1.0).sin();
        }

        let wav_bytes = tensor.to_audio_bytes(44100).unwrap();
        assert!(!wav_bytes.is_empty());
        assert!(wav_bytes[0..4] == [b'R', b'I', b'F', b'F']);
        assert!(wav_bytes[8..12] == [b'W', b'A', b'V', b'E']);

        let recovered = Tensor::from_audio_bytes(&wav_bytes).unwrap();
        assert_eq!(recovered.shape.len(), 2);
        assert_eq!(recovered.shape[0], 1);

        for i in 0..tensor.data.len() {
            let diff = (tensor.data[i] - recovered.data[i]).abs();
            assert!(diff < 0.001, "Sample {} differs by {}", i, diff);
        }
    }

    #[test]
    #[serial]
    fn test_audio_roundtrip_stereo() {
        setup();
        let mut tensor = Tensor::zeros(vec![2, 50]);
        for i in 0..50 {
            tensor.data[i] = (i as f64 / 50.0);
            tensor.data[50 + i] = 1.0 - (i as f64 / 50.0);
        }

        let wav_bytes = tensor.to_audio_bytes(44100).unwrap();
        let recovered = Tensor::from_audio_bytes(&wav_bytes).unwrap();

        assert_eq!(recovered.shape, tensor.shape);
    }

    #[test]
    #[serial]
    fn test_audio_info() {
        setup();
        let tensor = Tensor::zeros(vec![2, 1000]);
        let (channels, num_samples) = tensor.audio_info().unwrap();
        assert_eq!(channels, 2);
        assert_eq!(num_samples, 1000);
    }

    #[test]
    #[serial]
    fn test_audio_invalid_rank() {
        setup();
        let tensor = Tensor::zeros(vec![10, 10, 10]);
        let result = tensor.to_audio_bytes(44100);
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_audio_invalid_shape_1d() {
        setup();
        let tensor = Tensor::zeros(vec![100]);
        let result = tensor.to_audio_bytes(44100);
        assert!(result.is_err());
    }
}
