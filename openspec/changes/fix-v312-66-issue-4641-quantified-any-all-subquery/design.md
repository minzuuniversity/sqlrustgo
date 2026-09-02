## Context

Issue #4641: quantified subquery comparisons (`val > ANY (subq)`, `val > ALL (subq)`, `val = ANY (subq)`, etc.) silently return empty result sets. The parser correctly produces `Expression::QuantifiedOp` (parser.rs:7599), but the executors `eval_predicate` (engine_utils.rs:312) and `evaluate_expression` (expr_utils.rs:421) both fall through to `Value::Null` for this variant. WHERE evaluation then returns `false` for every row.

The existing `pre_evaluate_non_correlated_in_subquery` path (engine_select.rs:6120) already handles `IN (SELECT ...)` by executing the subquery once, collecting first-column values, and rewriting the AST to `InList`/`NotInList`. The fix mirrors that pattern for `QuantifiedOp`, but instead of rewriting we evaluate per-row using the engine's `execute_select` method (which is already reachable from `engine_select.rs`).

## Goals / Non-Goals

**Goals:**
- `val > ANY (subq)` returns rows where `val > x` for at least one `x` produced by `subq`.
- `val > ALL (subq)` returns rows where `val > x` for every `x` produced by `subq` (true when subq is empty).
- `val = ANY (subq)` and `val = ALL (subq)` analogous.
- Six operators supported: `=`, `!=`/`<>`, `<`, `<=`, `>`, `>=`.
- Non-correlated case: subquery executes once; result reused across all outer rows.
- Correlated case: conservative `true` fallback (matches existing IN/EXISTS pattern), so Q20-style correlated ANY still works.
- One new AST helper, one Step 1.6 wiring line, two new `Expression::QuantifiedOp` recognition sites, and unit + integration tests.

**Non-Goals:**
- Row-by-row correlated-subquery execution for QuantifiedOp (deferred — the conservative `true` fallback is consistent with IN / NOT IN / EXISTS).
- Rewriting QuantifiedOp into a different AST shape (e.g. a specialised node); the runtime evaluator is sufficient.
- ANY / ALL semantics with NULLs in the subquery result: SQL says `val > ANY (subq_with_null)` returns UNKNOWN (treated as false); we follow standard `eval_predicate` UNKNOWN→FALSE convention.
- `SOME` synonym: parser maps `SOME` to "ANY" quantifier; covered transitively.

## Decisions

### D1. Per-row evaluation via a new helper `eval_quantified_comparison`

Add a new helper:

```rust
fn eval_quantified_comparison(
    &self,
    outer_row: &[Value],
    outer_table_info: &TableInfo,
    binary_op: &Expression,         // the inner BinaryOp(val, op, sentinel)
    quantifier: &str,               // "ANY" | "ALL" | "SOME"
    subq: &SelectStatement,
) -> bool
```

1. Evaluate `binary_op` with `outer_row` and `outer_table_info` BUT substitute the placeholder `Expression::Literal("ANY_SUBQUERY")` on the right with a fresh local value... actually simpler:
   - Pull the LHS from the BinaryOp's `Box<Expression>`.
   - Pull the comparison op (string).
   - Execute the subquery via `self.execute_select(subq)`, get all first-column values.
   - For each value `x` in the result, evaluate `lhs OP x` against `outer_row`; aggregate with ANY / ALL.

Correlated case: detect via `subq_uses_outer_ref`. If correlated, return `true` (conservative, matches IN/EXISTS pattern).

### D2. Step 1.6 calls the new pre-evaluator

After the existing `pre_evaluate_non_correlated_in_subquery` rewrite at engine_select.rs:988, also call `pre_evaluate_non_correlated_quantified_subquery` which rewrites `Expression::QuantifiedOp(...)` non-correlated cases by executing the subquery once and substituting `Literal(true|false)` for the entire subtree. This avoids running the per-row evaluator when possible.

For correlated cases, the rewrite leaves `Expression::QuantifiedOp` intact, and the per-row `eval_quantified_comparison` (called from `eval_predicate`) handles them with the conservative `true` fallback.

### D3. Wire `Expression::QuantifiedOp` into `eval_predicate`

Add an arm in `engine_utils.rs::eval_predicate`:

```rust
Expression::QuantifiedOp(_, _, _) => {
    // Free function — no engine reachable. Conservative true matches IN/EXISTS.
    true
}
```

This matches the existing pattern at line 462 (`Expression::In(...) => true`). The "real" engine-aware evaluation happens via `pre_evaluate_non_correlated_quantified_subquery` in the Step 1.6 path.

### D4. Outer-ref detection: extend `expr_uses_outer_ref`

Add `E::QuantifiedOp(bin, _, subq)` to the `match` in `expr_uses_outer_ref` (engine_select.rs:6223):

```rust
E::QuantifiedOp(bin, _, subq) => {
    expr_uses_outer_ref(bin, inner_cols, engine) || subq_uses_outer_ref(engine, subq)
}
```

### D5. Subquery semantics

Standard SQL semantics:
- `<OP> ANY (subq)`: TRUE if exists `x` in subq result where `lhs <OP> x` is TRUE. FALSE if subq is empty AND semantics matches: actually ANY returns FALSE for empty subq (SQL spec: "any" with empty set is false).
  - Wait — SQL standard says `val > ANY (empty_set)` = FALSE. But `val > ALL (empty_set)` = TRUE.
  - MySQL and Postgres follow this convention. We follow.
- `<OP> ALL (subq)`: TRUE if for every `x` in subq result, `lhs <OP> x` is TRUE. TRUE if subq is empty.
- NULLs in subq: SQL says `NULL OP x` = UNKNOWN; for ANY, the row is treated as FALSE; for ALL, the row is treated as TRUE (since UNKNOWN is "not false"). To keep things simple and match the existing `eval_predicate` convention, we treat NULL in subq result as skipping the row (don't count for ANY, don't break ALL).
- Empty subquery: ANY → false, ALL → true (standard SQL).

## Risks / Trade-offs

- **Correlated case is conservative.** `val > ANY (subquery using outer.col)` returns true for all outer rows when the subquery has any row, which over-includes. This matches the existing IN/EXISTS pattern; users who need precise correlated semantics will need follow-up work.
- **NULL handling in subquery.** Standard semantics can be debated; our implementation treats NULL in subquery as "skip this row" for both ANY and ALL. This matches MySQL's `IN (subq)` behaviour (NULLs in subquery produce FALSE result for IN).
- **Subquery cardinality.** A 1M-row subquery executes once and stores the values in a Vec. For typical teaching queries (small tables) this is fine; for large production queries, the existing CBO infrastructure can route through `HashSemiJoin` later.
- **Performance.** We pre-execute the subquery once per outer query (not per outer row). This is O(N_outer + N_inner) — same as the existing IN-subquery pre-evaluator.

## Verification Plan

- Unit tests in `crates/parser/src/parser.rs` (already exist for parser; add one assertion for the `quantifier` field on `Expression::QuantifiedOp`).
- Integration tests in `tests/integration/sql/v312_66_quantified_any_all_test.rs`:
  - `v312_66_quantified_any_greater_than` — issue's example.
  - `v312_66_quantified_all_greater_than_empty` — `val > ALL (empty)` → all rows match.
  - `v312_66_quantified_all_greater_than_no_match` — `val > ALL (subq_with_higher_max)` → no rows.
  - `v312_66_quantified_any_equals` — `val = ANY (subq)`.
  - `v312_66_quantified_less_than_any` — symmetric.
  - `v312_66_quantified_not_equal_any` — `!= ANY`.
- Manual CLI repro: issue's exact `printf ... | sqlrustgo-cli sqlite --batch --mode csv /tmp/db` returns `2,20\n3,30`.
- `cargo build --all-features`: clean.
- `cargo test -p sqlrustgo-cli --lib`: no regression.
- `cargo test -p sqlrustgo-parser --lib`: no regression.
- `cargo test --test v312_66_quantified_any_all_test`: 6/6 pass.
- `cargo clippy --all-features`: clean.
