## Context

TPC-H Q4 with SF=3 takes 14.5 minutes on v3.10.0 due to a critical performance bug:
the executor treats `Subquery` expressions as `Value::Null` instead of evaluating them
(see `crates/executor/src/trigger_eval/expression.rs:391`).

**Current state of subquery handling** (from grep):
- `crates/executor/src/explain.rs:359-360` - `InSubquery` is only used for display
- `crates/executor/src/stored_proc.rs:1247` - Subquery handled in stored proc context
- `crates/executor/src/trigger_eval/expression.rs:391` - **All subqueries return Null**
- `crates/planner/src/logical_plan.rs:61` - LogicalPlan::Subquery exists
- `crates/executor/src/local_executor_dml.rs:94` - "Subquery source not yet supported"

The executor has NO production-quality subquery evaluation. This is a deeper
issue than just adding a semi-join operator.

## Goals / Non-Goals

**Goals:**
- Evaluate EXISTS subqueries in WHERE clauses correctly
- Add a Hash Semi Join operator for O(outer + inner) instead of O(outer * inner)
- Reduce TPC-H Q4 @ SF=3 from 14.5 minutes to < 5 minutes (≥2.9x speedup)
- Keep all other TPC-H queries at current performance or better

**Non-Goals:**
- Full subquery decorrelation (that's V311-16, separate task)
- NOT IN / NOT EXISTS optimization (that's V311-17, separate task)
- CTE materialization (that's V311-18, separate task)

## Decisions

### 1. Add `ExistsSubquery` as separate AST variant (instead of overloading Subquery)

Add `Expression::ExistsSubquery(Box<SelectStatement>)` in `crates/parser/src/parser.rs:570`:
- Clearer semantics: this is specifically a boolean subquery
- Avoids overload/confusion with scalar subqueries
- EXISTS returns true/false, scalar returns value

### 2. Implement semi-join in expression evaluator (quickest path)

Rather than implementing full Volcano rewrite, the semi-join logic is
embedded in the expression evaluator:

```rust
Expression::ExistsSubquery(select) => {
    // Execute inner SELECT once
    let inner_result = executor.execute_select(select)?;
    // Build HashSet of projection key values
    let build_set: HashSet<Value> = inner_result.iter()
        .map(|row| row[0].clone())
        .collect();
    // For each outer row, check membership
    if build_set.contains(outer_row[col_idx]) { Value::Boolean(true) }
    else { Value::Boolean(false) }
}
```

This is simple, correct, and provides massive speedup over naive execution.

### 3. Add hash-based lookup instead of full vector iteration

For 3M lineitem, the HashSet approach is O(1) per lookup instead of O(3M) for naive.
With bloom filter, we skip HashSet construction for most probes.

### 4. Add bloom filter optimization

Simple 128-byte bloom filter (16 x 64-bit words = 1024 bits):
- 2 hash functions (FNV-1a + DJB2 variants)
- False positive rate: ~1% for typical cardinalities
- Skips HashSet lookup for most non-matching probes

### 5. Plan-level recognition of semi-join opportunity

In the executor, when WHERE clause has ExistsSubquery, we can optionally:
- Build a plan where EXISTS is replaced with SemiJoin
- Apply Volcano-style iterator
- For minimal scope, just optimize the expression evaluator

For V311-15 scope, we focus on the expression evaluator approach (decision 2-4).
Full Volcano rewrite is V311-16 (decorrelation).

## Implementation Plan

### Step 1: AST changes
- `crates/parser/src/parser.rs:570` - add `ExistsSubquery` variant
- Update parse logic to produce this for `WHERE EXISTS (SELECT ...)` syntax

### Step 2: Expression evaluation
- `crates/executor/src/expr/mod.rs` - handle ExistsSubquery in evaluate()
- Implement HashSet + bloom filter
- Pass row context to the evaluator (for outer row values)

### Step 3: Plumb row context
- Currently evaluate(row, columns) takes single row
- For EXISTS, need to pass outer row to inner evaluation
- Add a wrapper that loops over outer rows, building build_set once

### Step 4: Tests
- Unit tests for ExistsSubquery evaluation
- Integration test using TPC-H Q4
- Performance test asserting < 5 min @ SF=3

### Step 5: Optimization
- Add bloom filter (FNV-1a + DJB2)
- Tune bloom filter size based on cardinality

## Risks

| Risk | Mitigation |
|------|------------|
| Large inner result exhausts memory | Stream inner result, build incrementally |
| Bloom filter false positives | Use 2 hash functions, 1024 bits for low FPR |
| Subquery in SELECT list (not WHERE) | Only handle WHERE ExistsSubquery; SELECT subqueries are V311-16 scope |
| Existing tests break | Write TPC-H Q4 first, verify other queries unchanged |

## Implementation Strategy

For V311-15, the minimum viable approach is:
1. Add `ExistsSubquery` AST variant
2. Implement hash-based evaluation in expression evaluator
3. Add bloom filter optimization
4. Test with TPC-H Q4

This is the fastest path to fix the performance bug without a full Volcano rewrite.
