# v3.8.0 Roadmap

## Status: DRAFT

> **Baseline**: `origin/develop/v3.8.0` (commit 44fea01c) — Execution Semantics Freeze (087bb12d)
> Full Plan: see `DEVELOPMENT_PLAN.md`

---

## Vision

v3.8.0 transforms SQLRustGo from a **query execution engine** into a **server-level transactional database** with explicit transaction lifecycle management.

> "Transaction ownership resides outside LocalExecutor. LocalExecutor remains stateless."

---

## Milestones

### M1: Phase 1 Complete (TransactionManager Integration)
**Target**: 2026-06-07

- TransactionManager connected to dispatch layer
- BEGIN/COMMIT/ROLLBACK routed to txn_manager
- LocalExecutor unchanged (no txn_manager field)
- G1 gate: 250 tests PASS

### M2: Phase 2+3 Complete (WriteBuffer + Commit Engine)
**Target**: 2026-06-14

- DML stages to write_buffer
- COMMIT flushes to StorageEngine
- No direct DML → StorageEngine path
- G2+G3 gate: DML buffering + commit flush verified

### M3: Phase 4+5 Complete (Rollback + Read Consistency)
**Target**: 2026-06-21

- ROLLBACK discards write_buffer
- Transaction snapshot for read-your-writes
- v3.9 MVCC foundation laid
- G4+G5 gate: rollback + snapshot isolation

### M4: v3.8.0 GA
**Target**: 2026-06-28

- Full transaction lifecycle (BEGIN → DML → COMMIT/ROLLBACK)
- 400+ tests PASS
- TPC-H SF=1 no regression
- GA gate documentation

---

## v3.8.0 Scope

### In Scope

✅ BEGIN / COMMIT / ROLLBACK lifecycle
✅ WriteBuffer for DML staging
✅ Commit Engine (buffer → storage)
✅ Rollback Engine (discard buffer)
✅ Transaction snapshot (read-your-writes)
✅ SSI conflict detection (existing, preserved)
✅ LocalExecutor remains stateless
✅ Gate reports: Alpha / Beta / RC / GA

### Out of Scope

❌ MVCC full implementation (→ v3.9)
❌ Two-phase commit (→ v3.9+)
❌ Savepoints
❌ XA protocol
❌ Distributed transactions
❌ StorageEngine transaction awareness

---

## v3.8 vs v3.7 Comparison

| Aspect | v3.7.0 | v3.8.0 |
|--------|--------|--------|
| Transaction ownership | Implicit (hard to trace) | Explicit (TransactionManager) |
| Write staging | Direct to StorageEngine | Buffered in TransactionManager |
| BEGIN handling | N/A (not connected) | txn_manager.begin() |
| COMMIT handling | N/A (not connected) | txn_manager.commit() → flush |
| ROLLBACK handling | N/A (not connected) | txn_manager.rollback() → discard |
| LocalExecutor | Stateless | Stateless (unchanged) |
| Read consistency | StorageEngine level | TransactionManager snapshot |
| SSI conflict detection | Yes | Yes (preserved) |

---

## Verification Checklist

Before v3.8.0 GA, all of these must remain true:

- [ ] `LocalExecutor` struct has **no** `txn_manager` field
- [ ] `LocalExecutor` struct has **no** `write_buffer` field
- [ ] `TransactionManager` is instantiated at **server/session** level, not inside executor
- [ ] All `BEGIN`/`COMMIT`/`ROLLBACK` go through `TransactionManager`
- [ ] DML operations stage through `write_buffer`, not direct to `StorageEngine`
- [ ] `COMMIT` flushes `write_buffer` → `StorageEngine`
- [ ] `ROLLBACK` discards `write_buffer` (no storage side effects)
- [ ] `read-your-writes` semantics enforced via snapshot
- [ ] 400+ tests pass (no regression from v3.7.0)
- [ ] TPC-H SF=1 completes with no degradation

---

## Dependencies

```
crates/transaction/src/manager.rs  (existing, stable)
    │
    ├── Phase 1: Connect to dispatch layer
    ├── Phase 2: Add WriteBuffer struct
    ├── Phase 3: Add flush_to_storage()
    ├── Phase 4: Add rollback() discard
    └── Phase 5: Add SnapshotMeta

crates/executor/src/              (no changes in v3.8)
    └── LocalExecutor remains stateless

crates/mysql-server/src/          (dispatch layer changes)
    ├── Session → TransactionManager
    └── BEGIN/COMMIT/ROLLBACK dispatch
```

---

## Risk Summary

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Dispatch layer changes break existing dispatch path | Medium | High | Isolated changes at line ~1081; extensive integration tests |
| WriteBuffer increases memory usage | Low | Medium | Configurable max_size; OOM protection |
| Concurrent commit causes SSI performance regression | Low | Low | Existing SSI implementation unchanged |
| Phase 5 snapshot breaks existing reads | Medium | High | Unit tests + integration tests before merging |

---

## Changelog

| Date | Change |
|------|--------|
| 2026-05-30 | Initial draft — server-level transaction model roadmap |