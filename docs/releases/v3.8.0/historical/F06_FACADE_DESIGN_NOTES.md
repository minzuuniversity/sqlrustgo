# F-06 TransactionalFacade — Design Notes & Stub Status

<!-- env:blocked:no-ci -->

> **PR**: F-06 (PR-800F)
> **Status**: STUB (compile-clean, behaviour pending rewrite)
> **Date**: 2026-06-03
> **Author**: Hermes Agent

## Summary

`crates/executor/src/execution/{transactional_facade,wal_transactional_facade}.rs`
contain the F-06 `TransactionalFacade` abstraction. The trait definition is
small (8 method signatures) and compiles cleanly. The `WalTransactionalFacade`
implementation was 195 lines but **never compiled** — it had 20+ errors against
the current API surface.

This commit replaces the broken implementation with a **minimal stub** that:
- Compiles cleanly (0 errors, 0 warnings introduced)
- Has all 8 trait methods implemented (some as `unimplemented!()`)
- Has 1 test that verifies the trait wiring
- Documents the original errors in a module-level comment

## Original errors (in the 195-line implementation)

1. `WalStorage<S>` was missing the second generic parameter `<T: WalManager>`
2. `WalStorage::new(inner, PathBuf)` signature was wrong; the real API takes a `WalManager` trait object, not a path
3. `WalStorage::new_without_wal` does not exist
4. `sqlrustgo_transaction::TransactionManager::new()` does not exist (real API: `TransactionManager::new(config)` or similar)
5. `parking_lot` not in `crates/executor/Cargo.toml` dependencies
6. `sqlrustgo_transaction` not in `crates/executor/Cargo.toml` dependencies
7. `LocalExecutor` no longer at the executor crate root
8. `sqlrustgo_planner::create_physical_plan` does not exist (real API: `Planner::plan()`)
9. `TransactionContext::new` signature may differ (real API may need `tx_id: TxId`, not `u64`)

These reflect a **deeper design mismatch**: the facade was written when
`executor` depended on `transaction` and had `LocalExecutor` at the root. Both
have since been refactored.

## Design decisions for the stub

- `WalTransactionalFacade<S>` keeps `S: StorageEngine` as a generic parameter
- Uses `std::sync::Mutex<S>` (not `Arc<RwLock<WalStorage<S>>>`) since WAL wiring is pending
- `begin`/`commit`/`rollback`/`execute_write`/`execute_read` are `unimplemented!()` — the type wiring is correct but the bodies need a real implementation
- `is_in_transaction`/`current_tx_id`/`validate_operation` are real (no tx state, so trivial)
- The `DriftGate` is still wired so `validate_operation` is a real delegation

## Rewrite plan (for F-06 follow-up)

When the real implementation is added, the following are needed:

1. **Add dependencies** to `crates/executor/Cargo.toml`:
   - `parking_lot` (or use `std::sync::RwLock`)
   - `sqlrustgo-transaction`

2. **Choose storage wrapping**:
   - Option A: Wrap `S` directly in `Arc<RwLock<S>>` (no WAL inside facade; WAL lives in storage engine)
   - Option B: Wrap `WalStorage<S, T>` (current design) — needs `T: WalManager` as a second generic
   - Option C: Drop the facade's storage ownership; pass `&mut S` to each method

3. **TxManager**:
   - Real `TransactionManager` API: `begin(iso) -> TxId`, `commit(id) -> Timestamp`, `rollback(id) -> ()`
   - `execute_write` must use the `S::insert/update/delete` API plus a WAL log
   - `execute_read` must use the planner + executor (current code references `sqlrustgo_planner::create_physical_plan` and `crate::LocalExecutor` which no longer exist)

4. **Test plan** (when real implementation lands):
   - `test_begin_commit_rollback_lifecycle`
   - `test_execute_write_insert`
   - `test_execute_write_update`
   - `test_execute_write_delete`
   - `test_validate_operation_blocks_invalid`
   - `test_drift_gate_integration`

## Verification

- `cargo check -p sqlrustgo-executor` → 0 errors, 0 warnings (was 20+ errors before)
- `cargo check --all-features` → clean
- `cargo test -p sqlrustgo-executor --lib wal_transactional_facade` → 1/1 PASS
- `cargo test -p sqlrustgo-executor --lib` → 328/328 PASS (no regression)

## Test results

| Test | Status |
|------|--------|
| `facade_stub_compiles_and_basic_state` | ✅ PASS |
| All other executor lib tests | ✅ 327/327 PASS (no regression) |

## References

- SPEC-006 TransactionalFacade contract: `docs/releases/v3.8.0/SPEC-006_TRANSACTIONAL_FACADE.md`
- F-06 entry in FEATURE_CHECKLIST: F-06 NOT_DONE
- WalStorage API: `crates/storage/src/wal_storage.rs`
- TransactionManager API: `crates/transaction/src/transaction_manager.rs`
