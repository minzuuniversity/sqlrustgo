## Why

V311-16 is the **third axis** of v3.11.0 subquery optimization (following V311-15 Hash Semi Join and V311-17 Hash Anti Join). It tackles **decorrelation** — the systematic transformation of correlated subqueries into JOINs so the optimizer's existing join reorder / cost-based optimizer can do its work.

### What's Already Done (V311-15/17)

| Operator | Pattern | Status |
|----------|---------|--------|
| Hash Semi Join | `WHERE EXISTS (SELECT ...)` | ✅ V311-15 |
| Hash Anti Join | `WHERE NOT EXISTS (SELECT ...)` | ✅ V311-17 |
| Scalar Subquery Fast-Path | `SELECT (SELECT AGG(...))` | ✅ existing (Q17 perf) |
| NOT IN Subquery Pre-Eval | `WHERE x NOT IN (SELECT ...)` | ✅ existing |

### What's Missing (V311-16)

The above paths run **per outer row** — they short-circuit EXISTS but don't fully **decorrelate**. A truly decorrelated query:

1. **Lifts** the inner SELECT into a derived table (CTE or inline view)
2. **Replaces** the subquery reference with a JOIN to that derived table
3. **Allows** join reorder / hash join selection from existing optimizer passes
4. **Avoids** per-outer-row execution entirely

#### Concrete Examples

```sql
-- Before V311-16 (V311-15 Naive or V311-15 Indexed):
SELECT o.* FROM orders o WHERE EXISTS (SELECT 1 FROM lineitem l WHERE l_orderkey = o_orderkey);
-- The inner SELECT runs once per outer row (or bucket-walked per outer).

-- After V311-16:
SELECT o.* FROM orders o 
JOIN (SELECT DISTINCT l_orderkey FROM lineitem WHERE <static filters>) l 
   ON l.l_orderkey = o.o_orderkey;
-- Single scan + hash join, O(N_inner + N_outer) total work.
```

For TPC-H Q2 (subquery with GROUP BY), Q17 (correlated scalar aggregate), Q20 (IN with subquery), this transformation reduces from O(outer × subquery_cost) to O(outer + subquery_cost).

### Real-World Impact

| Query | v3.11.0 baseline | V311-16 target |
|-------|------------------|----------------|
| TPC-H Q2 (subquery with MIN) | ~30s | < 5s |
| TPC-H Q17 (correlated scalar) | ~3s | < 1s |
| TPC-H Q20 (IN subquery) | moderate | speedup ≥ 4x |
| General correlated subqueries | O(N²) | O(N + subquery) |

## What Changes

### 1. Pattern Detector

- New module `crates/optimizer/src/decorrelate.rs`
- Function `find_correlated_subqueries(expr: &Expression) -> Vec<SubqueryPattern>`
- Recognizes 4 patterns:
  - `WHERE EXISTS (SELECT ...)` → Semi Join candidate
  - `WHERE NOT EXISTS (SELECT ...)` → Anti Join candidate
  - `WHERE x IN (SELECT y FROM ...)` → Inner Join candidate with distinct
  - `SELECT (SELECT AGG(col) FROM t WHERE t.x = outer.x)` → Lateral / Group-By candidate

### 2. Decorrelation Pass

- `pub fn try_decorrelate_plan(plan: LogicalPlan) -> LogicalPlan`
- For each detected pattern, attempt to:
  - **Extract** the inner SELECT body
  - **Identify** correlation columns (the columns referenced from outer)
  - **LIFT** static filters (no outer refs) into a filter on the derived view
  - **Build** an Inline View (Unnest/SubqueryAlias)
  - **REWRITE** as a JOIN (Semi / Anti / Inner / Group)

### 3. Integration with Existing Optimizer Chain

- `crates/optimizer/src/query_planner.rs::optimize()` calls `try_decorrelate_plan()`
- Output fed into existing predicate_pushdown + join_reorder passes
- Decorrelated plan is then physicalized via existing VolcExec

### 4. Tests (`tests/integration/optimizer/decorrelation_test.rs`)

- 8 tests:
  - simple_exists_decorrelates
  - exists_with_residual_decorrelates
  - not_exists_decorrelates
  - in_subquery_decorrelates
  - correlated_scalar_subquery_decorrelates
  - non_correlated_subquery_unchanged
  - subquery_with_or_predicate_decorrelates_partially
  - tpc_h_q2_shape_decorrelation

### 5. Fallback

If pattern doesn't match (e.g., subquery references outer in multiple places, uses aggregates, etc.), optimizer passes through unchanged. Original per-row execution (V311-15/17 indexed path or naive) applies.

## Capabilities

### New Capabilities

- `subquery-decorrelation`: optimizer pass that transforms correlated subqueries in WHERE/SELECT/IN to JOINs
- `decorrelated-exists-as-semi-join`: EXISTS subquery with no aggregate → Semi Join
- `decorrelated-not-exists-as-anti-join`: NOT EXISTS subquery with no aggregate → Anti Join  
- `decorrelated-correlated-scalar-as-group-by`: scalar correlated subquery → Group By Join

## Impact

### Affected Files (created/modified)

| File | Type | Lines |
|------|------|-------|
| `crates/optimizer/src/decorrelate.rs` | new | ~300 |
| `crates/optimizer/src/lib.rs` | modified | +5 (export) |
| `crates/optimizer/src/query_planner.rs` | modified | +10 (call site) |
| `tests/integration/optimizer/decorrelation_test.rs` | new | ~200 |
| `docs/releases/v3.11.0/perf/SUBQUERY_DECORRELATION.md` | new | ~80 |

### No Breaking Changes

- All V311-15/17/01 tests continue passing
- Existing correlated subquery execution (per-row) remains as fallback
- Decorrelated plans only KICK IN when pattern matches

## Acceptance Criteria

- **`cargo test decorrelation_test`**: 8/8 PASS
- **`cargo test q4_hash_semi_join_test`**: 4/4 PASS (no regression on V311-15)
- **`cargo test anti_join_main_path_test`**: 5/5 PASS (no regression on V311-17)
- **`cargo test q21_exists_hash_path_test`**: 2/2 PASS (no regression on V311-15)
- TPC-H Q2 with SF=0.1: < 5 seconds (baseline ~30s)
- `docs/releases/v3.11.0/perf/SUBQUERY_DECORRELATION.md` documents:
  - Pattern detection thresholds
  - Benchmarked speedups
  - Plan-shape examples

## Estimated Effort

| Step | Estimate | Complexity |
|------|----------|------------|
| Pattern detector | 8h | Medium |
| Decorrelation pass (lifter) | 16h | High (corner cases for aggregates, joins in subq) |
| Optimizer chain integration | 4h | Low |
| Tests | 8h | Medium |
| Documentation + benchmarks | 4h | Low |
| **Total** | **40h** (compressed to ~10h with V311-15/17 infra reuse) |

## Risk Assessment

| Risk | Severity | Mitigation |
|------|----------|------------|
| Decorrelated plan differs from per-row execution semantics | High | Strong test suite with same fixture data; assert same row counts |
| Subquery references outer columns multiple times | Medium | Only decorrelate if single correlation column |
| Aggregates inside correlated subquery (Q17) | Medium | Use GROUP BY JOIN rewrite; preserve SEMANTICS |
| Optimizer chain ordering: decorrelate BEFORE or AFTER join reorder? | Medium | Decorrelate BEFORE (pulls subq up, then reorder joins inner-outer) |

## Scope

### IN SCOPE (V311-16)

- WHERE EXISTS / NOT EXISTS simple decorrelation
- WHERE x IN (SELECT ... FROM single_table WHERE simple = outer.x)
- SELECT (SELECT AGG(col) FROM t WHERE ... = outer.x) for Q17-style

### OUT OF SCOPE (deferred)

- Correlated subqueries with multiple correlation columns (e.g., t.x = outer.x AND t.y = outer.y)  
- Subqueries with TOP/ORDER BY
- Subqueries with window functions
- Recursive CTE decorrelation

These get logged by optimizer as "not decorrelated" and fall through to existing per-row execution.

## Out-of-Scope Items (Design Constraint)

Stays consistent with V311-15/17/01: disk persistence, WAL, MVCC remain v3.12+ scope.
