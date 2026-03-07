
from __future__ import annotations
import sys
import json
from pathlib import Path
from typing import Optional, Dict, Any, List
from enum import Enum

# Ensure we can import hlx_ffi
sys.path.insert(0, "/home/matt")
from hlx_ffi import HlxRuntime

class BitSeedV1_3:
    \"\"\"
    Native HLX implementation of BitSeed (The Crow Brain).
    Uses libhlx.so via FFI to manage 512-bit latent state.
    \"\"\"
    def __init__(self, lib_path: str, hlx_source_path: str):
        self.rt = HlxRuntime(lib_path)
        with open(hlx_source_path, "r") as f:
            self.rt.compile_source(f.read())
        self.level = "seedling"
        self.learned_patterns = []
        self.observations = []

    def observe(self, content: str, source: str = "user", relevance: float = 1.0):
        result = self.rt.call("observe", source, content, relevance)
        self.observations.append({"source": source, "content": content, "relevance": relevance})
        return result

    def ask(self, question: str) -> str:
        return str(self.rt.call("ask", question))

    def get_status(self) -> dict:
        return {
            "level": self.level,
            "density": 512,
            "substrate": "Native HLX",
            "observations": len(self.observations)
        }
