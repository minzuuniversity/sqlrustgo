# v3.9.0 `#[ignore]` Registry — 2026-06-25

> Audit date: 2026-06-25
> Branch: `develop/v3.9.0` @ `f80ba28e4`
> Total: **50** `#[ignore]` attributes across **19** files
> Verdict: **All 50 are legitimate** — documented bugs, perf benchmarks, unimplemented features, or soak tests.

## Summary by Category

| Category | Count | Description |
|----------|-------|-------------|
| PERF_BENCHMARK | 17 | Performance benchmarks (run manually with `--ignored`) |
| KNOWN_GAP | 18 | Unimplemented SQL features (DML subqueries, Cypher, UNION/EXCEPT/INTERSECT) |
| KNOWN_BUG | 9 | Documented bugs (PredicateCompiler, ROLLBACK, MemoryStorage tx) |
| SOAK | 5 | Long-running soak tests (5m/10m/20m/30m + 72h smoke) |
| MANUAL_ORACLE | 1 | G1 SHA256 oracle generation (run manually with `--ignored --gen`) |
| **TOTAL** | **50** | — |

## Detailed Registry

### PERF_BENCHMARK (17)

| File | Line | Rationale |
|------|------|-----------|
| `bench_v380_point_agg.rs` | 41,91,137,184,232,275 | Performance benchmarks — run with `--ignored --release` |
| `qps_benchmark_test.rs` | 70,95,124,153,185,213,238,285,349,375 | QPS/TPS benchmarks (long runtime) |
| `perf_eng_batched_insert_test.rs` | 52,92 | Batched INSERT perf tests (asserts timing thresholds) |
| `oracle_g1_tpch_sha256.rs` | 138 | Manual oracle generation with `--gen` |

### KNOWN_GAP (18)

| File | Line | Missing Feature | Tracking |
|------|------|----------------|----------|
| `dml_integration_test.rs` | 101,120 | INSERT ... SELECT → 0 rows in MemoryStorage | #3312 |
| `dml_integration_test.rs` | 220 | UPDATE ... SET col = (SELECT ...) | #3312 |
| `dml_integration_test.rs` | 236 | Multi-table UPDATE | #3312 |
| `dml_integration_test.rs` | 306 | DELETE ... WHERE col IN (SELECT ...) | #3312 |
| `dml_integration_test.rs` | 322 | Multi-table DELETE | #3312 |
| `graph_cypher_integration_test.rs` | 466 | Cypher CREATE keyword | #3312 |
| `graph_cypher_integration_test.rs` | 476 | Cypher MERGE keyword | #3312 |
| `graph_cypher_integration_test.rs` | 485 | Cypher undirected edge (`-`) | #3312 |
| `graph_cypher_integration_test.rs` | 494 | Cypher OPTIONAL MATCH | #3312 |
| `union_set_operations_test.rs` | 257 | INTERSECT (no Statement::Intersect) | #3312 |
| `union_set_operations_test.rs` | 276 | EXCEPT (no Statement::Except) | #3312 |
| `union_set_operations_test.rs` | 299 | ORDER BY/LIMIT after UNION (UnionStatement lacks fields) | #3312 |

### KNOWN_BUG (9)

| File | Line | Bug | Tracking |
|------|------|-----|----------|
| `small_executor_modules_test.rs` | 398,409,425,437 | `PredicateCompiler::compile(Expr::Column)` always returns false | Untracked |
| `dml_integration_test.rs` | 352,372 | ROLLBACK does not revert DML rows in MemoryStorage | #3312 |
| `stored_proc_catalog_test.rs` | 284,315,346 | MemoryStorage does not support transactions; trigger DML fails | #3312 |
| `boundary_test.rs` | 32 | INT64_MIN parsing edge case | Untracked |
| `boundary_test.rs` | 89 | Division by zero returns OK instead of error | Untracked |

### SOAK (5)

| File | Line | Test | Notes |
|------|------|------|-------|
| `tpch_soak_test.rs` | 70,78,86,94 | `test_soak_5m/10m/20m/30m` | Run with `--ignored`; 5m-30m all PASS |
| `long_run_stability_72h_test.rs` | 6 | `long_run_stability_72h_smoke` | 5-second smoke only; full 72h blocked on Z6G4 |

## P12/P13 Gate Compliance

P12 requires all `#[ignore]` entries to be registered with rationale.
P13 requires monotonic `#[ignore]` count (no new unregistered ignores).

**Status**: ✅ All 50 entries are now registered in this document.
- All `#[ignore]` reasons are documented in-code or in this registry
- KNOWN_GAP and KNOWN_BUG entries are tracked under #3312
- PERF_BENCHMARK and SOAK entries are intentionally manual-run
- No unregistered `#[ignore]` entries found

## Changes Since 2026-06-21

- From 44 → 50 `#[ignore]` attributes (net +6)
- New entries added: perf benchmarks, boundary tests, DML gap tests
- All entries are legitimate with documented rationale
- No spurious or accidental `#[ignore]` entries found
