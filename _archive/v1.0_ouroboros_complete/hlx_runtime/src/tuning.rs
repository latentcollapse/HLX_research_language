//! Hardware-Specific Tuning
//!
//! Provides an abstraction layer for vendor-specific optimizations (NVIDIA/AMD/Intel).
//! This allows the runtime to swap SPIR-V kernels and tune workgroup sizes dynamically.

use hlx_core::{Instruction};

/// GPU Vendor IDs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vendor {
    Nvidia,
    Amd,
    Intel,
    Generic,
}

/// Trait for hardware-specific tuning parameters
pub trait BackendTuning: Send + Sync {
    /// Get the vendor ID
    fn vendor(&self) -> Vendor;

    /// Get the optimal tile size for tiled matrix operations (e.g. 16 for NVIDIA, might be different for AMD)
    fn optimal_tile_size(&self) -> u32;

    /// Get the optimal workgroup size for a specific instruction
    fn optimal_workgroup_size(&self, instruction: &Instruction) -> (u32, u32, u32);

    /// Get the optimized SPIR-V binary for a specific instruction
    /// Returns None if the generic kernel should be used.
    fn get_optimized_spv(&self, instruction_name: &str) -> Option<&'static [u8]>;
}

/// Default tuning implementation for generic hardware
pub struct GenericTuning;

impl BackendTuning for GenericTuning {
    fn vendor(&self) -> Vendor {
        Vendor::Generic
    }

    fn optimal_tile_size(&self) -> u32 {
        16 // Default standard
    }

    fn optimal_workgroup_size(&self, _instruction: &Instruction) -> (u32, u32, u32) {
        (16, 16, 1) // Default 2D workgroup
    }

    fn get_optimized_spv(&self, _instruction_name: &str) -> Option<&'static [u8]> {
        None // Use standard kernels
    }
}

/// Helper to detect vendor from Vulkan properties (placeholder)
pub fn detect_vendor(vendor_id: u32) -> Vendor {
    match vendor_id {
        0x10DE => Vendor::Nvidia,
        0x1002 | 0x1022 => Vendor::Amd,
        0x8086 => Vendor::Intel,
        _ => Vendor::Generic,
    }
}
