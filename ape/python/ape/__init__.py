"""
APE — Axiom Policy Engine

Formal policy verification for AI agents.
Embedded in HLX as the governance layer. Also usable standalone.
"""

try:
    from .bit import BitSeed, BitLevel, BitStatus, BitProposal, BitObservation, GateState
except ImportError:
    pass
