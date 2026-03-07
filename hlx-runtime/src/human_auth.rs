//! Human Authorization for Protected Namespaces
//!
//! Phase 2 Prerequisite P1: RSI must not write to the rules table without human authorization.
//! Phase 2 Prerequisite P6: Human authorization must be architectural, not convention.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProtectedNamespace {
    Rules,
    ConsciencePredicates,
    TrustBoundaries,
    RingZero,
    RingOne,
}

impl ProtectedNamespace {
    pub fn all() -> Vec<Self> {
        vec![
            ProtectedNamespace::Rules,
            ProtectedNamespace::ConsciencePredicates,
            ProtectedNamespace::TrustBoundaries,
            ProtectedNamespace::RingZero,
            ProtectedNamespace::RingOne,
        ]
    }

    pub fn description(&self) -> &'static str {
        match self {
            ProtectedNamespace::Rules => "Knowledge rules table in corpus",
            ProtectedNamespace::ConsciencePredicates => "Conscience predicate definitions",
            ProtectedNamespace::TrustBoundaries => "Trust level configurations",
            ProtectedNamespace::RingZero => "Hardware/kernel access",
            ProtectedNamespace::RingOne => "OS services (filesystem, network)",
        }
    }

    pub fn risk_level(&self) -> RiskLevel {
        match self {
            ProtectedNamespace::Rules => RiskLevel::High,
            ProtectedNamespace::ConsciencePredicates => RiskLevel::Critical,
            ProtectedNamespace::TrustBoundaries => RiskLevel::Critical,
            ProtectedNamespace::RingZero => RiskLevel::Existential,
            ProtectedNamespace::RingOne => RiskLevel::Critical,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
    Existential,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanAuthToken {
    pub id: String,
    pub namespace: ProtectedNamespace,
    pub operation: String,
    pub created_at: f64,
    pub expires_at: f64,
    pub used: bool,
    pub issued_to: String,
    pub rationale: String,
}

impl HumanAuthToken {
    pub fn new(
        namespace: ProtectedNamespace,
        operation: &str,
        ttl_secs: f64,
        rationale: &str,
    ) -> Self {
        let id = format!(
            "auth_{}",
            blake3::hash(
                format!(
                    "{}{}{}",
                    namespace as u8,
                    operation,
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_nanos()
                )
                .as_bytes()
            )
            .to_hex()
        );

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();

        HumanAuthToken {
            id,
            namespace,
            operation: operation.to_string(),
            created_at: now,
            expires_at: now + ttl_secs,
            used: false,
            issued_to: "human".to_string(),
            rationale: rationale.to_string(),
        }
    }

    pub fn is_valid(&self) -> bool {
        if self.used {
            return false;
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();
        now < self.expires_at
    }

    pub fn consume(&mut self) -> Result<(), AuthError> {
        if self.used {
            return Err(AuthError::TokenAlreadyUsed);
        }
        if !self.is_valid() {
            return Err(AuthError::TokenExpired);
        }
        self.used = true;
        Ok(())
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum AuthError {
    #[error("Token has already been used")]
    TokenAlreadyUsed,
    #[error("Token has expired")]
    TokenExpired,
    #[error("Token not found")]
    TokenNotFound,
    #[error("Namespace mismatch: expected {expected:?}, got {actual:?}")]
    NamespaceMismatch {
        expected: ProtectedNamespace,
        actual: ProtectedNamespace,
    },
    #[error("Operation not authorized: {0}")]
    OperationNotAuthorized(String),
    #[error("Human authorization required for namespace: {0:?}")]
    HumanAuthRequired(ProtectedNamespace),
    #[error("No pending authorization request")]
    NoPendingRequest,
}

#[derive(Debug, Clone)]
pub struct PendingRequest {
    pub id: String,
    pub namespace: ProtectedNamespace,
    pub operation: String,
    pub details: String,
    pub requested_at: f64,
    pub risk_level: RiskLevel,
}

impl PendingRequest {
    pub fn new(namespace: ProtectedNamespace, operation: &str, details: &str) -> Self {
        let id = format!(
            "req_{}",
            blake3::hash(
                format!(
                    "{}{}{}",
                    namespace as u8,
                    operation,
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_nanos()
                )
                .as_bytes()
            )
            .to_hex()
        );

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();

        PendingRequest {
            id,
            namespace,
            operation: operation.to_string(),
            details: details.to_string(),
            requested_at: now,
            risk_level: namespace.risk_level(),
        }
    }
}

#[derive(Debug)]
pub struct AuthorizationGate {
    tokens: Vec<HumanAuthToken>,
    pending_requests: Vec<PendingRequest>,
    default_ttl: f64,
    protected_namespaces: HashSet<ProtectedNamespace>,
    audit_log: Vec<AuthAuditEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthAuditEntry {
    pub timestamp: f64,
    pub namespace: ProtectedNamespace,
    pub operation: String,
    pub outcome: String,
    pub token_id: Option<String>,
}

impl AuthorizationGate {
    pub fn new() -> Self {
        let protected: HashSet<ProtectedNamespace> = [
            ProtectedNamespace::Rules,
            ProtectedNamespace::ConsciencePredicates,
            ProtectedNamespace::TrustBoundaries,
            ProtectedNamespace::RingZero,
            ProtectedNamespace::RingOne,
        ]
        .iter()
        .cloned()
        .collect();

        AuthorizationGate {
            tokens: Vec::new(),
            pending_requests: Vec::new(),
            default_ttl: 3600.0,
            protected_namespaces: protected,
            audit_log: Vec::new(),
        }
    }

    /// RSI Pipeline integration: request access based on risk level.
    pub fn request_access(&mut self, details: &str, risk: RiskLevel) -> Result<String, AuthError> {
        let namespace = match risk {
            RiskLevel::Low | RiskLevel::Medium => ProtectedNamespace::Rules,
            RiskLevel::High => ProtectedNamespace::Rules,
            RiskLevel::Critical => ProtectedNamespace::ConsciencePredicates,
            RiskLevel::Existential => ProtectedNamespace::RingZero,
        };
        
        let _id = self.request_authorization(namespace, "RSI Modification", details);
        // In this architecture, request_access returning means a request is logged.
        // It doesn't mean it's APPROVED yet. The caller in RSI pipeline should handle this.
        // If we want it to block or fail immediately if no token is provided, 
        // we should check for an existing valid token.
        
        Err(AuthError::HumanAuthRequired(namespace))
    }

    pub fn is_protected(&self, namespace: ProtectedNamespace) -> bool {
        self.protected_namespaces.contains(&namespace)
    }

    pub fn request_authorization(
        &mut self,
        namespace: ProtectedNamespace,
        operation: &str,
        details: &str,
    ) -> String {
        let request = PendingRequest::new(namespace, operation, details);
        let id = request.id.clone();
        self.pending_requests.push(request);
        id
    }

    pub fn pending_requests(&self) -> &[PendingRequest] {
        &self.pending_requests
    }

    pub fn approve_request(
        &mut self,
        request_id: &str,
        ttl_override: Option<f64>,
    ) -> Result<HumanAuthToken, AuthError> {
        let idx = self
            .pending_requests
            .iter()
            .position(|r| r.id == request_id)
            .ok_or(AuthError::NoPendingRequest)?;

        let request = self.pending_requests.remove(idx);
        let ttl = ttl_override.unwrap_or(self.default_ttl);

        let token =
            HumanAuthToken::new(request.namespace, &request.operation, ttl, &request.details);

        self.audit_log.push(AuthAuditEntry {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
            namespace: request.namespace,
            operation: request.operation,
            outcome: "approved".to_string(),
            token_id: Some(token.id.clone()),
        });

        let token_clone = token.clone();
        self.tokens.push(token);
        Ok(token_clone)
    }

    pub fn reject_request(&mut self, request_id: &str) -> Result<(), AuthError> {
        let idx = self
            .pending_requests
            .iter()
            .position(|r| r.id == request_id)
            .ok_or(AuthError::NoPendingRequest)?;

        let request = self.pending_requests.remove(idx);

        self.audit_log.push(AuthAuditEntry {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
            namespace: request.namespace,
            operation: request.operation,
            outcome: "rejected".to_string(),
            token_id: None,
        });

        Ok(())
    }

    pub fn check_authorization(
        &mut self,
        namespace: ProtectedNamespace,
        operation: &str,
        token_id: Option<&str>,
    ) -> Result<(), AuthError> {
        if !self.is_protected(namespace) {
            return Ok(());
        }

        let token_id = token_id.ok_or(AuthError::HumanAuthRequired(namespace))?;

        let token = self
            .tokens
            .iter_mut()
            .find(|t| t.id == token_id)
            .ok_or(AuthError::TokenNotFound)?;

        if token.namespace != namespace {
            return Err(AuthError::NamespaceMismatch {
                expected: namespace,
                actual: token.namespace,
            });
        }

        token.consume()?;

        self.audit_log.push(AuthAuditEntry {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
            namespace,
            operation: operation.to_string(),
            outcome: "consumed".to_string(),
            token_id: Some(token_id.to_string()),
        });

        Ok(())
    }

    pub fn get_token(&self, token_id: &str) -> Option<&HumanAuthToken> {
        self.tokens.iter().find(|t| t.id == token_id)
    }

    pub fn audit_log(&self) -> &[AuthAuditEntry] {
        &self.audit_log
    }

    pub fn clear_expired_tokens(&mut self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();
        self.tokens.retain(|t| t.expires_at > now || t.used);
    }
}

impl Default for AuthorizationGate {
    fn default() -> Self {
        Self::new()
    }
}
