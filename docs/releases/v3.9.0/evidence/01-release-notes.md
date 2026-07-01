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

# 01 - Release Notes

See [`../RELEASE_NOTES.md`](../RELEASE_NOTES.md) for full details.

## Summary

**v3.9.0 is a production-readiness release focused on three things**:

1. **TPC-H 22/22** — all 22 TPC-H queries pass in-process, all 22 pass wire-protocol, 21/22 match cell-level vs SQLite
2. **Q13 subquery fix** — `NOT IN (subquery_with_LIKE)` is tri-valued-logic correct
3. **Q9 6x faster** — 6-way join now 90ms (was 600ms) via hash-join pre-filter pushdown

## New Features
- TPC-H 22/22 SHA-256 baseline
- Backup/Restore/PITR (G6)
- Savepoint (G5)
- INT-2 Parallel Executor (G2)
- INT-3 Single Expression (G3)
- Time Travel + Hash Chain (G10)
- QPS/TPS Benchmark (G11)
- Cross-version upgrade chain (v3.6→v3.7→v3.8→v3.9)

## Bug Fixes
- Q13 subquery tri-valued-logic
- Q9 6x perf via hash-join
- ODUK (INSERT...ON DUPLICATE KEY UPDATE) bugfixes (#3370)
- 5 DML helpers extracted from execution_engine.rs
- SET NAMES hang fix
- TPC-H Q21 predicate pushdown
- Q8 + Q21 timeout fix

## Breaking Changes
None (binary-swap upgrade from v3.8.0)

## Known Issues (DRIFT, not blocking)
- C-ARCH-05: execution_engine.rs 1919 > 1800 lines (deferred to v3.9.1)
- SGL-005: 2 storage bypasses (pre-existing)
- No VACUUM (append-only, dead tuples accumulate)
- Q22 has known multi-COUNT divergence vs SQLite (not a bug)
