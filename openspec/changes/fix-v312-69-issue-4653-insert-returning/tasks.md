## 1. AST + parser

- [ ] 1.1 Add `pub returning: Option<Vec<String>>` field to `InsertStatement` (parser.rs:778).
- [ ] 1.2 Update every `InsertStatement { ... }` construction site in `crates/parser/src/parser.rs` to add `returning: None`. There is one construction site at parser.rs:7059 (in `parse_insert`).
- [ ] 1.3 In `parse_insert` (parser.rs:6763), after the `ON CONFLICT` / `ON DUPLICATE KEY UPDATE` handler and before the closing brace, add:
  ```rust
  // V312-69 / Issue #4653: optional RETURNING clause (PostgreSQL/MySQL 8.0+).
  let mut returning: Option<Vec<String>> = None;
  if matches!(self.current(), Some(Token::Returning)) {
      self.next();
      let mut cols = Vec::new();
      if matches!(self.current(), Some(Token::Multiply)) {
          self.next();
          cols.push("*".to_string());
      } else {
          loop {
              let name = match self.next() {
                  Some(Token::Identifier(n)) => n,
                  Some(t) => return Err(format!("Expected column name after RETURNING, got {:?}", t)),
                  None => return Err("Unexpected end of input after RETURNING".to_string()),
              };
              cols.push(name);
              if !matches!(self.current(), Some(Token::Comma)) {
                  break;
              }
              self.next();
          }
      }
      if cols.is_empty() {
          return Err("RETURNING requires at least one column or *".to_string());
      }
      returning = Some(cols);
  }
  ```
  and pass `returning` to the `InsertStatement` construction.
- [ ] 1.4 Check that `Token::Returning` exists in the parser's token enum (crates/parser/src/token.rs). If not, add it (with `returning` -> `Token::Returning` mapping in the lexer's keyword table at lexer.rs:540 area).

## 2. Executor

- [ ] 2.1 In `src/engine_dml.rs::execute_insert`, after the `all_records: Vec<Vec<Value>>` block (line 148), check `if let Some(cols) = &insert.returning` and build a projected `ExecutorResult`:
  ```rust
  if let Some(cols) = &insert.returning {
      let projected_rows: Vec<Vec<Value>> = all_records
          .into_iter()
          .map(|row| {
              if cols.len() == 1 && cols[0] == "*" {
                  row
              } else {
                  cols.iter()
                      .map(|c| {
                          table_info.columns.iter()
                              .position(|tc| tc.name == *c)
                              .and_then(|i| row.get(i).cloned())
                              .unwrap_or(Value::Null)
                      })
                      .collect()
              }
          })
          .collect();
      let row_count = projected_rows.len();
      return Ok(ExecutorResult::new(projected_rows, row_count));
  }
  ```
  Placed BEFORE the REPLACE/INSERT IGNORE/trigger flush logic at the bottom of the function so RETURNING wins.
- [ ] 2.2 Verify the new code compiles in the engine-dml crate.

## 3. Tests

- [ ] 3.1 Add 3 parser unit tests in `crates/parser/src/parser.rs` (in the `set_op_tests` mod):
  - `test_parse_insert_with_returning` — basic form.
  - `test_parse_insert_returning_star` — `RETURNING *`.
  - `test_parse_insert_with_on_conflict_and_returning` — combined.
- [ ] 3.2 Add 4 integration tests in `tests/integration/sql/v312_69_insert_returning_test.rs`:
  - `v312_69_insert_returning_returns_inserted_row` — basic form via engine.
  - `v312_69_insert_returning_with_omitted_clause_unaffected` — no RETURNING returns empty.
  - `v312_69_insert_returning_star_projects_all_columns`.
  - `v312_69_insert_returning_with_multi_row_insert` — multi-row insert.
- [ ] 3.3 Register the integration test in `Cargo.toml` under `[[test]]`.

## 4. Documentation and verification

- [ ] 4.1 Run `cargo build --all-features` and confirm clean build.
- [ ] 4.2 Run `cargo test -p sqlrustgo-parser --all-features --lib` and confirm no regression.
- [ ] 4.3 Run `cargo test -p sqlrustgo-cli --all-features --lib` and confirm 73/73.
- [ ] 4.4 Run `cargo test --test v312_69_insert_returning_test` and confirm 4/4.
- [ ] 4.5 Run `cargo test --test parser_e2e_test` and confirm no regression.
- [ ] 4.6 Run `cargo clippy --all-features` and confirm clean.
- [ ] 4.7 Manual CLI repro from issue body: `printf "CREATE TABLE t(id INT, val INT); INSERT INTO t VALUES (1, 100) RETURNING id, val;" | sqlrustgo-cli sqlite --batch --mode csv /tmp/db` returns `1,100` (currently empty).
