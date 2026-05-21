---
entity_type: overview
confidence: high
domains: [sqlrustgo, gmp, knowledge-graph, embedding, rag]
created: 2025-04-17
updated: 2025-04-17
tags: [system, dashboard, gmp]
---

# GMP 知识管理系统总览

## 系统架构

```
SQLRustGo GMP CLI (Rust)
├── GmpCliState
│   ├── storage: MemoryStorage       # 内存文档存储（空，仅 Stage 1）
│   ├── graph: InMemoryGraphStore   # 内存图存储（559节点/1842边）
│   ├── gmp: GmpExecutor            # 向量搜索（走临时Hash向量，Stage 1限制）
│   ├── name_to_id: HashMap         # "Type:Name" → NodeId 映射
│   └── backend: SqliteBackend      # SQLite 持久化
│
├── gmp.db     → documents(557) + embeddings(1024-dim bge-m3 BLOB)
└── graph.db   → nodes(559) + edges(1842)
```

## 三阶段演进

| 阶段 | 描述 | 状态 |
|------|------|------|
| Stage 1 双写 | MemoryStorage + SQLite 双写，CLI 包裹 | ✅ 完成 |
| Stage 2 可插拔 | StorageBackend trait → 替换 MemoryStorage | 待做 |
| Stage 3 内核 | SQLRustGo 内核替代 SQLite | 待做 |

## 核心数据

- **文档数**：557 SOP
- **图谱节点**：559（Department×10, Category×6, Chapter×8, SOP×535）
- **图谱边**：1842（归口管理部门/属于章节/属于类别/前置规程）
- **embedding**：bge-m3 1024维，存 BLOB（4096 bytes/条）

## CLI 命令

| 命令 | 功能 |
|------|------|
| Ping | 心跳测试 |
| Init | 初始化 schema |
| ImportDoc | 导入文档（双写） |
| SqlSearch | 文本精确匹配 + department/category/chapter 过滤 |
| GraphNeighbors | 图邻居查询（双向：出边+入边） |
| ListNodes | 列出节点 |
| ListEdgeTypes | 列出边类型 |
| TripleSearch | 三路 RRF 融合（vector+graph+exact） |

## 已知限制

1. **TripleSearch vector_score=0**：Stage 1 限制，GmpExecutor 用 MemoryStorage（空），vector_search 走临时 Hash 向量，与存储的 bge-m3 不兼容
2. **CLI 调用必须 stdbuf**：SQLite journal 文件锁竞态，需 `stdbuf -oL -eL` 包裹
3. **GraphNeighbors 节点名乱码**：部分 SOP 邻居显示 "文件: xxx.md" 格式

## 关键文件

- `/home/ai/sqlrustgo/crates/gmp/src/cli.rs` — CLI 命令处理（1049行）
- `/home/ai/sqlrustgo/crates/gmp/src/persist_sqlite.rs` — SQLite 持久化（576行）
- `/home/ai/sqlrustgo/target/debug/sqlrustgo-gmp-cli` — 编译产物（220MB）
- `~/gmp-qa/` — 数据目录

## 调用约定

```bash
export SQLRUSTGO_HOME=~/gmp-qa
stdbuf -oL -eL env SQLRUSTGO_HOME=~/gmp-qa ./target/debug/sqlrustgo-gmp-cli
```
