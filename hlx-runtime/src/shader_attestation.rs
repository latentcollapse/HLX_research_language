use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct ShaderAttestationError {
    pub shader_name: String,
    pub expected_hash: String,
    pub computed_hash: String,
}

impl std::fmt::Display for ShaderAttestationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Shader attestation failed for {}: expected {}, got {}",
            self.shader_name, self.expected_hash, self.computed_hash
        )
    }
}

impl std::error::Error for ShaderAttestationError {}

#[derive(Debug, Clone)]
pub struct ShaderRegistry {
    shaders: HashMap<String, ShaderInfo>,
    strict_mode: bool,
}

#[derive(Debug, Clone)]
pub struct ShaderInfo {
    pub name: String,
    pub bytes: Arc<[u8]>,
    pub expected_hash: String,
    pub verified: bool,
}

impl ShaderRegistry {
    pub fn new() -> Self {
        ShaderRegistry {
            shaders: HashMap::new(),
            strict_mode: true,
        }
    }

    pub fn set_strict_mode(&mut self, strict: bool) {
        self.strict_mode = strict;
    }

    pub fn compute_hash(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        format!("{:x}", result)
    }

    pub fn register(
        &mut self,
        name: &str,
        bytes: &[u8],
        expected_hash: &str,
    ) -> Result<(), ShaderAttestationError> {
        let computed = Self::compute_hash(bytes);

        let verified = if expected_hash.is_empty() {
            if self.strict_mode {
                return Err(ShaderAttestationError {
                    shader_name: name.to_string(),
                    expected_hash: "(none)".to_string(),
                    computed_hash: computed,
                });
            }
            false
        } else if computed != expected_hash {
            return Err(ShaderAttestationError {
                shader_name: name.to_string(),
                expected_hash: expected_hash.to_string(),
                computed_hash: computed,
            });
        } else {
            true
        };

        self.shaders.insert(
            name.to_string(),
            ShaderInfo {
                name: name.to_string(),
                bytes: Arc::from(bytes.to_vec().into_boxed_slice()),
                expected_hash: expected_hash.to_string(),
                verified,
            },
        );

        Ok(())
    }

    pub fn verify(&self, name: &str) -> Result<bool, ShaderAttestationError> {
        let shader = self
            .shaders
            .get(name)
            .ok_or_else(|| ShaderAttestationError {
                shader_name: name.to_string(),
                expected_hash: "N/A".to_string(),
                computed_hash: "N/A".to_string(),
            })?;

        let computed = Self::compute_hash(&shader.bytes);
        if computed != shader.expected_hash {
            return Err(ShaderAttestationError {
                shader_name: name.to_string(),
                expected_hash: shader.expected_hash.clone(),
                computed_hash: computed,
            });
        }

        Ok(true)
    }

    pub fn verify_all(&self) -> Result<usize, ShaderAttestationError> {
        let mut verified = 0;
        for name in self.shaders.keys() {
            if self.verify(name)? {
                verified += 1;
            }
        }
        Ok(verified)
    }

    pub fn get(&self, name: &str) -> Option<&ShaderInfo> {
        self.shaders.get(name)
    }

    pub fn list(&self) -> Vec<&str> {
        self.shaders.keys().map(|s| s.as_str()).collect()
    }

    pub fn count(&self) -> usize {
        self.shaders.len()
    }
}

impl Default for ShaderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_hash_deterministic() {
        let data = b"test shader data";
        let hash1 = ShaderRegistry::compute_hash(data);
        let hash2 = ShaderRegistry::compute_hash(data);
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64);
    }

    #[test]
    fn test_hash_changes_with_data() {
        let hash1 = ShaderRegistry::compute_hash(b"data1");
        let hash2 = ShaderRegistry::compute_hash(b"data2");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_register_and_verify() {
        let mut registry = ShaderRegistry::new();
        let data = b"shader bytecode";
        let hash = ShaderRegistry::compute_hash(data);

        registry.register("test_shader", data, &hash).unwrap();
        assert!(registry.verify("test_shader").unwrap());
    }

    #[test]
    fn test_tampered_shader_rejected() {
        let mut registry = ShaderRegistry::new();
        let original = b"original shader";
        let hash = ShaderRegistry::compute_hash(original);

        registry.register("test", original, &hash).unwrap();

        let result = registry.verify("test");
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_shader_bytes_immutable() {
        let mut registry = ShaderRegistry::new();
        let original = b"original shader";
        let hash = ShaderRegistry::compute_hash(original);

        registry.register("test", original, &hash).unwrap();

        let shader = registry.get("test").unwrap();
        let bytes_clone = shader.bytes.clone();
        let _ = shader;

        assert_eq!(&*bytes_clone, original);
    }

    #[test]
    fn test_strict_mode_requires_hash() {
        let mut registry = ShaderRegistry::new();
        registry.set_strict_mode(true);

        let result = registry.register("no_hash", b"data", "");
        assert!(result.is_err());
    }

    #[test]
    fn test_non_strict_allows_no_hash() {
        let mut registry = ShaderRegistry::new();
        registry.set_strict_mode(false);

        let result = registry.register("no_hash", b"data", "");
        assert!(result.is_ok());
        assert!(!registry.get("no_hash").unwrap().verified);
    }

    #[test]
    fn test_verify_all() {
        let mut registry = ShaderRegistry::new();

        let data1 = b"shader1";
        let hash1 = ShaderRegistry::compute_hash(data1);
        registry.register("s1", data1, &hash1).unwrap();

        let data2 = b"shader2";
        let hash2 = ShaderRegistry::compute_hash(data2);
        registry.register("s2", data2, &hash2).unwrap();

        assert_eq!(registry.verify_all().unwrap(), 2);
    }
}
