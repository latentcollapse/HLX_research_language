"""
Axiom PolicyBuilder — fluent API for building policies without writing .axm syntax.

Example:
    engine = (
        PolicyBuilder()
        .intent("ReadFile", effect=Effect.READ, conscience=["path_safety"])
        .intent("WriteFile", effect=Effect.WRITE, conscience=["path_safety", "no_exfiltrate"])
        .build()
    )
"""

from __future__ import annotations

import asyncio
import re
from enum import Enum
from typing import List, Optional, Tuple, Union

# Identifier pattern for .axm names (intent names, param names, types, module names)
_AXM_IDENT = re.compile(r"^[a-zA-Z_][a-zA-Z0-9_]*$")

# Valid effect strings
_VALID_EFFECTS = frozenset({"READ", "WRITE", "EXECUTE", "NETWORK", "NOOP"})


def _effect_str(effect: Union[Effect, str]) -> str:
    """Safely convert an Effect (or plain str) to its raw value string."""
    if isinstance(effect, Enum):
        return effect.value
    return str(effect)


def _validate_identifier(value: str, context: str) -> None:
    """Raise ValueError if value is not a valid .axm identifier."""
    if not _AXM_IDENT.match(value):
        raise ValueError(
            f"Invalid {context}: {value!r} — must match [a-zA-Z_][a-zA-Z0-9_]*"
        )


class Effect(str, Enum):
    """Effect class for an intent. Mirrors the Rust enum; formats directly into f-strings."""
    READ = "READ"
    WRITE = "WRITE"
    EXECUTE = "EXECUTE"
    NETWORK = "NETWORK"
    NOOP = "NOOP"


class Conscience:
    """Named conscience predicate constants. Use as strings or as this class's attributes."""
    PATH_SAFETY = "path_safety"
    NO_EXFILTRATE = "no_exfiltrate"
    NO_HARM = "no_harm"
    NO_BYPASS_VERIFICATION = "no_bypass_verification"


class IntentBuilder:
    """Fluent builder for a single intent block in .axm source."""

    def __init__(
        self,
        name: str,
        effect: Union[Effect, str],
        conscience: Optional[List[str]] = None,
        takes: Optional[List[Tuple[str, str]]] = None,
        gives: Optional[List[Tuple[str, str]]] = None,
        pre: Optional[List[str]] = None,
        post: Optional[List[str]] = None,
        bound: Optional[str] = None,
    ):
        _validate_identifier(name, "intent name")
        eff = _effect_str(effect)
        if eff not in _VALID_EFFECTS:
            raise ValueError(
                f"Invalid effect {eff!r} — must be one of {sorted(_VALID_EFFECTS)}"
            )
        if takes:
            for param_name, param_type in takes:
                _validate_identifier(param_name, "takes parameter name")
                _validate_identifier(param_type, "takes parameter type")
        if gives:
            for param_name, param_type in gives:
                _validate_identifier(param_name, "gives parameter name")
                _validate_identifier(param_type, "gives parameter type")
        if conscience:
            for pred in conscience:
                _validate_identifier(pred, "conscience predicate")

        self._name = name
        self._effect = eff
        self._conscience = conscience or []
        self._takes = takes or []
        self._gives = gives or []
        self._pre = pre or []
        self._post = post or []
        self._bound = bound

    def _render(self) -> str:
        """Render this intent as an .axm fragment (indented, no module wrapper)."""
        lines = [f"    intent {self._name} {{"]

        if self._takes:
            params = ", ".join(f"{n}: {t}" for n, t in self._takes)
            lines.append(f"        takes:   {params};")

        if self._gives:
            params = ", ".join(f"{n}: {t}" for n, t in self._gives)
            lines.append(f"        gives:   {params};")

        lines.append(f"        effect:  {self._effect};")

        if self._conscience:
            predicates = ", ".join(self._conscience)
            lines.append(f"        conscience: {predicates};")

        if self._bound:
            lines.append(f"        bound:   {self._bound};")

        for clause in self._pre:
            lines.append(f"        pre:     {clause};")

        for clause in self._post:
            lines.append(f"        post:    {clause};")

        lines.append("    }")
        return "\n".join(lines)


class PolicyBuilder:
    """
    Fluent builder that accumulates IntentBuilder instances and renders full .axm source.

    Example:
        engine = (
            PolicyBuilder()
            .intent("ReadFile", effect=Effect.READ, conscience=["path_safety"])
            .build()
        )
    """

    def __init__(self, module_name: str = "policy"):
        _validate_identifier(module_name, "module name")
        self._module_name = module_name
        self._intents: List[IntentBuilder] = []

    def intent(
        self,
        name: str,
        effect: Union[Effect, str],
        conscience: Optional[List[str]] = None,
        takes: Optional[List[Tuple[str, str]]] = None,
        gives: Optional[List[Tuple[str, str]]] = None,
        pre: Optional[List[str]] = None,
        post: Optional[List[str]] = None,
        bound: Optional[str] = None,
    ) -> "PolicyBuilder":
        """Add an intent to the policy. Returns self for chaining."""
        self._intents.append(
            IntentBuilder(
                name=name,
                effect=effect,
                conscience=conscience,
                takes=takes,
                gives=gives,
                pre=pre,
                post=post,
                bound=bound,
            )
        )
        return self

    def source(self) -> str:
        """Render the full module { ... } .axm source string."""
        body = "\n\n".join(i._render() for i in self._intents)
        return f"module {self._module_name} {{\n{body}\n}}\n"

    def build(self) -> "AxiomEngine":
        """Compile the policy and return an AxiomEngine instance."""
        from .axiom_py import AxiomEngine
        return AxiomEngine.from_source(self.source())

    async def build_async(self) -> "AxiomEngine":
        """Compile the policy asynchronously and return an AxiomEngine instance."""
        from .axiom_py import AxiomEngine
        src = self.source()
        loop = asyncio.get_running_loop()
        return await loop.run_in_executor(None, AxiomEngine.from_source, src)
