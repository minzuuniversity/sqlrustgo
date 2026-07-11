# Spec — intra-query-parallel-group-by (Phase 3 placeholder)

> **Capability**: Parallel GROUP BY with partial aggregates + final merge.
> **Status**: DEFERRED to v3.10.0 Phase 3 (separate change)
> **Note**: This spec is a placeholder for the design contract; it will be promoted to a real change in Phase 3.

## ADDED Requirements

### Requirement: Parallel GROUP BY SHALL compute partial aggregates per partition

The system SHALL partition the GROUP BY input by hash of the group-by key, compute partial aggregates per partition in parallel, then merge partial results on the main thread.

#### Scenario: GROUP BY with N=4 workers
- **WHEN** a `SELECT a, SUM(b) FROM t GROUP BY a` is executed with `--executor-parallelism=4`
- **AND** the table has >= 100,000 rows with >= 1,000 distinct `a` values
- **THEN** the executor partitions rows by hash of `a` into 4 buckets
- **AND** each thread computes a partial `HashMap<a, PartialAggregate>`
- **AND** the main thread merges the 4 partial HashMaps into the final result
- **AND** SUM, COUNT, MIN, MAX are combined associatively
- **AND** AVG is computed from merged `sum` + `count` (correctness within 1e-9)

#### Scenario: Correctness equivalent to sequential
- **WHEN** the same GROUP BY query is run with `--executor-parallelism=1` and `--executor-parallelism=4`
- **THEN** the row count is identical
- **AND** aggregate scalar values are identical within 1e-9 floating-point tolerance
- **AND** for integer aggregates (SUM, COUNT), the match is bit-exact
