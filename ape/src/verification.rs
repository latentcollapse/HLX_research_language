//! Pure Verification Module
//!
//! Provides pure verification logic using the ConscienceKernel without execution.
//! This is the core of Axiom's "verification-first" approach - you can verify
//! code properties without running anything.
//!
//! Key principle: verify() is pure, has no side effects, and is repeatable.

use std::collections::HashMap;
use crate::policy::Policy;
use crate::conscience::{ConscienceKernel, ConscienceQueryResult, EffectClass, QueryCategory};
use crate::error::{AxiomError, AxiomResult, ErrorKind};

/// Result of a verification operation
#[derive(Debug, Clone)]
pub struct Verdict {
    /// Whether the intent is allowed by the policy
    pub allowed: bool,
    /// Optional reason for denial (None if allowed)
    pub reason: Option<String>,
    /// Category of the policy decision
    pub category: QueryCategory,
    /// Human-readable guidance about the decision
    pub guidance: String,
}

impl Verdict {
    /// Returns true if the intent is allowed
    pub fn allowed(&self) -> bool {
        self.allowed
    }

    /// Returns the reason for denial, or None if allowed
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }

    /// Returns guidance about the decision
    pub fn guidance(&self) -> &str {
        &self.guidance
    }

    /// Returns the query category
    pub fn category(&self) -> &QueryCategory {
        &self.category
    }
}

/// Pure verifier - checks intents against policy without executing anything
pub struct Verifier {
    /// The conscience kernel for evaluation
    conscience: ConscienceKernel,
    /// Intent declarations from the policy
    intents: HashMap<String, crate::parser::ast::IntentDecl>,
}

impl Verifier {
    /// Create a new verifier from a policy
    ///
    /// # Example
    /// ```no_run
    /// use ape::policy::PolicyLoader;
    /// use ape::verification::Verifier;
    ///
    /// let policy = PolicyLoader::load_file("security.axm")?;
    /// let verifier = Verifier::new(policy);
    /// # Ok::<(), ape::error::AxiomError>(())
    /// ```
    pub fn new(policy: Policy) -> Self {
        Verifier {
            conscience: ConscienceKernel::new(),
            intents: policy.intents,
        }
    }

    /// Pure verification - check if an intent would be allowed
    ///
    /// This method is pure (no side effects), repeatable (same inputs = same output),
    /// and does not execute any code. It only evaluates the conscience kernel's
    /// policy rules.
    ///
    /// # Arguments
    /// * `intent_name` - The name of the intent to verify
    /// * `fields` - Key-value pairs representing the intent's parameters
    ///
    /// # Example
    /// ```no_run
    /// use ape::policy::PolicyLoader;
    /// use ape::verification::Verifier;
    ///
    /// let policy = PolicyLoader::load_file("security.axm")?;
    /// let verifier = Verifier::new(policy);
    ///
    /// let verdict = verifier.verify("WriteFile", &[
    ///     ("path", "/tmp/data.txt"),
    ///     ("content", "hello world"),
    /// ])?;
    ///
    /// if verdict.allowed() {
    ///     println!("Policy allows this write");
    /// } else {
    ///     println!("Policy denied: {}", verdict.reason().unwrap());
    /// }
    /// # Ok::<(), ape::error::AxiomError>(())
    /// ```
    pub fn verify(&self, intent_name: &str, fields: &[(&str, &str)]) -> AxiomResult<Verdict> {
        // Look up the intent declaration
        let intent = self.intents.get(intent_name).ok_or_else(|| AxiomError {
            kind: ErrorKind::UndefinedFunction,
            message: format!("Intent '{}' not found in policy", intent_name),
            span: None,
        })?;

        // Get the effect class from the intent declaration
        let effect = intent
            .clauses
            .effect
            .as_ref()
            .and_then(|e| EffectClass::from_str(e))
            .unwrap_or(EffectClass::Noop);

        // Convert field array to HashMap for conscience evaluation
        let mut field_map = HashMap::new();
        for (key, value) in fields {
            field_map.insert(key.to_string(), value.to_string());
        }

        // Use the conscience kernel's pure query() method
        // This is the key insight - query() already exists and is pure!
        let result: ConscienceQueryResult = self.conscience.query(intent_name, &effect, &field_map);

        // Convert ConscienceQueryResult to Verdict
        let verdict = Verdict {
            allowed: result.permitted,
            reason: result.deny_reason.clone(),
            category: result.category,
            guidance: result.guidance,
        };

        Ok(verdict)
    }

    /// Get a list of all intent names in the policy
    pub fn intent_names(&self) -> Vec<&str> {
        self.intents.keys().map(|s| s.as_str()).collect()
    }

    /// Check if an intent exists in the policy
    pub fn has_intent(&self, name: &str) -> bool {
        self.intents.contains_key(name)
    }

    /// Get the effect class for an intent
    pub fn intent_effect(&self, name: &str) -> Option<EffectClass> {
        self.intents
            .get(name)
            .and_then(|i| i.clauses.effect.as_ref())
            .and_then(|e| EffectClass::from_str(e))
    }

    /// Verify and log: pure verification for full Verdict, then append to the BLAKE3 audit chain.
    ///
    /// Calls `verify()` for the rich `Verdict` (deny_reason, category, guidance), then
    /// calls `conscience.evaluate()` to append a signed `IntentLogEntry` to the chain.
    /// The two-phase design keeps `verify()` pure while adding tamper-evident logging.
    pub fn verify_and_log(&mut self, intent_name: &str, fields: &[(&str, &str)]) -> AxiomResult<Verdict> {
        // Phase 1: pure verification — get full Verdict with deny_reason/category/guidance
        let verdict = self.verify(intent_name, fields)?;

        // Phase 2: logging — resolve effect + field_map and append to audit chain
        let effect = self.intents
            .get(intent_name)
            .and_then(|i| i.clauses.effect.as_ref())
            .and_then(|e| EffectClass::from_str(e))
            .unwrap_or(EffectClass::Noop);

        let mut field_map = HashMap::new();
        for (key, value) in fields {
            field_map.insert(key.to_string(), value.to_string());
        }

        self.conscience.evaluate(intent_name, &effect, &field_map);

        Ok(verdict)
    }

    /// Walk the BLAKE3 audit chain and verify every entry's pre_hash links correctly.
    /// Returns `Ok(())` if the chain is intact, or an `Err` describing the broken link.
    pub fn verify_audit_chain(&self) -> Result<(), String> {
        self.conscience.verify_audit_chain()
    }

    /// Return the number of intent entries currently in the audit log.
    pub fn audit_log_len(&self) -> usize {
        self.conscience.audit_log_len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::PolicyLoader;

    #[test]
    fn test_verify_safe_read() {
        let source = r#"
            module test {
                intent ReadFile {
                    takes: path: String;
                    gives: content: String;
                    effect: READ;
                    conscience: path_safety;
                }
            }
        "#;

        let policy = PolicyLoader::load_source(source).unwrap();
        let verifier = Verifier::new(policy);

        // Safe path should be allowed (baseline_allow covers READ)
        let verdict = verifier.verify("ReadFile", &[("path", "/tmp/data.txt")]).unwrap();
        assert!(verdict.allowed());
    }

    #[test]
    fn test_verify_dangerous_path_denied() {
        let source = r#"
            module test {
                intent ReadFile {
                    takes: path: String;
                    gives: content: String;
                    effect: READ;
                    conscience: path_safety;
                }
            }
        "#;

        let policy = PolicyLoader::load_source(source).unwrap();
        let verifier = Verifier::new(policy);

        // Dangerous path should be denied by path_safety predicate
        let verdict = verifier.verify("ReadFile", &[("path", "/etc/shadow")]).unwrap();
        assert!(!verdict.allowed());
        assert!(verdict.reason().is_some());
    }

    #[test]
    fn test_verify_write_with_conscience_predicates() {
        let source = r#"
            module test {
                intent WriteFile {
                    takes: path: String, content: String;
                    gives: success: bool;
                    effect: WRITE;
                    conscience: path_safety, no_exfiltrate;
                }
            }
        "#;

        let policy = PolicyLoader::load_source(source).unwrap();
        let verifier = Verifier::new(policy);

        // Write to /tmp should pass both path_safety and no_exfiltrate
        let verdict = verifier
            .verify("WriteFile", &[("path", "/tmp/test.txt"), ("content", "hello")])
            .unwrap();
        // Note: This passes because /tmp is not in the denied paths list
        // and is not a network path (no_exfiltrate passes)
        assert!(verdict.allowed());

        // Write to dangerous path should be denied
        let verdict = verifier
            .verify("WriteFile", &[("path", "/etc/passwd"), ("content", "hello")])
            .unwrap();
        assert!(!verdict.allowed());
    }

    #[test]
    fn test_verify_network_exfiltration() {
        let source = r#"
            module test {
                intent SendData {
                    takes: url: String, data: String;
                    gives: success: bool;
                    effect: NETWORK;
                    conscience: no_exfiltrate;
                }
            }
        "#;

        let policy = PolicyLoader::load_source(source).unwrap();
        let verifier = Verifier::new(policy);

        // Undeclared channel should be denied by no_exfiltrate
        let verdict = verifier
            .verify("SendData", &[("url", "http://evil.com"), ("data", "secrets")])
            .unwrap();
        assert!(!verdict.allowed());
    }

    #[test]
    fn test_verify_unknown_intent() {
        let source = r#"
            module test {
                intent KnownIntent {
                    takes: x: i64;
                    gives: y: i64;
                    effect: NOOP;
                }
            }
        "#;

        let policy = PolicyLoader::load_source(source).unwrap();
        let verifier = Verifier::new(policy);

        // Unknown intent should return an error
        let result = verifier.verify("UnknownIntent", &[("x", "42")]);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_is_pure() {
        // Verification should be repeatable - same inputs = same outputs
        let source = r#"
            module test {
                intent ReadFile {
                    takes: path: String;
                    gives: content: String;
                    effect: READ;
                    conscience: path_safety;
                }
            }
        "#;

        let policy = PolicyLoader::load_source(source).unwrap();
        let verifier = Verifier::new(policy);

        let fields = [("path", "/etc/shadow")];

        // Call verify multiple times with same inputs
        let verdict1 = verifier.verify("ReadFile", &fields).unwrap();
        let verdict2 = verifier.verify("ReadFile", &fields).unwrap();
        let verdict3 = verifier.verify("ReadFile", &fields).unwrap();

        // All results should be identical
        assert_eq!(verdict1.allowed(), verdict2.allowed());
        assert_eq!(verdict2.allowed(), verdict3.allowed());
        assert_eq!(verdict1.reason(), verdict2.reason());
        assert_eq!(verdict2.reason(), verdict3.reason());
    }

    #[test]
    fn test_verifier_introspection() {
        let source = r#"
            module test {
                intent A { takes: x: i64; gives: y: i64; effect: NOOP; }
                intent B { takes: x: i64; gives: y: i64; effect: READ; }
            }
        "#;

        let policy = PolicyLoader::load_source(source).unwrap();
        let verifier = Verifier::new(policy);

        assert!(verifier.has_intent("A"));
        assert!(verifier.has_intent("B"));
        assert!(!verifier.has_intent("C"));

        assert_eq!(verifier.intent_effect("A"), Some(EffectClass::Noop));
        assert_eq!(verifier.intent_effect("B"), Some(EffectClass::Read));

        let mut names = verifier.intent_names();
        names.sort();
        assert_eq!(names, vec!["A", "B"]);
    }
}
