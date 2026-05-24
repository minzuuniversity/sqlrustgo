# GMP-Platform SQLRustGo 自洽整改计划与进展报告

> **项目**: GMP-Platform v1.3.1 — SQLRustGo 存储层自洽整改
> **版本**: v1.3.1
> **日期**: 2026-05-24
> **状态**: 🔄 进行中
> **目标**: 完成 SPEC-v1.3.1-PERFORMANCE.md 中定义的 7 项改进
> **文档**: `docs/SPEC-v1.3.1-PERFORMANCE.md`

---

## 一、整改背景

### 1.1 问题来源

v1.3.0 基于 SQLRustGo 完成存储层重构，但经系统性调研（见 `docs/TECHNICAL_ANALYSIS.md`）发现以下问题：

| # | 问题类别 | 问题描述 | 严重度 |
|---|---------|---------|--------|
| P0-1 | JSON 存储 | `gmp_terms.synonyms/hypernyms/hyponyms` 使用 JSON 字符串，无法 SQL 查询 | 高 |
| P0-2 | 性能瓶颈 | Ollama API 每次查询都调用，网络开销 200-500ms | 高 |
| P0-3 | 测试可信度 | 408 场景测试用关键字匹配，无法验证语义正确性 | 高 |
| P1-1 | 硬编码规则 | 术语规则硬编码在 `run_layer0_pipeline.py`，不可热更新 | 中 |
| P1-2 | 串行检索 | 四通道串行执行，无法并行，浪费 30-50ms | 中 |
| P2-1 | 无 BM25 缓存 | 高频查询重复计算 BM25 | 低 |
| P2-2 | HNSW 未调优 | 向量索引参数未根据数据量自适应 | 低 |

### 1.2 性能基线（v1.3.0）

```
P50 延迟:  931 ms
P95 延迟:  3840 ms
408 通过率: 88%
同义词查询: O(n) JSON 解析
```

---

## 二、改进项执行记录

### 进度总览

| # | 改进项 | 优先级 | 状态 | PR | Commit |
|---|--------|--------|------|-----|--------|
| IMPROVEMENT-1 | LRU Embedding 缓存 | P0 | ✅ 已完成 | #142 | 3a762ab |
| IMPROVEMENT-2 | 同义词关系表 + SQL 初始化脚本 | P0 | ✅ 已完成 | #143 | 3c9c179 |
| IMPROVEMENT-3 | LLM 辅助 408 验证 | P0 | 🔄 进行中 | — | — |
| IMPROVEMENT-4 | tokio::join! 并行检索 | P1 | ⏳ 待执行 | — | — |
| IMPROVEMENT-5 | 规则外置数据库 | P1 | ✅ Schema 已完成 | — | — |
| IMPROVEMENT-6 | BM25 查询缓存 | P2 | ⏳ 待执行 | — | — |
| IMPROVEMENT-7 | HNSW 参数调优 | P2 | ⏳ 待执行 | — | — |

---

### IMPROVEMENT-1: LRU Embedding 缓存 ✅

**PR**: #142 (merged) — `feat(gmp-retrieval): LRU embedding cache — IMPROVEMENT-1 of v1.3.1`

**Commit**: 3a762ab (merged as 2138ca8)

#### 1.1 变更内容

**新增文件**:
- `crates/gmp-retrieval/src/embedding_cache.rs` — LRU 缓存实现

**修改文件**:
- `crates/gmp-retrieval/src/channel.rs` — `VectorChannel` 集成 `CachedEmbeddingStore`
- `crates/gmp-retrieval/src/lib.rs` — 导出新模块

#### 1.2 核心设计

```rust
pub struct EmbeddingCache {
    inner: Arc<RwLock<HashMap<String, CachedEntry>>>,  // LRU + TTL
    ttl_seconds: u64,       // 默认 86400s (24h)
    max_capacity: usize,    // 默认 10000 条
}

struct CachedEntry {
    embedding: Vec<f32>,
    created_at: Instant,       // TTL 计算
    last_accessed: Instant,    // LRU 计算
}
```

**LRU 算法**:
- `get()`: 命中时刷新 `last_accessed`（变为 MRU）
- `insert()`: 容量满时，淘汰 `last_accessed` 最旧的条目（LRU）
- 确定性淘汰：`min_by_key(|(_, entry)| entry.last_accessed)`

#### 1.3 性能收益

```
缓存命中: < 1ms (内存查找)
预期 P50 改善: 931ms → 400ms (改善 57%)
```

#### 1.4 测试验证

```
cargo test -p gmp-retrieval: 3/3 passed
  ✅ test_cache_basic     — 基础存取
  ✅ test_cache_ttl       — 24h TTL 过期
  ✅ test_cache_eviction  — LRU 确定性淘汰
cargo clippy --release --workspace -D warnings: ✅ 通过
```

---

### IMPROVEMENT-2: 同义词关系表 + SQL 初始化脚本 ✅

**PR**: #143 (merged) — `scripts(sql): add GMP-Platform SQL initialization + normalized term tables — IMPROVEMENT-2`

**Commit**: 3c9c179 (merged as b86c264)

#### 2.1 问题

```
原: gmp_terms.synonyms = '["偏离","异常"]'  (JSON 字符串)
- 无法 SQL 查询同义词
- 无法单独更新某条同义词
- 无法对同义词建索引
```

#### 2.2 解决方案

**新增两张表**:
```sql
CREATE TABLE term_synonyms (
    term_id INTEGER NOT NULL,
    synonym TEXT NOT NULL,
    FOREIGN KEY (term_id) REFERENCES gmp_terms(term_id)
);

CREATE TABLE term_hierarchy (
    term_id INTEGER NOT NULL,
    parent_term TEXT NOT NULL,
    hierarchy_type TEXT NOT NULL CHECK (hierarchy_type IN ('hypernym', 'hyponym')),
    FOREIGN KEY (term_id) REFERENCES gmp_terms(term_id)
);
```

**修改 `insert_term()`**: 将同义词写入 `term_synonyms`，层级写入 `term_hierarchy`，主表对应列设为 NULL

**修改 `scan_terms()`**: 从三张表联合重建 `GmpTerm`（向后兼容）

#### 2.3 SQL 初始化脚本

```
scripts/sql/
├── schema/
│   └── gmp_schema.sql           # 19 表，Layer 0-5 完整 Schema
├── data/
│   ├── gmp_terms.sql           # 15 术语 + 45 synonyms + 15 hierarchy
│   └── gmp_relations_taxonomy.sql  # 10 关系模板 + 22 分类规则
└── init_gmp_storage.sh          # 一键初始化 Shell 脚本
```

**Schema 统计**:
| 表类型 | 表名 | 说明 |
|--------|------|------|
| Layer 1 | `gmp_documents` | 源文档元数据 |
| Layer 1 | `gmp_document_chunks` | 文本分块 |
| Layer 1 | `gmp_chunks_fts` | 全文搜索索引 |
| Layer 1 | `gmp_embeddings` | 向量嵌入 (BLOB) |
| Layer 1 | `gmp_hnsw_meta` | HNSW 索引元数据 |
| Layer 2 | `gmp_terms` | GMP 术语主表 |
| Layer 2 | `term_synonyms` | **规范化同义词表** |
| Layer 2 | `term_hierarchy` | **层级关系表** |
| Layer 2 | `doc_entities` | 文档实体 |
| Layer 2 | `doc_relations` | 文档关系 |
| Layer 2 | `doc_taxonomy` | 文档分类 |
| Layer 2 | `doc_relevance` | 相关性评分 |
| Layer 0 | `extraction_rules` | **外置 NER 规则** |
| Layer 0 | `relation_templates` | **外置 RE 模板** |
| Layer 0 | `taxonomy_rules` | **外置分类规则** |
| Layer 0 | `gmp_term_patterns` | 术语关键字模式 |
| Layer 3 | `rerank_history` | 重排历史 |
| Layer 5 | `query_audit` | 查询审计 |

**数据统计**:
```
15  术语 (gmp_terms)
45  同义词 (term_synonyms)
15  层级关系 (term_hierarchy)
40  NER 抽取规则 (extraction_rules)
10  关系模板 (relation_templates)
22  分类规则 (taxonomy_rules)
30  术语关键字 (gmp_term_patterns)
```

#### 2.4 测试验证

```bash
# SQL 脚本验证
sqlite3 /tmp/test_gmp/gmp.db < scripts/sql/schema/gmp_schema.sql
sqlite3 /tmp/test_gmp/gmp.db < scripts/sql/data/gmp_terms.sql
sqlite3 /tmp/test_gmp/gmp.db < scripts/sql/data/gmp_relations_taxonomy.sql

结果:
19 tables, 45 synonyms, 15 hierarchy, 40 extraction rules ✅
cargo clippy --release --workspace -D warnings: ✅ 通过
```

---

### IMPROVEMENT-3: LLM 辅助 408 验证 🔄

**状态**: 🔄 进行中

**Spec**: `docs/SPEC-v1.3.1-PERFORMANCE.md` 第 2.3 节

#### 3.1 需求

当前 408 场景测试用**关键字匹配**验证：
```python
# 旧验证逻辑
def keyword_match(scenario, results):
    for doc in results:
        if all(kw in doc.content for kw in scenario.expected_keywords):
            return True
    return False
```

**问题**: 不验证文档是否真正回答了问题，只验证关键字出现。

**改进**: LLM 判断每篇文档是否真正回答了问题：
```rust
// Step 1: 检索候选文档 (Rule + Vector + FTS + Graph)
// Step 2: LLM 判断: "Does Doc-X answer Query-Y?"
// Step 3: 综合排序
// Step 4: 证据溯源验证
```

#### 3.2 实现规格

**新文件**: `crates/gmp-eval/src/llm_validator.rs`

**核心结构**:
```rust
pub struct LlmValidator {
    ollama_url: String,   // http://localhost:11434
    model: String,        // qwen2.5:14b
    client: Client,
}

pub async fn validate(&self, req: &LlmValidationRequest) -> Result<LlmValidationResult>

// LLM Prompt:
# Prompt: 给定 Query 和候选文档，判断每篇文档是否回答了问题
# Output: JSON { verdict: "PASS"|"PARTIAL"|"FAIL", relevant_doc_ids: [...] }
```

#### 3.3 集成到评估流程

```rust
async fn evaluate_scenario(scenario: &Scenario, results: &[ScoredChunk]) -> EvalResult {
    let keyword_pass = keyword_match(scenario, results);

    let llm_result = if ollama_available() {
        let validator = LlmValidator::new(OLLAMA_URL, "qwen2.5:14b");
        validator.validate(&req).await.ok()
    } else {
        None  // 回退到关键字验证
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

#### 3.4 验收标准

| 条件 | 验证方法 |
|------|---------|
| LLM 返回有效的 JSON verdict | `cargo test test_llm_validator` |
| verdict=PASS 时 total_pass +1 | 对比有/无 LLM 验证的通过率 |
| Ollama 不可用时回退到关键字验证 | 关闭 Ollama，验证仍能运行 |

---

### IMPROVEMENT-4: tokio::join! 并行检索 ⏳

**状态**: ⏳ 待执行

**Spec**: `docs/SPEC-v1.3.1-PERFORMANCE.md` 第 2.5 节

#### 4.1 当前（串行）

```rust
pub async fn search(&self, query: &str) -> SearchResult {
    let rules = self.rule_channel.search(&query, 10).await;    // 等待
    let vectors = self.vector_channel.search(&query, 30).await; // 等待
    let fts = self.fts_channel.search(&query, 30).await;       // 等待
    let graph = self.graph_channel.search(&query, 10).await;   // 等待
    self.fuse_results(rules, vectors, fts, graph)
}
```

#### 4.2 改进（并行）

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

#### 4.3 预期收益

```
总耗时: sum(各通道) → max(各通道)
节省: 30-50ms/查询
```

---

### IMPROVEMENT-5: 规则外置数据库 ✅ (Schema)

**状态**: ✅ Schema 已完成（数据迁移待执行）

**文件**: `scripts/sql/schema/gmp_schema.sql` — `extraction_rules`, `relation_templates`, `taxonomy_rules`, `gmp_term_patterns`

#### 5.1 Schema

```sql
CREATE TABLE extraction_rules (
    rule_id INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_type TEXT NOT NULL,
    pattern TEXT NOT NULL,
    canonical_form TEXT,
    priority INTEGER DEFAULT 5,
    active INTEGER DEFAULT 1
);

CREATE TABLE relation_templates (
    template_id INTEGER PRIMARY KEY AUTOINCREMENT,
    pattern TEXT NOT NULL,
    relation_type TEXT NOT NULL,
    src_type_hint TEXT,
    dst_type_hint TEXT,
    active INTEGER DEFAULT 1
);

CREATE TABLE taxonomy_rules (
    rule_id INTEGER PRIMARY KEY AUTOINCREMENT,
    dimension TEXT NOT NULL CHECK (dimension IN ('topic', 'doc_type', 'risk_level', 'department')),
    pattern TEXT NOT NULL,
    value TEXT NOT NULL,
    priority INTEGER DEFAULT 5,
    active INTEGER DEFAULT 1
);
```

#### 5.2 待执行

- 实现 `ExtractionRuleEngine` 从数据库加载规则
- 替换 `run_layer0_pipeline.py` 中的硬编码常量
- 规则热更新 API（可选）

---

### IMPROVEMENT-6: BM25 查询缓存 ⏳

**状态**: ⏳ 待执行

**Spec**: `docs/SPEC-v1.3.1-PERFORMANCE.md` 第 2.6 节

```rust
pub struct FtsCache {
    cache: Arc<RwLock<LruCache<String, Vec<ScoredChunk>>>>,
}
```

---

### IMPROVEMENT-7: HNSW 参数调优 ⏳

**状态**: ⏳ 待执行

**Spec**: `docs/SPEC-v1.3.1-PERFORMANCE.md` 第 2.7 节

```rust
pub fn auto_tune_hnsw(&self) -> HnswConfig {
    match self.count_embeddings().unwrap_or(0) {
        n if n < 10_000  => HnswConfig { m: 16, ef_c: 100, ef_s: 50 },
        n if n < 100_000 => HnswConfig { m: 32, ef_c: 200, ef_s: 100 },
        _                 => HnswConfig { m: 48, ef_c: 400, ef_s: 200 },
    }
}
```

---

## 三、风险与缓解

| 风险 | 概率 | 影响 | 缓解 |
|------|------|------|------|
| Ollama 服务不稳定 | 中 | LLM 验证失效 | 回退到关键字验证 |
| 缓存内存占用过高 | 低 | OOM | LRU 容量限制 + TTL |
| 同义词迁移丢数据 | 中 | 历史数据丢失 | 迁移前备份，验证条数 |
| 正则规则性能问题 | 低 | 匹配慢 | 预编译正则，按 priority 排序 |

---

## 四、验收测试

### 4.1 单元测试

```bash
cargo test -p gmp-retrieval -- embedding_cache
cargo test -p gmp-kg -- synonyms
cargo test -p gmp-eval -- llm_validator
```

### 4.2 集成测试

```bash
# 408 场景
cargo run --release -p gmp-cli -- evaluate --db ./data/gmp.db

# 性能基准
./scripts/benchmarks/benchmark.sh --runs 100 --concurrency 10
```

### 4.3 性能目标

| 指标 | v1.3.0 基线 | v1.3.1 目标 | 改善 |
|------|------------|------------|------|
| P50 延迟 | 931 ms | < 500 ms | -46% |
| P95 延迟 | 3840 ms | < 2000 ms | -48% |
| 408 通过率 | 88% | > 90% | +2pp |
| 同义词查询 | O(n) JSON 解析 | O(1) SQL | 高效 |

---

## 五、版本历史

| 版本 | 日期 | 改进项 | PR |
|------|------|--------|-----|
| v1.3.1-alpha | 2026-05-24 | IMPROVEMENT-1, 2 | #142, #143 |
| v1.3.1-beta | TBD | IMPROVEMENT-3, 4, 5, 6, 7 | TBD |
| v1.3.1 | TBD | 全量测试 + 发布 | TBD |

---

*最后更新: 2026-05-24*
