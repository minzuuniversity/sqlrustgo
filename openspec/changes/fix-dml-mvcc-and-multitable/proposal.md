## Why

Four `#[ignore]`-marked tests in `tests/dml_integration_test.rs` cover
pre-existing functionality gaps that have been masked by the ignore
attribute since the test scaffolding was added:

- `transaction_rollback_undoes_dml`,
  `transaction_update_then_rollback` — `MemoryStorage::rollback_transaction`
  returns `Err("Transactions not supported by this storage engine")`,
  so any DML inside a `BEGIN ... ROLLBACK` transaction leaks out as
  committed. `WalStorage` works correctly in production, but unit tests
  default to `MemoryStorage` so the tests are unrunnable.
- `update_multiple_tables`, `delete_multiple_tables` — `UpdateStatement`
  and `DeleteStatement` carry `table: String`, so `UPDATE a, b SET ...`
  and `DELETE a, b FROM a, b` aren't even representable in the AST.

These are the last two known gaps in `tests/dml_integration_test.rs`
after fixing the DELETE-bug, INSERT-SELECT path, and subquery in
SET/WHERE in earlier commits. Closing them takes the file from 20
passed / 4 ignored to 24 passed / 0 ignored.

## What Changes

- `crates/storage/src/engine.rs`: `MemoryStorage` gains an internal
  snapshot log (per-table `Vec<Vec<Value>>` plus WAL-style intent
  records written on `begin_transaction`) and a `rollback_transaction`
  that replays the log. `commit_transaction` discards the log.
  `in_transaction()` reflects the new state.
- `crates/parser/src/parser.rs`: `UpdateStatement.table: String` becomes
  `tables: Vec<TableRef>` (name + alias). `DeleteStatement` gains the
  same `tables: Vec<TableRef>` plus optional `using: Vec<TableRef>`.
  `parse_update` and `parse_delete` accept the multi-table forms.
- `src/execution_engine.rs`: `execute_update` / `execute_delete`
  build a combined schema across the listed tables (cartesian join
  for the unqualified case, like the existing SELECT path) and apply
  SET / WHERE / row-marking against the joined view, then write back.
  The single-table code path stays as a fast-path.

## Capabilities

### New Capabilities

- `memory-storage-transactions`: `MemoryStorage` honours
  `BEGIN` / `COMMIT` / `ROLLBACK` semantics — DML inside a transaction
  is invisible to peers until commit, rollback restores the prior
  state.
- `multi-table-dml`: SQL `UPDATE t1, t2 SET ...` and
  `DELETE t1, t2 FROM ...` parse and execute against the joined view.

### Modified Capabilities

- `dml-statements`: `UpdateStatement` and `DeleteStatement` carry a
  multi-table list instead of a single name; `execute_update` /
  `execute_delete` accept that list.

## Impact

- `crates/storage/src/engine.rs` (MemoryStorage implementation).
- `crates/parser/src/parser.rs` (AST and parser).
- `src/execution_engine.rs` (executors; the join helper may be
  reusable with the existing SELECT code path).
- `tests/dml_integration_test.rs`: four tests un-ignored; total goes
  from 20 passed / 4 ignored to 24 passed / 0 ignored.
- No external API or wire-protocol impact.