//! Operation Dispatcher - Route tensor ops to available backends
//!
//! Provides a unified interface for tensor operations that automatically
//! selects the best available backend (Vulkan GPU, CPU fallback).

use crate::{RuntimeError, RuntimeResult, Tensor};

#[derive(Debug, Clone)]
pub enum TensorOp {
    Add,
    Sub,
    Mul,
    Div,
    MatMul,
    Softmax,
    Relu,
    Sigmoid,
    Tanh,
    LayerNorm { eps: f64 },
    Reshape { shape: Vec<usize> },
    Transpose { dims: Vec<usize> },
    Concat { axis: usize },
    Split { axis: usize, indices: Vec<usize> },
}

#[derive(Debug, Clone)]
pub enum ImageOp {
    GaussianBlur { sigma: f32 },
    SobelEdges,
    Grayscale,
    Threshold { value: f32 },
    Brightness { delta: f32 },
    Contrast { factor: f32 },
    Invert,
    Sharpen { amount: f32 },
}

#[derive(Debug, Clone)]
pub enum Backend {
    Cpu,
    Vulkan,
}

pub struct Dispatcher {
    preferred_backend: Backend,
    vulkan_available: bool,
}

impl Default for Dispatcher {
    fn default() -> Self {
        Dispatcher {
            preferred_backend: Backend::Cpu,
            vulkan_available: false,
        }
    }
}

impl Dispatcher {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_backend(mut self, backend: Backend) -> Self {
        self.preferred_backend = backend;
        self
    }

    pub fn detect_vulkan(&mut self) -> bool {
        self.vulkan_available = Self::check_vulkan_available();
        if self.vulkan_available {
            self.preferred_backend = Backend::Vulkan;
        }
        self.vulkan_available
    }

    fn check_vulkan_available() -> bool {
        // Vulkan backend exists in backends/ directory
        // Runtime detection would check for GPU device
        // For now, return false until Vulkan is wired up
        std::env::var("HLX_VULKAN").is_ok()
    }

    pub fn dispatch_tensor(&self, op: &TensorOp, inputs: &[&Tensor]) -> RuntimeResult<Tensor> {
        match self.preferred_backend {
            Backend::Vulkan if self.vulkan_available => self.dispatch_vulkan_tensor(op, inputs),
            _ => self.dispatch_cpu_tensor(op, inputs),
        }
    }

    pub fn dispatch_image(&self, op: &ImageOp, input: &Tensor) -> RuntimeResult<Tensor> {
        match self.preferred_backend {
            Backend::Vulkan if self.vulkan_available => self.dispatch_vulkan_image(op, input),
            _ => self.dispatch_cpu_image(op, input),
        }
    }

    fn dispatch_cpu_tensor(&self, op: &TensorOp, inputs: &[&Tensor]) -> RuntimeResult<Tensor> {
        match op {
            TensorOp::Add => {
                if inputs.len() != 2 {
                    return Err(RuntimeError::new("Add requires 2 inputs", 0));
                }
                inputs[0].add(inputs[1])
            }
            TensorOp::Sub => {
                if inputs.len() != 2 {
                    return Err(RuntimeError::new("Sub requires 2 inputs", 0));
                }
                inputs[0].sub(inputs[1])
            }
            TensorOp::Mul => {
                if inputs.len() != 2 {
                    return Err(RuntimeError::new("Mul requires 2 inputs", 0));
                }
                inputs[0].mul(inputs[1])
            }
            TensorOp::Div => {
                if inputs.len() != 2 {
                    return Err(RuntimeError::new("Div requires 2 inputs", 0));
                }
                inputs[0].div(inputs[1])
            }
            TensorOp::MatMul => {
                if inputs.len() != 2 {
                    return Err(RuntimeError::new("MatMul requires 2 inputs", 0));
                }
                inputs[0].matmul(inputs[1])
            }
            TensorOp::Softmax => {
                if inputs.len() != 1 {
                    return Err(RuntimeError::new("Softmax requires 1 input", 0));
                }
                inputs[0].softmax()
            }
            TensorOp::Relu => {
                if inputs.len() != 1 {
                    return Err(RuntimeError::new("Relu requires 1 input", 0));
                }
                Ok(inputs[0].map_values(|v| v.max(0.0)))
            }
            TensorOp::Sigmoid => {
                if inputs.len() != 1 {
                    return Err(RuntimeError::new("Sigmoid requires 1 input", 0));
                }
                Ok(inputs[0].map_values(|v| 1.0 / (1.0 + (-v).exp())))
            }
            TensorOp::Tanh => {
                if inputs.len() != 1 {
                    return Err(RuntimeError::new("Tanh requires 1 input", 0));
                }
                Ok(inputs[0].map_values(|v| v.tanh()))
            }
            TensorOp::LayerNorm { eps } => {
                if inputs.len() != 1 {
                    return Err(RuntimeError::new("LayerNorm requires 1 input", 0));
                }
                Self::layer_norm(inputs[0], *eps)
            }
            TensorOp::Reshape { shape } => {
                if inputs.len() != 1 {
                    return Err(RuntimeError::new("Reshape requires 1 input", 0));
                }
                inputs[0].reshape(shape.clone())
            }
            TensorOp::Transpose { dims } => {
                if inputs.len() != 1 {
                    return Err(RuntimeError::new("Transpose requires 1 input", 0));
                }
                Self::transpose(inputs[0], dims)
            }
            TensorOp::Concat { axis } => Self::concat(inputs, *axis),
            TensorOp::Split { axis, indices } => Err(RuntimeError::new(
                "Split returns multiple tensors, use split_tensors",
                0,
            )),
        }
    }

    fn dispatch_vulkan_tensor(&self, op: &TensorOp, inputs: &[&Tensor]) -> RuntimeResult<Tensor> {
        // Vulkan backend would be called here
        // For now, fall back to CPU
        self.dispatch_cpu_tensor(op, inputs)
    }

    fn dispatch_cpu_image(&self, op: &ImageOp, input: &Tensor) -> RuntimeResult<Tensor> {
        if input.shape.len() != 3 {
            return Err(RuntimeError::new(
                format!(
                    "Image op requires CHW tensor (3 dims), got {} dims",
                    input.shape.len()
                ),
                0,
            ));
        }

        let (channels, height, width) = (input.shape[0], input.shape[1], input.shape[2]);

        match op {
            ImageOp::Grayscale => {
                if channels != 3 {
                    return Err(RuntimeError::new("Grayscale requires 3 channels", 0));
                }
                let mut gray = vec![0.0f64; height * width];
                for y in 0..height {
                    for x in 0..width {
                        let idx = y * width + x;
                        let r = input.data[idx];
                        let g = input.data[height * width + idx];
                        let b = input.data[2 * height * width + idx];
                        gray[idx] = 0.299 * r + 0.587 * g + 0.114 * b;
                    }
                }
                Ok(Tensor::from_data(vec![1, height, width], gray)?)
            }

            ImageOp::Invert => {
                let inverted: Vec<f64> = input.data.iter().map(|v| 1.0 - v).collect();
                Ok(Tensor::from_data(input.shape.clone(), inverted)?)
            }

            ImageOp::Brightness { delta } => {
                let adjusted: Vec<f64> = input
                    .data
                    .iter()
                    .map(|v| (v + delta).clamp(0.0, 1.0))
                    .collect();
                Ok(Tensor::from_data(input.shape.clone(), adjusted)?)
            }

            ImageOp::Contrast { factor } => {
                let adjusted: Vec<f64> = input
                    .data
                    .iter()
                    .map(|v| ((v - 0.5) * factor + 0.5).clamp(0.0, 1.0))
                    .collect();
                Ok(Tensor::from_data(input.shape.clone(), adjusted)?)
            }

            ImageOp::Threshold { value } => {
                let thresholded: Vec<f64> = input
                    .data
                    .iter()
                    .map(|v| if *v > *value as f64 { 1.0 } else { 0.0 })
                    .collect();
                Ok(Tensor::from_data(input.shape.clone(), thresholded)?)
            }

            ImageOp::GaussianBlur { sigma } => Self::gaussian_blur(input, *sigma),

            ImageOp::SobelEdges => Self::sobel_edges(input),

            ImageOp::Sharpen { amount } => Self::sharpen(input, *amount),
        }
    }

    fn dispatch_vulkan_image(&self, op: &ImageOp, input: &Tensor) -> RuntimeResult<Tensor> {
        // Vulkan backend would call shaders here
        // For now, fall back to CPU
        self.dispatch_cpu_image(op, input)
    }

    fn layer_norm(tensor: &Tensor, eps: f64) -> RuntimeResult<Tensor> {
        let mean: f64 = tensor.data.iter().sum::<f64>() / tensor.data.len() as f64;
        let var: f64 =
            tensor.data.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / tensor.data.len() as f64;
        let std = (var + eps).sqrt();

        let normalized: Vec<f64> = tensor.data.iter().map(|v| (v - mean) / std).collect();
        Tensor::from_data(tensor.shape.clone(), normalized)
    }

    fn transpose(tensor: &Tensor, dims: &[usize]) -> RuntimeResult<Tensor> {
        if dims.len() != tensor.shape.len() {
            return Err(RuntimeError::new(
                format!(
                    "Transpose dims length mismatch: {} vs {}",
                    dims.len(),
                    tensor.shape.len()
                ),
                0,
            ));
        }

        let new_shape: Vec<usize> = dims.iter().map(|&i| tensor.shape[i]).collect();
        let mut new_data = vec![0.0; tensor.data.len()];

        // Simple 2D transpose for now
        if tensor.shape.len() == 2 && dims == [1, 0] {
            let (rows, cols) = (tensor.shape[0], tensor.shape[1]);
            for i in 0..rows {
                for j in 0..cols {
                    new_data[j * rows + i] = tensor.data[i * cols + j];
                }
            }
        } else {
            return Err(RuntimeError::new(
                "Complex transpose not yet implemented",
                0,
            ));
        }

        Tensor::from_data(new_shape, new_data)
    }

    fn concat(tensors: &[&Tensor], axis: usize) -> RuntimeResult<Tensor> {
        if tensors.is_empty() {
            return Err(RuntimeError::new("Concat requires at least 1 tensor", 0));
        }

        if axis >= tensors[0].shape.len() {
            return Err(RuntimeError::new(
                format!("Concat axis {} out of bounds", axis),
                0,
            ));
        }

        // Verify shapes match on non-concat axes
        for (i, t) in tensors.iter().enumerate().skip(1) {
            if t.shape.len() != tensors[0].shape.len() {
                return Err(RuntimeError::new(
                    format!("Tensor {} has different rank", i),
                    0,
                ));
            }
            for (d, (&a, &b)) in tensors[0].shape.iter().zip(t.shape.iter()).enumerate() {
                if d != axis && a != b {
                    return Err(RuntimeError::new(
                        format!("Shape mismatch on dimension {}", d),
                        0,
                    ));
                }
            }
        }

        let mut new_shape = tensors[0].shape.clone();
        new_shape[axis] = tensors.iter().map(|t| t.shape[axis]).sum();

        let mut new_data = Vec::with_capacity(new_shape.iter().product());
        for t in tensors {
            new_data.extend_from_slice(&t.data);
        }

        Tensor::from_data(new_shape, new_data)
    }

    fn gaussian_blur(tensor: &Tensor, sigma: f32) -> RuntimeResult<Tensor> {
        let (channels, height, width) = (tensor.shape[0], tensor.shape[1], tensor.shape[2]);
        let kernel_size = ((sigma * 4.0).ceil() as usize) | 1; // Ensure odd
        let half = kernel_size / 2;

        // Create Gaussian kernel
        let mut kernel = vec![0.0f64; kernel_size];
        let sum: f64 = {
            let mut s = 0.0;
            for i in 0..kernel_size {
                let x = (i as f64) - half as f64;
                kernel[i] = (-(x * x) / (2.0 * (sigma as f64).powi(2))).exp();
                s += kernel[i];
            }
            s
        };
        for k in &mut kernel {
            *k /= sum;
        }

        let mut blurred = tensor.data.clone();

        // Apply separable Gaussian blur (horizontal then vertical)
        for c in 0..channels {
            let offset = c * height * width;

            // Horizontal pass
            let mut temp = vec![0.0f64; height * width];
            for y in 0..height {
                for x in 0..width {
                    let mut sum = 0.0;
                    for k in 0..kernel_size {
                        let nx = (x as isize + k as isize - half as isize)
                            .max(0)
                            .min(width as isize - 1) as usize;
                        sum += tensor.data[offset + y * width + nx] * kernel[k];
                    }
                    temp[y * width + x] = sum;
                }
            }

            // Vertical pass
            for y in 0..height {
                for x in 0..width {
                    let mut sum = 0.0;
                    for k in 0..kernel_size {
                        let ny = (y as isize + k as isize - half as isize)
                            .max(0)
                            .min(height as isize - 1) as usize;
                        sum += temp[ny * width + x] * kernel[k];
                    }
                    blurred[offset + y * width + x] = sum;
                }
            }
        }

        Tensor::from_data(tensor.shape.clone(), blurred)
    }

    fn sobel_edges(tensor: &Tensor) -> RuntimeResult<Tensor> {
        let (channels, height, width) = (tensor.shape[0], tensor.shape[1], tensor.shape[2]);

        let sobel_x: [[f64; 3]; 3] = [[-1.0, 0.0, 1.0], [-2.0, 0.0, 2.0], [-1.0, 0.0, 1.0]];
        let sobel_y: [[f64; 3]; 3] = [[-1.0, -2.0, -1.0], [0.0, 0.0, 0.0], [1.0, 2.0, 1.0]];

        let mut edges = vec![0.0f64; height * width];

        for c in 0..channels {
            let offset = c * height * width;

            for y in 1..(height - 1) {
                for x in 1..(width - 1) {
                    let mut gx = 0.0;
                    let mut gy = 0.0;

                    for ky in 0..3 {
                        for kx in 0..3 {
                            let idx = offset + (y + ky - 1) * width + (x + kx - 1);
                            gx += tensor.data[idx] * sobel_x[ky][kx];
                            gy += tensor.data[idx] * sobel_y[ky][kx];
                        }
                    }

                    let magnitude = (gx * gx + gy * gy).sqrt();
                    let out_idx = y * width + x;
                    edges[out_idx] = edges[out_idx].max(magnitude);
                }
            }
        }

        Tensor::from_data(vec![1, height, width], edges)
    }

    fn sharpen(tensor: &Tensor, amount: f32) -> RuntimeResult<Tensor> {
        // Sharpen = original + amount * (original - blurred)
        let blurred = Self::gaussian_blur(tensor, 1.0)?;
        let sharpened: Vec<f64> = tensor
            .data
            .iter()
            .zip(blurred.data.iter())
            .map(|(&o, &b)| (o + amount as f64 * (o - b)).clamp(0.0, 1.0))
            .collect();

        Tensor::from_data(tensor.shape.clone(), sharpened)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dispatcher_creation() {
        let dispatcher = Dispatcher::new();
        assert_eq!(dispatcher.preferred_backend, Backend::Cpu);
    }

    #[test]
    fn test_cpu_add() {
        let dispatcher = Dispatcher::new();
        let a = Tensor::from_flat(vec![1.0, 2.0, 3.0]);
        let b = Tensor::from_flat(vec![4.0, 5.0, 6.0]);

        let result = dispatcher
            .dispatch_tensor(&TensorOp::Add, &[&a, &b])
            .unwrap();
        assert_eq!(result.data, vec![5.0, 7.0, 9.0]);
    }

    #[test]
    fn test_cpu_matmul() {
        let dispatcher = Dispatcher::new();
        let a = Tensor::from_data(vec![2, 3], vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]).unwrap();
        let b = Tensor::from_data(vec![3, 2], vec![7.0, 8.0, 9.0, 10.0, 11.0, 12.0]).unwrap();

        let result = dispatcher
            .dispatch_tensor(&TensorOp::MatMul, &[&a, &b])
            .unwrap();
        assert_eq!(result.shape, vec![2, 2]);
    }

    #[test]
    fn test_grayscale() {
        let dispatcher = Dispatcher::new();
        let rgb = Tensor::from_data(
            vec![3, 2, 2],
            vec![1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.5, 0.5, 0.5, 0.5],
        )
        .unwrap();

        let gray = dispatcher
            .dispatch_image(&ImageOp::Grayscale, &rgb)
            .unwrap();
        assert_eq!(gray.shape, vec![1, 2, 2]);
    }

    #[test]
    fn test_brightness() {
        let dispatcher = Dispatcher::new();
        let img = Tensor::from_data(vec![3, 2, 2], vec![0.5; 12]).unwrap();

        let bright = dispatcher
            .dispatch_image(&ImageOp::Brightness { delta: 0.3 }, &img)
            .unwrap();
        assert!(bright.data.iter().all(|&v| (v - 0.8).abs() < 1e-10));
    }

    #[test]
    fn test_invert() {
        let dispatcher = Dispatcher::new();
        let img = Tensor::from_data(vec![3, 1, 1], vec![0.2, 0.5, 0.8]).unwrap();

        let inverted = dispatcher.dispatch_image(&ImageOp::Invert, &img).unwrap();
        assert!((inverted.data[0] - 0.8).abs() < 1e-10);
        assert!((inverted.data[1] - 0.5).abs() < 1e-10);
        assert!((inverted.data[2] - 0.2).abs() < 1e-10);
    }
}
