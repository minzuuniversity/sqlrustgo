# GMP-Platform v1.3.0 系统调研与改进分析

> **日期**: 2026-05-24
> **版本**: v1.3.0
> **目标**: 全面检查 SQLRustGo 集成后的数据存储、检索性能、术语管理，识别改进机会

---

## 一、JSON 格式使用情况检查

### 1.1 发现的问题

| 位置 | 问题 | 影响 |
|------|------|------|
| `gmp_embeddings.embedding` (JSON fallback) | `serde_json::to_string(&Vec<f32>)` 序列化向量 | 存储体积大、解析慢 |
| `gmp_terms.synonyms` | `serde_json::to_string(&synonyms)` 存为 TEXT，无法 SQL 查询 | 同义词查询效率低 |
| `gmp_terms.hypernyms/hyponyms` | 同上，JSON 字符串字段 | 无法层级遍历 |
| `run_layer0_pipeline.py` | 全部正则/规则硬编码在 Python dict/list 中 | 不可扩展、不可复用 |

### 1.2 JSON 存储实测差异

以 1024 维向量为例（二进制 vs JSON）：

```
Binary BLOB: [dim u32][f32 × 1024] = 4 + 4096 = 4100 bytes
JSON string: "[-0.123, 0.456, ...]"    ≈ 8200+ bytes (2x)
```

### 1.3 二进制 BLOB 已实现（正确）

`storage.rs:700-724` 已正确实现二进制路径：

```rust
// Binary BLOB path: [dim u32 BE][f32 BE × dim]
for emb in &bin_embs {
    let chunk_id = doc_value_int(emb, 0);
    let blob = doc_value_blob(emb, 2);
    let dim = u32::from_be_bytes([blob[0], blob[1], blob[2], blob[3]]) as usize;
    // ... 手动解析 f32 数组
}
```

**问题**: 这个手动解析效率低，应使用 `Vec::<f32>::from_slice` 或 `unsafe { mem::transmute }`。

---

## 二、gmp_terms 同义词表设计缺陷

### 2.1 当前 Schema

```sql
CREATE TABLE gmp_terms (
    term_id       INTEGER PRIMARY KEY,
    canonical_term TEXT NOT NULL,    -- "偏差"
    term_type     TEXT NOT NULL,    -- "entity"
    synonyms      TEXT,             -- JSON string: '["偏离","异常","OOS"]'
    hypernyms     TEXT,             -- JSON string: '["质量事件"]'
    hyponyms      TEXT,             -- JSON string: '["重大偏差","一般偏差"]'
    gmp_chapter   TEXT,
    entity_type   TEXT,
);
```

### 2.2 问题

- `synonyms` 是 JSON 字符串，无法执行 `WHERE synonyms LIKE '%偏差%'`
- 同义词展开需要 `serde_json::from_str` 解析，O(n) 复杂度
- 无法建立 `(term) --[SYNONYM_OF]-- (term)` 图关系

### 2.3 改进方案

**方案 A: 纯关系表**

```sql
-- 同义词关系
CREATE TABLE term_synonyms (
    term_id    INTEGER REFERENCES gmp_terms(term_id),
    synonym    TEXT NOT NULL,
    confidence REAL DEFAULT 1.0,
    PRIMARY KEY (term_id, synonym)
);

-- 层级关系
CREATE TABLE term_hierarchy (
    parent_id  INTEGER REFERENCES gmp_terms(term_id),
    child_id   INTEGER REFERENCES gmp_terms(term_id),
    rel_type   TEXT,              -- 'HYPERNYM' | 'HYPONYM' | 'PART_OF'
    PRIMARY KEY (parent_id, child_id)
);

-- 查询 "偏差" 的所有同义词
SELECT synonym FROM term_synonyms WHERE term_id = (
    SELECT term_id FROM gmp_terms WHERE canonical_term = '偏差'
);
```

**方案 B: 图节点（推荐）**

```
(:Term {term: "偏差", type: "entity"})
    --[SYNONYM_OF {word: "偏离"}]->
(:Term {term: "偏离", type: "synonym"})
    --[BROADER_THAN]->
(:Term {term: "质量事件", type: "hypernym"})
    --[NARROWER_THAN]->
(:Term {term: "重大偏差", type: "hyponym"})
```

**优点**: 可以用 Cypher 查询 "偏差的同义词的同义词"，支持多层级扩展。

---

## 三、硬编码术语规则分析

### 3.1 当前硬编码位置

**run_layer0_pipeline.py** 中的硬编码正则：

```python
ENTITY_PATTERNS = {
    'Deviation': [
        (r'重大偏差', '重大偏差'),
        (r'一般偏差', '一般偏差'),
        (r'OOS|超标|偏离', 'OOS'),
    ],
    ...
}

RELATION_TEMPLATES = [
    (r'(\w{2,20})触发([A-Z]{2,}-?\d+)', 'TRIGGERS', ...),
    ...
]

TAXONOMY_TOPIC_PATTERNS = [
    (r'偏差|偏离|异常', 'topic', '偏差', 10),
    ...
]
```

### 3.2 问题

| 问题 | 影响 |
|------|------|
| 无法热更新 | 修改规则需改代码 + 重新部署 |
| 无法版本化 | 无法追踪规则变更历史 |
| 无法协作 | 无法让领域专家直接编辑 |
| 重复定义 | 同一术语在不同阶段重复出现 |

### 3.3 改进方案：规则外置 + 数据库存储

```sql
-- 规则表
CREATE TABLE extraction_rules (
    rule_id      INTEGER PRIMARY KEY,
    rule_type    TEXT NOT NULL,       -- 'ENTITY'|'RELATION'|'TAXONOMY'|'SCORER'
    pattern      TEXT NOT NULL,       -- 正则表达式
    target_field TEXT NOT NULL,       -- 'entity_type'|'rel_type'|'topic'
    target_value TEXT NOT NULL,       -- '偏差'|'TRIGGERS'|'验证'
    weight       REAL DEFAULT 1.0,
    priority     INTEGER DEFAULT 0,
    enabled      INTEGER DEFAULT 1,
    created_at   INTEGER,
    updated_at   INTEGER
);

-- 使用时从 DB 加载
rules = storage.scan("extraction_rules").filter(|r| r.enabled && r.rule_type == "ENTITY")
```

**优点**: 热更新、版本化、领域专家可编辑。

---

## 四、检索性能分析

### 4.1 当前 QPS 数据

| 指标 | 值 | 来源 |
|------|-----|------|
| 平均延迟 | 1049.58 ms | 压力测试 (3 runs) |
| P50 延迟 | 931.65 ms | 压力测试 |
| P95 延迟 | 3840.87 ms | 压力测试 |
| P99 延迟 | 3840.87 ms | 压力测试 |

**注**: 测试场景有限（仅 3 次），数据仅供参考。

### 4.2 性能瓶颈分析

```
查询路径延迟分解（估算）:
├── Ollama bge-m3 向量化: ~200-500ms (网络 + 推理)
├── HNSW 向量检索 (Top-30): ~10-50ms (内存)
├── BM25 FTS 检索 (Top-30): ~20-100ms (磁盘)
├── RRF 融合: ~1-5ms (内存)
├── Cypher 图查询 (Top-10): ~50-200ms (内存图遍历)
├── Ollama Reranker (Top-20): ~100-300ms (网络 + 推理) ★ 可选
└── 总计: ~400-1150ms
```

### 4.3 改进方案

#### 4.3.1 向量检索优化

```rust
// 当前: 每次查询都调用 Ollama API
let embedding = ollama.embed(query)?;  // 200-500ms 网络开销

// 改进: 本地 embedding 缓存 (LRU)
pub struct CachedEmbeddingStore {
    cache: Arc<RwLock<LruCache<String, Vec<f32>>>>,
    ollama: OllamaClient,
    ttl_seconds: u64,
}

impl CachedEmbeddingStore {
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let key = sha256(text);
        if let Some(v) = self.cache.read().await.get(&key) {
            return Ok(v.clone());
        }
        let v = self.ollama.embed(text).await?;
        self.cache.write().await.put(key, v.clone());
        Ok(v)
    }
}
```

#### 4.3.2 HNSW 参数调优

```rust
// 当前默认参数 (从 sqlrustgo_storage 获取)
HnswConfig {
    m: 16,                    // 建议: 32-64 (高召回)
    ef_construction: 200,     // 建议: 400 (高质量)
    ef_search: 50,           // 建议: 100-200 (高精度)
    // ...
}

// 改进: 根据数据规模自动调优
let doc_count = storage.count("gmp_embeddings")?;
let (m, ef_c, ef_s) = match doc_count {
    n if n < 10_000 => (16, 100, 50),
    n if n < 100_000 => (32, 200, 100),
    _ => (48, 400, 200),
};
```

#### 4.3.3 BM25 缓存

```rust
// 高频查询词 → Top-Docs 缓存
pub struct FtsCache {
    cache: Arc<RwLock<LruCache<String, Vec<ScoredChunk>>>>,
}

impl FtsCache {
    pub fn search(&self, query: &str) -> Option<Vec<ScoredChunk>> {
        let key = self.normalize(query)?;
        self.cache.read().get(&key).cloned()
    }
}
```

#### 4.3.4 并行化通道检索

```rust
// 当前: 串行
let rules = self.rule_channel.search(&query, 10);
let vectors = self.vector_channel.search(&query, 30);  // 等待
let fts = self.fts_channel.search(&query, 30);         // 等待
let graph = self.graph_channel.search(&query, 10);     // 等待

// 改进: tokio 并行
let (rules, vectors, fts, graph) = tokio::join!(
    rule_channel.search(&query, 10),
    vector_channel.search(&query, 30),
    fts_channel.search(&query, 30),
    graph_channel.search(&query, 10),
);
```

---

## 五、408 场景测试的正确性与可信性分析

### 5.1 当前测试方法

```json
{
  "query": "年度内部审计计划应该包含哪些内容？",
  "expected_doc_ids": ["审计计划", "审计"],
  "category": "内审专项",
  "top_k": 10
}
```

**验证逻辑** (`gmp-eval`):

```rust
// 当前: 关键字匹配
fn evaluate_scenario(scenario: &Scenario, results: &[ScoredChunk]) -> bool {
    let expected: Vec<&str> = scenario.expected_doc_ids.iter()
        .map(|s| s.as_str())
        .collect();
    let matched: HashSet<_> = expected.iter()
        .filter(|e| results.iter().any(|r| r.content.contains(e)))
        .collect();
    matched.len() >= expected.len() / 2  // 50% 阈值
}
```

### 5.2 问题

| 问题 | 说明 |
|------|------|
| **关键字匹配不可靠** | "审计计划" 必须在文档中出现，否则失败 |
| **同义词未处理** | "年度审计方案" ≠ "审计计划"，会被判定为 FAIL |
| **无上下文语义** | 无法判断文档是否真正回答了问题 |
| **阈值主观** | 50% 匹配率无理论依据 |
| **无证据提取** | 不验证引用的文档片段是否与问题相关 |

### 5.3 改进方案：LLM 辅助验证

```python
def llm_validate_scenario(query: str, expected_doc_ids: list, retrieved_docs: list) -> dict:
    """
    使用 LLM 判断检索结果是否正确回答了问题。
    """
    doc_summaries = "\n".join([
        f"[Doc {i+1}] {d['title']}: {d['content'][:500]}..."
        for i, d in enumerate(retrieved_docs[:5])
    ])

    prompt = f"""
## 问题
{query}

## 检索到的文档
{doc_summaries}

## 要求判断
1. 上述文档是否包含回答问题所需的信息？
2. 如果是，指出最相关的 1-3 篇文档，说明原因。
3. 如果否，说明缺失的核心信息是什么。

## 输出格式 (JSON)
{{
  "verdict": "PASS | PARTIAL | FAIL",
  "relevant_docs": [doc_id, ...],
  "reason": "...",
  "missing_info": "..."
}}
"""
    response = ollama.chat(model="qwen2.5:14b", messages=[{"role": "user", "content": prompt}])
    return json.loads(response)
```

### 5.4 综合分析流程

```
┌─────────────────────────────────────────────────────────┐
│ Step 1: 关键字检索                                       │
│   query → keyword extraction → keyword search           │
│   找出所有包含关键字的文档候选集                          │
├─────────────────────────────────────────────────────────┤
│ Step 2: 向量相似度检索                                   │
│   query embedding → HNSW ANN → Top-K similar docs       │
│   找出语义相关的文档候选集                               │
├─────────────────────────────────────────────────────────┤
│ Step 3: 文档相关性评分 (LLM)                            │
│   prompt: "Does Doc-X answer Query-Y?" → [0-1 score]  │
│   筛选出真正相关的文档                                   │
├─────────────────────────────────────────────────────────┤
│ Step 4: 综合排序                                        │
│   combined_score = α × keyword_match + β × vector_sim  │
│                  + γ × llm_relevance                   │
│   输出最终 Top-K 文档 + 证据引用                         │
├─────────────────────────────────────────────────────────┤
│ Step 5: 可信性验证                                      │
│   验证引用的文档片段是否在原文中存在                      │
│   验证引用的文档片段是否真正支撑答案                      │
└─────────────────────────────────────────────────────────┘
```

### 5.5 测试场景扩充建议

| 维度 | 当前覆盖 | 建议扩充 |
|------|----------|----------|
| 偏差调查 | 部分 | 偏差分级、处理流程、CAPA 关联 |
| 验证确认 | 部分 | IQ/OQ/PQ/DQ 流程、设备验证 |
| 文件管理 | 部分 | SOP 编写、版本控制、批准流程 |
| 变更控制 | 无 | 变更申请、影响评估、批准 |
| 风险管理 | 无 | FMEA、关键点识别、风险评估 |

---

## 六、总结与改进优先级

### 高优先级 (P0)

| 改进项 | 预期收益 | 工作量 |
|--------|----------|--------|
| 向量缓存 (LRU) | 减少 200-500ms/次查询 | 中 |
| 同义词关系表 (方案 A) | 提升同义词查询效率 | 低 |
| LLM 辅助验证 | 提升测试可信度 | 高 |

### 中优先级 (P1)

| 改进项 | 预期收益 | 工作量 |
|--------|----------|--------|
| 规则外置数据库 | 可热更新、可协作 | 中 |
| 并行通道检索 | 减少 30-50ms/次查询 | 低 |
| BM25 缓存 | 减少高频查询延迟 | 低 |
| HNSW 参数调优 | 提升召回率/精度 | 低 |

### 低优先级 (P2)

| 改进项 | 预期收益 | 工作量 |
|--------|----------|--------|
| 二进制 BLOB 解析优化 | 微性能提升 | 低 |
| 图节点方案 (方案 B) | 支持复杂层级查询 | 高 |

---

*最后更新: 2026-05-24*
