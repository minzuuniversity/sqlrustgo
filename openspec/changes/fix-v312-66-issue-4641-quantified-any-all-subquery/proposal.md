## Why

`sqlrustgo-cli sqlite` (HEAD `b3123fd81`, develop/v3.12.0) silently returns empty result set for quantified subquery comparisons:

```
$ printf "CREATE TABLE a(val INT); CREATE TABLE b(val INT);
INSERT INTO a VALUES (10),(20),(30);
INSERT INTO b VALUES (10),(25),(35);
SELECT * FROM a WHERE val > ANY (SELECT val FROM b);" | sqlrustgo-cli sqlite --batch --mode csv /tmp/db
(empty — should be (20),(30))
```

Issue #4645 reports that `> ANY` / `> ALL` / `= ANY` / `= ALL` subqueries produce empty sets. Root cause: the parser correctly emits `Expression::QuantifiedOp` (parser.rs:7599), but both `eval_predicate` (engine_utils.rs:312) and `evaluate_expression` (expr_utils.rs:421) fall through to `Value::Null` for that variant. The WHERE clause then evaluates as `false` for every row, so all rows are dropped.

The fix is a per-row evaluator for `QuantifiedOp` that:
1. Evaluates the LHS of the inner comparison against the outer row.
2. Executes the subquery once (or per-row for correlated) and collects the first column of each result row.
3. Applies ANY / ALL semantics with the comparison operator.

This is exactly the same shape as the existing `pre_evaluate_non_correlated_in_subquery` path (engine_select.rs:6120) used for `IN (SELECT ...)` — we mirror the pattern for `QuantifiedOp`.

## What Changes

- Add a new helper `pre_evaluate_non_correlated_quantified(&self, where_expr) -> Expression` in `src/engine_select.rs` that rewrites non-correlated `QuantifiedOp` subtrees into a `Literal(true|false)` per-eval (or per-row when correlated) by executing the subquery and applying ANY/ALL semantics.
- Call the new helper from the existing Step 1.6 path (engine_select.rs:965) alongside `pre_evaluate_non_correlated_in_subquery`.
- Add an explicit `Expression::QuantifiedOp` arm to `eval_predicate` (engine_utils.rs:312) that handles the conservative fallback (correlated case where engine isn't reachable from this free function) — return `true` matching the IN/NOT IN conservative pattern.
- Add `Expression::QuantifiedOp` recognition to the outer-ref detector in `expr_uses_outer_ref` (engine_select.rs:6223) so the pre-evaluator can correctly classify correlated vs non-correlated.
- Add unit tests in `crates/parser/src/parser.rs` for the parser shapes (already covered, but assert `quantifier` string is correctly populated).
- Add integration tests in `tests/integration/sql/v312_66_quantified_any_all_test.rs` covering the issue's example, multi-row, ALL semantics, empty subquery, and correlated fallback.

No new public API. No breaking changes.

## Capabilities

### New Capabilities

- `executor-quantified-any-all`: `sqlrustgo` MUST evaluate `val > ANY (subq)`, `val > ALL (subq)`, `val = ANY (subq)`, `val = ALL (subq)` and the symmetric `<`, `>=`, `<=`, `!=` operators correctly. Non-correlated subqueries MUST execute once and the comparison applied across the result set with standard SQL ANY/ALL semantics. This unblocks teaching seeds that ship comparison-subquery filters.

### Modified Capabilities

- None. Existing IN / NOT IN / EXISTS / NOT EXISTS behaviour is unchanged.
