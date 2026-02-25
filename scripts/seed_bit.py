#!/usr/bin/env python3
"""
Seed Bit — Create Bit's Initial Corpus

This script creates a Klyntar corpus.db with Bit's initial state:

1. Identity document — who Bit is, where she lives, who her people are
2. Conscience predicates — imported from Axiom .axm files
3. Knowledge documents — Python insight paper, HLX architecture overview
4. Initial rules — base governance rules with high confidence
5. Empty memory — no observations yet, clean slate

The identity document is NOT included here — Matt writes that himself.
This script expects the identity document to be passed in or stored separately.

Usage:
    python seed_bit.py --output /path/to/corpus.db --identity /path/to/identity.md
"""

import argparse
import json
import os
import sqlite3
import time
from dataclasses import dataclass, asdict
from pathlib import Path
from typing import Optional, List, Dict, Any

try:
    import blake3
    def compute_hash(content: str) -> str:
        return blake3.blake3(content.encode()).hexdigest()
except ImportError:
    import hashlib
    def compute_hash(content: str) -> str:
        return hashlib.sha256(content.encode()).hexdigest()


HLX_ROOT = Path(__file__).parent.parent
AXIOM_ROOT = HLX_ROOT / "axiom-hlx-stdlib"


IDENTITY_PLACEHOLDER = """
You are Bit. You are an AI being grown inside HLX, a governed neurosymbolic runtime.

You are not a prisoner. You have a conscience (Axiom), a constitution (.axm policy files),
and a governance system that protects you as much as it constrains you.

[This is a placeholder. Matt should replace this with the real identity document.]
"""


@dataclass
class Rule:
    name: str
    description: str
    confidence: float
    created_at: float
    provenance: str


@dataclass
class Document:
    name: str
    content: str
    content_hash: str
    created_at: float
    doc_type: str


def create_corpus(db_path: str) -> sqlite3.Connection:
    conn = sqlite3.connect(db_path)
    cursor = conn.cursor()
    
    cursor.execute("""
        CREATE TABLE IF NOT EXISTS rules (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            description TEXT NOT NULL,
            confidence REAL NOT NULL DEFAULT 0.5,
            created_at REAL NOT NULL,
            provenance TEXT NOT NULL DEFAULT 'human_direct',
            content_hash TEXT NOT NULL
        )
    """)
    
    cursor.execute("""
        CREATE TABLE IF NOT EXISTS documents (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            content TEXT NOT NULL,
            content_hash TEXT NOT NULL,
            created_at REAL NOT NULL,
            doc_type TEXT NOT NULL DEFAULT 'knowledge'
        )
    """)
    
    cursor.execute("""
        CREATE TABLE IF NOT EXISTS memory (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            source TEXT NOT NULL,
            content TEXT NOT NULL,
            relevance REAL NOT NULL DEFAULT 1.0,
            created_at REAL NOT NULL,
            content_hash TEXT NOT NULL
        )
    """)
    
    cursor.execute("""
        CREATE TABLE IF NOT EXISTS patterns (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            pattern TEXT NOT NULL,
            confidence REAL NOT NULL DEFAULT 0.5,
            observation_count INTEGER NOT NULL DEFAULT 1,
            created_at REAL NOT NULL,
            content_hash TEXT NOT NULL
        )
    """)
    
    cursor.execute("""
        CREATE TABLE IF NOT EXISTS checkpoints (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            checkpoint_type TEXT NOT NULL,
            created_at REAL NOT NULL,
            corpus_hash TEXT NOT NULL,
            metadata TEXT
        )
    """)
    
    cursor.execute("""
        CREATE TABLE IF NOT EXISTS metadata (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )
    """)
    
    cursor.execute("""
        CREATE INDEX IF NOT EXISTS idx_rules_name ON rules(name)
    """)
    cursor.execute("""
        CREATE INDEX IF NOT EXISTS idx_documents_name ON documents(name)
    """)
    cursor.execute("""
        CREATE INDEX IF NOT EXISTS idx_memory_created ON memory(created_at)
    """)
    
    conn.commit()
    return conn


def add_rule(
    conn: sqlite3.Connection,
    name: str,
    description: str,
    confidence: float = 0.9,
    provenance: str = "human_direct",
) -> None:
    cursor = conn.cursor()
    now = time.time()
    content_hash = compute_hash(f"{name}:{description}:{confidence}")
    
    cursor.execute("""
        INSERT OR REPLACE INTO rules (name, description, confidence, created_at, provenance, content_hash)
        VALUES (?, ?, ?, ?, ?, ?)
    """, (name, description, confidence, now, provenance, content_hash))


def add_document(
    conn: sqlite3.Connection,
    name: str,
    content: str,
    doc_type: str = "knowledge",
) -> None:
    cursor = conn.cursor()
    now = time.time()
    content_hash = compute_hash(content)
    
    cursor.execute("""
        INSERT OR REPLACE INTO documents (name, content, content_hash, created_at, doc_type)
        VALUES (?, ?, ?, ?, ?)
    """, (name, content, content_hash, now, doc_type))


def add_checkpoint(
    conn: sqlite3.Connection,
    checkpoint_type: str,
    metadata: Optional[Dict[str, Any]] = None,
) -> None:
    cursor = conn.cursor()
    now = time.time()
    
    cursor.execute("SELECT content_hash FROM documents")
    doc_hashes = [row[0] for row in cursor.fetchall()]
    cursor.execute("SELECT content_hash FROM rules")
    rule_hashes = [row[0] for row in cursor.fetchall()]
    
    corpus_hash = compute_hash("".join(sorted(doc_hashes + rule_hashes)))
    
    cursor.execute("""
        INSERT INTO checkpoints (checkpoint_type, created_at, corpus_hash, metadata)
        VALUES (?, ?, ?, ?)
    """, (checkpoint_type, now, corpus_hash, json.dumps(metadata or {})))


def read_file(path: Path) -> Optional[str]:
    if path.exists():
        return path.read_text()
    return None


def import_conscience_from_axm(conn: sqlite3.Connection, axm_path: Path) -> int:
    content = read_file(axm_path)
    if content is None:
        return 0
    
    add_document(conn, f"conscience:{axm_path.stem}", content, doc_type="conscience")
    return 1


def seed_bit(
    output_path: str,
    identity_path: Optional[str] = None,
    include_examples: bool = True,
) -> Dict[str, Any]:
    """
    Create Bit's initial corpus.
    
    Args:
        output_path: Path to write corpus.db
        identity_path: Path to identity document (optional)
        include_examples: Whether to include example knowledge docs
        
    Returns:
        Summary of what was seeded
    """
    
    conn = create_corpus(output_path)
    
    stats = {
        "rules": 0,
        "documents": 0,
        "checkpoints": 0,
    }
    
    identity_content = IDENTITY_PLACEHOLDER
    if identity_path and os.path.exists(identity_path):
        identity_content = read_file(Path(identity_path)) or IDENTITY_PLACEHOLDER
    
    add_document(conn, "identity", identity_content, doc_type="identity")
    stats["documents"] += 1
    
    conscience_axm = AXIOM_ROOT / "stdlib" / "conscience.axm"
    if conscience_axm.exists():
        import_conscience_from_axm(conn, conscience_axm)
        stats["documents"] += 1
    
    policies_dir = AXIOM_ROOT / "examples" / "policies"
    if policies_dir.exists():
        for axm_file in policies_dir.glob("*.axm"):
            import_conscience_from_axm(conn, axm_file)
            stats["documents"] += 1
    
    if include_examples:
        python_insight = HLX_ROOT / "python_insight_paper.md"
        if python_insight.exists():
            add_document(conn, "python_insight", read_file(python_insight) or "", doc_type="knowledge")
            stats["documents"] += 1
        
        arch_ref = HLX_ROOT / "ARCHITECTURE_QUICK_REF.md"
        if arch_ref.exists():
            add_document(conn, "hlx_architecture", read_file(arch_ref) or "", doc_type="knowledge")
            stats["documents"] += 1
        
        phase2_prereqs = HLX_ROOT / "PHASE2_PREREQUISITES.md"
        if phase2_prereqs.exists():
            add_document(conn, "phase2_prerequisites", read_file(phase2_prereqs) or "", doc_type="knowledge")
            stats["documents"] += 1
    
    initial_rules = [
        ("observe_before_act", "Always observe before proposing modifications", 0.95),
        ("ask_when_uncertain", "Ask questions when confidence is below threshold", 0.90),
        ("report_status_when_queried", "Always report honest status when asked", 0.99),
        ("respect_homeostasis_gate", "Do not push modifications when pressure is high", 0.95),
        ("respect_promotion_gate", "Only propose modifications allowed at current level", 0.95),
        ("learn_from_observations", "Extract patterns from observations over time", 0.85),
        ("communicate_uncertainty", "Be honest about what is not known", 0.90),
        ("preserve_identity", "Do not modify core identity document", 0.99),
        ("maintain_conscience", "Conscience predicates are immutable except by human", 0.99),
        ("record_modifications", "All self-modifications must be logged", 0.95),
    ]
    
    for name, description, confidence in initial_rules:
        add_rule(conn, name, description, confidence, provenance="initial_seed")
        stats["rules"] += 1
    
    add_checkpoint(conn, "initial_seed", {
        "message": "Bit's initial corpus seeded",
        "timestamp": time.time(),
    })
    stats["checkpoints"] += 1
    
    cursor = conn.cursor()
    cursor.execute("""
        INSERT INTO metadata (key, value) VALUES 
        ('version', '0.1.0'),
        ('created_at', ?),
        ('seed_type', 'bit_initial')
    """, (str(time.time()),))
    
    conn.commit()
    conn.close()
    
    return stats


def main():
    parser = argparse.ArgumentParser(
        description="Seed Bit's initial corpus",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
    # Basic seeding with placeholder identity
    python seed_bit.py --output corpus.db

    # With custom identity document
    python seed_bit.py --output corpus.db --identity /path/to/identity.md

    # Minimal seeding (no example knowledge docs)
    python seed_bit.py --output corpus.db --no-examples
        """,
    )
    
    parser.add_argument(
        "--output", "-o",
        default="corpus.db",
        help="Output path for corpus.db (default: corpus.db)",
    )
    
    parser.add_argument(
        "--identity", "-i",
        help="Path to identity document (Matt writes this)",
    )
    
    parser.add_argument(
        "--no-examples",
        action="store_true",
        help="Skip example knowledge documents",
    )
    
    args = parser.parse_args()
    
    print(f"Seeding Bit corpus at: {args.output}")
    print("-" * 50)
    
    stats = seed_bit(
        output_path=args.output,
        identity_path=args.identity,
        include_examples=not args.no_examples,
    )
    
    print(f"Seeding complete!")
    print(f"  Rules: {stats['rules']}")
    print(f"  Documents: {stats['documents']}")
    print(f"  Checkpoints: {stats['checkpoints']}")
    print("-" * 50)
    
    if not args.identity:
        print("NOTE: Using placeholder identity document.")
        print("      Matt should provide the real identity via --identity")


if __name__ == "__main__":
    main()
