---
entity_type: concept
confidence: high
domains: [sqlrustgo, gmp, architecture]
created: 2025-04-17
updated: 2025-04-17
---

# GmpExecutor

## 架构

`GmpExecutor` 是 GMP 系统的核心执行引擎，封装在 `GmpCliState` 中：

```rust
pub struct GmpCliState {
    gmp: Arc<Mutex<GmpExecutor>>,
    backend: Option<Arc<SqliteBackend>>,
    // ...
}

pub struct GmpExecutor {
    storage: Arc<MemoryStorage>,          // ⬅️ 空！Stage 1 问题根源
    graph: Arc<InMemoryGraphStore>,
    llm: Arc<OllamaLayer>,
    embedding_model: Arc<dyn EmbeddingModel>,
}
```

## Stage 1 的核心限制

**`storage` 是空的 `MemoryStorage`**，所有 vector search 都走 `generate_embedding()` 生成临时向量。

### 问题链路

```
GmpExecutor::triple_search(query)
├── vector_search(query)
│   ├── generate_embedding(query)      # ⬅️ Hash 模型，3584 维
│   └── cosine_similarity(vec_q, vec_d)
│       └── vec_d 从 MemoryStorage 读   # ⬅️ MemoryStorage 为空！
└── graph_search + exact_search
    └── 正常（走 InMemoryGraphStore 和 SQLite）

SQLite gmp_documents.embedding BLOB（1024维 bge-m3）
  └─ 从未被 GmpExecutor 读取！
```

### 为什么 embedding 存了但用不上

| 组件 | 存储位置 | 向量维度 | 用途 |
|------|----------|----------|------|
| `persist_sqlite.rs` | `gmp_documents.embedding` BLOB | 1024（bge-m3） | **长期存储** |
| `GmpExecutor.storage` | `MemoryStorage`（内存） | ？ | **查询时使用** |

Stage 1 的双写逻辑：
- **写入**：同时写 `MemoryStorage`（内存）和 `SqliteBackend`（BLOB）
- **读取**：只从 `MemoryStorage` 读（内存）
- **问题**：进程重启后 `MemoryStorage` 空的，SQLite BLOB 从未被加载到 `MemoryStorage`

## 修复方向

### Stage 2：StorageBackend 可插拔

将 `MemoryStorage` 替换为 `SqliteBackend`，让 `GmpExecutor` 直接从 SQLite 读取 embedding BLOB。

### Stage 3：SQLRustGo 内核

用 `SqliteBackend` 替代 `MemoryStorage + InMemoryGraphStore`，实现真正的持久化 GMP。

## 相关

- [[SQLRustGo GMP CLI]]
- [[Persist Sqlite Backend]]
- [[Embedding BLOB Storage]]
- [[Triple Search RRF]]
