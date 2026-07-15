## Why

V311-15 is the **CORE performance bottleneck** identified during v3.10.0 168h SOAK (Issue #3792). TPC-H Q4 with SF=3 takes **14.5 minutes** on v3.10.0 because:

- Q4 has `WHERE EXISTS (SELECT * FROM lineitem l2 WHERE l2.l_orderkey = o_orderkey AND l2.l_suppkey <> o.o.o_suppkey AND l2.l_receiptdate > l2.l_commitdate)`
- Current executor treats subqueries as Value::Null (see `crates/executor/src/trigger_eval/expression.rs:391`)
- Result: Q4 becomes a degenerate query that returns 0 rows OR runs naive nested-loop
- Naive nested-loop: 450K orders × 3M lineitem = 1.35 trillion comparisons

The fix is to:
1. Properly evaluate Subquery expressions in WHERE clauses
2. Detect the `EXISTS` pattern and route to a Hash Semi Join operator
3. Build a HashTable once over the probe side, then for each outer row check if any match exists

## What Changes

- **Add proper Subquery evaluation to Expression** (not just Value::Null)
  - In `crates/executor/src/expr/mod.rs`: add `Subquery(Box<SelectStatement>)` variant
  - In `expression_to_value`: execute the subquery, return scalar value (for scalar subquery) or boolean (for EXISTS)
- **Add `ExistsSubquery` AST variant** (or repurpose Subquery with `EXISTS` flag)
  - Better: add `Expression::ExistsSubquery(Box<SelectStatement>)` as separate variant
- **Add `HashSemiJoin` executor** in `crates/executor/src/join/`
  - `pub struct HashSemiJoin { build_keys: Vec<usize>, build_side: ..., probe_side: ... }`
  - `impl HashSemiJoin { pub fn execute(&self) -> ... }`
- **Add bloom filter optimization** for probe phase
  - Build bloom filter from build_keys during build
  - Check bloom filter in probe to avoid full HashTable lookup for impossible matches
- **Add Volcano operator integration** in `crates/executor/src/executor.rs`
  - Recognize `WHERE EXISTS (SELECT ...)` pattern
  - Route to HashSemiJoin operator
  - Fallback to naive nested-loop for non-EXISTS subqueries (or future refactor)
- **Add planner rule** to detect semi-joinable patterns
  - In `crates/planner/src/optimizer.rs`: rule to push EXISTS into semi-join
  - Detect: `outer WHERE EXISTS (SELECT cols FROM inner WHERE inner.col = outer.col)`
- **Add regression test** in `tests/stress/q4_semi_join_test.rs`
  - 100K+ lineitem + 10K orders
  - Measure Q4 execution time
  - Assert: < 30 seconds (vs current 14.5 minutes for 3M+)

## Capabilities

### New Capabilities

- `hash-semi-join`: New HashSemiJoin operator that processes semi-join with O(build + probe) complexity instead of O(outer * inner)
- `exists-subquery-eval`: Subquery expressions (especially EXISTS) are properly evaluated instead of returning Null

## Impact

- **Modified**: `crates/executor/src/expr/mod.rs` - add Subquery/ExistsSubquery evaluation
- **Modified**: `crates/parser/src/parser.rs` - add ExistsSubquery AST variant
- **New**: `crates/executor/src/join/hash_semi_join.rs` - new operator
- **Modified**: `crates/executor/src/executor.rs` - route EXISTS pattern to semi-join
- **Modified**: `crates/planner/src/optimizer.rs` - add semi-join rule
- **New**: `tests/stress/q4_semi_join_test.rs` - regression test
- **New**: `docs/releases/v3.11.0/perf/SEMI_JOIN_PERF.md` - performance report
- **No new dependencies** - uses existing HashMap-based join infrastructure

## Acceptance Criteria

- TPC-H Q4 @ SF=3: **< 5 minutes** (v3.10.0 = 14.5 minutes, speedup >= 2.9x)
- `cargo test q4_semi_join_test` PASS (10/10)
- TPC-H Q1, Q3, Q5, Q6, Q12, Q14 performance NOT regressed
- 100K+ row TPC-H test dataset works (already in fixtures)
- Bloom filter effectiveness: 80%+ of probes filtered out for non-matching keys

## Implementation Notes

### Subquery evaluation approach

The simplest approach (for minimal scope):
- Keep current Subquery returning Value::Null
- ADD a separate ExistsSubquery variant that:
  - Executes the inner SELECT once
  - Stores the hash table of the projection key
  - For each outer row, checks if any inner row matches
  - This is effectively a manual semi-join in the expression evaluator

This avoids the complex Volcano rewrite while still getting the perf benefit.

### Bloom filter implementation

Simple FNV-1a hash + bit array of size 1024:
```rust
struct BloomFilter { bits: [u64; 16] }  // 1024 bits = 128 bytes
impl BloomFilter {
    fn add(&mut self, key: &Value) { /* 2 hash functions */ }
    fn might_contain(&self, key: &Value) -> bool { /* check both bits */ }
}
```
