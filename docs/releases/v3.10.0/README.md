# v3.10.0

> **版本定位**: MySQL 5.7 替代 — 功能稳定 + 基本性能优先
> **分支**: `develop/v3.9.0`（从 v3.9.0 分支开发）
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
