//! Runtime Capability Validation
//!
//! Validates HLX code against runtime capabilities to catch missing builtins
//! and feature mismatches before execution.

use hlx_core::RuntimeCapabilities;
use std::sync::Arc;
use parking_lot::RwLock;
use std::process::Command;
use tracing::{debug, warn};

/// Cache for runtime capabilities
pub struct CapabilityValidator {
    capabilities: Arc<RwLock<Option<RuntimeCapabilities>>>,
}

impl CapabilityValidator {
    pub fn new() -> Self {
        Self {
            capabilities: Arc::new(RwLock::new(None)),
        }
    }

    /// Load capabilities from runtime
    pub fn load_capabilities(&self) -> Result<(), String> {
        debug!("Loading runtime capabilities");

        // Try to execute `hlx capabilities` command
        let output = Command::new("hlx")
            .args(&["capabilities"])
            .output()
            .map_err(|e| format!("Failed to execute hlx capabilities: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("hlx capabilities failed: {}", stderr));
        }

        let json = String::from_utf8_lossy(&output.stdout);
        let caps = RuntimeCapabilities::from_json(&json)
            .map_err(|e| format!("Failed to parse capabilities: {}", e))?;

        debug!("Loaded {} builtins from runtime", caps.builtins.len());

        *self.capabilities.write() = Some(caps);
        Ok(())
    }

    /// Check if a builtin is available
    pub fn validate_builtin(&self, name: &str) -> BuiltinValidation {
        let caps = self.capabilities.read();

        if let Some(ref capabilities) = *caps {
            if let Some(builtin) = capabilities.get_builtin(name) {
                // Check if required features are available
                for required_feature in &builtin.required_features {
                    if !capabilities.has_feature(required_feature) {
                        return BuiltinValidation::MissingFeature {
                            builtin: name.to_string(),
                            required_feature: required_feature.clone(),
                            available_features: capabilities.features.clone(),
                        };
                    }
                }

                return BuiltinValidation::Available;
            } else {
                return BuiltinValidation::NotFound {
                    builtin: name.to_string(),
                    suggestion: self.find_similar_builtin(name, capabilities),
                };
            }
        }

        // Capabilities not loaded - assume available with warning
        warn!("Capabilities not loaded, cannot validate builtin: {}", name);
        BuiltinValidation::Unknown
    }

    /// Find similar builtin names (for suggestions)
    fn find_similar_builtin(&self, name: &str, caps: &RuntimeCapabilities) -> Option<String> {
        // Simple similarity heuristic: find builtins with similar prefix
        let name_lower = name.to_lowercase();

        for builtin in &caps.builtins {
            let builtin_lower = builtin.name.to_lowercase();

            // Check if names share a prefix
            if builtin_lower.starts_with(&name_lower[..name_lower.len().min(3)])
                || name_lower.starts_with(&builtin_lower[..builtin_lower.len().min(3)])
            {
                return Some(builtin.name.clone());
            }
        }

        None
    }

    /// Get builtin signature for documentation
    pub fn get_builtin_signature(&self, name: &str) -> Option<String> {
        let caps = self.capabilities.read();

        if let Some(ref capabilities) = *caps {
            return capabilities.get_builtin(name).map(|b| b.signature.clone());
        }

        None
    }

    /// Get builtin description
    pub fn get_builtin_description(&self, name: &str) -> Option<String> {
        let caps = self.capabilities.read();

        if let Some(ref capabilities) = *caps {
            return capabilities.get_builtin(name).map(|b| b.description.clone());
        }

        None
    }

    /// Check if capabilities are loaded
    pub fn is_loaded(&self) -> bool {
        self.capabilities.read().is_some()
    }
}

impl Default for CapabilityValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of builtin validation
#[derive(Debug, Clone)]
pub enum BuiltinValidation {
    /// Builtin is available
    Available,
    /// Builtin not found in runtime
    NotFound {
        builtin: String,
        suggestion: Option<String>,
    },
    /// Builtin exists but requires missing feature
    MissingFeature {
        builtin: String,
        required_feature: String,
        available_features: Vec<String>,
    },
    /// Capabilities not loaded (cannot validate)
    Unknown,
}

impl BuiltinValidation {
    pub fn is_available(&self) -> bool {
        matches!(self, BuiltinValidation::Available)
    }

    pub fn to_diagnostic_message(&self) -> Option<String> {
        match self {
            BuiltinValidation::Available => None,
            BuiltinValidation::NotFound { builtin, suggestion } => {
                let mut msg = format!("Builtin '{}' not found in runtime", builtin);
                if let Some(sugg) = suggestion {
                    msg.push_str(&format!("\nDid you mean '{}'?", sugg));
                }
                Some(msg)
            }
            BuiltinValidation::MissingFeature {
                builtin,
                required_feature,
                available_features,
            } => Some(format!(
                "Builtin '{}' requires feature '{}'\nAvailable features: [{}]\nRebuild runtime with: cargo build --features {}",
                builtin,
                required_feature,
                available_features.join(", "),
                required_feature
            )),
            BuiltinValidation::Unknown => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_validator() {
        let validator = CapabilityValidator::new();
        assert!(!validator.is_loaded());

        // Without loaded capabilities, should return Unknown
        let result = validator.validate_builtin("print");
        assert!(matches!(result, BuiltinValidation::Unknown));
    }
}
