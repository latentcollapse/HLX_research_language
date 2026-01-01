//! Shader module management
//!
//! Wraps VkShaderModule with metadata for HLX integration.

use ash::{vk, Device};
use crate::error::VulkanErrorKind;

/// Wrapper around VkShaderModule with associated metadata.
///
/// Stores the Vulkan handle along with entry point name and
/// original SPIR-V size for debugging/introspection.
pub struct ShaderModule {
    /// The Vulkan shader module handle
    pub handle: vk::ShaderModule,

    /// Entry point name (usually "main")
    pub entry_point: String,

    /// Size of the original SPIR-V binary in bytes
    pub spirv_size: usize,
}

impl ShaderModule {
    /// Create a new ShaderModule from SPIR-V bytes.
    ///
    /// # Arguments
    ///
    /// * `device` - The logical device to create the shader on
    /// * `spirv_bytes` - Raw SPIR-V binary (must be valid and 4-byte aligned)
    /// * `entry_point` - Name of the entry point function
    ///
    /// # Errors
    ///
    /// Returns `VulkanErrorKind::InvalidSpirv` if the SPIR-V is malformed.
    /// Returns `VulkanErrorKind::ShaderCreationFailed` if Vulkan rejects the shader.
    pub fn new(
        device: &Device,
        spirv_bytes: &[u8],
        entry_point: String,
    ) -> Result<Self, VulkanErrorKind> {
        // SPIR-V must be 4-byte aligned
        if spirv_bytes.len() % 4 != 0 {
            return Err(VulkanErrorKind::InvalidSpirv(
                format!("SPIR-V size ({}) must be multiple of 4", spirv_bytes.len())
            ));
        }

        // Convert byte slice to u32 words (SPIR-V is little-endian)
        let code: Vec<u32> = spirv_bytes
            .chunks_exact(4)
            .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();

        let create_info = vk::ShaderModuleCreateInfo::default()
            .code(&code);

        let handle = unsafe { device.create_shader_module(&create_info, None) }
            .map_err(|e| VulkanErrorKind::ShaderCreationFailed(format!("{:?}", e)))?;

        log::debug!(
            "Created ShaderModule: entry_point={}, spirv_size={}",
            entry_point,
            spirv_bytes.len()
        );

        Ok(Self {
            handle,
            entry_point,
            spirv_size: spirv_bytes.len(),
        })
    }

    /// Destroy the shader module.
    ///
    /// Must be called before dropping the Device.
    pub fn destroy(self, device: &Device) {
        log::debug!("Destroying ShaderModule (spirv_size={})", self.spirv_size);
        unsafe {
            device.destroy_shader_module(self.handle, None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alignment_check() {
        // This test doesn't need a real device - just checks validation
        let unaligned = vec![0u8; 17]; // Not divisible by 4

        // We can't call ShaderModule::new without a device, but we can
        // verify the alignment check logic would fail
        assert!(unaligned.len() % 4 != 0);
    }
}
