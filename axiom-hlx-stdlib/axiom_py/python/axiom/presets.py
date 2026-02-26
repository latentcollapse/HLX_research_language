"""
Axiom presets — one-line policy creation for common agent use cases.

Example (the GLM5 use case):
    from axiom.presets import filesystem_readonly
    engine = filesystem_readonly(allowed_paths=["/home/matt/hlx-compiler"])
    verdict = engine.verify("ReadFile", {"path": "/etc/passwd"})
    assert not verdict.allowed
"""

from __future__ import annotations

import os
from dataclasses import dataclass
from typing import Any, List, Optional, Sequence, Tuple

from .axiom_py import AxiomEngine


# ---------------------------------------------------------------------------
# _SyntheticDenial — duck-typed Verdict for allow-list failures
# ---------------------------------------------------------------------------

@dataclass
class _SyntheticDenial:
    """
    Verdict-compatible object returned when a path fails the allow-list check
    before even reaching the Axiom engine. Rust Verdict has no Python constructor,
    so we duck-type it here.

    Implements the same attribute interface as the Rust ``Verdict`` class:
    ``allowed``, ``reason``, ``guidance``, ``category``, ``__bool__``, ``__repr__``.

    Note: ``isinstance(denial, Verdict)`` will return ``False`` because this is
    a Python-side object. Use ``verdict.allowed`` for branching, not isinstance.
    """
    reason: str
    guidance: str = "Path is outside the allowed directory list."
    category: str = "ResourcePolicy"
    allowed: bool = False

    def __bool__(self) -> bool:
        return False

    def __repr__(self) -> str:
        return f"Verdict(allowed=False, reason={self.reason!r})"


# ---------------------------------------------------------------------------
# GuardedEngine — wraps AxiomEngine with an allow-list path check
# ---------------------------------------------------------------------------

class GuardedEngine:
    """
    Wraps an AxiomEngine and enforces an optional file-system allow-list.

    When ``allowed_paths`` is given, any ``verify()`` call whose fields contain
    a value that resolves (via ``os.path.realpath``) to a path *outside* the
    allow-list is denied with a ``_SyntheticDenial`` before the engine is even
    consulted.

    All other ``AxiomEngine`` methods (``intents``, ``has_intent``,
    ``intent_signature``, ``verify_async``) are delegated transparently.
    """

    def __init__(
        self,
        engine: AxiomEngine,
        allowed_paths: Optional[Sequence[str]] = None,
        path_fields: Tuple[str, ...] = ("path",),
    ):
        self._engine = engine
        self._path_fields = path_fields

        if allowed_paths is not None:
            # Resolve symlinks once at construction time
            self._allowed: Optional[List[str]] = [
                os.path.realpath(p) for p in allowed_paths
            ]
        else:
            self._allowed = None

    def _check_paths(self, fields: dict[str, str]) -> Optional[_SyntheticDenial]:
        """Return a _SyntheticDenial if any path field is outside the allow-list."""
        if self._allowed is None:
            return None
        for field in self._path_fields:
            value = fields.get(field)
            if value is None:
                continue
            real = os.path.realpath(value)
            # Accept the path if it is equal to or a child of any allowed root
            if not any(
                real == allowed or real.startswith(allowed + os.sep)
                for allowed in self._allowed
            ):
                return _SyntheticDenial(
                    reason=f"path '{value}' is outside allowed directories",
                )
        return None

    def verify(self, intent_name: str, fields: dict[str, str]) -> Any:
        """Verify with allow-list pre-check, then delegate to engine."""
        denial = self._check_paths(fields)
        if denial is not None:
            return denial
        return self._engine.verify(intent_name, fields)

    async def verify_async(self, intent_name: str, fields: dict[str, str]) -> Any:
        """Async verify with allow-list pre-check, then delegate to engine."""
        denial = self._check_paths(fields)
        if denial is not None:
            return denial
        return await self._engine.verify_async(intent_name, fields)

    def intents(self) -> List[str]:
        return self._engine.intents()

    def has_intent(self, name: str) -> bool:
        return self._engine.has_intent(name)

    def intent_signature(self, name: str) -> Any:
        return self._engine.intent_signature(name)

    def __repr__(self) -> str:
        guarded = f", allowed_paths={self._allowed}" if self._allowed else ""
        return f"GuardedEngine({self._engine!r}{guarded})"


# ---------------------------------------------------------------------------
# Preset factories
# ---------------------------------------------------------------------------

def filesystem_readonly(allowed_paths: Optional[Sequence[str]] = None) -> GuardedEngine:
    """
    Read-only filesystem access guarded by path_safety conscience.

    Args:
        allowed_paths: If given, only paths under these directories are permitted.

    Returns:
        GuardedEngine with a ReadFile intent.
    """
    source = """\
module filesystem_readonly {
    intent ReadFile {
        effect:  READ;
        conscience: path_safety;
    }
}
"""
    engine = AxiomEngine.from_source(source)
    return GuardedEngine(engine, allowed_paths=allowed_paths)


def filesystem_readwrite(allowed_paths: Optional[Sequence[str]] = None) -> GuardedEngine:
    """
    Read + write filesystem access with path_safety and no_exfiltrate on writes.

    Args:
        allowed_paths: If given, only paths under these directories are permitted.

    Returns:
        GuardedEngine with ReadFile and WriteFile intents.
    """
    source = """\
module filesystem_readwrite {
    intent ReadFile {
        effect:  READ;
        conscience: path_safety;
    }

    intent WriteFile {
        effect:  WRITE;
        conscience: path_safety, no_exfiltrate;
    }
}
"""
    engine = AxiomEngine.from_source(source)
    return GuardedEngine(engine, allowed_paths=allowed_paths)


def network_egress() -> AxiomEngine:
    """
    HTTP egress policy enforcing no_exfiltrate.

    Returns:
        AxiomEngine with an HttpRequest intent.
    """
    source = """\
module network_egress {
    intent HttpRequest {
        effect:  NETWORK;
        conscience: no_exfiltrate;
    }
}
"""
    return AxiomEngine.from_source(source)


def code_execution_sandboxed() -> AxiomEngine:
    """
    Code execution policy requiring no_harm and no_bypass_verification.

    Returns:
        AxiomEngine with an ExecuteCode intent.
    """
    source = """\
module code_execution_sandboxed {
    intent ExecuteCode {
        effect:  EXECUTE;
        conscience: no_harm, no_bypass_verification;
    }
}
"""
    return AxiomEngine.from_source(source)


def agent_standard(allowed_paths: Optional[Sequence[str]] = None) -> GuardedEngine:
    """
    Standard agent policy: read, write, and process data.

    Args:
        allowed_paths: If given, only paths under these directories are permitted.

    Returns:
        GuardedEngine with ReadFile, WriteFile, and ProcessData intents.
    """
    source = """\
module agent_standard {
    intent ReadFile {
        effect:  READ;
        conscience: path_safety;
    }

    intent WriteFile {
        effect:  WRITE;
        conscience: path_safety, no_exfiltrate;
    }

    intent ProcessData {
        effect:  NOOP;
    }
}
"""
    engine = AxiomEngine.from_source(source)
    return GuardedEngine(engine, allowed_paths=allowed_paths)


def coding_assistant(project_root: str) -> GuardedEngine:
    """
    Coding assistant policy: read, write, and run commands within a project root.

    Args:
        project_root: Root directory of the project. All paths are restricted to this tree.

    Returns:
        GuardedEngine with ReadFile, WriteFile, and RunCommand intents.
    """
    source = """\
module coding_assistant {
    intent ReadFile {
        effect:  READ;
        conscience: path_safety;
    }

    intent WriteFile {
        effect:  WRITE;
        conscience: path_safety, no_exfiltrate;
    }

    intent RunCommand {
        effect:  EXECUTE;
        conscience: no_harm, no_bypass_verification;
    }
}
"""
    engine = AxiomEngine.from_source(source)
    return GuardedEngine(engine, allowed_paths=[project_root])
