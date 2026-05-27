# v3.3.0 测试计划

> **版本**: v1.0
> **创建日期**: 2026-05-25 (历史重建)
> **维护人**: hermes-agent
> **分支**: `develop/v3.3.0`
> **类型**: 历史重建 (Historical Reconstruction)
> **Truthfulness Score**: 85% (部分数据来自 DEV_PLAN 推断，非实际测试执行记录)

---

## 一、版本概述

v3.3.0 定位为 **Industrial Trust Platform（工业级可信闭环平台）**，从 GMP Management Suite 升级而来。

### 1.1 战略演进

```
v3.2.x: 内核修复（覆盖率/性能/MySQL协议）
    ↓
v3.3.0: Trust Infrastructure（可信性工业闭环）
    ↓
v3.4.0: GMP Management Suite
    ↓
v3.5.0: AI Native GMP Platform
```

### 1.2 Trust Infrastructure 四层架构

```
┌─────────────────────────────────────────────────────────┐
│                    Trust Visualization                    │
│        (ComplianceDashboard, EvidenceChainViz)            │
├─────────────────────────────────────────────────────────┤
│                 Compliance Evidence Engine              │
│        (EvidencePackage, AuditManifest, HashProof)      │
├─────────────────────────────────────────────────────────┤
│                    Provenance Graph                      │
│         (Device→Operator→SOP→Batch→Deviation)        │
├─────────────────────────────────────────────────────────┤
│               Trust Infrastructure Kernel                 │
│  Performance Governance │ Crash Simulation │ WAL Formal │
└─────────────────────────────────────────────────────────┘
```

---

## 二、测试范围

### 2.1 核心功能模块

| 模块 | 功能 | 状态 | 说明 |
|------|------|------|------|
| sqlrustgo-perf-baseline | Performance Governance System | ✅ | PR #1243 |
| sqlrustgo-crash-sim | Crash Simulation Framework | ✅ | PR #1244 |
| sqlrustgo-wal-verification | WAL Formal Verification | ✅ | PR #1247 |
| sqlrustgo-compliance-engine | Compliance-as-Code Engine | ✅ | PR #1248 |
| sqlrustgo-evidence-engine | Evidence Engine | ✅ | PR #1249 |
| sqlrustgo-provenance-graph | Provenance Knowledge Graph | ✅ | PR #1250 |
| sqlrustgo-workflow-v2 | Workflow V2 | ✅ | PR #1252 |
| sqlrustgo-trust-viz | Trust Visualization | ✅ | PR #1253 |

### 2.2 内核修复项

| Issue | 功能 | PR | 状态 |
|-------|------|-----|------|
| #1196/#1197 | executor 模块拆分 + 覆盖率 | #1232 | ✅ |
| #1201 | MySQL Protocol 握手修复 | #1234 | ✅ |
| #1229 | TPC-H SF=1 22/22 验证 | — | ✅ |
| #1222 | Coverage SSOT 规范 | #1231 | ✅ |

### 2.3 AI 辅助开发工具

| Issue | 功能 | PR | 状态 |
|-------|------|-----|------|
| #1225 | AI Test Generator | #1230 | ✅ |
| #1226 | Intelligent Test Selection | #1230 | ✅ |
| #1227 | Bug Triage AI | #1230 | ✅ |
| #1228 | Flaky Test Detector | #1230 | ✅ |

---

## 三、测试策略

### 3.1 覆盖范围

| 层级 | 范围 | 目标覆盖率 |
|------|------|-----------|
| L1 Core | parser, planner, optimizer, executor, storage, transaction, catalog | ≥85% |
| L2 Integration | gmp-api, gmp-retrieval, workflow-v2 | ≥75% |
| L3 Application | evidence-engine, compliance-engine, trust-viz | ≥60% |

### 3.2 测试类型

| 类型 | 命令 | 目标 |
|------|------|------|
| 单元测试 | `cargo test --lib` | 全部通过 |
| 集成测试 | `cargo test --workspace` | 全部通过 |
| 覆盖率 | `cargo llvm-cov test --lib` | L1 ≥85% |
| SQL 兼容性 | `bash scripts/gate/check_sql_compat.sh` | ≥80% |
| TPC-H SF=1 | `bash scripts/gate/check_tpch.sh --sf1` | 22/22 PASS |
| 安全扫描 | `bash scripts/gate/check_security.sh` | 0 漏洞 |

---

## 四、重建说明

**重要声明**：本文档为历史重建文档。v3.3.0 原始开发周期中未创建 TEST_PLAN.md（文档缺失已记录在 DOCUMENT_COMPLIANCE_AUDIT.md 中）。

本文档基于以下来源重建：
1. `DEV_PLAN.md` - 计划功能和 Issue 映射
2. `COMPREHENSIVE_STATUS_REPORT.md` - 声称的完成状态
3. `oo/OO_ROADMAP.md` - OO 文档映射

**Truthfulness Score**: 85%（计划数据真实，测试执行数据来自推断）

---

*最后更新: {today}*
*Truthfulness Score: 85%*
*类型: 历史重建*
