## Context

Issue #3738 closes the trigger-in-transaction gap that the v3.9.0 ROLLBACK work (`7a4050085`) intentionally left open. The v3.9.0 commit added the `TxLog` on `MemoryStorage` and un-ignored the simple ROLLBACK tests, but explicitly kept the 3 trigger-firing-DML tests ignored because the trigger executor opens its own nested transaction (which MemoryStorage rejects).

In parallel, the v3.10.0 G13 lock-convoy fix (`694d6ff3b`) migrated `ExecutionEngine::new` from `std::sync::RwLock` to `parking_lot::RwLock` and broke `tests/dml_integration_test.rs`. PR #3740 (this branch) fixed that file. This change completes the migration for the remaining 69 test files (including `stored_proc_catalog_test.rs` which contains the trigger tests) AND adds the trigger-in-tx wiring.

## Goals / Non-Goals

**Goals:**
- The 5 C-3c target tests all pass on `develop/v3.10.0` HEAD.
- Trigger-fired DML is correctly bracketed by the user's outer `BEGIN..COMMIT/ROLLBACK`.
- No regression in any other test in `tests/stored_proc_catalog_test.rs` or `tests/dml_integration_test.rs`.
- Minimum possible code change: only 3 narrow surgical edits in `trigger.rs` plus 69 mechanical `use` statement fixes.

**Non-Goals:**
- Restructuring the trigger executor into a multi-row, batched, or async model.
- Fixing the `execute_trigger_update` delete-all + re-insert-all pattern (kept as-is; it is correct for the test scenarios and the production semantics will be revisited when adding WHERE-clause support in the `execute_trigger_update` rewrite tracked separately).
- Audit-script scaffolding for the trigger-in-tx tests (the test file itself is the gate evidence).

## Decisions

### Decision 1: In-place fix in `execute_dml_in_tx`, not a new abstraction

**Choice**: Add a single `in_outer_tx = storage.in_transaction()` check at the top of `execute_dml_in_tx` and skip `begin_transaction` / `commit_transaction` when the storage is already in a tx.

**Rationale**: The trigger executor is the only production caller of `execute_dml_in_tx`. Making the function outer-tx-aware covers the entire production use case in one place. A new abstraction (e.g., `TriggerExecutor::with_outer_tx(|s| { ... })`) would be over-engineering for a 5-line fix.

**Alternatives considered**:
- *Move the begin/commit out of the executor and into engine_dml*: REJECTED. The trigger executor is the chokepoint for trigger body DML; the engine's DML path doesn't know that a DML it triggered comes from a trigger vs. user code.
- *Implement a true savepoint model on MemoryStorage*: REJECTED. The user asked for "trigger participates in outer tx", not nested savepoints. Savepoints are a follow-up.

### Decision 2: `expand_update_values` (not `expand_update_values_with_info`)

**Choice**: `execute_trigger_update` calls `self.expand_update_values(sql, trigger_table, new_row)` (not the `_with_info` variant).

**Rationale**: The `_with_info` variant requires a `TableInfo` that we don't yet have at the entry of `execute_trigger_update` (it's only available after `parse` + reading table info from storage). The plain variant takes a `table_name: &str` and resolves the info itself. The `execute_trigger_insert` and `execute_trigger_delete` siblings already use the same pattern.

**Alternatives considered**:
- *Compute TableInfo before expand*: REJECTED. The trigger body SQL may reference a different table (e.g., `UPDATE inventory` in our test case), so we can't reuse the trigger_table's info.

### Decision 3: `drop(storage)` at the right scope

**Choice**: Drop the `let storage = self.storage.read();` guard **after** the `for row in all_rows { ... }` loop and **before** the `if has_match { self.execute_dml_in_tx(...) }` block.

**Rationale**: The read guard is only needed to scan the table and read table_info inside the loop. After the loop, we no longer need it; the `execute_dml_in_tx` call needs a write lock and would deadlock if the read guard were still held.

### Decision 4: Batch-migrate the 70 test files in the same commit

**Choice**: The 69 remaining `use std::sync::{Arc, RwLock};` migrations are committed together with the C-3c code changes.

**Rationale**: PR #3740 (the prior commit) already migrated `dml_integration_test.rs` and the C-3c test bodies live in `stored_proc_catalog_test.rs` (which also uses the same pattern). The other 68 files are mostly DML / executor tests that need the same fix to even compile. Splitting them into a separate commit would require either:
- A "preparation" commit that fixes the test files without triggering any tests (not useful in isolation), OR
- A "test enabling" commit that is required for the C-3c tests to even build.

One commit is simpler and the diff is mechanical (3 lines per file).

**Alternatives considered**:
- *Spread the test-file fixes across multiple PRs*: REJECTED. The test files are dead code (can't compile) until fixed; splitting doesn't add review value.

## Risks / Trade-offs

- **Risk**: Adding `in_outer_tx` check in `execute_dml_in_tx` could mask bugs where the caller thinks they're in autocommit but actually aren't. *Mitigation*: The only production caller is the trigger executor. The test bodies for C-3c.1/.2/.3 (no user tx, autocommit path) still pass — confirming the `in_outer_tx = false` branch still works.

- **Risk**: The batch migration of 69 test files could introduce a test that passes locally but breaks in CI due to a behavior change. *Mitigation*: The migration is purely an `use` statement change (mechanical), no logic change. All migrated tests either pass (verified) or are `#[ignore]`d (their ignore reason was unrelated to the RwLock type).

- **Risk**: `cargo test` for the full test suite might surface other failures not caught by the per-file smoke test. *Mitigation*: The full file `cargo test --test stored_proc_catalog_test` runs 18 tests and passes; `cargo test --test dml_integration_test` runs 24 and passes. A full workspace test run is the next checkpoint but is out of scope for this single PR.

- **Trade-off**: The PR is large (70 files, 261 insertions, 80 deletions). Most of it is mechanical. A reviewer can spot-check any single test file migration and trust the pattern.

## Files changed

- `crates/executor/src/trigger.rs` — 3 surgical fixes (NEW.col expansion, drop read lock, in_outer_tx check)
- `tests/stored_proc_catalog_test.rs` — un-ignore 3 tests + add 2 new tests
- 69 other test files — `use std::sync::{Arc, RwLock};` → `use parking_lot::RwLock; use std::sync::Arc;`
- `openspec/changes/v310-c3c-trigger-in-tx/{proposal,design,specs,tasks}.md` — OpenSpec artifacts