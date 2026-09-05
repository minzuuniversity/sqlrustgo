## Why

`sqlrustgo-cli sqlite` (HEAD develop/v3.12.0) accepts `ALTER TABLE t ADD COLUMN new_col INT DEFAULT 99` but the column is not actually added — `SELECT *` from `t` returns the original columns only, with rows un-backfilled. Issue #4647 reports this as another instance of the "DDL 静默假成功" pattern.

The root cause: `FileStorage::add_column` (in `crates/storage/src/file_storage.rs:3199`) pushes the new `ColumnDefinition` onto `data.info.columns` and calls `save_table` to persist, but does NOT backfill the existing `data.rows` with the default value. After the call, the in-memory state has 2-column schema with 1-element rows — a mismatched state. `scan()` (line 2845) returns the rows unchanged, so callers see only 1 column. `MemoryStorage::add_column` (in `crates/storage/src/engine.rs:1827`) does backfill correctly.

This is a P0 issue per the issue-classification report: schema evolution silently fails, breaking any DDL migration.

## What Changes

- Update `FileStorage::add_column` (file_storage.rs:3199) to backfill existing rows with the new column's default value (or `Value::Null` when no default is specified), using the same `default_fill_value` helper that `MemoryStorage::add_column` uses. Then save the table.
- The fix matches the existing `MemoryStorage::add_column` semantics exactly.

No public API change. No breaking change.

## Capabilities

### New Capabilities

- `executor-alter-table-add-column`: `sqlrustgo` MUST actually add a column to a table's schema AND backfill existing rows with the new column's DEFAULT (or NULL) when `ALTER TABLE t ADD COLUMN ...` is executed. Subsequent `SELECT * FROM t` MUST return the new column for all rows.

### Modified Capabilities

- None.
