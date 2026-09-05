## 1. Executor implementation

- [ ] 1.1 Add `fn query_sqlite_master(&self) -> SqlResult<ExecutorResult>` to `impl<S: StorageEngine + 'static> ExecutionEngine<S>` in `src/engine_select.rs` (placed just after `rewrite_view_from`, around line 537). The method:
  - Calls `self.storage.read().list_tables()` to enumerate.
  - For each table, calls `self.storage.read().get_table_info(name)` to get the column list.
  - Builds a row with 5 columns: `Value::Text("table")` (type), `Value::Text(name)`, `Value::Text(name)` (tbl_name), `Value::Integer(0)` (rootpage), `Value::Text(format!("CREATE TABLE {} (...)", name))` (sql) — the exact SQL is best-effort, but it must start with `CREATE TABLE`.
  - Returns `Ok(ExecutorResult::new(rows, rows.len()))`.
- [ ] 1.2 In `execute_select` (src/engine_select.rs:504), after the `rewrite_view_from` block (~line 537), add a check for `select.table` (and the bare name when aliased, mirroring the `lookup_table` logic at line 738):
  ```rust
  // V312-71 / Issue #4664: synthesize the canonical SQLite system table.
  if !select.from_subquery.is_some()
      && !select.from_values.is_some()
      && select.join_clause.is_empty()
  {
      let bare = select.table.split_once('|').map(|(t, _)| t).unwrap_or(&select.table);
      if bare.eq_ignore_ascii_case("sqlite_master") || bare.eq_ignore_ascii_case("sqlite_schema") {
          return self.query_sqlite_master();
      }
  }
  ```
  This intercepts the bare single-table form of the system-table query before storage is touched. Multi-table joins against system tables are not supported (deferred — users typically `SELECT * FROM sqlite_master` only).
- [ ] 1.3 For `SELECT name, sql FROM sqlite_master`, project the synthesized rows to the requested columns. Use a simple positional project: `col_idx` for the synthesized schema (type=0, name=1, tbl_name=2, rootpage=3, sql=4), map to lowercase header name. (We accept the simple case `SELECT *` and `SELECT <known_col>` — `SELECT * FROM (subq) AS x` etc. falls through to the regular path because the FROM is parsed as a subquery, not a bare table.)

## 2. Tests

- [ ] 2.1 Add 6 integration tests in `tests/integration/sql/v312_71_sqlite_master_test.rs` covering the spec scenarios.
- [ ] 2.2 Register the integration test in `Cargo.toml` under `[[test]]`.

## 3. Documentation and verification

- [ ] 3.1 Run `cargo build --all-features` and confirm clean.
- [ ] 3.2 Run `cargo test -p sqlrustgo-cli --all-features --lib` and confirm no regression.
- [ ] 3.3 Run `cargo test --test v312_71_sqlite_master_test` and confirm 6/6 pass.
- [ ] 3.4 Run `cargo clippy --all-features` and confirm clean.
- [ ] 3.5 Manual CLI repro from issue body: `printf "CREATE TABLE t(id int); SELECT * FROM sqlite_master;" | sqlrustgo-cli sqlite --batch --mode csv /tmp/db` returns one row for table `t`.
