---
entity_type: concept
confidence: high
domains: [sqlrustgo, gmp, sqlite]
created: 2025-04-17
updated: 2025-04-17
---

# Persist Sqlite Backend

## 是什么

SQLRustGo GMP 的 Stage 1 持久化方案。用 rusqlite 实现 StorageBackend trait，将文档和图数据写入 SQLite 文件。

## 双文件架构

```
~/gmp-qa/
├── gmp.db     # GMP 文档 + embeddings
└── graph.db   # 图节点 + 图边
```

## gmp.db Schema

```sql
CREATE TABLE gmp_documents (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  title         TEXT NOT NULL,
  doc_type      TEXT NOT NULL,
  content       TEXT NOT NULL,
  keywords      TEXT NOT NULL,          -- JSON 数组
  properties    TEXT NOT NULL,          -- JSON 对象 {department, category, chapter, code}
  version       INTEGER DEFAULT 1,
  created_at    INTEGER NOT NULL,
  updated_at    INTEGER NOT NULL,
  effective_date INTEGER,
  status        TEXT DEFAULT 'ACTIVE'
);

CREATE TABLE gmp_embeddings (
  doc_id     INTEGER PRIMARY KEY,
  embedding  BLOB NOT NULL,   -- 1024-dim f32 bge-m3 = 4096 bytes
  updated_at INTEGER NOT NULL
);
```

## graph.db Schema

```sql
CREATE TABLE graph_nodes (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  name        TEXT NOT NULL,
  node_type   TEXT NOT NULL,   -- Department / Category / Chapter / SOP
  properties  TEXT NOT NULL,   -- JSON
  created_at  INTEGER NOT NULL,
  updated_at  INTEGER NOT NULL,
  UNIQUE(node_type, name)
);

CREATE TABLE graph_edges (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  src         INTEGER NOT NULL,
  dst         INTEGER NOT NULL,
  edge_type   TEXT NOT NULL,   -- 归口管理部门 / 属于章节 / 属于类别 / 前置规程
  properties  TEXT NOT NULL,
  created_at  INTEGER NOT NULL,
  UNIQUE(src, dst, edge_type)
);

CREATE INDEX idx_node_name  ON graph_nodes(name);
CREATE INDEX idx_edge_src   ON graph_edges(src);
CREATE INDEX idx_edge_dst   ON graph_edges(dst);
CREATE INDEX idx_node_type  ON graph_nodes(node_type);
```

## StorageBackend Trait

```rust
pub trait StorageBackend: Send + Sync {
    fn init(&self) -> Result<(), String>;
    fn save_document(&self, doc: &DocumentRecord) -> Result<i64, String>;
    fn load_documents(&self) -> Result<Vec<DocumentRecord>, String>;
    fn save_embedding(&self, emb: &EmbeddingRecord) -> Result<(), String>;
    fn load_embeddings(&self) -> Result<Vec<EmbeddingRecord>, String>;
    fn save_node(&self, node: &NodeRecord) -> Result<i64, String>;
    fn load_nodes(&self) -> Result<Vec<NodeRecord>, String>;
    fn save_edge(&self, edge: &EdgeRecord) -> Result<i64, String>;
    fn load_edges(&self) -> Result<Vec<EdgeRecord>, String>;
    fn get_node(&self, node_type: &str, name: &str) -> Result<Option<NodeRecord>, String>;
}
```

## 关键实现细节

### embedding BLOB 存储

```rust
use bytemuck::cast_slice;

// 写入
let blob = cast_slice(&emb.embedding).to_vec();  // f32 → u8
conn.execute("INSERT BLOB ?", params![blob]);

// 读取
let blob: Vec<u8> = row.get("embedding")?;
let emb: Vec<f32> = cast_slice(&blob).to_vec();  // u8 → f32
```

### save_document 的 ON CONFLICT

```rust
INSERT INTO gmp_documents
  (title, doc_type, content, keywords, properties, ...)
  VALUES (...)
  ON CONFLICT(id) DO UPDATE SET
  title=excluded.title, ..., properties=excluded.properties;
```

### Graph Nodes 的唯一约束

```sql
UNIQUE(node_type, name)
-- 即 "Department:QA" 和 "SOP:变更控制程序" 不会冲突
```

## 已知 Bug & Fix

### 修复：save_document 漏写 properties

**症状**：properties 始终为 `{}`，department/category/chapter 过滤不生效

**根因**：INSERT 语句只有 9 列，漏了 `properties` 第10列

**修复**：
```rust
// 修复前
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)  // 漏了 properties

// 修复后
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
properties=excluded.properties,  // ON CONFLICT 也补上
```

## 相关

- [[SQLRustGo GMP CLI]]
- [[Embedding BLOB Storage]]
