//! Runtime Configuration
//!
//! Controls determinism settings and backend selection.

use serde::{Deserialize, Serialize};

/// Runtime configuration for deterministic execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    /// Global RNG seed (for any stochastic operations)
    pub seed: u64,

    /// Force deterministic mode (fixed workgroups, no dynamic alloc)
    pub deterministic: bool,

    /// Preferred backend
    pub backend: BackendType,

    /// Maximum registers to allocate
    pub max_registers: u32,

    /// Enable debug tracing
    pub debug: bool,

    /// GPU-specific settings
    pub gpu: GpuConfig,

    /// Optional input to pass to main() function
    pub main_input: Option<String>,
}

/// GPU-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuConfig {
    /// Fixed workgroup size for determinism
    pub workgroup_size: [u32; 3],
    
    /// Tile size for matrix operations
    pub tile_size: u32,
    
    /// Maximum buffer size (bytes)
    pub max_buffer_size: usize,
    
    /// Device index (for multi-GPU systems)
    pub device_index: u32,
}

/// Backend type selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackendType {
    /// CPU backend (always available)
    Cpu,

    /// Vulkan GPU backend
    Vulkan,

    /// Automatic selection (prefer GPU if available)
    Auto,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            seed: 0,
            deterministic: true,
            backend: BackendType::Auto,
            max_registers: 4096,
            debug: false,
            gpu: GpuConfig::default(),
            main_input: None,
        }
    }
}

impl Default for GpuConfig {
    fn default() -> Self {
        Self {
            workgroup_size: [256, 1, 1],
            tile_size: 16,
            max_buffer_size: 1024 * 1024 * 1024, // 1 GB
            device_index: 0,
        }
    }
}

impl RuntimeConfig {
    /// Create a strictly deterministic configuration
    pub fn deterministic() -> Self {
        Self {
            deterministic: true,
            ..Default::default()
        }
    }
    
    /// Create a debug configuration with tracing
    pub fn debug() -> Self {
        Self {
            debug: true,
            ..Default::default()
        }
    }
    
    /// Use CPU backend only
    pub fn cpu_only() -> Self {
        Self {
            backend: BackendType::Cpu,
            ..Default::default()
        }
    }
    
    /// Use Vulkan backend (fails if unavailable)
    pub fn vulkan() -> Self {
        Self {
            backend: BackendType::Vulkan,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = RuntimeConfig::default();
        assert!(config.deterministic);
        assert_eq!(config.seed, 0);
    }

    #[test]
    fn test_config_serialization() {
        let config = RuntimeConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let restored: RuntimeConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.seed, restored.seed);
    }
}
