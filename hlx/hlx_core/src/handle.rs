//! Strongly-Typed Handle System with Typestate Pattern
//!
//! Prevents handle misuse at compile-time by encoding resource type,
//! access pattern, and memory location in the type system.

use std::marker::PhantomData;
use serde::{Deserialize, Serialize};

/// Resource types
pub struct Tensor;
pub struct Window;
pub struct Buffer;
pub struct Shader;
pub struct Image;

/// Access patterns
pub struct Read;
pub struct Write;
pub struct ReadWrite;

/// Memory locations
pub struct CPU;
pub struct GPU;
pub struct Shared;  // CPU-accessible GPU memory

/// Strongly-typed handle with typestates
///
/// The type parameters encode:
/// - `Resource`: What kind of resource (Tensor, Window, etc.)
/// - `Access`: How it can be accessed (Read, Write, ReadWrite)
/// - `Location`: Where it lives (CPU, GPU, Shared)
///
/// This prevents errors like:
/// - Passing a Window handle to gpu_dispatch (expects Tensor)
/// - Reading GPU-only tensor without staging to CPU
/// - Writing to read-only resources
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Handle<Resource, Access, Location> {
    id: u64,
    _phantom: PhantomData<(Resource, Access, Location)>,
}

impl<R, A, L> Handle<R, A, L> {
    /// Get the raw handle ID (for FFI boundaries)
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Unsafe constructor (only for FFI boundaries and runtime internals)
    ///
    /// # Safety
    /// Caller must ensure the handle ID is valid and the type parameters
    /// accurately reflect the handle's capabilities.
    pub unsafe fn from_raw(id: u64) -> Self {
        Self {
            id,
            _phantom: PhantomData,
        }
    }
}

// Type aliases for common handle types
pub type TensorHandle<A, L> = Handle<Tensor, A, L>;
pub type WindowHandle = Handle<Window, ReadWrite, CPU>;
pub type BufferHandle<A, L> = Handle<Buffer, A, L>;
pub type ShaderHandle = Handle<Shader, Read, GPU>;
pub type ImageHandle<A, L> = Handle<Image, A, L>;

// Common tensor handle types
pub type GpuTensorWrite = Handle<Tensor, Write, GPU>;
pub type GpuTensorRead = Handle<Tensor, Read, GPU>;
pub type GpuTensorRW = Handle<Tensor, ReadWrite, GPU>;
pub type CpuTensorWrite = Handle<Tensor, Write, CPU>;
pub type CpuTensorRead = Handle<Tensor, Read, CPU>;
pub type SharedTensorRead = Handle<Tensor, Read, Shared>;

/// Capability transitions for Write handles
impl<R, L> Handle<R, Write, L> {
    /// Finalize writes and transition to readable
    ///
    /// This models the "seal" operation where you finish writing
    /// and make the resource available for reading.
    pub fn seal(self) -> Handle<R, Read, L> {
        unsafe { Handle::from_raw(self.id) }
    }

    /// Upgrade to read-write access
    pub fn to_read_write(self) -> Handle<R, ReadWrite, L> {
        unsafe { Handle::from_raw(self.id) }
    }
}

/// Capability transitions for Read handles
impl<R, L> Handle<R, Read, L> {
    /// Downgrade from read-write (lossy)
    pub fn from_read_write(handle: Handle<R, ReadWrite, L>) -> Self {
        unsafe { Handle::from_raw(handle.id) }
    }
}

/// Memory location transitions for GPU tensors
impl<A> Handle<Tensor, A, GPU> {
    /// Stage tensor from GPU to CPU-accessible memory
    ///
    /// This is required before you can read tensor data on the CPU.
    /// The resulting handle is in Shared memory (CPU-accessible).
    pub fn stage_to_cpu(self) -> Handle<Tensor, Read, Shared> {
        // In a real implementation, this would call a runtime function
        // to perform the GPU->CPU transfer
        unsafe { Handle::from_raw(self.id) }
    }
}

/// Memory location transitions for CPU tensors
impl<A> Handle<Tensor, A, CPU> {
    /// Upload tensor from CPU to GPU
    ///
    /// This copies the tensor data to GPU memory.
    pub fn upload_to_gpu(self) -> Handle<Tensor, A, GPU> {
        unsafe { Handle::from_raw(self.id) }
    }
}

/// Operations on shared (CPU-accessible) tensor handles
impl Handle<Tensor, Read, Shared> {
    /// Check if tensor can be read on CPU
    ///
    /// Always returns true for Shared memory tensors
    pub fn is_cpu_readable(&self) -> bool {
        true
    }
}

// Serialization support for handles (stores only the ID)
impl<R, A, L> Serialize for Handle<R, A, L> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.id.serialize(serializer)
    }
}

impl<'de, R, A, L> Deserialize<'de> for Handle<R, A, L> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let id = u64::deserialize(deserializer)?;
        Ok(unsafe { Self::from_raw(id) })
    }
}

/// Handle type information for runtime checks
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceType {
    Tensor,
    Window,
    Buffer,
    Shader,
    Image,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AccessPattern {
    Read,
    Write,
    ReadWrite,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryLocation {
    CPU,
    GPU,
    Shared,
}

/// Runtime handle metadata for validation
///
/// Used when type information is erased (e.g., in Value::Handle)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandleMetadata {
    pub id: u64,
    pub resource: ResourceType,
    pub access: AccessPattern,
    pub location: MemoryLocation,
}

impl HandleMetadata {
    pub fn new(id: u64, resource: ResourceType, access: AccessPattern, location: MemoryLocation) -> Self {
        Self {
            id,
            resource,
            access,
            location,
        }
    }

    /// Check if this handle is compatible with GPU dispatch
    pub fn is_gpu_compatible(&self) -> bool {
        matches!(self.location, MemoryLocation::GPU | MemoryLocation::Shared)
            && matches!(self.resource, ResourceType::Tensor | ResourceType::Buffer)
    }

    /// Check if this handle is CPU-readable
    pub fn is_cpu_readable(&self) -> bool {
        matches!(self.location, MemoryLocation::CPU | MemoryLocation::Shared)
            && matches!(self.access, AccessPattern::Read | AccessPattern::ReadWrite)
    }

    /// Check if this handle is writable
    pub fn is_writable(&self) -> bool {
        matches!(self.access, AccessPattern::Write | AccessPattern::ReadWrite)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_creation() {
        let handle: GpuTensorWrite = unsafe { Handle::from_raw(42) };
        assert_eq!(handle.id(), 42);
    }

    #[test]
    fn test_seal_transition() {
        let write_handle: GpuTensorWrite = unsafe { Handle::from_raw(42) };
        let read_handle: GpuTensorRead = write_handle.seal();
        assert_eq!(read_handle.id(), 42);
    }

    #[test]
    fn test_stage_to_cpu() {
        let gpu_handle: GpuTensorRead = unsafe { Handle::from_raw(42) };
        let shared_handle: SharedTensorRead = gpu_handle.stage_to_cpu();
        assert_eq!(shared_handle.id(), 42);
        assert!(shared_handle.is_cpu_readable());
    }

    #[test]
    fn test_upload_to_gpu() {
        let cpu_handle: CpuTensorWrite = unsafe { Handle::from_raw(42) };
        let gpu_handle: GpuTensorWrite = cpu_handle.upload_to_gpu();
        assert_eq!(gpu_handle.id(), 42);
    }

    #[test]
    fn test_handle_metadata() {
        let meta = HandleMetadata::new(
            42,
            ResourceType::Tensor,
            AccessPattern::Read,
            MemoryLocation::GPU,
        );

        assert!(meta.is_gpu_compatible());
        assert!(!meta.is_cpu_readable());
        assert!(!meta.is_writable());
    }

    #[test]
    fn test_shared_tensor_cpu_readable() {
        let meta = HandleMetadata::new(
            42,
            ResourceType::Tensor,
            AccessPattern::Read,
            MemoryLocation::Shared,
        );

        assert!(meta.is_gpu_compatible());
        assert!(meta.is_cpu_readable());
    }
}
