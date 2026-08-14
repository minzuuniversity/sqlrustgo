# V312-56C: Transaction / Crash Recovery Teaching

**Date**: 2026-08-15
**Issue**: #4252
**Status**: TEACHING_MATERIALS_CREATED

## Overview

SQLRustGo v3.12 includes WAL (Write-Ahead Logging) and MVCC (Multi-Version Concurrency Control) for transaction safety and crash recovery.

## Existing Test Infrastructure

### Transaction Tests
| File | Coverage |
|------|----------|
| `tests/integration/transaction/mvcc_transaction_test.rs` | MVCC transaction semantics |
| `tests/integration/transaction/savepoint_test.rs` | SAVEPOINT support |
| `tests/integration/transaction/sem1_savepoint_test.rs` | SEM-1 SAVEPOINT |

### Crash/Recovery Tests
| File | Coverage |
|------|----------|
| `tests/integration/stress/process_kill_crash_test.rs` | kill -9 crash scenarios |
| `tests/integration/stress/crash_monkey_test.rs` | Randomized crash injection |
| `tests/integration/stress/crash_test_framework.rs` | Crash test framework |
| `tests/integration/stress/crash_test_harness.rs` | Test harness |
| `tests/integration/stress/recovery_fuzzer_test.rs` | Recovery fuzzing |
| `tests/integration/stress/recovery_scenarios_test.rs` | Recovery scenarios |

## Key Concepts

### WAL (Write-Ahead Logging)
- All modifications are written to WAL before being applied to the main storage
- WAL enables crash recovery by replaying uncommitted transactions

### MVCC (Multi-Version Concurrency Control)
- Readers don't block writers
- Writers don't block readers
- Snapshot isolation for transactions

### Transaction States
```
BEGIN → (modifications) → COMMIT → (WAL flush) → durable
       ↘ (modifications) → ROLLBACK → (undo)
```

## Running Tests

```bash
# Transaction tests
cargo test --test mvcc_transaction_test
cargo test --test savepoint_test

# Crash/recovery tests
cargo test --test process_kill_crash_test
cargo test --test crash_monkey_test
cargo test --test recovery_fuzzer_test
```

## Verification Commands

```bash
# Check crash recovery gate
bash scripts/gate/check_v312_14_crash_recovery.sh

# Check WAL integration
cargo test --test wal_tx_contract_tests

# Check migration integration
cargo test --test migration_wal_integration_test
```

## Teaching SQL Fixtures

See `tests/compat/teaching_sql_v3_12/transaction/` for basic transaction SQL fixtures.

## Relationship to V312-14

V312-14 (Crash Recovery & Upgrade/Downgrade Verification) is the parent issue tracking the production hardening of these features.
