# SQLRustGo OO 文档统一索引

> **版本**: v3.6.0
> **更新日期**: 2026-05-29
> **维护人**: hermes-z440

---

## 概述

OO 文档（Operational Oxford — 牛津Operational Excellence运营卓越框架）是 SQLRustGo 项目关于**查询执行、事务管理、存储引擎、查询优化**等技术实现细节的文档集合。

本索引整合了 **v2.0 至 v3.5.0** 共 11 个版本的 OO 文档，提供统一的导航入口。

---

## 文档统计

| 版本 | OO 文档数 | 主要内容 |
|------|----------|----------|
| v2.0 | 1 | Vector Engine + Cascades 架构 |
| v2.6 | 18 | SQL-92 完整支持，各模块架构 |
| v2.7 | 21 | Enterprise Resilience, WAL 崩溃恢复, 混合检索 |
| v2.8 | 4 | Production + Distributed + Secure |
| v2.9 | 14 | Formal Verification Excellence |
| v3.0 | 24 | 完整 SQL 执行链路分析 |
| v3.1 | 10 | MERGE/GAP Lock/聚簇索引/存储加密 |
| v3.2 | 13 | GMP OO 文档（数字签名/电子签名/EBR等） |
| v3.3 | 9 | Trust Infrastructure（性能治理/崩溃模拟/合规引擎） |
| v3.4 | 1 | GMP Management Suite RC |
| v3.5 | 9 | AI Native GMP Platform Alpha |
| **总计** | **145** | - |

---

## OO 模块分类

### 1. 查询执行（Query Execution）

| 模块 | 说明 | 主要版本 |
|------|------|----------|
| 执行引擎架构 | executor 模块设计文档 | v2.6, v2.7, v3.0 |
| JOIN 执行 | Hash Join / Nested Loop Join | v3.0 |
| 聚合执行 | Group By / Aggregate Functions | v3.0 |
| 窗口函数 | Window Function 执行 | v2.9, v3.0 |
| 递归 CTE | WITH RECURSIVE 支持 | v2.9 |

### 2. 存储引擎（Storage Engine）

| 模块 | 说明 | 主要版本 |
|------|------|----------|
| B+ Tree | 索引数据结构 | v3.0 |
| Buffer Pool | 缓冲池管理 | v2.6, v2.7 |
| 聚簇索引 | Clustered Index 设计 | v3.1, v3.2 |
| 列式存储 | Columnar Storage | v2.0 |
| 存储加密 | Storage Encryption | v3.1 |

### 3. 事务管理（Transaction Management）

| 模块 | 说明 | 主要版本 |
|------|------|----------|
| WAL | Write-Ahead Logging | v2.6, v2.7, v2.8, v2.9, v3.0 |
| MVCC | Multi-Version Concurrency Control | v2.6, v2.7, v2.8 |
| GAP Lock | Gap Locking 分析 | v3.1, v3.2 |
| 崩溃恢复 | Crash Recovery | v2.7, v3.0 |

### 4. 查询优化（Query Optimization）

| 模块 | 说明 | 主要版本 |
|------|------|----------|
| CBO | Cost-Based Optimizer | v2.0, v2.6, v2.7, v3.1, v3.2 |
| Cascades | Cascades 优化框架 | v2.0 |
| 规则优化 | Rule-Based Optimization | v2.6 |

### 5. GMP 合规（GMP Compliance）

| 模块 | 说明 | 主要版本 |
|------|------|----------|
| 数字签名链 | Digital Signature Chain | v3.2 |
| 电子签名 | Electronic Signature (21 CFR Part 11) | v3.2 |
| Immutable Record | EBR (Electronic Batch Record) | v3.2 |
| Correction Chain | 纠正链追踪 | v3.2 |
| Provenance Tracking | 数据溯源 | v3.2, v3.3 |
| Trusted Timestamp | 可信时间戳 | v3.2 |
| HSM/KMS 集成 | 硬件安全模块集成 | v3.2 |
| GMP Workflow Engine | 工作流引擎 | v3.2 |

### 6. AI Native（AI 原生）

| 模块 | 说明 | 主要版本 |
|------|------|----------|
| AI Agent 架构 | AI Agent 系统设计 | v3.5 |
| BGE-M3 Embedding | 向量化检索 | v3.5 |
| Compliance Judge | 合规判决 | v3.5 |
| Deviation Investigator | 偏差调查 | v3.5 |
| Ollama 集成 | 本地 LLM 集成 | v3.5 |
| Report Generator | 报告生成 | v3.5 |
| Retrieval v3 | 检索增强 | v3.4, v3.5 |
| SSE Streaming | 服务端推送 | v3.5 |

### 7. 向量化与图（Vector & Graph）

| 模块 | 说明 | 主要版本 |
|------|------|----------|
| Vector Engine | 向量执行引擎 | v2.0 |
| Graph Store | 图存储 | v2.7 |
| 混合检索 | Hybrid Search | v2.7 |

---

## 版本追溯索引

### 按版本快速访问

- [v3.5.0 OO](releases/v3.5.0/oo/) — AI Native GMP Platform Alpha
- [v3.4.0 OO](releases/v3.4.0/oo/) — GMP Management Suite RC
- [v3.3.0 OO](releases/v3.3.0/oo/) — Trust Infrastructure
- [v3.2.0 OO](releases/v3.2.0/oo/) — GMP OO 文档
- [v3.1.0 OO](releases/v3.1.0/oo/) — OO 执行链路补充
- [v3.0.0 OO](releases/v3.0.0/oo/) — 完整 SQL 执行链路
- [v2.9.0 OO](releases/v2.9.0/oo/) — Formal Verification Excellence
- [v2.8.0 OO](releases/v2.8.0/oo/) — Production+Distributed+Secure
- [v2.7.0 OO](releases/v2.7.0/oo/) — Enterprise Resilience
- [v2.6.0 OO](releases/v2.6.0/oo/) — SQL-92 完整支持
- [v2.0 OO](releases/v2.0/oo/) — Vector Engine + Cascades

---

## OO 模块详情

### 执行链路（v3.0.0）

```
v3.0.0/oo/
├── advanced/          # 高级查询特性
├── bptree/           # B+ 树索引
├── cbo/              # CBO 集成
├── coverage/         # 覆盖率分析
├── dcl/              # DCL 执行
├── ddl/              # DDL 执行
├── distributed/      # 分布式执行
├── dml/              # DML 执行
├── execution/        # 执行引擎核心
├── join/             # JOIN 执行
├── query/            # 查询处理
├── recovery/         # 崩溃恢复
├── setops/           # SET Operations
├── transaction/      # 事务管理
├── wal/              # WAL 日志
└── SQL_EXECUTION_MATRIX.md
```

### GMP OO（v3.2.0）

```
v3.2.0/oo/GMP/
├── DIGITAL_SIGNATURE_CHAIN.md   # OO-1: 数字签名审计链
├── ELECTRONIC_SIGNATURE.md      # OO-2: 电子签名 (21 CFR Part 11)
├── IMMUTABLE_RECORD.md          # OO-3: Immutable Record / EBR
├── CORRECTION_CHAIN.md          # OO-4: Correction Chain
├── PROVENANCE_TRACKING.md       # OO-5: Provenance Tracking
├── TRUSTED_TIMESTAMP.md         # OO-6: Trusted Timestamp
├── HSM_KMS_INTEGRATION.md       # OO-6: HSM/KMS 集成
├── GMP_WORKFLOW_ENGINE.md       # OO-9: GMP Workflow Engine
└── README.md                    # GMP OO 索引
```

---

## 快速链接

| 内容 | 位置 |
|------|------|
| API 文档 | [`docs/api/`](api/) |
| 版本文档索引 | [`docs/releases/`](releases/) |
| 治理文档 | [`docs/governance/`](governance/) |
| 架构文档 | [`docs/architecture/`](architecture/) |
| 项目主 README | [`README.md`](README.md) |

---

## 贡献指南

### 添加新的 OO 文档

1. 在对应版本的 `docs/releases/vX.Y.Z/oo/` 目录下创建文档
2. 文档命名规范：`{MODULE}_{TOPIC}.md`
3. 文档头部包含 YAML frontmatter：
   ```yaml
   ---
   id: oo-{module}-{topic}
   title: {模块} - {主题}
   version: vX.Y.Z
   ---
   ```
4. 更新本索引文件

### 更新本索引

本索引由 `scripts/doc/gen_oo_doc.py` 脚本自动生成（规划中）。

---

## 参考

- [SQLRustGo 文档改进计划](../DOC_IMPROVEMENT_PLAN.md)
- [开源文档分析报告](../OSS_DOC_ANALYSIS.md)
- [GMP 平台文档](../gmp/)
- [Trust Infrastructure 文档](../trust-infrastructure/)