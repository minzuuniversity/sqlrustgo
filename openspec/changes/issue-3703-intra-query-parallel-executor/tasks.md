# Tasks — Issue #3703: Intra-Query Parallel Executor

> **Scope**: Phase 1 (parallel scan) only. Phase 2 (parallel hash join) and Phase 3 (parallel GROUP BY) are separate changes.
> **Target**: v3.10.0 Phase 0/1 (per V310_DEVELOPMENT_PLAN.md §4)
> **Branch**: `develop/v3.10.0` (this work extends the v3.10.0-alpha1 baseline)
> **Est**: 1 week design + implementation, 1 day testing/bench

---

## 1. CLI flag + feature gate

- [ ] 1.1 Add `--executor-parallelism` long-form flag to `crates/cli/src/main.rs` clap parser (default 1)
- [ ] 1.2 Add `SQLRUSTGO_EXECUTOR_PARALLELISM` env var as alternative source
- [ ] 1.3 Wire flag value into `ExecutionEngine::set_parallel_degree(N)` at server startup
- [ ] 1.4 Add `parallel-executor` cargo feature to `crates/executor/Cargo.toml` (gates rayon + parallel code)
- [ ] 1.5 Add `parallel-executor` feature to workspace `Cargo.toml` features list with docstring
- [ ] 1.6 Verify: `cargo build --release` (no features) produces binary identical in size to v3.10.0-alpha1
- [ ] 1.7 Verify: `cargo build --release --features parallel-executor` includes rayon code

## 2. LocalExecutor parallel path (wiring)

- [ ] 2.1 Add `#[cfg(feature = "parallel-executor")] pub fn execute_select_parallel(&mut self, stmt: &SelectStatement, degree: usize) -> SqlResult<ExecutorResult>` to `crates/executor/src/local_executor.rs`
- [ ] 2.2 Implement `scan_base_rows(stmt) -> Vec<Vec<Value>>` helper (extract base scan logic from existing `execute_select`)
- [ ] 2.3 Implement `run_volcano_pipeline(partition, stmt) -> SqlResult<ExecutorResult>` (Filter+Project+Agg on a single partition)
- [ ] 2.4 Implement `merge_partials(partials: Vec<ExecutorResult>) -> ExecutorResult` (extend rows in partition order)
- [ ] 2.5 Add ORDER BY check: if `stmt.order_by.is_some()` return sequential path
- [ ] 2.6 Add `PARALLEL_MIN_ROWS` check: if `base_rows.len() < PARALLEL_MIN_ROWS` return sequential path
- [ ] 2.7 In `execute_select`, dispatch to `execute_select_parallel` when `self.parallel_degree > 1` AND feature enabled

## 3. Tests

- [ ] 3.1 Unit: `parallel_scan_partitions_evenly` (1000 rows / 4 partitions = [250, 250, 250, 250])
- [ ] 3.2 Unit: `parallel_scan_partitions_remainder` (1003 rows / 4 partitions = [251, 251, 251, 250])
- [ ] 3.3 Unit: `parallel_scan_empty_table` (0 rows → empty result, no panic)
- [ ] 3.4 Unit: `parallel_scan_n_exceeds_rows` (3 rows / 8 partitions → 3 non-empty partitions)
- [ ] 3.5 Integration: `parallel_n1_eq_n4_cell_match` (TPC-H Q1 SF=0.01, diff = 0)
- [ ] 3.6 Integration: `parallel_n1_eq_n4_tpch_22_22` (all 22 TPC-H queries, cell-level match)
- [ ] 3.7 Integration: `parallel_preserves_order_by_falls_back` (ORDER BY query → sequential, rows in order)
- [ ] 3.8 Integration: `parallel_below_threshold_falls_back` (small table → sequential path)
- [ ] 3.9 Regression: `cargo test --all-features --lib` (25 existing tests pass under both N=1 and N=4)
- [ ] 3.10 Regression: full TPC-H 22/22 wire-protocol test (`cargo test --test tpch_sf01_inprocess_test`)

## 4. Benchmarks (perf gate)

- [ ] 4.1 Add `parallel_bench_q1_sf01` bench: N=1 vs N=8, assert N=8 P99 ≤ N=1 P99 / 1.5
- [ ] 4.2 Add `parallel_bench_large_scan_sf01` (Q6): N=1 vs N=8, assert ≥ 2x speedup
- [ ] 4.3 Add `parallel_bench_oltp_no_regression`: sysbench oltp_read_write 16 threads, N=4 within ±5% of N=1 QPS
- [ ] 4.4 Capture baseline N=1 numbers + N=4 / N=8 numbers; record in `docs/releases/v3.10.0/perf/INTRA_QUERY_PARALLEL_BENCH.md`

## 5. Documentation

- [ ] 5.1 Update `docs/releases/v3.10.0/RELEASE_NOTES.md` with "Intra-query parallel scan" entry
- [ ] 5.2 Update `CHANGELOG.md` with "feat(executor): intra-query parallel scan (--executor-parallelism=N)"
- [ ] 5.3 Update `V310_DEVELOPMENT_PLAN.md` M-5 row status (OPEN → IN_PROGRESS → CLOSED)
- [ ] 5.4 Update `V310_ISSUES_PLAN.md` to add #3735 (V310-M-5 sub-issue) — link to #3703
- [ ] 5.5 Add `--executor-parallelism` to `README.md` CLI options section

## 6. CI integration

- [ ] 6.1 Add `cargo build --features parallel-executor` to `check_alpha_v3.10.0.sh` A1_BUILD matrix
- [ ] 6.2 Add `cargo test --features parallel-executor --lib` to A2_TEST matrix
- [ ] 6.3 Add `parallel_bench_q1_sf01` to `check_perf_baseline.sh` (soft gate, log only on regression)
- [ ] 6.4 Verify all 4 cargo build variants compile: `default` / `--features parallel-executor` / `--no-default-features` / `--no-default-features --features parallel-executor`

## 7. Final verification

- [ ] 7.1 `bash scripts/gate/check_alpha_v3.10.0.sh` PASS (15/15 with new parallel tests)
- [ ] 7.2 `bash scripts/gate/check_stage.sh --version v3.10.0 --stage ALPHA` PASS (9/9)
- [ ] 7.3 22/22 TPC-H cell-level match between N=1 and N=4 (recorded in evidence)
- [ ] 7.4 All 25 lib tests + integration suite pass under both `--features parallel-executor` and default build
- [ ] 7.5 Open follow-up issues: #3736 (Phase 2: parallel hash join), #3737 (Phase 3: parallel GROUP BY)
- [ ] 7.6 Link #3703 as related to #3735 (V310-M-5 sub-issue)
- [ ] 7.7 Commit + push to `develop/v3.10.0`, comment on #3703 with PR link

---

## Acceptance gate

This change is **READY for merge** when:
- All 7.1-7.4 checks PASS
- 22/22 TPC-H cell-level match verified
- ≥2x speedup on Q1/Q6 SF=0.1 with N=8
- Zero regression on OLTP sysbench (N=4 within ±5% of N=1)
- All artifacts in `openspec/changes/issue-3703-intra-query-parallel-executor/` are `done`
