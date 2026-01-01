//! GPU Operations Module
//!
//! Reusable Vulkan GPU operations that can be called by both the training binary
//! and LC-B contract executor. This module provides the actual GPU implementations
//! for contracts 903-910.

use ash::{vk, Device, Entry, Instance};
use std::ffi::CString;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;

// =============================================================================
// GPU CONTEXT
// =============================================================================

/// Shared Vulkan context for GPU operations
pub struct GpuContext {
    pub entry: Entry,
    pub instance: Instance,
    pub device: Arc<Device>,
    pub compute_queue: vk::Queue,
    pub compute_queue_family: u32,
    pub memory_properties: vk::PhysicalDeviceMemoryProperties,
    pub command_pool: vk::CommandPool,
    pub descriptor_pool: vk::DescriptorPool,
    pub pipelines: PipelineCache,
}

/// Cached compute pipelines
pub struct PipelineCache {
    pub gemm_forward: Option<vk::Pipeline>,
    pub gemm_backward: Option<vk::Pipeline>,
    pub layernorm_forward: Option<vk::Pipeline>,
    pub layernorm_backward: Option<vk::Pipeline>,
    pub gelu_forward: Option<vk::Pipeline>,
    pub gelu_backward: Option<vk::Pipeline>,
    pub softmax_forward: Option<vk::Pipeline>,
    pub cross_entropy_forward: Option<vk::Pipeline>,
    pub cross_entropy_backward: Option<vk::Pipeline>,
    pub adam_step: Option<vk::Pipeline>,
    pub elementwise: Option<vk::Pipeline>,
    pub embedding_forward: Option<vk::Pipeline>,
    pub pipeline_layout: Option<vk::PipelineLayout>,
    pub desc_set_layout: Option<vk::DescriptorSetLayout>,
}

impl Default for PipelineCache {
    fn default() -> Self {
        Self {
            gemm_forward: None,
            gemm_backward: None,
            layernorm_forward: None,
            layernorm_backward: None,
            gelu_forward: None,
            gelu_backward: None,
            softmax_forward: None,
            cross_entropy_forward: None,
            cross_entropy_backward: None,
            adam_step: None,
            elementwise: None,
            embedding_forward: None,
            pipeline_layout: None,
            desc_set_layout: None,
        }
    }
}

// =============================================================================
// PUSH CONSTANT STRUCTURES
// =============================================================================

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GemmPushConstants {
    pub m: u32,
    pub k: u32,
    pub n: u32,
    pub mode: u32,  // 0 = forward, 1 = weight gradient, 2 = input gradient
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct LayerNormPushConstants {
    pub num_rows: u32,
    pub row_size: u32,
    pub eps: f32,
    pub _pad: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GeluPushConstants {
    pub num_elements: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SoftmaxPushConstants {
    pub num_rows: u32,
    pub row_size: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CrossEntropyPushConstants {
    pub num_positions: u32,
    pub vocab_size: u32,
    pub ignore_index: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct AdamPushConstants {
    pub num_params: u32,
    pub lr: f32,
    pub beta1: f32,
    pub beta2: f32,
    pub eps: f32,
    pub beta1_t: f32,
    pub beta2_t: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ElementwisePushConstants {
    pub num_elements: u32,
    pub mode: u32,  // 0 = add, 1 = mul, 2 = copy
    pub scalar: f32,
}

// =============================================================================
// GPU BUFFER
// =============================================================================

/// GPU buffer with associated memory
pub struct GpuBuffer {
    pub buffer: vk::Buffer,
    pub memory: vk::DeviceMemory,
    pub size: u64,
}

impl GpuBuffer {
    /// Create a new GPU buffer
    pub fn new(
        device: &Device,
        memory_properties: &vk::PhysicalDeviceMemoryProperties,
        size: u64,
        usage: vk::BufferUsageFlags,
    ) -> Result<Self, String> {
        let buffer_info = vk::BufferCreateInfo::default()
            .size(size)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let buffer = unsafe { device.create_buffer(&buffer_info, None) }
            .map_err(|e| format!("Failed to create buffer: {:?}", e))?;

        let mem_requirements = unsafe { device.get_buffer_memory_requirements(buffer) };

        let memory_type_index = find_memory_type(
            memory_properties,
            mem_requirements.memory_type_bits,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        ).ok_or_else(|| "No suitable memory type found".to_string())?;

        let alloc_info = vk::MemoryAllocateInfo::default()
            .allocation_size(mem_requirements.size)
            .memory_type_index(memory_type_index);

        let memory = unsafe { device.allocate_memory(&alloc_info, None) }
            .map_err(|e| format!("Failed to allocate memory: {:?}", e))?;

        unsafe { device.bind_buffer_memory(buffer, memory, 0) }
            .map_err(|e| format!("Failed to bind buffer memory: {:?}", e))?;

        Ok(Self { buffer, memory, size })
    }

    /// Upload data to the buffer
    pub fn upload<T: Copy>(&self, device: &Device, data: &[T]) -> Result<(), String> {
        let byte_size = std::mem::size_of_val(data) as u64;
        if byte_size > self.size {
            return Err(format!("Data size {} exceeds buffer size {}", byte_size, self.size));
        }

        unsafe {
            let ptr = device
                .map_memory(self.memory, 0, byte_size, vk::MemoryMapFlags::empty())
                .map_err(|e| format!("Failed to map memory: {:?}", e))?;

            std::ptr::copy_nonoverlapping(data.as_ptr() as *const u8, ptr as *mut u8, byte_size as usize);

            device.unmap_memory(self.memory);
        }

        Ok(())
    }

    /// Download data from the buffer
    pub fn download<T: Copy + Default>(&self, device: &Device, count: usize) -> Result<Vec<T>, String> {
        let byte_size = (count * std::mem::size_of::<T>()) as u64;
        if byte_size > self.size {
            return Err(format!("Requested size {} exceeds buffer size {}", byte_size, self.size));
        }

        let mut data = vec![T::default(); count];

        unsafe {
            let ptr = device
                .map_memory(self.memory, 0, byte_size, vk::MemoryMapFlags::empty())
                .map_err(|e| format!("Failed to map memory: {:?}", e))?;

            std::ptr::copy_nonoverlapping(ptr as *const u8, data.as_mut_ptr() as *mut u8, byte_size as usize);

            device.unmap_memory(self.memory);
        }

        Ok(data)
    }

    /// Destroy the buffer
    pub fn destroy(&self, device: &Device) {
        unsafe {
            device.destroy_buffer(self.buffer, None);
            device.free_memory(self.memory, None);
        }
    }
}

// =============================================================================
// GPU TENSOR
// =============================================================================

/// A tensor stored in GPU memory
pub struct GpuTensor {
    pub buffer: GpuBuffer,
    pub shape: Vec<usize>,
    pub strides: Vec<usize>,
}

impl GpuTensor {
    /// Create a new tensor with the given shape
    pub fn new(
        device: &Device,
        memory_properties: &vk::PhysicalDeviceMemoryProperties,
        shape: &[usize],
    ) -> Result<Self, String> {
        let numel: usize = shape.iter().product();
        let size = (numel * std::mem::size_of::<f32>()) as u64;

        let buffer = GpuBuffer::new(
            device,
            memory_properties,
            size,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST,
        )?;

        // Calculate strides (row-major)
        let mut strides = vec![1usize; shape.len()];
        for i in (0..shape.len().saturating_sub(1)).rev() {
            strides[i] = strides[i + 1] * shape[i + 1];
        }

        Ok(Self {
            buffer,
            shape: shape.to_vec(),
            strides,
        })
    }

    /// Number of elements
    pub fn numel(&self) -> usize {
        self.shape.iter().product()
    }

    /// Upload f32 data
    pub fn upload(&self, device: &Device, data: &[f32]) -> Result<(), String> {
        self.buffer.upload(device, data)
    }

    /// Download f32 data
    pub fn download(&self, device: &Device) -> Result<Vec<f32>, String> {
        self.buffer.download(device, self.numel())
    }

    /// Destroy the tensor
    pub fn destroy(&self, device: &Device) {
        self.buffer.destroy(device);
    }
}

// =============================================================================
// GPU OPERATIONS
// =============================================================================

/// Execute GEMM (General Matrix Multiply): C = A @ B
///
/// CONTRACT_906: TENSOR_GEMM
pub fn gemm(
    ctx: &GpuContext,
    a: &GpuTensor,
    b: &GpuTensor,
    c: &mut GpuTensor,
    cmd_buffer: vk::CommandBuffer,
) -> Result<(), String> {
    // Validate shapes
    if a.shape.len() != 2 || b.shape.len() != 2 || c.shape.len() != 2 {
        return Err("GEMM requires 2D tensors".to_string());
    }

    let m = a.shape[0] as u32;
    let k = a.shape[1] as u32;
    let n = b.shape[1] as u32;

    if b.shape[0] as u32 != k || c.shape[0] as u32 != m || c.shape[1] as u32 != n {
        return Err(format!("Shape mismatch: A[{},{}] @ B[{},{}] != C[{},{}]",
            m, k, b.shape[0], b.shape[1], c.shape[0], c.shape[1]));
    }

    let push = GemmPushConstants { m, k, n, mode: 0 };

    // Bind pipeline and dispatch
    if let Some(pipeline) = ctx.pipelines.gemm_forward {
        if let Some(layout) = ctx.pipelines.pipeline_layout {
            unsafe {
                ctx.device.cmd_bind_pipeline(
                    cmd_buffer,
                    vk::PipelineBindPoint::COMPUTE,
                    pipeline,
                );

                ctx.device.cmd_push_constants(
                    cmd_buffer,
                    layout,
                    vk::ShaderStageFlags::COMPUTE,
                    0,
                    push_to_bytes(&push),
                );

                // Dispatch workgroups
                let groups_x = (n + 15) / 16;
                let groups_y = (m + 15) / 16;
                ctx.device.cmd_dispatch(cmd_buffer, groups_x, groups_y, 1);
            }
        }
    }

    Ok(())
}

/// Execute Layer Normalization
///
/// CONTRACT_907: TENSOR_LAYERNORM
pub fn layernorm(
    ctx: &GpuContext,
    input: &GpuTensor,
    gamma: &GpuTensor,
    beta: &GpuTensor,
    output: &mut GpuTensor,
    eps: f32,
    cmd_buffer: vk::CommandBuffer,
) -> Result<(), String> {
    let num_rows = (input.numel() / input.shape.last().unwrap_or(&1)) as u32;
    let row_size = *input.shape.last().unwrap_or(&1) as u32;

    let push = LayerNormPushConstants {
        num_rows,
        row_size,
        eps,
        _pad: 0,
    };

    if let Some(pipeline) = ctx.pipelines.layernorm_forward {
        if let Some(layout) = ctx.pipelines.pipeline_layout {
            unsafe {
                ctx.device.cmd_bind_pipeline(
                    cmd_buffer,
                    vk::PipelineBindPoint::COMPUTE,
                    pipeline,
                );

                ctx.device.cmd_push_constants(
                    cmd_buffer,
                    layout,
                    vk::ShaderStageFlags::COMPUTE,
                    0,
                    push_to_bytes(&push),
                );

                ctx.device.cmd_dispatch(cmd_buffer, num_rows, 1, 1);
            }
        }
    }

    Ok(())
}

/// Execute GELU activation
///
/// CONTRACT_908: TENSOR_GELU
pub fn gelu(
    ctx: &GpuContext,
    input: &GpuTensor,
    output: &mut GpuTensor,
    cmd_buffer: vk::CommandBuffer,
) -> Result<(), String> {
    let num_elements = input.numel() as u32;
    let push = GeluPushConstants { num_elements };

    if let Some(pipeline) = ctx.pipelines.gelu_forward {
        if let Some(layout) = ctx.pipelines.pipeline_layout {
            unsafe {
                ctx.device.cmd_bind_pipeline(
                    cmd_buffer,
                    vk::PipelineBindPoint::COMPUTE,
                    pipeline,
                );

                ctx.device.cmd_push_constants(
                    cmd_buffer,
                    layout,
                    vk::ShaderStageFlags::COMPUTE,
                    0,
                    push_to_bytes(&push),
                );

                let groups = (num_elements + 255) / 256;
                ctx.device.cmd_dispatch(cmd_buffer, groups, 1, 1);
            }
        }
    }

    Ok(())
}

/// Execute Softmax
///
/// CONTRACT_909: TENSOR_SOFTMAX
pub fn softmax(
    ctx: &GpuContext,
    input: &GpuTensor,
    output: &mut GpuTensor,
    dim: i32,
    cmd_buffer: vk::CommandBuffer,
) -> Result<(), String> {
    let num_rows = (input.numel() / input.shape.last().unwrap_or(&1)) as u32;
    let row_size = *input.shape.last().unwrap_or(&1) as u32;

    let push = SoftmaxPushConstants { num_rows, row_size };

    if let Some(pipeline) = ctx.pipelines.softmax_forward {
        if let Some(layout) = ctx.pipelines.pipeline_layout {
            unsafe {
                ctx.device.cmd_bind_pipeline(
                    cmd_buffer,
                    vk::PipelineBindPoint::COMPUTE,
                    pipeline,
                );

                ctx.device.cmd_push_constants(
                    cmd_buffer,
                    layout,
                    vk::ShaderStageFlags::COMPUTE,
                    0,
                    push_to_bytes(&push),
                );

                ctx.device.cmd_dispatch(cmd_buffer, num_rows, 1, 1);
            }
        }
    }

    Ok(())
}

/// Execute Cross-Entropy Loss
///
/// CONTRACT_910: TENSOR_CROSS_ENTROPY
pub fn cross_entropy(
    ctx: &GpuContext,
    logits: &GpuTensor,
    targets: &GpuBuffer,  // u32 indices
    losses: &mut GpuBuffer,
    vocab_size: u32,
    cmd_buffer: vk::CommandBuffer,
) -> Result<(), String> {
    let num_positions = (logits.numel() / vocab_size as usize) as u32;

    let push = CrossEntropyPushConstants {
        num_positions,
        vocab_size,
        ignore_index: 0,  // PAD token
    };

    if let Some(pipeline) = ctx.pipelines.cross_entropy_forward {
        if let Some(layout) = ctx.pipelines.pipeline_layout {
            unsafe {
                ctx.device.cmd_bind_pipeline(
                    cmd_buffer,
                    vk::PipelineBindPoint::COMPUTE,
                    pipeline,
                );

                ctx.device.cmd_push_constants(
                    cmd_buffer,
                    layout,
                    vk::ShaderStageFlags::COMPUTE,
                    0,
                    push_to_bytes(&push),
                );

                let groups = (num_positions + 255) / 256;
                ctx.device.cmd_dispatch(cmd_buffer, groups, 1, 1);
            }
        }
    }

    Ok(())
}

/// Execute Adam optimizer step
///
/// CONTRACT_905: ADAM_OPTIMIZER
pub fn adam_step(
    ctx: &GpuContext,
    params: &mut GpuBuffer,
    grads: &GpuBuffer,
    m: &mut GpuBuffer,
    v: &mut GpuBuffer,
    lr: f32,
    beta1: f32,
    beta2: f32,
    eps: f32,
    step: u32,
    cmd_buffer: vk::CommandBuffer,
) -> Result<(), String> {
    let num_params = (params.size / 4) as u32;  // f32 = 4 bytes
    let beta1_t = beta1.powi(step as i32);
    let beta2_t = beta2.powi(step as i32);

    let push = AdamPushConstants {
        num_params,
        lr,
        beta1,
        beta2,
        eps,
        beta1_t,
        beta2_t,
    };

    if let Some(pipeline) = ctx.pipelines.adam_step {
        if let Some(layout) = ctx.pipelines.pipeline_layout {
            unsafe {
                ctx.device.cmd_bind_pipeline(
                    cmd_buffer,
                    vk::PipelineBindPoint::COMPUTE,
                    pipeline,
                );

                ctx.device.cmd_push_constants(
                    cmd_buffer,
                    layout,
                    vk::ShaderStageFlags::COMPUTE,
                    0,
                    push_to_bytes(&push),
                );

                let groups = (num_params + 255) / 256;
                ctx.device.cmd_dispatch(cmd_buffer, groups, 1, 1);
            }
        }
    }

    Ok(())
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

fn find_memory_type(
    memory_properties: &vk::PhysicalDeviceMemoryProperties,
    type_filter: u32,
    properties: vk::MemoryPropertyFlags,
) -> Option<u32> {
    for i in 0..memory_properties.memory_type_count {
        if (type_filter & (1 << i)) != 0
            && memory_properties.memory_types[i as usize]
                .property_flags
                .contains(properties)
        {
            return Some(i);
        }
    }
    None
}

fn push_to_bytes<T>(push: &T) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(push as *const T as *const u8, std::mem::size_of::<T>())
    }
}

/// Load SPIR-V shader from file
pub fn load_spirv(path: &Path) -> Result<Vec<u32>, String> {
    let mut file = File::open(path)
        .map_err(|e| format!("Failed to open shader {}: {}", path.display(), e))?;

    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|e| format!("Failed to read shader: {}", e))?;

    if bytes.len() % 4 != 0 {
        return Err("SPIR-V file size not a multiple of 4".to_string());
    }

    let words: Vec<u32> = bytes
        .chunks(4)
        .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect();

    Ok(words)
}

/// Create a compute pipeline from SPIR-V
pub fn create_compute_pipeline(
    device: &Device,
    spirv: &[u32],
    layout: vk::PipelineLayout,
    entry_point: &str,
) -> Result<vk::Pipeline, String> {
    let shader_info = vk::ShaderModuleCreateInfo::default().code(spirv);

    let shader_module = unsafe { device.create_shader_module(&shader_info, None) }
        .map_err(|e| format!("Failed to create shader module: {:?}", e))?;

    let entry_name = CString::new(entry_point).unwrap();
    let stage_info = vk::PipelineShaderStageCreateInfo::default()
        .stage(vk::ShaderStageFlags::COMPUTE)
        .module(shader_module)
        .name(&entry_name);

    let pipeline_info = vk::ComputePipelineCreateInfo::default()
        .stage(stage_info)
        .layout(layout);

    let pipeline = unsafe {
        device.create_compute_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
    }
    .map_err(|(_, e)| format!("Failed to create compute pipeline: {:?}", e))?[0];

    // Clean up shader module (pipeline has its own reference)
    unsafe { device.destroy_shader_module(shader_module, None) };

    Ok(pipeline)
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_constants_size() {
        assert_eq!(std::mem::size_of::<GemmPushConstants>(), 16);
        assert_eq!(std::mem::size_of::<LayerNormPushConstants>(), 16);
        assert_eq!(std::mem::size_of::<AdamPushConstants>(), 28);
    }
}
