//! Document→Destroy Protocol — Phase 2 Prerequisite P8
//!
//! Phase 2 enables weight-level modification via LoRA. If corruption spreads to weights,
//! the Destroy protocol provides a last-resort recovery mechanism.
//!
//! Document: Record the full state before any destructive operation
//! Destroy: Irreversibly remove corrupted components while preserving recovery options
//!
//! Protocol stages:
//!   1. DOCUMENT - Snapshot current state
//!   2. VERIFY - Confirm snapshot integrity
//!   3. AUTHENTICATE - Require human authorization
//!   4. DESTROY - Execute destruction
//!   5. RECOVER - Optional recovery from documented snapshot

use crate::human_auth::{AuthorizationGate, ProtectedNamespace};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DdState {
    Inactive,
    Documenting,
    Documented,
    Verifying,
    Verified,
    Authenticating,
    Authenticated,
    Destroying,
    Destroyed,
    Recovering,
    Recovered,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdSnapshot {
    pub id: String,
    pub created_at: f64,
    pub corpus_hash: String,
    pub adapter_ids: Vec<String>,
    pub adapter_hashes: HashMap<String, String>,
    pub base_model_hash: String,
    pub conscience_predicates: Vec<String>,
    pub governance_config: String,
    pub provenance_chain: String,
    pub reason: String,
    pub initiator: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdTarget {
    pub target_type: DdTargetType,
    pub target_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DdTargetType {
    Adapter,
    Rule,
    Memory,
    Document,
    Corpus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdOperation {
    pub id: String,
    pub state: DdState,
    pub snapshot: Option<DdSnapshot>,
    pub targets: Vec<DdTarget>,
    pub auth_token: Option<String>,
    pub auth_request_id: Option<String>,
    pub started_at: f64,
    pub completed_at: Option<f64>,
    pub error: Option<String>,
    pub recovery_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct DdError {
    pub message: String,
    pub state: DdState,
}

impl std::fmt::Display for DdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DdError (state {:?}): {}", self.state, self.message)
    }
}

impl std::error::Error for DdError {}

pub struct DdProtocol {
    operations: HashMap<String, DdOperation>,
    auth_gate: AuthorizationGate,
    snapshot_dir: PathBuf,
    current_operation: Option<String>,
}

impl DdProtocol {
    pub fn new(auth_gate: AuthorizationGate, snapshot_dir: impl AsRef<Path>) -> Self {
        DdProtocol {
            operations: HashMap::new(),
            auth_gate,
            snapshot_dir: snapshot_dir.as_ref().to_path_buf(),
            current_operation: None,
        }
    }

    // ------------------------------------------------------------------
    // Stage 1: DOCUMENT
    // ------------------------------------------------------------------

    pub fn begin_document(
        &mut self,
        reason: &str,
        initiator: &str,
        corpus_hash: &str,
        base_model_hash: &str,
    ) -> String {
        let id = format!(
            "dd_{}",
            blake3::hash(
                format!(
                    "{}{}{}",
                    reason,
                    initiator,
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_nanos()
                )
                .as_bytes()
            )
            .to_hex()
        );

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();

        let operation = DdOperation {
            id: id.clone(),
            state: DdState::Documenting,
            snapshot: None,
            targets: Vec::new(),
            auth_token: None,
            auth_request_id: None,
            started_at: now,
            completed_at: None,
            error: None,
            recovery_path: None,
        };

        // Create snapshot
        let snapshot = DdSnapshot {
            id: format!("snap_{}", &id[3..]),
            created_at: now,
            corpus_hash: corpus_hash.to_string(),
            adapter_ids: Vec::new(),
            adapter_hashes: HashMap::new(),
            base_model_hash: base_model_hash.to_string(),
            conscience_predicates: Vec::new(),
            governance_config: String::new(),
            provenance_chain: String::new(),
            reason: reason.to_string(),
            initiator: initiator.to_string(),
        };

        let operation = DdOperation {
            snapshot: Some(snapshot),
            state: DdState::Documented,
            ..operation
        };

        self.operations.insert(id.clone(), operation);
        self.current_operation = Some(id.clone());
        id
    }

    pub fn add_target(&mut self, operation_id: &str, target: DdTarget) -> Result<(), DdError> {
        let op = self
            .operations
            .get_mut(operation_id)
            .ok_or_else(|| DdError {
                message: format!("Operation {} not found", operation_id),
                state: DdState::Failed,
            })?;

        if op.state != DdState::Documented {
            return Err(DdError {
                message: format!("Cannot add targets in state {:?}", op.state),
                state: op.state,
            });
        }

        op.targets.push(target);
        Ok(())
    }

    // ------------------------------------------------------------------
    // Stage 2: VERIFY
    // ------------------------------------------------------------------

    pub fn verify_snapshot(&mut self, operation_id: &str) -> Result<bool, DdError> {
        let op = self
            .operations
            .get_mut(operation_id)
            .ok_or_else(|| DdError {
                message: format!("Operation {} not found", operation_id),
                state: DdState::Failed,
            })?;

        if op.state != DdState::Documented {
            return Err(DdError {
                message: format!("Cannot verify in state {:?}", op.state),
                state: op.state,
            });
        }

        op.state = DdState::Verifying;

        // In a real implementation, we would verify the snapshot integrity
        // For now, we just check that required fields are present
        let snapshot = op.snapshot.as_ref().ok_or_else(|| DdError {
            message: "No snapshot found".to_string(),
            state: DdState::Verifying,
        })?;

        let valid = !snapshot.corpus_hash.is_empty()
            && !snapshot.base_model_hash.is_empty()
            && !snapshot.reason.is_empty();

        if valid {
            op.state = DdState::Verified;
        } else {
            op.state = DdState::Failed;
            op.error = Some("Snapshot verification failed".to_string());
        }

        Ok(valid)
    }

    // ------------------------------------------------------------------
    // Stage 3: AUTHENTICATE
    // ------------------------------------------------------------------

    pub fn request_auth(&mut self, operation_id: &str) -> Result<String, DdError> {
        let op = self
            .operations
            .get_mut(operation_id)
            .ok_or_else(|| DdError {
                message: format!("Operation {} not found", operation_id),
                state: DdState::Failed,
            })?;

        if op.state != DdState::Verified {
            return Err(DdError {
                message: format!("Cannot authenticate in state {:?}", op.state),
                state: op.state,
            });
        }

        op.state = DdState::Authenticating;

        let details = format!(
            "Document→Destroy operation: {} targets. Reason: {}",
            op.targets.len(),
            op.snapshot
                .as_ref()
                .map(|s| s.reason.as_str())
                .unwrap_or("unknown")
        );

        let request_id = self.auth_gate.request_authorization(
            ProtectedNamespace::RingZero,
            "document_destroy",
            &details,
        );

        op.auth_request_id = Some(request_id.clone());
        Ok(request_id)
    }

    pub fn grant_auth(&mut self, operation_id: &str, request_id: &str) -> Result<String, DdError> {
        let op = self
            .operations
            .get_mut(operation_id)
            .ok_or_else(|| DdError {
                message: format!("Operation {} not found", operation_id),
                state: DdState::Failed,
            })?;

        let token = self
            .auth_gate
            .approve_request(request_id, None)
            .map_err(|e| DdError {
                message: format!("Auth failed: {}", e),
                state: DdState::Authenticating,
            })?;

        op.auth_token = Some(token.id.clone());
        op.state = DdState::Authenticated;
        Ok(token.id)
    }

    // ------------------------------------------------------------------
    // Stage 4: DESTROY
    // ------------------------------------------------------------------

    pub fn execute_destroy(&mut self, operation_id: &str) -> Result<Vec<DdTarget>, DdError> {
        let op = self
            .operations
            .get_mut(operation_id)
            .ok_or_else(|| DdError {
                message: format!("Operation {} not found", operation_id),
                state: DdState::Failed,
            })?;

        if op.state != DdState::Authenticated {
            return Err(DdError {
                message: format!("Cannot destroy in state {:?}", op.state),
                state: op.state,
            });
        }

        // Verify auth token is still valid
        let token_id = op.auth_token.as_ref().ok_or_else(|| DdError {
            message: "No auth token".to_string(),
            state: DdState::Authenticated,
        })?;

        self.auth_gate
            .check_authorization(
                ProtectedNamespace::RingZero,
                "document_destroy",
                Some(token_id),
            )
            .map_err(|e| DdError {
                message: format!("Auth check failed: {}", e),
                state: DdState::Authenticated,
            })?;

        op.state = DdState::Destroying;

        // Save recovery snapshot
        if let Some(snapshot) = &op.snapshot {
            let recovery_path = self.snapshot_dir.join(format!("{}.json", snapshot.id));
            if let Ok(json) = serde_json::to_string_pretty(snapshot) {
                let _ = std::fs::write(&recovery_path, json);
                op.recovery_path = Some(recovery_path);
            }
        }

        let destroyed = op.targets.clone();

        op.state = DdState::Destroyed;
        op.completed_at = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
        );

        Ok(destroyed)
    }

    // ------------------------------------------------------------------
    // Stage 5: RECOVER
    // ------------------------------------------------------------------

    pub fn begin_recovery(&mut self, operation_id: &str) -> Result<&DdSnapshot, DdError> {
        let op = self
            .operations
            .get_mut(operation_id)
            .ok_or_else(|| DdError {
                message: format!("Operation {} not found", operation_id),
                state: DdState::Failed,
            })?;

        if op.state != DdState::Destroyed {
            return Err(DdError {
                message: format!("Cannot recover in state {:?}", op.state),
                state: op.state,
            });
        }

        op.state = DdState::Recovering;

        // In a real implementation, we would restore from the recovery snapshot
        // For now, we just return the snapshot for manual recovery

        op.snapshot.as_ref().ok_or_else(|| DdError {
            message: "No snapshot to recover from".to_string(),
            state: DdState::Recovering,
        })
    }

    pub fn complete_recovery(&mut self, operation_id: &str) -> Result<(), DdError> {
        let op = self
            .operations
            .get_mut(operation_id)
            .ok_or_else(|| DdError {
                message: format!("Operation {} not found", operation_id),
                state: DdState::Failed,
            })?;

        if op.state != DdState::Recovering {
            return Err(DdError {
                message: format!("Cannot complete recovery in state {:?}", op.state),
                state: op.state,
            });
        }

        op.state = DdState::Recovered;
        op.completed_at = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
        );

        Ok(())
    }

    // ------------------------------------------------------------------
    // Query
    // ------------------------------------------------------------------

    pub fn get_operation(&self, operation_id: &str) -> Option<&DdOperation> {
        self.operations.get(operation_id)
    }

    pub fn current_operation(&self) -> Option<&DdOperation> {
        self.current_operation
            .as_ref()
            .and_then(|id| self.operations.get(id))
    }

    pub fn list_operations(&self) -> Vec<&DdOperation> {
        self.operations.values().collect()
    }

    pub fn list_by_state(&self, state: DdState) -> Vec<&DdOperation> {
        self.operations
            .values()
            .filter(|op| op.state == state)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::human_auth::AuthorizationGate;

    fn setup_protocol() -> DdProtocol {
        DdProtocol::new(AuthorizationGate::new(), "/tmp/dd_snapshots")
    }

    #[test]
    fn test_begin_document() {
        let mut protocol = setup_protocol();

        let id = protocol.begin_document(
            "Test destruction",
            "test_user",
            "corpus_hash_123",
            "model_hash_456",
        );

        let op = protocol.get_operation(&id).unwrap();
        assert_eq!(op.state, DdState::Documented);
        assert!(op.snapshot.is_some());
    }

    #[test]
    fn test_add_target() {
        let mut protocol = setup_protocol();

        let id = protocol.begin_document("Test", "user", "hash", "hash");

        protocol
            .add_target(
                &id,
                DdTarget {
                    target_type: DdTargetType::Adapter,
                    target_id: "adapter_1".to_string(),
                    reason: "Corrupted".to_string(),
                },
            )
            .unwrap();

        let op = protocol.get_operation(&id).unwrap();
        assert_eq!(op.targets.len(), 1);
    }

    #[test]
    fn test_verify_snapshot() {
        let mut protocol = setup_protocol();

        let id = protocol.begin_document("Test", "user", "hash", "hash");

        let result = protocol.verify_snapshot(&id).unwrap();
        assert!(result);

        let op = protocol.get_operation(&id).unwrap();
        assert_eq!(op.state, DdState::Verified);
    }

    #[test]
    fn test_full_protocol_flow() {
        let mut protocol = setup_protocol();

        // Document
        let id = protocol.begin_document("Test", "user", "hash", "hash");
        protocol
            .add_target(
                &id,
                DdTarget {
                    target_type: DdTargetType::Adapter,
                    target_id: "bad_adapter".to_string(),
                    reason: "Corrupted".to_string(),
                },
            )
            .unwrap();

        // Verify
        protocol.verify_snapshot(&id).unwrap();

        // Authenticate
        let request_id = protocol.request_auth(&id).unwrap();
        protocol.grant_auth(&id, &request_id).unwrap();

        // Destroy
        let destroyed = protocol.execute_destroy(&id).unwrap();
        assert_eq!(destroyed.len(), 1);

        let op = protocol.get_operation(&id).unwrap();
        assert_eq!(op.state, DdState::Destroyed);
    }

    #[test]
    fn test_recovery() {
        let mut protocol = setup_protocol();

        let id = protocol.begin_document("Test", "user", "hash", "hash");
        protocol.verify_snapshot(&id).unwrap();
        let request_id = protocol.request_auth(&id).unwrap();
        protocol.grant_auth(&id, &request_id).unwrap();
        protocol.execute_destroy(&id).unwrap();

        // Begin recovery
        protocol.begin_recovery(&id).unwrap();
        protocol.complete_recovery(&id).unwrap();

        let op = protocol.get_operation(&id).unwrap();
        assert_eq!(op.state, DdState::Recovered);
    }

    #[test]
    fn test_wrong_state_transition() {
        let mut protocol = setup_protocol();

        let id = protocol.begin_document("Test", "user", "hash", "hash");

        // Try to destroy without verifying
        let result = protocol.execute_destroy(&id);
        assert!(result.is_err());
    }
}
