## Context

GMP Platform 需要实现四路融合检索：Rule-based、Vector embedding、FTS (BM25)、Graph traversal。Issue #1363 要求集成 GMP Retrieval v3 到 GMP Platform，实现 RRF/Reranker 流水线和 LLM 查询改写。

**当前状态**:
- `gmp-retrieval` crate 已实现 BM25 和基础 RRF fusion
- `vector` crate 提供向量索引 (HNSW, IVF-PQ, Flat)
- `sqlrustgo-compliance-engine` 已完成 Ollama 集成

**约束**:
- P99 查询延迟 < 200ms
- 召回率 > 90%
- 使用 Ollama 本地 LLM (不依赖外部 API)

## Goals / Non-Goals

**Goals:**
- 实现四路检索融合 (Rule + Vector + FTS + Graph)
- 支持自然语言查询 (LLM 查询改写)
- Reranker 二次排序提升精度
- 与 GMP Platform 集成

**Non-Goals:**
- 不实现完整的 Graph traversal (外部 Graph DB 负责)
- 不实现分布式向量索引 (单节点优先)
- 不实现模型训练/微调

## Decisions

### 1. 融合架构：分层流水线

**决策**: Query → Rewriter → Multi-search → RRF Fusion → Reranker → Results

**理由**:
- 分层解耦，每层可独立优化
- 支持 pipeline 并行执行
- 便于添加新的检索源

**替代方案**:
- 并联融合 (同时查询所有源) - 不够灵活
- 串级融合 (一个输出作为下一个输入) - 延迟高

### 2. RRF k-factor 配置

**决策**: k=60 (基于 gmp-retrieval 现有值)

**理由**:
- 现有实现使用 k=60，效果良好
- k 太小导致排名抖动，k 太大概率扁平化

### 3. Vector Embedding 模型

**决策**: Ollama 支持 `nomic-embed-text` 或 `mxbai-embed-large`

**理由**:
- 高效的 embedding 模型
- Ollama 本地运行，无需 API
- 768 维向量，兼容 HNSW

### 4. Reranker 模型

**决策**: Ollama 支持 `cross-encoder` 风格排序

**理由**:
- 本地运行，无需外部服务
- 轻量级排序计算

### 5. 查询改写策略

**决策**: 提示词工程 + Ollama chat completion

**理由**:
- 简单有效，无需额外模型
- 可控的输出格式

## Risks / Trade-offs

| Risk | Mitigation |
|------|------------|
| LLM 改写延迟高 | 500ms timeout，fallback 到原始查询 |
| 向量检索召回率不足 | RRF 多路融合补偿 |
| BM25 占用内存 | LRU 缓存 + 限制索引大小 |
| Ollama 内存占用 | 限制并发请求数 |

## Open Questions

1. Graph traversal 数据源是否已有？还是需要实现？
2. 是否需要支持混合检索 (sparse + dense)？
3. 召回率测试使用什么数据集？
