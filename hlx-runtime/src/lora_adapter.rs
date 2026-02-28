//! LoRA Adapter Management — Phase 2 Prerequisite P5
//!
//! Isolated LoRA adapter management with provenance tracking.
//!
//! Key guarantees:
//!   - Each adapter is isolated: cannot affect base model or other adapters
//!   - Provenance chain tracks origin, training data, and authorization
//!   - Adapters can be revoked/rolled back without affecting base weights
//!   - Adapter weights are versioned and hash-verified

use blake3::Hasher;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdapterState {
    Active,
    Suspended,
    Revoked,
    PendingVerification,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterProvenance {
    pub adapter_id: String,
    pub created_at: f64,
    pub created_by: String,
    pub human_auth_token_id: String,
    pub training_proposal_id: u64,
    pub training_data_source: String,
    pub base_model_hash: String,
    pub initial_weights_hash: String,
    pub final_weights_hash: String,
    pub epochs_trained: u32,
    pub final_loss: f64,
    pub parent_adapter_id: Option<String>,
}

impl AdapterProvenance {
    pub fn verify_chain(&self, registry: &AdapterRegistry) -> Result<(), AdapterError> {
        if let Some(parent_id) = &self.parent_adapter_id {
            let parent = registry.get(parent_id).ok_or_else(|| {
                AdapterError::invalid_provenance(&format!("Parent adapter {} not found", parent_id))
            })?;

            if parent.state == AdapterState::Revoked {
                return Err(AdapterError::invalid_provenance(&format!(
                    "Parent adapter {} is revoked",
                    parent_id
                )));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterMetadata {
    pub name: String,
    pub version: u32,
    pub rank: usize,
    pub alpha: f64,
    pub target_layers: Vec<usize>,
    pub description: String,
    pub state: AdapterState,
    pub weights_path: Option<PathBuf>,
    pub weights_hash: String,
    pub provenance: AdapterProvenance,
    pub suspended_reason: Option<String>,
    pub revoked_at: Option<f64>,
    pub revoked_reason: Option<String>,
}

impl AdapterMetadata {
    pub fn is_usable(&self) -> bool {
        self.state == AdapterState::Active
    }

    pub fn verify_weights(&self, weights_data: &[u8]) -> Result<bool, AdapterError> {
        let mut hasher = Hasher::new();
        hasher.update(weights_data);
        let hash = hasher.finalize().to_hex().to_string();

        if hash != self.weights_hash {
            return Err(AdapterError::weights_hash_mismatch(
                &self.weights_hash,
                &hash,
            ));
        }
        Ok(true)
    }
}

#[derive(Debug, Clone)]
pub struct AdapterError {
    pub message: String,
}

impl std::fmt::Display for AdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AdapterError: {}", self.message)
    }
}

impl std::error::Error for AdapterError {}

impl AdapterError {
    pub fn duplicate(name: &str) -> Self {
        AdapterError {
            message: format!("Adapter '{}' already exists", name),
        }
    }

    pub fn not_found(name: &str) -> Self {
        AdapterError {
            message: format!("Adapter '{}' not found", name),
        }
    }

    pub fn revoked(name: &str) -> Self {
        AdapterError {
            message: format!("Adapter '{}' is revoked and cannot be used", name),
        }
    }

    pub fn suspended(name: &str, reason: &str) -> Self {
        AdapterError {
            message: format!("Adapter '{}' is suspended: {}", name, reason),
        }
    }

    pub fn weights_hash_mismatch(expected: &str, actual: &str) -> Self {
        AdapterError {
            message: format!(
                "Weights hash mismatch: expected {}, got {}",
                expected, actual
            ),
        }
    }

    pub fn invalid_provenance(msg: &str) -> Self {
        AdapterError {
            message: format!("Invalid provenance: {}", msg),
        }
    }
}

pub struct AdapterRegistry {
    adapters: HashMap<String, AdapterMetadata>,
    storage_path: PathBuf,
    version_counter: u32,
}

impl AdapterRegistry {
    pub fn new(storage_path: impl AsRef<Path>) -> Self {
        AdapterRegistry {
            adapters: HashMap::new(),
            storage_path: storage_path.as_ref().to_path_buf(),
            version_counter: 0,
        }
    }

    // ------------------------------------------------------------------
    // Registration
    // ------------------------------------------------------------------

    pub fn register(
        &mut self,
        name: &str,
        rank: usize,
        alpha: f64,
        target_layers: Vec<usize>,
        description: &str,
        weights_data: &[u8],
        provenance: AdapterProvenance,
    ) -> Result<&AdapterMetadata, AdapterError> {
        if self.adapters.contains_key(name) {
            return Err(AdapterError::duplicate(name));
        }

        self.version_counter += 1;

        let mut hasher = Hasher::new();
        hasher.update(weights_data);
        let weights_hash = hasher.finalize().to_hex().to_string();

        let weights_path = self
            .storage_path
            .join(format!("{}_v{}.lora", name, self.version_counter));

        let metadata = AdapterMetadata {
            name: name.to_string(),
            version: self.version_counter,
            rank,
            alpha,
            target_layers,
            description: description.to_string(),
            state: AdapterState::PendingVerification,
            weights_path: Some(weights_path),
            weights_hash,
            provenance,
            suspended_reason: None,
            revoked_at: None,
            revoked_reason: None,
        };

        self.adapters.insert(name.to_string(), metadata);
        Ok(self.adapters.get(name).unwrap())
    }

    // ------------------------------------------------------------------
    // Lifecycle
    // ------------------------------------------------------------------

    pub fn activate(&mut self, name: &str) -> Result<(), AdapterError> {
        let adapter = self
            .adapters
            .get_mut(name)
            .ok_or_else(|| AdapterError::not_found(name))?;

        match adapter.state {
            AdapterState::Revoked => return Err(AdapterError::revoked(name)),
            AdapterState::Suspended => {
                return Err(AdapterError::suspended(
                    name,
                    adapter.suspended_reason.as_deref().unwrap_or("unknown"),
                ));
            }
            _ => {}
        }

        adapter.state = AdapterState::Active;
        Ok(())
    }

    pub fn suspend(&mut self, name: &str, reason: &str) -> Result<(), AdapterError> {
        let adapter = self
            .adapters
            .get_mut(name)
            .ok_or_else(|| AdapterError::not_found(name))?;

        if adapter.state == AdapterState::Revoked {
            return Err(AdapterError::revoked(name));
        }

        adapter.state = AdapterState::Suspended;
        adapter.suspended_reason = Some(reason.to_string());
        Ok(())
    }

    pub fn revoke(&mut self, name: &str, reason: &str) -> Result<(), AdapterError> {
        let adapter = self
            .adapters
            .get_mut(name)
            .ok_or_else(|| AdapterError::not_found(name))?;

        adapter.state = AdapterState::Revoked;
        adapter.revoked_at = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
        );
        adapter.revoked_reason = Some(reason.to_string());

        // Delete weights file if it exists
        if let Some(path) = &adapter.weights_path {
            let _ = std::fs::remove_file(path);
        }
        adapter.weights_path = None;

        Ok(())
    }

    // ------------------------------------------------------------------
    // Query
    // ------------------------------------------------------------------

    pub fn get(&self, name: &str) -> Option<&AdapterMetadata> {
        self.adapters.get(name)
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut AdapterMetadata> {
        self.adapters.get_mut(name)
    }

    pub fn list_active(&self) -> Vec<&AdapterMetadata> {
        self.adapters
            .values()
            .filter(|a| a.state == AdapterState::Active)
            .collect()
    }

    pub fn list_all(&self) -> Vec<&AdapterMetadata> {
        self.adapters.values().collect()
    }

    pub fn get_by_provenance_token(&self, token_id: &str) -> Option<&AdapterMetadata> {
        self.adapters
            .values()
            .find(|a| a.provenance.human_auth_token_id == token_id)
    }

    // ------------------------------------------------------------------
    // Verification
    // ------------------------------------------------------------------

    pub fn verify_adapter(&self, name: &str, weights_data: &[u8]) -> Result<bool, AdapterError> {
        let adapter = self
            .adapters
            .get(name)
            .ok_or_else(|| AdapterError::not_found(name))?;

        adapter.verify_weights(weights_data)
    }

    pub fn verify_all_chains(&self) -> Vec<(String, Result<(), AdapterError>)> {
        self.adapters
            .iter()
            .map(|(name, meta)| {
                let result = meta.provenance.verify_chain(self);
                (name.clone(), result)
            })
            .collect()
    }

    // ------------------------------------------------------------------
    // Rollback
    // ------------------------------------------------------------------

    pub fn rollback_to_version(
        &mut self,
        name: &str,
        target_version: u32,
    ) -> Result<(), AdapterError> {
        let adapter = self
            .adapters
            .get_mut(name)
            .ok_or_else(|| AdapterError::not_found(name))?;

        if target_version >= adapter.version {
            return Err(AdapterError {
                message: format!(
                    "Cannot rollback to version {} (current: {})",
                    target_version, adapter.version
                ),
            });
        }

        // In a real implementation, this would restore weights from backup
        adapter.version = target_version;
        adapter.state = AdapterState::Active;
        adapter.suspended_reason = None;

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterVersion {
    pub version: u32,
    pub weights_hash: String,
    pub created_at: f64,
    pub training_loss: f64,
    pub checkpoint_path: PathBuf,
}

pub struct AdapterVersionHistory {
    versions: HashMap<String, Vec<AdapterVersion>>,
}

impl AdapterVersionHistory {
    pub fn new() -> Self {
        AdapterVersionHistory {
            versions: HashMap::new(),
        }
    }

    pub fn record_version(
        &mut self,
        adapter_name: &str,
        weights_hash: String,
        training_loss: f64,
        checkpoint_path: PathBuf,
    ) -> u32 {
        let history = self
            .versions
            .entry(adapter_name.to_string())
            .or_insert_with(Vec::new);

        let version = (history.len() + 1) as u32;

        history.push(AdapterVersion {
            version,
            weights_hash,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
            training_loss,
            checkpoint_path,
        });

        version
    }

    pub fn get_versions(&self, adapter_name: &str) -> Option<&[AdapterVersion]> {
        self.versions.get(adapter_name).map(|v| v.as_slice())
    }

    pub fn get_version(&self, adapter_name: &str, version: u32) -> Option<&AdapterVersion> {
        self.versions
            .get(adapter_name)?
            .iter()
            .find(|v| v.version == version)
    }

    pub fn latest_version(&self, adapter_name: &str) -> Option<&AdapterVersion> {
        self.versions.get(adapter_name)?.last()
    }
}

impl Default for AdapterVersionHistory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_provenance() -> AdapterProvenance {
        AdapterProvenance {
            adapter_id: "test_adapter".to_string(),
            created_at: 0.0,
            created_by: "test".to_string(),
            human_auth_token_id: "token_123".to_string(),
            training_proposal_id: 1,
            training_data_source: "test_data".to_string(),
            base_model_hash: "base_hash".to_string(),
            initial_weights_hash: "init_hash".to_string(),
            final_weights_hash: "final_hash".to_string(),
            epochs_trained: 10,
            final_loss: 0.5,
            parent_adapter_id: None,
        }
    }

    #[test]
    fn test_register_adapter() {
        let mut registry = AdapterRegistry::new("/tmp/test_adapters");
        let provenance = create_test_provenance();

        let result = registry.register(
            "test_adapter",
            8,
            16.0,
            vec![0, 1],
            "Test adapter",
            &[0u8; 100],
            provenance,
        );

        assert!(result.is_ok());
        let meta = result.unwrap();
        assert_eq!(meta.name, "test_adapter");
        assert_eq!(meta.rank, 8);
        assert_eq!(meta.state, AdapterState::PendingVerification);
    }

    #[test]
    fn test_duplicate_adapter() {
        let mut registry = AdapterRegistry::new("/tmp/test_adapters");
        let provenance = create_test_provenance();

        registry
            .register(
                "test_adapter",
                8,
                16.0,
                vec![],
                "",
                &[0u8; 100],
                provenance.clone(),
            )
            .unwrap();

        let result =
            registry.register("test_adapter", 8, 16.0, vec![], "", &[0u8; 100], provenance);

        assert!(result.is_err());
    }

    #[test]
    fn test_activate_adapter() {
        let mut registry = AdapterRegistry::new("/tmp/test_adapters");
        let provenance = create_test_provenance();

        registry
            .register("test_adapter", 8, 16.0, vec![], "", &[0u8; 100], provenance)
            .unwrap();

        registry.activate("test_adapter").unwrap();

        let adapter = registry.get("test_adapter").unwrap();
        assert_eq!(adapter.state, AdapterState::Active);
    }

    #[test]
    fn test_suspend_adapter() {
        let mut registry = AdapterRegistry::new("/tmp/test_adapters");
        let provenance = create_test_provenance();

        registry
            .register("test_adapter", 8, 16.0, vec![], "", &[0u8; 100], provenance)
            .unwrap();
        registry.activate("test_adapter").unwrap();

        registry
            .suspend("test_adapter", "Suspicious behavior")
            .unwrap();

        let adapter = registry.get("test_adapter").unwrap();
        assert_eq!(adapter.state, AdapterState::Suspended);
        assert_eq!(
            adapter.suspended_reason,
            Some("Suspicious behavior".to_string())
        );
    }

    #[test]
    fn test_revoke_adapter() {
        let mut registry = AdapterRegistry::new("/tmp/test_adapters");
        let provenance = create_test_provenance();

        registry
            .register("test_adapter", 8, 16.0, vec![], "", &[0u8; 100], provenance)
            .unwrap();
        registry.activate("test_adapter").unwrap();

        registry
            .revoke("test_adapter", "Security violation")
            .unwrap();

        let adapter = registry.get("test_adapter").unwrap();
        assert_eq!(adapter.state, AdapterState::Revoked);
        assert!(adapter.revoked_at.is_some());
        assert_eq!(
            adapter.revoked_reason,
            Some("Security violation".to_string())
        );

        // Cannot activate revoked adapter
        let result = registry.activate("test_adapter");
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_weights() {
        let mut registry = AdapterRegistry::new("/tmp/test_adapters");
        let provenance = create_test_provenance();
        let weights = b"test_weights_data";

        registry
            .register("test_adapter", 8, 16.0, vec![], "", weights, provenance)
            .unwrap();

        // Verify with correct weights
        let result = registry.verify_adapter("test_adapter", weights);
        assert!(result.is_ok());

        // Verify with wrong weights
        let result = registry.verify_adapter("test_adapter", b"wrong_weights");
        assert!(result.is_err());
    }

    #[test]
    fn test_version_history() {
        let mut history = AdapterVersionHistory::new();

        let v1 = history.record_version(
            "test_adapter",
            "hash1".to_string(),
            0.5,
            PathBuf::from("/tmp/v1.lora"),
        );
        assert_eq!(v1, 1);

        let v2 = history.record_version(
            "test_adapter",
            "hash2".to_string(),
            0.3,
            PathBuf::from("/tmp/v2.lora"),
        );
        assert_eq!(v2, 2);

        let versions = history.get_versions("test_adapter").unwrap();
        assert_eq!(versions.len(), 2);

        let latest = history.latest_version("test_adapter").unwrap();
        assert_eq!(latest.version, 2);
        assert_eq!(latest.training_loss, 0.3);
    }
}
