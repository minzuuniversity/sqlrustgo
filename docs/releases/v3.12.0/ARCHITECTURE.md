# SQLRustGo v3.12.0 架构草案

> **provenance:** generated_by=v3.12.0-remediation-round-3, generated_at=2026-08-10T10:49:33Z, commit=1903545df6d036f7f6d5035a0503b5fa932aac51, source_repo=openclaw/sqlrustgo, branch=develop/v3.12.0, policy=Anti-Fabrication-Policy-v1.0

> **阶段**: DRAFT
> **日期**: 2026-08-09
> **边界**: 本文定义 v3.12.0 目标架构和门禁入口，不声明实现已完成。
**commit**: 1903545df6d036f7f6d5035a0503b5fa932aac51

## 1. 架构目标

v3.12.0 面向 `~/gmp-platform` 的 GMP 内审检索系统，目标是把 SQLRustGo 收敛为一个可审计的数据内核：

- 关系表管理 GMP document、chunk、version、audit、relation 元数据。
- 向量存储和索引由 SQLRustGo 管理，并要求可重建、可验证。
- 图能力限定为 SQL-backed evidence graph projection，不声明通用图数据库。
- RAG 检索必须输出 evidence bundle，包括 source path、version、chunk hash、score components 和 citation text。

## 2. 目标组件

```text
GMP-Platform CLI/API/Eval
        |
        v
SQLRustGo GMP Kernel
        |
        +-- GMP relational schema
        +-- document/chunk/version store
        +-- embedding table + vector index rebuild
        +-- SQL-backed relation graph projection
        +-- hybrid retrieval + RRF
        +-- audit hash chain + ACL
        +-- backup/restore + upgrade verification
```

## 3. 非目标

- 不做通用 MySQL 5.7 替代声明。
- 不做独立通用向量数据库声明。
- 不做通用 Cypher/图数据库声明。
- 不用 GMP-only 测试替代 SQL correctness gate。

## 4. 进入开发的架构前置条件

- `docs/releases/v3.12.0/DEVELOPMENT_PLAN.md` 已定义 V312-01 至 V312-24。
- `docs/releases/v3.12.0/TEST_PLAN.md` 已定义 V312-G1 至 V312-G25。
- 252 Gitea 已创建 `#3887` 总控和 `#3888-#3911` 任务。
- Draft gate 仍只表示开发入口准备，不表示实现通过。
