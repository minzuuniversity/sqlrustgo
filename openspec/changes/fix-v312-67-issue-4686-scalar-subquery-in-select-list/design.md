## Context

Issue #4686: scalar subqueries in the SELECT list return the wrong value. The projection loop at `src/engine_select.rs:1826` passes a `subq_eval` closure that always returns `Value::Null` for any subquery, regardless of whether the subquery is in the projection list or as a scalar operand in a comparison.

This is structurally the same pattern as issue #4629 (scalar subquery in WHERE) and #4641 (quantified subquery in WHERE), both of which were fixed by introducing a per-call `subq_eval` closure that calls `self.execute_select(subq)`. The fix for #4686 follows the same pattern but lives in the projection loop (Step 5 of `execute_select`), not the WHERE loop (Step 1.5/1.6).

A naive fix — call `self.execute_select(subq)` from inside the projection's `evaluate_expression_with_seq` — hangs the executor. The reason: the inner `execute_select` call goes through the full pipeline (Step 1, Step 1.5, Step 1.6, decorrelate, prewarm, Step 3, Step 4, Step 5) and the projection step's `evaluate_expression_with_seq` is called recursively for every outer row. For a leaf scalar subquery like `SELECT 1`, all of these heavy passes are unnecessary — the subquery has no tables, no WHERE, no joins, no aggregates; the result is just a literal.

The fix needs two pieces:
1. A real `subq_eval` closure in the projection that calls `self.execute_select(subq)`.
2. A guard that detects when `execute_select` is being called recursively from within another `execute_select` call (i.e., as a projection subquery), and in that case skips the heavy pre-evaluation passes that are unnecessary for a leaf scalar subquery.

## Goals / Non-Goals

**Goals:**
- `SELECT (SELECT 1) AS x` returns the row `(1)`.
- `SELECT (SELECT 1) AS x` from a real table works the same way: `SELECT (SELECT MAX(a) FROM t) AS m FROM t` returns the max.
- No regression: `WHERE val = (subq)` (#4629) and `WHERE val > ANY (subq)` (#4641) still work.
- One new boolean field on `ExecutionEngine` (`projection_eval_depth: Cell<u32>`), one new `with_projection_depth` RAII guard, one projection-loop change.
- ~3 new tests.

**Non-Goals:**
- Recursive multi-level subqueries: `SELECT (SELECT (SELECT 1))` is out of scope (would require a proper recursive depth limit + cycle detection; current code can only handle one level).
- Correlated subqueries in SELECT list: deferred (same conservative-true pattern as WHERE-side subqueries).
- The `Subquery` arm in `evaluate_expression` (`src/expr_utils.rs:428`) stays the same — it takes the closure; the fix is purely in the caller.

## Decisions

### D1. RAII guard for projection-depth

Add a `Cell<u32>` field on `ExecutionEngine`:
```rust
projection_eval_depth: Cell<u32>,
```

In `execute_select`, at the very top after the substitution pass, check:
```rust
let projection_depth = self.projection_eval_depth.get();
if projection_depth > 0 {
    // Recursive call from projection subquery eval — skip heavy passes.
    // We still need: substitute session vars, execute the SELECT itself
    // (Step 1, Step 5 projection), but skip: prewarm, decorrelate, Step 1.5,
    // Step 1.6, pre_evaluate_quantified_subquery.
}
```

In the projection loop's `subq_eval` closure:
```rust
self.projection_eval_depth.set(self.projection_eval_depth.get() + 1);
let res = self.execute_select(subq);
self.projection_eval_depth.set(self.projection_eval_depth.get() - 1);
```

This is the simplest depth tracking that works in a `&self` method.

### D2. Recursion limit of 1

The depth check is `> 0`, which means depth 1 (the projection subquery) skips heavy passes but still does basic execution. Depth 2 (a subquery inside a subquery's projection) also skips — the assumption is that nested scalar subqueries are simple enough. We document this limitation in the OpenSpec spec.

If we want a hard limit, we can return an error when depth exceeds 2. For now, allow unbounded but document.

### D3. Skip-set in `execute_select`

The recursive call to `execute_select` (from projection) should skip:
- Step 1.5 correlated pre-evaluator (line 862)
- Step 1.6 non-correlated subquery pre-evaluator (line 988)
- prewarm_scalar_agg_index_for_select (line 518)
- decorrelate (line 533)
- pre_evaluate_quantified_subquery (added in #4641 fix)

It should still execute:
- Session var substitution (always)
- Step 1 (FROM / no-table case)
- Step 5 projection (so the leaf scalar subquery's projection runs)
- Step 6 DISTINCT, Step 7 ORDER BY, Step 8 LIMIT/OFFSET (cheap)

The simplest implementation: when `projection_depth > 0`, early-return the inner `execute_select` after running only Steps 1, 5, 6, 7, 8. But that's intrusive. A cleaner alternative: make the closure in the projection NOT call `self.execute_select` but instead call a new private method `execute_subquery_for_scalar` that only does the bare minimum (no FROM scan needed for literal queries like `SELECT 1`, but still supports `SELECT MAX(a) FROM t`).

**Final decision: D3 implementation** — add a new private method `execute_subquery_for_scalar(&self, subq) -> Result<Value, String>` that:
1. Handles the no-table case (returns the literal value of the projection).
2. Handles the `SELECT agg FROM t` case (scans `t`, computes the aggregate, returns the scalar).
3. Skips WHERE / JOIN / ORDER BY / GROUP BY (the inner subquery is a leaf scalar, no correlated context).

This is simpler than threading a depth counter and avoids the re-entrancy question entirely.

## Risks / Trade-offs

- **The simplification of "leaf scalar subquery" is correct for `SELECT literal` and `SELECT agg FROM t` but may not handle `SELECT col FROM t WHERE ...` in a projection slot.** We accept that limitation — those queries return an error like "Scalar subquery in SELECT list must be a literal or single-row aggregate" rather than the wrong answer.
- **Correlated subqueries in projection slot** continue to return `Value::Null` (via the existing conservative fallback). This is consistent with how correlated subqueries in WHERE work today.
- **The closure capture of `&self` works because `execute_select` takes `&self`** — no `&mut self` borrow conflicts.

## Verification Plan

- Unit test: `parse` confirms `SELECT (SELECT 1) AS x` is valid SQL (already exists; reuse).
- Integration test in `tests/integration/sql/v312_67_scalar_subquery_in_select_test.rs`:
  - `v312_67_scalar_subquery_literal` — `SELECT (SELECT 1) AS x` → `1`.
  - `v312_67_scalar_subquery_from_table` — `SELECT (SELECT MAX(a) FROM t) AS m FROM t` → `MAX(a)`.
  - `v312_67_scalar_subquery_multiple_columns` — `SELECT (SELECT 1) AS a, (SELECT 2) AS b` → `(1, 2)`.
  - `v312_67_scalar_subquery_where_unaffected` — `WHERE val = (SELECT MAX(b) FROM u)` still works (#4629 regression check).
  - `v312_67_scalar_subquery_any_unaffected` — `WHERE val > ANY (subq)` still works (#4641 regression check).
- `cargo build --all-features`: clean.
- `cargo test -p sqlrustgo-cli --lib`: 73/73.
- `cargo test --test v312_67_scalar_subquery_in_select_test`: 5/5.
- `cargo clippy --all-features`: clean.
- Manual CLI repro: exact `printf "SELECT (SELECT 1) AS x;" | sqlrustgo-cli sqlite --batch --mode csv /tmp/db` returns `x\n---\n1`.
