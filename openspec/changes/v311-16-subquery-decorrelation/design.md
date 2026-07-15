## Context

Existing correlated subquery execution in sqlrustgo uses **per-outer-row evaluation** (V311-15/17 indexed path). This is O(outer × inner_with_residual) — vastly better than naive O(outer × all_inner) but still **not optimal** because:

- The inner SELECT isn't materialized upfront
- Hash join algorithm can't be applied because the planner sees `Filter` not `Join`
- Cost-based join ordering can't decide which side is build/probe

V311-15 added `SubqueryIndex.key_to_rows: HashMap<Value, Vec<Vec<Value>>>`. V311-17 added `HashAntiJoin` bloom short-circuit. **Both are STILL per-outer-row drivers** — they cache but don't decorrelate.

The optimization that turns "subquery" into "join" is **decorrelation**. Once it's a join, the existing optimizer chain (predicate_pushdown, join_reorder, cbo cost model) can apply.

### Goal

Transform correlated subqueries in WHERE / SELECT into JOINs when the pattern permits, then let existing VolcExec + CBO handle it.

### Target Architecture

```
Before V311-16:
  LogicalPlan::Filter { predicate: Exists(subquery=...) }
    └── LogicalPlan::TableScan (orders)
    Execution:  for each orders row, evaluate subquery against orders row

After V311-16:
  LogicalPlan::Join { join_type: Semi, left: orders, right: InlineView(l_orderkey) }
    ├── LogicalPlan::TableScan (orders)
    └── LogicalPlan::InlineView {
         LogicalPlan::Distinct(Select(l_orderkey FROM lineitem WHERE <static>))
       }
    Execution:  hash join orders ⋉ lineitem_keys
```

For TPC-H Q17's `SELECT (SELECT 0.2*AVG(l_quantity) FROM lineitem WHERE l_partkey = p_partkey)`:
```
After V311-16:
  LogicalPlan::HashAggregate { 
    group_by: [p_partkey],
    aggregates: [0.2 * AVG(l_quantity)] 
  }
    └── LogicalPlan::Join { 
         left: part (with SELECT's other columns),
         right: lineitem 
         condition: l_partkey = p_partkey 
       }
```

## Goals / Non-Goals

**Goals:**
- Decorrelate simple WHERE EXISTS / NOT EXISTS / x IN (SELECT)
- Decorrelate correlated scalar subquery with single-column aggregate
- Skip optimization when pattern doesn't match (fall back to per-row execution)
- Verify no regression on existing V311-15/17/01 tests
- Demonstrate ≥2x speedup on targeted TPC-H queries

**Non-Goals:**
- Multi-correlation-column subqueries (defer)
- Recursive CTE inlining
- Aggregation decorrelation with multiple GROUP BYs at different levels

## Decisions

### Decision 1: Decorrelate BEFORE join reorder pass

**Choice**: Decorrelate first, then feed the rewrite into existing join_reorder. This ensures:
- Decorrelation never sees a join plan it doesn't understand
- Join reorder benefits from "lifted subquery" being a regular JOIN child

**Alternative considered**: Decorrelate lazily, after each join reorder step. Rejected — too complex, harder to test.

### Decision 2: Decoration limitations documented per pattern

**Choice**: Each of the 4 patterns has explicit "patterns we handle" + "patterns we skip" tables in `decorrelate.rs`. Unhandled patterns log at debug level and fall through.

**Why**: SQL subquery semantics are subtle (NULL handling, scope of outer refs). A conservative approach with explicit cases is robust.

### Decision 3: Use `LogicalPlan::Subquery` as the placeholder, then rewrite to `LogicalPlan::Join`

**Choice**: The logical plan already has `LogicalPlan::Subquery { subquery, .. }`. Use that as the input. Rewrite to a plain `LogicalPlan::Join`.

Looking at the existing `LogicalPlan` definition (`crates/planner/src/logical_plan.rs:62`):
```
Subquery { subquery: Box<LogicalPlan>, .. }
```

We'll preserve `Subquery` for `FROM (SELECT ...) AS alias` cases (CTE/inline view), but the WHERE-clause subquery gets promoted to a `Join`.

### Decision 4: Inline View via `LogicalPlan::Subquery` (no CTE materialization)

**Choice**: The lifted inner SELECT becomes a `LogicalPlan::Subquery` node that's a child of the new `Join`. We don't materialize as a CTE table — that would require V311-18 (CTE materialization).

**Why**: Keep V311-16 simple. Materialization optimizations come in V311-18.

### Decision 5: Scalar subquery becomes LEFT JOIN + GROUP BY

**Choice**: For `SELECT (SELECT AGG(col) FROM t WHERE x = outer.x) FROM outer`:
1. Rewrite outer into a `Join inner ON ...` (LEFT JOIN to keep outer rows)
2. Lift AGG into the outer query's projection as `GROUP BY outer.key, AGG(inner.col)`

Result: HashAggregate + Hash Join instead of per-row scalar eval.

**Alternative considered**: Mark as `LATERAL` join. Rejected — LATERAL semantics are subtly different (no explicit GROUP BY promotion), and our executor doesn't have a LATERAL operator.

## Implementation Plan

### Phase 1: Pattern Detection (decorrelate.rs, ~150 LoC)

```rust
pub enum SubqueryPattern {
    ExistsSemi(Box<Expression>),       // WHERE EXISTS (SELECT ...)
    NotExistsAnti(Box<Expression>),    // WHERE NOT EXISTS (SELECT ...)
    InToInnerJoin(Box<Expression>, Box<Expression>),  // WHERE x IN (SELECT y)
    ScalarAggGroupBy { inner: Box<Expression>, agg: Aggregate, on: Expression },
}

pub fn find_correlated_subqueries(
    where_expr: &Expression,
    select_exprs: &[Expression],
) -> Vec<(SubqueryLocation, SubqueryPattern)> {
    // Walk the WHERE expression tree, find Subquery/Exists/NotExists/In/NotIn/ScalarSubquery
    // For each, classify as one of the 4 patterns above.
    // Emit (location, pattern) pairs.
}
```

### Phase 2: Lifting Algorithm (~120 LoC)

```rust
pub fn try_decorrelate(plan: &mut LogicalPlan) -> bool {
    // For each WHERE-clause pattern in the plan:
    //   1. Lift the inner SELECT to a separate plan node (clone)
    //   2. Build a static filter (parts of WHERE referencing only inner columns)
    //   3. Add `DISTINCT` on the inner key columns (to avoid duplication)
    //   4. Build `LogicalPlan::Join` with appropriate join_type
    //   5. Update WHERE clause to remove the subquery reference
    // Returns true if anything was changed.
}
```

### Phase 3: Optimizer Chain Wiring (~30 LoC)

In `crates/optimizer/src/query_planner.rs::optimize()`:
```rust
let optimized = if crate::decorrelate::try_decorrelate(&mut plan) {
    plan  // plan is now a join, continue with existing passes
} else {
    plan
};
// then existing predicate_pushdown + join_reorder + cost_aware
```

### Phase 4: Tests (~150 LoC)

Create 8 tests in `tests/integration/optimizer/decorrelation_test.rs`:

1. `simple_exists_decorrelates` — basic EXISTS → Semi Join
2. `exists_with_residual_decorrelates` — EXISTS with non-equality clause → Filter + Semi Join
3. `not_exists_decorrelates` — NOT EXISTS → Anti Join
4. `in_subquery_decorrelates` — WHERE x IN (SELECT y) → Inner Join
5. `correlated_scalar_subquery_decorrelates` — `SELECT (SELECT AVG ... WHERE ...)` → Join + Group By
6. `non_correlated_subquery_unchanged` — Plain FROM (SELECT) unaffected
7. `subquery_with_or_predicate_passthrough` — Conservative: OR predicates pass through
8. `tpc_h_q2_full_decorrelation` — Q2-shaped query decorrelates successfully

### Phase 5: Verify + Bench (~30 LoC)

Run all prior tests (V311-15/17/01) for no-regression, plus TPC-H Q2/Q17 mini-benchmarks.

## Risks & Mitigations

| Risk | Severity | Mitigation |
|------|----------|------------|
| Decorrelated row counts differ from per-row path | High | Test with multiple fixtures; assert exact row counts |
| NULL semantics differ (EXISTS vs JOIN) | Medium | NOT IN contains NULL → "no match" (NOT EXISTS true) — handle explicitly |
| Multiple correlation columns | Medium | Skip (return false from `try_decorrelate`); existing per-row path applies |
| Inner SELECT has ORDER BY/LIMIT | Medium | Skip; not in scope |
| Aggregate in inner SELECT | Medium | Use `ScalarAggGroupBy` rewrite pattern; preserve GROUP BY semantics |

## Verification Strategy

- Functional: 8 new tests in `decorrelation_test`
- Regression: 25 prior tests still PASS
- Performance: V311-16 + V311-15/17 benchmarks together show ≥6x on Q2/Q17
