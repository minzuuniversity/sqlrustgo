## Context

Issue #4653: `INSERT ... RETURNING col_list` is silently dropped — the parser consumes the `RETURNING` token but doesn't store the column list anywhere, and the executor returns no rows. This breaks PostgreSQL/MySQL 8.0+ idioms for capturing auto-increment IDs.

Root cause: the parser's `parse_insert` (parser.rs:6763) ends after the optional `ON DUPLICATE KEY UPDATE` / `ON CONFLICT` clause, returning `Statement::Insert(InsertStatement { ... })`. There's no code to recognize or parse a `RETURNING col_list`. The `InsertStatement` struct has no field for it. The executor's `execute_insert` returns an empty result for the INSERT path (no SELECT to project).

The fix is two pieces:
1. **Parser**: recognize `RETURNING` after the ON CONFLICT / ON DUPLICATE KEY UPDATE clause, parse a comma-separated column list, and store it in `InsertStatement.returning`.
2. **Executor**: when `returning` is `Some(cols)`, project the just-inserted rows (`all_records`) to those columns and return them as the result.

This mirrors how PostgreSQL implements RETURNING (PostgreSQL 8.2+): RETURNING is evaluated after the INSERT runs, using the just-stored row data, and projects the same way a SELECT does.

## Goals / Non-Goals

**Goals:**
- `INSERT INTO t VALUES (1, 100) RETURNING id, val` returns one row with the inserted values.
- `INSERT INTO t VALUES (1, 100), (2, 200) RETURNING id` returns two rows.
- `INSERT INTO t (col1) VALUES (val1) RETURNING *` returns the full inserted rows.
- `RETURNING` with no columns (a PostgreSQL extension, returns the full row like `RETURNING *`) — out of scope; we require at least one column.
- `INSERT ... ON CONFLICT ... RETURNING ...` works (the RETURNING applies after the conflict resolution).
- `INSERT ... ON DUPLICATE KEY UPDATE ... RETURNING ...` works (MySQL 8.0+ form).
- InsertStatement gains a new `returning: Option<Vec<String>>` field.
- All existing construction sites for `InsertStatement` are updated to set `returning: None`.

**Non-Goals:**
- `RETURNING` with expressions (e.g. `RETURNING id, col+1`) — not standard, deferred.
- `RETURNING *` — deferred (we require explicit column list).
- `DELETE ... RETURNING` and `UPDATE ... RETURNING` — separate issues (#4685 has UPDATE multi-table but not RETURNING; deferred).

## Decisions

### D1. New field `returning: Option<Vec<String>>` on `InsertStatement`

Added at the end of the struct (parser.rs:778) so the order doesn't break existing field-by-field construction sites that destructure.

### D2. Parse RETURNING after the ON CONFLICT / ON DUPLICATE KEY UPDATE clause

The order in the SQL standard is:
```
INSERT INTO ... VALUES ...
  [ON CONFLICT ... DO UPDATE SET ... | ON DUPLICATE KEY UPDATE ...]
  [RETURNING ...]
```

So RETURNING is the last clause. The existing `parse_insert` ends at the `ON ...` handler. Add a new block after it that checks for `RETURNING` and parses a column list.

### D3. Parse column list as identifier list (with `*` support)

Use the existing `parse_column_list` helper if it exists, otherwise inline a comma-separated identifier parse. For `RETURNING *`, treat as "all columns of the table" — fetch from `table_info.columns` at execute time.

Let me check the existing parser for this — there should be a column-list helper.

Actually, looking at `parse_select_statement` (parser.rs:3899), column parsing for `SELECT col1, col2, *` exists. Reuse the same `*` handling: if the first item is `*`, project all columns; otherwise project the named columns.

### D4. Executor: project just-inserted rows

In `execute_insert` (src/engine_dml.rs:82-148), after `all_records` is built (lines 82-148), if `insert.returning.is_some()`, project each row to the requested columns and return as `ExecutorResult`. The projection reuses the same `table_info.columns` lookup that SELECT projection uses.

For `RETURNING *` (or unset columns), project all columns of the table.

## Risks / Trade-offs

- **`RETURNING` is a PostgreSQL/MySQL extension; SQLite supports it from 3.35+.** Our v3.12.0 already targets SQLite-compatible SQL with PG/MySQL extensions (we have `ON CONFLICT`, `ON DUPLICATE KEY UPDATE`, etc.), so this is consistent.
- **`RETURNING` may require touching multiple executor paths (insert into heap, insert into clustered, INSERT IGNORE, REPLACE).** All paths funnel into the same `all_records` variable at src/engine_dml.rs:148, so one projection point suffices.
- **`RETURNING` with no columns** (PostgreSQL `RETURNING *` shorthand) is parsed as `Some(vec!["*"])` and expanded at execute time.
- **AUTO_INCREMENT integration** (issue #4654, separate P0): RETURNING *captures* the row data, so once the executor returns the just-inserted row, the auto-incremented id (if it had been assigned) would be in the result. But issue #4654 says AUTOINCREMENT currently doesn't assign ids, so RETURNING id will return NULL until #4654 is fixed. This is acceptable — fixing #4653 makes RETURNING work correctly for explicit-value inserts, and the auto-increment integration is a follow-up.

## Verification Plan

- Unit tests in `crates/parser/src/parser.rs`:
  - `test_parse_insert_with_returning` — parses successfully and `returning` is `Some(["id", "val"])`.
  - `test_parse_insert_returning_star` — `RETURNING *` parses and `returning` is `Some(["*"])`.
  - `test_parse_insert_with_on_conflict_and_returning` — combination works.
- Integration test in `tests/integration/sql/v312_69_insert_returning_test.rs`:
  - `v312_69_insert_returning_returns_inserted_row` — `INSERT INTO t VALUES (1, 100) RETURNING id, val` returns `1,100`.
  - `v312_69_insert_returning_with_no_table_match_returns_null` — non-existent column yields Null.
  - `v312_69_insert_returning_with_omitted_clause_unaffected` — INSERT without RETURNING returns empty (existing behaviour).
- `cargo build --all-features` clean.
- `cargo test -p sqlrustgo-cli --lib` no regression.
- `cargo test --test v312_69_insert_returning_test` pass.
- `cargo clippy --all-features` clean.
- Manual CLI repro from issue body: `printf "CREATE TABLE t(id INT, val INT); INSERT INTO t VALUES (1, 100) RETURNING id, val;" | sqlrustgo-cli sqlite --batch --mode csv /tmp/db` returns `1,100`.
