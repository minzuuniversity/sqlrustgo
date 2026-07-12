# Spec — intra-query-parallel-main-path

> **Capability**: Production main-path integration of intra-query parallel scan
> via the existing guard in `engine_select.rs::execute_select`.
> **Status**: Proposed (Issue #3767, V310-13 / I-12)

## ADDED Requirements

### Requirement: Module declaration

The system SHALL export `parallel_executor` as a public module from the
executor crate so that production binaries can link against it.

#### Scenario: lib.rs declares parallel_executor

- **WHEN** building `sqlrustgo-executor` (default features)
- **THEN** `crates/executor/src/lib.rs` contains `pub mod parallel_executor;`
- **AND** `ParallelVolcanoExecutor` is reachable from the `sqlrustgo_executor`
  public API
- **AND** the binary builds without errors

### Requirement: Parallel filter guard conditions

The parallel filter guard at `engine_select.rs::execute_select` SHALL
activate ONLY when ALL of the following hold:
- `parallel_degree > 1` (set by `--executor-parallelism=N` or env var)
- `rows.len() >= PARALLEL_MIN_ROWS` (500,000 by default)
- The WHERE clause contains no correlated subqueries
- The SELECT has no `FOR UPDATE` / `LOCK IN SHARE MODE`

#### Scenario: All conditions met

- **WHEN** `parallel_degree=4` AND 1M rows AND no correlated subquery AND no FOR UPDATE
- **THEN** the executor partitions the rows into 4 chunks
- **AND** filters each chunk via `rayon::par_iter`
- **AND** the tracing span `parallel_filter_engaged` is emitted

#### Scenario: Missing one condition falls back to sequential

- **WHEN** only one of the 4 conditions fails (e.g. correlated subquery present)
- **THEN** the executor falls back to the sequential filter path
- **AND** no parallel filter tracing span is emitted
- **AND** result correctness is preserved (bit-exact match)

#### Scenario: FOR UPDATE disables parallel

- **WHEN** `parallel_degree=4` AND `FOR UPDATE` is present
- **THEN** parallel filter is bypassed (LockManager deadlock prevention)
- **AND** the sequential path runs
- **AND** the result set is identical to a sequential `parallel_degree=1` execution

### Requirement: End-to-end cell-level equivalence

Executing the same query with `parallel_degree=1` and `parallel_degree=N`
SHALL produce bit-identical results (modulo execution timing).

#### Scenario: TPC-H Q1 with parallel_degree=1 vs 4

- **WHEN** running `SELECT l_returnflag, l_linestatus, SUM(l_quantity), ...`
  on a TPC-H SF=0.01 lineitem table
- **AND** `parallel_degree=1`
- **AND** `parallel_degree=4`
- **THEN** the two result sets are identical (row-by-row, column-by-column)
- **AND** the aggregate scalar values match within 1e-9 floating-point tolerance

#### Scenario: Per-row count and column values

- **WHEN** running `SELECT k, v FROM t WHERE k < N` with N=500000 on a 1M-row table
- **AND** `parallel_degree ∈ {1, 2, 4, 8}`
- **THEN** the row count is identical across all degrees
- **AND** the multiset of (k, v) tuples is identical

### Requirement: Performance non-regression

The parallel main path SHALL NOT introduce >5% throughput regression on
OLTP workloads (small queries, high concurrency).

#### Scenario: OLTP small-query non-regression

- **WHEN** running a workload with 100 concurrent connections
- **AND** each connection executes a 100-row SELECT with WHERE clause
- **AND** `parallel_degree=4`
- **THEN** aggregate QPS is within ±5% of `parallel_degree=1` baseline

### Requirement: Instrumentation and observability

The parallel main path SHALL emit a tracing span on engagement so that
operators can confirm parallel activation in production logs.

#### Scenario: Tracing span emitted on engagement

- **WHEN** `parallel_degree=4` activates the parallel filter
- **THEN** a span `sqlrustgo::parallel_filter_engaged` is emitted
- **AND** the span carries `degree` and `rows_in` fields
- **AND** the span closes when the parallel filter completes

#### Scenario: No span when sequential fallback

- **WHEN** the parallel guard is bypassed (e.g. FOR UPDATE present)
- **THEN** no `parallel_filter_engaged` span is emitted
- **AND** the sequential filter emits its existing `tracing::info!` line

### Requirement: Feature gating

The parallel main path SHALL only activate when the `parallel-executor`
feature is enabled (which brings in rayon).

#### Scenario: Default build (no features)

- **WHEN** building with `cargo build` (no features)
- **THEN** the `parallel-executor` feature is OFF
- **AND** the guard at `engine_select.rs` is in #[cfg(feature = "parallel-executor")]
  (or returns false unconditionally when feature is off)
- **AND** sequential path is used

#### Scenario: Feature-enabled build

- **WHEN** building with `cargo build --features parallel-executor`
- **THEN** the parallel filter guard is active
- **AND** rayon is included
- **AND** `parallel_degree > 1` engages the parallel path

### Requirement: E2E test coverage

The system SHALL provide an integration test that verifies the main-path
parallel filter engages when `parallel_degree > 1`.

#### Scenario: E2E test using ExecutionEngine directly

- **WHEN** running `tests/parallel_main_path_test.rs`
- **AND** `parallel_degree=4`
- **AND** `rows.len() = 600_000` (above PARALLEL_MIN_ROWS)
- **THEN** the parallel filter path is engaged
- **AND** the result matches the `parallel_degree=1` baseline

#### Scenario: E2E test via CLI / env var

- **WHEN** `SQLRUSTGO_EXECUTOR_PARALLELISM=4` is set before constructing
  `ExecutionEngine`
- **THEN** the engine reads the env var
- **AND** `parallel_degree()` returns 4
- **AND** `set_parallel_degree()` can override it
