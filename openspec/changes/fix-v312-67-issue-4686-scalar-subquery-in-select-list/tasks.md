## 1. New scalar-subquery execution helper

- [ ] 1.1 Add a new private method `execute_subquery_for_scalar(&self, subq: &SelectStatement) -> Result<Value, String>` on `ExecutionEngine` (in `src/engine_select.rs`). It handles two cases:
  - **No-table case** (`subq.table.is_empty()` and no `from_subquery`): evaluate the projection's first column as a literal expression via `crate::expr_utils::evaluate_expression_with_seq` and return the result. (For `SELECT 1` the projection's first column is `Literal("1")` which evaluates to `Value::Integer(1)`.)
  - **Single-table aggregate case** (`subq.aggregates.len() == 1`): call `self.execute_select(subq)` and return the first row's first column. This re-uses the full engine but only for the small subset of subqueries that are leaf-scalar-safe (single aggregate, no correlated subqueries, no joins).
  - **Other cases** (multiple tables, joins, GROUP BY, correlated subqueries): return `Err("Scalar subquery in SELECT list is only supported for literal or single-table-aggregate form")` to avoid the hang we saw with the naive recursive call.
- [ ] 1.2 The helper's name `execute_subquery_for_scalar` makes the intent clear in the projection loop.

## 2. Wire the helper into the projection loop

- [ ] 2.1 In `src/engine_select.rs:1832` (the projection's `subq_eval` closure), replace `&|_| Ok(Value::Null)` with `&|subq: &SelectStatement| self.execute_subquery_for_scalar(subq)`. Capture `&self` implicitly via the closure environment.
- [ ] 2.2 Verify that the closure parameter type matches the `subq_eval: &dyn Fn(&SelectStatement) -> Result<Value, String>` signature in `evaluate_expression_with_seq` (it does — `&SelectStatement` is the same as `&sqlrustgo_parser::SelectStatement` after type resolution).

## 3. Tests

- [ ] 3.1 Add `tests/integration/sql/v312_67_scalar_subquery_in_select_test.rs` with 5 tests:
  - `v312_67_scalar_subquery_literal` — `SELECT (SELECT 1) AS x` → `1`.
  - `v312_67_scalar_subquery_from_table` — `SELECT (SELECT MAX(a) FROM t) AS m` against `t = (10, 20, 30)` → `30`.
  - `v312_67_scalar_subquery_multiple_columns` — `SELECT (SELECT 1) AS a, (SELECT 2) AS b` → `(1, 2)`.
  - `v312_67_scalar_subquery_empty` — `SELECT (SELECT * FROM t WHERE 1=0) AS empty` → `NULL`.
  - `v312_67_scalar_subquery_where_unaffected` — regression check for #4629.
  - `v312_67_scalar_subquery_any_unaffected` — regression check for #4641.
- [ ] 3.2 Register the integration test in `Cargo.toml` under `[[test]]`.

## 4. Documentation and verification

- [ ] 4.1 Run `cargo build --all-features` and confirm clean build.
- [ ] 4.2 Run `cargo test -p sqlrustgo-cli --all-features --lib` and confirm 73/73 still pass.
- [ ] 4.3 Run `cargo test -p sqlrustgo-parser --all-features --lib` and confirm 656+/658 pass (2 pre-existing unrelated failures).
- [ ] 4.4 Run `cargo test --test v312_67_scalar_subquery_in_select_test` and confirm 5/5 pass.
- [ ] 4.5 Run `cargo test --test v312_66_quantified_any_all_test` and confirm 7/7 still pass (#4641 regression).
- [ ] 4.6 Run `cargo clippy --all-features` and confirm clean.
- [ ] 4.7 Reproduce the issue scenario via `printf "SELECT (SELECT 1) AS x;" | sqlrustgo-cli sqlite --batch --mode csv /tmp/db` and confirm output `x\n---\n1`.
