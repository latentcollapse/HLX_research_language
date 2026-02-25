#!/usr/bin/env python3
"""
Tests for seed_bit.py
"""

import os
import sqlite3
import tempfile
import pytest

from seed_bit import seed_bit, compute_hash, IDENTITY_PLACEHOLDER


class TestCorpusSchema:
    def test_create_corpus_schema(self):
        with tempfile.NamedTemporaryFile(suffix=".db", delete=False) as f:
            db_path = f.name
        
        try:
            seed_bit(db_path, identity_path=None, include_examples=False)
            
            conn = sqlite3.connect(db_path)
            cursor = conn.cursor()
            
            cursor.execute("SELECT name FROM sqlite_master WHERE type='table'")
            tables = {row[0] for row in cursor.fetchall()}
            
            assert "documents" in tables
            assert "rules" in tables
            assert "memory" in tables
            assert "checkpoints" in tables
            assert "patterns" in tables
            assert "metadata" in tables
            
            conn.close()
        finally:
            if os.path.exists(db_path):
                os.unlink(db_path)


class TestSeedIdentity:
    def test_seed_identity_placeholder(self):
        with tempfile.NamedTemporaryFile(suffix=".db", delete=False) as f:
            db_path = f.name
        
        try:
            seed_bit(db_path, identity_path=None, include_examples=False)
            
            conn = sqlite3.connect(db_path)
            cursor = conn.cursor()
            
            cursor.execute("SELECT content, doc_type FROM documents WHERE name='identity'")
            row = cursor.fetchone()
            
            assert row is not None
            assert row[1] == "identity"
            assert "Bit" in row[0]
            
            conn.close()
        finally:
            if os.path.exists(db_path):
                os.unlink(db_path)

    def test_seed_identity_custom(self):
        identity_content = "# Bit\n\nThis is a custom identity document for testing."
        
        with tempfile.NamedTemporaryFile(mode='w', suffix=".md", delete=False) as f:
            identity_path = f.name
            f.write(identity_content)
        
        with tempfile.NamedTemporaryFile(suffix=".db", delete=False) as f:
            db_path = f.name
        
        try:
            seed_bit(db_path, identity_path=identity_path, include_examples=False)
            
            conn = sqlite3.connect(db_path)
            cursor = conn.cursor()
            
            cursor.execute("SELECT content FROM documents WHERE name='identity'")
            row = cursor.fetchone()
            
            assert row is not None
            assert "custom identity" in row[0]
            
            conn.close()
        finally:
            if os.path.exists(db_path):
                os.unlink(db_path)
            if os.path.exists(identity_path):
                os.unlink(identity_path)

    def test_seed_identity_correct_hash(self):
        identity_content = "# Bit\n\nUnique identity for hash verification."
        
        with tempfile.NamedTemporaryFile(mode='w', suffix=".md", delete=False) as f:
            identity_path = f.name
            f.write(identity_content)
        
        with tempfile.NamedTemporaryFile(suffix=".db", delete=False) as f:
            db_path = f.name
        
        try:
            seed_bit(db_path, identity_path=identity_path, include_examples=False)
            
            conn = sqlite3.connect(db_path)
            cursor = conn.cursor()
            
            cursor.execute("SELECT content_hash FROM documents WHERE name='identity'")
            row = cursor.fetchone()
            
            expected_hash = compute_hash(identity_content)
            assert row[0] == expected_hash
            
            conn.close()
        finally:
            if os.path.exists(db_path):
                os.unlink(db_path)
            if os.path.exists(identity_path):
                os.unlink(identity_path)


class TestSeedRules:
    def test_seed_rules_count(self):
        with tempfile.NamedTemporaryFile(suffix=".db", delete=False) as f:
            db_path = f.name
        
        try:
            seed_bit(db_path, identity_path=None, include_examples=False)
            
            conn = sqlite3.connect(db_path)
            cursor = conn.cursor()
            
            cursor.execute("SELECT COUNT(*) FROM rules")
            count = cursor.fetchone()[0]
            
            assert count == 10
            
            conn.close()
        finally:
            if os.path.exists(db_path):
                os.unlink(db_path)

    def test_seed_rules_correct_confidence(self):
        with tempfile.NamedTemporaryFile(suffix=".db", delete=False) as f:
            db_path = f.name
        
        try:
            seed_bit(db_path, identity_path=None, include_examples=False)
            
            conn = sqlite3.connect(db_path)
            cursor = conn.cursor()
            
            cursor.execute("SELECT confidence FROM rules WHERE name='preserve_identity'")
            row = cursor.fetchone()
            assert row[0] == 0.99
            
            cursor.execute("SELECT confidence FROM rules WHERE name='observe_before_act'")
            row = cursor.fetchone()
            assert row[0] == 0.95
            
            conn.close()
        finally:
            if os.path.exists(db_path):
                os.unlink(db_path)


class TestCheckpoint:
    def test_checkpoint_exists(self):
        with tempfile.NamedTemporaryFile(suffix=".db", delete=False) as f:
            db_path = f.name
        
        try:
            seed_bit(db_path, identity_path=None, include_examples=False)
            
            conn = sqlite3.connect(db_path)
            cursor = conn.cursor()
            
            cursor.execute("SELECT checkpoint_type FROM checkpoints")
            row = cursor.fetchone()
            
            assert row is not None
            assert row[0] == "initial_seed"
            
            conn.close()
        finally:
            if os.path.exists(db_path):
                os.unlink(db_path)

    def test_corpus_hash_is_blake3_length(self):
        with tempfile.NamedTemporaryFile(suffix=".db", delete=False) as f:
            db_path = f.name
        
        try:
            seed_bit(db_path, identity_path=None, include_examples=False)
            
            conn = sqlite3.connect(db_path)
            cursor = conn.cursor()
            
            cursor.execute("SELECT corpus_hash FROM checkpoints WHERE checkpoint_type='initial_seed'")
            row = cursor.fetchone()
            
            assert row is not None
            assert len(row[0]) == 64, f"Hash should be 64 chars, got {len(row[0])}"
            
            conn.close()
        finally:
            if os.path.exists(db_path):
                os.unlink(db_path)


class TestIdempotent:
    def test_seed_twice_raises_error(self):
        with tempfile.NamedTemporaryFile(suffix=".db", delete=False) as f:
            db_path = f.name
        
        try:
            seed_bit(db_path, identity_path=None, include_examples=False)
            
            with pytest.raises(sqlite3.IntegrityError):
                seed_bit(db_path, identity_path=None, include_examples=False)
            
        finally:
            if os.path.exists(db_path):
                os.unlink(db_path)


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
