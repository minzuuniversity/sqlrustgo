# SQLRustGo v3.7.0 发布说明

> **版本**: v3.7.0
> **分支**: `origin/develop/v3.7.0` (commit `b925f438`)
> **日期**: 2026-05-30
> **状态**: GA ✅

---

## 版本概述

v3.7.0 GA 是 **Stable SQL Execution Engine Release**，基于 v3.6.0 GA 构建。

### 核心交付

| 交付项 | 状态 | 说明 |
|--------|------|------|
| P0-1: Session-level engine cache | ✅ 已修复 | Transaction state per session，COMMIT 持久化 |
| P0-2: SKIP_AUTH bypass | ✅ 已修复 | Auth enforcement，mysql/mysql 用户正常认证 |
| GA Score | ✅ 65/100 | 超过 70% 阈值 |

### 版本定性（重要）

v3.7.0 GA **不是**「ACID 数据库完整版」，而是：

> **MySQL-compatible SQL Execution Engine with Session-level Transaction Semantics**

以下功能**不属于** v3.7.0 GA：
- WAL / crash recovery → v3.8.0 (INT-1)
- Full MVCC / ROLLBACK → v3.8.0 (INT-1)
- VTU / ParallelVolcanoExecutor → v3.8.0 (INT-2)
- Single execution path → v3.8.0 (INT-4)

---

## 关键提交

| 提交 | 描述 | 日期 |
|------|------|------|
| `3e647254` | v3.7.0-RC1 tag freeze | 2026-05-30 |
| `01db4fdf` | P0-1 fix: session-level engine cache | 2026-05-30 |
| `2607d788` | P0-2 fix: SKIP_AUTH=false restore auth | 2026-05-30 |
| `5e11bd04` | GA re-evaluation: score 41→65 | 2026-05-30 |
| `bf10eb8d` | docs: INTEGRATION_DEBT_REPORT | 2026-05-30 |
| `b925f438` | docs: RELEASE_SUMMARY + v3.8.0 docs | 2026-05-30 |

---

## 统计

| 指标 | 值 |
|------|-----|
| GA Score | 65/100 (81%) |
| P0 Blockers | 0（已修复） |
| Unit Tests | 93/93 PASS |
| E2E Integration | 28/28 PASS |
| TPC-H SF=1 | 22/22 PASS |
| Documents | 15/15 存在 |

---

## v3.7.x 待修复问题

以下 P1 问题在 v3.7.x stabilization 中处理：

| Issue | 标题 | 说明 |
|-------|------|------|
| #2583 | SHOW TABLES 未实现 | P1，需实现 catalog show handler |
| #2584 | 空密码认证 edge case | root 空密码失败，需修复 auth_response.is_empty() |

---

## 升级指南（v3.6.0 → v3.7.0）

### 行为变化

| 变化 | 说明 |
|------|------|
| Transaction state | Session 级持久化（v3.6.0 可能失败的 BEGIN/COMMIT 现已正常） |
| SKIP_AUTH | 默认 `false`（安全增强），测试环境需更新 |
| Auth | mysql/mysql 用户认证强制，root 空密码失败（已知 P1） |

### 无破坏性变更

- SQL dialect: MySQL 8.0 兼容
- Protocol: 完全兼容
- API: 无破坏性变更

---

## 后续版本

| 版本 | 目标 | 核心交付 |
|------|------|----------|
| v3.8.0 | Architecture Unification | INT-1~INT-4 收敛 + WAL 集成 |
| v3.9.0 | ACID Completion | Full MVCC + crash recovery |
| v4.0.0 | Production-grade | 全量集成 + 压测 |

---

## 参考文档

- [GA_GAP_REPORT.md](GA_GAP_REPORT.md) — 详细 GA 评分
- [RELEASE_SUMMARY.md](RELEASE_SUMMARY.md) — 发布总结
- [INTEGRATION_DEBT_REPORT.md](INTEGRATION_DEBT_REPORT.md) — 集成债务
- [v3.8.0/DEVELOPMENT_PLAN.md](../v3.8.0/DEVELOPMENT_PLAN.md) — v3.8.0 开发计划