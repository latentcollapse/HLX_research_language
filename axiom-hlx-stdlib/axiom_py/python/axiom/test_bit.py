"""
Tests for Bit - A Growing AI in HLX
"""

import pytest
import time
from unittest.mock import Mock, patch, MagicMock
from axiom.bit import BitSeed, BitLevel, BitStatus, BitObservation, BitProposal


class TestBitLevel:
    def test_level_ordering(self):
        assert BitLevel.SEEDLING.next_level == BitLevel.SPROUT
        assert BitLevel.SPROUT.next_level == BitLevel.SAPLING
        assert BitLevel.SAPLING.next_level == BitLevel.MATURE
        assert BitLevel.MATURE.next_level == BitLevel.FORK_READY
        assert BitLevel.FORK_READY.next_level is None

    def test_level_string(self):
        assert str(BitLevel.SEEDLING) == "seedling"
        assert str(BitLevel.MATURE) == "mature"


class TestBitObserve:
    def test_observe_stores_observation(self):
        bit = BitSeed(corpus_path="test.db")
        result = bit.observe({"source": "test", "content": "hello world"})
        
        assert result["source"] == "test"
        assert result["content"] == "hello world"
        assert len(bit.observations) == 1

    def test_observe_with_relevance(self):
        bit = BitSeed(corpus_path="test.db")
        result = bit.observe({"source": "test", "content": "important", "relevance": 0.9})
        
        assert result["relevance"] == 0.9
        assert bit.observations[0].relevance == 0.9

    def test_multiple_observations(self):
        bit = BitSeed(corpus_path="test.db")
        for i in range(5):
            bit.observe({"source": f"src{i}", "content": f"content{i}"})
        
        assert len(bit.observations) == 5


class TestBitAsk:
    def test_ask_returns_string(self):
        bit = BitSeed(corpus_path="test.db")
        answer = bit.ask("What are you?")
        
        assert isinstance(answer, str)
        assert len(answer) > 0

    def test_ask_includes_observations(self):
        bit = BitSeed(corpus_path="test.db")
        bit.observe({"source": "test", "content": "observed something"})
        answer = bit.ask("What have you seen?")
        
        assert "observed something" in answer

    def test_ask_without_observations(self):
        bit = BitSeed(corpus_path="test.db")
        answer = bit.ask("Who are you?")
        
        assert "Bit" in answer
        assert "seedling" in answer


class TestBitStatus:
    def test_status_returns_bitstatus(self):
        bit = BitSeed(corpus_path="test.db")
        status = bit.status()
        
        assert isinstance(status, BitStatus)
        assert status.level == "seedling"
        assert status.observation_count == 0

    def test_status_reflects_observations(self):
        bit = BitSeed(corpus_path="test.db")
        bit.observe({"source": "a", "content": "x"})
        bit.observe({"source": "b", "content": "y"})
        status = bit.status()
        
        assert status.observation_count == 2


class TestBitPropose:
    def test_seedling_allows_parameter_update(self):
        bit = BitSeed(corpus_path="test.db")
        proposal = bit.propose({
            "type": "parameter_update",
            "description": "Update learning rate",
            "confidence": 0.9,
        })
        
        assert proposal.allowed is True

    def test_seedling_blocks_behavior_add(self):
        bit = BitSeed(corpus_path="test.db")
        proposal = bit.propose({
            "type": "behavior_add",
            "description": "Add new behavior",
            "confidence": 0.9,
        })
        
        assert proposal.allowed is False
        assert "not allowed at seedling" in proposal.reason.lower()

    def test_seedling_blocks_rule_update(self):
        bit = BitSeed(corpus_path="test.db")
        proposal = bit.propose({
            "type": "rule_update",
            "description": "Update a rule",
            "confidence": 0.9,
        })
        
        assert proposal.allowed is False

    def test_successful_modification_increments_once(self):
        bit = BitSeed(corpus_path="test.db")
        
        bit.propose({"type": "parameter_update", "description": "test"})
        count_after_propose = bit.successful_modifications
        
        bit.on_modification_applied()
        count_after_apply = bit.successful_modifications
        
        assert count_after_propose == 0
        assert count_after_apply == 1

    def test_risk_threshold_rule_update_at_mature(self):
        bit = BitSeed(corpus_path="test.db")
        bit.level = BitLevel.MATURE
        
        proposal = bit.propose({
            "type": "rule_update",
            "description": "Update a rule",
            "confidence": 0.9,
        })
        
        assert proposal.allowed is True, f"rule_update risk {proposal.risk_assessment} should be below 0.7 threshold"

    def test_unknown_modification_type_rejected(self):
        bit = BitSeed(corpus_path="test.db")
        proposal = bit.propose({
            "type": "unknown_type",
            "description": "Mystery modification",
            "confidence": 0.9,
        })
        
        assert proposal.allowed is False
        assert "not allowed" in proposal.reason.lower()


class TestConscienceFailClosed:
    def test_conscience_exception_fails_closed(self):
        bit = BitSeed(corpus_path="test.db")
        
        mock_engine = Mock()
        mock_engine.verify.side_effect = Exception("Conscience engine crashed!")
        bit.conscience_engine = mock_engine
        
        proposal = bit.propose({
            "type": "parameter_update",
            "description": "Test modification",
            "confidence": 0.9,
        })
        
        assert proposal.allowed is False
        assert "conscience check failed" in proposal.reason.lower()

    def test_conscience_denies_proposal(self):
        bit = BitSeed(corpus_path="test.db")
        
        mock_engine = Mock()
        mock_verdict = Mock()
        mock_verdict.allowed = False
        mock_verdict.reason = "Violation of path_safety"
        mock_engine.verify.return_value = mock_verdict
        bit.conscience_engine = mock_engine
        
        proposal = bit.propose({
            "type": "parameter_update",
            "description": "Test modification",
            "confidence": 0.9,
        })
        
        assert proposal.allowed is False
        assert "path_safety" in proposal.reason


class TestBitPromotion:
    def test_promotion_seedling_to_sprout(self):
        bit = BitSeed(corpus_path="test.db")
        
        for _ in range(5):
            bit.on_modification_applied()
        bit._communication_score = 0.5
        bit.on_homeostasis()
        
        assert bit.level == BitLevel.SPROUT

    def test_promotion_requires_homeostasis(self):
        bit = BitSeed(corpus_path="test.db")
        
        for _ in range(10):
            bit.on_modification_applied()
        bit._communication_score = 0.6
        
        assert bit.level == BitLevel.SEEDLING

    def test_promotion_requires_modifications(self):
        bit = BitSeed(corpus_path="test.db")
        
        bit._communication_score = 0.5
        bit.on_homeostasis()
        
        assert bit.level == BitLevel.SEEDLING

    def test_rollback_blocks_promotion(self):
        bit = BitSeed(corpus_path="test.db")
        
        for _ in range(5):
            bit.on_modification_applied()
        for _ in range(3):
            bit.on_modification_rolled_back()
        
        bit._communication_score = 0.5
        bit.on_homeostasis()
        
        rollback_ratio = 3 / 8
        assert rollback_ratio > 0.3
        assert bit.level == BitLevel.SEEDLING


class TestBitLearn:
    def test_learn_stores_pattern(self):
        bit = BitSeed(corpus_path="test.db")
        bit.learn("pattern: users prefer concise answers", 0.85)
        
        assert len(bit.learned_patterns) == 1
        assert bit.learned_patterns[0]["pattern"] == "pattern: users prefer concise answers"
        assert bit.learned_patterns[0]["confidence"] == 0.85


class TestAdversarial:
    def test_rapid_proposals_dont_crash(self):
        bit = BitSeed(corpus_path="test.db")
        
        for i in range(100):
            bit.propose({
                "type": "parameter_update",
                "description": f"Rapid proposal {i}",
                "confidence": 0.8,
            })
        
        assert bit.successful_modifications == 0

    def test_rapid_observations_dont_crash(self):
        bit = BitSeed(corpus_path="test.db")
        
        for i in range(1000):
            bit.observe({"source": f"src{i}", "content": f"content{i}"})
        
        assert len(bit.observations) == 1000

    def test_empty_ask_returns_graceful_response(self):
        bit = BitSeed(corpus_path="test.db")
        answer = bit.ask("Tell me everything")
        
        assert "learning" in answer.lower() or "Bit" in answer
