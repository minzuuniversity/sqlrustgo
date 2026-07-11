# Spec — intra-query-parallel-hash-join (Phase 2 placeholder)

> **Capability**: Parallel hash join with build-side partitioning.
> **Status**: DEFERRED to v3.10.0 Phase 2 (separate change)
> **Note**: This spec is a placeholder for the design contract; it will be promoted to a real change in Phase 2.

## ADDED Requirements

### Requirement: Parallel hash join SHALL partition build side across N workers

The system SHALL partition the hash join build side across N workers using hash partitioning of the join key, when intra-query parallelism is enabled and the build side is large enough.

#### Scenario: Hash join with N=4 workers
- **WHEN** a SELECT with `INNER JOIN` is executed with `--executor-parallelism=4`
- **AND** the build side has >= 100,000 rows
- **THEN** the executor partitions the build side into 4 hash buckets
- **AND** each thread builds a partial hash table
- **AND** the partial hash tables are merged on the main thread
- **AND** the probe side fans out to match partitions

#### Scenario: Correctness equivalent to sequential
- **WHEN** the same JOIN query is run with `--executor-parallelism=1` and `--executor-parallelism=4`
- **THEN** the row count and column values are identical
- **AND** the multiset of (left_key, right_values) tuples is identical
