---
entity_type: concept
confidence: high
domains: [sqlrustgo, gmp, search]
created: 2025-04-17
updated: 2025-04-17
---

# Triple Search RRF

## 架构

三路搜索，RRF（Reciprocal Rank Fusion）融合：

```
query
├── Vector Search (GmpExecutor.vector_search)
│   └── 问题：MemoryStorage 为空，generate_embedding() 用 Hash 向量
├── Graph Search (GraphNeighbors)
│   └── 从 name_to_id 查 InMemoryGraphStore
└── Exact Search (SqlSearch)
    └── 全表扫描 + 文本词匹配

rrf_score = 0.7 * norm(vector_score) + 0.15 * norm(graph_score) + 0.15 * norm(exact_score)
```

## Stage 1 限制

vector_score 始终为 0，因为：

1. `GmpExecutor` 持有 `MemoryStorage`（空的）
2. `vector_search()` 调用 `generate_embedding(content)` 生成**临时向量**（HashEmbeddingModel，3584维）
3. SQLite 中存储的是 **bge-m3 embedding**（1024维，4096 bytes BLOB）
4. 维度不匹配 → 余弦相似度 = 0

## 修复方向

Stage 2 / Stage 3：让 GmpExecutor 感知 SQLite backend，从 SQLite 加载 embedding BLOB 而非用临时 Hash 向量。

## 相关

- [[SQLRustGo GMP CLI]]
- [[Embedding BLOB Storage]]
