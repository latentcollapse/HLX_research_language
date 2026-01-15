//! HLX Compiler Core
//!
//! Provides parsing, lowering, and lifting services for the HLX ecosystem.

pub mod ast;
pub mod parser;
pub mod emitter;
pub mod lower;
pub mod hlxa;
pub mod runic;
pub mod lift;
pub mod module_resolver;
pub mod substrate;
pub mod substrate_inference;

pub use hlxa::{HlxaParser, HlxaEmitter};
pub use runic::{RunicParser, RunicEmitter};
pub use emitter::Emitter;
pub use lower::lower_to_crate;
pub use lift::lift_from_crate;
pub use module_resolver::ModuleResolver;
pub use substrate::{Substrate, SwarmConfig, SwarmSize, SubstrateInfo, OperationHints};
pub use substrate_inference::SubstrateInference;