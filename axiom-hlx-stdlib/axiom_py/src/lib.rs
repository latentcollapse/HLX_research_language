//! Axiom Python Bindings
//!
//! Provides Python access to Axiom's verification-first policy engine.
//! Built with PyO3 for seamless integration with async agent frameworks.
//!
//! # Example
//!
//! ```python
//! from axiom import AxiomEngine
//!
//! # Sync usage
//! engine = AxiomEngine.from_file("policy.axm")
//! verdict = engine.verify("WriteFile", {"path": "/tmp/test.txt"})
//!
//! # Async usage (for agent frameworks)
//! verdict = await engine.verify_async("WriteFile", {"path": "/tmp/test.txt"})
//! ```

use axiom_lang::{AxiomEngine, IntentSignature, Verdict};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::sync::Arc;

/// Result of a verification operation
///
/// Attributes:
///     allowed (bool): Whether the intent is permitted by the policy
///     reason (str | None): Reason for denial, None if allowed
///     guidance (str): Human-readable guidance about the decision
///     category (str): Category of the policy decision
#[pyclass(name = "Verdict")]
pub struct PyVerdict {
    inner: Verdict,
}

#[pymethods]
impl PyVerdict {
    #[getter]
    fn allowed(&self) -> bool {
        self.inner.allowed()
    }

    #[getter]
    fn reason(&self) -> Option<&str> {
        self.inner.reason()
    }

    #[getter]
    fn guidance(&self) -> &str {
        self.inner.guidance()
    }

    #[getter]
    fn category(&self) -> String {
        use axiom_lang::conscience::QueryCategory;
        match self.inner.category() {
            QueryCategory::ChannelPolicy => "ChannelPolicy".to_string(),
            QueryCategory::ResourcePolicy => "ResourcePolicy".to_string(),
            QueryCategory::IrreversibleAction => "IrreversibleAction".to_string(),
            QueryCategory::ConscienceCore => "ConscienceCore".to_string(),
        }
    }

    fn __repr__(&self) -> String {
        if self.inner.allowed() {
            format!("Verdict(allowed=True)")
        } else {
            format!(
                "Verdict(allowed=False, reason={:?})",
                self.inner.reason().unwrap_or("unknown")
            )
        }
    }

    fn __bool__(&self) -> bool {
        self.inner.allowed()
    }
}

impl From<Verdict> for PyVerdict {
    fn from(verdict: Verdict) -> Self {
        PyVerdict { inner: verdict }
    }
}

/// Signature of an intent for introspection
///
/// Attributes:
///     name (str): Intent name
///     takes (list[tuple[str, str]]): Input parameters as (name, type) pairs
///     gives (list[tuple[str, str]]): Output parameters as (name, type) pairs
///     effect (str): Effect class (READ, WRITE, NETWORK, etc.)
///     conscience (list[str]): Conscience predicates that apply
#[pyclass(name = "IntentSignature")]
pub struct PyIntentSignature {
    inner: IntentSignature,
}

#[pymethods]
impl PyIntentSignature {
    #[getter]
    fn name(&self) -> &str {
        &self.inner.name
    }

    #[getter]
    fn takes(&self) -> Vec<(String, String)> {
        self.inner.takes.clone()
    }

    #[getter]
    fn gives(&self) -> Vec<(String, String)> {
        self.inner.gives.clone()
    }

    #[getter]
    fn effect(&self) -> &str {
        &self.inner.effect
    }

    #[getter]
    fn conscience(&self) -> Vec<String> {
        self.inner.conscience.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "IntentSignature(name={}, effect={}, predicates={})",
            self.inner.name,
            self.inner.effect,
            self.inner.conscience.join(", ")
        )
    }
}

impl From<IntentSignature> for PyIntentSignature {
    fn from(sig: IntentSignature) -> Self {
        PyIntentSignature { inner: sig }
    }
}

/// Axiom Policy Engine
///
/// The main interface for loading policies and verifying intents.
/// Supports both sync and async usage for integration with agent frameworks.
///
/// Example:
///     ```python
///     from axiom import AxiomEngine
///
///     # Load from file
///     engine = AxiomEngine.from_file("security.axm")
///
///     # Or load from source string
///     engine = AxiomEngine.from_source('''
///         module security {
///             intent ReadFile {
///                 takes: path: String;
///                 gives: content: String;
///                 effect: READ;
///                 conscience: path_safety;
///             }
///         }
///     ''')
///
///     # Sync verification
///     verdict = engine.verify("ReadFile", {"path": "/tmp/data.txt"})
///
///     # Async verification (non-blocking for agent frameworks)
///     verdict = await engine.verify_async("ReadFile", {"path": "/tmp/data.txt"})
///     ```
#[pyclass(name = "AxiomEngine")]
pub struct PyAxiomEngine {
    inner: Arc<AxiomEngine>,
}

fn extract_fields(fields: &PyDict) -> PyResult<Vec<(String, String)>> {
    fields
        .iter()
        .map(|(k, v)| {
            let key: String = k.extract().map_err(|e| {
                PyValueError::new_err(format!("Field key must be string: {}", e))
            })?;
            let value: String = v.extract().map_err(|e| {
                PyValueError::new_err(format!("Field value must be string: {}", e))
            })?;
            Ok((key, value))
        })
        .collect()
}

#[pymethods]
impl PyAxiomEngine {
    /// Load an Axiom policy from a file
    ///
    /// Args:
    ///     path: Path to the .axm policy file
    ///
    /// Returns:
    ///     AxiomEngine instance
    ///
    /// Raises:
    ///     ValueError: If the policy file cannot be loaded
    #[staticmethod]
    fn from_file(path: &str) -> PyResult<Self> {
        let engine = AxiomEngine::from_file(path)
            .map_err(|e| PyValueError::new_err(format!("Failed to load policy: {}", e)))?;
        Ok(PyAxiomEngine {
            inner: Arc::new(engine),
        })
    }

    /// Load an Axiom policy from source code
    ///
    /// Args:
    ///     source: Axiom policy source code as a string
    ///
    /// Returns:
    ///     AxiomEngine instance
    ///
    /// Raises:
    ///     ValueError: If the policy source is invalid
    #[staticmethod]
    fn from_source(source: &str) -> PyResult<Self> {
        let engine = AxiomEngine::from_source(source)
            .map_err(|e| PyValueError::new_err(format!("Failed to parse policy: {}", e)))?;
        Ok(PyAxiomEngine {
            inner: Arc::new(engine),
        })
    }

    /// Load an Axiom policy from a file (async)
    ///
    /// Non-blocking file I/O for async contexts.
    ///
    /// Args:
    ///     path: Path to the .axm policy file
    ///
    /// Returns:
    ///     AxiomEngine instance
    #[staticmethod]
    fn from_file_async<'py>(py: Python<'py>, path: String) -> PyResult<&'py PyAny> {
        pyo3_asyncio::tokio::future_into_py(py, async move {
            let engine = tokio::task::spawn_blocking(move || {
                AxiomEngine::from_file(&path)
                    .map_err(|e| PyValueError::new_err(format!("Failed to load policy: {}", e)))
            })
            .await
            .map_err(|e| PyRuntimeError::new_err(format!("Task join error: {}", e)))??;

            Ok(PyAxiomEngine {
                inner: Arc::new(engine),
            })
        })
    }

    /// Load an Axiom policy from source code (async)
    ///
    /// Non-blocking parse for async contexts.
    ///
    /// Args:
    ///     source: Axiom policy source code as a string
    ///
    /// Returns:
    ///     AxiomEngine instance
    #[staticmethod]
    fn from_source_async<'py>(py: Python<'py>, source: String) -> PyResult<&'py PyAny> {
        pyo3_asyncio::tokio::future_into_py(py, async move {
            let engine = tokio::task::spawn_blocking(move || {
                AxiomEngine::from_source(&source)
                    .map_err(|e| PyValueError::new_err(format!("Failed to parse policy: {}", e)))
            })
            .await
            .map_err(|e| PyRuntimeError::new_err(format!("Task join error: {}", e)))??;

            Ok(PyAxiomEngine {
                inner: Arc::new(engine),
            })
        })
    }

    /// Verify an intent against the policy (sync)
    ///
    /// This is a pure operation - no side effects, fully deterministic.
    ///
    /// Args:
    ///     intent_name: Name of the intent to verify
    ///     fields: Dictionary of parameter names to values
    ///
    /// Returns:
    ///     Verdict indicating whether the intent is allowed
    ///
    /// Raises:
    ///     RuntimeError: If verification fails unexpectedly
    fn verify(&self, intent_name: &str, fields: &PyDict) -> PyResult<PyVerdict> {
        let field_pairs = extract_fields(fields)?;

        let field_refs: Vec<(&str, &str)> = field_pairs
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

        let verdict = self
            .inner
            .verify(intent_name, &field_refs)
            .map_err(|e| PyRuntimeError::new_err(format!("Verification failed: {}", e)))?;

        Ok(PyVerdict::from(verdict))
    }

    /// Verify an intent against the policy (async)
    ///
    /// Non-blocking verification for async agent frameworks.
    /// Offloads CPU-bound verification to a thread pool.
    ///
    /// Args:
    ///     intent_name: Name of the intent to verify
    ///     fields: Dictionary of parameter names to values
    ///
    /// Returns:
    ///     Coroutine that resolves to Verdict
    fn verify_async<'py>(&self, py: Python<'py>, intent_name: String, fields: &PyDict) -> PyResult<&'py PyAny> {
        let field_pairs = extract_fields(fields)?;
        let engine = Arc::clone(&self.inner);

        pyo3_asyncio::tokio::future_into_py(py, async move {
            let verdict = tokio::task::spawn_blocking(move || {
                let field_refs: Vec<(&str, &str)> = field_pairs
                    .iter()
                    .map(|(k, v)| (k.as_str(), v.as_str()))
                    .collect();

                engine
                    .verify(&intent_name, &field_refs)
                    .map_err(|e| PyRuntimeError::new_err(format!("Verification failed: {}", e)))
            })
            .await
            .map_err(|e| PyRuntimeError::new_err(format!("Task join error: {}", e)))??;

            Ok(PyVerdict::from(verdict))
        })
    }

    /// Get a list of all intent names in the policy
    ///
    /// Returns:
    ///     List of intent name strings
    fn intents(&self) -> Vec<String> {
        self.inner.intents().into_iter().map(String::from).collect()
    }

    /// Get the signature of an intent
    ///
    /// Args:
    ///     name: Intent name
    ///
    /// Returns:
    ///     IntentSignature if found, None otherwise
    fn intent_signature(&self, name: &str) -> Option<PyIntentSignature> {
        self.inner
            .intent_signature(name)
            .map(PyIntentSignature::from)
    }

    /// Check if an intent exists in the policy
    ///
    /// Args:
    ///     name: Intent name to check
    ///
    /// Returns:
    ///     True if intent exists, False otherwise
    fn has_intent(&self, name: &str) -> bool {
        self.inner.has_intent(name)
    }

    fn __repr__(&self) -> String {
        let intent_count = self.inner.intents().len();
        format!("AxiomEngine(intents={})", intent_count)
    }
}

/// Get Axiom version information
#[pyfunction]
fn version() -> PyResult<String> {
    Ok(format!(
        "axiom-py {} (axiom-lang {})",
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_VERSION")
    ))
}

/// Quick verification helper (sync)
///
/// Load a policy and verify an intent in one call.
///
/// Args:
///     policy_path: Path to the .axm policy file
///     intent_name: Name of the intent to verify
///     fields: Dictionary of parameter names to values
///
/// Returns:
///     Verdict indicating whether the intent is allowed
#[pyfunction]
fn verify(policy_path: &str, intent_name: &str, fields: &PyDict) -> PyResult<PyVerdict> {
    let engine = PyAxiomEngine::from_file(policy_path)?;
    engine.verify(intent_name, fields)
}

/// Quick verification helper (async)
///
/// Load a policy and verify an intent in one async call.
/// Non-blocking for agent frameworks.
///
/// Args:
///     policy_path: Path to the .axm policy file
///     intent_name: Name of the intent to verify
///     fields: Dictionary of parameter names to values
///
/// Returns:
///     Coroutine that resolves to Verdict
#[pyfunction]
fn verify_async<'py>(py: Python<'py>, policy_path: String, intent_name: String, fields: &PyDict) -> PyResult<&'py PyAny> {
    let field_pairs = extract_fields(fields)?;

    pyo3_asyncio::tokio::future_into_py(py, async move {
        let engine = tokio::task::spawn_blocking(move || {
            AxiomEngine::from_file(&policy_path)
                .map_err(|e| PyValueError::new_err(format!("Failed to load policy: {}", e)))
        })
        .await
        .map_err(|e| PyRuntimeError::new_err(format!("Task join error: {}", e)))??;

        let engine = Arc::new(engine);

        let verdict = tokio::task::spawn_blocking(move || {
            let field_refs: Vec<(&str, &str)> = field_pairs
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect();

            engine
                .verify(&intent_name, &field_refs)
                .map_err(|e| PyRuntimeError::new_err(format!("Verification failed: {}", e)))
        })
        .await
        .map_err(|e| PyRuntimeError::new_err(format!("Task join error: {}", e)))??;

        Ok(PyVerdict::from(verdict))
    })
}

/// Axiom Python module
///
/// Provides policy verification for AI agents with sync and async support.
///
/// Key classes:
///     - AxiomEngine: Main policy engine for loading and verifying
///     - Verdict: Result of a verification operation
///     - IntentSignature: Intent metadata for introspection
///
/// Sync functions:
///     - verify(): One-shot verification helper
///
/// Async functions:
///     - verify_async(): Non-blocking verification for agent frameworks
#[pymodule]
fn axiom_py(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyAxiomEngine>()?;
    m.add_class::<PyVerdict>()?;
    m.add_class::<PyIntentSignature>()?;
    m.add_function(wrap_pyfunction!(version, m)?)?;
    m.add_function(wrap_pyfunction!(verify, m)?)?;
    m.add_function(wrap_pyfunction!(verify_async, m)?)?;

    m.add("__version__", env!("CARGO_PKG_VERSION"))?;

    Ok(())
}
