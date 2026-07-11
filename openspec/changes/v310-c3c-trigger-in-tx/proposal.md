## Why

Issue #3738 [V310-03c] Trigger 在事务内执行 (C-3c) is the third sub-task of the v3.10.0 ACID 正确性 work. After #3724 (C-3a/C-3b) fixed the test compilation and proved that `BEGIN ... ROLLBACK` correctly undoes in-memory DML, the remaining gap is **trigger DML participating in the user's transaction**: when a user does `BEGIN; INSERT INTO orders; ROLLBACK;` and a trigger fires on the INSERT, the trigger's side-effect (e.g., `UPDATE inventory SET stock = ...`) MUST be rolled back together with the outer transaction.

The trigger firing path was incomplete on `develop/v3.10.0`:
1. `execute_trigger_update` did not expand `NEW.col` references before parsing the trigger body SQL — it would call `parse()` on raw SQL containing `NEW.quantity` and the parser would either fail or, when the SQL was syntactically parseable, the row scan loop would be fed inconsistent state. In practice the test bodies hung because `parse()` had no defined behavior for `NEW.col` references.
2. `execute_trigger_update` held a read lock on `self.storage` (parking_lot::RwLock) when calling `execute_dml_in_tx` which needs a write lock. parking_lot does not allow reader→writer upgrade, so the call deadlocked.
3. `execute_dml_in_tx` always called `storage.begin_transaction()`, which returns "Nested transactions are not supported on MemoryStorage" when the storage was already in a tx (set by `begin_implicit_dml_tx` for the user's outer transaction).
4. The 3 target tests for C-3c.1, C-3c.2, C-3c.3 (`test_trigger_executes_insert`, `_update`, `_delete`) existed in `tests/stored_proc_catalog_test.rs` but were `#[ignore]`d with reason "MemoryStorage does not support transactions; trigger DML requires transaction boundary".
5. C-3c.4 (`test_trigger_rollback_undoes_trigger_modifications`) and C-3c.5 (`test_trigger_commit_persists_trigger_modifications`) had to be created from scratch.

## What Changes

- **`crates/executor/src/trigger.rs`**:
  - `execute_trigger_update`: call `self.expand_update_values(sql, trigger_table, new_row)` before `parse()` to substitute `NEW.col` with literal values.
  - `execute_trigger_update`: `drop(storage)` (the read lock guard) before calling `self.execute_dml_in_tx(...)` to avoid parking_lot reader→writer deadlock.
  - `execute_dml_in_tx`: detect an active outer transaction via `storage.in_transaction()` and skip `begin_transaction()` / `commit_transaction()` when one is already in progress, so the trigger's DML is part of the user's transaction.
- **`tests/stored_proc_catalog_test.rs`**:
  - Un-`#[ignore]` the 3 existing trigger firing tests.
  - Add `Value` import.
  - Add 2 new tests: `test_trigger_rollback_undoes_trigger_modifications` (C-3c.4) and `test_trigger_commit_persists_trigger_modifications` (C-3c.5).
- **69 other test files**: change `use std::sync::{Arc, RwLock};` to `use parking_lot::RwLock; use std::sync::Arc;` so they compile against the engine's `parking_lot::RwLock`-based `ExecutionEngine::new`. (The `dml_integration_test.rs` fix was already in the prior PR; this PR completes the same migration for the remaining 69 files including `stored_proc_catalog_test.rs`.)

## Capabilities

### New Capabilities

- `v310-c3c-trigger-in-tx`: Trigger-fired DML participates in the user's outer transaction. `BEGIN; INSERT (fires trigger); ROLLBACK;` undoes both the INSERT and the trigger's side-effects; `COMMIT` persists both.

### Modified Capabilities

None.

## Non-goals

- **Multi-row trigger recursion** (a trigger fires its own trigger on the same table) is out of scope. The trigger DML is wrapped in the outer tx; if the trigger's body DML fires a trigger on a different table, the second trigger's body DML is also part of the same outer tx (by the same mechanism), but recursion in the same statement is not tested.
- **Savepoint semantics within trigger bodies** are out of scope.
- **Async / multi-threaded trigger execution** is out of scope.

## Acceptance

- [x] `cargo test --test stored_proc_catalog_test` — 18/18 PASS, 0 FAILED, 0 IGNORED
- [x] All 5 C-3c target tests pass: insert, update, delete, rollback, commit
- [x] `cargo test --test dml_integration_test` — 24/24 still PASS (no regression from #3724)
- [x] `cargo fmt --check --all` — clean
- [x] `cargo clippy --lib -p sqlrustgo --all-features -- -D warnings` — clean
- [x] Comment posted on #3738 with summary and commit link

## Verification

```bash
# C-3c.1-3 (trigger firing on DML)
cargo test --test stored_proc_catalog_test -- test_trigger_executes_insert
cargo test --test stored_proc_catalog_test -- test_trigger_executes_update
cargo test --test stored_proc_catalog_test -- test_trigger_executes_delete

# C-3c.4 (trigger ROLLBACK)
cargo test --test stored_proc_catalog_test -- test_trigger_rollback_undoes_trigger_modifications

# C-3c.5 (trigger COMMIT)
cargo test --test stored_proc_catalog_test -- test_trigger_commit_persists_trigger_modifications
```

## Cross-references

- Parent: #3724 [V310-03] ACID 事务正确性 (C-3) — closed in PR #3740
- Master: #3721 [V310-MASTER] v3.10.0 总控
- C-3 splits: #3724 (a/b) and #3738 (c, this issue)
- Predecessor commits: `7a4050085` (v3.9.0 MemoryStorage TxLog), `694d6ff3b` (v3.10.0 G13 parking_lot migration)
