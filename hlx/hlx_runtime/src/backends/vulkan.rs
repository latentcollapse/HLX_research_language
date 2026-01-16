//! Vulkan Backend for HLX Runtime
//!
//! Executes LC-B instructions on GPU via SPIR-V compute shaders.
//! Prioritizes determinism and cross-vendor compatibility.

use ash::{vk, Entry, Instance, Device};
use gpu_allocator::vulkan::*;
use gpu_allocator::MemoryLocation;
use std::sync::{Arc, Mutex};
use std::ffi::CStr;
use std::mem::ManuallyDrop;
use bytemuck;

use crate::backend::{Backend, TensorHandle, DType, TensorMeta};
use crate::config::RuntimeConfig;
use crate::tuning::{BackendTuning, GenericTuning, detect_vendor};
use hlx_core::{Value, Result, HlxError};

pub struct VulkanBackend {
    _entry: Entry,
    instance: Instance,
    device: Device,
    queue: vk::Queue,
    #[allow(dead_code)]
    queue_family_index: u32,
    allocator: ManuallyDrop<Arc<Mutex<Allocator>>>,
    #[allow(dead_code)]
    tuning: Box<dyn BackendTuning>,

    // Command resources for memory transfer (planned for future use)
    #[allow(dead_code)]
    command_pool: vk::CommandPool,
    transfer_command_buffer: vk::CommandBuffer,
    transfer_fence: vk::Fence,
    
    // Resource tracking
    buffers: std::collections::HashMap<u64, (vk::Buffer, Allocation)>,
    metadata: std::collections::HashMap<u64, TensorMeta>,
    next_handle: u64,

    // Pipelines
    descriptor_set_layout: vk::DescriptorSetLayout,
    pipeline_layout: vk::PipelineLayout,
    pipelines: std::collections::HashMap<String, vk::Pipeline>,
}

// Include all compiled shaders
const SHADER_ADD: &[u8] = include_bytes!("vulkan/shaders/pointwise_add.spv");
const SHADER_GEMM: &[u8] = include_bytes!("vulkan/shaders/gemm.spv");
const SHADER_ACTIVATION: &[u8] = include_bytes!("vulkan/shaders/activation.spv");
const SHADER_SOFTMAX: &[u8] = include_bytes!("vulkan/shaders/softmax.spv");
const SHADER_LAYERNORM: &[u8] = include_bytes!("vulkan/shaders/layernorm.spv");
const SHADER_CROSS_ENTROPY: &[u8] = include_bytes!("vulkan/shaders/cross_entropy.spv");
const SHADER_ELEMENTWISE: &[u8] = include_bytes!("vulkan/shaders/elementwise.spv");
const SHADER_REDUCTION: &[u8] = include_bytes!("vulkan/shaders/reduction.spv");
const SHADER_CONV2D: &[u8] = include_bytes!("vulkan/shaders/conv2d.spv");
const SHADER_POOLING: &[u8] = include_bytes!("vulkan/shaders/pooling.spv");
const SHADER_BATCHNORM: &[u8] = include_bytes!("vulkan/shaders/batchnorm.spv");
const SHADER_DROPOUT: &[u8] = include_bytes!("vulkan/shaders/dropout.spv");
const SHADER_TRANSPOSE: &[u8] = include_bytes!("vulkan/shaders/transpose.spv");
const SHADER_GAUSSIAN_BLUR: &[u8] = include_bytes!("vulkan/shaders/gaussian_blur.spv");
const SHADER_SOBEL: &[u8] = include_bytes!("vulkan/shaders/sobel.spv");
const SHADER_GRAYSCALE: &[u8] = include_bytes!("vulkan/shaders/grayscale.spv");
const SHADER_THRESHOLD: &[u8] = include_bytes!("vulkan/shaders/threshold.spv");
const SHADER_BRIGHTNESS: &[u8] = include_bytes!("vulkan/shaders/brightness.spv");
const SHADER_CONTRAST: &[u8] = include_bytes!("vulkan/shaders/contrast.spv");
const SHADER_INVERT_COLORS: &[u8] = include_bytes!("vulkan/shaders/invert_colors.spv");
const SHADER_SHARPEN: &[u8] = include_bytes!("vulkan/shaders/sharpen.spv");
const SHADER_BASIC_VERT: &[u8] = include_bytes!("vulkan/shaders/basic_vert.spv");
const SHADER_BASIC_FRAG: &[u8] = include_bytes!("vulkan/shaders/basic_frag.spv");
const SHADER_PBR_FRAG: &[u8] = include_bytes!("vulkan/shaders/pbr_frag.spv");

impl VulkanBackend {
    pub fn new(_config: &RuntimeConfig) -> Result<Self> {
        unsafe {
            // ... (Previous initialization code remains, handled by context) ...
            // We need to re-create the init block to add the command pool setup.
            // Since replace requires exact match, I will assume the previous 'new' block ends before the struct init.
            
            // 1. Entry
            let entry = Entry::load().map_err(|e| HlxError::BackendError {
                message: format!("Failed to load Vulkan entry: {}", e),
            })?;

            // 2. Instance
            let app_name = CStr::from_bytes_with_nul(b"HLX Runtime\0").unwrap();
            let app_info = vk::ApplicationInfo::builder()
                .application_name(app_name)
                .application_version(0)
                .engine_name(app_name)
                .engine_version(0)
                .api_version(vk::API_VERSION_1_2); 

            let create_info = vk::InstanceCreateInfo::builder()
                .application_info(&app_info);

            let instance = entry.create_instance(&create_info, None)
                .map_err(|e| HlxError::BackendError { message: format!("Instance creation failed: {}", e) })?;

            // 3. Physical Device Selection
            let pdevices = instance.enumerate_physical_devices()
                .map_err(|e| HlxError::BackendError { message: format!("No physical devices: {}", e) })?;

            let (pdevice, queue_family_index) = pdevices.iter().find_map(|pdevice| {
                instance.get_physical_device_queue_family_properties(*pdevice)
                    .iter()
                    .enumerate()
                    .find_map(|(index, info)| {
                        if info.queue_flags.contains(vk::QueueFlags::COMPUTE) {
                            Some((*pdevice, index as u32))
                        } else {
                            None
                        }
                    })
            }).ok_or(HlxError::BackendError { message: "No compute-capable GPU got".to_string() })?;

            // Detect vendor for tuning
            let props = instance.get_physical_device_properties(pdevice);
            let tuning = match detect_vendor(props.vendor_id) {
                crate::tuning::Vendor::Nvidia => Box::new(GenericTuning), 
                crate::tuning::Vendor::Amd => Box::new(GenericTuning),    
                _ => Box::new(GenericTuning),
            };

            // 4. Device
            let queue_priorities = [1.0];
            let queue_info = vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(queue_family_index)
                .queue_priorities(&queue_priorities);

            let device_create_info = vk::DeviceCreateInfo::builder()
                .queue_create_infos(std::slice::from_ref(&queue_info));

            let device = instance.create_device(pdevice, &device_create_info, None)
                .map_err(|e| HlxError::BackendError { message: format!("Device creation failed: {}", e) })?;

            let queue = device.get_device_queue(queue_family_index, 0);

            // 5. Allocator
            let allocator = Allocator::new(&AllocatorCreateDesc {
                instance: instance.clone(),
                device: device.clone(),
                physical_device: pdevice,
                debug_settings: Default::default(),
                buffer_device_address: false,
                allocation_sizes: Default::default(),
            }).map_err(|e| HlxError::BackendError { message: format!("Allocator init failed: {}", e) })?;

            // 6. Command Pool & Transfer Resources
            let pool_create_info = vk::CommandPoolCreateInfo::builder()
                .queue_family_index(queue_family_index)
                .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
            
            let command_pool = device.create_command_pool(&pool_create_info, None)
                .map_err(|e| HlxError::BackendError { message: format!("Command pool creation failed: {}", e) })?;

            let cmd_buf_allocate_info = vk::CommandBufferAllocateInfo::builder()
                .command_pool(command_pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(1);

            let transfer_command_buffer = device.allocate_command_buffers(&cmd_buf_allocate_info)
                .map_err(|e| HlxError::BackendError { message: format!("Command buffer alloc failed: {}", e) })?[0];

            let fence_create_info = vk::FenceCreateInfo::builder();
            let transfer_fence = device.create_fence(&fence_create_info, None)
                .map_err(|e| HlxError::BackendError { message: format!("Fence creation failed: {}", e) })?;

            // 7. Descriptor Set Layout (3 storage buffers)
            let bindings = [
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(0)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::COMPUTE)
                    .build(),
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(1)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::COMPUTE)
                    .build(),
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(2)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::COMPUTE)
                    .build(),
            ];

            let layout_info = vk::DescriptorSetLayoutCreateInfo::builder()
                .bindings(&bindings);
            
            let descriptor_set_layout = device.create_descriptor_set_layout(&layout_info, None)
                .map_err(|e| HlxError::BackendError { message: format!("DS layout failed: {}", e) })?;

            // 8. Pipeline Layout (with Push Constants)
            let push_constant_range = vk::PushConstantRange::builder()
                .stage_flags(vk::ShaderStageFlags::COMPUTE)
                .offset(0)
                .size(128); // Support up to 128 bytes for complex shaders

            let pipeline_layout_info = vk::PipelineLayoutCreateInfo::builder()
                .set_layouts(std::slice::from_ref(&descriptor_set_layout))
                .push_constant_ranges(std::slice::from_ref(&push_constant_range));
            
            let pipeline_layout = device.create_pipeline_layout(&pipeline_layout_info, None)
                .map_err(|e| HlxError::BackendError { message: format!("Pipeline layout failed: {}", e) })?;

            Ok(Self {
                _entry: entry,
                instance,
                device,
                queue,
                queue_family_index,
                allocator: ManuallyDrop::new(Arc::new(Mutex::new(allocator))),
                tuning,
                command_pool,
                transfer_command_buffer,
                transfer_fence,
                buffers: std::collections::HashMap::new(),
                metadata: std::collections::HashMap::new(),
                next_handle: 1,
                descriptor_set_layout,
                pipeline_layout,
                pipelines: std::collections::HashMap::new(),
            })
        }
    }

    /// Helper to find a suitable memory type index (planned for future use)
    #[allow(dead_code)]
    fn find_memory_type(&self, _type_filter: u32, _properties: vk::MemoryPropertyFlags) -> Option<u32> {
        // This is handled by gpu-allocator usually, but useful for raw buffer creation checks
        None
    }

    fn get_or_create_pipeline(&mut self, name: &str, spv: &[u8]) -> Result<vk::Pipeline> {
        if let Some(&p) = self.pipelines.get(name) {
            return Ok(p);
        }

        unsafe {
            // Convert SPIR-V bytes to u32 words safely (no alignment requirements)
            let code_len = spv.len() / 4;
            let mut code = Vec::with_capacity(code_len);
            for i in 0..code_len {
                let bytes = [
                    spv[i * 4],
                    spv[i * 4 + 1],
                    spv[i * 4 + 2],
                    spv[i * 4 + 3],
                ];
                code.push(u32::from_le_bytes(bytes));
            }

            let shader_module_info = vk::ShaderModuleCreateInfo::builder()
                .code(&code);

            let shader_module = self.device.create_shader_module(&shader_module_info, None)
                .map_err(|e| HlxError::BackendError { message: format!("Shader module failed: {}", e) })?;

            let entry_name = CStr::from_bytes_with_nul(b"main\0").unwrap();
            let stage_info = vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::COMPUTE)
                .module(shader_module)
                .name(entry_name);

            let pipeline_info = vk::ComputePipelineCreateInfo::builder()
                .stage(stage_info.build())
                .layout(self.pipeline_layout);

            let pipeline = self.device.create_compute_pipelines(vk::PipelineCache::null(), &[pipeline_info.build()], None)
                .map_err(|e| HlxError::BackendError { message: format!("Pipeline creation failed: {:?}", e) })?[0];

            self.device.destroy_shader_module(shader_module, None);
            self.pipelines.insert(name.to_string(), pipeline);
            Ok(pipeline)
        }
    }

    /// Helper for activation functions (ReLU, GELU, Sigmoid, Tanh)
    /// mode: 0=ReLU, 1=GELU, 2=Sigmoid, 3=Tanh
    fn activation(&mut self, input: TensorHandle, out: TensorHandle, mode: u32) -> Result<()> {
        let meta = self.tensor_meta(input)?;
        let n = meta.shape.iter().product::<usize>() as u32;

        #[repr(C)]
        #[derive(Copy, Clone)]
        struct ActivationPushConstants {
            n: u32,
            mode: u32,
        }
        unsafe impl bytemuck::Pod for ActivationPushConstants {}
        unsafe impl bytemuck::Zeroable for ActivationPushConstants {}

        unsafe {
            let pipeline = self.get_or_create_pipeline("activation", SHADER_ACTIVATION)?;

            // Descriptor Pool & Set
            let pool_sizes = [vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: 2,
            }];
            let pool_info = vk::DescriptorPoolCreateInfo::builder()
                .max_sets(1)
                .pool_sizes(&pool_sizes);
            let descriptor_pool = self.device.create_descriptor_pool(&pool_info, None)
                .map_err(|e| HlxError::BackendError { message: format!("Pool failed: {}", e) })?;

            let set_info = vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(descriptor_pool)
                .set_layouts(std::slice::from_ref(&self.descriptor_set_layout));
            let descriptor_set = self.device.allocate_descriptor_sets(&set_info)
                .map_err(|e| HlxError::BackendError { message: format!("Set alloc failed: {}", e) })?[0];

            // Update Descriptor Set
            let buf_in = self.buffers.get(&input.0).unwrap().0;
            let buf_out = self.buffers.get(&out.0).unwrap().0;

            let info_in = vk::DescriptorBufferInfo { buffer: buf_in, offset: 0, range: vk::WHOLE_SIZE };
            let info_out = vk::DescriptorBufferInfo { buffer: buf_out, offset: 0, range: vk::WHOLE_SIZE };

            let writes = [
                vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_set)
                    .dst_binding(0)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(std::slice::from_ref(&info_in))
                    .build(),
                vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_set)
                    .dst_binding(1)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(std::slice::from_ref(&info_out))
                    .build(),
            ];

            self.device.update_descriptor_sets(&writes, &[]);

            // Push Constants
            let push_constants = ActivationPushConstants { n, mode };
            let push_bytes = bytemuck::bytes_of(&push_constants);

            // Record & Submit
            let begin_info = vk::CommandBufferBeginInfo::builder()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            self.device.begin_command_buffer(self.transfer_command_buffer, &begin_info)
                .map_err(|e| HlxError::BackendError { message: format!("Begin failed: {}", e) })?;

            self.device.cmd_bind_pipeline(self.transfer_command_buffer, vk::PipelineBindPoint::COMPUTE, pipeline);
            self.device.cmd_bind_descriptor_sets(self.transfer_command_buffer, vk::PipelineBindPoint::COMPUTE, self.pipeline_layout, 0, &[descriptor_set], &[]);
            self.device.cmd_push_constants(self.transfer_command_buffer, self.pipeline_layout, vk::ShaderStageFlags::COMPUTE, 0, push_bytes);

            let group_count = (n + 255) / 256;
            self.device.cmd_dispatch(self.transfer_command_buffer, group_count, 1, 1);

            self.device.end_command_buffer(self.transfer_command_buffer)
                .map_err(|e| HlxError::BackendError { message: format!("End failed: {}", e) })?;

            let submit_info = vk::SubmitInfo::builder()
                .command_buffers(std::slice::from_ref(&self.transfer_command_buffer));
            self.device.queue_submit(self.queue, &[submit_info.build()], self.transfer_fence)
                .map_err(|e| HlxError::BackendError { message: format!("Submit failed: {}", e) })?;

            self.device.wait_for_fences(&[self.transfer_fence], true, u64::MAX)
                .map_err(|e| HlxError::BackendError { message: format!("Wait failed: {}", e) })?;
            self.device.reset_fences(&[self.transfer_fence])
                .map_err(|e| HlxError::BackendError { message: format!("Reset failed: {}", e) })?;

            // Cleanup
            self.device.destroy_descriptor_pool(descriptor_pool, None);
        }

        Ok(())
    }
}

// Placeholder implementation of Backend trait
impl Backend for VulkanBackend {
    fn name(&self) -> &'static str { "Vulkan" }
    
    fn is_available(&self) -> bool { true }

    fn alloc_tensor(&mut self, shape: &[usize], dtype: DType) -> Result<TensorHandle> {
        let size_bytes = (shape.iter().product::<usize>() * dtype.size_bytes()) as u64;
        println!("[Vulkan] Allocating Tensor: {:?} ({:?}) -> {} bytes", shape, dtype, size_bytes);
        
        unsafe {
            // Create VkBuffer
            let buffer_info = vk::BufferCreateInfo::builder()
                .size(size_bytes)
                .usage(vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST)
                .sharing_mode(vk::SharingMode::EXCLUSIVE);

            let buffer = self.device.create_buffer(&buffer_info, None)
                .map_err(|e| HlxError::BackendError { message: format!("Buffer creation failed: {}", e) })?;

            let requirements = self.device.get_buffer_memory_requirements(buffer);

            // Allocate Memory
            let mut allocator = self.allocator.lock().unwrap();
            let allocation = allocator.allocate(&AllocationCreateDesc {
                name: "HLX Tensor",
                requirements,
                location: MemoryLocation::GpuOnly,
                linear: true, 
                allocation_scheme: AllocationScheme::GpuAllocatorManaged,
            }).map_err(|e| HlxError::BackendError { message: format!("Allocation failed: {}", e) })?;

            // Bind
            self.device.bind_buffer_memory(buffer, allocation.memory(), allocation.offset())
                .map_err(|e| HlxError::BackendError { message: format!("Bind failed: {}", e) })?;

            let handle = TensorHandle(self.next_handle);
            self.next_handle += 1;
            self.buffers.insert(handle.0, (buffer, allocation));
            self.metadata.insert(handle.0, TensorMeta {
                shape: shape.to_vec(),
                dtype,
                size_bytes: size_bytes as usize,
            });

            println!("[Vulkan] Allocation successful. Handle: {}", handle.0);
            Ok(handle)
        }
    }

    fn free_tensor(&mut self, handle: TensorHandle) -> Result<()> {
        self.metadata.remove(&handle.0);
        if let Some((buffer, allocation)) = self.buffers.remove(&handle.0) {
            unsafe {
                self.device.destroy_buffer(buffer, None);
                let mut allocator = self.allocator.lock().unwrap();
                allocator.free(allocation)
                    .map_err(|e| HlxError::BackendError { message: format!("Free failed: {}", e) })?;
            }
        }
        Ok(())
    }

    fn tensor_meta(&self, handle: TensorHandle) -> Result<TensorMeta> {
        self.metadata.get(&handle.0)
            .cloned()
            .ok_or(HlxError::ValidationFail { message: "Invalid handle".to_string() })
    }

    fn write_tensor(&mut self, handle: TensorHandle, data: &[u8]) -> Result<()> {
        let (dst_buffer, _) = self.buffers.get(&handle.0)
            .ok_or(HlxError::ValidationFail { message: "Invalid handle".to_string() })?;

        let size = data.len() as u64;

        unsafe {
            // 1. Create Staging Buffer
            let staging_info = vk::BufferCreateInfo::builder()
                .size(size)
                .usage(vk::BufferUsageFlags::TRANSFER_SRC)
                .sharing_mode(vk::SharingMode::EXCLUSIVE);
            
            let staging_buffer = self.device.create_buffer(&staging_info, None)
                .map_err(|e| HlxError::BackendError { message: format!("Staging buffer failed: {}", e) })?;

            let reqs = self.device.get_buffer_memory_requirements(staging_buffer);
            
            let mut allocator = self.allocator.lock().unwrap();
            let allocation = allocator.allocate(&AllocationCreateDesc {
                name: "Staging Buffer",
                requirements: reqs,
                location: MemoryLocation::CpuToGpu,
                linear: true,
                allocation_scheme: AllocationScheme::GpuAllocatorManaged,
            }).map_err(|e| HlxError::BackendError { message: format!("Staging alloc failed: {}", e) })?;

            // 2. Map and Copy
            let ptr = allocation.mapped_ptr().ok_or(HlxError::BackendError { message: "Failed to map staging buffer".to_string() })?;
            std::ptr::copy_nonoverlapping(data.as_ptr(), ptr.as_ptr() as *mut u8, data.len());

            self.device.bind_buffer_memory(staging_buffer, allocation.memory(), allocation.offset())
                .map_err(|e| HlxError::BackendError { message: format!("Staging bind failed: {}", e) })?;

            // 3. Record Copy Command
            let begin_info = vk::CommandBufferBeginInfo::builder()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            
            self.device.begin_command_buffer(self.transfer_command_buffer, &begin_info)
                .map_err(|e| HlxError::BackendError { message: format!("Begin command failed: {}", e) })?;

            let copy_region = vk::BufferCopy {
                src_offset: 0,
                dst_offset: 0,
                size,
            };
            self.device.cmd_copy_buffer(self.transfer_command_buffer, staging_buffer, *dst_buffer, &[copy_region]);

            self.device.end_command_buffer(self.transfer_command_buffer)
                .map_err(|e| HlxError::BackendError { message: format!("End command failed: {}", e) })?;

            // 4. Submit and Wait
            let submit_info = vk::SubmitInfo::builder()
                .command_buffers(std::slice::from_ref(&self.transfer_command_buffer));

            self.device.queue_submit(self.queue, &[submit_info.build()], self.transfer_fence)
                .map_err(|e| HlxError::BackendError { message: format!("Queue submit failed: {}", e) })?;

            self.device.wait_for_fences(&[self.transfer_fence], true, u64::MAX)
                .map_err(|e| HlxError::BackendError { message: format!("Wait fence failed: {}", e) })?;
            self.device.reset_fences(&[self.transfer_fence])
                .map_err(|e| HlxError::BackendError { message: format!("Reset fence failed: {}", e) })?;

            // 5. Cleanup Staging
            self.device.destroy_buffer(staging_buffer, None);
            allocator.free(allocation)
                .map_err(|e| HlxError::BackendError { message: format!("Free staging failed: {}", e) })?;
        }
        Ok(())
    }

    fn read_tensor(&self, handle: TensorHandle) -> Result<Vec<u8>> {
        let (src_buffer, _) = self.buffers.get(&handle.0)
            .ok_or(HlxError::ValidationFail { message: "Invalid handle".to_string() })?;
        
        let meta = self.metadata.get(&handle.0)
            .ok_or(HlxError::ValidationFail { message: "Invalid handle".to_string() })?;
        
        let size = (meta.shape.iter().product::<usize>() * meta.dtype.size_bytes()) as u64;
        let mut data = vec![0u8; size as usize];

        unsafe {
            // 1. Create Staging Buffer
            let staging_info = vk::BufferCreateInfo::builder()
                .size(size)
                .usage(vk::BufferUsageFlags::TRANSFER_DST)
                .sharing_mode(vk::SharingMode::EXCLUSIVE);
            
            let staging_buffer = self.device.create_buffer(&staging_info, None)
                .map_err(|e| HlxError::BackendError { message: format!("Staging buffer failed: {}", e) })?;

            let reqs = self.device.get_buffer_memory_requirements(staging_buffer);
            
            let mut allocator = self.allocator.lock().unwrap();
            let allocation = allocator.allocate(&AllocationCreateDesc {
                name: "Staging Read Buffer",
                requirements: reqs,
                location: MemoryLocation::GpuToCpu,
                linear: true,
                allocation_scheme: AllocationScheme::GpuAllocatorManaged,
            }).map_err(|e| HlxError::BackendError { message: format!("Staging alloc failed: {}", e) })?;

            self.device.bind_buffer_memory(staging_buffer, allocation.memory(), allocation.offset())
                .map_err(|e| HlxError::BackendError { message: format!("Staging bind failed: {}", e) })?;

            // 2. Record Copy Command
            let begin_info = vk::CommandBufferBeginInfo::builder()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            
            self.device.begin_command_buffer(self.transfer_command_buffer, &begin_info)
                .map_err(|e| HlxError::BackendError { message: format!("Begin command failed: {}", e) })?;

            let copy_region = vk::BufferCopy {
                src_offset: 0,
                dst_offset: 0,
                size,
            };
            self.device.cmd_copy_buffer(self.transfer_command_buffer, *src_buffer, staging_buffer, &[copy_region]);

            self.device.end_command_buffer(self.transfer_command_buffer)
                .map_err(|e| HlxError::BackendError { message: format!("End command failed: {}", e) })?;

            // 3. Submit and Wait
            let submit_info = vk::SubmitInfo::builder()
                .command_buffers(std::slice::from_ref(&self.transfer_command_buffer));

            self.device.queue_submit(self.queue, &[submit_info.build()], self.transfer_fence)
                .map_err(|e| HlxError::BackendError { message: format!("Queue submit failed: {}", e) })?;

            self.device.wait_for_fences(&[self.transfer_fence], true, u64::MAX)
                .map_err(|e| HlxError::BackendError { message: format!("Wait fence failed: {}", e) })?;
            self.device.reset_fences(&[self.transfer_fence])
                .map_err(|e| HlxError::BackendError { message: format!("Reset fence failed: {}", e) })?;

            // 4. Map and Read
            let ptr = allocation.mapped_ptr().ok_or(HlxError::BackendError { message: "Failed to map staging buffer".to_string() })?;
            std::ptr::copy_nonoverlapping(ptr.as_ptr() as *const u8, data.as_mut_ptr(), data.len());

            // 5. Cleanup
            self.device.destroy_buffer(staging_buffer, None);
            allocator.free(allocation)
                .map_err(|e| HlxError::BackendError { message: format!("Free staging failed: {}", e) })?;
        }
        Ok(data)
    }
    
    // Scalars
    fn scalar_add(&mut self, a: &Value, b: &Value) -> Result<Value> { a.add(b) }
    fn scalar_sub(&mut self, a: &Value, b: &Value) -> Result<Value> { a.sub(b) }
    fn scalar_mul(&mut self, a: &Value, b: &Value) -> Result<Value> { a.mul(b) }
    fn scalar_div(&mut self, a: &Value, b: &Value) -> Result<Value> { a.div(b) }
    fn scalar_mod(&mut self, a: &Value, b: &Value) -> Result<Value> { a.rem(b) }

    // Math functions
    fn scalar_sqrt(&mut self, a: &Value) -> Result<Value> {
        match a {
            Value::Float(f) => Ok(Value::Float((*f as f64).sqrt())),
            Value::Integer(i) => Ok(Value::Float((*i as f64).sqrt())),
            _ => Err(hlx_core::HlxError::TypeError { expected: "number".to_string(), got: a.type_name().to_string() }),
        }
    }
    fn scalar_pow(&mut self, base: &Value, exp: &Value) -> Result<Value> {
        match (base, exp) {
            (Value::Float(b), Value::Float(e)) => Ok(Value::Float((*b as f64).powf(*e as f64))),
            (Value::Float(b), Value::Integer(e)) => Ok(Value::Float((*b as f64).powi(*e as i32))),
            (Value::Integer(b), Value::Integer(e)) => {
                if *e >= 0 { Ok(Value::Integer((*b as i64).pow(*e as u32))) }
                else { Ok(Value::Float((*b as f64).powf(*e as f64))) }
            }
            (Value::Integer(b), Value::Float(e)) => Ok(Value::Float((*b as f64).powf(*e as f64))),
            _ => Err(hlx_core::HlxError::TypeError { expected: "number".to_string(), got: base.type_name().to_string() }),
        }
    }
    fn scalar_sin(&mut self, a: &Value) -> Result<Value> {
        match a {
            Value::Float(f) => Ok(Value::Float((*f as f64).sin())),
            Value::Integer(i) => Ok(Value::Float((*i as f64).sin())),
            _ => Err(hlx_core::HlxError::TypeError { expected: "number".to_string(), got: a.type_name().to_string() }),
        }
    }
    fn scalar_cos(&mut self, a: &Value) -> Result<Value> {
        match a {
            Value::Float(f) => Ok(Value::Float((*f as f64).cos())),
            Value::Integer(i) => Ok(Value::Float((*i as f64).cos())),
            _ => Err(hlx_core::HlxError::TypeError { expected: "number".to_string(), got: a.type_name().to_string() }),
        }
    }
    fn scalar_tan(&mut self, a: &Value) -> Result<Value> {
        match a {
            Value::Float(f) => Ok(Value::Float((*f as f64).tan())),
            Value::Integer(i) => Ok(Value::Float((*i as f64).tan())),
            _ => Err(hlx_core::HlxError::TypeError { expected: "number".to_string(), got: a.type_name().to_string() }),
        }
    }
    fn scalar_log(&mut self, a: &Value) -> Result<Value> {
        match a {
            Value::Float(f) => Ok(Value::Float((*f as f64).ln())),
            Value::Integer(i) => Ok(Value::Float((*i as f64).ln())),
            _ => Err(hlx_core::HlxError::TypeError { expected: "number".to_string(), got: a.type_name().to_string() }),
        }
    }
    fn scalar_exp(&mut self, a: &Value) -> Result<Value> {
        match a {
            Value::Float(f) => Ok(Value::Float((*f as f64).exp())),
            Value::Integer(i) => Ok(Value::Float((*i as f64).exp())),
            _ => Err(hlx_core::HlxError::TypeError { expected: "number".to_string(), got: a.type_name().to_string() }),
        }
    }
    fn scalar_floor(&mut self, a: &Value) -> Result<Value> {
        match a {
            Value::Float(f) => Ok(Value::Integer((*f as f64).floor() as i64)),
            Value::Integer(i) => Ok(Value::Integer(*i)),
            _ => Err(hlx_core::HlxError::TypeError { expected: "number".to_string(), got: a.type_name().to_string() }),
        }
    }
    fn scalar_ceil(&mut self, a: &Value) -> Result<Value> {
        match a {
            Value::Float(f) => Ok(Value::Integer((*f as f64).ceil() as i64)),
            Value::Integer(i) => Ok(Value::Integer(*i)),
            _ => Err(hlx_core::HlxError::TypeError { expected: "number".to_string(), got: a.type_name().to_string() }),
        }
    }
    fn scalar_round(&mut self, a: &Value) -> Result<Value> {
        match a {
            Value::Float(f) => Ok(Value::Integer((*f as f64).round() as i64)),
            Value::Integer(i) => Ok(Value::Integer(*i)),
            _ => Err(hlx_core::HlxError::TypeError { expected: "number".to_string(), got: a.type_name().to_string() }),
        }
    }
    fn scalar_abs(&mut self, a: &Value) -> Result<Value> {
        match a {
            Value::Float(f) => Ok(Value::Float((*f as f64).abs())),
            Value::Integer(i) => Ok(Value::Integer((*i as i64).abs())),
            _ => Err(hlx_core::HlxError::TypeError { expected: "number".to_string(), got: a.type_name().to_string() }),
        }
    }

    fn scalar_eq(&mut self, a: &Value, b: &Value) -> Result<Value> { Ok(Value::Boolean(a == b)) }
    fn scalar_ne(&mut self, a: &Value, b: &Value) -> Result<Value> { Ok(Value::Boolean(a != b)) }
    fn scalar_lt(&mut self, a: &Value, b: &Value) -> Result<Value> { Ok(Value::Boolean(a.lt(b)?)) }
    fn scalar_le(&mut self, a: &Value, b: &Value) -> Result<Value> { Ok(Value::Boolean(a.le(b)?)) }
    fn scalar_gt(&mut self, a: &Value, b: &Value) -> Result<Value> { Ok(Value::Boolean(!a.le(b)?)) }
    fn scalar_ge(&mut self, a: &Value, b: &Value) -> Result<Value> { Ok(Value::Boolean(!a.lt(b)?)) }

    fn pointwise_add(&mut self, a: TensorHandle, b: TensorHandle, out: TensorHandle) -> Result<()> {
        let pipeline = self.get_or_create_pipeline("add", SHADER_ADD)?;
        
        let meta_a = self.tensor_meta(a)?;
        let n = meta_a.shape.iter().product::<usize>() as u32;

        unsafe {
            // 1. Create Descriptor Pool & Set
            let pool_sizes = [vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: 3,
            }];
            let pool_info = vk::DescriptorPoolCreateInfo::builder()
                .max_sets(1)
                .pool_sizes(&pool_sizes);
            let descriptor_pool = self.device.create_descriptor_pool(&pool_info, None)
                .map_err(|e| HlxError::BackendError { message: format!("Pool failed: {}", e) })?;

            let set_info = vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(descriptor_pool)
                .set_layouts(std::slice::from_ref(&self.descriptor_set_layout));
            let descriptor_set = self.device.allocate_descriptor_sets(&set_info)
                .map_err(|e| HlxError::BackendError { message: format!("Set alloc failed: {}", e) })?[0];

            // 2. Update Descriptor Set
            let buf_a = self.buffers.get(&a.0).unwrap().0;
            let buf_b = self.buffers.get(&b.0).unwrap().0;
            let buf_out = self.buffers.get(&out.0).unwrap().0;

            let info_a = vk::DescriptorBufferInfo { buffer: buf_a, offset: 0, range: vk::WHOLE_SIZE };
            let info_b = vk::DescriptorBufferInfo { buffer: buf_b, offset: 0, range: vk::WHOLE_SIZE };
            let info_out = vk::DescriptorBufferInfo { buffer: buf_out, offset: 0, range: vk::WHOLE_SIZE };

            let writes = [
                vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_set)
                    .dst_binding(0)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(std::slice::from_ref(&info_a))
                    .build(),
                vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_set)
                    .dst_binding(1)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(std::slice::from_ref(&info_b))
                    .build(),
                vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_set)
                    .dst_binding(2)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(std::slice::from_ref(&info_out))
                    .build(),
            ];

            self.device.update_descriptor_sets(&writes, &[]);

            // 3. Record & Submit
            let begin_info = vk::CommandBufferBeginInfo::builder()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            self.device.begin_command_buffer(self.transfer_command_buffer, &begin_info)
                .map_err(|e| HlxError::BackendError { message: format!("Begin failed: {}", e) })?;

            self.device.cmd_bind_pipeline(self.transfer_command_buffer, vk::PipelineBindPoint::COMPUTE, pipeline);
            self.device.cmd_bind_descriptor_sets(self.transfer_command_buffer, vk::PipelineBindPoint::COMPUTE, self.pipeline_layout, 0, &[descriptor_set], &[]);
            self.device.cmd_push_constants(self.transfer_command_buffer, self.pipeline_layout, vk::ShaderStageFlags::COMPUTE, 0, &n.to_ne_bytes());
            
            let group_count = (n + 255) / 256;
            self.device.cmd_dispatch(self.transfer_command_buffer, group_count, 1, 1);

            self.device.end_command_buffer(self.transfer_command_buffer)
                .map_err(|e| HlxError::BackendError { message: format!("End failed: {}", e) })?;

            let submit_info = vk::SubmitInfo::builder()
                .command_buffers(std::slice::from_ref(&self.transfer_command_buffer));
            self.device.queue_submit(self.queue, &[submit_info.build()], self.transfer_fence)
                .map_err(|e| HlxError::BackendError { message: format!("Submit failed: {}", e) })?;

            self.device.wait_for_fences(&[self.transfer_fence], true, u64::MAX)
                .map_err(|e| HlxError::BackendError { message: format!("Wait failed: {}", e) })?;
            self.device.reset_fences(&[self.transfer_fence])
                .map_err(|e| HlxError::BackendError { message: format!("Reset failed: {}", e) })?;

            // 4. Cleanup
            self.device.destroy_descriptor_pool(descriptor_pool, None);
        }

        Ok(())
    }

    fn matmul(&mut self, a: TensorHandle, b: TensorHandle, out: TensorHandle) -> Result<()> {
        // GEMM: C = A @ B (alpha=1.0, beta=0.0)
        let meta_a = self.metadata.get(&a.0).ok_or(HlxError::BackendError {
            message: "Tensor A not got".to_string()
        })?;
        let meta_b = self.metadata.get(&b.0).ok_or(HlxError::BackendError {
            message: "Tensor B not got".to_string()
        })?;

        // Get dimensions: A is (M x K), B is (K x N), C is (M x N)
        let m = meta_a.shape[0] as u32;
        let k = meta_a.shape[1] as u32;
        let n = meta_b.shape[1] as u32;

        unsafe {
            // 1. Get/Create Pipeline
            let pipeline = self.get_or_create_pipeline("gemm", SHADER_GEMM)?;

            // 2. Descriptor Pool & Set
            let pool_sizes = [vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: 3,
            }];
            let pool_info = vk::DescriptorPoolCreateInfo::builder()
                .pool_sizes(&pool_sizes)
                .max_sets(1);
            let descriptor_pool = self.device.create_descriptor_pool(&pool_info, None)
                .map_err(|e| HlxError::BackendError { message: format!("Pool failed: {}", e) })?;

            let alloc_info = vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(descriptor_pool)
                .set_layouts(std::slice::from_ref(&self.descriptor_set_layout));
            let descriptor_set = self.device.allocate_descriptor_sets(&alloc_info)
                .map_err(|e| HlxError::BackendError { message: format!("Set alloc failed: {}", e) })?[0];

            // 3. Update Descriptor Sets
            let buf_a = self.buffers.get(&a.0).unwrap().0;
            let buf_b = self.buffers.get(&b.0).unwrap().0;
            let buf_out = self.buffers.get(&out.0).unwrap().0;

            let info_a = vk::DescriptorBufferInfo { buffer: buf_a, offset: 0, range: vk::WHOLE_SIZE };
            let info_b = vk::DescriptorBufferInfo { buffer: buf_b, offset: 0, range: vk::WHOLE_SIZE };
            let info_out = vk::DescriptorBufferInfo { buffer: buf_out, offset: 0, range: vk::WHOLE_SIZE };

            let writes = [
                vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_set)
                    .dst_binding(0)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(std::slice::from_ref(&info_a))
                    .build(),
                vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_set)
                    .dst_binding(1)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(std::slice::from_ref(&info_b))
                    .build(),
                vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_set)
                    .dst_binding(2)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(std::slice::from_ref(&info_out))
                    .build(),
            ];

            self.device.update_descriptor_sets(&writes, &[]);

            // 4. Push Constants: M, N, K, alpha, beta
            #[repr(C)]
            #[derive(Copy, Clone)]
            struct GemmPushConstants {
                m: u32,
                n: u32,
                k: u32,
                alpha: f32,
                beta: f32,
            }
            unsafe impl bytemuck::Pod for GemmPushConstants {}
            unsafe impl bytemuck::Zeroable for GemmPushConstants {}
            let push_constants = GemmPushConstants {
                m, n, k,
                alpha: 1.0,
                beta: 0.0,
            };
            let push_bytes = bytemuck::bytes_of(&push_constants);

            // 5. Record & Submit
            let begin_info = vk::CommandBufferBeginInfo::builder()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            self.device.begin_command_buffer(self.transfer_command_buffer, &begin_info)
                .map_err(|e| HlxError::BackendError { message: format!("Begin failed: {}", e) })?;

            self.device.cmd_bind_pipeline(self.transfer_command_buffer, vk::PipelineBindPoint::COMPUTE, pipeline);
            self.device.cmd_bind_descriptor_sets(self.transfer_command_buffer, vk::PipelineBindPoint::COMPUTE, self.pipeline_layout, 0, &[descriptor_set], &[]);
            self.device.cmd_push_constants(self.transfer_command_buffer, self.pipeline_layout, vk::ShaderStageFlags::COMPUTE, 0, push_bytes);

            // Dispatch with 16x16 workgroups (matching shader's local_size)
            let group_x = (n + 15) / 16;
            let group_y = (m + 15) / 16;
            self.device.cmd_dispatch(self.transfer_command_buffer, group_x, group_y, 1);

            self.device.end_command_buffer(self.transfer_command_buffer)
                .map_err(|e| HlxError::BackendError { message: format!("End failed: {}", e) })?;

            let submit_info = vk::SubmitInfo::builder()
                .command_buffers(std::slice::from_ref(&self.transfer_command_buffer));
            self.device.queue_submit(self.queue, &[submit_info.build()], self.transfer_fence)
                .map_err(|e| HlxError::BackendError { message: format!("Submit failed: {}", e) })?;

            self.device.wait_for_fences(&[self.transfer_fence], true, u64::MAX)
                .map_err(|e| HlxError::BackendError { message: format!("Wait failed: {}", e) })?;
            self.device.reset_fences(&[self.transfer_fence])
                .map_err(|e| HlxError::BackendError { message: format!("Reset failed: {}", e) })?;

            // 6. Cleanup
            self.device.destroy_descriptor_pool(descriptor_pool, None);
        }

        Ok(())
    }

    fn matmul_bias(&mut self, _a: TensorHandle, _b: TensorHandle, _bias: TensorHandle, _out: TensorHandle) -> Result<()> { Ok(()) }

    fn relu(&mut self, input: TensorHandle, out: TensorHandle) -> Result<()> {
        self.activation(input, out, 0) // mode 0 = ReLU
    }

    fn gelu(&mut self, input: TensorHandle, out: TensorHandle) -> Result<()> {
        self.activation(input, out, 1) // mode 1 = GELU
    }

    fn layer_norm(&mut self, input: TensorHandle, gamma: TensorHandle, beta: TensorHandle, out: TensorHandle, eps: f64) -> Result<()> {
        let meta = self.tensor_meta(input)?;

        // Assume shape is (batch_size, hidden_size)
        if meta.shape.len() != 2 {
            return Err(HlxError::ValidationFail {
                message: "LayerNorm requires 2D tensor (batch_size, hidden_size)".to_string()
            });
        }

        let batch_size = meta.shape[0] as u32;
        let hidden_size = meta.shape[1] as u32;

        #[repr(C)]
        #[derive(Copy, Clone)]
        struct LayerNormPushConstants {
            batch_size: u32,
            hidden_size: u32,
            epsilon: f32,
            pass: u32,
        }
        unsafe impl bytemuck::Pod for LayerNormPushConstants {}
        unsafe impl bytemuck::Zeroable for LayerNormPushConstants {}

        unsafe {
            let pipeline = self.get_or_create_pipeline("layernorm", SHADER_LAYERNORM)?;

            // Allocate temporary buffers for statistics (means and variances)
            let stats_size = (batch_size * std::mem::size_of::<f32>() as u32) as u64;

            let mean_buffer_info = vk::BufferCreateInfo::builder()
                .size(stats_size)
                .usage(vk::BufferUsageFlags::STORAGE_BUFFER)
                .sharing_mode(vk::SharingMode::EXCLUSIVE);
            let mean_buffer = self.device.create_buffer(&mean_buffer_info, None)
                .map_err(|e| HlxError::BackendError { message: format!("Mean buffer failed: {}", e) })?;

            let var_buffer_info = vk::BufferCreateInfo::builder()
                .size(stats_size)
                .usage(vk::BufferUsageFlags::STORAGE_BUFFER)
                .sharing_mode(vk::SharingMode::EXCLUSIVE);
            let var_buffer = self.device.create_buffer(&var_buffer_info, None)
                .map_err(|e| HlxError::BackendError { message: format!("Var buffer failed: {}", e) })?;

            let mean_reqs = self.device.get_buffer_memory_requirements(mean_buffer);
            let var_reqs = self.device.get_buffer_memory_requirements(var_buffer);

            let mut allocator = self.allocator.lock().unwrap();
            let mean_alloc = allocator.allocate(&AllocationCreateDesc {
                name: "LayerNorm Mean",
                requirements: mean_reqs,
                location: MemoryLocation::GpuOnly,
                linear: true,
                allocation_scheme: AllocationScheme::GpuAllocatorManaged,
            }).map_err(|e| HlxError::BackendError { message: format!("Mean alloc failed: {}", e) })?;

            let var_alloc = allocator.allocate(&AllocationCreateDesc {
                name: "LayerNorm Variance",
                requirements: var_reqs,
                location: MemoryLocation::GpuOnly,
                linear: true,
                allocation_scheme: AllocationScheme::GpuAllocatorManaged,
            }).map_err(|e| HlxError::BackendError { message: format!("Var alloc failed: {}", e) })?;
            drop(allocator);

            self.device.bind_buffer_memory(mean_buffer, mean_alloc.memory(), mean_alloc.offset())
                .map_err(|e| HlxError::BackendError { message: format!("Mean bind failed: {}", e) })?;
            self.device.bind_buffer_memory(var_buffer, var_alloc.memory(), var_alloc.offset())
                .map_err(|e| HlxError::BackendError { message: format!("Var bind failed: {}", e) })?;

            // Create descriptor pool (6 buffers)
            let pool_sizes = [vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: 6,
            }];
            let pool_info = vk::DescriptorPoolCreateInfo::builder()
                .max_sets(1)
                .pool_sizes(&pool_sizes);
            let descriptor_pool = self.device.create_descriptor_pool(&pool_info, None)
                .map_err(|e| HlxError::BackendError { message: format!("Pool failed: {}", e) })?;

            // Allocate descriptor set
            let bindings = [
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(0).descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .descriptor_count(1).stage_flags(vk::ShaderStageFlags::COMPUTE).build(),
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(1).descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .descriptor_count(1).stage_flags(vk::ShaderStageFlags::COMPUTE).build(),
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(2).descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .descriptor_count(1).stage_flags(vk::ShaderStageFlags::COMPUTE).build(),
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(3).descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .descriptor_count(1).stage_flags(vk::ShaderStageFlags::COMPUTE).build(),
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(4).descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .descriptor_count(1).stage_flags(vk::ShaderStageFlags::COMPUTE).build(),
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(5).descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .descriptor_count(1).stage_flags(vk::ShaderStageFlags::COMPUTE).build(),
            ];
            let layout_info = vk::DescriptorSetLayoutCreateInfo::builder()
                .bindings(&bindings);
            let layernorm_layout = self.device.create_descriptor_set_layout(&layout_info, None)
                .map_err(|e| HlxError::BackendError { message: format!("Layout failed: {}", e) })?;

            let set_info = vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(descriptor_pool)
                .set_layouts(std::slice::from_ref(&layernorm_layout));
            let descriptor_set = self.device.allocate_descriptor_sets(&set_info)
                .map_err(|e| HlxError::BackendError { message: format!("Set alloc failed: {}", e) })?[0];

            // Update descriptor sets
            let buf_in = self.buffers.get(&input.0).unwrap().0;
            let buf_out = self.buffers.get(&out.0).unwrap().0;
            let buf_gamma = self.buffers.get(&gamma.0).unwrap().0;
            let buf_beta = self.buffers.get(&beta.0).unwrap().0;

            let writes = [
                vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_set).dst_binding(0).descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(&[vk::DescriptorBufferInfo { buffer: buf_in, offset: 0, range: vk::WHOLE_SIZE }]).build(),
                vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_set).dst_binding(1).descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(&[vk::DescriptorBufferInfo { buffer: buf_out, offset: 0, range: vk::WHOLE_SIZE }]).build(),
                vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_set).dst_binding(2).descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(&[vk::DescriptorBufferInfo { buffer: buf_gamma, offset: 0, range: vk::WHOLE_SIZE }]).build(),
                vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_set).dst_binding(3).descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(&[vk::DescriptorBufferInfo { buffer: buf_beta, offset: 0, range: vk::WHOLE_SIZE }]).build(),
                vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_set).dst_binding(4).descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(&[vk::DescriptorBufferInfo { buffer: mean_buffer, offset: 0, range: vk::WHOLE_SIZE }]).build(),
                vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_set).dst_binding(5).descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(&[vk::DescriptorBufferInfo { buffer: var_buffer, offset: 0, range: vk::WHOLE_SIZE }]).build(),
            ];
            self.device.update_descriptor_sets(&writes, &[]);

            // Pass 0: Compute statistics
            let push_pass0 = LayerNormPushConstants { batch_size, hidden_size, epsilon: eps as f32, pass: 0 };
            let push_bytes0 = bytemuck::bytes_of(&push_pass0);

            let begin_info = vk::CommandBufferBeginInfo::builder()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            self.device.begin_command_buffer(self.transfer_command_buffer, &begin_info)
                .map_err(|e| HlxError::BackendError { message: format!("Begin failed: {}", e) })?;

            self.device.cmd_bind_pipeline(self.transfer_command_buffer, vk::PipelineBindPoint::COMPUTE, pipeline);
            self.device.cmd_bind_descriptor_sets(self.transfer_command_buffer, vk::PipelineBindPoint::COMPUTE, self.pipeline_layout, 0, &[descriptor_set], &[]);
            self.device.cmd_push_constants(self.transfer_command_buffer, self.pipeline_layout, vk::ShaderStageFlags::COMPUTE, 0, push_bytes0);
            self.device.cmd_dispatch(self.transfer_command_buffer, 1, batch_size, 1);

            // Memory barrier between passes
            let barrier = vk::MemoryBarrier::builder()
                .src_access_mask(vk::AccessFlags::SHADER_WRITE)
                .dst_access_mask(vk::AccessFlags::SHADER_READ);
            self.device.cmd_pipeline_barrier(
                self.transfer_command_buffer,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::DependencyFlags::empty(),
                &[barrier.build()],
                &[],
                &[],
            );

            // Pass 1: Normalize
            let push_pass1 = LayerNormPushConstants { batch_size, hidden_size, epsilon: eps as f32, pass: 1 };
            let push_bytes1 = bytemuck::bytes_of(&push_pass1);
            self.device.cmd_push_constants(self.transfer_command_buffer, self.pipeline_layout, vk::ShaderStageFlags::COMPUTE, 0, push_bytes1);
            self.device.cmd_dispatch(self.transfer_command_buffer, 1, batch_size, 1);

            self.device.end_command_buffer(self.transfer_command_buffer)
                .map_err(|e| HlxError::BackendError { message: format!("End failed: {}", e) })?;

            // Submit
            let submit_info = vk::SubmitInfo::builder()
                .command_buffers(std::slice::from_ref(&self.transfer_command_buffer));
            self.device.queue_submit(self.queue, &[submit_info.build()], self.transfer_fence)
                .map_err(|e| HlxError::BackendError { message: format!("Submit failed: {}", e) })?;
            self.device.wait_for_fences(&[self.transfer_fence], true, u64::MAX)
                .map_err(|e| HlxError::BackendError { message: format!("Wait failed: {}", e) })?;
            self.device.reset_fences(&[self.transfer_fence])
                .map_err(|e| HlxError::BackendError { message: format!("Reset failed: {}", e) })?;

            // Cleanup
            self.device.destroy_descriptor_set_layout(layernorm_layout, None);
            self.device.destroy_descriptor_pool(descriptor_pool, None);
            self.device.destroy_buffer(mean_buffer, None);
            self.device.destroy_buffer(var_buffer, None);
            let mut allocator = self.allocator.lock().unwrap();
            allocator.free(mean_alloc).map_err(|e| HlxError::BackendError { message: format!("Free mean failed: {}", e) })?;
            allocator.free(var_alloc).map_err(|e| HlxError::BackendError { message: format!("Free var failed: {}", e) })?;
        }

        Ok(())
    }
    fn softmax(&mut self, input: TensorHandle, out: TensorHandle, _dim: i32) -> Result<()> {
        let meta = self.tensor_meta(input)?;

        // Assume shape is (batch_size, seq_len)
        if meta.shape.len() != 2 {
            return Err(HlxError::ValidationFail {
                message: "Softmax requires 2D tensor (batch_size, seq_len)".to_string()
            });
        }

        let batch_size = meta.shape[0] as u32;
        let seq_len = meta.shape[1] as u32;

        #[repr(C)]
        #[derive(Copy, Clone)]
        struct SoftmaxPushConstants {
            batch_size: u32,
            seq_len: u32,
            pass: u32,
            _padding: u32,
        }
        unsafe impl bytemuck::Pod for SoftmaxPushConstants {}
        unsafe impl bytemuck::Zeroable for SoftmaxPushConstants {}

        unsafe {
            let pipeline = self.get_or_create_pipeline("softmax", SHADER_SOFTMAX)?;

            // Allocate temporary buffers for max and sum
            let stats_size = (batch_size * std::mem::size_of::<f32>() as u32) as u64;

            let max_buffer_info = vk::BufferCreateInfo::builder()
                .size(stats_size)
                .usage(vk::BufferUsageFlags::STORAGE_BUFFER)
                .sharing_mode(vk::SharingMode::EXCLUSIVE);
            let max_buffer = self.device.create_buffer(&max_buffer_info, None)
                .map_err(|e| HlxError::BackendError { message: format!("Max buffer failed: {}", e) })?;

            let sum_buffer_info = vk::BufferCreateInfo::builder()
                .size(stats_size)
                .usage(vk::BufferUsageFlags::STORAGE_BUFFER)
                .sharing_mode(vk::SharingMode::EXCLUSIVE);
            let sum_buffer = self.device.create_buffer(&sum_buffer_info, None)
                .map_err(|e| HlxError::BackendError { message: format!("Sum buffer failed: {}", e) })?;

            let max_reqs = self.device.get_buffer_memory_requirements(max_buffer);
            let sum_reqs = self.device.get_buffer_memory_requirements(sum_buffer);

            let mut allocator = self.allocator.lock().unwrap();
            let max_alloc = allocator.allocate(&AllocationCreateDesc {
                name: "Softmax Max",
                requirements: max_reqs,
                location: MemoryLocation::GpuOnly,
                linear: true,
                allocation_scheme: AllocationScheme::GpuAllocatorManaged,
            }).map_err(|e| HlxError::BackendError { message: format!("Max alloc failed: {}", e) })?;

            let sum_alloc = allocator.allocate(&AllocationCreateDesc {
                name: "Softmax Sum",
                requirements: sum_reqs,
                location: MemoryLocation::GpuOnly,
                linear: true,
                allocation_scheme: AllocationScheme::GpuAllocatorManaged,
            }).map_err(|e| HlxError::BackendError { message: format!("Sum alloc failed: {}", e) })?;
            drop(allocator);

            self.device.bind_buffer_memory(max_buffer, max_alloc.memory(), max_alloc.offset())
                .map_err(|e| HlxError::BackendError { message: format!("Max bind failed: {}", e) })?;
            self.device.bind_buffer_memory(sum_buffer, sum_alloc.memory(), sum_alloc.offset())
                .map_err(|e| HlxError::BackendError { message: format!("Sum bind failed: {}", e) })?;

            // Create descriptor pool (4 buffers)
            let pool_sizes = [vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: 4,
            }];
            let pool_info = vk::DescriptorPoolCreateInfo::builder()
                .max_sets(1)
                .pool_sizes(&pool_sizes);
            let descriptor_pool = self.device.create_descriptor_pool(&pool_info, None)
                .map_err(|e| HlxError::BackendError { message: format!("Pool failed: {}", e) })?;

            // Allocate descriptor set
            let bindings = [
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(0).descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .descriptor_count(1).stage_flags(vk::ShaderStageFlags::COMPUTE).build(),
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(1).descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .descriptor_count(1).stage_flags(vk::ShaderStageFlags::COMPUTE).build(),
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(2).descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .descriptor_count(1).stage_flags(vk::ShaderStageFlags::COMPUTE).build(),
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(3).descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .descriptor_count(1).stage_flags(vk::ShaderStageFlags::COMPUTE).build(),
            ];
            let layout_info = vk::DescriptorSetLayoutCreateInfo::builder()
                .bindings(&bindings);
            let softmax_layout = self.device.create_descriptor_set_layout(&layout_info, None)
                .map_err(|e| HlxError::BackendError { message: format!("Layout failed: {}", e) })?;

            let set_info = vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(descriptor_pool)
                .set_layouts(std::slice::from_ref(&softmax_layout));
            let descriptor_set = self.device.allocate_descriptor_sets(&set_info)
                .map_err(|e| HlxError::BackendError { message: format!("Set alloc failed: {}", e) })?[0];

            // Update descriptor sets
            let buf_in = self.buffers.get(&input.0).unwrap().0;
            let buf_out = self.buffers.get(&out.0).unwrap().0;

            let writes = [
                vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_set).dst_binding(0).descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(&[vk::DescriptorBufferInfo { buffer: buf_in, offset: 0, range: vk::WHOLE_SIZE }]).build(),
                vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_set).dst_binding(1).descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(&[vk::DescriptorBufferInfo { buffer: buf_out, offset: 0, range: vk::WHOLE_SIZE }]).build(),
                vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_set).dst_binding(2).descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(&[vk::DescriptorBufferInfo { buffer: max_buffer, offset: 0, range: vk::WHOLE_SIZE }]).build(),
                vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_set).dst_binding(3).descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(&[vk::DescriptorBufferInfo { buffer: sum_buffer, offset: 0, range: vk::WHOLE_SIZE }]).build(),
            ];
            self.device.update_descriptor_sets(&writes, &[]);

            // Pass 0: Compute max and sum
            let push_pass0 = SoftmaxPushConstants { batch_size, seq_len, pass: 0, _padding: 0 };
            let push_bytes0 = bytemuck::bytes_of(&push_pass0);

            let begin_info = vk::CommandBufferBeginInfo::builder()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            self.device.begin_command_buffer(self.transfer_command_buffer, &begin_info)
                .map_err(|e| HlxError::BackendError { message: format!("Begin failed: {}", e) })?;

            self.device.cmd_bind_pipeline(self.transfer_command_buffer, vk::PipelineBindPoint::COMPUTE, pipeline);
            self.device.cmd_bind_descriptor_sets(self.transfer_command_buffer, vk::PipelineBindPoint::COMPUTE, self.pipeline_layout, 0, &[descriptor_set], &[]);
            self.device.cmd_push_constants(self.transfer_command_buffer, self.pipeline_layout, vk::ShaderStageFlags::COMPUTE, 0, push_bytes0);
            self.device.cmd_dispatch(self.transfer_command_buffer, 1, batch_size, 1);

            // Memory barrier between passes
            let barrier = vk::MemoryBarrier::builder()
                .src_access_mask(vk::AccessFlags::SHADER_WRITE)
                .dst_access_mask(vk::AccessFlags::SHADER_READ);
            self.device.cmd_pipeline_barrier(
                self.transfer_command_buffer,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::DependencyFlags::empty(),
                &[barrier.build()],
                &[],
                &[],
            );

            // Pass 1: Normalize
            let push_pass1 = SoftmaxPushConstants { batch_size, seq_len, pass: 1, _padding: 0 };
            let push_bytes1 = bytemuck::bytes_of(&push_pass1);
            self.device.cmd_push_constants(self.transfer_command_buffer, self.pipeline_layout, vk::ShaderStageFlags::COMPUTE, 0, push_bytes1);
            self.device.cmd_dispatch(self.transfer_command_buffer, 1, batch_size, 1);

            self.device.end_command_buffer(self.transfer_command_buffer)
                .map_err(|e| HlxError::BackendError { message: format!("End failed: {}", e) })?;

            // Submit
            let submit_info = vk::SubmitInfo::builder()
                .command_buffers(std::slice::from_ref(&self.transfer_command_buffer));
            self.device.queue_submit(self.queue, &[submit_info.build()], self.transfer_fence)
                .map_err(|e| HlxError::BackendError { message: format!("Submit failed: {}", e) })?;
            self.device.wait_for_fences(&[self.transfer_fence], true, u64::MAX)
                .map_err(|e| HlxError::BackendError { message: format!("Wait failed: {}", e) })?;
            self.device.reset_fences(&[self.transfer_fence])
                .map_err(|e| HlxError::BackendError { message: format!("Reset failed: {}", e) })?;

            // Cleanup
            self.device.destroy_descriptor_set_layout(softmax_layout, None);
            self.device.destroy_descriptor_pool(descriptor_pool, None);
            self.device.destroy_buffer(max_buffer, None);
            self.device.destroy_buffer(sum_buffer, None);
            let mut allocator = self.allocator.lock().unwrap();
            allocator.free(max_alloc).map_err(|e| HlxError::BackendError { message: format!("Free max failed: {}", e) })?;
            allocator.free(sum_alloc).map_err(|e| HlxError::BackendError { message: format!("Free sum failed: {}", e) })?;
        }

        Ok(())
    }
    fn attention(&mut self, _q: TensorHandle, _k: TensorHandle, _v: TensorHandle, _out: TensorHandle, _mask: Option<TensorHandle>, _scale: f64) -> Result<()> { Ok(()) }
    fn cross_entropy(&mut self, logits: TensorHandle, targets: TensorHandle, loss_out: TensorHandle, _probs_out: TensorHandle) -> Result<()> {
        // logits should already be probabilities (after softmax)
        let meta = self.tensor_meta(logits)?;

        // Assume shape is (batch_size, num_classes)
        if meta.shape.len() != 2 {
            return Err(HlxError::ValidationFail {
                message: "CrossEntropy requires 2D tensor (batch_size, num_classes)".to_string()
            });
        }

        let batch_size = meta.shape[0] as u32;
        let num_classes = meta.shape[1] as u32;

        #[repr(C)]
        #[derive(Copy, Clone)]
        struct CrossEntropyPushConstants {
            batch_size: u32,
            num_classes: u32,
            epsilon: f32,
            reduction: u32,  // 0 = none, 1 = mean, 2 = sum
        }
        unsafe impl bytemuck::Pod for CrossEntropyPushConstants {}
        unsafe impl bytemuck::Zeroable for CrossEntropyPushConstants {}

        unsafe {
            let pipeline = self.get_or_create_pipeline("cross_entropy", SHADER_CROSS_ENTROPY)?;

            // Descriptor Pool (3 buffers)
            let pool_sizes = [vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: 3,
            }];
            let pool_info = vk::DescriptorPoolCreateInfo::builder()
                .max_sets(1)
                .pool_sizes(&pool_sizes);
            let descriptor_pool = self.device.create_descriptor_pool(&pool_info, None)
                .map_err(|e| HlxError::BackendError { message: format!("Pool failed: {}", e) })?;

            let set_info = vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(descriptor_pool)
                .set_layouts(std::slice::from_ref(&self.descriptor_set_layout));
            let descriptor_set = self.device.allocate_descriptor_sets(&set_info)
                .map_err(|e| HlxError::BackendError { message: format!("Set alloc failed: {}", e) })?[0];

            // Update Descriptor Sets
            let buf_logits = self.buffers.get(&logits.0).unwrap().0;
            let buf_targets = self.buffers.get(&targets.0).unwrap().0;
            let buf_loss = self.buffers.get(&loss_out.0).unwrap().0;

            let writes = [
                vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_set).dst_binding(0).descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(&[vk::DescriptorBufferInfo { buffer: buf_logits, offset: 0, range: vk::WHOLE_SIZE }]).build(),
                vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_set).dst_binding(1).descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(&[vk::DescriptorBufferInfo { buffer: buf_targets, offset: 0, range: vk::WHOLE_SIZE }]).build(),
                vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_set).dst_binding(2).descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(&[vk::DescriptorBufferInfo { buffer: buf_loss, offset: 0, range: vk::WHOLE_SIZE }]).build(),
            ];
            self.device.update_descriptor_sets(&writes, &[]);

            // Push Constants (reduction = 0 for per-sample loss)
            let push_constants = CrossEntropyPushConstants {
                batch_size,
                num_classes,
                epsilon: 1e-8,
                reduction: 0,  // No reduction
            };
            let push_bytes = bytemuck::bytes_of(&push_constants);

            // Record & Submit
            let begin_info = vk::CommandBufferBeginInfo::builder()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            self.device.begin_command_buffer(self.transfer_command_buffer, &begin_info)
                .map_err(|e| HlxError::BackendError { message: format!("Begin failed: {}", e) })?;

            self.device.cmd_bind_pipeline(self.transfer_command_buffer, vk::PipelineBindPoint::COMPUTE, pipeline);
            self.device.cmd_bind_descriptor_sets(self.transfer_command_buffer, vk::PipelineBindPoint::COMPUTE, self.pipeline_layout, 0, &[descriptor_set], &[]);
            self.device.cmd_push_constants(self.transfer_command_buffer, self.pipeline_layout, vk::ShaderStageFlags::COMPUTE, 0, push_bytes);
            self.device.cmd_dispatch(self.transfer_command_buffer, 1, batch_size, 1);

            self.device.end_command_buffer(self.transfer_command_buffer)
                .map_err(|e| HlxError::BackendError { message: format!("End failed: {}", e) })?;

            let submit_info = vk::SubmitInfo::builder()
                .command_buffers(std::slice::from_ref(&self.transfer_command_buffer));
            self.device.queue_submit(self.queue, &[submit_info.build()], self.transfer_fence)
                .map_err(|e| HlxError::BackendError { message: format!("Submit failed: {}", e) })?;
            self.device.wait_for_fences(&[self.transfer_fence], true, u64::MAX)
                .map_err(|e| HlxError::BackendError { message: format!("Wait failed: {}", e) })?;
            self.device.reset_fences(&[self.transfer_fence])
                .map_err(|e| HlxError::BackendError { message: format!("Reset failed: {}", e) })?;

            // Cleanup
            self.device.destroy_descriptor_pool(descriptor_pool, None);
        }

        Ok(())
    }
    fn reduce_sum(&mut self, input: TensorHandle, out: TensorHandle, dim: Option<i32>) -> Result<()> {
        let meta = self.tensor_meta(input)?;

        // For simplicity, assume reducing over last dimension or full tensor
        let (input_size, reduce_size, output_size) = if let Some(d) = dim {
            let d = if d < 0 { (meta.shape.len() as i32 + d) as usize } else { d as usize };
            let before: usize = meta.shape[..d].iter().product();
            let reduce = meta.shape[d];
            let after: usize = meta.shape[(d+1)..].iter().product();
            (before * reduce * after, reduce, before * after)
        } else {
            // Reduce entire tensor
            let total = meta.shape.iter().product();
            (total, total, 1)
        };

        #[repr(C)]
        #[derive(Copy, Clone)]
        struct ReductionPushConstants {
            input_size: u32,
            reduce_size: u32,
            output_size: u32,
            op: u32,  // 0=sum, 1=max, 2=min, 3=mean, 4=product
        }
        unsafe impl bytemuck::Pod for ReductionPushConstants {}
        unsafe impl bytemuck::Zeroable for ReductionPushConstants {}

        unsafe {
            let pipeline = self.get_or_create_pipeline("reduction", SHADER_REDUCTION)?;

            // Descriptor Pool & Set
            let pool_sizes = [vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: 2,
            }];
            let pool_info = vk::DescriptorPoolCreateInfo::builder()
                .max_sets(1)
                .pool_sizes(&pool_sizes);
            let descriptor_pool = self.device.create_descriptor_pool(&pool_info, None)
                .map_err(|e| HlxError::BackendError { message: format!("Pool failed: {}", e) })?;

            let set_info = vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(descriptor_pool)
                .set_layouts(std::slice::from_ref(&self.descriptor_set_layout));
            let descriptor_set = self.device.allocate_descriptor_sets(&set_info)
                .map_err(|e| HlxError::BackendError { message: format!("Set alloc failed: {}", e) })?[0];

            // Update Descriptor Set
            let buf_in = self.buffers.get(&input.0).unwrap().0;
            let buf_out = self.buffers.get(&out.0).unwrap().0;

            let info_in = vk::DescriptorBufferInfo { buffer: buf_in, offset: 0, range: vk::WHOLE_SIZE };
            let info_out = vk::DescriptorBufferInfo { buffer: buf_out, offset: 0, range: vk::WHOLE_SIZE };

            let writes = [
                vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_set).dst_binding(0).descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(std::slice::from_ref(&info_in)).build(),
                vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_set).dst_binding(1).descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(std::slice::from_ref(&info_out)).build(),
            ];
            self.device.update_descriptor_sets(&writes, &[]);

            // Push Constants
            let push_constants = ReductionPushConstants {
                input_size: input_size as u32,
                reduce_size: reduce_size as u32,
                output_size: output_size as u32,
                op: 0,  // OP_SUM
            };
            let push_bytes = bytemuck::bytes_of(&push_constants);

            // Record & Submit
            let begin_info = vk::CommandBufferBeginInfo::builder()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            self.device.begin_command_buffer(self.transfer_command_buffer, &begin_info)
                .map_err(|e| HlxError::BackendError { message: format!("Begin failed: {}", e) })?;

            self.device.cmd_bind_pipeline(self.transfer_command_buffer, vk::PipelineBindPoint::COMPUTE, pipeline);
            self.device.cmd_bind_descriptor_sets(self.transfer_command_buffer, vk::PipelineBindPoint::COMPUTE, self.pipeline_layout, 0, &[descriptor_set], &[]);
            self.device.cmd_push_constants(self.transfer_command_buffer, self.pipeline_layout, vk::ShaderStageFlags::COMPUTE, 0, push_bytes);

            // Dispatch: (1, output_size, 1) - one workgroup per output element
            self.device.cmd_dispatch(self.transfer_command_buffer, 1, output_size as u32, 1);

            self.device.end_command_buffer(self.transfer_command_buffer)
                .map_err(|e| HlxError::BackendError { message: format!("End failed: {}", e) })?;

            let submit_info = vk::SubmitInfo::builder()
                .command_buffers(std::slice::from_ref(&self.transfer_command_buffer));
            self.device.queue_submit(self.queue, &[submit_info.build()], self.transfer_fence)
                .map_err(|e| HlxError::BackendError { message: format!("Submit failed: {}", e) })?;

            self.device.wait_for_fences(&[self.transfer_fence], true, u64::MAX)
                .map_err(|e| HlxError::BackendError { message: format!("Wait failed: {}", e) })?;
            self.device.reset_fences(&[self.transfer_fence])
                .map_err(|e| HlxError::BackendError { message: format!("Reset failed: {}", e) })?;

            // Cleanup
            self.device.destroy_descriptor_pool(descriptor_pool, None);
        }

        Ok(())
    }
    fn embedding(&mut self, _indices: TensorHandle, _weight: TensorHandle, _out: TensorHandle) -> Result<()> { Ok(()) }
    fn adam_update(&mut self, _param: TensorHandle, _grad: TensorHandle, _m: TensorHandle, _v: TensorHandle, _lr: f64, _beta1: f64, _beta2: f64, _eps: f64, _step: u64) -> Result<()> { Ok(()) }

    fn dispatch_compute(
        &mut self,
        shader_bytes: &[u8],
        bindings: &[TensorHandle],
        push_constants: &[u8],
        workgroup_count: [u32; 3],
    ) -> Result<()> {
        use std::io::Write;
        println!("[Vulkan] Dispatching Compute...");
        std::io::stdout().flush().ok();
        
        unsafe {
            // 1. Create Descriptor Set Layout
            println!("[Vulkan] Creating DS Layout with {} bindings", bindings.len());
            std::io::stdout().flush().ok();
            
            let mut vk_bindings = Vec::with_capacity(bindings.len());
            for i in 0..bindings.len() {
                vk_bindings.push(vk::DescriptorSetLayoutBinding::builder()
                    .binding(i as u32)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::COMPUTE)
                    .build());
            }

            let layout_info = vk::DescriptorSetLayoutCreateInfo::builder()
                .bindings(&vk_bindings);
            
            let ds_layout = self.device.create_descriptor_set_layout(&layout_info, None)
                .map_err(|e| HlxError::BackendError { message: format!("DS layout failed: {}", e) })?;

            // 2. Create Pipeline Layout
            println!("[Vulkan] Creating Pipeline Layout (PC size: {})", push_constants.len());
            std::io::stdout().flush().ok();
            let push_ranges = if !push_constants.is_empty() {
                vec![vk::PushConstantRange::builder()
                    .stage_flags(vk::ShaderStageFlags::COMPUTE)
                    .offset(0)
                    .size(push_constants.len() as u32)
                    .build()]
            } else {
                vec![]
            };

            let pipeline_layout_info = vk::PipelineLayoutCreateInfo::builder()
                .set_layouts(std::slice::from_ref(&ds_layout))
                .push_constant_ranges(&push_ranges);
            
            let pipeline_layout = self.device.create_pipeline_layout(&pipeline_layout_info, None)
                .map_err(|e| HlxError::BackendError { message: format!("Pipeline layout failed: {}", e) })?;

            // 3. Create Pipeline
            println!("[Vulkan] Creating Shader Module ({} bytes)", shader_bytes.len());
            std::io::stdout().flush().ok();
            let code_len = shader_bytes.len() / 4;
            let mut code = Vec::with_capacity(code_len);
            for i in 0..code_len {
                let bytes = [
                    shader_bytes[i * 4],
                    shader_bytes[i * 4 + 1],
                    shader_bytes[i * 4 + 2],
                    shader_bytes[i * 4 + 3],
                ];
                code.push(u32::from_le_bytes(bytes));
            }

            let shader_module_info = vk::ShaderModuleCreateInfo::builder().code(&code);
            let shader_module = self.device.create_shader_module(&shader_module_info, None)
                .map_err(|e| HlxError::BackendError { message: format!("Shader module failed: {}", e) })?;

            let entry_name = CStr::from_bytes_with_nul(b"main\0").unwrap();
            let stage_info = vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::COMPUTE)
                .module(shader_module)
                .name(entry_name);

            let pipeline_info = vk::ComputePipelineCreateInfo::builder()
                .stage(stage_info.build())
                .layout(pipeline_layout);

            println!("[Vulkan] Creating Compute Pipeline...");
            std::io::stdout().flush().ok();
            let pipeline = self.device.create_compute_pipelines(vk::PipelineCache::null(), &[pipeline_info.build()], None)
                .map_err(|e| HlxError::BackendError { message: format!("Pipeline creation failed: {:?}", e) })?[0];

            // 4. Descriptor Pool & Set
            println!("[Vulkan] Allocating Descriptor Pool...");
            std::io::stdout().flush().ok();
            let pool_sizes = [vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: std::cmp::max(1, bindings.len() as u32),
            }];
            let pool_info = vk::DescriptorPoolCreateInfo::builder()
                .max_sets(1)
                .pool_sizes(&pool_sizes);
            let descriptor_pool = self.device.create_descriptor_pool(&pool_info, None)
                .map_err(|e| HlxError::BackendError { message: format!("Pool failed: {}", e) })?;

            let set_info = vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(descriptor_pool)
                .set_layouts(std::slice::from_ref(&ds_layout));
            let descriptor_set = self.device.allocate_descriptor_sets(&set_info)
                .map_err(|e| HlxError::BackendError { message: format!("Set alloc failed: {}", e) })?[0];

            // 5. Update Descriptor Sets
            println!("[Vulkan] Updating Descriptor Sets...");
            std::io::stdout().flush().ok();
            if !bindings.is_empty() {
                let mut writes = Vec::with_capacity(bindings.len());
                let mut buffer_infos = Vec::with_capacity(bindings.len()); 

                for handle in bindings.iter() {
                    let buf = self.buffers.get(&handle.0).ok_or(HlxError::ValidationFail { message: "Invalid handle".to_string() })?.0;
                    buffer_infos.push(vk::DescriptorBufferInfo { buffer: buf, offset: 0, range: vk::WHOLE_SIZE });
                }

                for (i, info) in buffer_infos.iter().enumerate() {
                    writes.push(vk::WriteDescriptorSet::builder()
                        .dst_set(descriptor_set)
                        .dst_binding(i as u32)
                        .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                        .buffer_info(std::slice::from_ref(info))
                        .build());
                }
                self.device.update_descriptor_sets(&writes, &[]);
            }

            // 6. Record & Dispatch
            println!("[Vulkan] Recording Commands...");
            std::io::stdout().flush().ok();
            let begin_info = vk::CommandBufferBeginInfo::builder().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            self.device.begin_command_buffer(self.transfer_command_buffer, &begin_info).map_err(|e| HlxError::BackendError { message: e.to_string() })?;

            self.device.cmd_bind_pipeline(self.transfer_command_buffer, vk::PipelineBindPoint::COMPUTE, pipeline);
            self.device.cmd_bind_descriptor_sets(self.transfer_command_buffer, vk::PipelineBindPoint::COMPUTE, pipeline_layout, 0, &[descriptor_set], &[]);
            if !push_constants.is_empty() {
                self.device.cmd_push_constants(self.transfer_command_buffer, pipeline_layout, vk::ShaderStageFlags::COMPUTE, 0, push_constants);
            }
            println!("[Vulkan] Dispatching ({}, {}, {})...", workgroup_count[0], workgroup_count[1], workgroup_count[2]);
            std::io::stdout().flush().ok();
            self.device.cmd_dispatch(self.transfer_command_buffer, workgroup_count[0], workgroup_count[1], workgroup_count[2]);

            self.device.end_command_buffer(self.transfer_command_buffer).map_err(|e| HlxError::BackendError { message: e.to_string() })?;

            println!("[Vulkan] Submitting...");
            std::io::stdout().flush().ok();
            let submit_info = vk::SubmitInfo::builder().command_buffers(std::slice::from_ref(&self.transfer_command_buffer));
            self.device.queue_submit(self.queue, &[submit_info.build()], self.transfer_fence).map_err(|e| HlxError::BackendError { message: e.to_string() })?;

            println!("[Vulkan] Waiting...");
            std::io::stdout().flush().ok();
            self.device.wait_for_fences(&[self.transfer_fence], true, u64::MAX).map_err(|e| HlxError::BackendError { message: e.to_string() })?;
            self.device.reset_fences(&[self.transfer_fence]).map_err(|e| HlxError::BackendError { message: e.to_string() })?;

            // 7. Cleanup
            println!("[Vulkan] Cleanup...");
            std::io::stdout().flush().ok();
            self.device.destroy_descriptor_pool(descriptor_pool, None);
            self.device.destroy_pipeline(pipeline, None);
            self.device.destroy_shader_module(shader_module, None);
            self.device.destroy_pipeline_layout(pipeline_layout, None);
            self.device.destroy_descriptor_set_layout(ds_layout, None);
        }
        Ok(())
    }

    // === Image Processing Operations ===

    fn gaussian_blur(
        &mut self,
        _input: TensorHandle,
        _out: TensorHandle,
        _sigma: &Value,
    ) -> Result<()> {
        // TODO: Dispatch gaussian_blur.comp shader
        Err(HlxError::BackendError {
            message: "gaussian_blur shader dispatch not yet implemented for Vulkan backend".to_string(),
        })
    }

    fn sobel_edges(
        &mut self,
        _input: TensorHandle,
        _out: TensorHandle,
        _threshold: &Value,
    ) -> Result<()> {
        // TODO: Dispatch sobel.comp shader
        Err(HlxError::BackendError {
            message: "sobel_edges shader dispatch not yet implemented for Vulkan backend".to_string(),
        })
    }

    fn grayscale(
        &mut self,
        _input: TensorHandle,
        _out: TensorHandle,
    ) -> Result<()> {
        // TODO: Implement grayscale compute shader
        Err(HlxError::BackendError {
            message: "grayscale not yet implemented for Vulkan backend".to_string(),
        })
    }

    fn threshold(
        &mut self,
        _input: TensorHandle,
        _out: TensorHandle,
        _value: &Value,
    ) -> Result<()> {
        // TODO: Implement threshold compute shader
        Err(HlxError::BackendError {
            message: "threshold not yet implemented for Vulkan backend".to_string(),
        })
    }

    fn brightness(
        &mut self,
        _input: TensorHandle,
        _out: TensorHandle,
        _factor: &Value,
    ) -> Result<()> {
        // TODO: Implement brightness compute shader
        Err(HlxError::BackendError {
            message: "brightness not yet implemented for Vulkan backend".to_string(),
        })
    }

    fn contrast(
        &mut self,
        _input: TensorHandle,
        _out: TensorHandle,
        _factor: &Value,
    ) -> Result<()> {
        // TODO: Implement contrast compute shader
        Err(HlxError::BackendError {
            message: "contrast not yet implemented for Vulkan backend".to_string(),
        })
    }

    fn invert_colors(
        &mut self,
        _input: TensorHandle,
        _out: TensorHandle,
    ) -> Result<()> {
        // TODO: Implement invert_colors compute shader
        Err(HlxError::BackendError {
            message: "invert_colors not yet implemented for Vulkan backend".to_string(),
        })
    }

    fn sharpen(
        &mut self,
        _input: TensorHandle,
        _out: TensorHandle,
    ) -> Result<()> {
        // TODO: Implement sharpen compute shader
        Err(HlxError::BackendError {
            message: "sharpen not yet implemented for Vulkan backend".to_string(),
        })
    }

    fn sync(&mut self) -> Result<()> {
        unsafe {
            self.device.device_wait_idle()
                .map_err(|e| HlxError::BackendError { message: format!("Wait idle failed: {}", e) })
        }
    }
}

// VulkanBackend-specific helper methods
// Cleanup
impl Drop for VulkanBackend {
    fn drop(&mut self) {
        unsafe {
            // Wait for all GPU operations to complete
            let _ = self.device.device_wait_idle();

            // Free all GPU buffers and their allocations
            {
                let mut allocator = self.allocator.lock().unwrap();
                for (_handle, (buffer, allocation)) in self.buffers.drain() {
                    self.device.destroy_buffer(buffer, None);
                    let _ = allocator.free(allocation);
                }
            } // Drop MutexGuard here

            // Destroy compute pipelines
            for (_name, pipeline) in self.pipelines.drain() {
                self.device.destroy_pipeline(pipeline, None);
            }

            // Destroy pipeline layout and descriptor set layout
            self.device.destroy_pipeline_layout(self.pipeline_layout, None);
            self.device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);

            // Destroy synchronization primitives
            self.device.destroy_fence(self.transfer_fence, None);

            // Destroy command pool (this also frees command buffers)
            self.device.destroy_command_pool(self.command_pool, None);

            // CRITICAL FIX FOR SEGFAULT:
            // Drop the allocator Arc BEFORE destroying the device.
            // gpu-allocator's Allocator internally holds a reference to the Vulkan device.
            // Rust's automatic field drop order (struct declaration order) would drop
            // device BEFORE allocator, causing use-after-free segfault on shutdown.
            // We wrap allocator in ManuallyDrop and explicitly drop it here.
            ManuallyDrop::drop(&mut self.allocator);

            // NOW safe to destroy device and instance
            self.device.destroy_device(None);
            self.instance.destroy_instance(None)
        }
    }
}
