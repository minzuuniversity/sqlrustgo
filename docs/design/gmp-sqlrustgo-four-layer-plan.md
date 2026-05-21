# 基于 SQLRustGo 的 GMP 合规内审检索系统 — 四层架构设计与开发计划

> 版本: v1.0 | 日期: 2026-05-20 | 目标: 4 周完成 391 场景测试

---

## 1. 架构总览

```
┌─────────────────────────────────────────────────────────────────┐
│                  接入层: CLI / Hermes Agent / Web API             │
├─────────────────────────────────────────────────────────────────┤
│ Layer 4: LLM 问答层                                              │
│   └── Ollama + Qwen2.5-14B (答案生成 + 意图分类 + 多跳推理)       │
├─────────────────────────────────────────────────────────────────┤
│ Layer 3: Reranker 重排层                                         │
│   └── BGE-Reranker-v2-m3 (Cross-Encoder, CPU优先)       │
│   └── RRF 融合 (k=60, 4通道: 规则/向量/全文/图谱)                │
├─────────────────────────────────────────────────────────────────┤
│ Layer 2: 检索引擎层                                              │
│   ├── 规则召回: agent-knowledge YAML (Top-10, weight=2.0)        │
│   ├── 向量召回: bge-m3 via Ollama + HNSW/IVF (Top-30)            │
│   ├── 全文召回: BM25 via SQLRustGo FTS (Top-30)                  │
│   ├── 图谱召回: Cypher + BFS/DFS 邻居扩展 (Top-10)               │
├─────────────────────────────────────────────────────────────────┤
│ Layer 1: 统一数据存储层 — SQLRustGo                               │
│   ├── RDBMS: 文档元数据 + 结构化字段 (storage engine)             │
│   ├── Vector: bge-m3 1024d 嵌入 (vector crate, HNSW)             │
│   ├── FTS: GMP词典 + BM25 (storage::fts)                         │
│   ├── Graph: 实体关系图谱 (graph crate, Cypher, 磁盘+WAL)        │
│   ├── GMP: 审计链 + 电子签名 + 偏差管理 (gmp crate)              │
│   └── Unified: DocumentTable + GraphLink (unified-storage)       │
└─────────────────────────────────────────────────────────────────┘
```

### 核心差异：SQLRustGo 统一底座 vs 原方案多组件

| 维度 | 原方案（Phase A-D） | SQLRustGo 方案 |
|------|-------------------|----------------|
| RDBMS | 独立的 gmp.db (SQLite) | SQLRustGo storage engine (内嵌) |
| 向量数据库 | sqlite-vec 扩展 | SQLRustGo vector crate (HNSW/IVF/Flat) |
| 全文索引 | sqlite-fts5 扩展 | SQLRustGo storage::fts + BM25 |
| 图数据库 | KuzuDB / Neo4j / petgraph | SQLRustGo graph crate (Cypher + Disk + WAL) |
| 跨组件数据同步 | 需手动维护一致性 | `unified-storage` + `GraphLink` 原生原子操作 |
| GMP 合规特性 | 需另外开发 | `gmp` crate 内置审计链/电子签名/偏差管理 |
| 部署 | 多进程/多依赖 | 单二进制 (< 50MB)，零外部数据库依赖 |

---

## 2. 数据资产

| 资产 | 路径 | 规模 | 格式 |
|------|------|------|------|
| GMP 统一文档集 | `gmp-md/` | 1696 份 MD | Markdown |
| 脱敏文档集 | `gmp-md-masking/` | 1696 份 MD | Markdown |
| 结构化规则库 | `agent-knowledge/` | 55+ YAML | YAML |
| 391 测试场景 | `gmp_kg_build/test_results_final/` | 391 条 JSON | JSON |
| 现有基线 | `gmp_kg_build/` | v22: 44.5% | Python |
| SQLRustGo 源码 | `sqlrustgo/` | 42 crates | Rust v3.2.0-RC |

---

## 3. Layer 1: 统一数据存储层

### 3.1 数据模型

#### 文档主表

```sql
CREATE TABLE gmp_documents (
    id          INTEGER PRIMARY KEY,
    doc_id      TEXT UNIQUE NOT NULL,     -- 如 "CH01-001"
    title       TEXT NOT NULL,
    chapter     TEXT,                      -- CH01~CH09 + APPX
    doc_type    TEXT,                      -- SOP/RECORD/REPORT/REGULATION
    department  TEXT,
    status      TEXT DEFAULT 'active',
    source_file TEXT,
    created_at  INTEGER,
    updated_at  INTEGER
);
```

#### 文档内容表（用于 FTS）

```sql
CREATE TABLE gmp_document_chunks (
    id          INTEGER PRIMARY KEY,
    doc_id      TEXT NOT NULL REFERENCES gmp_documents(doc_id),
    chunk_index INTEGER NOT NULL,
    content     TEXT NOT NULL,
    token_count INTEGER DEFAULT 0          -- BM25 文档长度
);
```

#### 向量嵌入表

```sql
CREATE TABLE gmp_embeddings (
    chunk_id    INTEGER PRIMARY KEY REFERENCES gmp_document_chunks(id),
    doc_id      TEXT NOT NULL,
    embedding   BLOB NOT NULL,             -- bge-m3 1024d × 4 bytes = 4096 bytes
    model       TEXT DEFAULT 'bge-m3',
    created_at  INTEGER
);
-- HNSW 向量索引 (SQLRustGo vector crate)
CREATE VECTOR INDEX idx_embedding_hnsw ON gmp_embeddings(embedding)
    USING hnsw WITH (m=16, ef_construction=200, metric='cosine');
```

#### 图节点表（通过 unified-storage GraphLink 关联）

```sql
-- 实体节点数据通过 graph crate 的 DiskGraphStore 管理
-- 关联关系通过 GraphLink 定义:
--   gmp_documents.doc_id → graph.node("SOP", key=doc_id)
--   gmp_documents.chapter → graph.node("Chapter", key=chapter)
--   gmp_documents.department → graph.node("Department", key=department)
```

### 3.2 图模型设计

```
实体节点:
  (:SOP         {code, title, version, status})
  (:Chapter     {code, title})                          -- CH01~CH09
  (:Department  {name})                                 -- 生产部/质量部/...
  (:Device      {name, model, location})
  (:Material    {name, type, storage_condition})
  (:Deviation   {type, level, status})                  -- 重大/一般/微小
  (:CAPA        {id, type, status})
  (:Validation  {type, target})                         -- IQ/OQ/PQ/清洁
  (:Regulation  {code, title, version})                 -- GMP-84, GMP-101...
  (:AuditFinding {severity, status, date})

语义关系边:
  (:SOP)-[:BELONGS_TO]->(:Chapter)
  (:SOP)-[:MANAGED_BY]->(:Department)
  (:Deviation)-[:TRIGGERS]->(:CAPA)
  (:CAPA)-[:VERIFIED_BY]->(:Validation)
  (:Device)-[:REQUIRES]->(:Validation)
  (:Device)-[:OPERATED_PER]->(:SOP)
  (:Regulation)-[:MANDATES]->(:SOP)
  (:AuditFinding)-[:GENERATES]->(:CAPA)
  (:Material)-[:HANDLED_PER]->(:SOP)
  (:SOP)-[:PRECEDES]->(:SOP)                             -- 流程依赖
```

### 3.3 SQLRustGo 优势

- **原子性**: `DocumentTable` + `GraphLink` 在同一次事务中完成插入文档与创建图节点
- **零同步延迟**: RDBMS、向量索引、图存储共享同一 WAL
- **GMP 内置**: 审计链 (`audit_chain`)、电子签名 (`electronic_signature`)、偏差记录 (`correction_chain`) 开箱可用

---

## 4. Layer 2: 检索引擎层

### 4.1 四通道路由

```
Query → ┌─ 规则召回 (agent-knowledge) ──→ Top-10 ──┐
        ├─ 向量召回 (bge-m3 + HNSW)    ──→ Top-30 ──┤
        ├─ 全文召回 (BM25 + FTS)       ──→ Top-30 ──┼── RRF(k=60) ──→ Top-20 ──→ Layer 3
        └─ 图谱召回 (Cypher + BFS)     ──→ Top-10 ──┘
```

### 4.2 通道实现

#### 通道A: 规则召回（优先级最高）

```rust
// 直接使用 agent-knowledge YAML 文件
// 匹配策略: query关键词 → category → 返回规则定义
pub struct RuleChannel {
    matcher: RuleMatcher,
}

impl RuleChannel {
    pub fn search(&self, query: &str) -> Vec<ScoredDoc> {
        // 1. 分词 → 匹配 YAML 键名
        // 2. 分数字段直接映射为 confidence
        // 3. 规则命中标记 source_hint: "rule"
        // 4. 在 RRF 中 weight=2.0
    }
}
```

#### 通道B: 向量召回

```rust
pub struct VectorChannel {
    ollama: OllamaProvider,   // bge-m3 via Ollama API
    index: HnswIndex,         // SQLRustGo vector crate
}

impl VectorChannel {
    pub fn search(&self, query: &str, top_k: usize) -> Vec<ScoredDoc> {
        let embedding = self.ollama.embed(query)?;   // 1024d
        let results = self.index.search(&embedding, top_k)?;
        // 返回 (chunk_id, cosine_similarity)
    }
}
```

#### 通道C: 全文 BM25 召回

```rust
pub struct FtsChannel {
    fts: InvertedIndex,       // SQLRustGo storage::fts
    scorer: Bm25Scorer,
}

impl FtsChannel {
    pub fn search(&self, query: &str, top_k: usize) -> Vec<ScoredDoc> {
        // 使用 FMM 分词器 (GMP 词典 194K 词)
        // 返回 (doc_id, bm25_score)
    }
}
```

#### 通道D: 图谱召回

```rust
pub struct GraphChannel {
    store: DiskGraphStore,    // SQLRustGo graph crate (Cypher + 遍历)
}

impl GraphChannel {
    pub fn search(&self, query: &str, vector_results: &[ScoredDoc], top_k: usize) -> Vec<ScoredDoc> {
        // 1. 从 Layer 2 向量结果中识别实体 (Device/Material/Deviation)
        // 2. Cypher: MATCH (n)-[r]->(m) WHERE n.name CONTAINS "混合机" RETURN m
        // 3. 2-hop BFS 邻居扩展 → 获取关联文档
    }
}
```

### 4.3 RRF 融合

```rust
pub fn rrf_fusion(channels: &[ChannelResults], k: f32) -> Vec<ScoredDoc> {
    // score = Σ (channel_weight / (k + rank_in_channel))
    // 规则通道 weight=2.0, 其他=1.0
    let mut scores: HashMap<String, f32> = HashMap::new();
    for channel in channels {
        for (rank, doc) in channel.results.iter().enumerate() {
            let rrf_score = channel.weight / (k + (rank + 1) as f32);
            *scores.entry(doc.id.clone()).or_insert(0.0) += rrf_score;
        }
    }
    // 按分数降序，返回 Top-20
}
```

---

## 5. Layer 3: Reranker 重排层

### 5.1 接口

```rust
#[async_trait]
pub trait Reranker: Send + Sync {
    async fn rerank(
        &self,
        query: &str,
        candidates: &[(String, String)],  // (doc_id, chunk_content)
        top_k: usize,
    ) -> Result<Vec<(String, f32)>>;
}
```

### 5.2 实现

```rust
pub struct OllamaReranker {
    base_url: String,      // http://localhost:11434
    model: String,          // "bge-reranker-v2-m3"
    client: Client,
}

impl Reranker for OllamaReranker {
    async fn rerank(&self, query: &str, candidates: &[(String, String)], top_k: usize) -> ... {
        // 对每对 (query, chunk_content) 调用 Ollama embedding
        // 计算 cosine_similarity(query_emb, doc_emb)
        // 按分数降序返回 top_k
    }
}
```

### 5.3 回退策略

```rust
pub enum RerankerMode {
    Ollama(String),       // 标准模式
    Noop,                 // CPU 不可用时跳过重排
}
```

---

## 6. Layer 4: LLM 问答层

### 6.1 意图分类器

```rust
#[derive(Debug)]
pub enum QueryIntent {
    RetrieveSop,            // 获取 SOP 原文
    CheckCompliance,        // 合规性判定 (是/否)
    CompareRegulation,      // 跨法规对比
    TraceProcess,           // 流程追溯
    ClassifyIncident,       // 事件分类 (偏差/变更/OOS)
    GenerateChecklist,      // 生成检查清单
    AnalyzeRootCause,       // 根因分析
}
```

### 6.2 答案生成

```rust
pub struct OllamaAnswerGenerator {
    llm: OllamaLlmClient,   // model: "qwen2.5:14b"
}

impl OllamaAnswerGenerator {
    pub async fn generate(
        &self,
        question: &str,
        intent: QueryIntent,
        contexts: &[RetrievedChunk],   // Layer 2+3 的 Top-10 结果
    ) -> GeneratedAnswer {
        let prompt = self.build_prompt(intent, question, contexts);
        let raw = self.llm.chat(&[system_msg, user_msg], 0.3, 1024).await?;
        self.parse_citations(&raw, contexts)
    }
}
```

**Prompt 模板**:

```
System: 你是 GMP 合规审核 AI 助手。严格基于材料回答，材料不足时明确说明无法回答。不编造信息。

User:
## 相关文档
{doc_1_title}: {doc_1_content_snippet}
{doc_2_title}: {doc_2_content_snippet}
...

## 问题
{question}

## 要求
1. 给出明确答案
2. 标注来源 [文档编号]
3. 不确定处标注 [需人工确认]
```

---

## 7. 四层全链路 Pipeline

```rust
pub struct GmpRetrievalPipeline {
    // Layer 1: 存储
    storage: SqlRustGoEngine,
    vector_index: HnswIndex,
    graph_store: DiskGraphStore,
    
    // Layer 2: 检索
    rule_channel: RuleChannel,
    vector_channel: VectorChannel,
    fts_channel: FtsChannel,
    graph_channel: GraphChannel,
    
    // Layer 3: 重排
    reranker: Box<dyn Reranker>,
    
    // Layer 4: LLM
    answer_generator: OllamaAnswerGenerator,
}

impl GmpRetrievalPipeline {
    pub async fn query(&self, query: &str) -> QueryResult {
        // Step 1: Layer 2 多路召回
        let rules   = self.rule_channel.search(query, 10);
        let vectors = self.vector_channel.search(query, 30);
        let fts     = self.fts_channel.search(query, 30);
        let graph   = self.graph_channel.search(query, &vectors, 10);
        
        // Step 2: RRF 融合
        let fused = rrf_fusion(&[
            ChannelResults::new(rules, 2.0),    // 规则 weight=2.0
            ChannelResults::new(vectors, 1.0),
            ChannelResults::new(fts, 1.0),
            ChannelResults::new(graph, 1.0),
        ], 60.0);
        
        // Step 3: Layer 3 Reranker 重排
        let top_candidates: Vec<_> = fused.iter().take(20)
            .map(|d| (d.id.clone(), d.content.clone())).collect();
        let reranked = self.reranker.rerank(query, &top_candidates, 10).await?;
        
        // Step 4: Layer 4 LLM 答案生成 (可选)
        let contexts = self.resolve_chunks(&reranked);
        let answer = self.answer_generator.generate(query, contexts).await?;
        
        QueryResult { reranked, answer }
    }
}
```

---

## 8. CLI 接口设计

```bash
# 索引构建 (一次性)
gmp-retrieve index --source gmp-md/ --source agent-knowledge/

# 检索查询
gmp-retrieve search "偏差分级标准"
gmp-retrieve search "混合机转速异常偏差应该怎么登记" --explain

# 评估模式
gmp-retrieve evaluate --test-set 391_scenarios.json --output results.json

# 交互式
gmp-retrieve shell
> 搜索: 偏差与CAPA的关系
```

---

## 9. 四周开发计划

### 时间分配

```
Week 1: Layer 1 数据底座 + SQLRustGo 增量增强
Week 2: Layer 2 检索引擎 (四通道 + RRF 融合)
Week 3: Layer 3 Reranker + Layer 4 LLM 问答
Week 4: 391 场景测试 + 调优 + 报告
```

### Week 1: 数据底座 + SQLRustGo 增强（Day 1-7）

| Day | 任务 | 产出 |
|-----|------|------|
| 1 | SQLRustGo 增量增强 #1: FTS BM25 评分 | `search_with_scores()` 可用 |
| 2 | SQLRustGo 增量增强 #2: RRF 融合 | `reciprocal_rank_fusion()` 可用 |
| 3 | 数据导入: 1696 份 gmp-md 文档 → SQLRustGo tables | `gmp_documents` + `gmp_document_chunks` 就绪 |
| 4 | 向量化: Ollama bge-m3 批量嵌入 1696 文档 | ~8700 chunks 向量入库 |
| 4 | HNSW 索引构建 | `idx_embedding_hnsw` 可用 |
| 5 | 图谱构建: agent-knowledge → 实体节点 + 关系边 | > 700 节点, > 1500 边 |
| 5 | 规则加载: agent-knowledge YAML → RuleMatcher | > 55 规则加载 |
| 6 | SQLRustGo 增量增强 #5: 规则通道权重 | RuleMatcher 集成到 RRF |
| 7 | Layer 1 端到端验证: 文档 → FTS/向量/图谱 三路可查 | 验证脚本 |

**Week 1 检查点**: 数据全部入库，BM25/向量/图谱三路独立可查。

### Week 2: Layer 2 检索引擎（Day 8-14）

| Day | 任务 | 产出 |
|-----|------|------|
| 8 | 实现 `RuleChannel` + 评估 | 规则命中率 ≥ 64%（对齐 agent-knowledge 基线） |
| 9 | 实现 `VectorChannel` (bge-m3 + HNSW) + 评估 | recall@20 ≥ 70% |
| 10 | 实现 `FtsChannel` (BM25 + FMM 分词) + 评估 | recall@20 ≥ 50% |
| 11 | 实现 `GraphChannel` (Cypher + BFS 邻居扩展) | 图谱增强召回可用 |
| 12 | 四通道 RRF 融合管线 | 端到端检索可用 |
| 13 | 实现 CLI (`index/search/evaluate` 子命令) | `gmp-retrieve` 二进制可用 |
| 14 | 391 场景 Layer 2 初步测试 | 通过率基线 (预估 ≥ 65%) |

**Week 2 检查点**: Layer 2 四通道 + RRF 融合完成，391 场景通过率 ≥ 65%。

### Week 3: Layer 3 + Layer 4（Day 15-21）

| Day | 任务 | 产出 |
|-----|------|------|
| 15 | SQLRustGo 增量增强 #3: Reranker crate | `OllamaReranker` 可用 |
| 16 | Reranker 集成到检索管线 | RRF → Reranker 端到端 |
| 17 | SQLRustGo 增量增强 #4: LLM Chat | `OllamaLlmClient` + `AnswerGenerator` 可用 |
| 18 | 意图分类器实现 | 7 种意图分类可用 |
| 19 | 答案生成 + 引用溯源 | RAG 全链路: 检索→重排→生成 |
| 20 | Layer 2+3 组合测试 (391 场景) | 通过率预估 ≥ 73% |
| 21 | 全链路 Layer 2+3+4 组合测试 | 通过率预估 ≥ 78% |

**Week 3 检查点**: 四层全链路可用，通过率 ≥ 78%。

### Week 4: 391 场景验证 + 调优（Day 22-28）

| Day | 任务 | 产出 |
|-----|------|------|
| 22 | 391 场景完整回归测试 | 自动化结果 JSON |
| 23 | 失败模式分类分析 (知识缺口/语义偏差/图谱不足/LLM幻觉) | 分析报告 |
| 24 | 快速修复迭代 (Day 22-23 发现的 Top-5 问题) | 补丁 |
| 25 | 回归测试 (第二轮) | 更新后的通过率 |
| 26 | 对比基线: v22 (44.5%) / Layer 2 alone / Layer 2+3 / Full | 对比表 |
| 27 | 性能压测: 10 并发 × 100 轮, 延迟 P50/P95/P99 | 性能报告 |
| 28 | 最终报告 + 代码提交 | 完整交付 |

**Week 4 检查点**: 391 场景通过率 ≥ 80%，最终报告。

---

## 10. 通过率预估分解

| 阶段 | 能力 | 通过率 | 提升 |
|------|------|--------|------|
| 基线 | v22 (FTS5 BM25 + 简单语义) | 44.5% | — |
| Week 1 | Layer 1 数据完整 + BM25/HNSW 可用 | — | — |
| Week 2 | Layer 2: 四通道 + RRF 融合 | **~68%** | +23.5% |
| Week 3 | Layer 3: Reranker 重排 | **~73%** | +5% |
| Week 3 | Layer 4: LLM 答案生成 (复杂查询) | **~80%** | +7% |
| Week 4 | 调优 + 修复 | **≥ 80%** | fine-tune |

### 无法覆盖的 ~20% (≈78/391)

| 类别 | 数量 | 原因 | 缓解策略 |
|------|------|------|----------|
| 中国 GMP 条文查询 | ~30 | 知识库仅 EU GMP，缺完整中国 GMP 2010 版 | 标注为"知识缺口"，待后续补充 |
| 跨法规深度对比 | ~20 | 需要 LLM 多步对比推理，14B 模型能力边界 | LLM 返回对比框架，标注 [需人工确认] |
| 高度情境化设备操作 | ~15 | 缺少设备手册/运行参数细节 | 标注数据缺失 |
| LLM 幻觉 | ~13 | 即使有上下文，14B 模型仍可能编造 | 引用溯源机制 + 人工抽查 |

---

## 11. 硬件与依赖

| 组件 | 要求 | 说明 |
|------|------|------|
| CPU | ≥ 16 核 | bge-m3 嵌入生成 + BM25 索引 |
| RAM | ≥ 32 GB | 模型加载 + 向量索引内存 |
| GPU | 可选 | RTX 5080 用于加速 bge-reranker（当前兼容性问题先用 CPU） |
| 磁盘 | ≥ 20 GB | 1696 文档 + HNSW 索引 + 图存储 |
| Ollama | localhost:11434 | bge-m3 / bge-reranker-v2-m3 / qwen2.5:14b |
| Rust | ≥ 1.80 | 编译 SQLRustGo |

---

## 12. 风险与缓解

| 风险 | 概率 | 影响 | 缓解 |
|------|------|------|------|
| SQLRustGo 增量增强阻塞 Hermes 进度 | 中 | 高 | 增强部分相对独立，可先 mock 接口并行开发检索管线 |
| bge-m3 嵌入生成速度慢 (CPU) | 高 | 中 | 一次性批处理，1696 文档预计 2-4 小时 |
| Ollama API 不稳定 | 中 | 中 | 增加重试机制 (3次) + 超时 30s |
| 图谱实体抽取不足 | 中 | 中 | agent-knowledge YAML 已有 55+ 规则可自动抽取，目标 ≥ 700 节点 |
| 391 场景未达 80% | 中 | 中 | 分层分析失败原因，标注知识缺口，区分"系统能力不足"vs"数据不全" |
| RTX 5080 兼容问题 | 高 | 低 | 全部 CPU 模式可运行，GPU 仅用于加速 reranker |
| 4 周时间不够 | 低 | 中 | Week 1-2 是最关键的基础，即使 Week 3-4 压缩，Layer 2 产物也可独立交付 |

---

## 13. 交付物清单

1. **`gmp-retrieve`** CLI 二进制 (< 50MB)
2. **`gmp.db`** SQLRustGo 数据库文件（含 1696 文档 + FTS + 向量 + 图）
3. **391 场景测试结果** JSON + Markdown 报告
4. **跨系统对比报告** (v22 vs Layer 2 vs Layer 2+3 vs Layer 2+3+4)
5. **SQLRustGo 增量补全 PR** (BM25/RRF/Reranker/LLM/规则权重)
6. **运维文档**: 索引构建、模型部署、CLI 使用

---

> **结论**: 基于 SQLRustGo 的统一数据底座，4 周内可完成四层架构的 GMP 合规内审检索系统开发，391 场景目标通过率 ≥ 80%（比当前最优 v22 的 44.5% 提升 35.5 个百分点）。