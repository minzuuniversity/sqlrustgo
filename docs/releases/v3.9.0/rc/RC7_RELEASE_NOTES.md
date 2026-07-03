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

# v3.9.0-rc7 Release Notes

> **Date**: 2026-06-12
> **Tag**: `v3.9.0-rc7` @ `642ff9cf9`

## What's New Since rc6

### Documentation

- **Performance overview**: Comprehensive performance overview document (PR #3363)
- **MariaDB comparison**: Real MariaDB comparison vs SQLRustGo (PR #3363)
- **Sysbench comparison**: SYSBENCH_MARIADB_COMPARISON_20260612.md

### Performance Baseline

- TPC-H 22-query runtime: ~30s (v3.8.0) → ~2.3s (v3.9.0)
- Q9 (6-way join): ~600ms → ~90ms (6.7x faster)
- Q1 (aggregation): ~150ms → ~50ms
- General executor: ~2x faster via IR plan + better join ordering

## All RC7 Deliverables

| Deliverable | Status |
|-------------|--------|
| Performance overview | ✅ Complete |
| MariaDB comparison | ✅ Complete |
| SYSBENCH comparison | ✅ Complete |

## Next: GA

GA pending: 24h real soak completion (running on 250), 72h/168h soak (pending 24h)
GA target: 2026-12-15
