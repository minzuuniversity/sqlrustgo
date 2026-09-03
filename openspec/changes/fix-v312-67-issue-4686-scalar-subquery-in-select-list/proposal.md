## Why

`sqlrustgo-cli sqlite` (HEAD `d18482e24`, develop/v3.12.0) silently returns empty for scalar subqueries in the SELECT list:

```
$ printf "SELECT (SELECT 1) AS x;" | sqlrustgo-cli sqlite --batch --mode csv /tmp/db
x
---
  
```

Expected:
```
x
---
1
```

The output row is the empty input row `vec![]` (zero columns) — the projected scalar subquery's value never reaches the output. Issue #4686 reports this and notes it shares a root cause with #4629 (scalar subqueries in WHERE).

Root cause: the projection loop at `src/engine_select.rs:1826` passes a `subq_eval` closure that always returns `Value::Null`:
```rust
&|_| Ok(Value::Null),
```

The intent was conservative — the WHERE pre-evaluator (line 939) has the real subquery execution closure, but projection does not. The fix threads `self` (the engine) into the projection loop so the Subquery arm can call `self.execute_select(subq)`, then take the first row's first column as the scalar value.

A naive implementation of this (recursing `self.execute_select` from within the projection) hangs because the inner `execute_select` call goes through the full pipeline including pre-evaluators, decorrelators, and other expensive passes — and the call from the projection `evaluate_expression_with_seq` re-enters these passes. The fix uses a `RecursionGuard` pattern (thread-local) to skip the heavy pre-evaluation passes when called recursively from projection.

## What Changes

- Add a `std::cell::RefCell<Option<()>>`-based recursion guard (or a simple boolean field on `ExecutionEngine` since `execute_select` takes `&self`) so the recursive call from projection skips the prewarm / decorrelate / Step 1.5 / Step 1.6 passes — they are unnecessary for a leaf scalar subquery like `SELECT 1`.
- Update the projection loop in `src/engine_select.rs` to pass a real `subq_eval` closure that calls `self.execute_select(subq)` and returns the first row's first column (or `Value::Null` on empty).
- Add a `Expression::Subquery` arm to `evaluate_expression_with_seq` (already exists at `src/expr_utils.rs:428` but takes the closure) — keep the closure-only approach; the fix is in the caller.

No public API change. No storage/WAL change.

## Capabilities

### New Capabilities

- `executor-scalar-subquery-in-select-list`: `sqlrustgo` MUST evaluate `SELECT (subq) AS alias` correctly. The scalar subquery's first column of its first row becomes the cell value; an empty subquery result yields `NULL`.

### Modified Capabilities

- None. `WHERE val = (subq)` (issue #4629) and `WHERE val > ANY (subq)` (issue #4641) are unaffected.
