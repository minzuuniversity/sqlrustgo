# T-17 + T-18 SPEC: Network + Memory Fault Injection

> **Issues**: #2835 (T-17), #2836 (T-18)
> **Version**: v3.8.0
> **Branch**: `fix/t-17-t-18-fault-injection`
> **Date**: 2026-06-03
> **Author**: Hermes Agent (via openspec)
> **Status**: COMPLETED (PR pending)
> **Openspec change**: `openspec/changes/t-17-t-18-fault-injection/`

## 1. Background

v3.0.0 COMPLETE_LEGACY_TRACKING_REPORT.md identified T-17 (Network 30%
packet loss) and T-18 (Memory fault injection) as test gaps. These tests
verify the database's resilience to real-world failure conditions.

T-15 (deadlock injection) was closed in PR-2839. This change continues the
fault-injection test pattern for the remaining 2 test categories.

## 2. Scope

- **Add** `tests/network_fault_injection_test.rs` — 7 network fault scenarios
- **Add** `tests/memory_fault_injection_test.rs` — 7 memory fault scenarios
- **Total**: 14 test scenarios across 2 fault categories

## 3. Test Coverage Matrix

### 3.1 Network Fault (T-17) - 7 scenarios

| Test | Scenario | Coverage |
|------|----------|----------|
| test_30pct_packet_loss_during_select | 30% packet loss | 5 SELECT queries |
| test_zero_loss_baseline | 0% loss baseline | 100 queries |
| test_full_loss_failure | 100% loss | 1 query |
| test_connection_drop_releases_lock | Connection drop | Lock cleanup |
| test_sustained_packet_loss_60s_simulated | 1000 queries at 30% | Performance/stability |
| test_packet_loss_during_prepared_statement | Multi-stmt | 4 stmts |
| test_server_retry_logic | 50% loss + retry | Up to 5 attempts |

### 3.2 Memory Fault (T-18) - 7 scenarios

| Test | Scenario | Coverage |
|------|----------|----------|
| test_oom_during_single_row_insert | OOM at single insert | Budget 100, alloc 50+50 |
| test_oom_during_bulk_insert_1000_rows | OOM at bulk insert | Budget 5000, 100 rows of 100 |
| test_oom_during_transaction_commit_wal_flush | OOM at WAL flush | Budget 200, alloc 100+300 |
| test_oom_during_query_plan_execution | OOM at multi-stage | Budget 500, 3 stages of 200 |
| test_concurrent_queries_with_interleaved_oom | Concurrent OOM | 4 threads × 200 |
| test_memory_leak_detection_across_operations | Leak detection | 10 alloc/free rounds |
| test_oom_recovery_no_partial_state | OOM recovery | Partial state cleanup |

## 4. Acceptance Criteria

- [x] 7+ test cases for T-17 (network)
- [x] 7+ test cases for T-18 (memory)
- [x] All tests pass (14/14)
- [x] Cargo clippy clean for new files
- [x] INT5_PLUS_DEBT_INVENTORY.md updated (T-17, T-18 CLOSED)
- [ ] `cross_version_debt.sh` shows T-17, T-18 CLOSED (after PR merge)
- [ ] PR merged to develop/v3.8.0

## 5. Test Plan

```bash
# Run new tests
cargo test --test network_fault_injection_test
cargo test --test memory_fault_injection_test

# Clippy
cargo clippy --test network_fault_injection_test --test memory_fault_injection_test

# Verify gate
bash scripts/gate/check_cross_version_debt.sh
```

## 6. Implementation

### 6.1 Network Fault (`tests/network_fault_injection_test.rs`)

Uses:
- `PacketLossStats` — atomic counter for sent/lost/recovered
- `FaultInjectingClient` — mock client with configurable loss rate (0-100%)
- Deterministic loss based on query hash (reproducible, not random)

### 6.2 Memory Fault (`tests/memory_fault_injection_test.rs`)

Uses:
- `MemoryBudget` — atomic-tracked allocation counter
- `try_alloc(size)` — returns Ok or Err(OOM) based on budget
- `free(size)` — releases allocation
- `oom_triggered` flag + `reset()` for next test

## 7. Verification Results (2026-06-03)

| Test Suite | Result |
|------------|--------|
| network_fault_injection_test | 7/7 PASS |
| memory_fault_injection_test | 7/7 PASS |
| cargo test -p sqlrustgo --lib | 21/21 PASS (no regression) |
| cargo clippy (new tests) | 0 errors, 0 warnings |

## 8. References

- v3.0.0 COMPLETE_LEGACY_TRACKING_REPORT.md (T-17, T-18)
- T-15 (PR-2839) — precedent for fault injection tests
- openspec change: `t-17-t-18-fault-injection`
- MULTI_VERSION_GOVERNANCE_DAG.md (Phase 4)
- INT5_PLUS_DEBT_INVENTORY.md
