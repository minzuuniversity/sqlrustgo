# Spec — parallel-perf-fix (Issue #3776 / F-36)

> **Capability**: Eliminate parallel-execution throughput regression.
> **Status**: Proposed
> **Default state**: `parallel_degree=1` (sequential, zero regression by spec)

## ADDED Requirements

### Requirement: No regression at small dataset sizes

The system SHALL NOT introduce >10% throughput regression on small datasets
(rows < 50,000) when `parallel_degree > 1`.

#### Scenario: 1K rows with N=4 partitions

- **WHEN** `parallel_scan(table, 4)` is called with `len(rows) = 1_000`
- **THEN** the wall-clock time is within ±10% of `scan(table)` (sequential)
- **AND** peak memory allocation is within 1.5× of `scan(table)`

#### Scenario: 10K rows with N=4 partitions

- **WHEN** `parallel_scan(table, 4)` with `len(rows) = 10_000`
- **THEN** wall-clock time is within ±15% of `scan(table)`
- **AND** the implementation does NOT clone the underlying Vec per partition
  (verified via RefCount check or `Arc::strong_count`)

#### Scenario: Default parallel_degree=1 unchanged

- **WHEN** `parallel_degree=1` and `len(rows) = 1_000_000`
- **THEN** the implementation uses the same code path as before this fix
- **AND** the existing INT-2 tests still pass with bit-exact results

### Requirement: CBO-guided guard activation

The system SHALL replace the hard `rows.len() >= PARALLEL_MIN_ROWS = 500_000`
threshold with a CBO-based decision in `engine_select.rs::execute_select`.

#### Scenario: Below 50K rows always sequential

- **WHEN** `parallel_degree=4` AND `len(rows) = 30_000`
- **THEN** the executor uses the sequential filter path
- **AND** no `parallel_filter_engaged` tracing span is emitted

#### Scenario: Above 50K with high selectivity (selective filter)

- **WHEN** `parallel_degree=4` AND `len(rows) = 500_000` AND filter selectivity = 0.1
- **THEN** the executor uses the parallel filter path
- **AND** the parallel filter tracing span is emitted

#### Scenario: FOR UPDATE bypasses CBO entirely

- **WHEN** `parallel_degree=4` AND `FOR UPDATE` is present
- **THEN** CBO is NOT consulted
- **AND** sequential path is used (lock deadlock prevention)

#### Scenario: CBO returns false on unknown stats

- **WHEN** `parallel_degree=4` AND table has no row count stats
- **THEN** CBO returns `false` (conservative)
- **AND** sequential path is used

### Requirement: No memory regression on parallel_scan

The system SHALL NOT increase peak memory allocation when partitioning a
relation into N chunks. Memory should scale with N×Arc-overhead rather than
N×data-size.

#### Scenario: 1M rows with N=4 partitions

- **WHEN** `parallel_scan(table, 4)` with `len(rows) = 1_000_000`
- **THEN** peak Vec allocation is **at most** 1× the source data (1 Arc bump)
- **AND** each partition iterator references the shared Arc
- **AND** dropping one partition does NOT cascade-drop the others

#### Scenario: Verify shared Arc via Arc::strong_count

- **WHEN** a test calls `parallel_scan(table, 4)` and inspects the iterators
- **THEN** `Arc::strong_count(&shared_data)` == `num_partitions + 1` (one per
  partition + the source)

### Requirement: Performance correctness validation

The system SHALL provide automated tests that prevent regressions.

#### Scenario: Regression test at 1K rows

- **WHEN** running `tests/parallel_scan_perf_baseline.rs`
- **AND** `len(rows) = 1_000`
- **THEN** the test asserts sequential and parallel(S=4) are within 10%
  of each other (wall-clock + memory)

#### Scenario: Parallel speedup at 1M rows

- **WHEN** `len(rows) = 1_000_000`
- **THEN** parallel(S=8) median time is ≤ 67% of sequential (1.5x speedup)
- **AND** the test runs in a tight loop and takes the median of 5 runs

### Requirement: Backward compatibility

The fix SHALL be backward compatible with all existing behavior.

#### Scenario: Existing INT-2 tests still pass

- **WHEN** running `cargo test --test int2_substance_parallel_test --features parallel-executor`
- **THEN** all 9 INT-2 tests pass
- **AND** cell-level equivalence is bit-exact

#### Scenario: Existing parallel_semantic tests still pass

- **WHEN** running `cargo test --test parallel_semantic_tests --features parallel-executor`
- **THEN** all 17 tests pass
- **AND** NULL semantics, edge cases, and activation logic unchanged

#### Scenario: Default cargo build still works

- **WHEN** running `cargo check` (no features)
- **THEN** the executor lib compiles without errors
- **AND** `task_scheduler` stub continues to provide sequential fallback

## Refs

- Issue #3776 (V310-19, F-36)
- `tests/parallel_scan_bench_test.rs` (baseline measurements)
- `crates/executor/src/parallel_group_by.rs` (similar Arc-sharing approach)
- `crates/optimizer/src/unified_cost.rs` (CBO `should_parallelize_with` API)
- `src/engine_select.rs:262` (current guard)
- `crates/storage/src/engine.rs:1074` (current `parallel_scan`)
