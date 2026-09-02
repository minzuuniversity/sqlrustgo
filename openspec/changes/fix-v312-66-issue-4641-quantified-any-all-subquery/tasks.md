## 1. Outer-ref detection extension

- [ ] 1.1 Add `E::QuantifiedOp(bin, _, subq)` arm to the match in `expr_uses_outer_ref` (`src/engine_select.rs:6223`) so the pre-evaluator can correctly classify correlated vs non-correlated QuantifiedOp subqueries.

## 2. Engine-aware quantified-comparison evaluator

- [ ] 2.1 Add a new private helper `eval_quantified_comparison(&self, outer_row, outer_table_info, binary_op, quantifier, subq) -> bool` in `src/engine_select.rs`. Pulls the LHS out of the inner BinaryOp, executes the subquery via `self.execute_select(subq)`, extracts the first column of each result row into `Vec<Value>`, and applies ANY / ALL semantics with the comparison operator string extracted from the BinaryOp. Returns `bool` (true = row passes the WHERE clause).
- [ ] 2.2 Handle the 6 SQL operators: `=`, `!=`/`<>`, `<`, `<=`, `>`, `>=`. Map each to the same comparison used by `sql_compare` (engine_utils.rs:590) and `compare_values` (expr_utils.rs).
- [ ] 2.3 For correlated subqueries (detected via `subq_uses_outer_ref`), return `true` (conservative fallback matching existing IN/EXISTS pattern).

## 3. Pre-evaluator for non-correlated QuantifiedOp

- [ ] 3.1 Add `pre_evaluate_non_correlated_quantified_subquery(&self, where_expr) -> Expression` in `src/engine_select.rs`. Walks the WHERE tree, executes each non-correlated QuantifiedOp subquery once via `self.execute_select`, computes the boolean result, and substitutes the subtree with `Expression::Literal("TRUE")` or `Expression::Literal("FALSE")`. Returns the rewritten tree by reference for unchanged subtrees (avoiding deep clones).
- [ ] 3.2 Wire the new pre-evaluator into Step 1.6 (`src/engine_select.rs:988`) alongside the existing `pre_evaluate_non_correlated_in_subquery` call. If the rewrite produces a different tree, use the rewritten tree in the per-row `eval_predicate` call.

## 4. `eval_predicate` conservative fallback

- [ ] 4.1 Add an explicit `Expression::QuantifiedOp(_, _, _) => true` arm to `eval_predicate` (`src/engine_utils.rs:312`) matching the IN/NOT IN conservative pattern. This prevents panics if any path forgets to call the new pre-evaluator.

## 5. Tests

- [ ] 5.1 Add 1 parser-side unit test in `crates/parser/src/parser.rs` (in `set_op_tests` mod near the v312_65 tests) asserting `Expression::QuantifiedOp(_, "ANY", _)` shape for `val > ANY (SELECT val FROM b)`.
- [ ] 5.2 Add 7 integration tests in `tests/integration/sql/v312_66_quantified_any_all_test.rs`:
  - `v312_66_quantified_any_greater_than` — issue #4641 example.
  - `v312_66_quantified_all_greater_than_no_match` — `> ALL` with no outer row exceeding max.
  - `v312_66_quantified_all_greater_than_empty` — `> ALL` with empty subquery → all rows.
  - `v312_66_quantified_any_equals` — `= ANY` matches against any subquery value.
  - `v312_66_quantified_less_than_any` — `< ANY` symmetric.
  - `v312_66_quantified_not_equal_any` — `!= ANY`.
  - `v312_66_quantified_correlated_does_not_panic` — correlated subquery does not panic.
- [ ] 5.3 Register the integration test in `Cargo.toml` under `[[test]]`.

## 6. Documentation and verification

- [ ] 6.1 Run `cargo build --all-features` and confirm clean build.
- [ ] 6.2 Run `cargo test -p sqlrustgo-cli --all-features --lib` and confirm no regression (73/73).
- [ ] 6.3 Run `cargo test -p sqlrustgo-parser --all-features --lib` and confirm no regression (656+ pass).
- [ ] 6.4 Run `cargo test --test v312_66_quantified_any_all_test` and confirm 7/7 pass.
- [ ] 6.5 Run `cargo clippy --all-features` and confirm clean.
- [ ] 6.6 Reproduce the issue scenario via the exact `printf ... | sqlrustgo-cli sqlite --batch --mode csv /tmp/db` command from the issue body and confirm exit 0 with output `2,20\n3,30`.
