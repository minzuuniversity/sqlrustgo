# v3.8.0+1 Post-GA Plan — Contract Gap Resolution

> **Issue**: #2776
> **Author**: Claude-MacMini
> **Date**: 2026-06-03
> **Status**: DRAFT
> **Based on**: ADR-006, ADR-007, ISSUE-2743

---

## 1. Executive Summary

v3.8.0 GA ships with **19/31 TX+WAL contract tests passing**. The 12 failures
are deferred to v3.8.0+1 per ADR-006 decision (2026-06-03).

| Category | Passed | Failed | Total |
|----------|--------|--------|-------|
| TX-Lifecycle | TBD | 4 | TBD |
| WAL Recovery | TBD | 8 | TBD |
| **Total** | **19** | **12** | **31** |

---

## 2. Contract Gap Classification

### 2.1 TX-Lifecycle Gaps (4 tests)

**Root Cause**: EEK v0 spec requires `DML without active tx → Err`, but current
implementation uses implicit autocommit.

| Gap ID | Description | Impact | Proposed Fix |
|--------|-------------|--------|--------------|
| TX-1 | DML without BEGIN → autocommit (current) vs error (spec) | Behavioral inconsistency | Add `strict_tx_mode` flag |
| TX-2 | COMMIT without BEGIN → no-op (current) vs error (spec) | Behavioral inconsistency | Add validation |
| TX-3 | ROLLBACK without BEGIN → no-op (current) vs error (spec) | Behavioral inconsistency | Add validation |
| TX-4 | SELECT in explicit transaction expected | Behavioral inconsistency | Document expected behavior |

**Fix Approach**:
- Implement `strict_tx_mode` configuration option
- When enabled: DML without active tx returns error
- When disabled (default): autocommit behavior preserved
- Migration path: deprecate autocommit in v3.9.0, remove in v3.10.0

### 2.2 WAL Recovery Gaps (8 tests)

**Root Cause**: Multi-tx ordering and partial-write semantics differ from spec assumptions.

| Gap ID | Description | Impact | Proposed Fix |
|--------|-------------|--------|--------------|
| WAL-1 | Multi-tx LSN ordering not guaranteed | Recovery may replay in wrong order | Implement TXID-ordered replay |
| WAL-2 | Partial-write idempotency not guaranteed | Duplicate records on replay | Add dedup logic based on PK |
| WAL-3 | Last-lsn checkpoint may not be latest | Data loss on crash | Implement proper checkpoint |
| WAL-4 | Transaction boundary blur between WAL entries | Recovery inconsistency | Add TX commit markers |
| WAL-5 | Buffer flush ordering not deterministic | Non-deterministic recovery | Implement flush ordering |
| WAL-6 | Delete replay skips updated rows | Data integrity issue | Fix delete predicate |
| WAL-7 | Insert buffer merge duplicates on replay | Data duplication | Fix buffer merge logic |
| WAL-8 | COMMIT record may be lost on crash | Transaction not durable | Implement commit guarantee |

**Fix Approach**:
- Based on ADR-007 (WAL Architecture Clarification) findings
- Implement TXID-ordered WAL replay
- Add deduplication based on primary key
- Implement proper transaction boundary markers

---

## 3. v3.8.0+1 Implementation Plan

### Phase 1: TX-Lifecycle Fix (2 weeks)

```
Week 1:
- [ ] Add `strict_tx_mode` configuration option
- [ ] Implement DML validation without active tx
- [ ] Add tests for TX-1 through TX-4

Week 2:
- [ ] Update documentation
- [ ] Run full contract test suite
- [ ] Create migration guide
```

### Phase 2: WAL Recovery Fix (4 weeks)

```
Week 1-2:
- [ ] Implement TXID-ordered WAL replay
- [ ] Add primary key deduplication
- [ ] Fix delete replay predicate

Week 3:
- [ ] Implement proper checkpoint
- [ ] Add transaction boundary markers
- [ ] Fix buffer flush ordering

Week 4:
- [ ] Integration testing
- [ ] Run full contract test suite
- [ ] Performance regression testing
```

---

## 4. Success Criteria

| Criteria | Target | Verification |
|----------|--------|--------------|
| TX-Lifecycle tests | 4/4 PASS | `cargo test tx_wal_contract -- TX-` |
| WAL Recovery tests | 8/8 PASS | `cargo test tx_wal_contract -- WAL-` |
| Overall contract tests | 31/31 PASS | `cargo test tx_wal_contract` |
| No existing test regression | 0 regressions | Full test suite |

---

## 5. Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| TX-lifecycle change breaks autocommit callers | Medium | High | Feature flag, gradual rollout |
| WAL replay change breaks existing tests | Medium | High | Comprehensive test coverage |
| Schedule overrun | High | Medium | Prioritize critical paths |

---

## 6. References

- [ADR-006: TX+WAL Contract Deferral](../governance/adr/ADR-006-tx-wal-contract-deferral.md)
- [ADR-007: WAL Architecture Clarification](../governance/adr/ADR-007-wal-architecture-clarification.md)
- [ISSUE-2743: Contract Test Gaps](../issues/ISSUE-2743_contract_test_gaps.md)
- [ISSUE-2742: WAL Architecture Clarification](../issues/ISSUE-2742_wal_architecture.md)
