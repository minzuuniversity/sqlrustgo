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

# v3.9.0-rc5 Release Notes

> **Date**: 2026-06-12
> **Tag**: `v3.9.0-rc5`

## What's New Since rc4

### Engineering

- **IS NULL pushdown**: skip IS NULL/IS NOT NULL in single-table predicate pushdown (PR #3259)
- **Cross-version upgrade chain**: v3.6→v3.7→v3.8→v3.9 complete chain (PR #3361, Issue #3270)
- **D7 reliability wrapper**: 60-90s timeout wrapper to avoid blocking GA gate

## All RC5 Blockers Closed

| # | Issue | Status |
|---|-------|--------|
| #3270 | Cross-version upgrade chain | ✅ Closed (PR #3361) |
| #3259 | IS NULL pushdown fix | ✅ Closed (PR #3259) |

## Next: RC6

RC6 focuses on: INT-2/INT-3 full substance tests completion
