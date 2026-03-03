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
        self._init_beliefs_schema()
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


    def reason_symbolically(self, question: str) -> tuple[str, float]:
        """
        Reason about a question using symbolic knowledge from corpus and learned patterns.
        
        Returns:
            Tuple of (symbolic_answer, confidence_0_to_1)
        """
        import string
        import re
        
        # Tokenize question into word set (underscores → spaces so compound names split)
        question_lower = re.sub(r'[\W_]+', ' ', question.lower())
        question_words = set(question_lower.split())
        
        if not question_words:
            return ("", 0.0)
        
        matches = []
        
        # Query corpus rules
        try:
            conn = sqlite3.connect(self.corpus_path)
            cursor = conn.cursor()
            
            # Get rules (schema: name, description, confidence — no content column)
            cursor.execute(
                "SELECT name, description, confidence FROM rules ORDER BY confidence DESC"
            )
            for row in cursor.fetchall():
                name, description, confidence = row
                rule_text = f"{name} {description}"
                rule_words = set(re.sub(r'[\W_]+', ' ', rule_text.lower()).split())
                overlap = len(question_words & rule_words)
                if overlap > 0:
                    matches.append({
                        'type': 'rule',
                        'name': name,
                        'description': description,
                        'confidence': confidence,
                        'overlap': overlap
                    })
            
            # Get recent memory (last 30)
            cursor.execute(
                "SELECT source, content, relevance FROM memory ORDER BY relevance DESC, created_at DESC LIMIT 30"
            )
            for row in cursor.fetchall():
                source, content, relevance = row
                mem_text = f"{source} {content}"
                mem_words = set(re.sub(r'[\W_]+', ' ', mem_text.lower()).split())
                overlap = len(question_words & mem_words)
                if overlap > 0:
                    matches.append({
                        'type': 'memory',
                        'source': source,
                        'content': content,
                        'confidence': relevance,
                        'overlap': overlap
                    })
            
            # Get documents (K-12 curriculum, encyclopedia articles)
            cursor.execute(
                "SELECT name, content, doc_type FROM documents WHERE length(content) < 2000 ORDER BY created_at DESC LIMIT 20"
            )
            for row in cursor.fetchall():
                name, content, doc_type = row
                doc_text = f"{name} {content}"
                doc_words = set(re.sub(r'[\W_]+', ' ', doc_text.lower()).split())
                overlap = len(question_words & doc_words)
                if overlap > 0:
                    matches.append({
                        'type': 'document', 'name': name, 'content': content,
                        'confidence': 0.8, 'overlap': overlap
                    })

            conn.close()
        except Exception as e:
            # If DB fails, continue with learned patterns only
            pass
        
        # Score learned patterns
        for pattern in self.learned_patterns:
            pattern_text = pattern.get('pattern', '')
            pattern_conf = pattern.get('confidence', 0.5)
            pattern_words = set(re.sub(r'[\W_]+', ' ', pattern_text.lower()).split())
            overlap = len(question_words & pattern_words)
            if overlap > 0:
                matches.append({
                    'type': 'pattern',
                    'content': pattern_text,
                    'confidence': pattern_conf,
                    'overlap': overlap
                })
        
        if not matches:
            return ("", 0.0)
        
        # Sort by overlap (best matches first)
        matches.sort(key=lambda x: x['overlap'], reverse=True)
        
        # Calculate confidence
        confidence = 0.0
        best_rule = next((m for m in matches if m['type'] == 'rule'), None)
        best_pattern = next((m for m in matches if m['type'] == 'pattern'), None)
        
        if best_rule:
            rule_contrib = best_rule['confidence'] * min(best_rule['overlap'] / 3.0, 1.0)
            confidence += rule_contrib
        
        if best_pattern:
            pattern_contrib = best_pattern['confidence'] * 0.7 * min(best_pattern['overlap'] / 2.0, 1.0)
            confidence += pattern_contrib
        
        confidence = min(confidence, 1.0)
        
        # Build answer from top matches
        answer_parts = []
        
        # Add top documents (up to 2) - K-12 and encyclopedia
        documents = [m for m in matches[:5] if m['type'] == 'document'][:2]
        if documents:
            answer_parts.append("From my knowledge base:")
            for doc in documents:
                content_preview = doc['content'][:200] + "..." if len(doc['content']) > 200 else doc['content']
                answer_parts.append(f"  - {content_preview}")
        
        # Add top rules (up to 3)
        rules = [m for m in matches[:5] if m['type'] == 'rule'][:3]
        if rules:
            answer_parts.append("Based on my conscience rules:")
            for rule in rules:
                answer_parts.append(f"  - {rule['name']}: {rule['description']}")
        
        # Add top patterns (up to 2)
        patterns = [m for m in matches[:5] if m['type'] == 'pattern'][:2]
        if patterns:
            if rules:
                answer_parts.append("\nAnd from learned patterns:")
            else:
                answer_parts.append("From my learned patterns:")
            for pat in patterns:
                answer_parts.append(f"  - {pat['content']}")
        
        # Add memory context if relevant (up to 1)
        memory = next((m for m in matches if m['type'] == 'memory'), None)
        if memory:
            answer_parts.append(f"\nContext from {memory['source']}: {memory['content']}")

        answer = "\n".join(answer_parts) if answer_parts else ""
        return (answer, confidence)

    def ask(self, question: str) -> str:
        """
        Ask Bit a question. She answers from her current knowledge.

        Args:
            question: The question to ask

        Returns:
            Bit's answer, or "I don't know yet" if she doesn't have an answer
        """
        # Phase 19B: Try belief system first for self-referential questions
        belief_answer, belief_conf = self.answer_from_beliefs(question)
        if belief_answer and belief_conf >= 0.5:
            return f"[Symbolic] {belief_answer} (confidence: {belief_conf:.2f})"

        # Fall back to symbolic reasoning
        symbolic_answer, symbolic_conf = self.reason_symbolically(question)
        if symbolic_answer and symbolic_conf >= 0.4:
            return f"[Symbolic] {symbolic_answer} (confidence: {symbolic_conf:.2f})"

        # General response with observations and patterns
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

    # ------------------------------------------------------------------ #
    # Belief system - Phase 19B                                          #
    # ------------------------------------------------------------------ #

    def add_belief(
        self,
        subject: str,
        predicate: str,
        obj: str,
        raw_source: str = "",
        source_type: str = "training",
        confidence: float = 0.5
    ) -> dict:
        """
        Add a belief to Bitsy's self-model.

        Beliefs with subject="I" form her self-identity. Duplicate beliefs
        reinforce existing entries (confidence increases).

        Args:
            subject: The subject ("I", "Matt", etc.)
            predicate: The relationship ("am", "name is", "built", etc.)
            obj: The object/value ("Bitsy", "an AI", etc.)
            raw_source: Original text before transformation
            source_type: "training", "observation", "inference", or "bond"
            confidence: Initial confidence (0.0-1.0)

        Returns:
            Dict with belief status and handle
        """
        import hashlib

        content = f"{subject}:{predicate}:{obj}"
        content_hash = hashlib.sha256(content.encode()).hexdigest()[:16]

        try:
            conn = sqlite3.connect(self.corpus_path)
            cursor = conn.cursor()

            cursor.execute(
                "SELECT id, confidence, reinforcement_count FROM beliefs WHERE content_hash = ?",
                (content_hash,)
            )
            row = cursor.fetchone()

            if row:
                belief_id, old_conf, old_count = row
                new_conf = min(old_conf + 0.05, 1.0)
                new_count = old_count + 1

                cursor.execute(
                    """UPDATE beliefs
                       SET confidence = ?, reinforcement_count = ?, created_at = CURRENT_TIMESTAMP
                       WHERE id = ?""",
                    (new_conf, new_count, belief_id)
                )
                conn.commit()
                conn.close()

                return {
                    "status": "reinforced",
                    "belief_id": belief_id,
                    "confidence": new_conf,
                    "reinforcement_count": new_count,
                }
            else:
                cursor.execute(
                    """INSERT INTO beliefs
                       (subject, predicate, object, raw_source, source_type, confidence, content_hash)
                       VALUES (?, ?, ?, ?, ?, ?, ?)""",
                    (subject, predicate, obj, raw_source, source_type, confidence, content_hash)
                )
                belief_id = cursor.lastrowid
                conn.commit()
                conn.close()

                return {
                    "status": "created",
                    "belief_id": belief_id,
                    "confidence": confidence,
                    "reinforcement_count": 1,
                }
        except Exception as e:
            return {"status": "error", "error": str(e)}

    def query_beliefs(
        self,
        subject: str = None,
        predicate: str = None,
        min_confidence: float = 0.0,
        limit: int = 10
    ) -> list:
        """
        Query Bitsy's beliefs.

        Args:
            subject: Filter by subject (e.g., "I" for self-model)
            predicate: Filter by predicate (e.g., "am", "name is")
            min_confidence: Minimum confidence threshold
            limit: Max results to return

        Returns:
            List of belief dicts with subject, predicate, object, confidence
        """
        try:
            conn = sqlite3.connect(self.corpus_path)
            cursor = conn.cursor()

            query = "SELECT subject, predicate, object, confidence, source_type, reinforcement_count FROM beliefs WHERE confidence >= ?"
            params = [min_confidence]

            if subject:
                query += " AND subject = ?"
                params.append(subject)
            if predicate:
                query += " AND predicate LIKE ?"
                params.append(f"%{predicate}%")

            query += " ORDER BY confidence DESC, reinforcement_count DESC LIMIT ?"
            params.append(limit)

            cursor.execute(query, params)
            rows = cursor.fetchall()
            conn.close()

            return [
                {
                    "subject": row[0],
                    "predicate": row[1],
                    "object": row[2],
                    "confidence": row[3],
                    "source_type": row[4],
                    "reinforcement_count": row[5],
                }
                for row in rows
            ]
        except Exception:
            return []

    def get_self_model(self) -> dict:
        """
        Get Bitsy's self-model - all beliefs where subject="I".

        Returns:
            Dict with beliefs grouped by predicate type
        """
        beliefs = self.query_beliefs(subject="I", min_confidence=0.3)

        identity = {
            "name": [],
            "nature": [],
            "capabilities": [],
            "possessions": [],
            "relationships": [],
            "all": beliefs,
        }

        for b in beliefs:
            pred = b["predicate"].lower()
            if "name" in pred:
                identity["name"].append(b)
            elif pred in ("am", "was"):
                identity["nature"].append(b)
            elif pred == "can":
                identity["capabilities"].append(b)
            elif pred in ("have", "has"):
                identity["possessions"].append(b)
            elif "built" in pred or "created" in pred or "by" in pred:
                identity["relationships"].append(b)
            else:
                identity["nature"].append(b)

        return identity

    def answer_from_beliefs(self, question: str) -> tuple[str, float]:
        """
        Attempt to answer a question using the belief system.

        Returns (answer, confidence). If no good match, returns ("", 0.0).
        """
        import re

        question_lower = question.lower()

        if re.search(r'what is your name|who are you', question_lower):
            beliefs = self.query_beliefs(subject="I", predicate="name", limit=1)
            if beliefs:
                b = beliefs[0]
                return (f"My name is {b['object']}", b['confidence'])
            beliefs = self.query_beliefs(subject="I", predicate="am", limit=1)
            if beliefs:
                b = beliefs[0]
                return (f"I am {b['object']}", b['confidence'])

        if re.search(r'who (built|made|created) you', question_lower):
            beliefs = self.query_beliefs(subject="I", limit=100)
            for b in beliefs:
                if 'built' in b['object'] or 'created' in b['object'] or 'by' in b['object']:
                    return (f"I am {b['object']}", b['confidence'])

        if re.search(r'what can you do|what are your capabilities', question_lower):
            beliefs = self.query_beliefs(subject="I", predicate="can", min_confidence=0.4)
            if beliefs:
                caps = [b['object'] for b in beliefs[:3]]
                conf = sum(b['confidence'] for b in beliefs[:3]) / len(beliefs[:3])
                return (f"I can {', and I can '.join(caps)}", conf)

        # Generic: look for keyword matches in beliefs
        words = set(re.sub(r'[\W_]+', ' ', question_lower).split())
        all_beliefs = self.query_beliefs(subject="I", limit=20)

        best_match = None
        best_score = 0

        for b in all_beliefs:
            belief_text = f"{b['subject']} {b['predicate']} {b['object']}".lower()
            belief_words = set(re.sub(r'[\W_]+', ' ', belief_text).split())
            overlap = len(words & belief_words)
            score = overlap * b['confidence']
            if score > best_score:
                best_score = score
                best_match = b

        if best_match and best_score > 0.5:
            return (f"{best_match['subject']} {best_match['predicate']} {best_match['object']}",
                    best_match['confidence'])

        return ("", 0.0)

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

    def _init_beliefs_schema(self) -> None:
        """Create beliefs table if it doesn't exist (Phase 19B)."""
        try:
            conn = sqlite3.connect(self.corpus_path)
            conn.execute("""
                CREATE TABLE IF NOT EXISTS beliefs (
                    id INTEGER PRIMARY KEY,
                    subject TEXT NOT NULL,
                    predicate TEXT NOT NULL,
                    object TEXT NOT NULL,
                    raw_source TEXT,
                    source_type TEXT DEFAULT 'training',
                    confidence REAL DEFAULT 0.5,
                    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    reinforcement_count INTEGER DEFAULT 1,
                    content_hash TEXT UNIQUE
                )
            """)
            conn.execute("CREATE INDEX IF NOT EXISTS idx_beliefs_subject ON beliefs(subject)")
            conn.execute("CREATE INDEX IF NOT EXISTS idx_beliefs_predicate ON beliefs(predicate)")
            conn.commit()
            conn.close()
        except Exception:
            pass  # Non-fatal — beliefs work without persistence

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

    def verify_audit_chain(self) -> dict:
        """Walk the BLAKE3 audit chain and verify every pre_hash link is intact.

        Returns a dict with:
          - chain_ok: True/False/None (None = no conscience engine loaded)
          - entries: number of logged intent entries
          - error: description of broken link, or None
        """
        if self.conscience_engine is None:
            return {"chain_ok": None, "entries": 0, "error": "No conscience engine loaded"}

        try:
            n = self.conscience_engine.audit_log_len()
            result = self.conscience_engine.verify_audit_chain()
            # verify_audit_chain() returns None on success (Ok(())) or raises on error
            return {"chain_ok": True, "entries": n, "error": None}
        except Exception as e:
            return {"chain_ok": False, "entries": 0, "error": str(e)}

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
