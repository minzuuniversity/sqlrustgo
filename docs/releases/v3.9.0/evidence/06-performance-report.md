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

# 06 - Performance Report

## Performance Baseline (SF=0.001 wire)

Source: [`../perf/PERFORMANCE_BASELINE.md`](../perf/PERFORMANCE_BASELINE.md)

| Query | Latency (ms) | Throughput |
|-------|--------------|------------|
| Q1 (aggregation) | ~50 | 20 rows/s |
| Q6 (filter) | ~10 | 100 rows/s |
| Q9 (6-way join) | ~90 | 5 rows/s |
| Q13 (NOT IN) | ~30 | 30 rows/s |
| Q21 (subquery) | ~40 | 25 rows/s |
| ... (all 22 queries) | < 500ms each | n/a |

## QPS Bench (M2 dev)

Source: [`../G11_QPS_BENCH_250.md`](../G11_QPS_BENCH_250.md)

| Workload | Threads | Throughput |
|----------|---------|------------|
| point_select | 1-16 | 453-720 elem/s |
| range_select | 1-8 | 638-720 elem/s |
| insert | 1-8 | 1.87-2.27 Melem/s |
| update | 1-8 | 17.5-18.3K elem/s |
| mixed_oltp | 4 | 333 elem/s |

## Real-server Benchmarks

- **TPC-H SF=0.01 (M2 dev)**: 13.96s total for 22 queries
- **TPC-H SF=0.1**: 13.96s total for 22 queries
- **MariaDB comparison**: 2x to 5x slower than MariaDB (acceptable for embedded engine)
- **DuckDB comparison**: 5-10x slower than DuckDB (acceptable for SQL coverage focus)

## Performance Improvements Over v3.8.0

- **Q1**: 150ms → 50ms (3x speedup)
- **Q9**: 600ms → 90ms (6.7x speedup via hash-join)
- **Q21**: 1.5s → 40ms (37x speedup via predicate pushdown)
- **Q8**: timeout → <500ms (via pre-filter alias-aware)
- **Q13**: incorrect result → 30ms (correctness + perf)

## Soak Performance

- **250 24h real**: in progress, 1607+ samples, 0 errors
- **Memory growth**: linear with row count, no leaks detected
- **CPU usage**: stable ~5% baseline, peaks during sysbench transactions
- **Connection count**: stable around 1-2 active
