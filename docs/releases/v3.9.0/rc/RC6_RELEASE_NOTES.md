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

# v3.9.0-rc6 Release Notes

> **Date**: 2026-06-12
> **Tag**: `v3.9.0-rc6`

## What's New Since rc5

### Engineering

- **INT-2 full substance tests**: 9 tests for ParallelExecutor wiring (PR #3362, Issue #3108)
- **INT-3 full substance tests**: 17 tests for Expression delegation (PR #3362, Issue #3146)
- **Combined test suite**: INT-2 + INT-3 substance tests total 30 tests

## All RC6 Blockers Closed

| # | Issue | Status |
|---|-------|--------|
| #3108 | INT-2 ParallelExecutor | ✅ Closed (PR #3362) |
| #3146 | INT-3 Expression delegation | ✅ Closed (PR #3362) |

## Next: RC7

RC7 focuses on: Performance documentation finalization, MariaDB comparison
