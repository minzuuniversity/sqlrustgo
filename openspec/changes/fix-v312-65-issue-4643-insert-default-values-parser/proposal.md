## Why

`sqlrustgo-cli sqlite` (HEAD `d8c6e31f0`, develop/v3.12.0) rejects the SQLite/PostgreSQL/MySQL-standard `INSERT INTO t DEFAULT VALUES` syntax with:

```
$ printf "CREATE TABLE t(id INT, val INT DEFAULT 100, name VARCHAR(20) DEFAULT 'hello'); INSERT INTO t DEFAULT VALUES; SELECT * FROM t;" | sqlrustgo-cli sqlite --batch --mode csv /tmp/db
Error: sqlrustgo:error:parse: Parse error: Expected VALUES or SELECT
```

Issue #4643 reports this as a teaching-seed blocker. The root cause is localised to `parse_insert` in `crates/parser/src/parser.rs:6876` — the dispatch only recognises `Token::Values` or `Token::Select` after the column-list close, and returns `Err("Expected VALUES or SELECT")` on anything else (including `Token::Default`).

The executor already has full DEFAULT-substitution infrastructure: `materialise_default_tokens` in `src/engine_helpers.rs:32` walks each row and replaces `Value::Text("DEFAULT")` sentinels with the column's `default_value` (or `NULL` when the column has no default). `INSERT VALUES (DEFAULT)` already uses this path (parser.rs:6845-6848 emits the sentinel). `INSERT DEFAULT VALUES` is just "one row, every column is DEFAULT" — a trivial extension that reuses the same machinery.

## What Changes

- Add `default_values: bool` field to `InsertStatement` (parser.rs:764) — `true` when the user wrote `DEFAULT VALUES`, `false` otherwise.
- In `parse_insert` (parser.rs:6763), after parsing the optional column list, accept `DEFAULT VALUES` (where `DEFAULT` is the keyword `Token::Default`) as an alternative to `VALUES (...)` and `SELECT`. Set `default_values = true` and skip the values/select parsing.
- In `execute_insert` (`src/engine_dml.rs:60`), when `default_values` is `true`, build a single record where every column position is `Value::Text("DEFAULT")` and call `materialise_default_tokens` to substitute each column's default value. This produces exactly one row matching the table's column-default definitions.
- Parser AST `InsertStatement::default_values` defaults to `false`, so existing tests, serialisations, and match arms are unaffected.
- Add `default_values: bool` to every `Statement::Insert(...)` construction site (so the new field is non-optional).

No engine/storage/WAL change. No public API change beyond the additional struct field. No breaking change.

## Capabilities

### New Capabilities

- `parser-insert-default-values`: `sqlrustgo_parser` MUST accept `INSERT INTO <table> DEFAULT VALUES` and emit `Statement::Insert(InsertStatement { default_values: true, values: [], select: None, .. })`. The executor MUST insert one row with each column populated from its declared `DEFAULT` expression (or `NULL` when the column has no default). This unblocks teaching seeds that ship SQLite/MySQL-standard `DEFAULT VALUES` inserts.

### Modified Capabilities

- None. Existing `INSERT VALUES (...)` and `INSERT ... SELECT` behaviour is unchanged.
