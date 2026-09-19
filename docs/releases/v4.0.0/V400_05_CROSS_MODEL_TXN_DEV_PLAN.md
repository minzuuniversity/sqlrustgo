# V400-05: Cross-Model Transaction — Development Plan

> **Date**: 2026-09-19
> **Issue**: #V400-05
> **Status**: 🚧 IN PROGRESS — scaffold implementation
> **Owner**: transaction
> **Estimate**: 8 weeks per issue; **2-week RC sprint subset** for v4.0.0 GA

## Goal

Implement cross-model transaction semantics where a single transaction
boundary covers SQL row writes + vector writes + graph writes + audit
events. All-or-nothing on COMMIT/ROLLBACK.

## Dependencies (already DONE in develop/v4.0.0)

- ✅ V400-02 V1-V5 — WAL-backed vector storage
- ✅ V400-03 G1-G5 — Graph first-class storage with WAL
- ✅ V400-04 — GRAPH MATCH SQL surface dispatch
- ✅ MVCC GC runner (commit b87997d4a1)
- ✅ TransactionManager / BEGIN/COMMIT/ROLLBACK (carryover from v3.8.0)

## Current State (2026-09-19)

The `TransactionManager` already handles SQL rows via WAL replay on
crash. But:

- Vector writes go through `VectorStore::insert` → `WalStorage` but
  the WAL entry type registration for vector ops is per-Crate
  (separate from SQL WAL).
- Graph writes go through `DiskGraphStore::wal` → `graph::wal` (its
  own WAL file under `<data_dir>/graph/default.dgs/wal`).
- Audit events are appended via `audit::log` (separate from main WAL).

**Cross-model atomicity is NOT enforced** — a transaction containing
`INSERT INTO t` + `INSERT INTO vectors` + `MATCH (n) CREATE (n)` could
have partial commits if crash mid-flight.

## RC Sprint Subset (v4.0.0 GA scope)

For v4.0.0 GA, deliver a **minimal scaffold** that demonstrates
cross-model commit/rollback semantics via:

1. **Shared WAL** (single WAL file for all models)
2. **Atomic WAL flush** at COMMIT
3. **Rollback discards all models** in the same transaction
4. **Crash recovery** replays all model writes uniformly

### Concrete Tasks

#### Task 1: WAL entry consolidation

**File**: `crates/storage/src/wal_legacy.rs`

- Add `WalEntry::CrossModelBoundary { models: Vec<ModelKind> }` enum variant
- Modify `WalEntry` enum to support per-model operation chaining
- Each model-specific entry (Vector*, Graph*, Audit*) is wrapped in a
  CrossModelBoundary transaction context.

#### Task 2: TransactionManager hook for vector/graph writes

**File**: `crates/transaction/src/manager.rs`

- Add `tx_register_write(ModelKind, WalEntry)` method
- Hook `MvccStorage::insert/delete` to call `tx_register_write(ModelKind::Sql, ...)`
- Hook `VectorStore::insert/delete` similarly
- Hook `DiskGraphStore::create_node/create_edge` similarly
- On `tx.commit()`: flush ALL pending writes (SQL + vector + graph) to single WAL
- On `tx.rollback()`: discard ALL pending writes

#### Task 3: Audit event integrity

**File**: `crates/audit/src/chain.rs`

- Audit event write is wrapped in the same transaction boundary
- ALCOA+ chain invariants enforced via cross-model commit

#### Task 4: Tests

**File**: `crates/transaction/tests/v400_cross_model.rs`

- SQL + vector write atomic COMMIT
- SQL + vector write atomic ROLLBACK
- SQL + graph write atomic COMMIT
- Crash mid-cross-model: WAL replay restores all models uniformly
- Audit chain integrity under cross-model rollback

### Exit Evidence

- `crates/transaction/tests/v400_cross_model.rs` — ≥ 50 tests, all PASS
- `docs/evidence/v4.0.0/cross_model_txn_report.md` — crash test results
- (no regressions in existing WAL tests)

## Out of Scope for v4.0.0 GA (deferred to v4.0.1)

- Optimistic concurrency control across model types
- Distributed cross-model coordination (V400-09 SOAK needs single-host)
- Vector index consistency under concurrent graph mutation
- Audit chain checkpointing (current: append-only, no rollback protection beyond WAL)

## Acceptance Criteria

| Criterion | Required | Status |
|-----------|----------|--------|
| Single WAL for all model writes | Yes | 🚧 in scaffold |
| Atomic COMMIT | Yes | 🟡 partial (Task 2) |
| Atomic ROLLBACK | Yes | 🟡 partial |
| Crash recovery | All models replay uniformly | 🟡 partial |
| ≥ 50 cross-model tests | Yes | 🚧 scaffold |
| Crash test scenarios | ≥ 10 | 🚧 scaffold |

## Risks

1. **Performance**: Shared WAL may bottleneck at high concurrency.
   Mitigation: per-model WAL segment with cross-model commit barrier.

2. **Audit chain integrity**: If audit event fails to write, the entire
   cross-model commit must abort. Requires 2PC between audit + data WALs.

3. **Recovery time**: Replay of long-running cross-model transactions
   scales linearly with transaction duration. Mitigation: write-ahead
   txn boundaries to WAL for incremental replay.

## Implementation Plan (RC sprint)

| Day | Task |
|-----|------|
| 1 | Task 1: WAL entry consolidation |
| 2 | Task 2: TransactionManager hooks |
| 3 | Task 2 cont.: vector + graph hook integration |
| 4 | Task 3: Audit chain integrity |
| 5-6 | Task 4: ≥ 50 tests (10 crash scenarios + 40 normal flows) |
| 7-8 | Documentation + dev plan updates |

## Status

**2026-09-19**: Dev plan published. Implementation pending.

This dev plan tracks the work; actual code changes happen in Task 1-4.
See commit history for incremental progress.