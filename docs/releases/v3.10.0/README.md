# v3.10.0 — 开发中

> **版本定位**: MySQL 5.7 替代 — 功能稳定 + 基本性能优先 + Wired SOAK 闭环
> **分支**: `develop/v3.10.0` (forked from `main` at `23353c0c54`, 2026-07-11)
> **创建日期**: 2026-06-25
> **当前状态**: **DRAFT** — 计划阶段，已完成 ISSUE 总控定义，等待 Gitea 创建 issue
> **目标**: v3.9.0 中 44 个 ignore 测试 + 跨版本历史遗留债务整合 + Wired SOAK 闭环

---

## 核心目标

- **ACID 正确性**: ROLLBACK MVCC 完整、MemoryStorage 事务边界
- **DML 完整性**: INSERT SELECT、子查询 UPDATE/DELETE、多表 DML
- **UNION 集合操作**: INTERSECT、EXCEPT、UNION ORDER BY/LIMIT
- **ALTER TABLE**: RENAME/MODIFY COLUMN 完整支持
- **崩溃恢复**: 真实 kill -9 进程级测试
- **Wired SOAK 闭环**: sysbench prepare/run 真接入（ADR-013）

## 计划文档

| 文件 | 内容 |
|------|------|
| `plans/V310_DEVELOPMENT_PLAN.md` | 完整开发计划（功能 backlog + 历史债务整合） |
| `plans/V310_CLI_BINARY_PLAN.md` | CLI binary 实现计划（C-6） |
| `plans/V310_VERSION_PLAN.md` | 版本号与里程碑计划 |
| **`plans/V310_ISSUES_PLAN.md`** | **总控 ISSUE + 12 个子 ISSUE 完整定义** |
| `STAGE.yaml` | 当前阶段状态（DRAFT） |

## 总控 ISSUE 计划

**12 个子 ISSUE**（详见 [`plans/V310_ISSUES_PLAN.md`](plans/V310_ISSUES_PLAN.md)）:

| # | ISSUE | 主题 | 估时 |
|---|--------|------|------|
| 1 | V310-01 | DML 完整性 | 80h |
| 2 | V310-02 | UNION 集合操作 | 45h |
| 3 | V310-03 | ACID 事务正确性 | 80h |
| 4 | V310-04 | ALTER TABLE 完整性 | 20h |
| 5 | V310-05 | 真实崩溃恢复 + 24h SOAK | 80h |
| 6 | V310-06 | Wired-SOAK DDL (PR1) | 40h |
| 7 | V310-07 | Catalog 4 层重构 (PR2) | 80h |
| 8 | V310-08 | DDL 执行路径 (PR3) | 80h |
| 9 | V310-09 | Wire 握手修复 (PR4) | 60h |
| 10 | V310-10 | 覆盖率提升至 ≥80% | 40h |
| 11 | V310-11 | TPC-H SF=1 22/22 闭环 | 80h |
| 12 | V310-12 | 其他 ignore 测试 + 跨版本债 | 60h |

**总估时**: ~745h (~5 个月全职)

## 历史债务整合

本文档整合了以下历史遗留债务:
- `INT5_PLUS_DEBT_INVENTORY.md` — v3.0→v3.8 跨版本债务全量清单
- `ARCH_SEM_DEBT_REMEDIATION_PLAN.md` — ARCH-2/3 + SEM-1~4
- `IGNORE_REGISTRY_2026-06-25.md` — v3.9.0 ignore 测试审计（44个）
- [ADR-013](../../governance/adr/ADR-013-v310-wired-soak-ddl-and-wire-protocol-repair.md) — Wired SOAK + DDL 修复 RFC

## 后续步骤

1. 等待 Gitea 恢复访问后批量创建 V310-MASTER + 12 个子 ISSUE
2. 在 v3.9.0 GA 治理框架下，验证总控 ISSUE 链接与依赖关系
3. 进入 DRAFT → ALPHA 阶段，启动实施

---

*最近更新: 2026-07-11 — develop/v3.10.0 分支创建 + ISSUE 计划文档完成*
