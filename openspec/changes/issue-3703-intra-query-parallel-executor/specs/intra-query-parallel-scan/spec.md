# Spec — intra-query-parallel-scan

> **Capability**: Single-query scan-stage parallelism via partition-based work distribution.
> **Status**: Proposed (Issue #3703 Phase 1)
> **Default state**: `--executor-parallelism=1` (sequential, zero-regression)

## ADDED Requirements

### Requirement: Parallel scan activation

The system SHALL activate parallel scan only when ALL of the following hold:
- `--executor-parallelism=N` is set with N > 1
- The binary is compiled with `--features parallel-executor`
- The SELECT statement has no `ORDER BY` clause (ordering preserved by sequential path)
- The base table row count is `>= PARALLEL_MIN_ROWS` (100,000 rows, defined in `parallel_executor.rs`)

#### Scenario: Default sequential behavior
- **WHEN** `--executor-parallelism=1` (or unset, default)
- **THEN** the executor runs the existing single-threaded `execute_select` path
- **AND** all 25 lib tests + 22/22 TPC-H tests pass with bit-exact results

#### Scenario: Opt-in parallel with sufficient rows
- **WHEN** `--executor-parallelism=4` AND table has 200,000 rows
- **THEN** the executor partitions the row stream into 4 chunks
- **AND** runs Filter → Project → Agg on each chunk in parallel via `rayon::par_iter()`
- **AND** merges partial results on the main thread

#### Scenario: Below threshold falls back
- **WHEN** `--executor-parallelism=4` AND table has 500 rows
- **THEN** the executor falls back to sequential `execute_select` (rayon overhead exceeds benefit)
- **AND** no error is raised

### Requirement: Partition algorithm

The system SHALL partition the input row stream evenly across N workers using the existing `ParallelVolcanoExecutor::partition_scan` algorithm:
- `base_rows = total_rows / N`
- `remainder = total_rows % N`
- First `remainder` partitions receive `base_rows + 1` rows
- Remaining partitions receive `base_rows` rows
- Empty partitions are excluded from the parallel pipeline (avoid `par_iter` of empty slices)

#### Scenario: Even partition
- **WHEN** total_rows = 1000, N = 4
- **THEN** partitions are [250, 250, 250, 250]

#### Scenario: Uneven partition with remainder
- **WHEN** total_rows = 1003, N = 4
- **THEN** partitions are [251, 251, 251, 250] (remainder = 3 distributed to first 3 partitions)

#### Scenario: N exceeds row count
- **WHEN** total_rows = 3, N = 8
- **THEN** partitions are [1, 1, 1] (only 3 non-empty partitions; 5 empty partitions skipped)

### Requirement: Result correctness (cell-level equivalence)

The system SHALL produce cell-level equivalent results between sequential and parallel execution paths.

#### Scenario: N=1 vs N=4 cell-level match
- **WHEN** the same SELECT query (no ORDER BY) is executed with `--executor-parallelism=1` and `--executor-parallelism=4`
- **THEN** the row count is identical
- **AND** the multiset of (column_values) is identical
- **AND** aggregate scalar values (SUM, COUNT, AVG) are identical within floating-point tolerance (1e-9)

#### Scenario: TPC-H 22/22 consistency
- **WHEN** running the full TPC-H SF=0.1 suite with `--executor-parallelism=4`
- **THEN** all 22 queries produce results matching the N=1 baseline
- **AND** the cell_diff tool reports 0 differences

### Requirement: ORDER BY falls back to sequential

The system SHALL NOT activate parallel scan when the SELECT statement contains an `ORDER BY` clause, to preserve row ordering.

#### Scenario: ORDER BY triggers sequential fallback
- **WHEN** query is `SELECT * FROM t WHERE x > 0 ORDER BY y`
- **AND** `--executor-parallelism=4` is set
- **THEN** the executor uses sequential `execute_select`
- **AND** rows are returned in `ORDER BY y` order
- **AND** a log message is emitted: `"parallel scan skipped: ORDER BY present, falling back to sequential"`

### Requirement: Storage thread safety

The system SHALL rely on the G13-OLTP-2 fix (PR #3680) for `parking_lot::RwLock` fair-FIFO read access. N concurrent workers calling `storage.scan()` SHALL NOT deadlock or starve.

#### Scenario: N=8 concurrent scans
- **WHEN** `--executor-parallelism=8` on a table with 1M rows
- **THEN** all 8 workers successfully acquire read locks via `storage_read()`
- **AND** no worker waits more than 100ms (fair FIFO order)
- **AND** no deadlock or starvation occurs over 1000 iterations

### Requirement: Performance non-regression

The system SHALL NOT introduce >5% throughput regression on OLTP workloads (small queries, high concurrency).

#### Scenario: OLTP mixed RW workload
- **WHEN** running sysbench oltp_read_write with 16 client threads
- **AND** `--executor-parallelism=4`
- **THEN** QPS is within ±5% of `--executor-parallelism=1` baseline
- **AND** P99 latency is within ±10% of baseline

#### Scenario: Large query speedup target
- **WHEN** running TPC-H Q1 on SF=0.1 with `--executor-parallelism=8`
- **THEN** wall-clock time is at least 2x faster than `--executor-parallelism=1` (target ≥ 2x speedup)
- **AND** speedup is measured across 10 runs, taking median

### Requirement: Feature gate (compilation)

The parallel scan code path SHALL be gated behind the `parallel-executor` cargo feature.

#### Scenario: Default build has no parallel code
- **WHEN** building with `cargo build --release` (no features)
- **THEN** `parallel-executor` feature is OFF
- **AND** `execute_select_parallel` is not compiled
- **AND** the binary size is identical to v3.10.0-alpha1 baseline

#### Scenario: Feature-enabled build
- **WHEN** building with `cargo build --release --features parallel-executor`
- **THEN** `parallel-executor` feature is ON
- **AND** rayon + parallel code is included
- **AND** `--executor-parallelism=N` with N>1 is functional
