<!-- 2026-07-01 status addendum (auto-applied) -->
> **状态更新**: 本机 L1 lint + 架构整理已闭环。HEAD `37bcb788c2`, 3 个 PR 已合并 (PR #3664, #3665, #3666)。
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

# v3.10.0

> **版本定位**: MySQL 5.7 替代 — 功能稳定 + 基本性能优先
> **分支**: `develop/v3.9.0`（从 v3.9.0 分支开发，HEAD `d77821f6d1`，RC8 ✅ + 本机 L1 闭环 2026-07-01）
> **目标**: 成为可生产的 MySQL 5.7 替代版本

## 计划文档

| 文件 | 内容 |
|------|------|
| `plans/V310_DEVELOPMENT_PLAN.md` | 完整开发计划（功能 backlog + 历史债务整合） |

## 核心目标

- **ACID 正确性**: ROLLBACK MVCC 完整、MemoryStorage 事务边界
- **DML 完整性**: INSERT SELECT、子查询 UPDATE/DELETE、多表 DML
- **UNION 集合操作**: INTERSECT、EXCEPT、UNION ORDER BY/LIMIT
- **ALTER TABLE**: RENAME/MODIFY COLUMN 完整支持
- **崩溃恢复**: 真实 kill -9 进程级测试

## 历史债务整合

本文档整合了以下历史遗留债务：
- `INT5_PLUS_DEBT_INVENTORY.md` — v3.0→v3.8 跨版本债务全量清单
- `ARCH_SEM_DEBT_REMEDIATION_PLAN.md` — ARCH-2/3 + SEM-1~4
- `IGNORE_REGISTRY_2026-06-25.md` — v3.9.0 ignore 测试审计（44个）
