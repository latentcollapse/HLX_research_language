//! Backends module
//!
//! Contains CPU and Vulkan backend implementations.

#[cfg(feature = "cpu")]
pub mod cpu;

#[cfg(feature = "vulkan")]
pub mod vulkan;
