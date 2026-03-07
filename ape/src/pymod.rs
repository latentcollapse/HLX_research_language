//! PyO3 Python bindings for the Axiom Policy Engine
//!
//! Exposes `ConscienceKernel` and `EffectClass` to Python so that
//! `from ape import ConscienceKernel` works in Bitsy's TUI.
//!
//! The Conscience is digital physics — not a configurable policy engine.
//! The genesis predicates are hardcoded and immutable.

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

use crate::conscience::{ConscienceKernel as RustKernel, ConscienceVerdict, EffectClass};

/// Python-facing ConscienceKernel — the digital physics engine
#[pyclass(name = "ConscienceKernel")]
pub struct PyConscienceKernel {
    inner: RustKernel,
}

#[pymethods]
impl PyConscienceKernel {
    /// Create a new ConscienceKernel with genesis predicates installed
    #[new]
    fn new() -> Self {
        PyConscienceKernel {
            inner: RustKernel::new(),
        }
    }

    /// Verify an intent against the conscience.
    ///
    /// Args:
    ///     intent: Intent name (e.g. "WriteFile")
    ///     effect: Effect class ("READ", "WRITE", "EXECUTE", "NETWORK")
    ///     fields: Dict of field key-value pairs
    ///
    /// Returns: dict with "allowed" (bool) and "reason" (str or None)
    fn verify(
        &mut self,
        intent: &str,
        effect: &str,
        fields: std::collections::HashMap<String, String>,
    ) -> PyResult<PyObject> {
        let effect_class = EffectClass::from_str(effect)
            .ok_or_else(|| PyRuntimeError::new_err(format!("Unknown effect class: {}", effect)))?;

        let verdict = self.inner.evaluate(intent, &effect_class, &fields);

        Python::with_gil(|py| {
            let dict = pyo3::types::PyDict::new(py);
            match verdict {
                ConscienceVerdict::Allow => {
                    dict.set_item("allowed", true)?;
                    dict.set_item("reason", py.None())?;
                }
                ConscienceVerdict::Deny(reason) => {
                    dict.set_item("allowed", false)?;
                    dict.set_item("reason", reason)?;
                }
                ConscienceVerdict::Unknown => {
                    dict.set_item("allowed", false)?;
                    dict.set_item("reason", "No predicate applies — default deny")?;
                }
            }
            Ok(dict.into())
        })
    }
}

/// The `ape` Python module — digital physics for AI
#[pymodule]
fn ape(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyConscienceKernel>()?;
    Ok(())
}
