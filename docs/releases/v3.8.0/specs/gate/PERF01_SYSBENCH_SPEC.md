# PERF-01: sysbench OLTP_READ_WRITE Benchmark SPEC

**Issue**: #2989
**Status**: DRAFT (Phase 1 of 3)
**Estimated effort**: ~40h
**Target version**: v3.9.0 (RC integration)

## Background

`crates/bench/src/workload/oltp_read_write.rs` implements the
`oltp_read_write` workload following the sysbench OLTP_READ_WRITE.lua
pattern (point-select, range-scan, update-index, update-non-index,
insert, delete). It is dispatched from
`crates/bench/src/workload/mod.rs:55`.

**Gap**: no end-to-end driver that runs the workload at
1/4/16/64 threads and produces comparison numbers against MySQL 5.7.

## Phase 1: SPEC (this document) [DONE in v3.8.0]

## Phase 2: Driver + JSON output [~12h]

1. Create `crates/bench/src/bin/sysbench_oltp_rw.rs`:
   - CLI: `sqlrustgo-bench sysbench-oltp-rw --threads 1,4,16,64 --tables 10 --rows 100000`
   - Runs oltp_read_write at each thread count
   - Outputs JSON via `crates/bench/src/report/json_report.rs`
2. Add `[[bin]]` entry in `crates/bench/Cargo.toml`
3. Add a smoke test that runs 1 thread × 1000 rows × 10s

## Phase 3: MySQL 5.7 comparison [~16h]

1. Use `crates/bench/src/db/mysql.rs` (already exists) to drive MySQL 5.7
2. Run identical workload against MySQL 5.7 reference
3. Diff the JSON output, produce a comparison report:
   - latency p50/p95/p99
   - throughput (tps)
   - error rate

## Phase 4: Multi-version governance integration [~12h]

1. Append SQLRustGo + MySQL 5.7 numbers to
   `docs/benchmarks/sysbench-oltp-rw-v3.9.0.md`
2. Wire into `scripts/gate/check_perf_regression.sh` (new)
3. Update `MULTI_VERSION_GOVERNANCE_REPORT.md` with the deltas

## Acceptance Criteria

- Phase 1, 2, 3, 4 complete
- Comparison report shows SQLRustGo ≥ MySQL 5.7 on at least one metric
  (or honest regression report if not)
- Driver is reproducible: identical inputs produce identical numbers

## Risks

- MySQL 5.7 setup may not be available on all dev machines.
  Mitigation: use Docker, document in
  `docs/benchmarks/MYSQL57_SETUP.md`.
- 64-thread runs may be noisy. Mitigation: warmup + multiple iterations
  + report median of 3 runs.
- Workload semantics may diverge from real sysbench Lua.
  Mitigation: keep the Rust implementation faithful to the Lua; document
  differences in `docs/benchmarks/SYSBENCH_FIDELITY.md`.

## Reference

- sysbench OLTP_READ_WRITE.lua (canonical)
- `docs/benchmarks/` existing TPC-H numbers
- `crates/bench/examples/tpch_compare.rs` (precedent for cross-DB compare)
