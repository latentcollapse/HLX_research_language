//! HLX Code Generation Framework
//!
//! Synthetic code generation for:
//! - Safety-critical boilerplate (aerospace, medical, automotive)
//! - Training data for AI models (LoRA, fine-tuning)
//! - Enterprise templates (REST APIs, compliance patterns)
//!
//! ## Primary Use Case: Safety-Critical Code Generation
//!
//! Generate massive amounts of compliant, certified-ready code for:
//! - Aerospace (DO-178C, DO-254)
//! - Medical (IEC 62304, ISO 13485)
//! - Automotive (ISO 26262, MISRA-C, AUTOSAR)
//! - Nuclear (NQA-1)
//! - Financial (SOX, PCI-DSS)
//!
//! ## Bonus: Training Data Generation
//!
//! Same infrastructure can generate LoRA training datasets,
//! security vulnerability datasets, code review pairs, etc.

pub mod core;
pub mod domains;

pub use core::{CodeGenerator, GeneratorConfig, GeneratedCodeset, GeneratedCode};
pub use domains::aerospace::{AerospaceGenerator, AerospaceConfig, SafetyLevel, Standard, ComponentType};
