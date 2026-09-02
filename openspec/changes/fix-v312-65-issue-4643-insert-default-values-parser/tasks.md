## 1. Parser AST + dispatch

- [ ] 1.1 Add `pub default_values: bool` field to `InsertStatement` struct in `crates/parser/src/parser.rs` (parser.rs:764). Field defaults to `false`.
- [ ] 1.2 Update every existing `InsertStatement { ... }` construction site in `crates/parser/src/parser.rs` to add `default_values: false`. Sites include `parse_insert` for INSERT VALUES / INSERT SELECT branches.
- [ ] 1.3 In `parse_insert` (parser.rs:6763), after the optional column-list parser and before the existing VALUES/SELECT dispatch, add a new branch that matches `Some(Token::Default)` followed by `Token::Values`. Set a local `default_values` flag to `true` and skip the values/select parsing. Carry the flag through to the `InsertStatement` construction.
- [ ] 1.4 Update the parser's error message at parser.rs:6877 to include `DEFAULT VALUES` in the suggestion list so future readers see it.

## 2. Executor dispatch

- [ ] 2.1 In `execute_insert` (`src/engine_dml.rs:60`), add a branch before the existing VALUES/SELECT logic that handles `insert.default_values == true`. Build a single sentinel row of `Value::Text("DEFAULT")` (one per column) and call `materialise_default_tokens` with `&[]` as the column-list argument so the no-column-list substitution branch fires (engine_helpers.rs:44-60).
- [ ] 2.2 Verify the `Statement::Insert(_)` match in `src/execution_engine.rs:737` (engine.execute) does not need changes — the executor's `execute_insert` is called unconditionally for any InsertStatement.

## 3. Tests

- [ ] 3.1 In `crates/parser/src/parser.rs` (in the existing `set_op_tests` mod near the `v312_63_parser_issues_test`-style tests, around parser.rs:14913), add three parser unit tests:
  - `test_parse_insert_default_values_basic` — `INSERT INTO t DEFAULT VALUES` → `Statement::Insert(InsertStatement { default_values: true, .. })`.
  - `test_parse_insert_default_values_lowercase` — case-insensitive keyword.
  - `test_parse_insert_default_without_values_errors` — `INSERT INTO t DEFAULT` → `Err` containing `"VALUES"`.
- [ ] 3.2 Create `tests/integration/sql/v312_65_insert_default_values_test.rs` integration test with 3 tests:
  - `v312_65_insert_default_values_with_defaults` — CREATE TABLE with DEFAULT clauses, INSERT DEFAULT VALUES, SELECT → row of all defaults.
  - `v312_65_insert_default_values_without_defaults` — all-NULL row.
  - `v312_65_insert_default_values_nonexistent_table_errors` — Err on missing table.
- [ ] 3.3 Register the integration test in `Cargo.toml` under `[[test]]`.

## 4. Documentation and verification

- [ ] 4.1 Run `cargo build --all-features` and confirm clean build.
- [ ] 4.2 Run `cargo test -p sqlrustgo-parser --all-features` and confirm green (no new pre-existing failures introduced).
- [ ] 4.3 Run `cargo test --test v312_65_insert_default_values_test` and confirm 3/3 pass.
- [ ] 4.4 Run `cargo test -p sqlrustgo-cli --all-features --lib` and confirm 73/73 still pass (no regression).
- [ ] 4.5 Run `cargo clippy --all-features` and confirm clean.
- [ ] 4.6 Reproduce the issue scenario via `printf 'CREATE TABLE t(...); INSERT INTO t DEFAULT VALUES; SELECT * FROM t;' | sqlrustgo-cli sqlite --batch --mode csv /tmp/db` and confirm: exit 0, output contains the expected row.
