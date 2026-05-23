# GMP 合规性检索系统 - 算法实现报告

> **分支**: `feature/v110-sqlrustgo-integration`  
> **基准**: `origin/develop/v1.1.0`  
> **提交**: `8362016`  
> **日期**: 2026-05-22  
> **PR**: [#72](http://192.168.0.252:3000/openclaw/GMP-Platform/pulls/72)

---

## 1. 概述

本 PR 为 GMP 合规性检索系统 v1.1.0 引入 sqlrustgo v3.2.0 集成，包含 MySQL 协议服务器、Cypher 图查询引擎和 HNSW 向量索引。

### 1.1 新增/修改文件统计

| 文件 | 变化 | 说明 |
|------|------|------|
| `crates/gmp-api/` (new) | +194 行 | REST API 服务器（actix-web） |
| `crates/gmp-server/` (new) | +202 行 | MySQL 协议服务器（sqlrustgo） |
| `crates/gmp-storage/src/cypher_engine.rs` (new) | +267 行 | Cypher 图查询引擎 |
| `crates/gmp-storage/src/hnsw_vector_index.rs` (new) | +144 行 | HNSW 向量索引 |
| `crates/gmp-storage/src/sqlrustgo_adapter.rs` (new) | +203 行 | sqlrustgo 存储适配器 |
| `crates/gmp-ner/` (deleted) | -697 行 | 废弃的 NER 模块 |
| `Cargo.toml` | +7 行 | workspace 添加 actix-web，移除 gmp-ner |
| `crates/gmp-storage/Cargo.toml` | +9 行 | rusqlite 升级到 0.39 |
| `crates/gmp-storage/src/storage.rs` | +2 行 | i64 类型修复 |
| `Cargo.lock` | +4863 行 | sqlrustgo 依赖链 |
| **合计** | **+4941 / -1408 行** | |

---

## 2. 核心模块实现

### 2.1 sqlrustgo_adapter.rs - 存储适配器

sqlrustgo v3.2.0 存储引擎适配器，负责将 GMP 平台的文档存储桥接到 sqlrustgo 的 MVCC 存储引擎。

**关键设计**：
```rust
// 文档→sqlrustgo 存储映射
pub struct SqlRustGoAdapter {
    engine: Arc<SqlRustGoStorage>,
    config: SqlRustGoConfig,
}

impl SqlRustGoAdapter {
    // 批量写入文档（带事务）
    pub async fn batch_insert(&self, docs: Vec<Document>) -> Result<usize> {
        let mut tx = self.engine.begin_write().await?;
        for doc in docs {
            tx.insert_document(&doc)?;
        }
        tx.commit().await
    }

    // 语义检索（通过 sqlrustgo 的全文索引）
    pub async fn semantic_search(&self, query: &str, top_k: usize) -> Result<Vec<ScoredChunk>> {
        // 使用 sqlrustgo 的 FTS5 引擎
        self.engine.full_text_search(query, top_k).await
    }
}
```

**sqlrustgo 依赖链**（从 `sqlrustgo-mysql-server` 传递引入）：
- `sqlrustgo-server` → `sqlrustgo-executor` → `sqlrustgo-planner` → `sqlrustgo-optimizer`
- `sqlrustgo-storage` (v3.0.0, 核心存储引擎)
- `sqlrustgo-transaction` (MVCC SSI)
- `sqlrustgo-gmp` (GMP 专用算子)
- `sqlrustgo-distributed` (分布式支持)
- 第三方依赖：`rusqlite 0.39`、`tokio`、`tracing`、`lz4`、`zstd`

### 2.2 cypher_engine.rs - Cypher 图查询引擎

为 GMP 图谱查询实现 OpenCypher 子集，支持多跳关系推理。

**支持的 Cypher 语法**：
```rust
// 支持的模式
MATCH (a:Document)-[:RELATES_TO]->(b:Section) 
WHERE a.id = 'xxx' 
RETURN b

MATCH (doc:Document)-[:CONTAINS]->(chunk:Chunk)<-[:CITES]-(ref:Reference)
WHERE chunk.content CONTAINS '偏差'
RETURN doc, ref
```

**查询计划**：
```
Cypher Query
    │
    ├─► Pattern Analyzer ──► AST (Abstract Syntax Tree)
    │                            │
    │                      ┌────┴────┐
    │                 Clauses     Expressions
    │                 (MATCH,      (WHERE, RETURN,
    │                  WHERE,       ORDER BY)
    │                  RETURN)
    │
    ├─► Graph Index Lookup (via sqlrustgo_adapter)
    │
    └─► Result Formatter
```

**实现统计**：
- 行数：267 行
- 支持 Clauses：MATCH, WHERE, RETURN, ORDER BY, LIMIT
- 支持 Patterns：节点、`-[:REL]->` 单边关系、`-[:REL*1..3]->` 可变长度路径
- 支持 Functions：`count()`, `collect()`, `exists()`

### 2.3 hnsw_vector_index.rs - HNSW 向量索引

Hierarchical Navigable Small World 图算法实现，支持高维向量近似最近邻搜索。

**核心参数**：
```rust
pub struct HnswConfig {
    pub m: usize,           // 每个节点的最大连接数（默认 16）
    pub ef_construction: usize,  // 构造时搜索范围（默认 200）
    pub ef_search: usize,         // 搜索时搜索范围（默认 100）
    pub level_cap: usize,         // 最大层级（默认 6）
}
```

**索引结构**：
```
Level 5 (top):     [Node A] ────── [Node B]
                         │
Level 4:          [Node A] ── [C] ── [Node B]
                         │
Level 3:     [D] ── [A] ── [C] ── [B] ── [E]
             ...继续向下扩展直到 Level 0
```

**插入算法**（简化）：
1. 随机确定新节点层级 `L`（指数衰减概率）
2. 从顶层（`L_max`）开始，找到最接近的节点
3. 沿下层链下降，每层用 `ef_construction` 搜索最近邻
4. 更新所有层的连接边（`m` 个最近邻）

**搜索算法**：
1. 从顶层入口节点开始
2. 贪心搜索当前层最近邻
3. 降至下一层，重复直到 Level 0
4. 用 `ef_search` 精排 Top-K

**性能指标**（基于 391 场景测试）：
- 向量通道通过率：**0/89**（所有 FTS 通过的场景均未启用向量）
- 原因：当前实现中 `vector_weight = 1.0`，但 `RetrievalEngine` 的 `semantic_search` 尚未连接到实际向量索引

### 2.4 gmp-server - MySQL 协议服务器

基于 `sqlrustgo-mysql-server` 的 GMP 数据服务，暴露 MySQL 兼容接口。

**架构**：
```
MySQL Client (mysql CLI / JDBC)
        │
        ▼
sqlrustgo-mysql-server (port 3306)
        │
        ├─► MySQL Protocol Parser
        │         └─► sqlrustgo SQL Parser
        │
        └─► Query Engine
                  ├─► Planner
                  ├─► Optimizer (CBO)
                  └─► Executor
                            │
                      ┌─────┴─────┐
                      │ GMP算子   │
                      │ (MVCC)   │
                      └───────────┘
```

**启动配置**（`config/default.toml`）：
```toml
[database]
path = "/var/data/gmp.db"

[server]
host = "0.0.0.0"
port = 3306

[gmp]
enable_graph = true
enable_vector = true
```

### 2.5 gmp-api - REST API 服务器

基于 `actix-web` 的 RESTful API，为前端和第三方系统提供 GMP 检索接口。

**端点**：
| Method | Path | 说明 |
|--------|------|------|
| GET | `/health` | 健康检查 |
| POST | `/api/v1/search` | 检索（支持 FTS/Vector/Cypher） |
| GET | `/api/v1/document/{id}` | 获取文档 |
| GET | `/api/v1/stats` | 索引统计 |

**请求示例**：
```bash
curl -X POST http://localhost:8080/api/v1/search \
  -H "Content-Type: application/json" \
  -d '{"query": "偏差调查处理流程", "top_k": 10}'
```

---

## 3. 测试结果

### 3.1 391 场景测试（FTS 通道）

| 指标 | 值 |
|------|-----|
| 总场景 | 408 |
| 通过 | 89 |
| 失败 | 319 |
| **通过率** | **21.8%** |
| 平均召回率 | 0.193 |
| 平均 MRR | 0.106 |
| FTS 通过 | 89 |
| Vector 通过 | 0 |

**按类别通过率**（Top 5）：

| 类别 | 通过 | 总数 | 通过率 |
|------|------|------|--------|
| CAPA生成 | 10 | 10 | **100%** |
| 偏差登记工具 | 10 | 10 | **100%** |
| 法规对比 | 8 | 10 | 80% |
| 法规对比扩展 | 6 | 10 | 60% |
| 文档管理 | 5 | 10 | 50% |

**0% 通过率类别**（待优化）：

| 类别 | 总数 | 失败原因 |
|------|------|----------|
| 部门角色检索 | 8 | 需图索引支持 |
| 多跳因果推理 | 8 | Cypher 未集成到主检索流程 |
| 交叉场景 | 10 | 多通道融合权重问题 |
| CAPA关联 | 4 | 需多步推理链 |
| 投诉vs召回 | 5 | 语义相似度不足 |

### 3.2 50 场景深度评估（evaluation_results.json）

| 指标 | 值 |
|------|-----|
| 总场景 | 50 |
| 通过 | 44 |
| 失败 | 6 |
| **通过率** | **88%** |
| 平均延迟 | 1049.58 ms |

**失败案例**（6 个）：

| 类别 | 问题 | 召回文档 | Hit Rate |
|------|------|----------|----------|
| 图谱查询 | 生产清场管理要求 | 清场管理规程, 防混淆管理规程 | 0.0 |
| 验证与确认 | 计量器具校准周期 | 电子秤校准步骤, 计量器具管理 | 0.0 |
| 自检与审计 | 数据完整性 ALCOA 原则 | ALCOA原则, 数据完整性管理规程 | 0.0 |
| 物料管理 | 原辅料接收检验标准 | 物料验收规程, 原料入库验收规程 | 0.0 |
| 物料管理 | 麻醉精神药品管理 | 麻醉精神药品管理规程 | 0.0 |
| 物料管理 | 包装材料验收标准 | 物料验收规程 | 0.0 |

**分析**：失败案例的召回文档在语义上与查询相关，但文档 ID 匹配失败。这表明当前评估使用精确 ID 匹配（`doc_id` 字符串相等），而实际召回的相关文档可能使用不同的 ID 格式。

---

## 4. 已知问题与限制

### 4.1 向量通道未启用

**症状**：391 测试中 `vector_passed = 0`，所有向量检索场景均失败。

**根因**：
1. `RetrievalEngine`（`gmp-retrieval/src/engine.rs`）仅为占位结构，未实现实际的向量索引连接
2. `HnswVectorIndex` 已实现但未集成到检索流水线
3. 需将 `sqlrustgo_adapter.semantic_search()` 的结果与 `hnsw_vector_index.search()` 融合

**修复方案**：
```rust
// retrieval/engine.rs 中实现
pub async fn search(&self, query: &str, top_k: usize) -> Vec<ScoredChunk> {
    // 并行执行三个通道
    let (fts_results, vector_results, cypher_results) = tokio::join!(
        self.fts_channel.search(query, top_k),
        self.vector_channel.search(&self.embed(query), top_k),
        self.cypher_engine.query(query),  // 需先转换为 Cypher
    );

    // RRF 融合
    self.rrf_fusion(fts_results, vector_results, cypher_results, top_k)
}
```

### 4.2 Cypher 引擎未集成

**症状**：图查询类别（部门角色检索、多跳因果推理）0% 通过。

**根因**：`CypherEngine` 已实现但：
1. 未从自然语言查询自动转换为 Cypher
2. 未连接到 `sqlrustgo_adapter` 的实际图索引
3. 未集成到 `RetrievalEngine` 的多通道融合

### 4.3 评估指标问题

**症状**：50 深度评估中 6 个失败案例的召回文档看起来语义相关，但 Hit Rate 为 0。

**根因**：评估使用精确 `doc_id` 字符串匹配。召回的文档如 "物料验收规程" 和 "原料入库验收规程" 语义相关但 ID 不相等。

**建议**：使用 LLM-as-Judge 或语义相似度（`chunk_text` BLEU/Embedding）作为辅助评估指标。

---

## 5. 构建验证

```bash
$ cargo build
   Compiling gmp-server v1.0.0
   Compiling gmp-api v1.0.0
   Compiling gmp-storage v1.0.0
   ...
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 18.95s

# 无 error，仅 warnings
```

**Workspace 9 个 Crates**：
- gmp-core ✅
- gmp-storage ✅
- gmp-retrieval ✅
- gmp-reranker ✅
- gmp-llm ✅
- gmp-cli ✅
- gmp-server ✅ (新增)
- gmp-api ✅ (新增)
- gmp-eval ✅

---

## 6. 后续计划

### v1.2.0 优先级（P0）

| 编号 | 任务 | 阻塞 |
|------|------|------|
| P0-1 | 向量通道集成（HNSW → RRF fusion） | 91 场景 |
| P0-2 | 自然语言 → Cypher 转换器 | 28 场景 |
| P0-3 | 评估指标改进（LLM-as-Judge） | 指标准确性 |
| P0-4 | gmp-api 端到端集成测试 | 部署验证 |

### v1.3.0 优先级（P1）

| 编号 | 任务 | 预期收益 |
|------|------|----------|
| P1-1 | sqlrustgo CBO 优化器集成 | 查询延迟 -40% |
| P1-2 | MVCC SSI 并发控制 | 读写并发 |
| P1-3 | WAL 预写日志 | 数据可靠性 |

---

## 7. 附录

### A. sqlrustgo 依赖链（90+ crates）

```
sqlrustgo-mysql-server
├── sqlrustgo-server v3.0.0
├── sqlrustgo-executor v3.0.0
├── sqlrustgo-planner v3.0.0
├── sqlrustgo-optimizer v3.0.0
├── sqlrustgo-storage v3.0.0  ←─ 核心存储引擎（rusqlite 0.39）
├── sqlrustgo-transaction v3.0.0
├── sqlrustgo-gmp v3.0.0
└── sqlrustgo-distributed v0.1.0
```

### B. 代码统计

| 指标 | 值 |
|------|-----|
| 总代码行数 | 5,818 行 |
| 本 PR 新增 | 4,941 行 |
| 本 PR 删除 | 1,408 行 |
| 新增模块 | 4 个（cypher_engine, hnsw_vector_index, sqlrustgo_adapter, gmp-server, gmp-api） |

### C. Docker 支持

```bash
# 启动 gmp-server
cd crates/gmp-server
docker-compose up -d

# 验证
mysql -h localhost -P 3306 -u root -proot123 -e "SELECT * FROM gmp.documents LIMIT 1;"
```
