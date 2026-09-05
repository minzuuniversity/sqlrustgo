## Why

`sqlrustgo-cli sqlite` (HEAD develop/v3.12.0) returns `Table not found: sqlite_master` when querying the standard SQLite `sqlite_master` system table. Issue #4664 reports this. Many tools (ORM mappers, migration scripts, schema inspectors) probe `sqlite_master` to discover tables; the error blocks this canonical introspection pattern.

The fix is to synthesize the `sqlite_master` view at execute time, populating it from the storage engine's `list_tables()` and `get_table_info()`. We do NOT persist `sqlite_master` to disk — it's a virtual view assembled on demand.

## What Changes

- Add a new method `query_sqlite_master(&self) -> SqlResult<ExecutorResult>` on `ExecutionEngine` (in `src/engine_select.rs`). It:
  - Calls `self.storage.read().list_tables()` to enumerate user tables.
  - For each table, calls `self.storage.read().get_table_info(name)` to get the columns.
  - Emits one row per table with columns: `type`, `name`, `tbl_name`, `rootpage`, `sql` (synthesized `CREATE TABLE ...` statement).
- In `execute_select` (line 504), at the very top after `rewrite_view_from`, check if `select.table` (or its alias) is one of: `sqlite_master`, `sqlite_schema`. If so, return the synthesized result directly. Other system tables (sqlite_temp_master, sqlite_sequence) are deferred.
- All other system-table accesses (e.g. `information_schema.*`) remain out of scope.

No public API change. No storage trait change. No breaking change.

## Capabilities

### New Capabilities

- `executor-sqlite-master-system-table`: `sqlrustgo` MUST return a synthesized `sqlite_master` view on `SELECT * FROM sqlite_master`, `SELECT * FROM sqlite_schema`, and any column subset. The result has columns `type`, `name`, `tbl_name`, `rootpage`, `sql` matching SQLite's actual schema layout.

### Modified Capabilities

- None.
