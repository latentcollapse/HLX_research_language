"""
Axiom - Policy verification for AI agents

A verification-first policy engine that provides deterministic,
auditable safety enforcement for AI agent actions.

Supports both sync and async usage for integration with agent frameworks.

Example (sync):
    from axiom import AxiomEngine

    engine = AxiomEngine.from_file("policy.axm")
    verdict = engine.verify("WriteFile", {"path": "/tmp/test.txt"})

    if verdict.allowed:
        print("Action permitted")
    else:
        print(f"Denied: {verdict.reason}")

Example (async):
    from axiom import AxiomEngine

    engine = await AxiomEngine.from_file_async("policy.axm")
    verdict = await engine.verify_async("WriteFile", {"path": "/tmp/test.txt"})

    if verdict.allowed:
        print("Action permitted")
"""

from .axiom_py import (
    AxiomEngine,
    Verdict,
    IntentSignature,
    verify,
    verify_async,
    version,
)
from .builder import PolicyBuilder, IntentBuilder, Effect, Conscience
from .guard import guard, AxiomDenied

__version__ = "0.2.0"

__all__ = [
    # Core Rust bindings
    "AxiomEngine",
    "Verdict",
    "IntentSignature",
    "verify",
    "verify_async",
    "version",
    # Builder API
    "PolicyBuilder",
    "IntentBuilder",
    "Effect",
    "Conscience",
    # Decorator API
    "guard",
    "AxiomDenied",
    "__version__",
]
