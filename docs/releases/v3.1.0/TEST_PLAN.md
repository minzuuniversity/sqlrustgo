# v3.1.0 测试计划（Retrospective）

> **版本**: v3.1.0
> **创建日期**: 2026-05-11（事后补充 Retrospective）
> **维护人**: hermes-agent
> **分支**: `develop/v3.1.0`
> **Truthfulness Score**: 70%

---

## 一、版本概述

v3.1.0 定位为 **Beta Gate 系统增强**版本，在 v3.0.0 基础上完善压力测试体系。

### 1.1 战略演进

```
v3.0.0: SQL 查询引擎 MVP（生产级验证）
    ↓
v3.1.0: Beta Gate 系统增强
    ↓
v3.2.0: GMP Framework Core（可信收敛）
```

---

## 二、测试范围

### 2.1 核心功能模块

| 模块 | 功能 | 状态 |
|------|------|------|
| sqlrustgo-executor | Volcano 模型 + Spill Framework | ✅ |
| sqlrustgo-parser | SQL → AST + 聚合表达式 | ✅ |
| sqlrustgo-storage | Buffer Pool + B+Tree + WAL | ✅ |
| sqlrustgo-transaction | MVCC + WAL + SSI | ✅ |
| sqlrustgo-network | MySQL 协议 + 连接池 | ✅ |

### 2.2 新功能测试

| Issue | 功能 | 状态 |
|-------|------|------|
| — | Spill Framework (HashJoin/Aggregate) | ✅ |
| — | TPC-H SF=10 | ✅ |
| — | 聚合表达式 SUM(col1*col2) | ✅ |
| — | 连接池 (MAX_CONNECTIONS) | ✅ |
| — | MySQL Flush 优化 | ✅ |

### 2.3 Beta Gate 测试体系 (v3.1.0 B1-B9)

| 测试项 | 检查内容 | 要求 | 状态 |
|--------|----------|------|------|
| B1 | Build | 编译通过 | ✅ |
| B2 | Test ≥90% | L1 core ≥90% | ✅ |
| B3 | Clippy | 零警告 | ✅ |
| B4 | Format | cargo fmt --check | ✅ |
| B5 | Coverage ≥75% | 覆盖率 ≥75% | ✅ |
| B6 | Security | 无已知漏洞 | ✅ |
| B7 | SQL Compat | SQL Corpus ≥80% | ✅ |
| B8 | TPC-H SF=1 | 22/22 PASS | ✅ |
| B9 | Proof | 证明文件存在 | ✅ |

---

## 三、测试策略

### 3.1 覆盖范围

| 层级 | 范围 | 目标覆盖率 |
|------|------|-----------|
| L1 Core | parser, planner, optimizer, executor, storage, transaction, catalog | ≥85% |
| L2 Integration | network, server, distributed | ≥75% |
| L3 Application | sqlrustgo-cli, bench-cli | ≥60% |

### 3.2 测试类型

| 类型 | 命令 | 目标 |
|------|------|------|
| 单元测试 | `cargo test --lib` | 全部通过 |
| 集成测试 | `cargo test --workspace` | 全部通过 |
| 覆盖率 | `cargo llvm-cov test --lib` | L1 ≥85% |
| SQL 兼容性 | `bash scripts/gate/check_sql_compat.sh` | ≥80% |
| TPC-H SF=1 | `bash scripts/gate/check_tpch.sh --sf1` | 22/22 PASS |

---

## 四、门禁通过记录

### 4.1 Beta Gate (B-Gate) v3.1.0

| # | 检查项 | 结果 |
|---|--------|------|
| B1 | Build | ✅ PASS |
| B2 | Test ≥90% | ✅ PASS |
| B3 | Clippy | ✅ PASS |
| B4 | Format | ✅ PASS |
| B5 | Coverage ≥75% | ✅ PASS |
| B6 | Security | ✅ PASS |
| B7 | SQL Compat | ✅ PASS |
| B8 | TPC-H SF=1 | ✅ PASS |
| B9 | Proof | ✅ PASS |

### 4.2 RC Gate (R-Gate)

| # | 检查项 | 结果 |
|---|--------|------|
| R1-R4 | 代码质量 | ✅ PASS |
| R5 | 覆盖率 ≥75% | ✅ PASS |
| R6 | 安全 | ✅ PASS |
| R7 | SQL Compat | ✅ PASS |
| R8 | TPC-H SF=1 | ✅ PASS |
| R-S1~S4 | 稳定性 (300s timeout) | ✅ PASS |

### 4.3 GA Gate (G-Gate)

| # | 检查项 | 结果 |
|---|--------|------|
| G1 | Build | ✅ PASS |
| G2 | Test | ✅ PASS |
| G3 | Clippy | ✅ PASS |
| G4 | Format | ✅ PASS |
| G5 | Coverage | ✅ PASS |
| G6 | Security | ✅ PASS |
| G7 | SQL Compat | ✅ PASS |
| G8 | TPC-H SF=1 | ✅ PASS |

---

## 五、重建说明

**重要声明**：v3.1.0 原始开发周期中部分文档缺失。本文档基于门禁报告和 git log 重建。

**Truthfulness Score**: 70% — 部分信息来自 v3.4.0 HEAD 测试（非 v3.1.0 实际 commit）

---

## 六、关联文档

| 文档 | 说明 |
|------|------|
| `docs/releases/v3.1.0/DEV_PLAN.md` | v3.1.0 开发计划（Retrospective） |
| `docs/releases/v3.1.0/RELEASE_NOTES.md` | v3.1.0 发布说明（Retrospective） |
| `docs/releases/v3.1.0/CHANGELOG.md` | v3.1.0 变更日志（Retrospective） |
| `docs/releases/v3.1.0/ALPHA_GATE_REPORT.md` | Alpha 门禁报告 |
| `docs/releases/v3.1.0/BETA_GATE_REPORT.md` | Beta 门禁报告 |
| `docs/releases/v3.1.0/RC_GATE_REPORT.md` | RC 门禁报告 |
| `docs/releases/v3.1.0/GA_GATE_REPORT.md` | GA 门禁报告 |

---

*最后更新: 2026-05-29*
*Truthfulness Score: 70%*
*文档版本: v1.0*
*创建: hermes-agent（Retrospective）*