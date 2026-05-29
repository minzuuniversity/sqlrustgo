# v3.4.0 迁移指南

> 本文档说明从 v3.3.0 迁移到 v3.4.0 的必要变更。

## 变更概述

v3.4.0 聚焦于 GMP Management Suite（GMP 合规管理套件），主要变更：

- GMP Management API（批次记录、电子签名、审计链）
- GMP Retrieval v2（四路检索：BM25 + RRF + Reranker + LLM Chat）
- Device OPC UA 集成
- Rule Engine Visual Editor

## 破坏性变更

**无破坏性变更**。v3.4.0 完全向后兼容 v3.3.0。

## 配置迁移

### 新增配置项

```toml
[gmp]
api_enabled = true
device_enabled = true

[gmp.rule_engine]
enabled = true
visual_editor = true

[gmp.retrieval]
bm25_enabled = true
rrf_k = 60
reranker_model = "qwen3-reranker"
llm_chat_model = "qwen3:8b"
```

## API 变更

### 新增端点

| 端点 | 方法 | 说明 |
|------|------|------|
| `/api/v1/gmp/batch` | CRUD | 批次管理 |
| `/api/v1/gmp/signature` | POST | 电子签名 |
| `/api/v1/gmp/audit` | GET/POST | 审计链管理 |
| `/api/v1/gmp/device` | GET | OPC UA 设备数据 |
| `/api/v1/gmp/rule` | CRUD | 规则引擎 |
| `/api/v1/gmp/dashboard` | GET | 仪表板 |
| `/api/v1/gmp/export` | POST | JSON/PDF 导出 |
| `/api/v1/gmp/search` | POST | 四路融合检索 |

### 原有端点

无变更。

## 回滚方案

如遇到问题，可回滚到 v3.3.0：

```bash
git checkout v3.3.0
cargo build --release
```
