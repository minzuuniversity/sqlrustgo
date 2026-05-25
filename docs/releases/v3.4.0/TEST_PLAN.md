# v3.4.0 测试计划

> **版本**: v3.4.0
> **创建日期**: 2026-05-26
> **维护人**: hermes-agent
> **分支**: `develop/v3.4.0`
> **Truthfulness Score**: 100%

---

## 一、版本概述

v3.4.0 定位为 **GMP Management Suite（GMP 管理套件）**，构建完整的 GMP 管理能力。

### 1.1 战略演进

```
v3.3.0: Industrial Trust Platform（工业级可信闭环）
    ↓
v3.4.0: GMP Management Suite（GMP 管理套件）
    ↓
v3.5.0: AI Native GMP Platform
```

---

## 二、测试范围

### 2.1 核心功能模块

| 模块 | 功能 | 状态 |
|------|------|------|
| sqlrustgo-gmp-api | GMP REST API 完整实现 | ✅ |
| sqlrustgo-gmp-retrieval | GMP Retrieval V2 (Rule/Vector/FTS/Graph) | ✅ |
| sqlrustgo-workflow-v2 | 工作流引擎 V2 | ✅ |
| sqlrustgo-evidence-engine | 证据引擎 | ✅ |
| sqlrustgo-compliance-engine | 合规引擎 | ✅ |

### 2.2 新功能

| Issue | 功能 | 状态 |
|-------|------|------|
| — | GMP API Framework | ✅ |
| — | Retrieval V2 (RRF/Reranker) | ✅ |
| — | Graph DB 集成 | ✅ |

---

## 三、测试策略

### 3.1 覆盖范围

| 层级 | 范围 | 目标覆盖率 |
|------|------|-----------|
| L1 Core | parser, planner, optimizer, executor, storage, transaction, catalog | ≥85% |
| L2 Integration | gmp-api, gmp-retrieval, workflow-v2 | ≥75% |
| L3 Application | evidence-engine, compliance-engine | ≥60% |

### 3.2 测试类型

| 类型 | 命令 | 目标 |
|------|------|------|
| 单元测试 | `cargo test --lib` | 全部通过 |
| 集成测试 | `cargo test --workspace` | 全部通过 |
| 覆盖率 | `cargo llvm-cov test --lib` | L1 ≥85% |
| SQL 兼容性 | `bash scripts/gate/check_sql_compat.sh` | ≥80% |
| TPC-H SF=1 | `bash scripts/gate/check_tpch.sh --sf1` | 22/22 PASS |

---

## 四、重建说明

**重要声明**：v3.4.0 原始开发周期中未创建 TEST_PLAN.md（文档缺失已记录在 DOCUMENT_COMPLIANCE_AUDIT.md 中）。

本文档基于实际测试执行结果和 GA_GATE_REPORT.md 重建。

---

*最后更新: {today}*
*Truthfulness Score: 100%*
