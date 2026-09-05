## 1. Parser AST

- [x] 1.1 In `crates/parser/src/parser.rs` around the `JoinClause` struct, add a new field:
  ```rust
  /// V312-73 / Issue #4649: `JOIN t2 USING (col1, col2, ...)`. When set,
  /// the join is `t1.col_i = t2.col_i` for each column in the list
  /// (USING-merge semantics) and each USING column appears once in the
  /// output (the duplicate right-side column is projected away).
  /// `None` for plain `ON` joins.
  pub using_columns: Option<Vec<String>>,
  ```

- [x] 1.2 Add `using_columns: None` to every other `JoinClause { ... }` constructor in the codebase:
  - `crates/parser/src/parser.rs` (3 sites: comma-join rewriter chain pushes + cartesian extra_tables)
  - `crates/optimizer/src/join_reorder.rs` (join reorder planner)

## 2. Parser — USING grammar

- [x] 2.1 In `crates/parser/src/parser.rs::parse_join_clause` (around the existing ON branch), add a `USING (col1, col2, ...)` branch after the alias check. Reject `USING ()` (empty column list). The branch is mutually exclusive with ON — ON takes precedence when both are present (matching PostgreSQL/SQLite).

## 3. Executor — USING-merge semantics

- [x] 3.1 In `src/engine_select.rs::execute_single_join`, before `find_join_key_index` is called, build the `(left_idx, right_idx)` pairs from `using_columns` via `lookup_column` (already a private free fn in this file). Feed them as `JoinKey::Pairs(pairs)` to the existing hash-join machinery — no new join algorithm required.

- [x] 3.2 After the matched rows and combined schema are produced, drop the right-side USING columns from both:
  - row values: remove `left_col_count + right_idx` from every row (in reverse to keep earlier indices valid);
  - schema: keep all columns whose combined-schema index is not in `drop_indices`.

## 4. Tests

- [x] 4.1 Parser unit tests in `crates/parser/src/parser.rs`:
  - `test_parse_join_using` (asserts `using_columns == Some(["id"])`)
  - `test_parse_join_using_multi_cols` (asserts multi-column list)
  - `test_parse_join_using_left` (asserts LEFT join type preserved)
  - `test_parse_join_using_empty_errors` (asserts `USING ()` syntax error)
  - Upgrade silent `test_parse_inner_join_using` to assert the AST instead of `let _ =`.

- [x] 4.2 Integration test `tests/integration/sql/v312_73_join_using_clause_test.rs` covering:
  - INNER JOIN USING single col: match only, output 2 cols.
  - INNER JOIN USING no overlap: COUNT = 0 (regression — pre-fix this would have returned the cartesian count).
  - LEFT JOIN USING unmatched left: 3 rows (one per left row), null padding for unmatched.
  - SELECT \* with USING: column count matches left + right - 1.
  - USING (col1, col2): multi-column USING match and projection.
  - Register in `Cargo.toml` `[[test]]` block.

## 5. Documentation and verification

- [x] 5.1 `cargo build --all-features` — clean.
- [x] 5.2 `cargo test -p sqlrustgo-parser --lib test_parse_join_using` — 4/4.
- [x] 5.3 `cargo test -p sqlrustgo-parser --lib test_parse_inner_join_using` — 1/1.
- [x] 5.4 `cargo test --test v312_73_join_using_clause_test` — 5/5.
- [x] 5.5 CLI repro from issue body (`sqlrustgo sqlite --batch`) — LEFT JOIN USING returns 3 rows (one per left row, null-padded), not a 3×3 Cartesian product.