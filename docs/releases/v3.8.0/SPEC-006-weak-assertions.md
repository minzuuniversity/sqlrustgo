# SPEC-006 — G-01 follow-up: 5 weak assertions 修复

<!-- env:blocked:no-ci -->

> **PR Number**: SPEC-006 (G-01 follow-up)
> **PR Title**: Fix 5 weak `assert!(*.is_ok())` assertions (G-01 §2.3.1)
> **Version**: v3.8.0
> **Branch**: `fix/weak-assertions` (从 `6e79e083b` 切出)
> **Auditor**: Claude (claude-macmini, governance-engineer)
> **Created**: 2026-06-03
> **Status**: DRAFT — 待执行

---

## 1. 概述

### 1.1 问题

G-01 Validation Chain §2.3.1 要求**断言具体**。
当前 5 处 `assert!(*.is_ok())` 无错误信息，测试失败时无法定位原因。

### 1.2 5 处 weak assertions（check_validation_chain.sh evidence）

| 文件 | 行号 | 变量名 |
|------|------|--------|
| `tests/stored_proc_catalog_test.rs` | 87 | `create1` |
| `tests/stored_proc_catalog_test.rs` | 108 | `insert_result` |
| `crates/graph/src/store/disk_graph_store.rs` | 661 | `store` |
| `crates/graph/src/sharded_graph.rs` | 403 | `edge_id` |
| `crates/unified-query/src/adapters/graph.rs` | 239 | `results` |

## 2. 设计

### 2.1 修复模式

**From**:
```rust
assert!(result.is_ok());
```

**To**:
```rust
assert!(result.is_ok(), "expected ok, got: {:?}", result);
```

### 2.2 不修改语义

- 仅添加 `, "..."` 错误信息部分
- 不改测试逻辑

## 3. 完成标准

- [ ] 5 处 `assert!(*.is_ok())` 全部带错误信息
- [ ] `check_validation_chain.sh` §2.3.1：5 个 evidence 全部修复
- [ ] `cargo check --tests`：clean
- [ ] PR 合并

## 4. Refs

- SPEC-004 / ADR-009 (G-01 framework)
- Task #2772 (closed)
- TEST_REVIEW_TEMPLATE §2.3.1
