# SQLRustGo v4.0.0

> **状态**: 规划中
> **产品目标**: 生产级 SQL + Vector + Graph + GMP 多模型数据库
> **规划日期**: 2026-08-08

v4.0.0 被规划为 SQLRustGo 的多模型生产版本。它把 v3.12.0 中服务于 GMP/RAG 的内部向量和图能力，提升为数据库的一等公民子系统。

## 发布契约

只有所有 gate 都通过后，才允许使用以下 v4.0.0 声明：

> SQLRustGo v4.0.0 是面向受控 SQL、vector、graph 和 GMP knowledge workloads 的生产级多模型数据库。

## 必要基础

v4.0.0 依赖 v3.12.0 先证明以下能力：

- GMP schema and ingestion。
- SQLRustGo-managed embedding persistence。
- 带 rebuildable index 的内部 vector retrieval。
- SQL-backed graph projection。
- ALCOA+ audit controls。
- 168h mixed SOAK。

## 关键文档

| 文档 | 用途 |
|---|---|
| `VERSION_PLAN.md` | 产品范围和工作包 |
| `TEST_PLAN.md` | multi-model gate 和测试矩阵 |
| `GMP_PLATFORM_REQUIREMENTS.md` | GMP-Platform v1.5/v1.6 consumer contract |
| `CHANGELOG.md` | 规划中的版本历史 |

## 不可协商的门禁

- vector storage 必须 WAL-backed。
- graph storage 必须 WAL-backed。
- SQL、vector、graph、GMP audit writes 必须共享同一个 transaction boundary。
- backup/restore 必须能重建 vector 和 graph indexes。
- access control 必须同时覆盖 SQL、vector、graph 和 GMP 路径。
- 168h multi-model SOAK 必须在 GA 前完成。
- GMP-Platform consumer gate 必须覆盖 408、REST/WebUI、audit、CJK 和 upload smoke。

## 附录：英文原文

> 本附录保留本文件改写前的英文原文，便于追溯历史语义；当前正式阅读与执行口径以上方中文正文为准。

# SQLRustGo v4.0.0

> **Status**: PLANNED
> **Product target**: production multi-model SQL + Vector + Graph + GMP database
> **Planning date**: 2026-08-08

v4.0.0 is planned as the multi-model production release. It promotes vector and graph capabilities from v3.12.0 internal GMP/RAG support into first-class database subsystems.

## Release Contract

Allowed v4.0.0 claim, only after all gates pass:

> SQLRustGo v4.0.0 is a production multi-model database for controlled SQL, vector, graph, and GMP knowledge workloads.

## Required Foundation

v4.0.0 depends on v3.12.0 proving:

- GMP schema and ingestion.
- SQLRustGo-managed embedding persistence.
- Internal vector retrieval with rebuildable index.
- SQL-backed graph projection.
- ALCOA+ audit controls.
- 168h mixed SOAK.

## Key Documents

| Document | Purpose |
|---|---|
| `VERSION_PLAN.md` | Product scope and work packages |
| `TEST_PLAN.md` | Multi-model gate and test matrix |
| `GMP_PLATFORM_REQUIREMENTS.md` | GMP-Platform v1.5/v1.6 consumer contract |
| `CHANGELOG.md` | Planned release history |

## Non-Negotiable Gates

- Vector storage must be WAL-backed.
- Graph storage must be WAL-backed.
- SQL, vector, graph, and GMP audit writes must share one transaction boundary.
- Backup/restore must rebuild vector and graph indexes.
- Access control must apply to SQL, vector, graph, and GMP paths.
- 168h multi-model SOAK must complete before GA.
- GMP-Platform consumer gate must cover 408, REST/WebUI, audit, CJK, and upload smoke.
