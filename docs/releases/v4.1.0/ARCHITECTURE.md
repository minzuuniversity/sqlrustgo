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

## 2. 组件

### 2.1 实际执行链路（v4.1.0 HEAD 事实）

```text
SQL text / MySQL wire / sqlite-like CLI
        |
Parser -> ExecutionEngine（手写解释器）-> Storage
        |                                    |
        +-- Optimizer（局部接入）             Catalog / Transaction / WAL / MVCC
        |
Vector index / Graph store (v4.x 一等公民子系统)
        |
GMP documents / chunks / audit trail / relations / embeddings
        |
Hybrid retrieval / SQL-backed graph projection
```

**执行者是 `src/engine_select.rs`（9450 行）+ `src/engine_dml.rs`（2478 行）的手写
解释器，不是物理计划执行。**

### 2.2 未接入生产的部分（IAIA-410 F3，#5193 裁决为选项 C）

`crates/planner` 的物理计划层 —— `PhysicalPlan` trait **没有 `execute()` 方法**，
`SeqScanExec::execute` 硬编码 `Ok(vec![])`，`IndexScanExec` 无 `execute`。
`NoOpPlanner` 委托给 `DefaultPlanner`，但 `DefaultPlanner::new()` 的全部外部构造点位于
`crates/planner/tests/`，且 `mysql-server` 从不 import planner。

**因此：**

- 这些算子的单元测试通过率**不构成生产正确性证据** —— 它们跑的是生产不可达的路径
- B2 门禁中它们的通过**不代表**任何用户可见行为被验证
- `crates/planner/src/lib.rs` 的 `// TODO: Add these modules after migration` 表明
  该层被项目自己排在「迁移之后」，**从未承诺为 v4.1.0 的交付物**

`crates/optimizer` 是**部分接入**，不可整体删除。其中真被生产调用的是：

| 模块 | 生产引用 | 性质 |
|---|---|---|
| `decorrelate` | 3 | 真调用（`try_decorrelate` 有埋点） |
| `stats` | 3 | 真调用（`cbo_estimator.rs:209`） |
| `unified_cost` | 2 | 真调用（`engine_builder.rs` 七处构造 `UnifiedCostModel`） |
| `rules` | 2 | ⚠️ **只借 `Expr`/`BinaryOperator` 类型做转换，未调用任何优化规则** |

按「有引用 = 在用」判断会误留 `rules`；按 crate 整体删除会误伤生产路径。详见
`ARCHITECTURE_ROUTE_DECISION_5193.md` §1.3 与 `CORE_PATH_INVENTORY.md`。

### 2.3 工作空间布局

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
