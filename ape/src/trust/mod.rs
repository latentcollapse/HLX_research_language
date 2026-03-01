//! Trust Architecture — Provenance chaining and trust algebra (Part VII)
//!
//! Every value carries an implicit trust tag. Trust is algebra, not reputation.
//! Trust changes come from formal verification (promotion) or governance (downgrade),
//! never from heuristics or time-based decay.

/// The four trust levels, ordered from highest (T0) to lowest (T3)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TrustLevel {
    /// T0: Created by Axiom, never left the system
    TrustedInternal = 0,
    /// T1: Passed through `do Verify`
    TrustedVerified = 1,
    /// T2: Returned by intent contacting external systems
    UntrustedExternal = 2,
    /// T3: Derived from UNTRUSTED_EXTERNAL via any transform
    UntrustedTainted = 3,
}

impl TrustLevel {
    /// Trust algebra: trust(output) = max(trust(input_0), trust(input_1), ...)
    /// Max selects LEAST trusted (highest numeric level). Taint is infectious.
    pub fn combine(levels: &[TrustLevel]) -> TrustLevel {
        levels.iter().copied().max().unwrap_or(TrustLevel::TrustedInternal)
    }

    /// Can this trust level be promoted? Only via `do Verify` (P18)
    pub fn can_promote(&self) -> bool {
        !matches!(self, TrustLevel::TrustedInternal)
    }

    /// Promote trust level by one step (only via Verify intent)
    pub fn promote(self) -> TrustLevel {
        match self {
            TrustLevel::TrustedInternal => TrustLevel::TrustedInternal,
            TrustLevel::TrustedVerified => TrustLevel::TrustedInternal,
            TrustLevel::UntrustedExternal => TrustLevel::TrustedVerified,
            TrustLevel::UntrustedTainted => TrustLevel::UntrustedExternal,
        }
    }

    /// Infect: any operation on tainted data produces tainted output
    pub fn infect(self, other: TrustLevel) -> TrustLevel {
        // max selects least trusted
        if self as u8 > other as u8 {
            self
        } else {
            other
        }
    }

    pub fn is_trusted(&self) -> bool {
        matches!(self, TrustLevel::TrustedInternal | TrustLevel::TrustedVerified)
    }

    pub fn is_untrusted(&self) -> bool {
        !self.is_trusted()
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            TrustLevel::TrustedInternal => "TRUSTED_INTERNAL",
            TrustLevel::TrustedVerified => "TRUSTED_VERIFIED",
            TrustLevel::UntrustedExternal => "UNTRUSTED_EXTERNAL",
            TrustLevel::UntrustedTainted => "UNTRUSTED_TAINTED",
        }
    }
}

impl std::fmt::Display for TrustLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A value tagged with its trust provenance
#[derive(Debug, Clone)]
pub struct Trusted<T> {
    pub value: T,
    pub trust: TrustLevel,
    /// Chain of trust transitions for this value
    pub provenance_chain: Vec<TrustTransition>,
}

impl<T> Trusted<T> {
    pub fn new_internal(value: T) -> Self {
        Trusted {
            value,
            trust: TrustLevel::TrustedInternal,
            provenance_chain: vec![TrustTransition {
                from: TrustLevel::TrustedInternal,
                to: TrustLevel::TrustedInternal,
                action: TransitionAction::Genesis,
            }],
        }
    }

    pub fn new_external(value: T) -> Self {
        Trusted {
            value,
            trust: TrustLevel::UntrustedExternal,
            provenance_chain: vec![TrustTransition {
                from: TrustLevel::UntrustedExternal,
                to: TrustLevel::UntrustedExternal,
                action: TransitionAction::ExternalInput,
            }],
        }
    }

    /// Apply the trust transition function: T(n+1) = f(T(n), Action)
    pub fn with_transition(mut self, new_trust: TrustLevel, action: TransitionAction) -> Self {
        self.provenance_chain.push(TrustTransition {
            from: self.trust,
            to: new_trust,
            action,
        });
        self.trust = new_trust;
        self
    }
}

/// A recorded trust transition
#[derive(Debug, Clone)]
pub struct TrustTransition {
    pub from: TrustLevel,
    pub to: TrustLevel,
    pub action: TransitionAction,
}

/// What caused a trust transition
#[derive(Debug, Clone)]
pub enum TransitionAction {
    /// Value created within Axiom
    Genesis,
    /// Value came from external system (via intent)
    ExternalInput,
    /// Promotion via `do Verify` (P18 — sole promotion path)
    Verify,
    /// Downgrade via governance action (Gate 3)
    GovernanceDowngrade(String),
    /// Infection via algebra (combined with tainted input)
    TaintInfection,
    /// Derived value (any computation on the value)
    Derived,
}

/// Trust context for tracking provenance through computation
#[derive(Debug)]
pub struct TrustTracker {
    /// Map from variable name to trust level
    trust_tags: std::collections::HashMap<String, TrustLevel>,
}

impl TrustTracker {
    pub fn new() -> Self {
        TrustTracker {
            trust_tags: std::collections::HashMap::new(),
        }
    }

    /// Set the trust level for a variable
    pub fn set(&mut self, name: &str, trust: TrustLevel) {
        self.trust_tags.insert(name.to_string(), trust);
    }

    /// Get the trust level for a variable (defaults to TrustedInternal for literals)
    pub fn get(&self, name: &str) -> TrustLevel {
        self.trust_tags
            .get(name)
            .copied()
            .unwrap_or(TrustLevel::TrustedInternal)
    }

    /// Compute the trust level for an expression that combines multiple inputs
    pub fn combine_inputs(&self, names: &[&str]) -> TrustLevel {
        let levels: Vec<TrustLevel> = names.iter().map(|n| self.get(n)).collect();
        TrustLevel::combine(&levels)
    }

    /// Record that a value was verified (sole promotion path).
    /// M7: Now returns Result — caller must prove a Verify intent actually executed
    /// by providing the intent execution receipt hash.
    pub fn verify(&mut self, name: &str, verify_receipt: &str) -> Result<TrustLevel, String> {
        if verify_receipt.is_empty() {
            return Err(format!(
                "Trust promotion for '{}' requires a non-empty verify_receipt \
                 (proof that `do Verify` actually executed)",
                name
            ));
        }
        let current = self.get(name);
        let promoted = current.promote();
        self.set(name, promoted);
        Ok(promoted)
    }

    /// Record that a value came from an external intent
    pub fn mark_external(&mut self, name: &str) {
        self.set(name, TrustLevel::UntrustedExternal);
    }

    /// Record that a value was derived from tainted data
    pub fn mark_tainted(&mut self, name: &str) {
        self.set(name, TrustLevel::UntrustedTainted);
    }

    /// Check if using an untrusted value in a trust-requiring context
    pub fn check_trust_boundary(&self, name: &str, required: TrustLevel) -> Result<(), String> {
        let actual = self.get(name);
        if actual as u8 > required as u8 {
            Err(format!(
                "Trust boundary violation: '{}' has trust level {} but {} is required. Use `do Verify` to promote.",
                name, actual, required
            ))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trust_algebra_taint_infectious() {
        // "Three pure functions on untrusted data = still untrusted data"
        let levels = vec![
            TrustLevel::TrustedInternal,
            TrustLevel::TrustedInternal,
            TrustLevel::UntrustedTainted,
        ];
        assert_eq!(TrustLevel::combine(&levels), TrustLevel::UntrustedTainted);
    }

    #[test]
    fn test_trust_promotion_via_verify() {
        let t = TrustLevel::UntrustedExternal;
        let promoted = t.promote();
        assert_eq!(promoted, TrustLevel::TrustedVerified);
    }

    #[test]
    fn test_trust_no_self_promotion() {
        let t = TrustLevel::TrustedInternal;
        assert_eq!(t.promote(), TrustLevel::TrustedInternal);
    }

    #[test]
    fn test_trust_tracker() {
        let mut tracker = TrustTracker::new();
        tracker.set("x", TrustLevel::TrustedInternal);
        tracker.mark_external("api_data");

        // Combining trusted + untrusted = untrusted
        let combined = tracker.combine_inputs(&["x", "api_data"]);
        assert_eq!(combined, TrustLevel::UntrustedExternal);

        // M7: Verify without receipt should fail
        assert!(tracker.verify("api_data", "").is_err());

        // Verify with receipt promotes
        let promoted = tracker.verify("api_data", "verify_receipt_hash_abc").unwrap();
        assert_eq!(promoted, TrustLevel::TrustedVerified);
    }
}
