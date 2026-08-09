# SQLRustGo v4.0.0 变更日志

> **状态**: 规划中
> **日期**: 2026-08-08

## v4.0.0-planned

这是面向多模型生产数据库版本的初始规划条目。v4.0.0 只有在 v3.12.0 先证明 GMP 内审检索路径后，才应进入 RC/GA 轨道。

### 已规划内容

- 一等公民 vector column 与 vector index syntax。
- WAL-backed vector storage 和 index rebuild。
- 一等公民 property graph node/edge storage。
- graph traversal query surface。
- SQL、vector、graph 和 GMP audit writes 的 cross-model transaction semantics。
- SQL/vector/graph/GMP data 的 unified backup/restore。
- unified access control and audit。
- 168h multi-model SOAK。

### GA 前禁止的声明

- 没有 WAL-backed vector recovery 时，不得宣称 vector database。
- 没有 WAL-backed graph recovery 时，不得宣称 graph database。
- 没有 cross-model transaction tests 时，不得宣称 multi-model production。

## 版本历史

| 版本 | 日期 | 阶段 | 说明 |
|---|---|---|---|
| v4.0.0 | TBD | PLANNED | Multi-model SQL + Vector + Graph + GMP database |

## 附录：英文原文

> 本附录保留本文件改写前的英文原文，便于追溯历史语义；当前正式阅读与执行口径以上方中文正文为准。

# Changelog -- SQLRustGo v4.0.0

> **Status**: PLANNED
> **Date**: 2026-08-08

## v4.0.0-planned

Initial planning entry for the multi-model production database release.

### Planned

- First-class vector column and vector index syntax.
- WAL-backed vector storage and index rebuild.
- First-class property graph node/edge storage.
- Graph traversal query surface.
- Cross-model transaction semantics for SQL, vector, graph, and GMP audit writes.
- Unified backup/restore across SQL/vector/graph/GMP data.
- Unified access control and audit.
- 168h multi-model SOAK.

### Not Allowed Before GA

- Vector database claim without WAL-backed vector recovery.
- Graph database claim without WAL-backed graph recovery.
- Multi-model production claim without cross-model transaction tests.

## Version History

| Version | Date | Stage | Notes |
|---|---|---|---|
| v4.0.0 | TBD | PLANNED | Multi-model SQL + Vector + Graph + GMP database |
