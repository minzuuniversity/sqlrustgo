---
entity_type: concept
confidence: high
domains: [sqlrustgo, gmp, storage]
created: 2025-04-17
updated: 2025-04-17
---

# Embedding BLOB Storage

## 存储规范

| 字段 | 值 |
|------|-----|
| 模型 | bge-m3 |
| 向量维度 | 1024 |
| 每条大小 | 4096 bytes |
| 存储格式 | SQLite BLOB |
| 表 | `gmp_documents.embedding` |

## Schema

```sql
CREATE TABLE gmp_documents (
    id          INTEGER PRIMARY KEY,
    doc_id      TEXT NOT NULL UNIQUE,
    title       TEXT NOT NULL,
    content     TEXT NOT NULL,
    properties  TEXT NOT NULL DEFAULT '{}',  -- JSON: department/category/chapter/code
    embedding   BLOB,                          -- 4096 bytes, bge-m3 1024-dim float32
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
);
```

## 读写流程

**写入（Python / import_sops.py）：**

```python
# 1. 生成 embedding
response = ollama.embeddings(model="bge-m3", prompt=doc_text)
vec = response["embedding"]  # list[float], 1024 dims

# 2. 转成 bytes（BLOB）
vec_bytes = struct.pack(f"{len(vec)}f", *vec)  # 4096 bytes

# 3. 存入 SQLite
cursor.execute(
    "INSERT INTO gmp_documents (doc_id, title, content, properties, embedding, ...)"
    "VALUES (?, ?, ?, ?, ?, ...)",
    (doc_id, title, content, json.dumps(props), Buffer(vec_bytes), ...)
)
```

**读取（Rust / persist_sqlite.rs）：**

```rust
fn load_embeddings(&self, doc_ids: &[String]) -> Result<Vec<Option<Vec<f32>>>> {
    // 从 gmp_documents.embedding BLOB 读取
    // 返回 Vec<Option<Vec<f32>>>，维度 1024
}
```

## BLOB 序列化格式

- **类型**：`REAL32`（32-bit IEEE 754 float）
- **字节序**：大端（网络序，与 `struct.pack(">f", ...)` 兼容）
- **长度**：固定 1024 × 4 = 4096 bytes
- **对齐**：4-byte 对齐，SQLite BLOB 无对齐问题

## 为什么用 BLOB 而不是 TEXT

| 方案 | 大小 | 解析速度 | 精度 |
|------|------|----------|------|
| TEXT（base64） | 5464 bytes | 需 base64 解码 | float 精度 |
| TEXT（JSON array） | ~8192 bytes | 需 JSON 解析 | float 精度 |
| **BLOB** | **4096 bytes** | **直接内存映射** | **float 精度** |

BLOB 是最佳选择：体积最小、解析最快、无精度损失。

## Stage 1 的向量维度不匹配问题

| 组件 | 向量维度 | 来源 |
|------|----------|------|
| SQLite 存储 | **1024** | bge-m3 |
| Rust `vector_search()` | **3584** | `generate_embedding()` Hash 模型 |

`GmpExecutor::generate_embedding()` 用的是 `HashEmbeddingModel`（输出 3584 维），
与 SQLite 存储的 bge-m3（1024 维）完全不兼容，导致 **vector_score = 0**。

详见：[[Triple Search RRF]]

## 相关

- [[Persist Sqlite Backend]]
- [[Triple Search RRF]]
- [[GMP Data Rebuild]]
