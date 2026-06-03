# SPEC-005 — G-01 follow-up: 38 `#[ignore]` 理由补充

<!-- env:blocked:no-ci -->

> **PR Number**: SPEC-005 (G-01 follow-up)
> **PR Title**: Add reasons to 38 `#[ignore]` tests (G-01 §2.4.2)
> **Version**: v3.8.0
> **Branch**: `fix/ignore-test-reasons` (从 `6e79e083b` 切出)
> **Auditor**: Claude (claude-macmini, governance-engineer)
> **Created**: 2026-06-03
> **Status**: DRAFT — 待执行

---

## 1. 概述

### 1.1 问题

G-01 Validation Chain (SPEC-004 / ADR-009) 初始运行发现 5 个 FAIL，其中
**§2.4.2 `#[ignore]` 必须有理由** 失败最严重：**38 个 `#[ignore]` 缺理由**。

### 1.2 现状（基线 6e79e083b）

| 类别 | 数量 | 文件 |
|------|------|------|
| Long-run stability (72h) | 4 | `tests/long_run_stability_72h_test.rs` |
| Long-run stability (24h) | 11 | `tests/long_run_stability_test.rs` |
| QPS benchmark | 10 | `tests/qps_benchmark_test.rs` |
| Boundary edge-case | 3 | `tests/boundary_test.rs` |
| WAL contract | 1 | `tests/wal_tx_contract_test.rs:549` |
| Parallel executor | 2 | `crates/executor/src/parallel_executor.rs` |
| Vector (parallel_knn) | 2 | `crates/vector/src/parallel_knn.rs` |
| Vector (hnsw) | 4 | `crates/vector/src/hnsw.rs` |
| Parser | 1 | `crates/parser/src/parser.rs:4136` |
| Storage mmap | 1 | `crates/storage/src/mmap_vector_store.rs:283` |
| **总计** | **38** | 10 个文件 |

### 1.3 调研发现

- 多个文件有 `// inline comment` 跟随 `#[ignore]`（应作为 reason 来源）
- 部分 `#[ignore]` 无任何上下文（需用 default reason）

## 2. 设计

### 2.1 Reason 分类策略

| 来源 | Strategy |
|------|----------|
| 已有 inline `// comment` | 提取并使用 |
| 文件级 pattern | 按文件批量 default |
| 特定测试 | SPECIFIC_REASONS map |

### 2.2 生成的 Reason 格式

```
#[ignore = "<reason text>"]
```

- 最长 80 字符（截断）
- 不带 "TODO" / "FIXME" 前缀
- 包含触发条件或追踪上下文

### 2.3 不修改语义

- 不改变测试逻辑
- 不改变 `#[test]` 位置
- 仅添加 `= "..."` 部分

## 3. 实施

3.1 Python 脚本批量处理 38 处
3.2 验证：
   - `bash scripts/gate/check_validation_chain.sh` 重跑 §2.4.2
   - 期望：0 FAIL（38/38 都有 reason）
3.3 保留所有原 `// comment` 内容（无信息丢失）

## 4. 完成标准

- [ ] 38 个 `#[ignore]` 全部有 `= "..."` 理由
- [ ] `check_validation_chain.sh` 重跑：§2.4.2 PASS
- [ ] 编译通过（`cargo check --tests`）
- [ ] 不修改任何测试逻辑
- [ ] PR 合并

## 5. Refs

- Parent: SPEC-004 / ADR-009
- Task: #2772 (closed)
- G-01: `docs/governance/TEST_REVIEW_TEMPLATE.md` §2.4.2
