# EXEC-03 Aggregates Resolution (RC1)

**Issue**: #2969
**Status**: RESOLVED for RC1 / Frozen for v3.9.0+
**Date**: 2026-06-04

## Summary

EXEC-03 requested 4 advanced aggregate functions (StdDev, Variance, Median,
GroupConcat) to be added on top of the standard 5 (Count, Sum, Avg, Min, Max).

## RC1 Resolution

The **5 standard aggregates** (Count/Sum/Avg/Min/Max) **are** already
implemented and working through the canonical `ExecutionEngine::execute`
path. Verified by `tests/aggregate_smoke_test.rs` (3 tests, all passing):

- `aggregate_5_basics` — Count, Sum, Avg, Min, Max on simple table
- `aggregate_group_by` — Group By with Sum + Count
- `aggregate_having` — Having clause with Sum threshold

The 4 advanced aggregates (StdDev/Variance/Median/GroupConcat) **are**
implemented in `crates/executor/src/local_executor.rs` and
`crates/executor/src/parallel_executor.rs` — but those files are
**not declared in `crates/executor/src/lib.rs`** and are therefore
**not compiled** (dead code).

## Decision

Per user directive 2026-06-04 (Route B: v3.8.0 → GA without v3.9.0):

1. **Accept the 5 standard aggregates as fulfilling v3.8.0 GA scope**
   (RC1 baseline test: 3/3 pass)
2. **Freeze the 4 advanced aggregates + mod-tree activation to v3.9.0+**
   (the work is non-trivial and orthogonal to v3.8.0 GA stability)
3. **Document the freeze rationale** in
   `docs/releases/v3.8.0/V380_FROZEN_TO_V390.md` (already listed)

## Verification

```
cargo test --test aggregate_smoke_test
  aggregate_5_basics   ok
  aggregate_group_by   ok
  aggregate_having     ok
  3 passed; 0 failed
```

## Reference

- Issue: #2969 (EXEC-03)
- Related: V380_FROZEN_TO_V390.md (frozen items list)
- Tests: `tests/aggregate_smoke_test.rs`
