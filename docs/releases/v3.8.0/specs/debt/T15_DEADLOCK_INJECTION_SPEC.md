# T-15 SPEC: Deadlock Injection Test (Runtime)

> **Issue**: #2834
> **Version**: v3.8.0
> **Branch**: `fix/t-15-deadlock-injection`
> **Date**: 2026-06-03
> **Author**: Hermes Agent (claimed #2834)
> **Status**: COMPLETED

## 1. Background

v3.0.0 COMPLETE_LEGACY_TRACKING_REPORT.md defined T-15 as a test gap:
**Deadlock injection** scenarios. Existing TLA+ PROOF-026 covers Write Skew/SSI
formally but lacks concrete runtime tests.

## 2. Scope

This SPEC adds 8 runtime deadlock scenarios as supplementary tests to the
formal TLA+ PROOF-026 proof.

## 3. Design

### 3.1 API Usage

`sqlrustgo_transaction::deadlock::DeadlockDetector` provides:
- `new()` / `with_timeout(timeout)` - constructor
- `add_edge(blocked, holder)` - add wait-for edge
- `remove_edges_for(tx_id)` - cleanup after commit/rollback
- `detect_cycle(start)` - DFS-based cycle detection

### 3.2 Test Coverage Matrix

| Test | Scenario | Edge case? |
|------|----------|-----------|
| test_no_deadlock_independent_transactions | Linear wait (no cycle) | base |
| test_detect_direct_cycle_two_transactions | T1 ↔ T2 (2-cycle) | common |
| test_detect_indirect_cycle_three_transactions | T1 → T2 → T3 → T1 | 3-cycle |
| test_detect_self_loop | T1 → T1 (self-loop) | edge |
| test_remove_edges_after_commit | Cleanup after commit | lifecycle |
| test_detect_multiple_independent_cycles | Two separate cycles | composition |
| test_concurrent_random_wait_for_graphs | Stress: 50 trials × 20 txs | stress |
| test_timeout_configuration | Timeout config (5s, 100ms, 30s) | config |

## 4. Acceptance Criteria

- [x] 8 test cases (>= 5 required)
- [x] All tests pass (8/8)
- [x] Cargo clippy clean (0 warnings)
- [x] Cargo fmt clean
- [ ] `cross_version_debt.sh` shows T-15 CLOSED (after PR merge)
- [ ] PR merged to develop/v3.8.0

## 5. Test Plan

```bash
# Run new tests
cargo test -p sqlrustgo-transaction --test deadlock_injection_test

# Verify clippy
cargo clippy --all-features -p sqlrustgo-transaction -- -D warnings

# Run full transaction suite (no regression)
cargo test -p sqlrustgo-transaction

# Verify cross-version debt gate
bash scripts/gate/check_cross_version_debt.sh  # shows T-15 CLOSED
```

## 6. Implementation

File: `crates/transaction/tests/deadlock_injection_test.rs` (250 lines)

Uses:
- `sqlrustgo_transaction::deadlock::DeadlockDetector`
- `sqlrustgo_transaction::mvcc::TxId`

## 7. References

- v3.0.0 COMPLETE_LEGACY_TRACKING_REPORT.md (T-15)
- TLA+ PROOF-026 (Write Skew/SSI formal proof) — complementary
- MULTI_VERSION_GOVERNANCE_DAG.md
- INT5_PLUS_DEBT_INVENTORY.md
- Issue #2834
