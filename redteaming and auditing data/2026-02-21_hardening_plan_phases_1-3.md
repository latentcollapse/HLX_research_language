# HLX Hardening Plan

> Created: February 21, 2026
> Status: PHASE 1 & 2 COMPLETE
> Based on: HLX_audit.md
> Last Updated: February 21, 2026

---

## Overview

This document tracks the systematic hardening of HLX based on security audit findings. Each issue has a concrete implementation plan, test requirements, and status.

---

## Phase 1: Critical Fixes ✅ COMPLETE

### 1.1 RSI Voting Sybil Attack
**Priority**: CRITICAL
**Status**: ✅ COMPLETE
**Location**: `hlx-runtime/src/rsi.rs`

**Changes Implemented**:
- [x] Added `HashSet<u64>` to track unique voter IDs per proposal
- [x] Reject duplicate votes from same agent_id
- [x] Added `has_voted()`, `voter_count()` helper methods
- [x] Updated VM to pass agent_id with each vote

**Tests**: 4 new tests

---

### 1.2 Rollback Mechanism Broken
**Priority**: CRITICAL
**Status**: ✅ COMPLETE
**Location**: `hlx-runtime/src/rsi.rs`

**Changes Implemented**:
- [x] Full state serialization with bincode
- [x] `AgentMemorySnapshot` struct with all fields
- [x] Proper rollback restores behaviors, parameters, weight_matrices, cycle_config
- [x] BLAKE3 hash computation for state integrity

**Tests**: 5 new tests

---

### 1.3 Bytecode Integrity Verification
**Priority**: CRITICAL
**Status**: ✅ COMPLETE
**Location**: `hlx-runtime/src/bytecode.rs`

**Changes Implemented**:
- [x] Added `serialize()` and `deserialize()` methods
- [x] 50-byte header: magic (`LC-B`), version, sizes, BLAKE3 hash
- [x] Integrity verification on deserialize
- [x] Rejects invalid magic, truncated data, tampered bytecode

**Tests**: 5 new tests

---

## Phase 2: High Priority Fixes ✅ COMPLETE

### 2.1 Tensor Size Limits
**Priority**: HIGH
**Status**: ✅ COMPLETE
**Location**: `hlx-runtime/src/tensor.rs`

**Changes Implemented**:
- [x] `TensorLimits` struct with max_elements, max_rank, max_dimension
- [x] Global allocation tracking with atomic counters
- [x] `new_with_limits()`, `zeros_with_limits()` constructors
- [x] Default limit: 10^9 elements, max rank 8

**Tests**: 5 new tests

---

### 2.2 Consensus Minimum Quorum
**Priority**: HIGH
**Status**: ✅ COMPLETE
**Location**: `hlx-runtime/src/rsi.rs`

**Changes Implemented**:
- [x] `min_quorum(total_agents)` = max(3, ceil(total_agents * 0.2))
- [x] `is_approved()` now requires quorum proportional to agent pool
- [x] Small pools (≤14 agents) still require minimum 3 votes
- [x] Large pools require proportional participation

**Tests**: 4 new tests

---

### 2.3 Agent Spawn Rate Limiting
**Priority**: HIGH
**Status**: ✅ COMPLETE
**Location**: `hlx-runtime/src/vm.rs`

**Changes Implemented**:
- [x] `SpawnRateLimit` struct with configurable max_spawns and window
- [x] Default: 10 spawns per 60 seconds
- [x] Maximum total agent count (default: 1000)
- [x] Builder pattern: `with_spawn_rate_limit()`, `with_max_agents()`

**Tests**: 2 new tests

---

## Progress Summary

| Phase | Total | Completed | In Progress | Blocked |
|-------|-------|-----------|-------------|---------|
| 1 - Critical | 3 | 3 | 0 | 0 |
| 2 - High | 3 | 3 | 0 | 0 |
| 3 - Medium | 3 | 0 | 0 | 0 |
| 4 - Long-term | 3 | 0 | 0 | 0 |

**Total Tests**: 55 passing

---

## Phase 3: Medium Priority Fixes (NEXT)

### 3.1 Governance Config Immutability
**Priority**: MEDIUM
**Status**: NOT STARTED
**Location**: `hlx-runtime/src/governance.rs:143-144`

**Problem**: `strict_mode` and limits can be modified at runtime.

**Required Changes**:
- [ ] Make config read-only after initialization
- [ ] Require consensus to modify governance config
- [ ] Log any config changes

---

### 3.2 Vulkan Shader Attestation
**Priority**: MEDIUM
**Status**: NOT STARTED
**Location**: `backends/vulkan/mod.rs:46-71`

**Problem**: 23 shaders loaded without integrity check.

**Required Changes**:
- [ ] Embed SHA-256 hashes in binary
- [ ] Verify each shader on load
- [ ] Reject modified shaders

---

### 3.3 Barrier Timeout
**Priority**: MEDIUM
**Status**: NOT STARTED
**Location**: `hlx-runtime/src/scale.rs:29-41`

**Problem**: No timeout on barrier wait. Agents can hang forever.

**Required Changes**:
- [ ] Add timeout parameter to barrier creation
- [ ] Return error on timeout
- [ ] Allow barrier cancellation

---

## Phase 4: Long-Term Hardening

### 4.1 Formal Verification
**Priority**: ONGOING
**Status**: NOT STARTED

- [ ] Verify bounded ranking function with model checker
- [ ] Prove termination guarantees
- [ ] Verify predicate logic

### 4.2 Differential Testing Infrastructure
**Priority**: ONGOING
**Status**: NOT STARTED

- [ ] Build before/after comparison framework
- [ ] Automated regression testing
- [ ] Performance variance detection

### 4.3 Audit Logging
**Priority**: ONGOING
**Status**: NOT STARTED

- [ ] Log all RSI proposals
- [ ] Log all governance decisions
- [ ] Cryptographic log integrity

---

## Sign-off Requirements

Before public release:
- [x] All Phase 1 items complete with tests passing
- [x] All Phase 2 items complete with tests passing
- [ ] Phase 3 items complete
- [ ] Red team attack suite executed without critical findings
- [ ] 48-hour continuous operation test passed
- [ ] Audit log review by second reviewer

---

*Last Updated: February 21, 2026*
