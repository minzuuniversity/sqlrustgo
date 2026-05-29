# v3.5.0 迁移指南

> 本文档说明从 v3.4.0 迁移到 v3.5.0 的必要变更。

## 变更概述

v3.5.0 聚焦于 AI Agent Layer（AI 代理层），主要变更：

- GMP 合规报告自动生成（跨语言翻译）
- AI 偏差调查助手
- LLM 本地推理（Ollama 集成）
- 四路融合检索（RAG v3）

## 破坏性变更

**无破坏性变更**。v3.5.0 完全向后兼容 v3.4.0。

## 配置迁移

### 新增配置项

```toml
[gmp]
ai_report_enabled = true
偏差调查_enabled = true

[gmp.llm]
provider = "ollama"  # 或 "openai"
model = "qwen2.5:7b"
endpoint = "http://localhost:11434"

[gmp.retrieval]
vector_enabled = true
graph_enabled = true
fts_enabled = true
rrf_k = 60
```

### 从 v3.4.0 升级

v3.4.0 配置可直接使用，新增配置项有默认值。

## API 变更

### 新增端点

| 端点 | 方法 | 说明 |
|------|------|------|
| `/api/v1/gmp/report/translate` | POST | LLM 模式翻译 |
| `/api/v1/gmp/report/translate/local` | POST | 本地模式翻译 |
| `/api/v1/gmp/investigate` | POST | 偏差调查 |
| `/api/v1/gmp/retrieve` | POST | 四路融合检索 |
| `/health` | GET | 健康检查 |

### 原有端点

无变更。

## 依赖变更

### Rust 版本

- v3.4.0: Rust 1.75+
- v3.5.0: Rust 1.75+（无变化）

### 关键依赖

| 依赖 | v3.4.0 | v3.5.0 | 说明 |
|------|--------|--------|------|
| gmp-api | 新增 | v0.1.0 | AI 报告生成 |
| gmp-llm | 新增 | v0.1.0 | Ollama 集成 |
| tokio | 1.35 | 1.35 | 无变化 |
| serde | 1.0 | 1.0 | 无变化 |

## 回滚方案

如遇到问题，可回滚到 v3.4.0：

```bash
git checkout v3.4.0
cargo build --release
```

## 获取帮助

- Issue: https://github.com/minzuuniversity/sqlrustgo/issues
- 文档: https://github.com/minzuuniversity/sqlrustgo/docs
