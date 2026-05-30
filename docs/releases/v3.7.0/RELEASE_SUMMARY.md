# v3.7.0 GA 发布总结

> **版本**: v3.7.0  
> **类型**: Stable SQL Execution Engine GA  
> **发布日期**: 2026-05-30  
> **分支**: `origin/develop/v3.7.0` (tag: `v3.7.0-RC1`, commit `3e647254`)  
> **GA Score**: 65/100 (81%)  
> **Auditor**: Hermes Agent

---

## 一、发布定性

### 1.1 真实定位（必须明确）

v3.7.0 GA **不是**「ACID 数据库完整版」，而是：

> **MySQL-compatible SQL Execution Engine with Session-level Transaction Semantics**

| 维度 | v3.7.0 真实状态 |
|------|----------------|
| SQL DDL/DML | ✅ 完整可用 |
| Wire Protocol | ✅ MySQL protocol stable |
| Transaction | ⚠️ Session 级（不是 MVCC 级） |
| WAL / Crash Recovery | ❌ 未实现 |
| Rollback | ⚠️ MVCC stub（记录但不隔离） |
| VTU / SIMD | ❌ 未接入主执行路径 |
| Auth | ✅ 强制认证 |
| Execution Path | ❌ 双路径并存 |

---

### 1.2 GA 包容范围（✅ 包含）

以下功能在 v3.7.0 GA 中**视为稳定**：

| 功能 | 说明 |
|------|------|
| Parser → AST | `sqlrustgo-parser` 完整 |
| DDL (CREATE/ALTER/DROP) | Table / Index / View |
| DML (INSERT/UPDATE/DELETE) | 单表达式 |
| SELECT / CTE / Window Functions | 完整 |
| MySQL Wire Protocol | COM_QUERY / COM_STMT_PREPARE |
| Authentication | mysql/mysql 用户强制认证 |
| Session Transaction State | BEGIN/COMMIT 持久化 |
| E2E Integration Tests | 28 files, 0 failures |

---

### 1.3 GA 排除范围（❌ 不包含）

以下**不是** v3.7.0 GA 的一部分，任何相关问题应归入 v3.8.0：

| 排除项 | 相关 Issue | 计划版本 |
|--------|-----------|----------|
| WAL / Crash Recovery | INT-1 (#2588) | v3.8.0 |
| Full MVCC Rollback | INT-1 | v3.8.0 |
| DML 经过 TransactionManager | INT-1 | v3.8.0 |
| ParallelVolcanoExecutor 接入 | INT-2 (#2589) | v3.8.0 |
| expr crate 收敛 | INT-3 (#2590) | v3.8.0 |
| mysql-server 统一执行路径 | INT-4 (#2591) | v3.8.0 |
| execution_engine.rs 拆分 | #2597 | v3.8.0 |
| SHOW TABLES | P1 | v3.7.x |
| 空密码认证 | P1 | v3.7.x |
| 覆盖率测量统一 | #2596 | v3.7.x |

---

## 二、v3.7.0 交付物清单

### 2.1 核心交付

| 交付物 | 位置 | 状态 |
|--------|------|------|
| GA Gap Report | `docs/releases/v3.7.0/GA_GAP_REPORT.md` | ✅ |
| Integration Debt Report | `docs/releases/v3.7.0/INTEGRATION_DEBT_REPORT.md` | ✅ |
| Integration Readiness Report | `docs/releases/v3.7.0/INTEGRATION_READINESS_REPORT.md` | ✅ |
| Integration Status | `docs/releases/v3.7.0/INTEGRATION_STATUS.md` | ✅ |
| Feature Matrix | `docs/releases/v3.7.0/FEATURE_MATRIX.md` | ✅ |
| Test Plan | `docs/releases/v3.7.0/TEST_PLAN.md` | ✅ |
| Version Plan | `docs/releases/v3.7.0/VERSION_PLAN.md` | ✅ |
| Changelog | `docs/releases/v3.7.0/CHANGELOG.md` | ✅ |
| Release Notes | `docs/releases/v3.7.0/RELEASE_NOTES.md` | ✅ |
| Performance Targets | `docs/releases/v3.7.0/PERFORMANCE_TARGETS.md` | ✅ |

### 2.2 关键 Commits

| Commit | 描述 | 类型 |
|--------|------|------|
| `3e647254` | v3.7.0-RC1 tag freeze | freeze |
| `01db4fdf` | P0-1: session-level engine cache | P0 fix |
| `2607d788` | P0-2: SKIP_AUTH=false | P0 fix |
| `5e11bd04` | GA re-evaluation: 41→65 | docs |
| `af886c46` | v3.6.0→v3.7.0 merge | integration |

### 2.3 Issue 状态

| 类别 | 数量 | 状态 |
|------|------|------|
| P0 Blockers | 0 | ✅ All Fixed |
| P1 Issues | 4 | Open → v3.7.x |
| INT Issues | 4 | Open → v3.8.0 |
| P2 Tech Debt | 6 | Open → v3.8.0 |

---

## 三、版本战略定位

### 3.1 版本路线图

```
v3.6.0  GA — Stable Baseline
v3.7.0  GA — Stable Execution Engine (Session Txn)
v3.8.0  Alpha/Beta — Architecture Unification
v3.9.0  Alpha/Beta — ACID Completion
v4.0.0  GA — Production-grade Database Engine
```

### 3.2 各版本核心目标

| 版本 | 类型 | 核心交付 | GA 包容定义 |
|------|------|----------|-------------|
| v3.7.0 | Stable Engine | Session txn + Auth | SQL engine 可用性 |
| v3.8.0 | Arch Unification | INT-1~INT-4 收敛 | 执行路径统一 |
| v3.9.0 | ACID Completion | WAL + MVCC | 真实数据库语义 |
| v4.0.0 | Production | 全量集成 + 压测 | 完整数据库 |

---

## 四、已知限制（必须文档化）

### 4.1 INT-1: DML 不经过 WAL

**状态**: ❌ Open (v3.8.0)

**影响**: 无 crash recovery。任何非预期进程终止均导致未刷盘数据丢失。

**Workaround**: 应用层定期 `FLUSH TABLES` 或在非预期关闭后手动检查数据一致性。

---

### 4.2 ROLLBACK MVCC Stub

**状态**: ⚠️ Stub

**影响**: `ROLLBACK` 不会实际隔离未提交数据。并发事务可能出现脏读。

**Workaround**: v3.8.0 完成 MVCC 前，避免并发未提交写入。

---

### 4.3 双执行路径

**状态**: ❌ 未统一

**影响**: `mysql-server` → `ExecutionEngine` → `MemoryStorage` 与 `bench-cli` → `LocalExecutor` 行为可能不一致。

**Workaround**: 使用 `mysql-server` 作为唯一生产入口。

---

## 五、升级指南（v3.6.0 → v3.7.0）

### 5.1 兼容变化

| 变化 | 说明 |
|------|------|
| Transaction state | Session 级持久化（行为改善） |
| SKIP_AUTH | 默认 `false`（安全增强） |
| Auth | `mysql/mysql` 认证强制 |

### 5.2 行为变化

* **Transaction**: v3.6.0 的 `BEGIN`/`COMMIT` 可能失败 → v3.7.0 正常
* **Auth**: `SKIP_AUTH=true` 的测试环境需更新认证配置

### 5.3 无破坏性变更

* SQL dialect 兼容 MySQL 8.0
* Protocol 完全兼容
* 无 API 破坏性变更

---

## 六、后续行动

| 优先级 | 行动 | 负责人 |
|--------|------|--------|
| P0 | 发布 v3.7.0 tag | 人工 |
| P1 | 处理 SHOW TABLES（P1） | v3.7.x |
| P2 | 处理空密码 auth edge case | v3.7.x |
| P2 | 创建 `develop/v3.8.0` 分支 | 人工 |
| P3 | 覆盖率测量统一 | v3.8.0 |

---

## 七、最终判定

> **✅ v3.7.0 GA 发布 — Stable Execution Engine Release**  
> **❌ 不适用于 ACID 数据库完整语义**  
> **📋 明确 scope 边界，防止 GA 语义误判**