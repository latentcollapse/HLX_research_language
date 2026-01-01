//! Vulkan context management
//!
//! Handles instance, device, and queue initialization.
//! Manages shader module caching for HLX determinism.

use ash::{vk, Entry, Instance, Device};
use pyo3::prelude::*;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::ffi::CString;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::error::VulkanErrorKind;
use crate::shader::ShaderModule;
use crate::pipeline::GraphicsPipeline;
use crate::buffer::Buffer;
use crate::validation;

/// Manages Vulkan instance, device, and shader cache.
///
/// This is the main entry point for HLX Vulkan operations.
/// It provides content-addressed shader caching where the same
/// SPIR-V binary always produces the same shader ID.
///
/// # Python Example
///
/// ```python
/// from hlx_vulkan import VulkanContext
///
/// # Create context (auto-selects first GPU)
/// ctx = VulkanContext()
/// print(f"Using: {ctx.device_name}")
///
/// # Load shader (returns content-addressed ID)
/// shader_id = ctx.load_shader(spirv_bytes, "main")
///
/// # Second load of same bytes returns same ID (cache hit)
/// assert ctx.load_shader(spirv_bytes, "main") == shader_id
/// assert ctx.is_shader_cached(shader_id)
///
/// # Cleanup
/// ctx.cleanup()
/// ```
#[pyclass]
pub struct VulkanContext {
    /// GPU device name (e.g., "NVIDIA GeForce RTX 5060")
    #[pyo3(get)]
    pub device_name: String,

    /// Vulkan API version (e.g., "1.4.312")
    #[pyo3(get)]
    pub api_version: String,

    // Internal Vulkan state (not exposed to Python)
    #[allow(dead_code)]
    entry: Entry,
    instance: Instance,
    #[allow(dead_code)]
    physical_device: vk::PhysicalDevice,
    device: Device,
    #[allow(dead_code)]
    compute_queue: vk::Queue,
    #[allow(dead_code)]
    compute_queue_family: u32,

    // Physical device memory properties (used for buffer allocation)
    memory_properties: vk::PhysicalDeviceMemoryProperties,

    // HLX-specific: content-addressed shader cache
    // Key: hex hash of SPIR-V bytes
    // Value: compiled VkShaderModule wrapper
    shader_cache: HashMap<String, ShaderModule>,

    // Pipeline cache
    // Key: pipeline ID (contract name)
    // Value: compiled graphics pipeline (we store these but don't expose to Python yet)
    #[allow(dead_code)]
    pipeline_cache: HashMap<String, Option<(vk::Pipeline, vk::PipelineLayout)>>,

    // Render pass cache
    render_pass_cache: HashMap<String, vk::RenderPass>,

    // Buffer cache
    // Key: buffer ID (content-addressed hex string)
    // Value: GPU buffer wrapper
    buffer_cache: HashMap<String, Buffer>,

    // Track cleanup state
    cleaned_up: bool,
}

#[pymethods]
impl VulkanContext {
    /// Create a new Vulkan context.
    ///
    /// Initializes Vulkan instance, selects a physical device,
    /// creates a logical device with a compute queue.
    ///
    /// # Arguments
    ///
    /// * `device_index` - GPU index (0 = first available, default)
    /// * `enable_validation` - Enable Vulkan validation layers (default: false)
    ///
    /// # Returns
    ///
    /// VulkanContext instance ready for shader loading.
    ///
    /// # Raises
    ///
    /// * `RuntimeError` - If Vulkan initialization fails
    #[new]
    #[pyo3(signature = (device_index=0, enable_validation=false))]
    pub fn new(device_index: usize, enable_validation: bool) -> PyResult<Self> {
        log::info!(
            "Initializing VulkanContext (device_index={}, validation={})",
            device_index,
            enable_validation
        );

        // Load Vulkan entry points
        let entry = unsafe { Entry::load() }
            .map_err(|e| VulkanErrorKind::EntryLoadFailed(e.to_string()))?;

        // Create instance
        let instance = Self::create_instance(&entry, enable_validation)?;

        // Select physical device
        let (physical_device, device_name, api_version) =
            Self::select_physical_device(&instance, device_index)?;

        // Create logical device and get compute queue
        let (device, compute_queue, compute_queue_family) =
            Self::create_logical_device(&instance, physical_device)?;

        // Get physical device memory properties for buffer allocation
        let memory_properties =
            unsafe { instance.get_physical_device_memory_properties(physical_device) };

        log::info!(
            "VulkanContext initialized: {} (Vulkan {})",
            device_name,
            api_version
        );

        Ok(Self {
            device_name,
            api_version,
            entry,
            instance,
            physical_device,
            device,
            compute_queue,
            compute_queue_family,
            memory_properties,
            shader_cache: HashMap::new(),
            pipeline_cache: HashMap::new(),
            render_pass_cache: HashMap::new(),
            buffer_cache: HashMap::new(),
            cleaned_up: false,
        })
    }

    /// Load a shader from SPIR-V binary.
    ///
    /// Uses content-addressed caching: identical SPIR-V bytes always
    /// return the same shader ID, and the second load is a cache hit.
    ///
    /// # Arguments
    ///
    /// * `spirv_bytes` - SPIR-V binary as bytes
    /// * `entry_point` - Shader entry point name (usually "main")
    ///
    /// # Returns
    ///
    /// Shader ID (16-character hex string). Use this ID for:
    /// - Cache checks with `is_shader_cached()`
    /// - Future pipeline creation (Phase 2)
    ///
    /// # Raises
    ///
    /// * `RuntimeError` - If SPIR-V is invalid or shader creation fails
    pub fn load_shader(&mut self, spirv_bytes: &[u8], entry_point: &str) -> PyResult<String> {
        // Generate content-addressed ID from SPIR-V bytes
        let shader_id = Self::compute_shader_id(spirv_bytes);

        // Check cache first
        if self.shader_cache.contains_key(&shader_id) {
            log::debug!("Shader cache HIT: {}", shader_id);
            return Ok(shader_id);
        }

        log::debug!("Shader cache MISS: {} (loading)", shader_id);

        // Validate SPIR-V before passing to Vulkan
        validation::validate_spirv(spirv_bytes)
            .map_err(|e| VulkanErrorKind::InvalidSpirv(e))?;

        // Create shader module
        let shader_module = ShaderModule::new(
            &self.device,
            spirv_bytes,
            entry_point.to_string(),
        )?;

        // Cache it
        self.shader_cache.insert(shader_id.clone(), shader_module);

        log::info!(
            "Shader loaded: {} (entry={}, size={}, cache_size={})",
            shader_id,
            entry_point,
            spirv_bytes.len(),
            self.shader_cache.len()
        );

        Ok(shader_id)
    }

    /// Load a shader from the ShaderDatabase.
    ///
    /// Imports the Python `shaderdb` module, queries the database for
    /// a shader by handle, and loads it using the existing `load_shader` logic.
    ///
    /// # Arguments
    ///
    /// * `py` - Python runtime reference
    /// * `handle` - Shader handle (e.g., "&h_shader_ea82522...")
    /// * `db_path` - Path to shader database directory
    ///
    /// # Returns
    ///
    /// Shader ID (16-character hex string) on success.
    ///
    /// # Raises
    ///
    /// * `RuntimeError` - If database query fails or shader is invalid
    pub fn load_shader_from_db(
        &mut self,
        py: Python,
        handle: String,
        db_path: String,
    ) -> PyResult<String> {
        log::info!(
            "Loading shader from database: handle={}, db_path={}",
            handle,
            db_path
        );

        // Import shaderdb module from Python
        // Try hlx_runtime.shaderdb first, then fallback to shaderdb
        let shaderdb = match py.import_bound("hlx_runtime.shaderdb") {
            Ok(m) => m,
            Err(_) => py.import_bound("shaderdb")?,
        };

        // Create ShaderDatabase instance
        let db = shaderdb
            .getattr("ShaderDatabase")?
            .call1((db_path.clone(),))?;

        // Query database for SPIR-V bytes
        let spirv_bytes: Vec<u8> = db.call_method1("get", (handle.clone(),))?.extract()?;

        if spirv_bytes.is_empty() {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                format!("Empty SPIR-V returned for handle: {}", handle),
            ));
        }

        log::debug!(
            "Retrieved {} bytes from database for handle: {}",
            spirv_bytes.len(),
            handle
        );

        // Use existing load_shader logic with "main" as default entry point
        self.load_shader(spirv_bytes.as_slice(), "main")
    }

    /// Check if a shader is in the cache.
    ///
    /// # Arguments
    ///
    /// * `shader_id` - Shader ID returned by `load_shader()`
    ///
    /// # Returns
    ///
    /// True if the shader is cached, False otherwise.
    pub fn is_shader_cached(&self, shader_id: &str) -> bool {
        self.shader_cache.contains_key(shader_id)
    }

    /// Get current shader cache size.
    ///
    /// # Returns
    ///
    /// Number of cached shader modules.
    pub fn cache_size(&self) -> usize {
        self.shader_cache.len()
    }

    /// Create a vertex buffer from vertex data.
    ///
    /// # Arguments
    ///
    /// * `vertices` - Flat array of f32: [x,y,z, nx,ny,nz, r,g,b, ...]
    ///   Each vertex is 9 floats (36 bytes): 3 pos + 3 normal + 3 color
    ///
    /// # Returns
    ///
    /// Buffer ID (16-character hex string) for use in rendering
    ///
    /// # Raises
    ///
    /// * `RuntimeError` - If buffer creation or upload fails
    pub fn create_vertex_buffer(
        &mut self,
        vertices: Vec<f32>,
    ) -> PyResult<String> {
        if vertices.is_empty() {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "Vertex buffer cannot be empty",
            ));
        }

        let size = (vertices.len() * std::mem::size_of::<f32>()) as u64;
        log::info!(
            "Creating vertex buffer: {} vertices ({} bytes)",
            vertices.len() / 9,
            size
        );

        // Create buffer with VERTEX_BUFFER usage and HOST_VISIBLE memory
        let buffer = Buffer::new(
            Arc::new(self.device.clone()),
            size,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            self.memory_properties,
        )
        .map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!(
                "Failed to create vertex buffer: {}",
                e
            ))
        })?;

        // Upload vertex data
        buffer.upload_data(&vertices).map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!(
                "Failed to upload vertex data: {}",
                e
            ))
        })?;

        // Generate buffer ID from vertex data (content-addressed)
        // Convert f32 slice to bytes for hashing
        let vertices_bytes = unsafe {
            std::slice::from_raw_parts(
                vertices.as_ptr() as *const u8,
                vertices.len() * std::mem::size_of::<f32>(),
            )
        };
        let buffer_id = Self::compute_buffer_id(vertices_bytes);

        log::info!(
            "Vertex buffer created: {} (ID: {})",
            size,
            buffer_id
        );

        // Cache the buffer
        self.buffer_cache.insert(buffer_id.clone(), buffer);

        Ok(buffer_id)
    }

    /// Create a uniform buffer for matrices and constants.
    ///
    /// # Arguments
    ///
    /// * `size_bytes` - Size of buffer in bytes (e.g., 64 for 4×4 matrix)
    ///
    /// # Returns
    ///
    /// Buffer ID (16-character hex string)
    ///
    /// # Raises
    ///
    /// * `RuntimeError` - If buffer creation fails
    pub fn create_uniform_buffer(
        &mut self,
        size_bytes: usize,
    ) -> PyResult<String> {
        if size_bytes == 0 {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "Uniform buffer size must be > 0",
            ));
        }

        log::info!("Creating uniform buffer: {} bytes", size_bytes);

        // Create buffer with UNIFORM_BUFFER usage and HOST_VISIBLE memory
        let buffer = Buffer::new(
            Arc::new(self.device.clone()),
            size_bytes as u64,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            self.memory_properties,
        )
        .map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!(
                "Failed to create uniform buffer: {}",
                e
            ))
        })?;

        // Generate buffer ID from size (deterministic)
        let buffer_id = Self::compute_buffer_id(&size_bytes.to_le_bytes());

        log::info!(
            "Uniform buffer created: {} bytes (ID: {})",
            size_bytes,
            buffer_id
        );

        // Cache the buffer
        self.buffer_cache.insert(buffer_id.clone(), buffer);

        Ok(buffer_id)
    }

    /// Update a uniform buffer with new data.
    ///
    /// # Arguments
    ///
    /// * `buffer_id` - Buffer ID returned by `create_uniform_buffer()`
    /// * `data` - Byte data to upload (list of u8)
    ///
    /// # Returns
    ///
    /// Ok on success
    ///
    /// # Raises
    ///
    /// * `RuntimeError` - If buffer not found or upload fails
    pub fn update_uniform_buffer(
        &mut self,
        buffer_id: String,
        data: Vec<u8>,
    ) -> PyResult<()> {
        let buffer = self.buffer_cache.get(&buffer_id).ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(format!(
                "Uniform buffer not found: {}",
                buffer_id
            ))
        })?;

        if data.len() as u64 > buffer.len() {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(format!(
                "Data size {} exceeds buffer size {}",
                data.len(),
                buffer.len()
            )));
        }

        buffer.upload_data(&data).map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!(
                "Failed to update uniform buffer: {}",
                e
            ))
        })?;

        log::debug!(
            "Updated uniform buffer {}: {} bytes",
            buffer_id,
            data.len()
        );

        Ok(())
    }

    /// Clear the buffer cache.
    ///
    /// Destroys all cached buffers and frees GPU memory.
    pub fn clear_buffer_cache(&mut self) {
        log::info!("Clearing buffer cache ({} entries)", self.buffer_cache.len());
        for (id, _buffer) in self.buffer_cache.drain() {
            log::debug!("Destroying cached buffer: {}", id);
        }
    }

    /// Clear the shader cache.
    ///
    /// Destroys all cached VkShaderModule objects and frees GPU resources.
    pub fn clear_cache(&mut self) {
        log::info!("Clearing shader cache ({} entries)", self.shader_cache.len());
        for (id, module) in self.shader_cache.drain() {
            log::debug!("Destroying cached shader: {}", id);
            module.destroy(&self.device);
        }
    }

    /// Get GPU memory information.
    ///
    /// # Returns
    ///
    /// Dict with:
    /// - `heap_count`: Number of memory heaps
    /// - `heap_sizes_bytes`: List of heap sizes in bytes
    pub fn get_memory_info(&self, py: Python<'_>) -> PyResult<PyObject> {
        let props = unsafe {
            self.instance
                .get_physical_device_memory_properties(self.physical_device)
        };

        let heaps: Vec<u64> = props.memory_heaps[..props.memory_heap_count as usize]
            .iter()
            .map(|h| h.size)
            .collect();

        let dict = pyo3::types::PyDict::new_bound(py);
        dict.set_item("heap_count", props.memory_heap_count)?;
        dict.set_item("heap_sizes_bytes", heaps)?;

        Ok(dict.into())
    }

    /// Create a graphics pipeline from a CONTRACT_902 JSON definition
    ///
    /// Parses the contract to extract shader handles, creates shader modules,
    /// and builds a graphics pipeline.
    ///
    /// # Arguments
    ///
    /// * `py` - Python runtime reference
    /// * `contract_json` - CONTRACT_902 JSON string
    /// * `db_path` - Path to shader database directory
    ///
    /// # Returns
    ///
    /// Pipeline ID (16-character hex string)
    ///
    /// # Raises
    ///
    /// * `RuntimeError` - If pipeline creation fails
    pub fn create_pipeline_from_contract(
        &mut self,
        py: Python,
        contract_json: String,
        db_path: String,
    ) -> PyResult<String> {
        log::info!("Creating pipeline from CONTRACT_902");

        // Parse JSON
        let contract: serde_json::Value = serde_json::from_str(&contract_json)
            .map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!(
                    "Invalid CONTRACT_902 JSON: {}",
                    e
                ))
            })?;

        // Extract CONTRACT_902 data
        let contract_902 = contract
            .get("902")
            .ok_or_else(|| {
                pyo3::exceptions::PyRuntimeError::new_err(
                    "Missing '902' key in CONTRACT",
                )
            })?;

        // Extract pipeline name from @1
        let pipeline_name = contract_902
            .get("@1")
            .and_then(|v| v.as_str())
            .unwrap_or("unnamed_pipeline");

        log::debug!("Pipeline name: {}", pipeline_name);

        // Generate pipeline ID from contract name
        let pipeline_id = Self::compute_shader_id(pipeline_name.as_bytes());

        // Extract shader stages from @3
        let shader_stages_obj = contract_902.get("@3").ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "Missing '@3' (shader stages) in CONTRACT_902",
            )
        })?;

        // Load shaders from database
        let mut shader_handles = Vec::new();
        let mut stage_types = Vec::new();

        // Process each shader stage - first pass: collect handles
        if let Some(stages) = shader_stages_obj.as_object() {
            for (key, stage_obj) in stages {
                let shader_handle = stage_obj
                    .get("@1")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        pyo3::exceptions::PyRuntimeError::new_err(format!(
                            "Missing shader handle in stage {}",
                            key
                        ))
                    })?
                    .to_string();

                let stage_type = stage_obj
                    .get("@2")
                    .and_then(|v| v.as_str())
                    .unwrap_or("VERTEX_SHADER")
                    .to_string();

                log::debug!(
                    "Stage {}: {} ({})",
                    key,
                    shader_handle,
                    stage_type
                );

                shader_handles.push(shader_handle);
                stage_types.push(stage_type);
            }
        }

        if shader_handles.is_empty() {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "No shader stages found in CONTRACT_902",
            ));
        }

        // Load all shaders first
        let mut shader_ids = Vec::new();
        for handle in &shader_handles {
            let shader_id =
                self.load_shader_from_db(py, handle.clone(), db_path.clone())?;
            shader_ids.push(shader_id);
        }

        log::debug!("Loaded {} shader stages", shader_ids.len());

        // Build stage infos with borrowed shader modules
        // This must all happen in one scope to keep entry_points alive
        {
            let entry_points: Vec<CString> = shader_ids
                .iter()
                .map(|id| {
                    self.shader_cache
                        .get(id)
                        .map(|m| CString::new(m.entry_point.clone()).unwrap())
                        .unwrap_or_else(|| CString::new("main").unwrap())
                })
                .collect();

            let mut stage_infos = Vec::new();
            for (i, (shader_id, stage_type)) in
                shader_ids.iter().zip(stage_types.iter()).enumerate()
            {
                let shader_module = self.shader_cache.get(shader_id).ok_or_else(|| {
                    pyo3::exceptions::PyRuntimeError::new_err(format!(
                        "Failed to retrieve shader module: {}",
                        shader_id
                    ))
                })?;

                // Determine stage flags
                let stage_flags = match stage_type.as_str() {
                    "VERTEX_SHADER" => vk::ShaderStageFlags::VERTEX,
                    "FRAGMENT_SHADER" => vk::ShaderStageFlags::FRAGMENT,
                    "GEOMETRY_SHADER" => vk::ShaderStageFlags::GEOMETRY,
                    "COMPUTE_SHADER" => vk::ShaderStageFlags::COMPUTE,
                    _ => vk::ShaderStageFlags::VERTEX,
                };

                // Create shader stage create info
                let stage_create_info = vk::PipelineShaderStageCreateInfo::default()
                    .stage(stage_flags)
                    .module(shader_module.handle)
                    .name(&entry_points[i]);

                stage_infos.push(stage_create_info);
            }

            log::debug!("Constructed {} shader stage infos", stage_infos.len());

            // Create render pass
            let render_pass = crate::pipeline::create_simple_render_pass(&self.device)
                .map_err(|e| {
                    pyo3::exceptions::PyRuntimeError::new_err(format!(
                        "Failed to create render pass: {}",
                        e
                    ))
                })?;

            // Store render pass in cache
            self.render_pass_cache
                .insert(pipeline_id.clone(), render_pass);

            // Create graphics pipeline
            let _pipeline = GraphicsPipeline::create(
                Arc::new(self.device.clone()),
                &stage_infos,
                render_pass,
                0,
            )
            .map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!(
                    "Failed to create graphics pipeline: {}",
                    e
                ))
            })?;

            log::debug!("Pipeline created and destroyed");
        }

        // For now, we destroy immediately (no persistent storage)
        // In production, we would store the pipeline for later use
        // _pipeline.destroy();

        log::info!(
            "Pipeline created successfully: {} (ID: {})",
            pipeline_name,
            pipeline_id
        );

        Ok(pipeline_id)
    }

    /// Clean up Vulkan resources.
    ///
    /// **Must be called before the context is dropped.**
    ///
    /// Destroys all cached shaders and buffers, the logical device, and the instance.
    /// After calling this, the context cannot be used.
    pub fn cleanup(&mut self) {
        if self.cleaned_up {
            log::warn!("VulkanContext.cleanup() called twice");
            return;
        }

        log::info!("Cleaning up VulkanContext");

        // Destroy cached buffers first
        self.clear_buffer_cache();

        // Destroy cached shaders
        self.clear_cache();

        // Destroy Vulkan objects in reverse order of creation
        unsafe {
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }

        self.cleaned_up = true;
        log::info!("VulkanContext cleanup complete");
    }
}

// Private implementation methods
impl VulkanContext {
    /// Create Vulkan instance with optional validation layers.
    fn create_instance(entry: &Entry, enable_validation: bool) -> Result<Instance, VulkanErrorKind> {
        let app_name = CString::new("HLX Vulkan Runtime").unwrap();
        let engine_name = CString::new("HLX").unwrap();

        let app_info = vk::ApplicationInfo::default()
            .application_name(&app_name)
            .application_version(vk::make_api_version(0, 1, 0, 0))
            .engine_name(&engine_name)
            .engine_version(vk::make_api_version(0, 1, 0, 0))
            .api_version(vk::API_VERSION_1_2);

        // Optionally enable validation layers
        let layer_names: Vec<CString> = if enable_validation {
            log::info!("Enabling Vulkan validation layers");
            vec![CString::new("VK_LAYER_KHRONOS_validation").unwrap()]
        } else {
            vec![]
        };
        let layer_ptrs: Vec<*const i8> = layer_names.iter().map(|l| l.as_ptr()).collect();

        let create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_layer_names(&layer_ptrs);

        let instance = unsafe { entry.create_instance(&create_info, None) }.map_err(|e| {
            VulkanErrorKind::InitializationFailed(format!("vkCreateInstance failed: {:?}", e))
        })?;

        log::debug!("Vulkan instance created");
        Ok(instance)
    }

    /// Select a physical device (GPU) by index.
    fn select_physical_device(
        instance: &Instance,
        device_index: usize,
    ) -> Result<(vk::PhysicalDevice, String, String), VulkanErrorKind> {
        let devices = unsafe { instance.enumerate_physical_devices() }.map_err(|e| {
            VulkanErrorKind::InitializationFailed(format!(
                "enumerate_physical_devices failed: {:?}",
                e
            ))
        })?;

        if devices.is_empty() {
            return Err(VulkanErrorKind::NoSuitableDevice);
        }

        log::info!("Found {} Vulkan device(s)", devices.len());

        // Log all devices for debugging
        for (i, &dev) in devices.iter().enumerate() {
            let props = unsafe { instance.get_physical_device_properties(dev) };
            let name = unsafe {
                std::ffi::CStr::from_ptr(props.device_name.as_ptr())
                    .to_string_lossy()
                    .to_string()
            };
            log::debug!("  Device {}: {}", i, name);
        }

        // Select requested device
        let device = devices
            .get(device_index)
            .copied()
            .ok_or(VulkanErrorKind::NoSuitableDevice)?;

        let props = unsafe { instance.get_physical_device_properties(device) };

        let device_name = unsafe {
            std::ffi::CStr::from_ptr(props.device_name.as_ptr())
                .to_string_lossy()
                .to_string()
        };

        let api_version = format!(
            "{}.{}.{}",
            vk::api_version_major(props.api_version),
            vk::api_version_minor(props.api_version),
            vk::api_version_patch(props.api_version)
        );

        log::info!("Selected device {}: {} (API {})", device_index, device_name, api_version);

        Ok((device, device_name, api_version))
    }

    /// Create logical device with compute queue.
    fn create_logical_device(
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
    ) -> Result<(Device, vk::Queue, u32), VulkanErrorKind> {
        let queue_families =
            unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

        // Find a queue family with compute support
        let compute_family = queue_families
            .iter()
            .enumerate()
            .find(|(_, props)| props.queue_flags.contains(vk::QueueFlags::COMPUTE))
            .map(|(i, _)| i as u32)
            .ok_or_else(|| {
                VulkanErrorKind::InitializationFailed(
                    "No compute queue family found".to_string()
                )
            })?;

        log::debug!("Using queue family {} for compute", compute_family);

        let queue_priorities = [1.0f32];
        let queue_create_info = vk::DeviceQueueCreateInfo::default()
            .queue_family_index(compute_family)
            .queue_priorities(&queue_priorities);

        let device_create_info =
            vk::DeviceCreateInfo::default().queue_create_infos(std::slice::from_ref(&queue_create_info));

        let device = unsafe {
            instance.create_device(physical_device, &device_create_info, None)
        }
        .map_err(|e| {
            VulkanErrorKind::InitializationFailed(format!("vkCreateDevice failed: {:?}", e))
        })?;

        let queue = unsafe { device.get_device_queue(compute_family, 0) };

        log::debug!("Logical device and compute queue created");

        Ok((device, queue, compute_family))
    }

    /// Compute content-addressed shader ID from SPIR-V bytes.
    ///
    /// Uses a fast hash for cache lookup. The actual content-addressing
    /// for HLX memoization uses BLAKE2b on the Python side.
    fn compute_shader_id(spirv_bytes: &[u8]) -> String {
        let mut hasher = DefaultHasher::new();
        spirv_bytes.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }

    /// Compute content-addressed buffer ID from data.
    ///
    /// Uses same hashing scheme as shaders for consistency.
    fn compute_buffer_id<T: AsRef<[u8]>>(data: T) -> String {
        let mut hasher = DefaultHasher::new();
        data.as_ref().hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }

    /// Get buffer from cache (internal use).
    ///
    /// # Arguments
    ///
    /// * `buffer_id` - Buffer ID
    ///
    /// # Returns
    ///
    /// Reference to buffer if found
    pub fn get_buffer(&self, buffer_id: &str) -> Option<&Buffer> {
        self.buffer_cache.get(buffer_id)
    }
}

impl Drop for VulkanContext {
    fn drop(&mut self) {
        if !self.cleaned_up {
            // This is a safety net. Users should call cleanup() explicitly.
            log::warn!(
                "VulkanContext dropped without explicit cleanup(). \
                 Call ctx.cleanup() to avoid resource leaks."
            );
            // Note: We don't call cleanup() here because it may panic
            // if the Vulkan driver is already torn down.
        }
    }
}
