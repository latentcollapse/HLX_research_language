//! GPU Dispatch Layer — Phase 17
//!
//! Provides a unified interface for tensor operations that can dispatch to either
//! GPU (Vulkan/wgpu) or CPU. The GPU backend is behind the `gpu` feature flag.
//!
//! Design per HLX-S:
//! - Single GPU only (no PCIe sync drift)
//! - All shaders must pass attestation before execution
//! - Transparent CPU fallback when GPU unavailable
//! - Deterministic: same inputs → same outputs regardless of backend

use crate::shader_attestation::ShaderRegistry;

/// Backend selection for tensor compute
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComputeBackend {
    /// CPU-only (always available)
    Cpu,
    /// GPU via wgpu/Vulkan (requires `gpu` feature)
    Gpu,
}

/// Result of a GPU probe
#[derive(Debug, Clone)]
pub struct GpuInfo {
    pub available: bool,
    pub device_name: String,
    pub backend: String,
    pub max_buffer_size: usize,
}

/// GPU dispatch context
pub struct GpuDispatch {
    backend: ComputeBackend,
    gpu_info: Option<GpuInfo>,
    shader_registry: ShaderRegistry,
}

impl GpuDispatch {
    /// Create a new GPU dispatch context.
    /// Probes for available GPU; falls back to CPU if none found.
    pub fn new() -> Self {
        let (backend, gpu_info) = Self::probe_gpu();

        GpuDispatch {
            backend,
            gpu_info,
            shader_registry: ShaderRegistry::new(),
        }
    }

    /// Force CPU-only mode
    pub fn cpu_only() -> Self {
        GpuDispatch {
            backend: ComputeBackend::Cpu,
            gpu_info: None,
            shader_registry: ShaderRegistry::new(),
        }
    }

    /// Current compute backend
    pub fn backend(&self) -> ComputeBackend {
        self.backend
    }

    /// GPU info (if available)
    pub fn gpu_info(&self) -> Option<&GpuInfo> {
        self.gpu_info.as_ref()
    }

    /// Access shader registry for attestation
    pub fn shader_registry(&self) -> &ShaderRegistry {
        &self.shader_registry
    }

    /// Mutable access to shader registry
    pub fn shader_registry_mut(&mut self) -> &mut ShaderRegistry {
        &mut self.shader_registry
    }

    /// Dispatch a tensor blend operation: result = a * (1 - alpha) + b * alpha
    pub fn tensor_blend(&self, a: &[f64], b: &[f64], alpha: f64) -> Vec<f64> {
        assert_eq!(a.len(), b.len(), "tensor_blend: mismatched lengths");

        match self.backend {
            ComputeBackend::Gpu => {
                // GPU path would dispatch a compute shader here.
                // For now, falls through to CPU (transparent fallback).
                self.cpu_tensor_blend(a, b, alpha)
            }
            ComputeBackend::Cpu => self.cpu_tensor_blend(a, b, alpha),
        }
    }

    /// Dispatch a tensor normalize operation (L2 norm)
    pub fn tensor_normalize(&self, data: &[f64]) -> Vec<f64> {
        match self.backend {
            ComputeBackend::Gpu => self.cpu_tensor_normalize(data),
            ComputeBackend::Cpu => self.cpu_tensor_normalize(data),
        }
    }

    /// Dispatch a tensor convolution
    pub fn tensor_convolve(&self, signal: &[f64], kernel: &[f64]) -> Vec<f64> {
        match self.backend {
            ComputeBackend::Gpu => self.cpu_tensor_convolve(signal, kernel),
            ComputeBackend::Cpu => self.cpu_tensor_convolve(signal, kernel),
        }
    }

    /// Dispatch a dot product
    pub fn tensor_dot(&self, a: &[f64], b: &[f64]) -> f64 {
        assert_eq!(a.len(), b.len(), "tensor_dot: mismatched lengths");

        match self.backend {
            ComputeBackend::Gpu => self.cpu_tensor_dot(a, b),
            ComputeBackend::Cpu => self.cpu_tensor_dot(a, b),
        }
    }

    /// Dispatch element-wise multiply
    pub fn tensor_mul(&self, a: &[f64], b: &[f64]) -> Vec<f64> {
        assert_eq!(a.len(), b.len(), "tensor_mul: mismatched lengths");

        match self.backend {
            ComputeBackend::Gpu => self.cpu_tensor_mul(a, b),
            ComputeBackend::Cpu => self.cpu_tensor_mul(a, b),
        }
    }

    /// Dispatch scalar multiply
    pub fn tensor_scale(&self, data: &[f64], scalar: f64) -> Vec<f64> {
        match self.backend {
            ComputeBackend::Gpu => self.cpu_tensor_scale(data, scalar),
            ComputeBackend::Cpu => self.cpu_tensor_scale(data, scalar),
        }
    }

    // ---- CPU implementations (always available) ----

    fn cpu_tensor_blend(&self, a: &[f64], b: &[f64], alpha: f64) -> Vec<f64> {
        let inv = 1.0 - alpha;
        a.iter().zip(b.iter()).map(|(x, y)| x * inv + y * alpha).collect()
    }

    fn cpu_tensor_normalize(&self, data: &[f64]) -> Vec<f64> {
        let norm: f64 = data.iter().map(|x| x * x).sum::<f64>().sqrt();
        if norm < f64::EPSILON {
            return vec![0.0; data.len()];
        }
        data.iter().map(|x| x / norm).collect()
    }

    fn cpu_tensor_convolve(&self, signal: &[f64], kernel: &[f64]) -> Vec<f64> {
        if signal.is_empty() || kernel.is_empty() {
            return Vec::new();
        }
        let out_len = signal.len() + kernel.len() - 1;
        let mut result = vec![0.0; out_len];
        for (i, &s) in signal.iter().enumerate() {
            for (j, &k) in kernel.iter().enumerate() {
                result[i + j] += s * k;
            }
        }
        result
    }

    fn cpu_tensor_dot(&self, a: &[f64], b: &[f64]) -> f64 {
        a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
    }

    fn cpu_tensor_mul(&self, a: &[f64], b: &[f64]) -> Vec<f64> {
        a.iter().zip(b.iter()).map(|(x, y)| x * y).collect()
    }

    fn cpu_tensor_scale(&self, data: &[f64], scalar: f64) -> Vec<f64> {
        data.iter().map(|x| x * scalar).collect()
    }

    // ---- GPU probe ----

    #[cfg(feature = "gpu")]
    fn probe_gpu() -> (ComputeBackend, Option<GpuInfo>) {
        // When the `gpu` feature is enabled, attempt wgpu initialization.
        // This is where wgpu::Instance::new() and adapter enumeration would go.
        // For now, return CPU fallback until wgpu is wired.
        (ComputeBackend::Cpu, None)
    }

    #[cfg(not(feature = "gpu"))]
    fn probe_gpu() -> (ComputeBackend, Option<GpuInfo>) {
        (ComputeBackend::Cpu, None)
    }
}

impl Default for GpuDispatch {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_fallback_default() {
        let dispatch = GpuDispatch::new();
        // Without the gpu feature, always CPU
        assert_eq!(dispatch.backend(), ComputeBackend::Cpu);
    }

    #[test]
    fn test_blend() {
        let dispatch = GpuDispatch::cpu_only();
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 5.0, 6.0];
        let result = dispatch.tensor_blend(&a, &b, 0.5);
        assert_eq!(result, vec![2.5, 3.5, 4.5]);
    }

    #[test]
    fn test_blend_alpha_zero() {
        let dispatch = GpuDispatch::cpu_only();
        let a = vec![1.0, 2.0];
        let b = vec![9.0, 9.0];
        let result = dispatch.tensor_blend(&a, &b, 0.0);
        assert_eq!(result, vec![1.0, 2.0]);
    }

    #[test]
    fn test_blend_alpha_one() {
        let dispatch = GpuDispatch::cpu_only();
        let a = vec![1.0, 2.0];
        let b = vec![9.0, 9.0];
        let result = dispatch.tensor_blend(&a, &b, 1.0);
        assert_eq!(result, vec![9.0, 9.0]);
    }

    #[test]
    fn test_normalize() {
        let dispatch = GpuDispatch::cpu_only();
        let data = vec![3.0, 4.0];
        let result = dispatch.tensor_normalize(&data);
        let expected_norm = 5.0;
        assert!((result[0] - 3.0 / expected_norm).abs() < 1e-10);
        assert!((result[1] - 4.0 / expected_norm).abs() < 1e-10);
    }

    #[test]
    fn test_normalize_zero_vector() {
        let dispatch = GpuDispatch::cpu_only();
        let data = vec![0.0, 0.0, 0.0];
        let result = dispatch.tensor_normalize(&data);
        assert_eq!(result, vec![0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_convolve() {
        let dispatch = GpuDispatch::cpu_only();
        let signal = vec![1.0, 2.0, 3.0];
        let kernel = vec![0.5, 0.5];
        let result = dispatch.tensor_convolve(&signal, &kernel);
        assert_eq!(result.len(), 4);
        assert!((result[0] - 0.5).abs() < 1e-10);
        assert!((result[1] - 1.5).abs() < 1e-10);
        assert!((result[2] - 2.5).abs() < 1e-10);
        assert!((result[3] - 1.5).abs() < 1e-10);
    }

    #[test]
    fn test_dot_product() {
        let dispatch = GpuDispatch::cpu_only();
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 5.0, 6.0];
        let result = dispatch.tensor_dot(&a, &b);
        assert_eq!(result, 32.0);
    }

    #[test]
    fn test_element_multiply() {
        let dispatch = GpuDispatch::cpu_only();
        let a = vec![2.0, 3.0];
        let b = vec![4.0, 5.0];
        let result = dispatch.tensor_mul(&a, &b);
        assert_eq!(result, vec![8.0, 15.0]);
    }

    #[test]
    fn test_scale() {
        let dispatch = GpuDispatch::cpu_only();
        let data = vec![1.0, 2.0, 3.0];
        let result = dispatch.tensor_scale(&data, 2.0);
        assert_eq!(result, vec![2.0, 4.0, 6.0]);
    }

    #[test]
    fn test_shader_registry_access() {
        let mut dispatch = GpuDispatch::cpu_only();
        let data = b"test compute shader";
        let hash = ShaderRegistry::compute_hash(data);
        dispatch.shader_registry_mut().register("test_cs", data, &hash).unwrap();
        assert_eq!(dispatch.shader_registry().count(), 1);
    }
}
