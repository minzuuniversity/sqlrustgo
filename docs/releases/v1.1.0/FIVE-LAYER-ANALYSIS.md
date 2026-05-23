# GMP Platform 五层架构实现分析报告

> 版本: v1.1.0 | 日期: 2026-05-22 | 分支: develop/v1.1.0 (已合并 PR #72)
> 基准: `7bc4360` Merge pull request #72

---

## 一、五层架构总览

```
┌─────────────────────────────────────────────────────────────────┐
│ Layer 5: LLM 推理与问答                                          │
│   gmp-llm — Ollama + 意图分类 + 答案生成                         │
├─────────────────────────────────────────────────────────────────┤
│ Layer 4: Reranker 重排                                           │
│   gmp-reranker — BGE-Reranker-v2-m3 Cross-Encoder              │
├─────────────────────────────────────────────────────────────────┤
│ Layer 3: 检索引擎层                                               │
│   gmp-retrieval — 四通道 RRF 融合                                │
│   gmp-storage — search_fts_enhanced (Phase 3 增强)               │
├─────────────────────────────────────────────────────────────────┤
│ Layer 2: 知识图谱层                                               │
│   gmp-graph — GmpGraphWriter → sqlrustgo-graph                  │
├─────────────────────────────────────────────────────────────────┤
│ Layer 1: 统一数据存储层                                          │
│   gmp-storage — sqlrustgo-storage (FileStorage)                  │
│   + Layer 0 表: gmp_terms / doc_entities / doc_relations         │
│   + Layer 0 表: doc_taxonomy / doc_relevance                     │
├─────────────────────────────────────────────────────────────────┤
│ Layer 0: 离线知识抽取管线 ★ 已实现 Phase 1-3 ★                  │
│   gmp-kg      — KgStorage (5张表的 CRUD)                        │
│   gmp-classifier — 规则分类器 (6维度)                            │
│   gmp-re      — 关系抽取器 (10+ 关系模板)                        │
│   gmp-scorer  — 相关性评分 (5维度加权)                           │
│   gmp-graph   — 图写入器 (Layer 0 → Layer 2)                     │
└─────────────────────────────────────────────────────────────────┘
```

---

## 二、Layer 0 实现详情

### 2.1 架构位置

Layer 0 是离线知识抽取管线，在文档入库后执行一次，预计算知识结构供后续层使用。

**数据流向**：
```
GMP 文档库 (gmp_documents)
       ↓
┌─────────────────────────────────────────┐
│ Step 0: 术语库构建                      │
│   gmp_terms.sql (796行, 508+ 术语)       │
│   gmp_terms.json (4040行, 同义词/层级)   │
└─────────────────────────────────────────┘
       ↓
┌─────────────────────────────────────────┐
│ Step 1: NER 实体识别                    │
│   gmp-classifier (规则分类)             │
│   输出 → doc_entities 表                │
└─────────────────────────────────────────┘
       ↓
┌─────────────────────────────────────────┐
│ Step 2: 关系抽取                        │
│   gmp-re (正则模板)                     │
│   输出 → doc_relations 表               │
└─────────────────────────────────────────┘
       ↓
┌─────────────────────────────────────────┐
│ Step 3: 多维分类                        │
│   6维度: topic/doc_type/risk/dept/etc   │
│   输出 → doc_taxonomy 表                │
└─────────────────────────────────────────┘
       ↓
┌─────────────────────────────────────────┐
│ Step 4: 相关性评分                      │
│   5维度加权: 术语密度/法规引用/完整性...  │
│   输出 → doc_relevance 表               │
└─────────────────────────────────────────┘
       ↓
┌─────────────────────────────────────────┐
│ Step 5: 图谱写入                        │
│   gmp-graph → sqlrustgo-graph           │
│   实体节点 + 关系边 → Layer 2            │
└─────────────────────────────────────────┘
```

### 2.2 核心模块分析

#### gmp-kg (Knowledge Graph Storage)

**文件**: `crates/gmp-kg/src/storage.rs` (320+ 行)

**功能**: 使用 `sqlrustgo_storage::FileStorage` 实现 Layer 0 的 5 张表存储。

**管理的表**:

| 表名 | 行数估计 | 说明 |
|------|---------|------|
| `gmp_terms` | 508+ | GMP 标准术语 + 同义词 + 层级 |
| `doc_entities` | ? | NER 抽取的实体 |
| `doc_relations` | ? | RE 抽取的关系 |
| `doc_taxonomy` | ? | 文档分类 (6维度) |
| `doc_relevance` | ? | 相关性评分 (5维度) |

**关键方法**:
```rust
// 术语
fn insert_term(&mut self, term: &GmpTerm) -> Result<()>
fn scan_terms(&self) -> Result<Vec<GmpTerm>>

// 实体
fn insert_entity(&mut self, entity: &DocEntity) -> Result<()>

// 关系
fn insert_relation(&mut self, relation: &DocRelation) -> Result<()>

// 分类
fn upsert_taxonomy(&mut self, taxonomy: &DocTaxonomy) -> Result<()>

// 评分
fn upsert_relevance(&mut self, relevance: &DocRelevance) -> Result<()>

// 统计
fn get_stats(&self) -> Result<KgStats>
```

**存储引擎依赖**: `sqlrustgo_storage::FileStorage` (非 rusqlite)

---

#### gmp-classifier (Document Classifier)

**文件**: `crates/gmp-classifier/src/classifier.rs` + `taxonomy.rs`

**分类维度** (6个):

| 维度 | 示例值 | 规则来源 |
|------|--------|---------|
| `topic` | 偏差/验证/设备/物料/文件/人员 | 标题关键词匹配 |
| `sub_topic` | 偏差调查/CAPA/偏差分级 | 内容模式 |
| `doc_type` | SOP/记录/报告/规程/法规 | 标题 + 内容 |
| `risk_level` | 高/中/低 | 关键词: "重大"/"高风险" |
| `department` | 质量部/生产部/工程部 | 内容中的部门词 |
| `regulation` | GMP/EU GMP/FDA/中国GMP | 法规引用 |

**分类方法**: 纯规则匹配，无 LLM 参与。

**置信度计算**:
```rust
// 每个维度 0.25，满分 1.0
taxonomy.confidence = matched_dimensions * 0.25
```

**规则结构** (来自 `taxonomy.rs`):
```rust
struct TaxonomyRule {
    id: u32,
    dimension: &'static str,    // "topic", "doc_type", etc.
    patterns: Vec<Regex>,
    value: &'static str,        // 匹配后返回的值
}
```

---

#### gmp-re (Relation Extractor)

**文件**: `crates/gmp-re/src/extractor.rs` + `relations.rs`

**关系模板** (10+ 种, 来自 `default_templates()`):

| 关系类型 | 方向 | 模式示例 |
|---------|------|---------|
| `TRIGGERS` | Deviation → CAPA | `(\w+)触发(\w+CAPA)` |
| `CAUSED_BY` | Deviation → RootCause | `(\w+)由(\w+)引起` |
| `MANAGED_BY` | SOP → Department | `(\w+)由(\w+)管理` |
| `REQUIRES` | Device → Validation | `(\w+)需进行(IQ\|OQ\|PQ)` |
| `BELONGS_TO` | SOP → Chapter | `(\w+)属于(\w+章)` |
| `GENERATES` | Audit → CAPA | `(\w+)生成(\w+CAPA)` |
| `VERIFIED_BY` | CAPA → Validation | `(\w+)需(\w+)验证` |
| `PRECEDES` | SOP → SOP | `(\w+)在(\w+)之前` |
| `MANDATES` | Regulation → SOP | `(\w+)要求(\w+)` |

**抽取流程**:
```rust
fn extract(&self, doc_id: &str, text: &str) -> RelationResult {
    for tmpl in &self.templates {
        for caps in tmpl.pattern.find_iter(text) {
            // 提取 src/dst 实体对
            relations.push(GmpRelation::new(src, dst, tmpl.type));
        }
    }
    // 按位置排序返回
}
```

**注意**: 当前实现仅为**内存中的规则匹配**，未持久化到 `doc_relations` 表。

---

#### gmp-scorer (Relevance Scorer)

**文件**: `crates/gmp-scorer/src/scorer.rs` + `types.rs`

**评分维度** (5个):

| 维度 | 权重 | 计算方法 |
|------|------|---------|
| `term_density` | 0.30 | 术语密度 = unique_terms/total_terms |
| `reg_citation` | 0.25 | GMP条款×2 + 章节×1.5 + 其他×1 / 10 |
| `process_completeness` | 0.20 | 5指标加权: 步骤/职责/判定/记录/附件 |
| `entity_density` | 0.15 | entity_count / max_entity_count |
| `dept_coverage` | 0.10 | 有部门=1.0, 无=0.0 |

**Overall Score**: 加权求和，范围 0.0 ~ 1.0

**元数据类型** `DocMetadata`:
```rust
struct DocMetadata {
    doc_id: String,
    title: String,
    content: String,
    doc_type: Option<String>,
    department: Option<String>,
    term_stats: TermStats,      // 术语统计
    reg_stats: RegStats,        // 法规引用统计
    process_stats: ProcessStats, // 流程完整性
    entity_count: usize,
}
```

---

#### gmp-graph (Graph Writer)

**文件**: `crates/gmp-graph/src/writer.rs`

**功能**: 将 Layer 0 抽取结果写入 `sqlrustgo-graph` 的 `InMemoryGraphStore`。

**节点类型**:
- `Entity` — GMP 实体节点 (去重合并)
- `Document` — 文档节点

**边类型**:
- `MENTIONED_IN` — 实体 → 文档 (表面形式出现)
- `TRIGGERS` / `CAUSED_BY` / `MANAGED_BY` / `REQUIRES` 等 — 实体间关系

**去重逻辑**:
```rust
// 同一 canonical_term + entity_type 只创建一个节点
let cache_key = format!("{}:{}", entity_type, canonical_term);
if let Some(&node_id) = self.entity_cache.get(&cache_key) {
    return node_id;  // 命中缓存
}
```

**依赖**: `sqlrustgo-graph` crate (InMemoryGraphStore)

---

### 2.3 Layer 0 数据表 (gmp_storage)

在 `gmp-storage` 的 SQLite 中实现（通过 rusqlite，**不同于** gmp-kg 的 FileStorage）：

**schema 来自** `crates/gmp-storage/src/storage.rs` 中的 `init_schema()`:

```sql
-- gmp_terms: GMP 标准术语库
CREATE TABLE gmp_terms (
    term_id       INTEGER PRIMARY KEY,
    canonical_term TEXT NOT NULL,     -- 标准词: "偏差"
    term_type     TEXT NOT NULL,     -- 'entity'|'process'|'regulation'|'document'
    synonyms      TEXT NOT NULL,      -- JSON: ["偏离","异常","偏差事件"]
    hypernyms     TEXT,               -- 上位词: "质量事件"
    hyponyms      TEXT,               -- 下位词: ["重大偏差","一般偏差"]
    gmp_chapter   TEXT,               -- 关联GMP章节: "第7章"
    entity_type   TEXT
);

-- doc_entities: NER 实体
CREATE TABLE doc_entities (
    id          INTEGER PRIMARY KEY,
    doc_id      TEXT NOT NULL,
    entity_type TEXT NOT NULL,        -- 'SOP'|'Deviation'|'Device'|...
    surface_form TEXT NOT NULL,       -- 原文中的表面形式
    canonical_term TEXT NOT NULL,     -- 标准化到 gmp_terms
    start_pos   INTEGER,
    end_pos     INTEGER,
    confidence  REAL DEFAULT 1.0
);

-- doc_relations: 关系
CREATE TABLE doc_relations (
    id           INTEGER PRIMARY KEY,
    doc_id       TEXT NOT NULL,
    relation_type TEXT NOT NULL,      -- 'TRIGGERS'|'BELONGS_TO'|...
    src_entity   TEXT NOT NULL,
    dst_entity   TEXT NOT NULL,
    confidence   REAL DEFAULT 1.0
);

-- doc_taxonomy: 分类
CREATE TABLE doc_taxonomy (
    doc_id      TEXT PRIMARY KEY,
    topic       TEXT,
    sub_topic   TEXT,
    doc_type    TEXT,
    regulation  TEXT,
    risk_level  TEXT,
    department  TEXT,
    confidence  REAL DEFAULT 1.0
);

-- doc_relevance: 相关性评分
CREATE TABLE doc_relevance (
    doc_id       TEXT PRIMARY KEY,
    overall_score REAL NOT NULL,      -- 0.0 ~ 1.0
    term_density REAL,
    reg_citation REAL,
    process_completeness REAL,
    entity_density REAL,
    dept_coverage REAL,
    confidence   REAL DEFAULT 1.0
);
```

---

## 三、Layer 1 → Layer 2 → Layer 3 集成

### 3.1 Layer 3 增强检索 (Phase 3)

**文件**: `crates/gmp-storage/src/storage.rs` 第 435-650 行

**新增方法**:

```rust
// T3.1: 分类过滤通道
pub fn search_taxonomy_filter(
    &self,
    topic: Option<&str>,
    doc_type: Option<&str>,
    department: Option<&str>,
    risk_level: Option<&str>,
) -> Result<Vec<String>>  // 返回 doc_ids

// T3.3: 术语扩展检索 (同义词)
pub fn get_synonyms(&self, term: &str) -> Result<Vec<String>>

// T3.4: 实体链接检索
pub fn search_by_entity(&self, entity_term: &str) -> Result<Vec<String>>

// T3.2: 相关性增强排序
pub fn search_fts_enhanced(
    &self,
    query: &str,
    top_k: usize,
    use_layer0_boost: bool,
) -> Result<Vec<SearchResult>>
```

**search_fts_enhanced 流程**:
```
输入: query + top_k + use_layer0_boost
         │
         ▼
    Step 1: 同义词扩展查询
         ├─ 分词 query
         ├─ 对每个 token 查询 get_synonyms()
         └─ 生成 expanded_query = "term1 OR syn1 OR syn2 OR term2 ..."
         │
         ▼
    Step 2: BM25 检索 (top_k * 2)
         │
         ▼
    Step 3 (if use_layer0_boost):
         ├─ 获取 relevance_score (doc_relevance.overall_score)
         ├─ 获取 entity_density
         └─ 计算 boost = 0.5 * relevance + 0.3 * entity_density
         │
         ▼
    Step 4: 重新排序 + truncate top_k
         │
         ▼
    输出: Vec<SearchResult>
```

### 3.2 Layer 0 数据门控

```rust
// has_layer0_data: 检查 Layer 0 数据是否已填充
pub fn has_layer0_data(&self) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM doc_relevance WHERE overall_score > 0",
        [], |row| row.get(0)
    )?;
    Ok(count > 0)
}
```

---

## 四、实现差距分析

### 4.1 已实现 vs 规划

| 规划项 | 状态 | 说明 |
|--------|------|------|
| T0.5 GMP术语库 (SQLite) | ✅ | 508术语, gmp_terms.sql |
| T1.1 NER 实体识别 | ⚠️ | gmp-classifier 规则分类 ≠ NER |
| T1.2 关系抽取 | ⚠️ | gmp-re 规则模板, 未持久化 |
| T1.3 多维分类 | ✅ | gmp-classifier, 6维度 |
| T1.4 相关性评分 | ✅ | gmp-scorer, 5维度 |
| T1.5 批量处理脚本 | ✅ | build_knowledge_graph.sh |
| T2.1 图谱批量写入 | ⚠️ | gmp-graph writer, 未集成到管线 |
| T3.1 分类过滤通道 | ✅ | search_taxonomy_filter() |
| T3.2 相关性增强排序 | ✅ | search_fts_enhanced() |
| T3.3 术语扩展检索 | ✅ | get_synonyms() + search_fts_enhanced |
| T3.4 实体链接检索 | ✅ | search_by_entity() |

### 4.2 关键缺失

#### 缺失 1: NER 未真正实现

**问题**: `gmp-classifier` 是**文档分类器**，不是 NER 实体识别器。

**区别**:
- 分类器: 输入 "偏差调查报告" → 输出 `{ topic: "偏差", doc_type: "SOP" }`
- NER: 输入 "偏差调查报告" → 输出 `{ entities: [{"type":"SOP","surface":"偏差调查程序","canonical":"偏差调查程序"}] }`

**现状**: `doc_entities` 表存在，但无实际写入代码。

#### 缺失 2: doc_relations 未持久化

**问题**: `gmp-re` 的 `RelationExtractor` 输出在内存中，未写入 `doc_relations` 表。

**验证**:
```bash
sqlite3 data/gmp.db "SELECT COUNT(*) FROM doc_relations;"
-- 预期: > 0
-- 实际: 需运行 build_knowledge_graph.sh 后验证
```

#### 缺失 3: gmp-kg 和 gmp-storage 存储引擎不统一

**问题**:
- `gmp-kg`: 使用 `sqlrustgo_storage::FileStorage` (v3.0.0)
- `gmp-storage`: 使用 `rusqlite` (SQLite wrapper)

两者底层不同，导致 Layer 0 数据分散在两个存储引擎。

#### 缺失 4: build_knowledge_graph.sh 未完全实现

**问题**: Shell 脚本框架完整，但各 Step 的实际调用为空:
```bash
# Step 1 NER - 仅有注释，无实际调用
if [ -f "$NER_BIN" ]; then
    "$NER_BIN" --db "$DB_PATH" process-all 2>/dev/null || \
    echo "  Note: gmp-ner CLI not implemented yet"
fi
```

---

## 五、测试数据验证

### 5.1 术语库

```bash
$ sqlite3 data/gmp.db "SELECT COUNT(*) FROM gmp_terms;"
508

$ sqlite3 data/gmp.db "SELECT COUNT(*) FROM doc_entities;"
0  # NER 未运行

$ sqlite3 data/gmp.db "SELECT COUNT(*) FROM doc_taxonomy;"
0  # 分类未运行

$ sqlite3 data/gmp.db "SELECT COUNT(*) FROM doc_relevance;"
0  # 评分未运行
```

### 5.2 391/408 场景测试结果

| 指标 | 值 | 分析 |
|------|-----|------|
| 通过率 | 21.8% (89/408) | FTS 通道为主 |
| 平均召回率 | 0.193 | 较低 |
| 平均 MRR | 0.106 | 较低 |
| Vector 通道 | 0/89 | 未启用 |

**根因**: Layer 0 数据（doc_entities/doc_taxonomy/doc_relevance）全为 0，增强检索无效。

---

## 六、修复路线图

### P0: 让 Layer 0 真正运转

| 任务 | 阻塞 | 工作量 |
|------|------|--------|
| 实现 NER → doc_entities 写入 | T1.1 | 中 |
| 实现 RE → doc_relations 写入 | T1.2 | 中 |
| 实现 分类 → doc_taxonomy 写入 | T1.3 | 小 |
| 实现 评分 → doc_relevance 写入 | T1.4 | 小 |
| 统一存储引擎 (gmp-kg → rusqlite) | 架构 | 大 |
| 运行 build_knowledge_graph.sh 全量处理 | P0 完成 | 中 |

### P1: 集成到检索管道

| 任务 | 说明 |
|------|------|
| retrieval/engine.rs 调用 search_fts_enhanced | Layer 3 增强 |
| retrieval/engine.rs 调用 search_taxonomy_filter | 分类过滤 |
| retrieval/engine.rs 集成 Cypher 图查询 | Layer 2 通道 |

---

## 七、代码统计

| Crate | 文件 | 行数 | 状态 |
|-------|------|------|------|
| gmp-kg | storage.rs | ~320 | 框架完成 |
| gmp-kg | types.rs | ~80 | 完成 |
| gmp-classifier | classifier.rs | ~120 | 完成 |
| gmp-classifier | taxonomy.rs | ~200 | 规则库 |
| gmp-re | extractor.rs | ~120 | 框架完成 |
| gmp-re | relations.rs | ~150 | 模板库 |
| gmp-scorer | scorer.rs | ~180 | 完成 |
| gmp-scorer | types.rs | ~100 | 元类型 |
| gmp-graph | writer.rs | ~150 | 完成 |
| gmp-storage | storage.rs (Phase 3) | ~220 | 完成 |

---

**报告生成时间**: 2026-05-22
**分析基于**: develop/v1.1.0 @ `7bc4360`
