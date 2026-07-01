<!-- 2026-07-01 status addendum (auto-applied) -->
> **状态更新**: 本机 L1 lint + 架构整理已闭环。HEAD `d77821f6d1`, 3 个 PR 已合并 (PR #3664, #3665, #3666)。
> - `src/execution_engine.rs` 1471 行 (AD-001 1500 目标达标, 2630 → 1471)
> - C-ARCH-05 上限锁回 1500 (从 3000/1800 统一)
> - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
> - Open issues (4, 全部硬件阻塞, 本机无法推进):
>   - #3648 TPC-H 混合负载 SOAK 跨平台验证 (需要 Z6G4/Z440)
>   - #3423 TPC-H SF=1.0 baseline (需要 75GB+ 磁盘, Mac mini 仅 1GB)
>   - #3265 72h 长跑 SOAK (blocked-on-S1, 需 72+ 小时持续运行)
>   - #3266 168h 长跑 SOAK (blocked-on-S1, 需 168 小时持续运行)
> - 详见: issue #3667 (closed as state snapshot) + CHANGELOG.md
>
> 本文件原始内容保持不变,仅顶部加 addendum。

---

# v3.9.0 `#[ignore]` Registry — 2026-06-25 (updated)

> Audit date: 2026-06-25
> Branch: `develop/v3.9.0` @ `9e2806278`
> Total: **44** `#[ignore]` attributes across **19** files (was 50)
> Verdict: **All 44 are legitimate** — documented bugs, perf benchmarks, unimplemented features, or soak tests.

## Summary by Category

| Category | Count | Description |
|----------|-------|-------------|
| PERF_BENCHMARK | 17 | Performance benchmarks (run manually with `--ignored`) |
| KNOWN_GAP | 18 | Unimplemented SQL features (DML subqueries, Cypher, UNION/EXCEPT/INTERSECT) |
| KNOWN_BUG | 3 | Documented bugs (ROLLBACK, MemoryStorage tx) — **6 fixed by commits d87801e43 + 9e2806278** |
| SOAK | 5 | Long-running soak tests (5m/10m/20m/30m + 72h smoke) |
| MANUAL_ORACLE | 1 | G1 SHA256 oracle generation (run manually with `--ignored --gen`) |
| **TOTAL** | **44** | — |

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

### KNOWN_BUG (3 — was 9, 6 fixed by commits d87801e43 + 9e2806278)

| File | Line | Bug | Tracking |
|------|------|-----|----------|
| `dml_integration_test.rs` | 352,372 | ROLLBACK does not revert DML rows in MemoryStorage | #3312 |
| `stored_proc_catalog_test.rs` | 284,315,346 | MemoryStorage does not support transactions; trigger DML fails | #3312 |


### SOAK (5)

| File | Line | Test | Notes |
|------|------|------|-------|
| `tpch_soak_test.rs` | 70,78,86,94 | `test_soak_5m/10m/20m/30m` | Run with `--ignored`; 5m-30m all PASS |
| `long_run_stability_72h_test.rs` | 6 | `long_run_stability_72h_smoke` | 5-second smoke only; full 72h blocked on Z6G4 |

## P12/P13 Gate Compliance

P12 requires all `#[ignore]` entries to be registered with rationale.
P13 requires monotonic `#[ignore]` count (no new unregistered ignores).

**Status**: ✅ All 44 entries are registered.
- **4 PredicateCompiler tests un-ignored** (commit `d87801e43`): `test_predicate_compiler_column_true`, `test_predicate_compiler_column_false`, `test_predicate_compiler_binary_and`, `test_predicate_compiler_unary_not`
- KNOWN_GAP and KNOWN_BUG entries tracked under #3312
- PERF_BENCHMARK and SOAK entries are intentionally manual-run
- No unregistered `#[ignore]` entries found

## Changes Since 2026-06-21

- From 44 → 50 → 44 `#[ignore]` attributes (net -6 from fixes)
- **4 PredicateCompiler bugs fixed** (commit `d87801e43`)
- **2 boundary test bugs fixed** (commit `9e2806278`): INT64_MIN parsing, zero-division parsing
- All remaining entries are legitimate with documented rationale
