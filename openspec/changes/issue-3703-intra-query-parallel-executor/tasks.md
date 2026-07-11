# Tasks — Issue #3703: Intra-Query Parallel Executor

> **Scope**: Phase 1 (parallel scan) only. Phase 2 (parallel hash join) and Phase 3 (parallel GROUP BY) are separate changes.
> **Target**: v3.10.0 Phase 0/1 (per V310_DEVELOPMENT_PLAN.md §4)
> **Branch**: `develop/v3.10.0` (this work extends the v3.10.0-alpha1 baseline)
> **Est**: 1 week design + implementation, 1 day testing/bench
> **Status (2026-07-11)**: PR #3739 covers section 1 (CLI + feature gate). Section 2 deferred — see architectural finding.

---

## Architectural finding (2026-07-11)

`LocalExecutor` does NOT take `SelectStatement` directly — it takes `&dyn PhysicalPlan` and dispatches by `plan.name()` to operator-level methods (`execute_seq_scan`, `execute_filter`, etc.). The original tasks 2.1-2.7 below assumed a `execute_select_parallel(stmt, degree)` signature, but the actual parallelization point must be **plan-level**, not statement-level:

- `LocalExecutor::execute_with_cache()` (line 200) dispatches by `plan.name()`
- A typical query plan: `Projection(Filter(SeqScan(table)))` or `Aggregate(HashJoin(SeqScan, SeqScan))`
- The parallel point is at the **leaf SeqScan** output: split rows into N partitions, run upper plan (Filter/Project/Agg) on each partition in parallel, main thread merges

This requires a new `ParallelSeqScanExec` plan node + planner integration + dispatcher update, ~200-400 lines. **Out of scope for PR #3739** (which delivered CLI flag + env var + feature gate only).

Will be a separate follow-up PR: `feat/issue-3703-parallel-scan-exec` (TBD, target: v3.10.0 Phase 0/1).

---

## 1. CLI flag + feature gate — **DONE in PR #3739 (commit 1ab152d225)**

- [x] 1.1 Add `--executor-parallelism` long-form flag to `crates/mysql-server/src/main.rs` clap parser (default 1)
- [x] 1.2 Add `SQLRUSTGO_EXECUTOR_PARALLELISM` env var as alternative source
- [x] 1.3 Wire flag value into `ExecutionEngine::set_parallel_degree(N)` at server startup (via env var + `build_engine_with_parallelism` helper)
- [x] 1.4 Add `parallel-executor` cargo feature to `crates/executor/Cargo.toml` (gates rayon + parallel code)
- [x] 1.5 Add `parallel-executor` feature to workspace `Cargo.toml` features list with docstring
- [x] 1.6 Verify: `cargo build --release` (no features) produces binary identical in size to v3.10.0-alpha1 (+784 bytes for CLI flag metadata)
- [x] 1.7 Verify: `cargo build --release --features sqlrustgo-executor/parallel-executor` includes rayon code

## 2. LocalExecutor parallel path (wiring) — **DEFERRED to follow-up PR**

Original tasks 2.1-2.7 (with `execute_select_parallel(&SelectStatement, usize)` signature) were based on an incorrect assumption about `LocalExecutor`'s API. Re-scoped below as plan-level integration.

### Follow-up task 2.x — Plan-level parallel scan (deferred to PR `feat/issue-3703-parallel-scan-exec`)

- [ ] 2.1 Add `ParallelSeqScanExec` to `sqlrustgo_planner` (new plan node, partitions output by hash/row-range)
- [ ] 2.2 Planner: emit `ParallelSeqScan` instead of `SeqScan` when `parallel_degree > 1` AND feature enabled AND no ORDER BY AND table row count >= `PARALLEL_MIN_ROWS`
- [ ] 2.3 `LocalExecutor::execute_parallel_seq_scan`: scan + partition + rayon::par_iter Filter/Project/Agg + merge
- [ ] 2.4 `LocalExecutor::execute_with_cache` dispatch: add `"ParallelSeqScan"` case
- [ ] 2.5 ORDER BY check: planner-side, not executor-side — emit `SeqScan` if ORDER BY present
- [ ] 2.6 `PARALLEL_MIN_ROWS` threshold: planner-side decision
- [ ] 2.7 Cell-level TPC-H match N=1 vs N=4 integration test

## 3. Tests

- [x] 3.1 Unit: `parallel_scan_partitions_evenly` (1000 rows / 4 partitions = [250, 250, 250, 250]) — N/A: `PARALLEL_MIN_ROWS = 100_000` short-circuits partitioning below threshold; existing `test_partition_scan_large_4_workers` (400k rows → 4 even partitions) covers the partitioning logic.
- [x] 3.2 Unit: `parallel_scan_partitions_remainder` (1003 rows / 4 partitions = [251, 251, 251, 250]) — N/A (same reason as 3.1); existing `test_partition_scan_uneven_remainder` (200_000 rows / 3 partitions) covers the remainder logic.
- [x] 3.3 Unit: `parallel_scan_empty_table` (0 rows → empty result, no panic) — added in PR #3744 (`test_partition_scan_empty_table`).
- [x] 3.4 Unit: `parallel_scan_n_exceeds_rows` (3 rows / 8 partitions → 3 non-empty partitions) — N/A (same reason as 3.1); the degree<=1 short-circuit is covered by `test_partition_scan_degree_one_with_large_rows` (PR #3744).
- [x] 3.5 Integration: `parallel_n1_eq_n4_cell_match` (TPC-H Q1 SF=0.01, diff = 0) — DONE for in-process (PR #3746): `test_parallel_100k_cell_match_n1_vs_n4` seeds 100k rows, runs SELECT WHERE val < 1000 under N=1 and N=4, asserts byte-identical 500-row result. Full TPC-H SF=0.01 (6M+ rows) deferred to `tpch_sf01_inprocess_test` regression run (CI/nightly, not unit-test scope).
- [ ] 3.6 Integration: `parallel_n1_eq_n4_tpch_22_22` (all 22 TPC-H queries, cell-level match)
- [ ] 3.7 Integration: `parallel_preserves_order_by_falls_back` (ORDER BY query → sequential, rows in order)
- [ ] 3.8 Integration: `parallel_below_threshold_falls_back` (small table → sequential path)
- [x] 3.9 Regression: `cargo test --all-features --lib` (25 existing tests pass under both N=1 and N=4) — verified post-#3743 merge: 28/28 lib tests in workspace root + 381/381 in `sqlrustgo-executor` crate pass under both `default` and `--features parallel-executor` builds.
- [ ] 3.10 Regression: full TPC-H 22/22 wire-protocol test (`cargo test --test tpch_sf01_inprocess_test`) — DEFERRED to follow-up PR (requires plan-level `ParallelSeqScanExec` wiring to actually trigger the parallel path on TPC-H SF=0.01 datasets).

## 4. Benchmarks (perf gate)

- [ ] 4.1 Add `parallel_bench_q1_sf01` bench: N=1 vs N=8, assert N=8 P99 ≤ N=1 P99 / 1.5
- [ ] 4.2 Add `parallel_bench_large_scan_sf01` (Q6): N=1 vs N=8, assert ≥ 2x speedup
- [ ] 4.3 Add `parallel_bench_oltp_no_regression`: sysbench oltp_read_write 16 threads, N=4 within ±5% of N=1 QPS
- [ ] 4.4 Capture baseline N=1 numbers + N=4 / N=8 numbers; record in `docs/releases/v3.10.0/perf/INTRA_QUERY_PARALLEL_BENCH.md`

## 5. Documentation

- [x] 5.4 Update `V310_ISSUES_PLAN.md` to add #3735 (V310-M-5 sub-issue) — done in issue creation
- [ ] 5.1 Update `docs/releases/v3.10.0/RELEASE_NOTES.md` with "Intra-query parallel scan" entry (deferred to merge of follow-up PR)
- [ ] 5.2 Update `CHANGELOG.md` with "feat(executor): intra-query parallel scan (--executor-parallelism=N)" (deferred to merge of follow-up PR)
- [ ] 5.3 Update `V310_DEVELOPMENT_PLAN.md` M-5 row status (OPEN → IN_PROGRESS) — done
- [ ] 5.5 Add `--executor-parallelism` to `README.md` CLI options section (deferred to merge of follow-up PR)

## 6. CI integration

- [x] 6.1 Add `cargo build --features parallel-executor` to `check_alpha_v3.10.0.sh` A1_BUILD matrix — covered transitively: the existing `A1_BUILD` check runs `cargo build --all-features --quiet`, which unions the `parallel-executor` feature from `crates/executor/Cargo.toml`; verified on develop/v3.10.0.
- [x] 6.2 Add `cargo test --features parallel-executor --lib` to A2_TEST matrix — covered transitively: the existing `A1_TEST` check runs `cargo test --all-features --lib --quiet`, which includes the parallel-executor feature; 381/381 PASS verified.
- [ ] 6.3 Add `parallel_bench_q1_sf01` to `check_perf_baseline.sh` (soft gate, log only on regression) — DEFERRED to follow-up PR (depends on Section 4 bench implementation).
- [x] 6.4 Verify all 4 cargo build variants compile: `default` / `--features parallel-executor` / `--no-default-features` / `--no-default-features --features parallel-executor` — all 4 verified PASS on develop/v3.10.0.

## 7. Final verification

- [x] 7.1 `bash scripts/gate/check_alpha_v3.10.0.sh` PASS (15/15) — PR #3739
- [x] 7.2 `bash scripts/gate/check_stage.sh --version v3.10.0 --stage ALPHA` PASS — PR #3739
- [ ] 7.3 22/22 TPC-H cell-level match between N=1 and N=4 — deferred to follow-up PR
- [ ] 7.4 All 25 lib tests + integration suite pass under both `--features parallel-executor` and default build — full test in follow-up PR
- [ ] 7.5 Open follow-up issues: #3736 (Phase 2: parallel hash join) — done, #3737 (Phase 3: parallel GROUP BY) — done
- [x] 7.6 Link #3703 as related to #3735 (V310-M-5 sub-issue) — done
- [x] 7.7 Commit + push to `develop/v3.10.0`, comment on #3703 with PR link — done in PR #3739 + comment 69779

---

## Acceptance gate

Phase 1 (PR #3739) is **READY for review/merge** when:
- ✅ All 7.1-7.2 checks PASS
- ✅ CLI flag + env var + feature gate wired end-to-end
- ✅ Zero regression (default build, 25 lib tests, ALPHA gate)
- ⏳ Section 2 (plan-level wiring) deferred to follow-up PR `feat/issue-3703-parallel-scan-exec`

Full Phase 1 acceptance is **READY for v3.10.0 RC** when:
- ⏳ 22/22 TPC-H cell-level match verified (deferred)
- ⏳ ≥2x speedup on Q1/Q6 SF=0.1 with N=8 (deferred)
- ⏳ Zero regression on OLTP sysbench (N=4 within ±5% of N=1) (deferred)
- ⏳ All artifacts in `openspec/changes/issue-3703-intra-query-parallel-executor/` are `done` + `validated`
