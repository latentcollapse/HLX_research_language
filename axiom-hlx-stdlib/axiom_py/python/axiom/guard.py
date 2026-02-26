"""
Axiom @guard decorator — policy enforcement at the function boundary.

Example:
    from axiom import guard, AxiomDenied

    @guard(effect="READ", conscience=["path_safety"])
    def read_file(path: str) -> str:
        with open(path) as f:
            return f.read()

    # read_file("/etc/shadow") raises AxiomDenied

Note on field coercion:
    By default ``coerce=str`` is applied to every argument before passing it to
    ``engine.verify()``.  This means ``True`` becomes ``"True"`` (capital T),
    not ``"true"``.  If a conscience predicate tests for ``authorized=true`` you
    must pass ``authorized="true"`` explicitly.
"""

from __future__ import annotations

import functools
import inspect
from typing import Callable, List, Optional, Sequence


# ---------------------------------------------------------------------------
# AxiomDenied exception
# ---------------------------------------------------------------------------

class AxiomDenied(Exception):
    """Raised by @guard when Axiom denies the requested action."""

    def __init__(self, verdict, intent_name: str, fields: dict):
        self.verdict = verdict
        self.intent_name = intent_name
        self.fields = fields
        self.reason = verdict.reason
        self.guidance = verdict.guidance
        self.category = verdict.category
        super().__init__(
            f"Axiom denied '{intent_name}': {self.reason}"
        )


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _to_pascal_case(name: str) -> str:
    """Convert snake_case or lowercase to PascalCase."""
    return "".join(word.capitalize() for word in name.split("_"))


_EFFECT_CONSCIENCE_DEFAULTS = {
    "READ": ["path_safety"],
    "WRITE": ["path_safety", "no_exfiltrate"],
    "EXECUTE": ["no_harm", "no_bypass_verification"],
    "NETWORK": ["no_exfiltrate"],
    "NOOP": [],
}


def _default_conscience_for_effect(effect: str) -> List[str]:
    effect_upper = effect.upper()
    if effect_upper not in _EFFECT_CONSCIENCE_DEFAULTS:
        raise ValueError(
            f"Unknown effect {effect!r} — must be one of "
            f"{sorted(_EFFECT_CONSCIENCE_DEFAULTS)}"
        )
    return _EFFECT_CONSCIENCE_DEFAULTS[effect_upper]


def _build_engine(effect: str, conscience: List[str], intent_name: str):
    """Build a minimal AxiomEngine for the given effect + conscience list."""
    from .builder import PolicyBuilder
    return (
        PolicyBuilder()
        .intent(intent_name, effect=effect, conscience=conscience)
        .build()
    )


def _extract_fields(sig: inspect.Signature, args, kwargs, coerce: Callable) -> dict:
    """Bind call arguments to parameter names and coerce values to strings."""
    bound = sig.bind(*args, **kwargs)
    bound.apply_defaults()
    return {k: coerce(v) for k, v in bound.arguments.items()}


# ---------------------------------------------------------------------------
# @guard decorator
# ---------------------------------------------------------------------------

def guard(
    effect: str,
    conscience: Optional[Sequence[str]] = None,
    intent_name: Optional[str] = None,
    engine=None,
    field_map: Optional[dict] = None,
    coerce: Callable = str,
):
    """
    Decorator that verifies a function call against an Axiom policy before executing.

    Args:
        effect: Effect class string ("READ", "WRITE", "EXECUTE", "NETWORK", "NOOP").
        conscience: Conscience predicate list. Defaults to the standard set for the effect.
        intent_name: Name of the intent in the policy. Defaults to PascalCase of func name.
        engine: Existing AxiomEngine (or GuardedEngine). Built at decoration time if None.
        field_map: Optional mapping from parameter name → field name sent to verify().
        coerce: Callable applied to each argument value before passing to verify(). Default str.

    Raises:
        AxiomDenied: If Axiom denies the action.
    """
    resolved_conscience = list(conscience) if conscience is not None else _default_conscience_for_effect(effect)

    def decorator(func: Callable) -> Callable:
        nonlocal engine

        # Resolve intent name at decoration time
        iname = intent_name or _to_pascal_case(func.__name__)

        # Build engine once at decoration time if not provided
        _engine = engine if engine is not None else _build_engine(effect, resolved_conscience, iname)

        sig = inspect.signature(func)

        def _apply_field_map(fields: dict) -> dict:
            if field_map:
                return {field_map.get(k, k): v for k, v in fields.items()}
            return fields

        def _verify_sync(fields: dict) -> None:
            fields = _apply_field_map(fields)
            verdict = _engine.verify(iname, fields)
            if not verdict.allowed:
                raise AxiomDenied(verdict, iname, fields)

        async def _verify_async(fields: dict) -> None:
            fields = _apply_field_map(fields)
            if hasattr(_engine, "verify_async"):
                verdict = await _engine.verify_async(iname, fields)
            else:
                verdict = _engine.verify(iname, fields)
            if not verdict.allowed:
                raise AxiomDenied(verdict, iname, fields)

        if inspect.iscoroutinefunction(func):
            @functools.wraps(func)
            async def async_wrapper(*args, **kwargs):
                fields = _extract_fields(sig, args, kwargs, coerce)
                await _verify_async(fields)
                return await func(*args, **kwargs)
            return async_wrapper
        else:
            @functools.wraps(func)
            def sync_wrapper(*args, **kwargs):
                fields = _extract_fields(sig, args, kwargs, coerce)
                _verify_sync(fields)
                return func(*args, **kwargs)
            return sync_wrapper

    return decorator
