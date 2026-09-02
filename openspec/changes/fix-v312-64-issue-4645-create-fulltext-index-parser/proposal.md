## Why

`sqlrustgo-cli sqlite` (HEAD `020998074`, develop/v3.12.0) rejects `CREATE FULLTEXT INDEX` with a parse error:

```
$ printf 'CREATE TABLE t(body text); CREATE FULLTEXT INDEX ft_idx ON t(body);' \
    | sqlrustgo-cli sqlite --batch --mode csv /tmp/db
Error: sqlrustgo:error:parse: Parse error: Expected TABLE, INDEX, PROCEDURE, FUNCTION, TRIGGER, ROLE, VIEW, SEQUENCE, DATABASE, or USER after CREATE, got Fulltext
```

Issue #4645 reports this as a teaching-seed blocker: applications that ship MySQL-compatible `CREATE FULLTEXT INDEX` DDL cannot load their schemas through sqlrustgo. The error message itself points to the bug — the parser's `parse_create` dispatcher (crates/parser/src/parser.rs:2616-2657) only recognises ten `CREATE` object kinds and reports `Fulltext` as unexpected, even though the lexer already promotes the keyword to `Token::Fulltext` (crates/parser/src/lexer.rs:544).

The full FTS5/FTS3 storage engine is out of scope for this change. The minimum acceptable behaviour, per the issue, is "应被解析 (SQLite FTS5 语法) 或接受为不可识别 (但不应解析错)" — the parser must produce a valid AST node, and the executor must surface a clear runtime error instead of a parser crash. This matches how `CREATE FULLTEXT INDEX` is handled in PostgreSQL/MySQL drivers today: the SQL parses, the engine returns a "not implemented" error.

## What Changes

- Add `Token::Fulltext` arm to the `parse_create` dispatcher in `crates/parser/src/parser.rs:2616` that delegates to a new `parse_create_fulltext_index` method.
- Add `parse_create_fulltext_index` that consumes the FTS-style `CREATE FULLTEXT INDEX <name> ON <table>(<col_list>)` syntax and returns a new `Statement::CreateFulltextIndex` variant carrying `{ name, table, columns }`.
- Add `CreateFulltextIndexStatement` struct to the parser AST (alongside `CreateIndexStatement` at parser.rs:240).
- Re-export `CreateFulltextIndexStatement` from `crates/parser/src/lib.rs` so other crates can `match` on it.
- Wire `Statement::CreateFulltextIndex` into the execution engine's `execute()` dispatch (`src/execution_engine.rs:729`) — return `SqlError::ExecutionError("FULLTEXT INDEX is not yet implemented (issue #4645)")` so the user gets a clear runtime error rather than a parse crash.
- Add a parse-only regression test under `crates/parser/src/parser.rs` (in the existing `ddl_create_database_tests` / `parser_e2e` region) covering the FTS3 syntax shape.
- Add a CLI integration regression test in `tests/integration/sql/v312_64_create_fulltext_index_test.rs` verifying: parser accepts, executor returns a clear error.

No FTS5/FTS3 storage engine implementation. No new external dependency. No breaking change.

## Capabilities

### New Capabilities

- `parser-create-fulltext-index`: the parser must accept `CREATE FULLTEXT INDEX <name> ON <table>(<col_list>)` and emit a `Statement::CreateFulltextIndex` AST node. The executor must reject execution with a clear "FULLTEXT INDEX is not yet implemented" runtime error. This unblocks schema-loading for MySQL-style teaching seeds and makes the AST ready for a future FTS5/FTS3 storage implementation.

### Modified Capabilities

- None. Existing `CREATE INDEX`, `CREATE UNIQUE INDEX`, `CREATE TABLE`, and friends are unchanged.
