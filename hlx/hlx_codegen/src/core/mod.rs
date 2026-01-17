//! Core code generation framework

pub mod generator;
pub mod quality;
pub mod diversity;

pub use generator::{CodeGenerator, GeneratorConfig, GeneratedCodeset, GeneratedCode, CodeMetadata};
pub use quality::{QualityValidator, QualityScore};
pub use diversity::{DiversityEngine, DiversityScore};
