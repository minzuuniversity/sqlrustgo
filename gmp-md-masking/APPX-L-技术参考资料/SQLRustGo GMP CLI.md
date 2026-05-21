---
entity_type: concept
confidence: high
domains: [sqlrustgo, gmp, graph]
created: 2025-04-17
updated: 2025-04-17
---

# SQLRustGo GMP CLI

## 是什么

Rust 编写的 GMP（药品生产质量管理规范）知识管理系统 CLI 工具，通过 JSON stdin/stdout 协议与外部交互。

## 核心组件

### GmpCliState

```rust
struct GmpCliState {
    storage: Arc<RwLock<MemoryStorage>>,     // 内存文档存储（空，Stage 1）
    graph: Arc<RwLock<InMemoryGraphStore>>,  // 内存图存储（559节点/1842边）
    gmp: GmpExecutor,                        // 向量搜索执行器
    name_to_id: Arc<RwLock<HashMap<String, NodeId>>>,  // "Type:Name" → NodeId
    backend: Arc<SqliteBackend>,              // SQLite 持久化
}
```

### 协议格式

输入（单行 JSON）：
```json
{"cmd":"Ping"}
{"cmd":"SqlSearch","query":"偏差","chapter":"02","top_k":5}
{"cmd":"GraphNeighbors","node_type":"Department","node_name":"QA","top_k":8}
```

输出：
```json
{"success":true,"data":{...},"time_ms":0}
{"success":true,"data":{"center":{...},"neighbors":[...]}}
```

## 命令列表

| 命令 | 说明 | 过滤参数 |
|------|------|---------|
| Ping | 心跳测试 | - |
| Init | 初始化 schema | - |
| ImportDoc | 导入文档 | title, doc_type, content, keywords, properties |
| SqlSearch | 文本精确匹配 | query, department, category, chapter, top_k |
| GraphNeighbors | 图邻居查询 | node_type, node_name, rel_type, top_k |
| ListNodes | 列出节点 | node_type, limit |
| ListEdgeTypes | 列出边类型 | - |
| TripleSearch | 三路 RRF 融合 | query, top_k |

## 调用约定

**必须用 `stdbuf -oL -eL` 包裹**，否则加载 ~/gmp-qa 数据时会超时卡死（SQLite journal 文件锁竞态）。

```bash
export SQLRUSTGO_HOME=~/gmp-qa
(echo '{"cmd":"Ping"}'; sleep 3) | stdbuf -oL -eL env SQLRUSTGO_HOME=~/gmp-qa ./sqlrustgo-gmp-cli
```

## 数据流

```
import_doc (Python)
  → CLI JSON stdin
  → GmpCliState.import_doc()
  → gmp.import_document()  [写入 MemoryStorage]
  → backend.save_document() [写入 gmp.db]
  → backend.save_embedding() [生成 embedding BLOB → gmp.db]

reload_from_backend (CLI 启动时)
  → backend.load_documents() → MemoryStorage
  → backend.load_embeddings() → GmpExecutor
  → backend.load_nodes() → InMemoryGraphStore
  → backend.load_edges() → InMemoryGraphStore
  → 构建 name_to_id HashMap
```

## 编译

```bash
cd /home/ai/sqlrustgo
export PATH="$HOME/.cargo/bin:$PATH"
cargo build -p sqlrustgo-gmp
# 输出: target/debug/sqlrustgo-gmp-cli (220MB)
```

## 相关

- [[Persist Sqlite Backend]]
- [[Graph Neighbors Bidirectional Fix]]
- [[Sqlsearch Chapter Filter Fix]]
