## 1. Parser AST + dispatch

- [ ] 1.1 Add `CreateFulltextIndexStatement` struct in `crates/parser/src/parser.rs` next to `CreateIndexStatement` (parser.rs:240) with fields `name: String`, `table: String`, `columns: Vec<String>`, `if_not_exists: bool`.
- [ ] 1.2 Add `CreateFulltextIndex(CreateFulltextIndexStatement)` variant to the `Statement` enum (search for `CreateIndex(` in parser.rs).
- [ ] 1.3 Re-export `CreateFulltextIndexStatement` from `crates/parser/src/lib.rs` (next to the existing `CreateIndexStatement` re-export).
- [ ] 1.4 Add `parse_create_fulltext_index(&mut self) -> Result<Statement, String>` method on `impl Parser` that parses `FULLTEXT INDEX <name> [IF NOT EXISTS] ON <table>(<col1>, <col2>, ...)`. Mirror the shape of `parse_create_index` (parser.rs:2859).
- [ ] 1.5 Add `Some(Token::Fulltext) => self.parse_create_fulltext_index()` arm to `parse_create`'s `match` block (parser.rs:2616).
- [ ] 1.6 Update the error message at parser.rs:2654-2660 to mention `FULLTEXT` so users with malformed input see it in the suggestion list.

## 2. Executor dispatch

- [ ] 2.1 Wire `Statement::CreateFulltextIndex(_)` arm into `ExecutionEngine::execute` in `src/execution_engine.rs:729` returning `Err(SqlError::ExecutionError("FULLTEXT INDEX is not yet implemented (issue #4645); use CREATE VIRTUAL TABLE ... USING fts5 for SQLite FTS5".to_string()))`.
- [ ] 2.2 Add the same arm to `Statement::CreateFulltextIndex` exhaustive matches if `cargo build` surfaces any other match sites.

## 3. Tests

- [ ] 3.1 In `crates/parser/src/parser.rs` (in the `ddl_database_tests` mod near parser.rs:13946, or in a new `#[cfg(test)] mod fulltext_index_tests` block at the end of the file), add four unit tests:
  - `test_parse_create_fulltext_index_basic` — single column.
  - `test_parse_create_fulltext_index_multi_column` — three columns.
  - `test_parse_create_fulltext_index_if_not_exists` — `IF NOT EXISTS` flag.
  - `test_parse_create_fulltext_index_missing_name_errors` — negative test.
  - `test_parse_create_fulltext_index_missing_columns_errors` — negative test.
- [ ] 3.2 Create `tests/integration/sql/v312_64_create_fulltext_index_test.rs` as an integration test that invokes `sqlrustgo_cli::run` via the binary in a child process (or directly via the public API). Assert: parser accepts, executor returns a runtime error containing `"FULLTEXT INDEX is not yet implemented"` and the issue number `"4645"`.
- [ ] 3.3 Register the integration test in `Cargo.toml` under `[[test]]`.

## 4. Documentation and verification

- [ ] 4.1 Run `cargo build --all-features` and confirm clean build.
- [ ] 4.2 Run `cargo test -p sqlrustgo-parser --all-features` and confirm green.
- [ ] 4.3 Run `cargo test -p sqlrustgo-cli --all-features --lib` and confirm green.
- [ ] 4.4 Run `cargo clippy --all-features -- -D warnings` and confirm clean.
- [ ] 4.5 Reproduce the issue scenario via `printf 'CREATE TABLE t(body TEXT); CREATE FULLTEXT INDEX ft_idx ON t(body);' | target/debug/sqlrustgo sqlite --batch --mode csv /tmp/db` and confirm: parse succeeds, executor prints runtime error with issue #4645, exit code 1.
