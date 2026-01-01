//! Graphics pipeline management
//!
//! Handles creation and management of Vulkan graphics pipelines
//! from HLX CONTRACT_902 JSON definitions.

use ash::{vk, Device};
use std::sync::Arc;
use crate::error::VulkanErrorKind;

/// Wrapper around VkGraphicsPipeline and VkPipelineLayout
///
/// Manages both the pipeline and its layout together since they
/// are closely coupled in Vulkan.
pub struct GraphicsPipeline {
    /// The compiled graphics pipeline
    pub pipeline: vk::Pipeline,

    /// The pipeline layout (defines descriptor sets, push constants)
    pub layout: vk::PipelineLayout,

    /// Reference to device (needed for cleanup)
    device: Arc<Device>,
}

impl GraphicsPipeline {
    /// Create a graphics pipeline from shader modules and configuration
    ///
    /// # Arguments
    ///
    /// * `device` - Logical device
    /// * `shader_stages` - Compiled shader stage create infos
    /// * `render_pass` - Target render pass
    /// * `subpass` - Subpass index (usually 0)
    ///
    /// # Returns
    ///
    /// Compiled GraphicsPipeline ready for rendering
    pub fn create(
        device: Arc<Device>,
        shader_stages: &[vk::PipelineShaderStageCreateInfo],
        render_pass: vk::RenderPass,
        subpass: u32,
    ) -> Result<Self, VulkanErrorKind> {
        // Create a simple pipeline layout (no descriptor sets for now)
        let layout = unsafe {
            device.create_pipeline_layout(
                &vk::PipelineLayoutCreateInfo::default(),
                None,
            )
        }
        .map_err(|e| {
            VulkanErrorKind::PipelineCreationFailed(format!(
                "Failed to create pipeline layout: {:?}",
                e
            ))
        })?;

        // Define vertex input state - fixed for demo
        // vec3 position (12 bytes) + vec3 normal (12 bytes) + vec3 color (12 bytes) = 36 bytes
        let vertex_binding = vk::VertexInputBindingDescription {
            binding: 0,
            stride: 36,
            input_rate: vk::VertexInputRate::VERTEX,
        };

        let vertex_attributes = [
            vk::VertexInputAttributeDescription {
                location: 0,
                binding: 0,
                format: vk::Format::R32G32B32_SFLOAT, // position
                offset: 0,
            },
            vk::VertexInputAttributeDescription {
                location: 1,
                binding: 0,
                format: vk::Format::R32G32B32_SFLOAT, // normal
                offset: 12,
            },
            vk::VertexInputAttributeDescription {
                location: 2,
                binding: 0,
                format: vk::Format::R32G32B32_SFLOAT, // color
                offset: 24,
            },
        ];

        let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(std::slice::from_ref(&vertex_binding))
            .vertex_attribute_descriptions(&vertex_attributes);

        // Input assembly state - triangle list
        let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);

        // Rasterization state - CCW culling, fill mode
        let rasterization_state = vk::PipelineRasterizationStateCreateInfo::default()
            .polygon_mode(vk::PolygonMode::FILL)
            .cull_mode(vk::CullModeFlags::BACK)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .depth_bias_enable(false)
            .line_width(1.0);

        // Multisample state - no MSAA
        let multisample_state = vk::PipelineMultisampleStateCreateInfo::default()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        // Depth/stencil state - disabled for simple rendering
        let depth_stencil_state = vk::PipelineDepthStencilStateCreateInfo::default()
            .depth_test_enable(false)
            .depth_write_enable(false)
            .depth_bounds_test_enable(false)
            .stencil_test_enable(false);

        // Color blend state - no blending
        let color_blend_attachment = vk::PipelineColorBlendAttachmentState {
            blend_enable: vk::FALSE,
            color_write_mask: vk::ColorComponentFlags::RGBA,
            ..Default::default()
        };

        let color_blend_state = vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op_enable(false)
            .attachments(std::slice::from_ref(&color_blend_attachment));

        // Viewport state - dynamic viewport/scissor
        let viewport_state = vk::PipelineViewportStateCreateInfo::default()
            .viewport_count(1)
            .scissor_count(1);

        // Dynamic state - viewport and scissor are dynamic
        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state = vk::PipelineDynamicStateCreateInfo::default()
            .dynamic_states(&dynamic_states);

        // Create the graphics pipeline
        let pipeline_create_info = vk::GraphicsPipelineCreateInfo::default()
            .stages(shader_stages)
            .vertex_input_state(&vertex_input_state)
            .input_assembly_state(&input_assembly_state)
            .viewport_state(&viewport_state)
            .rasterization_state(&rasterization_state)
            .multisample_state(&multisample_state)
            .depth_stencil_state(&depth_stencil_state)
            .color_blend_state(&color_blend_state)
            .dynamic_state(&dynamic_state)
            .layout(layout)
            .render_pass(render_pass)
            .subpass(subpass);

        let pipeline = unsafe {
            device.create_graphics_pipelines(
                vk::PipelineCache::null(),
                std::slice::from_ref(&pipeline_create_info),
                None,
            )
        }
        .map_err(|e| {
            VulkanErrorKind::PipelineCreationFailed(format!(
                "Failed to create graphics pipeline: {:?}",
                e
            ))
        })?
        .into_iter()
        .next()
        .ok_or_else(|| {
            VulkanErrorKind::PipelineCreationFailed(
                "Pipeline creation returned empty result".to_string(),
            )
        })?;

        log::info!("Graphics pipeline created successfully");

        Ok(Self {
            pipeline,
            layout,
            device,
        })
    }

    /// Destroy the pipeline and its layout
    pub fn destroy(self) {
        log::debug!("Destroying graphics pipeline");
        unsafe {
            self.device.destroy_pipeline(self.pipeline, None);
            self.device.destroy_pipeline_layout(self.layout, None);
        }
    }

    /// Get the pipeline handle
    pub fn handle(&self) -> vk::Pipeline {
        self.pipeline
    }

    /// Get the pipeline layout handle
    pub fn layout_handle(&self) -> vk::PipelineLayout {
        self.layout
    }
}

/// Create a simple render pass for offscreen rendering
///
/// Minimal render pass with just color attachment (RGBA8)
pub fn create_simple_render_pass(
    device: &Device,
) -> Result<vk::RenderPass, VulkanErrorKind> {
    // Color attachment only (minimal render pass)
    let color_attachment = vk::AttachmentDescription {
        flags: vk::AttachmentDescriptionFlags::empty(),
        format: vk::Format::R8G8B8A8_SRGB,
        samples: vk::SampleCountFlags::TYPE_1,
        load_op: vk::AttachmentLoadOp::CLEAR,
        store_op: vk::AttachmentStoreOp::STORE,
        stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
        stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
        initial_layout: vk::ImageLayout::UNDEFINED,
        final_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
    };

    let attachments = [color_attachment];

    // Color attachment reference
    let color_attachment_ref = vk::AttachmentReference {
        attachment: 0,
        layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
    };

    // Subpass with just color attachment (no depth for simplicity)
    let subpass_desc = vk::SubpassDescription::default()
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .color_attachments(std::slice::from_ref(&color_attachment_ref));

    let render_pass_create_info = vk::RenderPassCreateInfo::default()
        .attachments(&attachments)
        .subpasses(std::slice::from_ref(&subpass_desc));

    unsafe { device.create_render_pass(&render_pass_create_info, None) }
        .map_err(|e| {
            VulkanErrorKind::PipelineCreationFailed(format!(
                "Failed to create render pass: {:?}",
                e
            ))
        })
}
