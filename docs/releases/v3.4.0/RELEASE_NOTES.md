# v3.4.0 Release Notes

> **版本**: v3.4.0 GA
> **发布日期**: 2026-05-24
> **分支**: `origin/develop/v3.4.0`
> **状态**: ✅ GA Released

---

## 版本概述

v3.4.0 是 SQLRustGo 的第三个主要 GA 版本，聚焦于 GMP（Graph Memory Protocol）检索能力的全面提升和 Trust Infrastructure 的正式纳入。

---

## 核心变更

### GMP 检索能力提升

- **四通道检索架构**：Rule Engine / Vector Search / FTS5 / Knowledge Graph + RRF Reranking
- **检索质量**：端到端 RAG Pipeline，支持混合检索 + LLM 重排序
- **gmp-api 服务**：新增完整的 GMP API 服务 (Axum + React)
- **gmp-retrieval**：BM25 + 语义混合检索实现

### Trust Infrastructure 纳入

- Provenance Graph（数据溯源图）
- Evidence Engine（证据链引擎）
- Compliance Engine（合规检查引擎）
- WAL Verification（WAL 写前验证）
- Trust Visualization（信任可视化）

### MySQL 5.7 协议兼容

- 新增 `sqlrustgo-mysql-server` crate，支持 MySQL 5.7 wire protocol
- 微信小程序端接入 GMP 服务

---

## 门禁通过记录

| 阶段 | 日期 | 状态 |
|------|------|------|
| Alpha | 2026-04-25 | ✅ PASS (16/16) |
| Beta | 2026-05-10 | ✅ PASS (14/14) |
| RC | 2026-05-20 | ✅ PASS (22/22) |
| GA | 2026-05-24 | ✅ PASS (29/29 + 豁免 G5) |

**豁免记录**: G5 Coverage 82.88% < 85%，批准豁免 EX-v340-002

---

## 升级说明

从 v3.3.0 升级无需数据库迁移。

---

*发布于 2026-05-25*
