# SPEC-015: v3.6.0 P2-1/P2-2 Deferred Features (FULL OUTER JOIN + Distributed Execution)

> **Version**: v3.8.0 (audit + scope clarification)
> **Branch**: `develop/v3.8.0`
> **Date**: 2026-06-03
> **Author**: Hermes Agent
> **Status**: SPEC document (no implementation)

## 1. Background

v3.6.0 DEVELOPMENT_PLAN.md defines 2 P2 (priority-2) features:

| ID | Feature | Issue | Original Status |
|----|---------|-------|-----------------|
| P2-1 | FULL OUTER JOIN/MERGE executor | — | "延迟至 v3.7.0" |
| P2-2 | 分布式执行路径 | — | "延迟至 v3.7.0" |

v3.7.0 DEVELOPMENT_PLAN.md 把它们标为"未来架构方向"。v3.8.0 FEATURE_CHECKLIST
把它们归到 F-07~F-15 NOT_DONE。

This SPEC document captures the current state and planned future implementation
for these 2 long-deferred features.

---

## 2. P2-1: FULL OUTER JOIN / MERGE Executor

### 2.1 Feature Definition (from v3.6.0)

> "FULL OUTER JOIN/MERGE executor"
> "延迟至 v3.7.0"

### 2.2 Current State (v3.8.0)

**Implementation**: ❌ Not implemented

**Existing JOIN support** in v3.8.0:
- `INNER JOIN` ✅
- `LEFT JOIN` ✅
- `RIGHT JOIN` ✅
- `CROSS JOIN` ✅
- `FULL OUTER JOIN` ❌ (this is what's missing)
- `MERGE` (SQL standard) ❌

**Code location**: `crates/executor/src/local_executor.rs` (JOIN impl)

### 2.3 Implementation Plan (v3.8.0+1 or later)

```rust
// Pseudo-code for FULL OUTER JOIN
fn execute_full_outer_join(
    left: &TableScan,
    right: &TableScan,
    condition: &Expression,
) -> SqlResult<Vec<Record>> {
    // 1. Hash-build the right side
    // 2. For each left row, probe right side
    //    - Match → add merged row
    //    - No match → add left row + NULL for right cols
    // 3. For each right row that didn't match any left row
    //    - Add NULL + right row
    // 4. Return combined results
}
```

**Effort estimate**: 200-400 LOC + 50+ tests
**Target**: v3.8.0+1 or v3.9.0 (per POST_GA_PLAN.md)

### 2.4 Test Plan (when implemented)

- `test_full_outer_join_basic` (3 tables, 2 matched, 1 left-only, 1 right-only)
- `test_full_outer_join_no_match` (left and right disjoint)
- `test_full_outer_join_with_complex_condition` (multi-col join)
- `test_full_outer_join_with_aggregation` (GROUP BY after FULL OUTER JOIN)
- `test_full_outer_join_performance` (large tables, hash join time)

### 2.5 Gate Integration

When implemented, add to:
- `check_integration_gate.sh` (test runner)
- `check_performance.sh` (perf comparison vs LEFT+UNION fallback)
- `check_arch_invariants.sh` (validate SQL → AST → Plan → Execution path consistency)

---

## 3. P2-2: Distributed Execution Path

### 3.1 Feature Definition (from v3.6.0)

> "分布式执行路径"
> "延迟至 v3.7.0"

### 3.2 Current State (v3.8.0)

**Implementation**: ❌ Not implemented

**v3.8.0 is single-node** (per v3.8.0 ARCHITECTURE.md)
- All execution in one process
- WAL is local (no replication)
- ShardedGraphStore is single-machine

**Distributed enablers in v3.8.0**:
- WAL: local only (not replicated)
- Network: MySQL wire protocol server only
- Storage: file-based, no shared FS

### 3.3 Implementation Plan (per POST_GA_PLAN.md)

**Phase 1 (v3.8.0+1)**: WAL replication
- `crates/wal-replication/` (scaffolded)
- Primary → Replica streaming
- Read-after-write consistency on replica

**Phase 2 (v3.9.0)**: Distributed query execution
- Plan fragment partitioning
- Network shuffle between fragments
- Coordinator + worker pool

**Phase 3 (v4.0.0)**: Sharded execution
- Hash-partitioned tables
- Cross-shard JOIN
- Parallel aggregation

**Target**: v3.8.0+1 (WAL replication) → v3.9.0 (distributed) → v4.0.0 (sharded)

### 3.4 Test Plan (when implemented)

**Phase 1 (WAL replication)**:
- `test_wal_replication_basic` (primary write, replica read)
- `test_wal_replication_lag` (replica catches up after delay)
- `test_wal_replication_failover` (primary down, replica promoted)

**Phase 2 (Distributed execution)**:
- `test_distributed_select_aggregation` (SUM across 3 nodes)
- `test_distributed_join` (replicated table + sharded table JOIN)
- `test_distributed_fault_tolerance` (1 node down, query continues)

### 3.5 Gate Integration

When implemented, add to:
- `check_integration_gate.sh` (distributed test runner)
- `check_architecture_freeze.sh` (verify execution path consistency)
- `check_attack_surface.sh` (network security)
- `check_performance.sh` (distributed vs single-node comparison)

---

## 4. Why Deferred

Both P2-1 and P2-2 are **architectural changes** with high implementation cost:

| Feature | Implementation cost | Risk | ROI |
|---------|--------------------|----- |-----|
| P2-1 FULL OUTER JOIN | 200-400 LOC + 50 tests | Low (algorithmic, well-known) | Medium (rare use case) |
| P2-2 Distributed Execution | 2000+ LOC + infrastructure | High (network, consistency, fault tolerance) | High for large data |

v3.8.0 prioritized **architectural unification** (single execution path, WAL
integration) over these features per v3.8.0 DEVELOPMENT_PLAN.md §1.1:

> "v3.8.0 不是 feature release, 而是 Execution Architecture Consolidation Release"

---

## 5. References

- v3.6.0 DEVELOPMENT_PLAN.md (P2-1, P2-2 originals)
- v3.7.0 DEVELOPMENT_PLAN.md (P2-1, P2-2 future)
- v3.8.0 DEVELOPMENT_PLAN.md (architecture unification)
- v3.8.0 FEATURE_CHECKLIST.md (F-07~F-15 NOT_DONE)
- v3.8.0 POST_GA_PLAN.md (v3.8.0+1 WAL replication)
- docs/contracts/UPDATE_REPLAY_CONTRACT.md (related contract)
