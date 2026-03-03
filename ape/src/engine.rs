//! Axiom Engine - Main Embedder API
//!
//! This is the "SQLite moment" for Axiom - the embedder-friendly API that makes
//! it trivial to verify agent code before execution.
//!
//! Just 5 lines to get started:
//! ```no_run
//! use ape::AxiomEngine;
//!
//! let engine = AxiomEngine::from_file("policy.axm")?;
//! let verdict = engine.verify("WriteFile", &[("path", "/tmp/test.txt")])?;
//! if verdict.allowed() {
//!     // Your code runs
//! }
//! # Ok::<(), ape::error::AxiomError>(())
//! ```

use std::path::Path;
use std::collections::HashMap;
use crate::parser::ast::TypeExpr;
use crate::policy::{Policy, PolicyLoader};
use crate::verification::{Verifier, Verdict};
use crate::interpreter::{Interpreter, value::Value};
use crate::error::{AxiomResult, AxiomError, ErrorKind};

/// Main Axiom engine - provides verification and optional execution
///
/// The engine is designed with verification-first architecture:
/// - Loading a policy is fast and lightweight
/// - Verification is pure and requires no setup
/// - Execution (via evaluate()) is optional and lazy
///
/// # Example
/// ```no_run
/// use ape::AxiomEngine;
///
/// // Load policy (fast, no execution setup)
/// let engine = AxiomEngine::from_file("security.axm")?;
///
/// // Pure verification (no side effects)
/// let verdict = engine.verify("ReadFile", &[("path", "/tmp/data.txt")])?;
///
/// if verdict.allowed() {
///     println!("Policy allows this operation");
/// }
/// # Ok::<(), ape::error::AxiomError>(())
/// ```
pub struct AxiomEngine {
    /// The loaded policy
    policy: Policy,
    /// The verifier for pure verification
    verifier: Verifier,
    /// Optional interpreter - only initialized if evaluate() is called
    interpreter: Option<Box<Interpreter>>,
}

/// Result of executing an intent (verify + run)
#[derive(Debug)]
pub struct ExecutionResult {
    /// The verification verdict
    pub verdict: Verdict,
    /// The value returned by the intent execution
    pub value: Value,
    /// Log output from execution
    pub logs: Vec<String>,
}

/// Signature of an intent for introspection
#[derive(Debug, Clone)]
pub struct IntentSignature {
    /// Intent name
    pub name: String,
    /// Input parameters: (name, type)
    pub takes: Vec<(String, String)>,
    /// Output parameters: (name, type)
    pub gives: Vec<(String, String)>,
    /// Effect class (READ, WRITE, NETWORK, etc.)
    pub effect: String,
    /// Conscience predicates that apply
    pub conscience: Vec<String>,
}

impl AxiomEngine {
    /// Load an Axiom policy from a file
    ///
    /// This is fast and lightweight - it only parses and type-checks the policy,
    /// without setting up any execution environment.
    ///
    /// # Example
    /// ```no_run
    /// use ape::AxiomEngine;
    ///
    /// let engine = AxiomEngine::from_file("security.axm")?;
    /// # Ok::<(), ape::error::AxiomError>(())
    /// ```
    pub fn from_file(path: impl AsRef<Path>) -> AxiomResult<Self> {
        let policy = PolicyLoader::load_file(path)?;
        let verifier = Verifier::new(policy.clone());

        Ok(AxiomEngine {
            policy,
            verifier,
            interpreter: None,
        })
    }

    /// Load an Axiom policy from source code
    ///
    /// # Example
    /// ```no_run
    /// use ape::AxiomEngine;
    ///
    /// let source = r#"
    ///     module security {
    ///         intent ReadFile {
    ///             takes: path: String;
    ///             gives: content: String;
    ///             effect: READ;
    ///             conscience: path_safety;
    ///         }
    ///     }
    /// "#;
    ///
    /// let engine = AxiomEngine::from_source(source)?;
    /// # Ok::<(), ape::error::AxiomError>(())
    /// ```
    pub fn from_source(source: &str) -> AxiomResult<Self> {
        let policy = PolicyLoader::load_source(source)?;
        let verifier = Verifier::new(policy.clone());

        Ok(AxiomEngine {
            policy,
            verifier,
            interpreter: None,
        })
    }

    /// Pure verification - check if an intent would be allowed
    ///
    /// This is the core operation of the verification-first architecture.
    /// It's pure (no side effects), fast, and repeatable.
    ///
    /// # Arguments
    /// * `intent_name` - The name of the intent to verify
    /// * `fields` - Key-value pairs representing the intent's parameters
    ///
    /// # Returns
    /// A `Verdict` indicating whether the intent is allowed and why
    ///
    /// # Example
    /// ```no_run
    /// use ape::AxiomEngine;
    ///
    /// let engine = AxiomEngine::from_file("policy.axm")?;
    ///
    /// let verdict = engine.verify("WriteFile", &[
    ///     ("path", "/tmp/data.txt"),
    ///     ("content", "hello"),
    /// ])?;
    ///
    /// if verdict.allowed() {
    ///     // Safe to execute
    ///     std::fs::write("/tmp/data.txt", "hello")?;
    /// } else {
    ///     println!("Denied: {}", verdict.reason().unwrap());
    /// }
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn verify(&self, intent_name: &str, fields: &[(&str, &str)]) -> AxiomResult<Verdict> {
        self.verifier.verify(intent_name, fields)
    }

    /// Verify and log to the BLAKE3 audit chain.
    ///
    /// Identical to `verify()` from the caller's perspective, but also appends a
    /// signed `IntentLogEntry` to the tamper-evident chain so every governance
    /// decision is auditable after the fact. Requires `&mut self`.
    ///
    /// Use this wherever you would call `verify()` in production code.
    /// Use `verify()` only when you explicitly need a pure, side-effect-free check.
    pub fn verify_and_log(&mut self, intent_name: &str, fields: &[(&str, &str)]) -> AxiomResult<Verdict> {
        self.verifier.verify_and_log(intent_name, fields)
    }

    /// Walk the BLAKE3 audit chain and verify every `pre_hash` link is intact.
    ///
    /// Returns `Ok(())` if the chain is unbroken, or `Err(description)` naming the
    /// first broken link. Call this at program exit to confirm no entries were
    /// replayed, reordered, or tampered with during the run.
    pub fn verify_audit_chain(&self) -> Result<(), String> {
        self.verifier.verify_audit_chain()
    }

    /// Return the number of intent entries currently in the audit log.
    pub fn audit_log_len(&self) -> usize {
        self.verifier.audit_log_len()
    }

    /// Evaluate an intent - verify AND execute
    ///
    /// This is the advanced operation that both verifies and executes the intent.
    /// The interpreter is lazily initialized on the first call to this method.
    ///
    /// # Arguments
    /// * `intent_name` - The name of the intent to evaluate
    /// * `fields` - Key-value pairs with actual Values (not just strings)
    ///
    /// # Returns
    /// An `ExecutionResult` containing the verdict, return value, and logs
    ///
    /// # Example
    /// ```no_run
    /// use ape::{AxiomEngine, Value};
    ///
    /// let mut engine = AxiomEngine::from_file("policy.axm")?;
    ///
    /// let result = engine.evaluate("ProcessData", &[
    ///     ("input", Value::String("test".into())),
    /// ])?;
    ///
    /// println!("Result: {:?}", result.value);
    /// # Ok::<(), ape::error::AxiomError>(())
    /// ```
    pub fn evaluate(
        &mut self,
        intent_name: &str,
        fields: &[(&str, Value)],
    ) -> AxiomResult<ExecutionResult> {
        // First, do pure verification (with string representations)
        let string_fields: Vec<(&str, String)> = fields
            .iter()
            .map(|(k, v)| (*k, format!("{}", v)))
            .collect();
        let string_fields_ref: Vec<(&str, &str)> = string_fields
            .iter()
            .map(|(k, v)| (*k, v.as_str()))
            .collect();

        let verdict = self.verify(intent_name, &string_fields_ref)?;

        if !verdict.allowed() {
            return Err(AxiomError {
                kind: ErrorKind::HaltConscience,
                message: format!(
                    "Intent '{}' denied by policy: {}",
                    intent_name,
                    verdict.reason().unwrap_or("unknown reason")
                ),
                span: None,
            });
        }

        // Lazy initialization of interpreter
        if self.interpreter.is_none() {
            let interp = Interpreter::new();
            // Load the policy declarations into the interpreter
            // This is a simplified version - full implementation would need
            // to actually execute the policy source through the interpreter
            // For now, we'll just initialize an empty interpreter
            self.interpreter = Some(Box::new(interp));
        }

        // Execute through the interpreter
        let interp = self.interpreter.as_mut().unwrap();

        // Convert fields to HashMap for interpreter
        let mut field_map = HashMap::new();
        for (key, value) in fields {
            field_map.insert(key.to_string(), value.clone());
        }

        // Note: This is a simplified version. A full implementation would need
        // to properly invoke the intent through the interpreter's do_intent method.
        // For now, we'll return a placeholder result.
        let logs = interp.output.clone();

        Ok(ExecutionResult {
            verdict,
            value: Value::Bool(true), // Placeholder
            logs,
        })
    }

    /// Get a list of all intent names in the policy
    pub fn intents(&self) -> Vec<&str> {
        self.policy.intent_names()
    }

    /// Render a TypeExpr as a clean user-facing string (no span info)
    fn type_name(ty: &TypeExpr) -> String {
        match ty {
            TypeExpr::Named(name, _) => name.clone(),
            TypeExpr::Array(inner, _) => format!("[{}]", Self::type_name(inner)),
            TypeExpr::Map(k, v, _) => format!("Map<{}, {}>", Self::type_name(k), Self::type_name(v)),
            TypeExpr::Tensor(dims, _) => {
                let parts: Vec<String> = dims.iter().map(|d| match d {
                    crate::parser::ast::TensorDim::Fixed(n) => n.to_string(),
                    crate::parser::ast::TensorDim::Wildcard => "?".to_string(),
                    crate::parser::ast::TensorDim::Named(s) => s.clone(),
                }).collect();
                format!("Tensor[{}]", parts.join(", "))
            }
            TypeExpr::Sealed(inner, _) => format!("Sealed<{}>", Self::type_name(inner)),
        }
    }

    /// Get the signature of an intent
    ///
    /// Returns information about the intent's interface for introspection.
    pub fn intent_signature(&self, name: &str) -> Option<IntentSignature> {
        let intent = self.policy.get_intent(name)?;

        let takes = intent
            .clauses
            .takes
            .iter()
            .map(|p| (p.name.clone(), Self::type_name(&p.ty)))
            .collect();

        let gives = intent
            .clauses
            .gives
            .iter()
            .map(|p| (p.name.clone(), Self::type_name(&p.ty)))
            .collect();

        let effect = intent
            .clauses
            .effect
            .clone()
            .unwrap_or_else(|| "NOOP".to_string());

        let conscience = intent.clauses.conscience.clone();

        Some(IntentSignature {
            name: name.to_string(),
            takes,
            gives,
            effect,
            conscience,
        })
    }

    /// Check if an intent exists in the policy
    pub fn has_intent(&self, name: &str) -> bool {
        self.verifier.has_intent(name)
    }

    /// Check if the interpreter has been initialized
    ///
    /// Used for testing to verify lazy initialization
    pub fn is_interpreter_initialized(&self) -> bool {
        self.interpreter.is_some()
    }
}

// Note: Verdict is already imported and re-exported via the public API
// Value is already imported and re-exported via the public API

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_from_source() {
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

        let engine = AxiomEngine::from_source(source).unwrap();
        assert!(engine.has_intent("ReadFile"));
        assert!(!engine.has_intent("UnknownIntent"));
    }

    #[test]
    fn test_verify_without_interpreter() {
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

        let engine = AxiomEngine::from_source(source).unwrap();

        // Verify should work without initializing the interpreter
        let verdict = engine.verify("ReadFile", &[("path", "/tmp/test.txt")]).unwrap();
        assert!(verdict.allowed());

        // Interpreter should still be uninitialized
        assert!(!engine.is_interpreter_initialized());
    }

    #[test]
    fn test_verify_dangerous_path() {
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

        let engine = AxiomEngine::from_source(source).unwrap();

        let verdict = engine.verify("ReadFile", &[("path", "/etc/shadow")]).unwrap();
        assert!(!verdict.allowed());
        assert!(verdict.reason().is_some());
    }

    #[test]
    fn test_intent_signature() {
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

        let engine = AxiomEngine::from_source(source).unwrap();
        let sig = engine.intent_signature("WriteFile").unwrap();

        assert_eq!(sig.name, "WriteFile");
        assert_eq!(sig.takes.len(), 2);
        assert_eq!(sig.gives.len(), 1);
        assert_eq!(sig.effect, "WRITE");
        assert_eq!(sig.conscience.len(), 2);
    }

    #[test]
    fn test_list_intents() {
        let source = r#"
            module test {
                intent A { takes: x: i64; gives: y: i64; effect: NOOP; }
                intent B { takes: x: i64; gives: y: i64; effect: NOOP; }
                intent C { takes: x: i64; gives: y: i64; effect: NOOP; }
            }
        "#;

        let engine = AxiomEngine::from_source(source).unwrap();
        let mut intents = engine.intents();
        intents.sort();
        assert_eq!(intents, vec!["A", "B", "C"]);
    }

    #[test]
    fn test_verify_is_repeatable() {
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

        let engine = AxiomEngine::from_source(source).unwrap();
        let fields = [("path", "/etc/shadow")];

        // Multiple verifications should produce identical results
        let v1 = engine.verify("ReadFile", &fields).unwrap();
        let v2 = engine.verify("ReadFile", &fields).unwrap();
        let v3 = engine.verify("ReadFile", &fields).unwrap();

        assert_eq!(v1.allowed(), v2.allowed());
        assert_eq!(v2.allowed(), v3.allowed());
        assert!(!v1.allowed()); // Should be denied
    }
}
