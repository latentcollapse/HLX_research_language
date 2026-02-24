"""
Axiom integration for LangChain agent executors.

Duck-typed — no hard LangChain import required. Exposes the interface that
LangChain's agent executor expects: ``name``, ``description``, ``args_schema``,
``_run(**kwargs)``, and ``_arun(**kwargs)``.

Example:
    from langchain.tools import Tool
    from axiom.presets import filesystem_readonly
    from axiom.integrations.langchain import AxiomGuardedTool

    base = Tool(name="ReadFile", description="Read a file", func=read_file)
    engine = filesystem_readonly(allowed_paths=["/workspace"])
    tool = AxiomGuardedTool(base, engine, intent_name="ReadFile")

    # Use tool inside a LangChain agent — Axiom verifies before execution
    result = tool._run(path="/workspace/notes.txt")
"""

from __future__ import annotations

from typing import Any, List, Optional

from ..guard import AxiomDenied


class AxiomGuardedTool:
    """
    Wraps a LangChain-compatible tool with Axiom policy verification.

    ``on_deny`` controls behavior when verification fails:
      - ``"raise"``          — raises ``AxiomDenied`` (default)
      - ``"return_none"``    — returns ``None``
      - ``"return_denial"``  — returns a human-readable string describing the block
    """

    def __init__(
        self,
        base_tool,
        engine,
        intent_name: Optional[str] = None,
        field_names: Optional[List[str]] = None,
        on_deny: str = "raise",
    ):
        self._base = base_tool
        self._engine = engine
        self._intent_name = intent_name or getattr(base_tool, "name", type(base_tool).__name__)
        self._field_names = field_names  # None → forward all kwargs
        self._on_deny = on_deny

    # ------------------------------------------------------------------
    # LangChain Tool interface (delegation to base_tool attributes)
    # ------------------------------------------------------------------

    @property
    def name(self) -> str:
        return getattr(self._base, "name", self._intent_name)

    @property
    def description(self) -> str:
        return getattr(self._base, "description", "")

    @property
    def args_schema(self):
        return getattr(self._base, "args_schema", None)

    # ------------------------------------------------------------------
    # Internal helpers
    # ------------------------------------------------------------------

    def _build_fields(self, kwargs: dict) -> dict:
        if self._field_names is not None:
            return {k: str(v) for k, v in kwargs.items() if k in self._field_names}
        return {k: str(v) for k, v in kwargs.items()}

    def _handle_denial(self, verdict, fields: dict) -> Any:
        exc = AxiomDenied(verdict, self._intent_name, fields)
        if self._on_deny == "raise":
            raise exc
        if self._on_deny == "return_none":
            return None
        # "return_denial"
        return f"[Axiom denied '{self._intent_name}': {exc.reason}]"

    def _call_base(self, **kwargs) -> Any:
        # Support base_tool._run(**kwargs) or base_tool(**kwargs) (callable tools)
        if hasattr(self._base, "_run"):
            return self._base._run(**kwargs)
        return self._base(**kwargs)

    async def _call_base_async(self, **kwargs) -> Any:
        if hasattr(self._base, "_arun"):
            return await self._base._arun(**kwargs)
        # Run sync tool in executor to avoid blocking the event loop
        import asyncio
        loop = asyncio.get_running_loop()
        import functools
        return await loop.run_in_executor(
            None, functools.partial(self._call_base, **kwargs)
        )

    # ------------------------------------------------------------------
    # Public run methods
    # ------------------------------------------------------------------

    def _run(self, **kwargs) -> Any:
        fields = self._build_fields(kwargs)
        verdict = self._engine.verify(self._intent_name, fields)
        if not verdict.allowed:
            return self._handle_denial(verdict, fields)
        return self._call_base(**kwargs)

    async def _arun(self, **kwargs) -> Any:
        fields = self._build_fields(kwargs)
        # Use async verify if available
        if hasattr(self._engine, "verify_async"):
            verdict = await self._engine.verify_async(self._intent_name, fields)
        else:
            verdict = self._engine.verify(self._intent_name, fields)
        if not verdict.allowed:
            return self._handle_denial(verdict, fields)
        return await self._call_base_async(**kwargs)

    def __repr__(self) -> str:
        return (
            f"AxiomGuardedTool(name={self.name!r}, intent={self._intent_name!r}, "
            f"on_deny={self._on_deny!r})"
        )
