# v3.3.0 迁移指南

> 本文档说明从 v3.2.0 迁移到 v3.3.0 的必要变更。

## 变更概述

v3.3.0 聚焦于 Trust Infrastructure（可信基础设施），主要变更：

- Workflow V2 Engine
- Provenance Knowledge Graph（溯源知识图谱）
- GMP Evidence Engine（证据引擎）
- Compliance-as-Code Engine（合规即代码引擎）
- WAL Formal Verification（WAL 形式化验证框架）
- MySQL Protocol 修复

## 破坏性变更

**无破坏性变更**。v3.3.0 完全向后兼容 v3.2.0。

## 配置迁移

### 新增配置项

```toml
[workflow]
v2_enabled = true

[provenance]
graph_enabled = true

[evidence]
engine_enabled = true

[compliance]
as_code_enabled = true
```

## API 变更

### 新增端点

| 端点 | 方法 | 说明 |
|------|------|------|
| `/api/v1/workflow/v2` | CRUD | Workflow V2 |
| `/api/v1/provenance` | GET/POST | 溯源图谱 |
| `/api/v1/evidence` | CRUD | 证据引擎 |
| `/api/v1/compliance` | CRUD | 合规规则 |

## 回滚方案

如遇到问题，可回滚到 v3.2.0：

```bash
git checkout v3.2.0
cargo build --release
```
