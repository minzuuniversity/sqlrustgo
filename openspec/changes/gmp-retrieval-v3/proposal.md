## Why

GMP Platform 需要将四路融合检索 (Rule/Vector/FTS/Graph) 集成到生产环境，实现 >90% 召回率和 P99 <200ms 的查询延迟。当前 gmp-retrieval crate 仅实现了 BM25 和基础 RRF fusion，需要扩展为完整的融合检索系统。

## What Changes

- 实现四路检索融合：Rule-based、Vector embedding、FTS (BM25)、Graph traversal
- 扩展 RRF (Reciprocal Rank Fusion) 为可配置的 k-factor 支持
- 集成 reranker 支持二次排序
- 添加 LLM 查询改写 (query rewriting) 支持自然语言查询
- 集成 Ollama 本地 LLM 用于向量化和查询改写
- 提供 GMP Platform API 集成

## Capabilities

### New Capabilities

- `gmp-retrieval-v3`: 四路融合检索引擎，支持 Rule/Vector/FTS/Graph 联合查询
- `query-rewriter`: LLM 驱动的查询改写，将自然语言转为结构化查询
- `reranker`: 基于 LLM 的 reranking 二次排序
- `ollama-vectorizer`: Ollama 集成用于 embedding 生成

### Modified Capabilities

- `gmp-retrieval` (existing): 扩展支持四路融合和 reranker 流水线

## Impact

- **新增 crate**: `sqlrustgo-gmp-retrieval` 扩展
- **修改 crate**: `sqlrustgo-gmp`, `sqlrustgo-vector`
- **API 变更**: 新增 `POST /api/v1/retrieval/search` 端点
- **依赖**: Ollama 本地模型 (embedding + reranker)
- **性能目标**: P99 < 200ms, 召回率 > 90%
