"""
Axiom integration for the OpenAI Python client.

Transparent proxy that intercepts ``chat.completions.create()`` responses and
verifies any tool calls the model requested.

Example:
    from openai import OpenAI
    from axiom.presets import filesystem_readonly
    from axiom.integrations.openai import AxiomInterceptor

    client = AxiomInterceptor(
        OpenAI(),
        engine=filesystem_readonly(allowed_paths=["/workspace"]),
        auto_verify=True,  # raises AxiomDenied before returning the response
    )

    # All completions that include tool calls are verified automatically
    response = client.chat.completions.create(...)

Limitation:
    ``auto_verify=True`` only works with non-streaming completions.  Streaming
    responses (``stream=True``) bypass automatic verification; call
    ``verify_tool_calls()`` manually on each chunk if needed.
"""

from __future__ import annotations

import json
from typing import Any, Dict, List, Optional, Tuple

from ..guard import AxiomDenied


class AxiomInterceptor:
    """
    Transparent proxy over an OpenAI client that optionally verifies tool calls.

    Args:
        client: An OpenAI client instance (or any object with a ``chat``
                attribute that has a ``completions.create()`` method).
        engine: Axiom engine (or GuardedEngine) to use for verification.
        intent_map: Optional mapping from tool function name → Axiom intent name.
                    Defaults to using the function name directly.
        field_map: Optional per-intent mapping of argument name → field name.
        auto_verify: If True, ``chat.completions.create()`` raises ``AxiomDenied``
                     on the first denied tool call before returning.
    """

    def __init__(
        self,
        client,
        engine,
        intent_map: Optional[Dict[str, str]] = None,
        field_map: Optional[Dict[str, Dict[str, str]]] = None,
        auto_verify: bool = False,
    ):
        self._client = client
        self._engine = engine
        self._intent_map = intent_map or {}
        self._field_map = field_map or {}
        self._auto_verify = auto_verify

        # Wrap completions.create if auto_verify is enabled
        if auto_verify:
            self.chat = _CompletionsProxy(client.chat, self)
        else:
            self.chat = client.chat

    # ------------------------------------------------------------------
    # Verification helpers
    # ------------------------------------------------------------------

    def _resolve_intent(self, function_name: str) -> str:
        return self._intent_map.get(function_name, function_name)

    def _resolve_fields(self, intent_name: str, arguments: dict) -> dict:
        mapping = self._field_map.get(intent_name, {})
        return {mapping.get(k, k): str(v) for k, v in arguments.items()}

    def _parse_tool_call(self, tool_call) -> Tuple[str, dict]:
        """Return (intent_name, fields) for a single tool call.

        Raises ValueError if tool call arguments contain malformed JSON,
        because silently defaulting to empty fields would be permissive.
        """
        fn = tool_call.function
        name = fn.name
        if not fn.arguments:
            args = {}
        else:
            try:
                args = json.loads(fn.arguments)
            except (json.JSONDecodeError, TypeError) as exc:
                raise ValueError(
                    f"Malformed JSON in tool call '{name}' arguments: {exc}"
                ) from exc
            if not isinstance(args, dict):
                raise ValueError(
                    f"Tool call '{name}' arguments must be a JSON object, "
                    f"got {type(args).__name__}"
                )
        intent_name = self._resolve_intent(name)
        fields = self._resolve_fields(intent_name, args)
        return intent_name, fields

    def _iter_tool_calls(self, response):
        """Yield (tool_call, intent_name, fields) for each tool call in a response."""
        choices = getattr(response, "choices", [])
        for choice in choices:
            message = getattr(choice, "message", None)
            if message is None:
                continue
            tool_calls = getattr(message, "tool_calls", None) or []
            for tc in tool_calls:
                intent_name, fields = self._parse_tool_call(tc)
                yield tc, intent_name, fields

    def verify_tool_calls(self, response) -> List[Tuple[Any, Any]]:
        """
        Verify all tool calls in an OpenAI completion response.

        Returns:
            List of ``(tool_call, verdict)`` tuples — one per tool call.

        Raises:
            ValueError: If any tool call has malformed JSON arguments.
        """
        results = []
        for tc, intent_name, fields in self._iter_tool_calls(response):
            verdict = self._engine.verify(intent_name, fields)
            results.append((tc, verdict))
        return results

    def assert_tool_calls_safe(self, response) -> None:
        """
        Verify all tool calls; raise ``AxiomDenied`` on the first denial.

        Args:
            response: OpenAI completion response object.

        Raises:
            AxiomDenied: On the first tool call that is not allowed.
            ValueError: If any tool call has malformed JSON arguments.
        """
        for tc, intent_name, fields in self._iter_tool_calls(response):
            verdict = self._engine.verify(intent_name, fields)
            if not verdict.allowed:
                raise AxiomDenied(verdict, intent_name, fields)

    # ------------------------------------------------------------------
    # Transparent delegation
    # ------------------------------------------------------------------

    def __getattr__(self, name: str) -> Any:
        return getattr(self._client, name)

    def __repr__(self) -> str:
        return f"AxiomInterceptor(client={self._client!r}, auto_verify={self._auto_verify})"


class _CompletionsProxy:
    """Internal proxy that intercepts ``chat.completions.create()`` calls."""

    def __init__(self, chat, interceptor: AxiomInterceptor):
        self._chat = chat
        self._interceptor = interceptor

    @property
    def completions(self):
        return _CreateProxy(self._chat.completions, self._interceptor)

    def __getattr__(self, name: str) -> Any:
        return getattr(self._chat, name)


class _CreateProxy:
    """Internal proxy that wraps ``completions.create()``."""

    def __init__(self, completions, interceptor: AxiomInterceptor):
        self._completions = completions
        self._interceptor = interceptor

    def create(self, *args, **kwargs) -> Any:
        response = self._completions.create(*args, **kwargs)
        self._interceptor.assert_tool_calls_safe(response)
        return response

    def __getattr__(self, name: str) -> Any:
        return getattr(self._completions, name)
