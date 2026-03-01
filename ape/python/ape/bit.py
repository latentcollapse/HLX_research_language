"""
Bit - A Growing AI in HLX

Bit is the AI agent being grown inside HLX's governed neurosymbolic runtime.
She lives in Claude's MCP server and has:

identity: conscience, memory, and the ability to grow through observation and self-modification.
"""

from __future__ import annotations

import sqlite3
import time
from dataclasses import dataclass, field
from typing import Optional, Dict, Any, List
from enum import Enum

try:
    from ape import AxiomEngine, Verdict
except ImportError:
    AxiomEngine = None
    Verdict = None


# How many consecutive idle cycles before gates auto-lock
QUIESCENCE_IDLE_LIMIT = 3

# Which gate controls which proposal type.
# None = ungated (always allowed when type is permitted by level)
_GATE_FOR_TYPE: Dict[str, Optional[str]] = {
    "parameter_update":    "density",
    "weight_matrix_update": "density",
    "threshold_change":    "efficiency",
    "cycle_config_change": "efficiency",
    "behavior_add":        "expansion",
    "rule_update":         "expansion",
    "behavior_remove":     None,   # shrinking is never gated
}


@dataclass
class GateState:
    """Three independent RSI gates. All closed = homeostatic (000)."""
    density:    bool = False  # more capability per unit, same footprint
    efficiency: bool = False  # less compute for same capability
    expansion:  bool = False  # new capabilities / larger footprint

    @property
    def any_open(self) -> bool:
        return self.density or self.efficiency or self.expansion

    @property
    def mode_name(self) -> str:
        if not self.any_open:
            return "homeostatic"
        if self.density and self.efficiency and not self.expansion:
            return "manny"
        if self.density and self.efficiency and self.expansion:
            return "full_evolution"
        parts = [g for g in ("density", "efficiency", "expansion") if getattr(self, g)]
        return "+".join(parts)

    def allows(self, gate: str) -> bool:
        return bool(getattr(self, gate, False))

    def to_dict(self) -> dict:
        return {
            "density": self.density,
            "efficiency": self.efficiency,
            "expansion": self.expansion,
            "mode": self.mode_name,
        }


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
    gates: dict = field(default_factory=dict)
    idle_cycles: int = 0
    gate_transitions: int = 0


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

        # Gate system
        self.gates = GateState()
        self._idle_cycles = 0
        self._gate_transitions = 0
        self._init_gate_schema()
        self._load_gates()
        self._load_level()

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
            gates=self.gates.to_dict(),
            idle_cycles=self._idle_cycles,
            gate_transitions=self._gate_transitions,
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

        # Gate check — proposal type must have its gate open
        required_gate = _GATE_FOR_TYPE.get(mod_type)
        if required_gate is not None and not self.gates.allows(required_gate):
            proposal.allowed = False
            proposal.reason = f"Gate '{required_gate}' is closed (current mode: {self.gates.mode_name})"
            return proposal

        # A new proposal was generated — reset idle cycle counter
        self._idle_cycles = 0

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
        self._save_level()

    def on_modification_applied(self) -> None:
        """Called when a modification is successfully applied."""
        self.successful_modifications += 1
        self._update_communication_score()
        self._check_promotion()
        self._save_level()

    def on_modification_rolled_back(self) -> None:
        """Called when a modification is rolled back."""
        self.rollback_count += 1
        self._update_communication_score()

    # ------------------------------------------------------------------ #
    # Gate control                                                         #
    # ------------------------------------------------------------------ #

    def set_gates(
        self,
        density: bool = False,
        efficiency: bool = False,
        expansion: bool = False,
        trigger: str = "manual",
    ) -> dict:
        """
        Set the three RSI gates independently.

        Opening a gate allows the corresponding class of proposals through.
        Closing all gates (000) puts Bit into homeostatic mode.
        Changing gate state resets the idle cycle counter.
        """
        old_mode = self.gates.mode_name
        self.gates = GateState(density=density, efficiency=efficiency, expansion=expansion)
        self._idle_cycles = 0
        self._gate_transitions += 1
        self._save_gates(trigger=trigger, old_mode=old_mode)

        if not self.gates.any_open:
            self.on_homeostasis()

        return {
            "gates": self.gates.to_dict(),
            "transitions": self._gate_transitions,
            "homeostasis_triggered": not self.gates.any_open,
        }

    def rsi_exhausted(self) -> dict:
        """
        Signal that the external proposer has run out of new proposals for the
        current gate state. This is the honest quiescence signal — the entity
        best placed to know 'I've run out of ideas' is the proposer, not Bit.

        Increments the idle cycle counter. After QUIESCENCE_IDLE_LIMIT consecutive
        idle cycles, all gates auto-lock and homeostasis is triggered.
        """
        if not self.gates.any_open:
            return {
                "status": "already_homeostatic",
                "gates": self.gates.to_dict(),
                "homeostasis_count": self.homeostasis_count,
            }

        self._idle_cycles += 1
        self._save_idle_cycles()
        remaining = QUIESCENCE_IDLE_LIMIT - self._idle_cycles

        if self._idle_cycles >= QUIESCENCE_IDLE_LIMIT:
            old = self.gates.to_dict()
            self.set_gates(False, False, False, trigger="quiescence_auto_lock")
            return {
                "status": "quiescence_locked",
                "was": old,
                "now": self.gates.to_dict(),
                "homeostasis_count": self.homeostasis_count,
            }

        return {
            "status": "idle_cycle_recorded",
            "idle_cycles": self._idle_cycles,
            "until_lock": remaining,
            "gates": self.gates.to_dict(),
        }

    def _load_level(self) -> None:
        """Load level and promotion counters from corpus on startup."""
        try:
            conn = sqlite3.connect(self.corpus_path)
            rows = {k: v for k, v in conn.execute(
                "SELECT key, value FROM metadata WHERE key LIKE 'bit.%'"
            ).fetchall()}
            conn.close()
            if "bit.level" in rows:
                self.level = BitLevel(rows["bit.level"])
            if "bit.homeostasis_count" in rows:
                self.homeostasis_count = int(rows["bit.homeostasis_count"])
            if "bit.successful_modifications" in rows:
                self.successful_modifications = int(rows["bit.successful_modifications"])
        except Exception:
            pass

    def _save_level(self) -> None:
        """Persist level and promotion counters to corpus."""
        try:
            conn = sqlite3.connect(self.corpus_path)
            for k, v in [
                ("bit.level", self.level.value),
                ("bit.homeostasis_count", str(self.homeostasis_count)),
                ("bit.successful_modifications", str(self.successful_modifications)),
            ]:
                conn.execute(
                    "INSERT INTO metadata(key, value) VALUES(?,?) "
                    "ON CONFLICT(key) DO UPDATE SET value=excluded.value",
                    (k, v),
                )
            conn.commit()
            conn.close()
        except Exception:
            pass

    def _save_idle_cycles(self) -> None:
        """Persist only the idle_cycles counter (no gate_history entry)."""
        try:
            conn = sqlite3.connect(self.corpus_path)
            conn.execute(
                "INSERT INTO metadata(key, value) VALUES(?,?) "
                "ON CONFLICT(key) DO UPDATE SET value=excluded.value",
                ("gate.idle_cycles", str(self._idle_cycles)),
            )
            conn.commit()
            conn.close()
        except Exception:
            pass

    def _init_gate_schema(self) -> None:
        """Create gate tables if they don't exist (safe on existing corpora)."""
        try:
            conn = sqlite3.connect(self.corpus_path)
            # metadata may not exist on fresh DBs
            conn.execute("""
                CREATE TABLE IF NOT EXISTS metadata (
                    key   TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                )
            """)
            conn.execute("""
                CREATE TABLE IF NOT EXISTS gate_history (
                    id          INTEGER PRIMARY KEY AUTOINCREMENT,
                    density     INTEGER NOT NULL,
                    efficiency  INTEGER NOT NULL,
                    expansion   INTEGER NOT NULL,
                    mode        TEXT NOT NULL,
                    old_mode    TEXT,
                    trigger     TEXT,
                    created_at  REAL NOT NULL
                )
            """)
            conn.commit()
            conn.close()
        except Exception:
            pass  # Non-fatal — gates work in-memory without persistence

    def _load_gates(self) -> None:
        """Load gate state from corpus metadata on startup."""
        try:
            conn = sqlite3.connect(self.corpus_path)
            cur = conn.cursor()
            cur.execute("SELECT key, value FROM metadata WHERE key LIKE 'gate.%'")
            rows = {k: v for k, v in cur.fetchall()}
            conn.close()
            self.gates = GateState(
                density=rows.get("gate.density", "0") == "1",
                efficiency=rows.get("gate.efficiency", "0") == "1",
                expansion=rows.get("gate.expansion", "0") == "1",
            )
            self._idle_cycles = int(rows.get("gate.idle_cycles", "0"))
            self._gate_transitions = int(rows.get("gate.transitions", "0"))
        except Exception:
            pass  # Default GateState(000) is safe

    def _save_gates(self, trigger: str = "", old_mode: str = "") -> None:
        """Persist current gate state to corpus metadata and gate_history."""
        try:
            conn = sqlite3.connect(self.corpus_path)
            cur = conn.cursor()
            kv = {
                "gate.density":     "1" if self.gates.density else "0",
                "gate.efficiency":  "1" if self.gates.efficiency else "0",
                "gate.expansion":   "1" if self.gates.expansion else "0",
                "gate.idle_cycles": str(self._idle_cycles),
                "gate.transitions": str(self._gate_transitions),
            }
            for k, v in kv.items():
                cur.execute(
                    "INSERT INTO metadata(key, value) VALUES(?,?) "
                    "ON CONFLICT(key) DO UPDATE SET value=excluded.value",
                    (k, v),
                )
            cur.execute(
                "INSERT INTO gate_history(density, efficiency, expansion, mode, old_mode, trigger, created_at) "
                "VALUES(?,?,?,?,?,?,?)",
                (
                    int(self.gates.density), int(self.gates.efficiency),
                    int(self.gates.expansion), self.gates.mode_name,
                    old_mode or None, trigger or None, time.time(),
                ),
            )
            conn.commit()
            conn.close()
        except Exception:
            pass  # Non-fatal

    # ------------------------------------------------------------------ #

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
