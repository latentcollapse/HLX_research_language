//! Vulkan Backend for HLX Runtime
//!
//! Executes LC-B instructions on GPU via SPIR-V compute shaders.
//! Prioritizes determinism and cross-vendor compatibility.

use ash::{vk, Entry, Instance, Device};
use gpu_allocator::vulkan::*;
use gpu_allocator::MemoryLocation;
use std::sync::{Arc, Mutex};
use std::ffi::CStr;
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
    allocator: Arc<Mutex<Allocator>>,
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

const SHADER_ADD: &[u8] = include_bytes!("vulkan/shaders/pointwise_add.spv");

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
            }).ok_or(HlxError::BackendError { message: "No compute-capable GPU found".to_string() })?;

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
                .size(4); // single uint32 for 'n'

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
                allocator: Arc::new(Mutex::new(allocator)),
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
            let shader_module_info = vk::ShaderModuleCreateInfo::builder()
                .code(bytemuck::cast_slice(spv));
            
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
}

// Placeholder implementation of Backend trait
impl Backend for VulkanBackend {
    fn name(&self) -> &'static str { "Vulkan" }
    
    fn is_available(&self) -> bool { true }

    fn alloc_tensor(&mut self, shape: &[usize], dtype: DType) -> Result<TensorHandle> {
        let size_bytes = (shape.iter().product::<usize>() * dtype.size_bytes()) as u64;
        
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

    fn matmul(&mut self, _a: TensorHandle, _b: TensorHandle, _out: TensorHandle) -> Result<()> {
        // TODO: Record command buffer
        Ok(())
    }

    fn matmul_bias(&mut self, _a: TensorHandle, _b: TensorHandle, _bias: TensorHandle, _out: TensorHandle) -> Result<()> { Ok(()) }
    fn layer_norm(&mut self, _input: TensorHandle, _gamma: TensorHandle, _beta: TensorHandle, _out: TensorHandle, _eps: f64) -> Result<()> { Ok(()) }
    fn softmax(&mut self, _input: TensorHandle, _out: TensorHandle, _dim: i32) -> Result<()> { Ok(()) }
    fn gelu(&mut self, _input: TensorHandle, _out: TensorHandle) -> Result<()> { Ok(()) }
    fn relu(&mut self, _input: TensorHandle, _out: TensorHandle) -> Result<()> { Ok(()) }
    fn attention(&mut self, _q: TensorHandle, _k: TensorHandle, _v: TensorHandle, _out: TensorHandle, _mask: Option<TensorHandle>, _scale: f64) -> Result<()> { Ok(()) }
    fn cross_entropy(&mut self, _logits: TensorHandle, _targets: TensorHandle, _loss_out: TensorHandle, _probs_out: TensorHandle) -> Result<()> { Ok(()) }
    fn reduce_sum(&mut self, _input: TensorHandle, _out: TensorHandle, _dim: Option<i32>) -> Result<()> { Ok(()) }
    fn embedding(&mut self, _indices: TensorHandle, _weight: TensorHandle, _out: TensorHandle) -> Result<()> { Ok(()) }
    fn adam_update(&mut self, _param: TensorHandle, _grad: TensorHandle, _m: TensorHandle, _v: TensorHandle, _lr: f64, _beta1: f64, _beta2: f64, _eps: f64, _step: u64) -> Result<()> { Ok(()) }
    
    fn sync(&mut self) -> Result<()> {
        unsafe {
            self.device.device_wait_idle()
                .map_err(|e| HlxError::BackendError { message: format!("Wait idle failed: {}", e) })
        }
    }
}

// Cleanup
impl Drop for VulkanBackend {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}
