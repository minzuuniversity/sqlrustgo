## Why

`sqlrustgo-cli sqlite` (HEAD develop/v3.12.0) silently drops the `RETURNING` clause on `INSERT` statements:

```
$ printf "CREATE TABLE t(id INT, val INT); INSERT INTO t VALUES (1, 100) RETURNING id, val;" \
    | sqlrustgo-cli sqlite --batch --mode csv /tmp/db
(no output — should be "1,100")
```

Issue #4653 reports this. The parser consumes `RETURNING col1, col2` as a no-op (no error, no field stored), and the executor returns no rows. The bug is the parser never records the RETURNING column list in `InsertStatement`.

This is a P0 issue per the issue-classification report: silent accept-and-ignore is a "假成功" pattern that breaks the PostgreSQL/MySQL 8.0+ INSERT … RETURNING idiom (used everywhere for capturing auto-increment IDs).

## What Changes

- Add a `pub returning: Option<Vec<String>>` field to `InsertStatement` (parser.rs).
- In `parse_insert` (parser.rs:6763), after parsing the optional `ON CONFLICT` / `ON DUPLICATE KEY UPDATE` clause, check for the `RETURNING` keyword. If present, consume it, then parse a comma-separated identifier list (mirroring the `SELECT column list` pattern) and store it in `insert.returning`.
- In `execute_insert` (src/engine_dml.rs:60), if `insert.returning` is `Some(cols)`, build the `ExecutorResult` from the just-inserted rows (the rows built earlier as `all_records`) instead of returning an empty result. Project each row to the requested columns (lookup by name in `table_info.columns`).
- Update every existing `InsertStatement { ... }` construction to add `returning: None`.

No public API change beyond the new field. No storage/WAL change. No breaking change for existing behaviour (RETURNING-less INSERTs are unaffected).

## Capabilities

### New Capabilities

- `parser-insert-returning`: `sqlrustgo` MUST accept `INSERT ... RETURNING col1, col2, ...` and the executor MUST return one result row per inserted row, projected to the requested columns.

### Modified Capabilities

- None. Existing INSERT behaviour is unchanged for statements without RETURNING.
