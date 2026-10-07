# SQLRustGo v4.1.0 架构草案

> **provenance:** generated_by=sisyphus, generated_at=2026-10-07T08:52:56Z, commit=017eb9ff370161682a931a40f3f536bba562ab24, source_repo=openclaw/sqlrustgo, branch=develop/v4.1.0, policy=Anti-Fabrication-Policy-v1.0

> **阶段**: DRAFT (NOT YET ALPHA, per `docs/releases/v4.1.0/STAGE.yaml`)
> **日期**: 2026-10-07
> **边界**: 本文定义 v4.1.0 目标架构和门禁入口，不声明实现已完成；缺陷与阻塞以 `docs/releases/v4.1.0/DEFECTS_AND_ISSUES.md` 为准。

## 1. 架构目标

v4.1.0 是 v4.0.0 之后的维护延续线（post-v4.0.0 maintenance continuation，
per `docs/releases/v4.1.0/VERSION_PLAN.md` §1），继承 v4.0.0 的多模型产品契约
（SQL + Vector + Graph + GMP，per `docs/releases/v4.0.0/README.md`），并在
develop/v4.1.0 上完成以下架构性工作：

- 真实 bugfix 前移：V400-05/06/07 cross-model transaction + AuditChain ALCOA+、
  zombie-fix core、workers.push wrapper 恢复、DML/storage 回归修复
  （per `docs/releases/v4.1.0/VERSION_PLAN.md` §2）。
- 5 远端同步与漂移检测工具面：`scripts/sync/5remotes_sync.sh`、
  `scripts/sync/5remotes_drift_check.sh`（per `docs/releases/v4.1.0/README.md` §3.2）。
- 性能与正确性收敛计划：group commit / 并发 flush / PK fast-path 等
  （per `docs/releases/v4.1.0/PERFORMANCE_OPTIMIZATION_PLAN.md` 及同目录
  PERF_* A/B 报告）。
- 事务隔离设计推进（per `docs/releases/v4.1.0/TRANSACTION_ISOLATION_DESIGN_2026-10-04.md`）。
- Parser 覆盖率从实测 60.58% 向 80% 目标收敛（per 根 `README.md` §1）。

## 2. 目标组件

```text
SQL text / MySQL wire / sqlite-like CLI
        |
Parser -> Planner -> Optimizer -> Executor
        |
Catalog / Storage / Transaction / WAL / MVCC
        |
Vector index / Graph store (v4.x 一等公民子系统)
        |
GMP documents / chunks / audit trail / relations / embeddings
        |
Hybrid retrieval / SQL-backed graph projection
```

工作空间 crate 位于 `crates/`（40+ workspace members），集成测试位于 `tests/`。

## 3. 非目标

- 不做通用 MySQL 5.7 替代声明（per 根 `README.md` §0 禁止声明）。
- 不做独立通用向量数据库声明。
- 不做通用 Cypher/图数据库声明。
- 不作 GA 声明：v4.1.0 当前为 DRAFT；v4.0.0 GA 已于 2026-09-30 撤销
  （per `docs/releases/v4.0.0/STAGE.yaml` `ga_promotion_status.verdict`）。

## 4. 进入开发的架构前置条件

- `docs/releases/v4.1.0/VERSION_PLAN.md` 已定义范围与 v4.0.0 delta。
- `docs/releases/v4.1.0/STAGE.yaml` 为阶段 SSOT（当前 DRAFT）。
- `docs/releases/v4.1.0/DEV_PLAN.md` 与 `docs/releases/v4.1.0/design/`
  已承载设计文档（含 `5025_DATABASE_ISOLATION_DESIGN.md`）。
- `docs/releases/v4.1.0/DEFECTS_AND_ISSUES.md` 目录在案
  （P0×6 + P1×7 + P2×7）；backlog 24 条见 `docs/releases/v4.1.0/ISSUES_PLAN.md`。
- Draft gate 仍只表示开发入口准备，不表示实现通过。
