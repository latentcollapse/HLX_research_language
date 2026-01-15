//! Runtime Capability Schema
//!
//! Defines the capabilities of the HLX runtime, including available builtins,
//! platform features, and version information. This schema is emitted by the
//! runtime and consumed by the LSP for validation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Complete runtime capability schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeCapabilities {
    /// Runtime version (e.g., "0.5.0")
    pub version: String,
    /// Build timestamp
    pub build_timestamp: String,
    /// Target platform (e.g., "linux", "macos", "windows")
    pub platform: String,
    /// Architecture (e.g., "x86_64", "aarch64")
    pub arch: String,
    /// Enabled features
    pub features: Vec<String>,
    /// Available builtins
    pub builtins: Vec<BuiltinSpec>,
    /// Backend capabilities
    pub backends: Vec<BackendCapability>,
}

/// Builtin function specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuiltinSpec {
    /// Function name (e.g., "alloc_tensor", "gpu_dispatch")
    pub name: String,
    /// Function signature
    pub signature: String,
    /// Brief description
    pub description: String,
    /// Minimum runtime version required
    pub min_version: Option<String>,
    /// Required features (e.g., ["vulkan"], ["cuda"])
    pub required_features: Vec<String>,
    /// Supported platforms (empty = all platforms)
    pub platforms: Vec<String>,
    /// Parameter specifications
    pub parameters: Vec<ParameterSpec>,
    /// Return type
    pub return_type: String,
    /// Whether this is unsafe/experimental
    pub stability: StabilityLevel,
}

/// Parameter specification for a builtin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterSpec {
    pub name: String,
    pub param_type: String,
    pub description: String,
    pub optional: bool,
}

/// Stability level of a builtin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StabilityLevel {
    Stable,
    Unstable,
    Experimental,
    Deprecated,
}

/// Backend capability information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendCapability {
    /// Backend name (e.g., "vulkan", "cuda", "cpu")
    pub name: String,
    /// Whether this backend is available
    pub available: bool,
    /// Backend version/driver info
    pub version: Option<String>,
    /// Supported operations
    pub operations: Vec<String>,
    /// Maximum tensor size, buffer count, etc.
    pub limits: HashMap<String, u64>,
}

impl RuntimeCapabilities {
    /// Create a new capabilities structure with system defaults
    pub fn new() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            build_timestamp: chrono::Utc::now().to_rfc3339(),
            platform: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            features: Self::detect_features(),
            builtins: Vec::new(),
            backends: Vec::new(),
        }
    }

    /// Detect compile-time features
    fn detect_features() -> Vec<String> {
        let mut features = vec!["cpu".to_string()];

        #[cfg(feature = "vulkan")]
        features.push("vulkan".to_string());

        #[cfg(feature = "cuda")]
        features.push("cuda".to_string());

        #[cfg(feature = "metal")]
        features.push("metal".to_string());

        features
    }

    /// Add a builtin to the capabilities
    pub fn add_builtin(&mut self, builtin: BuiltinSpec) {
        self.builtins.push(builtin);
    }

    /// Add a backend capability
    pub fn add_backend(&mut self, backend: BackendCapability) {
        self.backends.push(backend);
    }

    /// Check if a builtin is available with the given name
    pub fn has_builtin(&self, name: &str) -> bool {
        self.builtins.iter().any(|b| b.name == name)
    }

    /// Check if a feature is enabled
    pub fn has_feature(&self, feature: &str) -> bool {
        self.features.iter().any(|f| f == feature)
    }

    /// Get builtin by name
    pub fn get_builtin(&self, name: &str) -> Option<&BuiltinSpec> {
        self.builtins.iter().find(|b| b.name == name)
    }

    /// Serialize to JSON
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize from JSON
    pub fn from_json(json: &str) -> serde_json::Result<Self> {
        serde_json::from_str(json)
    }
}

impl Default for RuntimeCapabilities {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating builtin specifications
pub struct BuiltinSpecBuilder {
    spec: BuiltinSpec,
}

impl BuiltinSpecBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            spec: BuiltinSpec {
                name: name.into(),
                signature: String::new(),
                description: String::new(),
                min_version: None,
                required_features: Vec::new(),
                platforms: Vec::new(),
                parameters: Vec::new(),
                return_type: "()".to_string(),
                stability: StabilityLevel::Stable,
            },
        }
    }

    pub fn signature(mut self, sig: impl Into<String>) -> Self {
        self.spec.signature = sig.into();
        self
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.spec.description = desc.into();
        self
    }

    pub fn min_version(mut self, version: impl Into<String>) -> Self {
        self.spec.min_version = Some(version.into());
        self
    }

    pub fn require_feature(mut self, feature: impl Into<String>) -> Self {
        self.spec.required_features.push(feature.into());
        self
    }

    pub fn platform(mut self, platform: impl Into<String>) -> Self {
        self.spec.platforms.push(platform.into());
        self
    }

    pub fn parameter(
        mut self,
        name: impl Into<String>,
        param_type: impl Into<String>,
        description: impl Into<String>,
        optional: bool,
    ) -> Self {
        self.spec.parameters.push(ParameterSpec {
            name: name.into(),
            param_type: param_type.into(),
            description: description.into(),
            optional,
        });
        self
    }

    pub fn returns(mut self, return_type: impl Into<String>) -> Self {
        self.spec.return_type = return_type.into();
        self
    }

    pub fn stability(mut self, level: StabilityLevel) -> Self {
        self.spec.stability = level;
        self
    }

    pub fn build(self) -> BuiltinSpec {
        self.spec
    }
}

/// Macro to define builtins more concisely
#[macro_export]
macro_rules! builtin_spec {
    (
        $name:expr,
        $signature:expr,
        $description:expr
        $(, features = [$($feature:expr),*])?
        $(, platforms = [$($platform:expr),*])?
        $(, params = [$(($param_name:expr, $param_type:expr, $param_desc:expr)),*])?
        $(, returns = $return_type:expr)?
        $(, stability = $stability:expr)?
    ) => {
        {
            let mut builder = $crate::capabilities::BuiltinSpecBuilder::new($name)
                .signature($signature)
                .description($description);

            $(
                $(
                    builder = builder.require_feature($feature);
                )*
            )?

            $(
                $(
                    builder = builder.platform($platform);
                )*
            )?

            $(
                $(
                    builder = builder.parameter($param_name, $param_type, $param_desc, false);
                )*
            )?

            $(
                builder = builder.returns($return_type);
            )?

            $(
                builder = builder.stability($stability);
            )?

            builder.build()
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_capabilities() {
        let mut caps = RuntimeCapabilities::new();
        assert!(!caps.version.is_empty());
        assert!(!caps.platform.is_empty());

        caps.add_builtin(
            BuiltinSpecBuilder::new("test_fn")
                .signature("test_fn() -> int")
                .description("Test function")
                .returns("int")
                .build(),
        );

        assert!(caps.has_builtin("test_fn"));
        assert!(!caps.has_builtin("nonexistent"));
    }

    #[test]
    fn test_builtin_spec_builder() {
        let spec = BuiltinSpecBuilder::new("alloc_tensor")
            .signature("alloc_tensor(shape: Array, dtype: String) -> Handle")
            .description("Allocate GPU tensor")
            .require_feature("vulkan")
            .parameter("shape", "Array", "Tensor shape", false)
            .parameter("dtype", "String", "Data type", false)
            .returns("Handle")
            .build();

        assert_eq!(spec.name, "alloc_tensor");
        assert_eq!(spec.required_features, vec!["vulkan"]);
        assert_eq!(spec.parameters.len(), 2);
    }

    #[test]
    fn test_json_serialization() {
        let caps = RuntimeCapabilities::new();
        let json = caps.to_json().unwrap();
        let deserialized = RuntimeCapabilities::from_json(&json).unwrap();

        assert_eq!(caps.version, deserialized.version);
        assert_eq!(caps.platform, deserialized.platform);
    }
}
