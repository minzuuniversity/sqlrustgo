# SQLRustGo v4.0.0 架构草案

> **provenance:** generated_by=sisyphus, generated_at=2026-10-07T08:52:56Z, commit=017eb9ff370161682a931a40f3f536bba562ab24, source_repo=openclaw/sqlrustgo, branch=develop/v4.1.0, policy=Anti-Fabrication-Policy-v1.0

> **阶段**: DRAFT（`docs/releases/v4.0.0/STAGE.yaml` `ga_promotion_status.actual_status` = "DRAFT — v4.0.0 never legitimately reached GA"；verdict = "REVOKED 2026-09-30"）
> **日期**: 2026-10-07
> **边界**: 本文定义 v4.0.0 目标架构和文档入口，不声明实现已完成，**不构成 GA 声明**（GA 已撤销）；权威阶段状态以 `docs/releases/v4.0.0/STAGE.yaml` 为准。

## 1. 架构目标

v4.0.0 被规划为 SQLRustGo 的多模型生产版本：把 v3.12.0 中服务于 GMP/RAG 的
内部向量和图能力提升为一等公民子系统（per `docs/releases/v4.0.0/README.md`
"产品目标" 与 "发布契约"）：

- 受控 SQL、vector、graph 和 GMP knowledge workloads 的多模型数据库。
- 继承 v3.12.0 已证明的基础：GMP schema and ingestion、SQLRustGo-managed
  embedding persistence、带 rebuildable index 的内部 vector retrieval、
  SQL-backed graph projection、ALCOA+ audit controls、168h mixed SOAK
  （per `docs/releases/v4.0.0/README.md` "必要基础"）。
- 存储/事务层 Phase B 工作面（WAL group commit、shard router、MVCC GC 等），
  设计与验证记录见 `docs/releases/v4.0.0/` 下 PHASE_B_* 与
  V400_02..09 系列文档。

## 2. 目标组件

```text
        SQL / MySQL wire / CLI entry
                  |
          Parser / Planner / Optimizer / Executor
                  |
    Catalog / Storage (B+Tree, pages, WAL group commit)
    Transaction (MVCC, isolation) / Shard router
                  |
    Vector subsystem     Graph subsystem
    (embedding + index)  (SQL-backed projection)
                  |
         GMP schema / audit hash chain / ACL
```

工作空间 crate 位于 `crates/`（含 `vector`、`graph`），设计文档见
`docs/releases/v4.0.0/V400_02_VECTOR_WAL_ACCEPTANCE.md`、
`docs/releases/v4.0.0/V400_03_GRAPH_ACCEPTANCE.md`。

## 3. 非目标

- 不做通用 MySQL 5.7 替代声明。
- 不做独立通用向量数据库声明。
- 不做通用 Cypher/图数据库声明。
- 在 `docs/releases/v4.0.0/STAGE.yaml` `ga_promotion_status` 修复前，
  不作任何 GA 声明（REVOKED 2026-09-30，阻塞事实与修复路径见
  `docs/releases/v4.0.0/GA_RELEASE_TIMELINE.md`）。

## 4. 进入开发的架构前置条件

- `docs/releases/v4.0.0/VERSION_PLAN.md` 已定义产品范围和工作包。
- `docs/releases/v4.0.0/TEST_PLAN.md` 已定义 multi-model gate 和测试矩阵。
- `docs/releases/v4.0.0/STAGE.yaml` 为阶段 SSOT（含 5 条 blocking_facts）。
- `docs/releases/v4.0.0/CLAIM_DOWNGRADE_MANIFEST.md` 记录 release-claim 边界。
- Draft gate 仍只表示开发入口准备，不表示实现通过。
