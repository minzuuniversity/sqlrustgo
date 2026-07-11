## 1. Trigger Executor Fixes

- [x] 1.1 In `crates/executor/src/trigger.rs::execute_trigger_update`, replace `let normalized = sql.replace(". ", ".");` with `let expanded = self.expand_update_values(sql, trigger_table, new_row);` and update the `parse()` call to use `&expanded`
- [x] 1.2 In `execute_trigger_update`, add `drop(storage);` after the `for row in all_rows { ... }` loop and before the `if has_match { self.execute_dml_in_tx(...) }` block
- [x] 1.3 In `execute_dml_in_tx`, add `let in_outer_tx = storage.in_transaction();` check; skip `begin_transaction` and `commit_transaction` when `in_outer_tx` is true

## 2. Test Un-Ignore

- [x] 2.1 In `tests/stored_proc_catalog_test.rs`, remove the 3 `#[ignore = "MemoryStorage does not support transactions; trigger DML requires transaction boundary"]` lines from `test_trigger_executes_insert`, `test_trigger_executes_update`, `test_trigger_executes_delete`

## 3. New Tests for C-3c.4 and C-3c.5

- [x] 3.1 Add `test_trigger_rollback_undoes_trigger_modifications` to `tests/stored_proc_catalog_test.rs` — verifies trigger DML is rolled back with the outer transaction
- [x] 3.2 Add `test_trigger_commit_persists_trigger_modifications` to `tests/stored_proc_catalog_test.rs` — verifies trigger DML persists with the outer transaction
- [x] 3.3 Add `use sqlrustgo::Value;` import to the test file

## 4. Test File Migration (Batch)

- [x] 4.1 Migrate `tests/stored_proc_catalog_test.rs` `use std::sync::{Arc, RwLock};` → `use parking_lot::RwLock; use std::sync::Arc;` (matches engine signature)
- [x] 4.2 Migrate the same pattern in the other 68 test files that use `ExecutionEngine::with_memory_and_catalog` or similar APIs requiring `Arc<RwLock<...>>`

## 5. Verification

- [x] 5.1 `cargo test --test stored_proc_catalog_test` — 18/18 PASS, 0 FAILED, 0 IGNORED
- [x] 5.2 `cargo test --test dml_integration_test` — 24/24 PASS (no regression from #3724)
- [x] 5.3 `cargo fmt --check --all` — clean
- [x] 5.4 `cargo clippy --lib -p sqlrustgo --all-features -- -D warnings` — clean
- [x] 5.5 OpenSpec change validated: `openspec validate v310-c3c-trigger-in-tx`

## 6. Documentation and Issue Closure

- [x] 6.1 Commit the change with a message referencing #3738 and the OpenSpec change name
- [x] 6.2 Push branch to origin
- [x] 6.3 Post completion comment on #3738 with commit link
- [x] 6.4 Verify #3724 (parent) can now be closed — both C-3a/C-3b (#3724) and C-3c (#3738) are done
- [x] 6.5 Close #3724 with note linking to both PRs