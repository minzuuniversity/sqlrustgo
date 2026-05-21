# SPEC: Vector Channel 修复

## 现状分析

### 问题根因

`VectorChannel::search()` 获取了 embedding 但永远返回 `vec![]`：
```rust
// channel.rs:175-188
pub async fn search(&self, query: &str, top_k: usize) -> Vec<ScoredChunk> {
    match self.get_embedding(query).await {
        Ok(embedding) => {
            let _ = embedding;
            vec![]  // ← 问题：没有实际查询向量数据库
        }
        Err(_) => vec![],
    }
}
```

### 根因分析

1. `gmp-storage` 没有向量存储表（只有 `gmp_document_chunks` + FTS）
2. `gmp-storage` 没有 `search_vectors()` 方法
3. `FourChannelEngine` 的 `VectorChannel` 接收 `ollama_url` 和 `embedding_model`，但无法连接到底层存储

### 现状：gmp-storage 表结构

| 表名 | 用途 |
|------|------|
| `gmp_documents` | 文档元数据 |
| `gmp_document_chunks` | chunk 内容 |
| `gmp_chunks_fts` | FTS5 全文索引 |
| ❌ 无向量表 | — |

## 需求分析

### 功能边界

1. **向量存储**: 在 `gmp-storage` 中添加 `gmp_embeddings` 表，存储 chunk 向量
2. **向量索引**: 添加 `gmp_embeddings` 的 HNSW 索引（或使用内积相似度）
3. **VectorChannel 搜索**: 实现 `top_k` 近邻搜索，返回 `ScoredChunk`

### 验收标准

| 条件 | 验证方法 |
|------|---------|
| `gmp-storage` 有向量存储 | `CREATE TABLE gmp_embeddings` 存在 |
| VectorChannel 返回非空 | `cargo run -- search "偏差分级" --channel vector` 返回结果 |
| 向量搜索有实际排序 | 结果按相似度降序排列 |

## 实现方案

### 方案 A: 简化实现（本次采用）

使用 SQLite 内置的近似近邻搜索：

1. **添加 `gmp_embeddings` 表**：
```sql
CREATE TABLE gmp_embeddings (
    chunk_id INTEGER PRIMARY KEY REFERENCES gmp_document_chunks(id),
    embedding BLOB NOT NULL  -- binary f32 vector
);
CREATE INDEX idx_embeddings_chunk ON gmp_embeddings(chunk_id);
```

2. **实现 cosine similarity 搜索**（Python-style伪代码）：
```sql
-- 对所有嵌入向量计算内积，返回 top_k
SELECT chunk_id, doc_id, content,
       dot(embedding, query_embedding) as score
FROM gmp_embeddings e
JOIN gmp_document_chunks c ON e.chunk_id = c.id
ORDER BY score DESC
LIMIT top_k;
```

3. **在 `index_documents()` 中生成向量**：
   - 调用 Ollama `/api/embeddings` 获取所有 chunk 的向量
   - 存储到 `gmp_embeddings` 表

### 方案 B: 完整 HNSW（未来）

使用 `sqlean-org/sqlite-vector` 或 `vectordb-rs`，需要较大改动。

**本次采用方案 A**。

## 工作量估算

| 任务 | 文件 | 复杂度 |
|------|------|--------|
| 添加 `gmp_embeddings` 表 schema | `gmp-storage/src/storage.rs` | 低 |
| 添加 `index_embeddings()` 批量生成向量 | `gmp-storage/src/storage.rs` | 中 |
| 添加 `search_vectors()` 相似度搜索 | `gmp-storage/src/storage.rs` | 中 |
| 修复 VectorChannel.search() | `gmp-retrieval/src/channel.rs` | 低 |

**总计**: 约 150 行代码
