# v3.4.0 迁移指南

> 本文档说明从 v3.3.0 迁移到 v3.4.0 的必要变更。

## 变更概述

v3.4.0 聚焦于 GMP Management Suite（管理套件），主要变更：

- GMP Management API (REST) — Batch/ Audit/ Device/ Signature/ Export/ Dashboard/ RuleEngine
- GMP Retrieval v2 — BM25 + RRF Fusion + Ollama Reranker + LLM Chat
- Trust Visualization CLI
- Workflow V2 集成测试（470+ 测试用例）
- Chaos testing framework (OOM, I/O Error, Crash)
- Crash Simulation & WAL Verification suites

## 破坏性变更

**无破坏性变更**。v3.4.0 完全向后兼容 v3.3.0。

## 配置迁移

### 新增配置项

```toml
[gmp]
api_enabled = true
retrieval_v2_enabled = true

[gmp.rest]
port = 8080

[gmp.retrieval]
bm25_enabled = true
vector_enabled = true
rrf_k = 60
reranker_model = "mxbai-embed-large"
```

### 从 v3.3.0 升级

v3.3.0 配置可直接使用，新增配置项有默认值。

## API 变更

### 新增端点

| 端点 | 方法 | 说明 |
|------|------|------|
| `/api/v1/gmp/batch` | POST | EBR 批次管理 |
| `/api/v1/gmp/audit` | GET/POST | 审计记录 |
| `/api/v1/gmp/device` | GET/POST | 设备管理（OPC UA） |
| `/api/v1/gmp/signature` | POST | 电子签名 |
| `/api/v1/gmp/export` | GET | 数据导出 |
| `/api/v1/gmp/dashboard` | GET | 仪表盘 |
| `/api/v1/gmp/rule` | GET/POST/PUT/DELETE | 规则编辑器 |
| `/api/v1/gmp/retrieve` | POST | GMP 混合检索 v2 |
| `/health` | GET | 健康检查 |

### 原有端点

无变更。

## 依赖变更

### Rust 版本

- v3.3.0: Rust 1.75+
- v3.4.0: Rust 1.75+（无变化）

### 关键依赖

| 依赖 | v3.3.0 | v3.4.0 | 说明 |
|------|--------|--------|------|
| gmp-api | - | 新增 | GMP REST API |
| workflow-v2 | - | 新增 | 工作流引擎 v2 |
| trust-cli | - | 新增 | 可信可视化 CLI |
| tokio | 1.35 | 1.35 | 无变化 |
| serde | 1.0 | 1.0 | 无变化 |

## 回滚方案

如遇到问题，可回滚到 v3.3.0：

```bash
git checkout v3.3.0
cargo build --release
```

## 已知问题

无已知问题。