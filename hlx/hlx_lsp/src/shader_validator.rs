//! SPIR-V Shader Validation
//!
//! Validates HLX code against SPIR-V shader contracts to catch mismatches at compile time.
//! This prevents the "Ghost Dispatch" class of segfaults by verifying shader expectations
//! before runtime execution.

use anyhow::{Context, Result, anyhow};
use rspirv::dr::{Loader, Module, Operand};
use spirv::Decoration;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

/// Reflected shader contract extracted from SPIR-V binary
#[derive(Debug, Clone)]
pub struct ShaderContract {
    /// Shader file path
    pub path: PathBuf,
    /// Push constant block (if any)
    pub push_constant: Option<PushConstantBlock>,
    /// Descriptor set bindings
    pub bindings: Vec<DescriptorBinding>,
    /// Entry point name
    pub entry_point: String,
    /// Workgroup size (for compute shaders)
    pub workgroup_size: Option<[u32; 3]>,
}

/// Push constant block specification
#[derive(Debug, Clone)]
pub struct PushConstantBlock {
    /// Total size in bytes
    pub size: u32,
    /// Members (for detailed validation)
    pub members: Vec<PushConstantMember>,
}

#[derive(Debug, Clone)]
pub struct PushConstantMember {
    pub name: String,
    pub offset: u32,
    pub size: u32,
    pub type_name: String,
}

/// Descriptor set binding specification
#[derive(Debug, Clone)]
pub struct DescriptorBinding {
    /// Set number
    pub set: u32,
    /// Binding number
    pub binding: u32,
    /// Descriptor type
    pub descriptor_type: DescriptorType,
    /// Array count (1 for non-arrays)
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DescriptorType {
    StorageBuffer,
    UniformBuffer,
    StorageImage,
    SampledImage,
    Sampler,
    Unknown,
}

/// Contract violation detected during validation
#[derive(Debug, Clone)]
pub enum ContractViolation {
    /// Shader file not found
    ShaderNotFound {
        path: String,
    },
    /// Push constant size mismatch
    PushConstantSizeMismatch {
        expected: u32,
        actual: u32,
    },
    /// Push constants not properly aligned
    MisalignedPushConstants {
        size: u32,
        required_alignment: u32,
    },
    /// Number of bindings doesn't match
    BindingCountMismatch {
        expected: usize,
        actual: usize,
    },
    /// Invalid SPIR-V file
    InvalidSpirv {
        error: String,
    },
}

impl std::fmt::Display for ContractViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContractViolation::ShaderNotFound { path } => {
                write!(f, "Shader not found: {}", path)
            }
            ContractViolation::PushConstantSizeMismatch { expected, actual } => {
                write!(
                    f,
                    "Push constant size mismatch: shader expects {} bytes, got {} bytes",
                    expected, actual
                )
            }
            ContractViolation::MisalignedPushConstants { size, required_alignment } => {
                write!(
                    f,
                    "Push constants not aligned: size {} is not a multiple of {} bytes",
                    size, required_alignment
                )
            }
            ContractViolation::BindingCountMismatch { expected, actual } => {
                write!(
                    f,
                    "Binding count mismatch: shader expects {} bindings, got {}",
                    expected, actual
                )
            }
            ContractViolation::InvalidSpirv { error } => {
                write!(f, "Invalid SPIR-V file: {}", error)
            }
        }
    }
}

impl ShaderContract {
    /// Load and reflect a SPIR-V shader from file
    pub fn from_spirv_file(path: &Path) -> Result<Self> {
        debug!("Loading shader contract from: {}", path.display());

        // Read SPIR-V binary
        let bytes = std::fs::read(path)
            .with_context(|| format!("Failed to read shader: {}", path.display()))?;

        // Parse SPIR-V module
        let mut loader = Loader::new();
        rspirv::binary::parse_bytes(&bytes, &mut loader)
            .with_context(|| "Failed to parse SPIR-V binary")?;

        let module = loader.module();

        // Extract contract information
        let push_constant = Self::extract_push_constants(&module)?;
        let bindings = Self::extract_bindings(&module)?;
        let entry_point = Self::extract_entry_point(&module)?;
        let workgroup_size = Self::extract_workgroup_size(&module)?;

        Ok(Self {
            path: path.to_path_buf(),
            push_constant,
            bindings,
            entry_point,
            workgroup_size,
        })
    }

    /// Extract push constant block from SPIR-V module
    fn extract_push_constants(module: &Module) -> Result<Option<PushConstantBlock>> {
        // Look for OpVariable with PushConstant storage class
        for inst in &module.types_global_values {
            if inst.class.opcode == spirv::Op::Variable {
                if let Some(storage_class) = inst.operands.get(0) {
                    if let Operand::StorageClass(spirv::StorageClass::PushConstant) = storage_class {
                        // Found push constant block
                        // Get the pointer type
                        let type_id = inst.result_type.ok_or_else(|| anyhow!("Push constant has no type"))?;

                        // Find the struct type it points to
                        if let Some(struct_size) = Self::get_struct_size(module, type_id) {
                            debug!("Found push constant block: {} bytes", struct_size);

                            return Ok(Some(PushConstantBlock {
                                size: struct_size,
                                members: Vec::new(), // TODO: Extract member details
                            }));
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    /// Extract descriptor bindings from SPIR-V module
    fn extract_bindings(module: &Module) -> Result<Vec<DescriptorBinding>> {
        let mut bindings = Vec::new();
        let mut binding_info: HashMap<u32, (u32, u32)> = HashMap::new(); // id -> (set, binding)

        // First pass: collect decoration information
        for inst in &module.annotations {
            if inst.class.opcode == spirv::Op::Decorate {
                if let (Some(target_id), Some(decoration)) = (
                    inst.operands.get(0).and_then(|op| {
                        if let Operand::IdRef(id) = op { Some(*id) } else { None }
                    }),
                    inst.operands.get(1).and_then(|op| {
                        if let Operand::Decoration(dec) = op { Some(dec) } else { None }
                    })
                ) {
                    match decoration {
                        Decoration::DescriptorSet => {
                            if let Some(Operand::LiteralBit32(set)) = inst.operands.get(2) {
                                binding_info.entry(target_id).or_insert((0, 0)).0 = *set;
                            }
                        }
                        Decoration::Binding => {
                            if let Some(Operand::LiteralBit32(binding)) = inst.operands.get(2) {
                                binding_info.entry(target_id).or_insert((0, 0)).1 = *binding;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // Second pass: find variables with bindings
        for inst in &module.types_global_values {
            if inst.class.opcode == spirv::Op::Variable {
                if let Some(result_id) = inst.result_id {
                    if let Some(&(set, binding)) = binding_info.get(&result_id) {
                        // Determine descriptor type from storage class
                        let descriptor_type = if let Some(storage_class) = inst.operands.get(0) {
                            match storage_class {
                                Operand::StorageClass(spirv::StorageClass::StorageBuffer) => {
                                    DescriptorType::StorageBuffer
                                }
                                Operand::StorageClass(spirv::StorageClass::Uniform) => {
                                    DescriptorType::UniformBuffer
                                }
                                _ => DescriptorType::Unknown,
                            }
                        } else {
                            DescriptorType::Unknown
                        };

                        bindings.push(DescriptorBinding {
                            set,
                            binding,
                            descriptor_type,
                            count: 1, // TODO: Handle arrays
                        });
                    }
                }
            }
        }

        // Sort by (set, binding) for consistent ordering
        bindings.sort_by_key(|b| (b.set, b.binding));

        debug!("Found {} descriptor bindings", bindings.len());
        Ok(bindings)
    }

    /// Extract entry point name
    fn extract_entry_point(module: &Module) -> Result<String> {
        for inst in &module.entry_points {
            if inst.class.opcode == spirv::Op::EntryPoint {
                if let Some(Operand::LiteralString(name)) = inst.operands.get(1) {
                    return Ok(name.clone());
                }
            }
        }

        Ok("main".to_string()) // Default fallback
    }

    /// Extract workgroup size for compute shaders
    fn extract_workgroup_size(module: &Module) -> Result<Option<[u32; 3]>> {
        // Look for OpExecutionMode with LocalSize
        for inst in &module.execution_modes {
            if inst.class.opcode == spirv::Op::ExecutionMode {
                if let Some(Operand::ExecutionMode(spirv::ExecutionMode::LocalSize)) = inst.operands.get(1) {
                    if let (
                        Some(Operand::LiteralBit32(x)),
                        Some(Operand::LiteralBit32(y)),
                        Some(Operand::LiteralBit32(z)),
                    ) = (inst.operands.get(2), inst.operands.get(3), inst.operands.get(4)) {
                        return Ok(Some([*x, *y, *z]));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Get the size of a struct type
    fn get_struct_size(module: &Module, type_id: u32) -> Option<u32> {
        // Find the type instruction
        for inst in &module.types_global_values {
            if inst.result_id == Some(type_id) {
                // If it's a pointer, follow to the pointee type
                if inst.class.opcode == spirv::Op::TypePointer {
                    if let Some(Operand::IdRef(pointee_id)) = inst.operands.get(1) {
                        return Self::get_struct_size(module, *pointee_id);
                    }
                }

                // If it's a struct, look for size decoration
                if inst.class.opcode == spirv::Op::TypeStruct {
                    // Look for decoration with size
                    for annotation in &module.annotations {
                        if annotation.class.opcode == spirv::Op::Decorate {
                            if let (Some(Operand::IdRef(target)), Some(Operand::Decoration(Decoration::ArrayStride))) =
                                (annotation.operands.get(0), annotation.operands.get(1)) {
                                if *target == type_id {
                                    if let Some(Operand::LiteralBit32(size)) = annotation.operands.get(2) {
                                        return Some(*size);
                                    }
                                }
                            }
                        }
                    }

                    // Fallback: estimate from members
                    // This is a simplified estimation
                    let member_count = inst.operands.len();
                    if member_count > 0 {
                        // Rough estimate: 16 bytes per member (common for vec4/mat4)
                        return Some((member_count * 16) as u32);
                    }
                }
            }
        }

        None
    }

    /// Validate a gpu_dispatch call against this contract
    pub fn validate_dispatch(
        &self,
        binding_count: usize,
        push_constant_size: Option<u32>,
    ) -> Vec<ContractViolation> {
        let mut violations = Vec::new();

        // Check binding count
        if binding_count != self.bindings.len() {
            violations.push(ContractViolation::BindingCountMismatch {
                expected: self.bindings.len(),
                actual: binding_count,
            });
        }

        // Check push constant size
        if let Some(expected_size) = self.push_constant.as_ref().map(|pc| pc.size) {
            if let Some(actual_size) = push_constant_size {
                if actual_size != expected_size {
                    violations.push(ContractViolation::PushConstantSizeMismatch {
                        expected: expected_size,
                        actual: actual_size,
                    });
                }

                // Check alignment (Vulkan requires 4-byte alignment)
                if actual_size % 4 != 0 {
                    violations.push(ContractViolation::MisalignedPushConstants {
                        size: actual_size,
                        required_alignment: 4,
                    });
                }
            } else {
                // Shader expects push constants but none provided
                violations.push(ContractViolation::PushConstantSizeMismatch {
                    expected: expected_size,
                    actual: 0,
                });
            }
        } else if push_constant_size.is_some() {
            // Shader doesn't expect push constants but some were provided
            warn!("Shader doesn't use push constants but {} bytes were provided", push_constant_size.unwrap());
        }

        violations
    }

    /// Generate a human-readable summary of this contract
    pub fn summary(&self) -> String {
        let mut s = format!("Shader: {}\n", self.path.display());
        s.push_str(&format!("Entry Point: {}\n", self.entry_point));

        if let Some(wg) = self.workgroup_size {
            s.push_str(&format!("Workgroup Size: {}x{}x{}\n", wg[0], wg[1], wg[2]));
        }

        if let Some(ref pc) = self.push_constant {
            s.push_str(&format!("Push Constants: {} bytes\n", pc.size));
        } else {
            s.push_str("Push Constants: None\n");
        }

        s.push_str(&format!("Bindings: {}\n", self.bindings.len()));
        for binding in &self.bindings {
            s.push_str(&format!(
                "  [set={}, binding={}] {:?}\n",
                binding.set, binding.binding, binding.descriptor_type
            ));
        }

        s
    }
}

/// Cache of shader contracts
pub struct ShaderContractCache {
    contracts: parking_lot::RwLock<HashMap<PathBuf, ShaderContract>>,
}

impl ShaderContractCache {
    pub fn new() -> Self {
        Self {
            contracts: parking_lot::RwLock::new(HashMap::new()),
        }
    }

    /// Get or load a shader contract
    pub fn get_or_load(&self, path: &Path) -> Result<ShaderContract> {
        // Check cache first
        {
            let cache = self.contracts.read();
            if let Some(contract) = cache.get(path) {
                return Ok(contract.clone());
            }
        }

        // Load and cache
        let contract = ShaderContract::from_spirv_file(path)?;

        {
            let mut cache = self.contracts.write();
            cache.insert(path.to_path_buf(), contract.clone());
        }

        Ok(contract)
    }

    /// Invalidate cached contract (e.g., when shader file changes)
    pub fn invalidate(&self, path: &Path) {
        let mut cache = self.contracts.write();
        cache.remove(path);
    }

    /// Clear all cached contracts
    pub fn clear(&self) {
        let mut cache = self.contracts.write();
        cache.clear();
    }
}

impl Default for ShaderContractCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shader_contract_cache() {
        let cache = ShaderContractCache::new();

        // Cache should be empty initially
        assert_eq!(cache.contracts.read().len(), 0);

        // Test invalidation and clearing
        cache.clear();
        assert_eq!(cache.contracts.read().len(), 0);
    }

    #[test]
    fn test_contract_violation_display() {
        let violation = ContractViolation::PushConstantSizeMismatch {
            expected: 128,
            actual: 12,
        };

        let msg = format!("{}", violation);
        assert!(msg.contains("128"));
        assert!(msg.contains("12"));
    }
}
