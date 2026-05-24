# SPEC: GMP-Platform v1.3.1 性能优化与架构改进

> **版本**: v1.3.1
> **日期**: 2026-05-24
> **状态**: Draft
> **PRIORITY**: P0 = 高, P1 = 中, P2 = 低

---

## 1. 背景与目标

### 1.1 问题陈述

v1.3.0 基于 SQLRustGo 完成了存储层重构，但存在以下问题：

| # | 问题 | 影响 |
|---|------|------|
| P0-1 | Ollama API 每次查询都调用，网络开销 200-500ms | 查询延迟高 |
| P0-2 | `gmp_terms.synonyms` 用 JSON 存储，无法 SQL 查询 | 同义词展开慢 |
| P0-3 | 408 场景测试用关键字匹配，无法验证语义正确性 | 测试可信度低 |
| P1-1 | 术语规则硬编码在 `run_layer0_pipeline.py` | 不可热更新 |
| P1-2 | 四通道串行检索，无法并行 | 延迟浪费 30-50ms |
| P2-1 | BM25 无查询缓存 | 高频查询重复计算 |
| P2-2 | HNSW 参数未调优 | 召回率/精度非最优 |

### 1.2 目标

```
v1.3.1 目标:
- 查询延迟: P50 < 500ms (当前 931ms)
- 同义词查询: 支持 SQL 展开
- 408 测试: LLM 辅助验证，覆盖率 > 90%
- 术语规则: 数据库存储，支持热更新
```

---

## 2. 改进项详细规格

### IMPROVEMENT-1: LRU Embedding 缓存 [P0]

**文件**: `crates/gmp-retrieval/src/channel.rs`

#### 2.1.1 需求

```
- 查询文本 → SHA256 hash → LRU cache key
- 缓存命中: 直接返回 (0ms)
- 缓存未命中: 调用 Ollama API，存入缓存
- TTL: 24 小时（可配置）
- 最大容量: 10000 条（可配置）
```

#### 2.1.2 实现

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use sha2::{Sha256, Digest};

pub struct CachedEmbeddingStore {
    cache: Arc<RwLock<HashMap<String, CachedVector>>>,
    ollama_url: String,
    model: String,
    ttl_seconds: u64,
    max_capacity: usize,
}

#[derive(Clone)]
struct CachedVector {
    embedding: Vec<f32>,
    timestamp: u64,
}

impl CachedEmbeddingStore {
    pub fn new(ollama_url: String, model: String) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            ollama_url,
            model,
            ttl_seconds: 86400, // 24h
            max_capacity: 10000,
        }
    }

    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let key = self.compute_key(text);

        // Try cache first
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(&key) {
                if self.is_valid(cached) {
                    return Ok(cached.embedding.clone());
                }
            }
        }

        // Cache miss: call Ollama
        let embedding = self.call_ollama(text).await?;

        // Update cache
        {
            let mut cache = self.cache.write().await;
            if cache.len() >= self.max_capacity {
                // Evict oldest
                if let Some(oldest_key) = cache.keys().next().cloned() {
                    cache.remove(&oldest_key);
                }
            }
            cache.insert(key, CachedVector {
                embedding: embedding.clone(),
                timestamp: current_timestamp(),
            });
        }

        Ok(embedding)
    }

    fn compute_key(&self, text: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(text.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    fn is_valid(&self, cached: &CachedVector) -> bool {
        current_timestamp() - cached.timestamp < self.ttl_seconds
    }

    async fn call_ollama(&self, text: &str) -> Result<Vec<f32>> {
        let client = reqwest::Client::new();
        let resp = client
            .post(format!("{}/api/embeddings", self.ollama_url))
            .json(&serde_json::json!({
                "model": self.model,
                "prompt": text
            }))
            .send()
            .await?;
        #[derive(serde::Deserialize)]
        struct Resp { embedding: Vec<f32> }
        let resp: Resp = resp.json().await?;
        Ok(resp.embedding)
    }
}
```

#### 2.1.3 验收标准

| 条件 | 验证方法 |
|------|---------|
| 相同文本第二次查询不调 Ollama | 第一次查日志有 `/api/embeddings`，第二次无 |
| 缓存 TTL 过期后重新请求 | 设置 TTL=1s，2s 后验证 |
| 缓存容量超限后淘汰旧条目 | 插入 10001 条，检查第 1 条被淘汰 |
| 不同文本各自独立缓存 | 两个不同查询，验证缓存 2 条 |

#### 2.1.4 性能目标

```
缓存命中: < 1ms (内存查找)
缓存未命中: 维持原 Ollama 开销
预期 P50 延迟改善: 931ms → 400ms (改善 57%)
```

---

### IMPROVEMENT-2: 同义词关系表 [P0]

**文件**: `crates/gmp-kg/src/storage.rs`

#### 2.2.1 当前 Schema

```sql
-- 问题: synonyms 是 JSON 字符串
CREATE TABLE gmp_terms (
    term_id       INTEGER PRIMARY KEY,
    canonical_term TEXT NOT NULL,
    synonyms      TEXT,  -- JSON: '["偏离","异常"]'
    ...
);
```

#### 2.2.2 改进 Schema

```rust
// 保留 gmp_terms 主表（简化版）
let terms_table = TableInfo {
    name: "gmp_terms".to_string(),
    columns: vec![
        ColumnDefinition::new("term_id", "integer"),
        ColumnDefinition::new("canonical_term", "text"),
        ColumnDefinition::new("term_type", "text"),
        ColumnDefinition::new("gmp_chapter", "text"),
        ColumnDefinition::new("entity_type", "text"),
    ],
    // ...
};

// 新增: term_synonyms 表
let synonyms_table = TableInfo {
    name: "term_synonyms".to_string(),
    columns: vec![
        ColumnDefinition::new("id", "integer"),
        ColumnDefinition::new("term_id", "integer"),  // FK → gmp_terms.term_id
        ColumnDefinition::new("synonym", "text"),
        ColumnDefinition::new("confidence", "real"),   // 0.0~1.0
    ],
    // primary key: (term_id, synonym)
};

// 新增: term_hierarchy 表
let hierarchy_table = TableInfo {
    name: "term_hierarchy".to_string(),
    columns: vec![
        ColumnDefinition::new("id", "integer"),
        ColumnDefinition::new("parent_id", "integer"), // FK → gmp_terms.term_id
        ColumnDefinition::new("child_id", "integer"),  // FK → gmp_terms.term_id
        ColumnDefinition::new("rel_type", "text"),     // 'HYPERNYM' | 'HYPONYM' | 'PART_OF'
    ],
    // primary key: (parent_id, child_id)
};
```

#### 2.2.3 新增查询方法

```rust
impl KgStorage {
    /// Get all synonyms for a canonical term
    pub fn get_synonyms(&self, canonical_term: &str) -> Result<Vec<String>> {
        let terms = self.storage.scan("gmp_terms")?;
        let term_id = terms.iter()
            .filter(|r| doc_value_str(r, 1) == canonical_term)
            .find_map(|r| doc_value_int_opt(r, 0));

        let term_id = match term_id {
            Some(id) => id,
            None => return Ok(vec![]),
        };

        let synonyms = self.storage.scan("term_synonyms")?;
        Ok(synonyms.iter()
            .filter(|r| doc_value_int(r, 1) == term_id)
            .map(|r| doc_value_str(r, 2))
            .collect())
    }

    /// Expand a query with synonyms
    pub fn expand_query_with_synonyms(&self, query: &str) -> Result<Vec<String>> {
        let mut expanded = vec![query.to_string()];
        for word in query.split_whitespace() {
            let synonyms = self.get_synonyms(word)?;
            expanded.extend(synonyms);
        }
        Ok(expanded)
    }

    /// Get hypernyms (上位词)
    pub fn get_hypernyms(&self, term: &str) -> Result<Vec<String>> {
        // ... Cypher-style traversal via term_hierarchy
    }
}
```

#### 2.2.4 数据迁移

```python
# scripts/migrate_terms_to_relation_tables.py
"""
将现有的 JSON synonyms 迁移到 term_synonyms 表
"""
def migrate(conn):
    cursor = conn.cursor()
    cursor.execute("SELECT term_id, synonyms FROM gmp_terms WHERE synonyms IS NOT NULL")
    for term_id, synonyms_json in cursor.fetchall():
        synonyms = json.loads(synonyms_json)
        for syn in synonyms:
            cursor.execute(
                "INSERT OR IGNORE INTO term_synonyms (term_id, synonym, confidence) VALUES (?, ?, 1.0)",
                (term_id, syn)
            )
    conn.commit()
```

#### 2.2.5 验收标准

| 条件 | 验证方法 |
|------|---------|
| `get_synonyms("偏差")` 返回 `["偏离","异常","OOS"]` | `python -c "import sqlrustgo; ..."` |
| `expand_query_with_synonyms("偏差调查")` 返回 `["偏差调查","偏离","异常"]` | 单元测试 |
| SQL 查询 `SELECT synonym FROM term_synonyms WHERE term_id=?` 可执行 | 手动验证 |

---

### IMPROVEMENT-3: LLM 辅助 408 验证 [P0]

**文件**: `crates/gmp-eval/src/llm_validator.rs` (新文件)

#### 2.3.1 需求

```
原始验证: expected_doc_ids 中的关键字必须在检索结果中出现
改进验证: LLM 判断每篇文档是否真正回答了问题
```

#### 2.3.2 实现

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct LlmValidationRequest {
    pub query: String,
    pub documents: Vec<DocSnippet>,
    pub expected_doc_ids: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocSnippet {
    pub doc_id: String,
    pub title: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LlmValidationResult {
    pub verdict: ValidationVerdict,  // PASS / PARTIAL / FAIL
    pub relevant_doc_ids: Vec<String>,
    pub reason: String,
    pub missing_info: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ValidationVerdict {
    PASS,
    PARTIAL,
    FAIL,
}

pub struct LlmValidator {
    ollama_url: String,
    model: String,
    client: reqwest::Client,
}

impl LlmValidator {
    pub fn new(ollama_url: String, model: String) -> Self {
        Self {
            ollama_url,
            model,
            client: reqwest::Client::new(),
        }
    }

    pub async fn validate(&self, req: &LlmValidationRequest) -> Result<LlmValidationResult> {
        let prompt = self.build_prompt(req);

        let resp = self.client
            .post(format!("{}/api/generate", self.ollama_url))
            .json(&serde_json::json!({
                "model": self.model,
                "prompt": prompt,
                "stream": false
            }))
            .send()
            .await?;

        let resp_text: String = resp.json().await?;

        // Parse JSON from response
        let result: LlmValidationResult = serde_json::from_str(&resp_text)
            .context("Failed to parse LLM validation response")?;

        Ok(result)
    }

    fn build_prompt(&self, req: &LlmValidationRequest) -> String {
        let docs_text = req.documents.iter()
            .enumerate()
            .map(|(i, d)| format!("[Doc {}] {}\n{}\n", i + 1, d.title, &d.content[..d.content.len().min(500)]))
            .collect::<Vec<_>>()
            .join("\n\n");

        format!(r#"
## 问题
{}

## 检索到的文档
{}

## 要求判断
1. 上述文档是否包含回答问题所需的信息？
2. 如果是，指出最相关的 1-3 篇文档，说明原因。
3. 如果否，说明缺失的核心信息是什么。

## 输出格式 (JSON)
{{
  "verdict": "PASS" | "PARTIAL" | "FAIL",
  "relevant_doc_ids": ["doc_id", ...],
  "reason": "...",
  "missing_info": "..." (可选)
}}
"#, req.query, docs_text)
    }
}
```

#### 2.3.3 集成到评估流程

```rust
// gmp-eval/src/main.rs
async fn evaluate_scenario(scenario: &Scenario, results: &[ScoredChunk]) -> EvalResult {
    // Step 1: 原始关键字验证 (fallback)
    let keyword_pass = keyword_match(scenario, results);

    // Step 2: LLM 验证 (P0 - 需要 Ollama)
    let llm_result = if ollama_available() {
        let validator = LlmValidator::new(OLLAMA_URL, "qwen2.5:14b");
        let req = LlmValidationRequest {
            query: scenario.query.clone(),
            documents: results.iter().take(5).map(|r| DocSnippet {
                doc_id: r.chunk_id.to_string(),
                title: r.doc_id.clone(),
                content: r.content.clone(),
            }).collect(),
            expected_doc_ids: scenario.expected_doc_ids.clone(),
        };
        validator.validate(&req).await.ok()
    } else {
        None
    };

    // 综合判定
    let verdict = match (keyword_pass, &llm_result) {
        (true, _) => Verdict::PASS,
        (false, Some(LlmValidationResult { verdict: PASS, .. })) => Verdict::PASS,
        (false, Some(LlmValidationResult { verdict: PARTIAL, .. })) => Verdict::PARTIAL,
        _ => Verdict::FAIL,
    };

    EvalResult { verdict, keyword_pass, llm_result }
}
```

#### 2.3.4 验收标准

| 条件 | 验证方法 |
|------|---------|
| LLM 返回有效的 JSON verdict | 运行 `cargo test test_llm_validator` |
| verdict=PASS 时 total_pass +1 | 对比有/无 LLM 验证的通过率 |
| Ollama 不可用时回退到关键字验证 | 关闭 Ollama，验证仍能运行 |

---

### IMPROVEMENT-4: 规则外置数据库 [P1]

**文件**: `crates/gmp-kg/src/rules.rs` (新文件)

#### 2.4.1 Schema

```sql
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

-- 示例数据
INSERT INTO extraction_rules (rule_type, pattern, target_field, target_value, weight, priority)
VALUES ('ENTITY', r'重大偏差', 'entity_type', '重大偏差', 1.0, 10);
```

#### 2.4.2 实现

```rust
pub struct ExtractionRuleEngine {
    storage: KgStorage,
}

impl ExtractionRuleEngine {
    pub fn load_rules(&self, rule_type: &str) -> Result<Vec<ExtractionRule>> {
        let rows = self.storage.scan("extraction_rules")?;
        Ok(rows.iter()
            .filter(|r| {
                doc_value_str(r, 1) == rule_type &&
                doc_value_int(r, 8) == 1  // enabled
            })
            .map(|r| ExtractionRule {
                pattern: Regex::new(&doc_value_str(r, 2)).ok(),
                target_field: doc_value_str(r, 3),
                target_value: doc_value_str(r, 4),
                weight: doc_value_float(r, 5),
            })
            .filter_map(|r| r.pattern.as_ref()?; r)
            .collect())
    }

    pub fn extract_entities(&self, text: &str) -> Vec<EntityMatch> {
        let rules = self.load_rules("ENTITY").unwrap_or_default();
        let mut matches = vec![];
        for rule in rules {
            for cap in rule.pattern.find_iter(text) {
                matches.push(EntityMatch {
                    surface: cap.as_str().to_string(),
                    entity_type: rule.target_value.clone(),
                    confidence: rule.weight,
                });
            }
        }
        matches
    }
}
```

#### 2.4.3 验收标准

| 条件 | 验证方法 |
|------|---------|
| `INSERT` 新规则后，`extract_entities` 能识别新实体 | 插入 `r'微生物限度'` → '检测' 规则，验证文本匹配 |
| `UPDATE enabled=0` 后规则失效 | 禁用规则，验证不匹配 |
| 规则按 priority 排序 | 相同文本匹配多条规则，验证优先级高的先匹配 |

---

### IMPROVEMENT-5: 并行通道检索 [P1]

**文件**: `crates/gmp-retrieval/src/engine.rs`

#### 2.5.1 当前 (串行)

```rust
pub async fn search(&self, query: &str) -> SearchResult {
    let rules = self.rule_channel.search(&query, 10).await;    // 等待
    let vectors = self.vector_channel.search(&query, 30).await; // 等待
    let fts = self.fts_channel.search(&query, 30).await;       // 等待
    let graph = self.graph_channel.search(&query, 10).await;   // 等待
    self.fuse_results(rules, vectors, fts, graph)
}
```

#### 2.5.2 改进 (并行)

```rust
pub async fn search(&self, query: &str) -> SearchResult {
    let (rules, vectors, fts, graph) = tokio::join!(
        self.rule_channel.search(&query, 10),
        self.vector_channel.search(&query, 30),
        self.fts_channel.search(&query, 30),
        self.graph_channel.search(&query, 10),
    );
    self.fuse_results(rules, vectors, fts, graph)
}
```

#### 2.5.3 验收标准

| 条件 | 验证方法 |
|------|---------|
| 并行执行时间 ≈ max(各通道时间) | 对比串行/并行耗时 |
| 结果与串行一致 | 两次搜索结果相同 |

---

### IMPROVEMENT-6: BM25 查询缓存 [P2]

**文件**: `crates/gmp-storage/src/cache.rs` (新文件)

```rust
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct FtsCache {
    cache: Arc<RwLock<LruCache<String, Vec<ScoredChunk>>>>,
}

impl FtsCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(LruCache::new(capacity))),
        }
    }

    pub async fn search(&self, fts: &InvertedIndex, query: &str, top_k: usize) -> Vec<ScoredChunk> {
        let key = self.normalize_query(query);
        let cache = self.cache.read().await;
        if let Some(results) = cache.get(&key) {
            return results.clone();
        }
        drop(cache);

        let results = fts.search_with_scores(query, top_k);

        let mut cache = self.cache.write().await;
        cache.put(key, results.clone());
        results
    }

    fn normalize_query(&self, query: &str) -> String {
        query.split_whitespace().collect::<Vec<_>>().join(" ")
    }
}
```

---

### IMPROVEMENT-7: HNSW 参数调优 [P2]

**文件**: `crates/gmp-storage/src/storage.rs`

```rust
pub fn auto_tune_hnsw(&self) -> HnswConfig {
    let doc_count = self.count_embeddings().unwrap_or(0);

    match doc_count {
        n if n < 10_000 => HnswConfig {
            m: 16,
            ef_construction: 100,
            ef_search: 50,
        },
        n if n < 100_000 => HnswConfig {
            m: 32,
            ef_construction: 200,
            ef_search: 100,
        },
        _ => HnswConfig {
            m: 48,
            ef_construction: 400,
            ef_search: 200,
        },
    }
}
```

---

## 3. 实施计划

### Phase 1: P0 改进 (Week 1)

| Week | Task | Owner | PR |
|------|-------|-------|-----|
| Day 1-2 | IMPROVEMENT-1: LRU Embedding 缓存 | Agent | PR-141 |
| Day 3-4 | IMPROVEMENT-2: 同义词关系表 | Agent | PR-142 |
| Day 5-7 | IMPROVEMENT-3: LLM 辅助验证 | Agent | PR-143 |

### Phase 2: P1 改进 (Week 2)

| Week | Task | Owner | PR |
|------|-------|-------|-----|
| Day 8-9 | IMPROVEMENT-4: 规则外置数据库 | Agent | PR-144 |
| Day 10-11 | IMPROVEMENT-5: 并行通道检索 | Agent | PR-145 |

### Phase 3: P2 改进 (Week 3)

| Week | Task | Owner | PR |
|------|-------|-------|-----|
| Day 12-13 | IMPROVEMENT-6: BM25 缓存 | Agent | PR-146 |
| Day 14 | IMPROVEMENT-7: HNSW 调优 | Agent | PR-147 |

### Phase 4: 集成测试 (Day 15)

- 全量 408 场景回归测试
- 性能基准对比 (v1.3.0 vs v1.3.1)
- 发布 v1.3.1

---

## 4. 风险与缓解

| 风险 | 概率 | 影响 | 缓解 |
|------|------|------|------|
| Ollama 服务不稳定 | 中 | LLM 验证失效 | 回退到关键字验证 |
| 缓存内存占用过高 | 低 | OOM | LRU 容量限制 + TTL |
| 正则规则性能问题 | 低 | 匹配慢 | 预编译正则，按 priority 排序 |
| 同义词迁移丢数据 | 中 | 历史数据丢失 | 迁移前备份，验证条数 |

---

## 5. 验收测试

### 5.1 单元测试

```bash
cargo test -p gmp-retrieval -- embedding_cache
cargo test -p gmp-kg -- synonyms
cargo test -p gmp-eval -- llm_validator
```

### 5.2 集成测试

```bash
# 408 场景
cargo run --release -p gmp-cli -- evaluate --db ./data/gmp.db

# 性能基准
./scripts/benchmarks/benchmark.sh --runs 100 --concurrency 10
```

### 5.3 性能目标

| 指标 | v1.3.0 基线 | v1.3.1 目标 | 改善 |
|------|------------|------------|------|
| P50 延迟 | 931 ms | < 500 ms | -46% |
| P95 延迟 | 3840 ms | < 2000 ms | -48% |
| 408 通过率 | 88% | > 90% | +2pp |
| 同义词查询 | O(n) JSON解析 | O(1) SQL | 高效 |

---

*最后更新: 2026-05-24*
