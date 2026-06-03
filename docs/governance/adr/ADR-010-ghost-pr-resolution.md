# ADR-010: Ghost PR Resolution — F-07~F-15 Formal Deferral

## Status

**Accepted** — v3.8.0 GA (2026-06-03)

## Context

During v3.8.0 development, 9 PRs (F-07~F-15) were planned but never implemented.
These are "ghost PRs" — they exist in planning documents but have no code.

This ADR provides formal deferral decisions for each ghost PR, per
Truthfulness Principle (ADR-001): we do not pretend these PRs were implemented.

## Decisions

### F-07: PR-810 ExecutionEngine → Router

**Status**: DEFERRED to v3.9.0

**Rationale**: Router pattern is valuable but not critical for v3.8.0 transaction
lifecycle goals. ExecutionEngine refactoring risk is high.

**Restart Entry**:
```bash
git checkout develop/v3.9.0
git checkout -b feature/pr-810-router
```

---

### F-08: PR-820 TransactionManager Session Binding

**Status**: DEFERRED to v3.9.0

**Rationale**: Session binding enables per-connection transaction state, but
v3.8.0 achieves transaction lifecycle with session-level TM at server level.

**Restart Entry**:
```bash
git checkout develop/v3.9.0
git checkout -b feature/pr-820-session-binding
```

---

### F-09: PR-840 WriteBuffer Integration

**Status**: DEFERRED to v3.9.0

**Rationale**: WriteBuffer is critical for v3.9.0 MVCC implementation.
v3.8.0 achieves basic transaction lifecycle without it.

**Restart Entry**:
```bash
git checkout develop/v3.9.0
git checkout -b feature/pr-840-write-buffer
```

---

### F-10: PR-850 DML Through WriteBuffer

**Status**: DEFERRED to v3.9.0

**Rationale**: Depends on F-09 (WriteBuffer). Same rationale applies.

**Restart Entry**:
```bash
git checkout develop/v3.9.0
git checkout -b feature/pr-850-dml-write-buffer
```

---

### F-11: PR-860 COMMIT Flushes WriteBuffer

**Status**: DEFERRED to v3.9.0

**Rationale**: Depends on F-09/F-10. Commit flush is v3.9.0 milestone.

**Restart Entry**:
```bash
git checkout develop/v3.9.0
git checkout -b feature/pr-860-commit-flush
```

---

### F-12: PR-870 ROLLBACK Discards WriteBuffer

**Status**: DEFERRED to v3.9.0

**Rationale**: Depends on F-09/F-10. Rollback discard is v3.9.0 milestone.

**Restart Entry**:
```bash
git checkout develop/v3.9.0
git checkout -b feature/pr-870-rollback-discard
```

---

### F-13: PR-880 WAL Recovery Integration

**Status**: DEFERRED to v3.9.0

**Rationale**: v3.8.0 has basic WAL replay (PR-830C/D/E). Full recovery
integration with WriteBuffer is v3.9.0 goal.

**Restart Entry**:
```bash
git checkout develop/v3.9.0
git checkout -b feature/pr-880-recovery-integration
```

---

### F-14: PR-890 WAL TPC-H Validation

**Status**: CANCELLED

**Rationale**: TPC-H validation is already covered by existing gate tests
(`cargo test --test tpch_gate_test`). No need for separate WAL TPC-H PR.

---

### F-15: PR-900 WAL Performance Baseline

**Status**: DEFERRED to v3.9.0

**Rationale**: Performance baseline should be measured after F-09~F-13
WriteBuffer implementation is complete.

**Restart Entry**:
```bash
git checkout develop/v3.9.0
git checkout -b feature/pr-900-perf-baseline
```

---

## Summary Table

| PR | Feature | Decision | Target Version |
|----|---------|----------|----------------|
| F-07 | PR-810 Router | DEFERRED | v3.9.0 |
| F-08 | PR-820 Session Binding | DEFERRED | v3.9.0 |
| F-09 | PR-840 WriteBuffer | DEFERRED | v3.9.0 |
| F-10 | PR-850 DML WriteBuffer | DEFERRED | v3.9.0 |
| F-11 | PR-860 COMMIT Flush | DEFERRED | v3.9.0 |
| F-12 | PR-870 ROLLBACK Discard | DEFERRED | v3.9.0 |
| F-13 | PR-880 Recovery Integration | DEFERRED | v3.9.0 |
| F-14 | PR-890 TPC-H Validation | CANCELLED | N/A |
| F-15 | PR-900 Performance Baseline | DEFERRED | v3.9.0 |

## Consequences

### Positive

- Clear roadmap for v3.9.0 feature development
- No "ghost PRs" remaining — all have formal decisions
- DEFERRED_PRS.md restart entries provide immediate continuation points

### Negative

- v3.8.0 does not include WriteBuffer-based DML staging
- Full MVCC deferred to v3.9.0

## References

- DEFERRED_PRS.md
- FEATURE_CHECKLIST.md
- ADR-001: Truthfulness Framework
- ADR-006: TX+WAL Contract Deferral
- ADR-007: WAL Architecture Clarification

## Revision History

- 2026-06-03: Accepted (Claude-MacMini)
