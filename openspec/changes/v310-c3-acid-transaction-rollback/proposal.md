## Why

Issue #3724 [V310-03] ACID 事务正确性 (C-3) is the **highest-priority P0** in v3.10.0 — explicitly marked "ACID 不完整不能作为生产替代" in `docs/releases/v3.10.0/plans/V310_DEVELOPMENT_PLAN.md §1.3`. Without true ROLLBACK semantics, the engine cannot ship as a MySQL 5.7 replacement.

Two of the three target tests (`transaction_rollback_undoes_dml`, `transaction_update_then_rollback`) already exist as `#[test]` in `tests/dml_integration_test.rs`, but **the entire test file fails to compile** on `develop/v3.10.0` due to an `Arc<RwLock<MemoryStorage>>` vs `Arc<parking_lot::RwLock<MemoryStorage>>` type mismatch at `tests/dml_integration_test.rs:30`. Until this is fixed, **no test in that file can run** — so we cannot even measure the current ROLLBACK correctness.

This change delivers C-3a (ROLLBACK undoes DML) and C-3b (MemoryStorage transaction boundaries) end-to-end. C-3c (trigger-in-tx) is split out as follow-up issue #3738 because it depends on a separate trigger-execution path that is not on the C-3 critical path.

## What Changes

- **Fix test compilation**: replace `std::sync::RwLock` with `parking_lot::RwLock` in `tests/dml_integration_test.rs` (the only such usage on this branch).
- **Verify ROLLBACK semantics**: run the 5 target tests for C-3a/C-3b and confirm they pass:
  - `transaction_rollback_undoes_dml`
  - `transaction_update_then_rollback`
  - `transaction_commit_persists_dml` (regression guard)
  - `transaction_delete_then_commit` (regression guard)
  - `failed_insert_does_not_corrupt_table` (regression guard)
- **Audit MemoryStorage transaction boundary**: verify that `BEGIN`/`COMMIT`/`ROLLBACK` are correctly bracketing all DML operations on the in-memory backend, with no leaked state across boundaries.
- **Add gate runner**: `check_c3_rollback.sh` — wraps the 5 tests + `cargo build` for the test harness; logs to `audit/c3-acid-rollback.log` for gate evidence.
- **Document completion**: post a results comment on issue #3724 and link to the test run output.

## Capabilities

### New Capabilities

- `v310-c3-acid-rollback`: ACID ROLLBACK correctness for in-memory DML, with end-to-end test coverage proving INSERT/UPDATE/DELETE inside `BEGIN ... ROLLBACK` does not leak into post-rollback state.

### Modified Capabilities

None. The existing `memory-storage-transactions` spec (in `openspec/specs/`) covers related ground but is broader (covers storage engine transaction semantics); this change does not alter its requirements — it verifies they hold for the C-3a/C-3b scope.

## Non-goals

- **C-3c (trigger-in-tx)** is **explicitly out of scope** and tracked in issue #3738. The current PR's ROLLBACK semantics apply to user-issued DML only; trigger firing during a transaction is a separate work item.
- **Crash recovery** (kill -9 mid-transaction, redo/undo log replay) is **out of scope** for this PR — covered by issue #3726 (V310-05, depends on this issue).
- **24h SOAK validation** is **out of scope** — covered by issue #3726.
- **MVCC snapshot isolation semantics** beyond what the existing `transaction_update_then_rollback` test exercises (e.g., concurrent readers seeing pre-image during writer's transaction) — that is part of #3726 / SEM-1 broader MVCC work.
- **Wire-protocol-level transaction handling** (mysql-server COM_QUERY BEGIN/COMMIT/ROLLBACK packet semantics) — covered by issue #3730 (V310-09).

## Acceptance

- [ ] `cargo test --test dml_integration_test` runs cleanly with no compile errors
- [ ] All 5 named tests above PASS
- [ ] `cargo clippy --all-features -- -D warnings` clean
- [ ] `cargo fmt --check --all` clean
- [ ] `audit/c3-acid-rollback.log` contains the test output
- [ ] Comment posted on issue #3724 linking to the commit