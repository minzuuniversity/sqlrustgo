# SQLRustGo v4.0.0 测试计划

> **版本**: v4.0.0
> **状态**: 规划中
> **日期**: 2026-08-08
> **目标**: 生产级 SQL + Vector + Graph + GMP 多模型数据库

## 1. 测试矩阵

| Gate | 领域 | 方法 | 阈值 |
|---|---|---|---|
| V400-G1 | SQL 回归 | workspace build/test/fmt/clippy | 退出码 0，0 warning |
| V400-G2 | Vector SQL | parser + executor + storage E2E | vector insert/search/delete 通过 |
| V400-G3 | 向量恢复 | vector writes 期间 crash | WAL replay 恢复 index/data |
| V400-G4 | 图存储 | node/edge CRUD + traversal tests | graph 结果确定 |
| V400-G5 | 图恢复 | graph writes 期间 crash | WAL replay 恢复 nodes/edges |
| V400-G6 | 跨模型事务 | SQL + vector + graph + audit 同一事务 | commit/rollback 原子化 |
| V400-G7 | 备份恢复 | full multi-model restore | counts、hashes、indexes 相等 |
| V400-G8 | 安全 | ACL 和 audit 绕过测试 | fail closed |
| V400-G9 | 检索质量 | GMP SQL + vector + graph hybrid set | citation 和 path evidence 完整 |
| V400-G10 | 多模型长稳 | 168h mixed production workload | 0 crash，无一致性破坏 |

## 2. 必需 fixtures

| Fixture | 用途 |
|---|---|
| GMP 语料 | document/chunk/audit workload |
| 向量语料 | ANN correctness、recall、latency |
| 图语料 | traversal correctness 和 path expansion |
| 跨模型事务 fixture | SQL row + vector + graph + audit atomicity |
| 崩溃/恢复 fixture | WAL and backup verification |

## 3. 拒绝规则

只要出现以下任一情况，v4.0.0 不得进入 GA：

- vector storage 不是 WAL-backed。
- graph storage 不是 WAL-backed。
- SQL/vector/graph writes 不能参与同一个 transaction boundary。
- backup/restore 不能重建 vector 和 graph indexes。
- access control 只覆盖 SQL，不覆盖 vector/graph 路径。
- graph query support 只是 archived crate，且没有 production revalidation。
- 168h multi-model SOAK 缺失或不完整。

## 附录：英文原文

> 本附录保留本文件改写前的英文原文，便于追溯历史语义；当前正式阅读与执行口径以上方中文正文为准。

# SQLRustGo v4.0.0 Test Plan

> **Version**: v4.0.0
> **Status**: PLANNED
> **Date**: 2026-08-08
> **Target**: production multi-model SQL + Vector + Graph + GMP database

## 1. Test Matrix

| Gate | Area | Method | Threshold |
|---|---|---|---|
| V400-G1 | SQL regression | workspace build/test/fmt/clippy | exit 0, 0 warnings |
| V400-G2 | Vector SQL | parser + executor + storage E2E | vector insert/search/delete 通过 |
| V400-G3 | Vector recovery | crash during vector writes | WAL replay restores index/data |
| V400-G4 | 图存储 | node/edge CRUD + traversal tests | graph 结果确定 |
| V400-G5 | Graph recovery | crash during graph writes | WAL replay restores nodes/edges |
| V400-G6 | 跨模型事务 | SQL + vector + graph + audit 同一事务 | commit/rollback 原子化 |
| V400-G7 | Backup/restore | full multi-model restore | counts, hashes, indexes equal |
| V400-G8 | 安全 | ACL 和 audit 绕过测试 | fail closed |
| V400-G9 | 检索质量 | GMP SQL + vector + graph hybrid set | citation 和 path evidence 完整 |
| V400-G10 | Multi-model SOAK | 168h mixed production workload | 0 crash, no consistency break |

## 2. Required Fixtures

| Fixture | Purpose |
|---|---|
| GMP 语料 | document/chunk/audit workload |
| Vector corpus | ANN correctness, recall, latency |
| Graph corpus | traversal correctness and path expansion |
| 跨模型事务 fixture | SQL row + vector + graph + audit atomicity |
| 崩溃/恢复 fixture | WAL and backup verification |

## 3. Rejection Rules

v4.0.0 must not enter GA if any of these are true:

- vector storage is not WAL-backed.
- graph storage is not WAL-backed.
- SQL/vector/graph writes cannot participate in one transaction boundary.
- backup/restore cannot rebuild vector and graph indexes.
- access control applies to SQL but not vector/graph paths.
- graph query support is only an archived crate with no production revalidation.
- 168h multi-model SOAK is missing or incomplete.
