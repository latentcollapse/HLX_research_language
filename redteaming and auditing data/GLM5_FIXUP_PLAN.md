# GLM5 Fixup Plan — Post-Audit

**Context**: Opus audited all MVP code. The Rust infrastructure is solid. The gates are now wired into the RSI pipeline (Opus did this). What remains is bug fixes, hash alignment, and tests.

**Current state**: 268 tests passing. Homeostasis and Promotion gates are live in `RSIPipeline.create_proposal()` and `apply_proposal()`.

---

## Task 1: Fix `bit.py` Bugs (CRITICAL — 3 bugs)

File: `axiom-hlx-stdlib/axiom_py/python/axiom/bit.py`

### Bug 1: Double-counting successful modifications

`propose()` increments `self.successful_modifications` when a proposal is allowed (around line 270). Then `on_modification_applied()` increments it again (around line 298). If both are called for the same modification, the count is doubled and Bit promotes faster than intended.

**Fix**: Remove the increment from `propose()`. Only count in `on_modification_applied()` — that's when a modification actually lands.

### Bug 2: Conscience check fails open

If `self.conscience_engine.verify()` throws an exception, the `_check_conscience()` method returns `None`. In `propose()`, the guard (around line 258) only rejects when `verdict is not None and not verdict.allowed`. So an exception = `None` = allowed through.

**Fix**: Change the guard to fail-closed:
```python
# Before (fails open):
if verdict is not None and not verdict.allowed:
    ...

# After (fails closed):
if verdict is None or not verdict.allowed:
    reason = verdict.reason if verdict else "conscience check failed"
    ...
```

### Bug 3: `rule_update` risk always exceeds threshold

In `_assess_risk()`, `rule_update` has hardcoded risk `0.8`. In `propose()`, the threshold is `0.7`. So `RuleUpdate` proposals are ALWAYS rejected even at Mature level.

**Fix**: Lower `rule_update` risk to `0.65` (below threshold but still high), OR raise the threshold for rule modifications. The Rust side uses `0.8` for RuleUpdate risk assessment, but the Rust pipeline's `max_risk` is `0.7` too — so the same issue exists there. For now, set `rule_update` risk to `0.65` in Python. We'll address the Rust side as a follow-up.

---

## Task 2: Fix `seed_bit.py` Hash Mismatch

File: `scripts/seed_bit.py`

The seeding script uses `hashlib.sha256` everywhere. The Rust integrity system (`integrity.rs`) uses BLAKE3. This means the Rust side cannot verify hashes created by the seeding script.

**Fix**: Replace `hashlib.sha256` with `blake3`. The `blake3` Python package is a direct binding to the same BLAKE3 library Rust uses.

```bash
pip install blake3
```

Then in `seed_bit.py`, replace:
```python
import hashlib
# ...
hashlib.sha256(content.encode()).hexdigest()
```

With:
```python
import blake3
# ...
blake3.blake3(content.encode()).hexdigest()
```

Apply this to ALL hash computations in the file: `_hash_content()`, checkpoint hashing, and the corpus-wide hash.

---

## Task 3: Add `bit.py` Tests

File: Create `axiom-hlx-stdlib/axiom_py/python/axiom/test_bit.py`

Required tests (minimum):

```python
def test_seedling_allows_parameter_update():
    """Seedling level should allow ParameterUpdate proposals."""

def test_seedling_blocks_behavior_add():
    """Seedling level should NOT allow BehaviorAdd proposals."""

def test_successful_modification_increments_once():
    """propose() + on_modification_applied() should only increment count once."""

def test_conscience_exception_fails_closed():
    """If conscience engine throws, proposal should be rejected."""

def test_risk_threshold_rule_update():
    """RuleUpdate at Mature level should pass risk check after fix."""

def test_promotion_seedling_to_sprout():
    """After enough successful modifications + homeostasis, level should advance."""

def test_ask_returns_string():
    """ask() should return a non-empty string response."""

def test_observe_stores_observation():
    """observe() should add to observation list."""

def test_status_returns_dict():
    """status() should return a dict with expected keys."""

# Adversarial:
def test_rapid_proposals_dont_crash():
    """Submitting 100 proposals rapidly should not crash or corrupt state."""
```

Use `unittest` or `pytest`. Mock the conscience engine for the exception test.

---

## Task 4: Add `seed_bit.py` Tests

File: Create `scripts/test_seed_bit.py`

Required tests:

```python
def test_create_corpus_schema():
    """Corpus DB should have documents, rules, memory, checkpoints tables."""

def test_seed_identity():
    """After seeding, identity document should exist with correct hash."""

def test_seed_rules():
    """After seeding, all 10 initial rules should exist with correct confidence scores."""

def test_checkpoint_exists():
    """After seeding, initial_seed checkpoint should be present."""

def test_corpus_hash_is_blake3():
    """Checkpoint hash should match BLAKE3 recomputation."""

def test_idempotent_or_error():
    """Running seed twice should either be idempotent or raise a clear error."""
```

Use a temporary directory for the corpus DB so tests don't pollute the real one.

---

## Task 5: Fix `integrity.rs` — Layer 2 Skip (LOW PRIORITY)

File: `hlx-runtime/src/integrity.rs`

`full_verification()` sets `layer2_passed: true` without actually calling `verify_conscience()`. This means "full" verification skips conscience checks.

**Fix**: Call `self.verify_conscience()` inside `full_verification()` and use the result for `layer2_passed`.

Also: `compute_memory_hash()` truncates content at 256 bytes (line ~96). Either remove the truncation or increase to a reasonable limit (4096+ bytes). Two memories differing only after byte 256 would produce identical hashes — that's a correctness bug.

---

## Task 6: Axiom Integration in RSI Pipeline (DEFERRED)

This is the `#[cfg(feature = "axiom")]` bridge between the Rust RSI pipeline and the Axiom policy engine. It requires:
- Adding axiom-lang as a dependency (already in Cargo.toml as optional)
- Creating an `AxiomEngine` in `RSIPipeline` when the feature is enabled
- Calling `engine.verify()` before the homeostasis gate in `create_proposal()`
- Mapping `ModificationType` variants to Axiom intent names

**This is complex and should be a separate focused session.** The pipeline works correctly without it — Axiom verification happens at the Python layer via bit.py for now. Skip this task for now.

---

## Verification

After completing Tasks 1-5, run:

```bash
# Rust tests (should still be 268 passing)
cd ~/HLX/hlx-runtime && cargo test --all

# Python tests
cd ~/HLX/axiom-hlx-stdlib/axiom_py/python/axiom && python -m pytest test_bit.py -v
cd ~/HLX/scripts && python -m pytest test_seed_bit.py -v
```

---

## What Opus Already Did

- Fixed non-exhaustive match in `rsi.rs` (added `RuleAdd`/`RuleRemove`/`RuleUpdate` to `AgentMemory::apply_modification()` with proper `rules` field)
- Added `Lowerer` import to `e2e_test.rs`
- **Wired `HomeostasisGate` and `PromotionGate` into `RSIPipeline`**:
  - `create_proposal()` now checks homeostasis gate (Block/Homeostasis/SlowDown/Proceed) then promotion gate (`is_modification_allowed`) before confidence and risk checks
  - `apply_proposal()` now calls `homeostasis_gate.record_modification()` and `promotion_gate.on_successful_modification()` after successful application
  - Homeostasis achieving triggers `promotion_gate.on_homeostasis()` for level advancement
  - Added `homeostasis()` and `promotion()` accessor methods on `RSIPipeline`
  - Updated `test_proposal_voting` to use Seedling-allowed `ParameterUpdate`

The full pipeline flow is now:
```
Homeostasis → Promotion → Confidence → Risk → Governance → Voting → Apply
                                                                      ↓
                                               record_modification() + on_successful_modification()
```

268 tests passing.

---

*Written by Opus, 2026-02-25*
