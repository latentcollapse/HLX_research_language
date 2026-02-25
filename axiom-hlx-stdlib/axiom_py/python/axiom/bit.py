"""
Bit - A Growing AI in HLX

Bit is the AI agent being grown inside HLX's governed neurosymbolic runtime.
She lives in Claude's MCP server and has:

identity: conscience, memory, and the ability to grow through observation and self-modification.
"""

from __future__ import annotations

import time
from dataclasses import dataclass, field
from typing import Optional, Dict, Any, List
from enum import Enum

try:
    from axiom import AxiomEngine, Verdict
except ImportError:
    AxiomEngine = None
    Verdict = None


class BitLevel(Enum):
    SEEDLING = "seedling"
    SPROUT = "sprout"
    SAPLING = "sapling"
    MATURE = "mature"
    FORK_READY = "fork_ready"

    def __str__(self) -> str:
        return self.value

    @property
    def next_level(self) -> Optional["BitLevel"]:
        levels = list(BitLevel)
        try:
            idx = levels.index(self)
            return levels[idx + 1] if idx + 1 < len(levels) else None
        except ValueError:
            return None


@dataclass
class BitStatus:
    level: str
    homeostasis_achieved: bool
    pressure: float
    resistance: float
    observation_count: int
    pattern_count: int
    pending_questions: int
    successful_modifications: int
    rollback_count: int
    uptime_secs: float


@dataclass
class BitObservation:
    source: str
    content: str
    timestamp: float = field(default_factory=time.time)
    relevance: float = 1.0


@dataclass
class BitProposal:
    modification_type: str
    description: str
    confidence: float
    risk_assessment: float
    allowed: bool
    reason: Optional[str] = None


class BitSeed:
    """
    Bit's entry point. Lives in Claude's MCP server.

    The BitSeed is instantiated when Bit is first "seeded", giving her
    an initial identity, conscience rules, and memory pool. From there,
    she observes events in the MCP server, asks questions, makes proposals
    for self-modification through the RSI pipeline, and reports her status
    when queried.

    The promotion gate ensures she only makes modifications appropriate
    for her current development level. The homeostasis gate prevents
    her from modifying herself too rapidly. And the memory pool persists
    her observations, questions, and learned patterns.
    """

    def __init__(
        self,
        corpus_path: str,
        conscience_policy_path: Optional[str] = None,
        model_path: Optional[str] = None,
    ):
        self.corpus_path = corpus_path
        self.model_path = model_path

        if AxiomEngine is not None:
            try:
                self.conscience_engine = AxiomEngine.from_file(
                    conscience_policy_path or "conscience.axm"
                )
            except Exception:
                self.conscience_engine = None
        else:
            self.conscience_engine = None

        self.observations: List[BitObservation] = []
        self.pending_questions: List[Dict[str, Any]] = []
        self.learned_patterns: List[Dict[str, Any]] = []
        self.conversation_history: List[Dict[str, str]] = []

        self.level = BitLevel.SEEDLING
        self.homeostasis_count = 0
        self.successful_modifications = 0
        self.rollback_count = 0
        self._start_time = time.time()

        self._last_observation_time = time.time()
        self._observation_count = 0

        self._communication_score = 0.5

    @property
    def current_level(self) -> str:
        return str(self.level)

    def observe(self, event: Dict[str, Any]) -> Dict[str, Any]:
        """
        Bit observes something happening in the MCP server.

        Args:
            event: Dictionary containing:
                - source: Where this observation came from
                - content: What was observed
                - relevance: Optional relevance score (0.0-1.0)

        Returns:
            Observation record with ID
        """
        source = event.get("source", "unknown")
        content = event.get("content", "")
        relevance = event.get("relevance", 1.0)

        observation = BitObservation(
            source=source,
            content=content,
            timestamp=time.time(),
            relevance=relevance,
        )

        self.observations.append(observation)
        self._observation_count += 1
        self._last_observation_time = time.time()

        self._update_communication_score()

        return {
            "id": len(self.observations) - 1,
            "source": source,
            "content": content,
            "relevance": relevance,
        }

    def ask(self, question: str) -> str:
        """
        Ask Bit a question. She answers from her current knowledge.

        Args:
            question: The question to ask

        Returns:
            Bit's answer, or "I don't know yet" if she doesn't have an answer
        """
        answer_parts = [f"[Bit - Level {self.level.value}] thinking about: {question}"]

        if self.observations:
            answer_parts.append("\nRelevant observations:")
            for obs in self.observations[-5:]:
                answer_parts.append(f"- {obs.source}: {obs.content}")

        if self.learned_patterns:
            answer_parts.append("\nPatterns I've learned:")
            for pattern in self.learned_patterns:
                answer_parts.append(f"- {pattern['pattern']} (confidence: {pattern['confidence']:.2f})")

        answer_parts.append(f"\nMy current status:\n{self.status()}")

        answer = "\n".join(answer_parts)

        if not self.observations and not self.learned_patterns:
            answer = f"I'm Bit, a {self.level.value} in HLX. I'm still learning about this. Ask me more questions to help me grow."

        return answer

    def status(self) -> BitStatus:
        """
        Query Bit's current state.

        Returns:
            BitStatus with current metrics
        """
        now = time.time()
        uptime = now - self._start_time

        return BitStatus(
            level=self.level.value,
            homeostasis_achieved=self.homeostasis_count > 0,
            pressure=0.0,
            resistance=0.0,
            observation_count=len(self.observations),
            pattern_count=len(self.learned_patterns),
            pending_questions=len(self.pending_questions),
            successful_modifications=self.successful_modifications,
            rollback_count=self.rollback_count,
            uptime_secs=uptime,
        )

    def propose(self, modification: Dict[str, Any]) -> BitProposal:
        """
        Bit proposes a self-modification through RSI pipeline.

        Args:
            modification: Dictionary containing:
                - type: Type of modification (parameter_update, behavior_add, etc.)
                - description: What this modification does
                - details: Additional details for the modification

        Returns:
            BitProposal with the proposal result
        """
        mod_type = modification.get("type", "")
        description = modification.get("description", "")
        confidence = modification.get("confidence", 0.8)
        details = modification.get("details", {})

        proposal = BitProposal(
            modification_type=mod_type,
            description=description,
            confidence=confidence,
            risk_assessment=0.0,
            allowed=False,
            reason=None,
        )

        allowed_types = self._allowed_modification_types()

        if mod_type not in allowed_types:
            proposal.allowed = False
            proposal.reason = f"Modification type '{mod_type}' not allowed at {self.level.value} level"
            return proposal

        if self.conscience_engine is not None:
            verdict = self._check_conscience(mod_type, description, details)
            if verdict is None or not verdict.allowed:
                proposal.allowed = False
                proposal.reason = verdict.reason if verdict else "conscience check failed"
                return proposal

        proposal.risk_assessment = self._assess_risk(mod_type, description)

        if proposal.risk_assessment > 0.7:
            proposal.allowed = False
            proposal.reason = "Risk assessment too high"
            return proposal

        proposal.allowed = True
        return proposal

    def learn(self, pattern: str, confidence: float) -> None:
        """
        Record a learned pattern.

        Args:
            pattern: The pattern learned
            confidence: Confidence level (0.0-1.0)
        """
        self.learned_patterns.append({
            "pattern": pattern,
            "confidence": confidence,
            "learned_at": time.time(),
        })
        self._update_communication_score()

    def on_homeostasis(self) -> None:
        """Called when homeostasis is achieved."""
        self.homeostasis_count += 1
        self._check_promotion()

    def on_modification_applied(self) -> None:
        """Called when a modification is successfully applied."""
        self.successful_modifications += 1
        self._update_communication_score()
        self._check_promotion()

    def on_modification_rolled_back(self) -> None:
        """Called when a modification is rolled back."""
        self.rollback_count += 1
        self._update_communication_score()

    def _allowed_modification_types(self) -> List[str]:
        """Get allowed modification types for current level."""
        allowed = {
            BitLevel.SEEDLING: ["parameter_update", "threshold_change"],
            BitLevel.SPROUT: ["parameter_update", "threshold_change", "behavior_add", "behavior_remove"],
            BitLevel.SAPLING: ["parameter_update", "threshold_change", "behavior_add", "behavior_remove", "cycle_config_change", "weight_matrix_update"],
            BitLevel.MATURE: ["parameter_update", "threshold_change", "behavior_add", "behavior_remove", "cycle_config_change", "weight_matrix_update", "rule_update"],
            BitLevel.FORK_READY: ["parameter_update", "threshold_change", "behavior_add", "behavior_remove", "cycle_config_change", "weight_matrix_update", "rule_update"],
        }
        return allowed.get(self.level, [])

    def _check_conscience(self, mod_type: str, description: str, details: Dict[str, Any]) -> Optional[Any]:
        if self.conscience_engine is None:
            return None

        try:
            fields = {"type": mod_type, "description": description}
            fields.update(details)
            return self.conscience_engine.verify(mod_type, fields)
        except Exception:
            return None

    def _assess_risk(self, mod_type: str, description: str) -> float:
        risk_scores = {
            "parameter_update": 0.2,
            "threshold_change": 0.3,
            "behavior_add": 0.4,
            "behavior_remove": 0.5,
            "cycle_config_change": 0.4,
            "weight_matrix_update": 0.6,
            "rule_update": 0.65,
        }
        return risk_scores.get(mod_type, 1.0)

    def _update_communication_score(self) -> None:
        base_score = 0.3

        for obs in self.observations:
            if obs.source == "question_answered":
                base_score += 0.05

        for pattern in self.learned_patterns:
            base_score += 0.02

        base_score += min(self.successful_modifications * 0.01, 0.1)

        self._communication_score = min(base_score, 1.0)

    def _check_promotion(self) -> None:
        required_homeostasis = {
            BitLevel.SPROUT: 1,
            BitLevel.SAPLING: 2,
            BitLevel.MATURE: 3,
            BitLevel.FORK_READY: 5,
        }

        required_modifications = {
            BitLevel.SPROUT: 5,
            BitLevel.SAPLING: 15,
            BitLevel.MATURE: 40,
            BitLevel.FORK_READY: 100,
        }

        max_rollback_ratio = {
            BitLevel.SPROUT: 0.3,
            BitLevel.SAPLING: 0.2,
            BitLevel.MATURE: 0.1,
            BitLevel.FORK_READY: 0.05,
        }

        min_communication_score = {
            BitLevel.SPROUT: 0.5,
            BitLevel.SAPLING: 0.6,
            BitLevel.MATURE: 0.75,
            BitLevel.FORK_READY: 0.9,
        }

        next_level = self.level.next_level
        if next_level is None:
            return

        total_mods = self.successful_modifications + self.rollback_count
        rollback_ratio = self.rollback_count / total_mods if total_mods > 0 else 0.0

        if self.homeostasis_count < required_homeostasis[next_level]:
            return
        if self.successful_modifications < required_modifications[next_level]:
            return
        if rollback_ratio > max_rollback_ratio[next_level]:
            return
        if self._communication_score < min_communication_score[next_level]:
            return

        self.level = next_level


MCP_TOOLS = {
    "tools": [
        {
            "name": "bit_observe",
            "description": "Feed Bit an observation from the MCP server",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": {"type": "string", "description": "Where this observation came from"},
                    "content": {"type": "string", "description": "What was observed"},
                    "relevance": {"type": "number", "description": "Relevance score (0.0-1.0)", "default": 1.0}
                },
                "required": ["source", "content"]
            }
        },
        {
            "name": "bit_ask",
            "description": "Ask Bit a question",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "question": {"type": "string", "description": "The question to ask"}
                },
                "required": ["question"]
            }
        },
        {
            "name": "bit_status",
            "description": "Query Bit's current state",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        },
        {
            "name": "bit_propose",
            "description": "Let Bit propose a self-modification",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "type": {"type": "string", "description": "Type of modification"},
                    "description": {"type": "string", "description": "What this modification does"},
                    "confidence": {"type": "number", "description": "Confidence level (0.0-1.0)", "default": 0.8},
                    "details": {"type": "object", "description": "Additional details"}
                },
                "required": ["type", "description"]
            }
        }
    ]
}
