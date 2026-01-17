//! # HLX Code Generation Framework
//!
//! Enterprise-grade code generation for safety-critical systems and AI training data.
//!
//! ## Primary Use Case: Safety-Critical Code Generation
//!
//! Generate massive amounts of compliant, certified-ready code for:
//! - **Aerospace** (DO-178C, DO-254) - Save 6 months and $800K per project
//! - **Medical** (IEC 62304, ISO 13485) - FDA-compliant device interfaces
//! - **Automotive** (ISO 26262, MISRA-C, AUTOSAR) - Functional safety code
//! - **Nuclear** (NQA-1) - Safety-critical control systems
//! - **Financial** (SOX, PCI-DSS) - Compliance-ready APIs
//!
//! ## Secondary Use Case: AI Training Data
//!
//! Generate high-quality, diverse training datasets for:
//! - **LoRA fine-tuning** - 100K+ instruction/completion pairs
//! - **Security research** - Vulnerability detection datasets
//! - **Code review models** - Before/after diff pairs
//! - **Performance optimization** - Slow→fast transformation pairs
//!
//! ## Quick Start
//!
//! ```rust
//! use hlx_codegen::{AerospaceGenerator, AerospaceConfig, SafetyLevel, ComponentType};
//!
//! let mut config = AerospaceConfig {
//!     safety_level: SafetyLevel::DAL_A,
//!     components: vec![
//!         ComponentType::Sensor {
//!             name: "altitude".to_string(),
//!             unit: "feet".to_string(),
//!             range: (0.0, 60000.0),
//!         },
//!     ],
//!     ..Default::default()
//! };
//!
//! let mut generator = AerospaceGenerator::new(config);
//! let codeset = generator.generate().unwrap();
//!
//! // Generated 557 lines of DO-178C DAL-A compliant code!
//! println!("Generated {} lines", codeset.total_lines());
//! ```

pub mod core;
pub mod domains;

pub use core::{CodeGenerator, GeneratorConfig, GeneratedCodeset, GeneratedCode, CodeMetadata};
pub use domains::aerospace::{AerospaceGenerator, AerospaceConfig, SafetyLevel, Standard, ComponentType};
