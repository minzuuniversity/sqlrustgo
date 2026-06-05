# SPEC-007 — G-01 follow-up: 清理 2 个 commented-out tests

<!-- env:blocked:no-ci -->

> **PR Number**: SPEC-007 (G-01 follow-up)
> **PR Title**: Remove 2 commented-out `#[test]` blocks (G-01 §2.4.4)
> **Version**: v3.8.0
> **Branch**: `fix/g01-test-cleanup` (从 `6e79e083b` 切出)
> **Auditor**: Claude (claude-macmini, governance-engineer)
> **Created**: 2026-06-03
> **Status**: DRAFT — 待执行

---

## 1. 概述

### 1.1 问题

G-01 Validation Chain §2.4.4 禁止 commented-out tests。
当前 2 处 `// #[test]` 注释块，违反规则。

### 1.2 2 处 commented-out tests

| 文件 | 行号 | 上下文 |
|------|------|--------|
| `crates/executor/src/window_executor.rs` | 770 | "Empty partition test removed - causes panic" |
| `crates/bench/tests/oltp_test.rs` | 86 | "UPDATE is parsed but not fully executed" |

### 1.3 决策

**删除**而非复活：
- 注释已说明根因（edge-case panic / UPDATE 不完整）
- 这些 issue 应作为独立 issue 跟踪（不藏在代码注释里）
- 复活需要 PR-840 等依赖完成（独立 PR 范围）

## 2. 实施

- 删除 2 个 commented-out test 块
- 保留解释性注释（"Empty partition test removed..."）
- 不修改任何 `#[test]` 实际代码

## 3. 完成标准

- [ ] 2 处 `// #[test]` 块删除
- [ ] `check_validation_chain.sh` §2.4.4 PASS
- [ ] 解释性 `// NOTE:` 注释保留
- [ ] 编译通过
- [ ] PR 合并

## 4. Refs

- SPEC-004 / ADR-009 (G-01 framework)
- Task #2772 (closed)
- TEST_REVIEW_TEMPLATE §2.4.4
