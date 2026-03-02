#!/usr/bin/env python3
"""
Bit MCP Server - Exposes Bit as an MCP tool for Claude and other LLMs

This creates a Model Context Protocol server that allows Claude to interact
with Bit through the standard MCP interface.

"""

import os
import sys
from pathlib import Path

from mcp.server.fastmcp import FastMCP

HLX_ROOT = Path(__file__).parent.parent.parent
sys.path.insert(0, str(HLX_ROOT / "axiom-hlx-stdlib" / "axiom_py" / "python"))
from axiom.bit import BitSeed, BitLevel

import sqlite3

BIT_DIR = Path(__file__).parent
CORPUS_PATH = os.environ.get("BIT_CORPUS_PATH", str(BIT_DIR / "corpus.db"))
bit = BitSeed(corpus_path=CORPUS_PATH)

# Phase 19C/D: Knowledge extraction integration
sys.path.insert(0, str(BIT_DIR.parent))
from knowledge_extractor import KnowledgeExtractor
knowledge_extractor = KnowledgeExtractor(bit)

mcp = FastMCP("bit")


@mcp.tool()
def bit_observe(source: str, content: str, relevance: float = 1.0) -> dict:
    """Feed Bit an observation from the environment."""
    return bit.observe({"source": source, "content": content, "relevance": relevance})


@mcp.tool()
def bit_ask(question: str) -> str:
    """Ask Bit a question. She answers from her current knowledge."""
    return bit.ask(question)


@mcp.tool()
def bit_status() -> dict:
    """Query Bit's current state and metrics."""
    status = bit.status()
    return {
        "level": status.level,
        "homeostasis_achieved": status.homeostasis_achieved,
        "observation_count": status.observation_count,
        "pattern_count": status.pattern_count,
        "pending_questions": status.pending_questions,
        "successful_modifications": status.successful_modifications,
        "rollback_count": status.rollback_count,
        "uptime_secs": round(status.uptime_secs, 2),
    }
@mcp.tool()
def bit_propose(type: str, description: str, confidence: float = 0.8) -> dict:
    """Let Bit propose a self-modification through her RSI pipeline"""
    proposal = bit.propose({
        "type": type,
        "description": description,
        "confidence": confidence,
    })
    return {
        "allowed": proposal.allowed,
        "modification_type": proposal.modification_type,
        "risk_assessment": proposal.risk_assessment,
        "reason": proposal.reason,
    }
@mcp.tool()
def bit_learn(pattern: str, confidence: float) -> dict:
    """Record a pattern Bit has learned"""
    bit.learn(pattern, confidence)
    return {
        "learned": True,
        "pattern": pattern,
        "confidence": confidence,
        "total_patterns": len(bit.learned_patterns),
    }
@mcp.tool()
def bit_homeostasis() -> dict:
    """Signal Bit has achieved homeostasis"""
    bit.on_homeostasis()
    return {
        "homeostasis_count": bit.homeostasis_count,
        "current_level": bit.level.value,
    }

@mcp.tool()
def bit_bond(response: str, source: str = "llm") -> dict:
    """
    Feed Bit an LLM bond response to extract and learn from.
    
    Phase 19D: Bond responses are processed through the knowledge extractor,
    extracting beliefs (with lower initial confidence) and patterns.
    """
    result = knowledge_extractor.ingest_bond_response(response, confidence=0.4)
    return {
        "beliefs_extracted": len(result["beliefs_added"]),
        "patterns_extracted": len(result["patterns_added"]),
        "beliefs": result["beliefs_added"],
        "patterns": result["patterns_added"],
    }

@mcp.tool()
def bit_get_self_model() -> dict:
    """
    Get Bit's self-model - all beliefs where subject='I'.
    
    Returns her identity, capabilities, and self-knowledge.
    """
    return bit.get_self_model()

@mcp.tool()
def bit_ingest_identity(content: str) -> dict:
    """
    Ingest identity document content (K-12 Level 0).
    
    Transforms second-person statements to first-person beliefs.
    """
    result = knowledge_extractor.ingest_k12_level0(content)
    return {
        "statements_processed": result["total_statements"],
        "beliefs_added": len(result["beliefs_added"]),
        "patterns_added": len(result["patterns_added"]),
        "errors": len(result["errors"]),
    }

@mcp.resource("bit://identity")
def bit_identity() -> str:
    """Bit's identity document"""
    conn = sqlite3.connect(CORPUS_PATH)
    cursor = conn.cursor()
    cursor.execute("SELECT content FROM documents WHERE name='identity'")
    row = cursor.fetchone()
    conn.close()
    return row[0] if row else "Identity not found"


@mcp.resource("bit://status")
def bit_status_resource() -> str:
    """Bit's current status as a human-readable string"""
    status = bit.status()
    return f"""Bit Status:
Level: {status.level}
Homeostasis: {status.homeostasis_achieved}
Observations: {status.observation_count}
Patterns: {status.pattern_count}
Modifications: {status.successful_modifications}
Rollbacks: {status.rollback_count}
Uptime: {status.uptime_secs:.1f}s
"""


if __name__ == "__main__":
    import asyncio
    print(f"Starting Bit MCP Server...", file=sys.stderr)
    print(f"Corpus: {CORPUS_PATH}", file=sys.stderr)
    print(f"Level: {bit.level.value}", file=sys.stderr)
    print(f"Observations: {len(bit.observations)}", file=sys.stderr)
    print(f"Patterns: {len(bit.learned_patterns)}", file=sys.stderr)
    mcp.run(transport="stdio")
