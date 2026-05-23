# GMP-Platform 五层架构 — 离线知识抽取管线实现计划

> 版本: v1.1.0 | 日期: 2026-05-22 | 状态: 规划中
> 基于 DeepSeek 分析 + 用户决策

---

## 一、现有四层架构问题分析

### 1.1 当前架构

```
Layer 4: LLM 问答层     ← Ollama + 意图分类 + 答案生成
Layer 3: Reranker 层   ← BGE-Reranker-v2-m3 重排
Layer 2: 检索引擎层     ← 四通道: 规则/向量/BM25/图谱 + RRF
Layer 1: 数据存储层     ← SQLRustGo (RDBMS + Vector + FTS + Graph)
```

### 1.2 缺失的核心能力

| 缺失能力 | 影响 | 体现 |
|---------|------|------|
| **GMP 实体识别 (NER)** | 无法理解文档中的领域概念 | "偏差调查" ≠ "偏差处理" 被视为不同词 |
| **语义关系抽取** | 无法建立文档间深层关联 | 跨文档流程-结果关系丢失 |
| **多维文档分类** | 缺乏稳定的 GMP 分类体系 | 检索结果缺乏结构性 |
| **相关性预计算** | 查询时无预判文档质量 | 大量无关文档进入重排 |
| **术语标准化** | 同义词未映射到标准术语 | "OOS" ≠ "超标" 检索丢失 |

### 1.3 16 个失败场景的根因

当前评测 `expected_doc_ids = ["审计计划", "审计"]`，但 FTS 只做字面匹配：
- "审计计划" 必须在文档内容中出现
- 若文档写的是"年度审计方案"（同义词），则检索失败
- 根本原因：**没有术语标准化 + 没有语义扩展**

---

## 二、五层目标架构

### 2.1 架构图

```
┌─────────────────────────────────────────────────────────────────┐
│ Layer 5: LLM 推理与问答                                         │
│   Ollama + 意图分类 + 多跳推理 + 答案生成 + 可解释引用            │
├─────────────────────────────────────────────────────────────────┤
│ Layer 4: Reranker 重排                                          │
│   BGE-Reranker-v2-m3 (Cross-Encoder) + RRF 融合                 │
├─────────────────────────────────────────────────────────────────┤
│ Layer 3: 检索引擎层                                             │
│   四通道: 规则/向量/BM25/图谱 → Top-30 → RRF(k=60) → Top-20    │
├─────────────────────────────────────────────────────────────────┤
│ Layer 2: 知识图谱层                                             │
│   GMP 实体节点 + 语义关系边 + Cypher 查询 + 路径推理             │
│   ─ 节点类型: SOP/Deviation/CAPA/Validation/Device/Material 等  │
│   ─ 关系类型: TRIGGERS/BELONGS_TO/MANAGED_BY/REQUIRES/PRECEDES  │
├─────────────────────────────────────────────────────────────────┤
│ Layer 1: 统一数据存储层 (SQLRustGo)                             │
│   RDBMS: gmp_documents + gmp_document_chunks                    │
│   Vector: gmp_embeddings (HNSW, bge-m3 1024d)                   │
│   FTS: gmp_chunks_fts (BM25 + GMP 词典)                         │
│   Graph: graph_nodes + graph_edges (DiskGraphStore + WAL)        │
├─────────────────────────────────────────────────────────────────┤
│ Layer 0: 离线知识抽取管线 ★ 新增 ★                              │
│   ┌──────────────────────────────────────────────────────────┐  │
│   │ Step 0: GMP 术语库构建                                    │  │
│   │   └── 600+ 标准术语 + 300+ 同义词组 + 50+ 上下位关系      │  │
│   │ Step 1: NER 实体识别 → 实体标准化 → 实体库               │  │
│   │ Step 2: 关系抽取 (规则模板 + LLM 辅助) → 关系库          │  │
│   │ Step 3: 多维分类 → taxonomy_doc_mapping                 │  │
│   │ Step 4: 相关性预评分 → relevance_scores                 │  │
│   │ Step 5: 图谱写入 (节点 + 边) → Layer 2                  │  │
│   └──────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 Layer 0 与其他层的关系

```
Layer 0 输出 → Layer 2 (知识图谱)
Layer 0 输出 → Layer 1 (补充字段: entity_tags, taxonomy_path, relevance_score)
Layer 0 输出 → Layer 3 (检索时可选: 过滤/增强/相关性排序)
Layer 0 输出 → Layer 4 (答案引用可溯源到图谱实体路径)
```

---

## 三、Layer 0: 离线知识抽取管线详细设计

### 3.1 Step 0: GMP 术语库构建

**目标**: 建立 GMP 领域的标准术语体系，解决同义词/上下位词问题。

**数据来源**:
1. 从现有 `gmp-md/` 文档中统计高频词（TF-IDF 筛选）
2. 参考 `agent-knowledge/L0-trust-zone/rules/` 中的领域关键词
3. 参考 GMP 法规原文中的标准表述

**术语库结构**:

```sql
CREATE TABLE gmp_terms (
    term_id       INTEGER PRIMARY KEY,
    canonical_term TEXT NOT NULL,        -- 标准词: "偏差"
    term_type     TEXT NOT NULL,        -- 'entity'|'process'|'regulation'|'document'
    synonyms      TEXT NOT NULL,        -- JSON数组: ["偏离","异常","偏差事件"]
    hypernyms     TEXT,                 -- 上位词: "质量事件"
    hyponyms      TEXT,                 -- 下位词: ["重大偏差","一般偏差"]
    gmp_chapter   TEXT,                 -- 关联GMP章节: "第7章"
    created_at    INTEGER
);

-- 示例
-- term_id=1, canonical_term="偏差", term_type="entity"
-- synonyms=["偏离","异常","偏差事件","偏差情况"]
-- hypernym="质量事件"
-- hyponyms=["重大偏差","一般偏差","微小偏差"]
```

**初始规模**: 600+ 标准术语，300+ 同义词组

**实施步骤**:

| 步骤 | 任务 | 工具/方法 |
|------|------|----------|
| 0.1 | 从 GMP 法规原文提取名词短语 | 正则 + 词性标注 |
| 0.2 | 聚合同义词（基于共现矩阵） | Python + jieba |
| 0.3 | 人工审核 + 修正 | 领域专家 |
| 0.4 | 建立上下位层次关系 | 手工构建 |
| 0.5 | 导入 SQLite `gmp_terms` 表 | SQL |

### 3.2 Step 1: NER 实体识别

**目标**: 从每篇文档中识别 GMP 领域实体，并标准化到术语库。

**实体类型**:

| 实体类型 | 示例 | 识别难度 |
|---------|------|---------|
| SOP/规程 | "清洁验证方案", "偏差调查程序" | 低 |
| 偏差类型 | "重大偏差", "一般偏差", "OOS" | 低 |
| 设备 | "混合机", "压片机", "烘干箱" | 中 |
| 物料 | "原料药", "辅料", "中间体" | 低 |
| 人员角色 | "质量授权人", "生产负责人" | 低 |
| 验证类型 | "IQ", "OQ", "PQ", "清洁验证" | 低 |
| 法规条款 | "GMP第84条", "EU GMP Annex 11" | 中 |
| 风险等级 | "高风险", "中风险", "低风险" | 低 |

**实现方案**: 基于规则 + LLM 辅助

```rust
// 方案A: 基于规则的 NER（优先，实现简单）
pub struct GmpNerExtractor {
    term_dict: HashSet<String>,        // 从 gmp_terms 加载
    phrase_patterns: Vec<Regex>,       // 如 r"\w+(验证|管理|规程|程序)"
    context_templates: Vec<String>,    // 如 "XXX偏差"
}

// 方案B: LLM 辅助（召回率高，但慢）
pub struct LlmNerExtractor {
    llm: OllamaClient,
    prompt_template: String,
}
```

**输出**:

```sql
CREATE TABLE doc_entities (
    id          INTEGER PRIMARY KEY,
    doc_id      TEXT NOT NULL,
    entity_type TEXT NOT NULL,         -- 'SOP'|'Deviation'|'Device'|...
    surface_form TEXT NOT NULL,        -- 原文中的表面形式
    canonical_term TEXT NOT NULL,      -- 标准化到 gmp_terms
    start_pos   INTEGER,
    end_pos     INTEGER,
    confidence  REAL DEFAULT 1.0,
    created_at  INTEGER
);
```

### 3.3 Step 2: 关系抽取

**目标**: 识别实体间的语义关系，构建图谱边。

**关系类型**（预定义 + 可扩展）:

| 关系 | 方向 | 示例 |
|------|------|------|
| BELONGS_TO | SOP → Chapter | "清洁验证 → 第7章" |
| MANAGED_BY | SOP → Department | "偏差调查 → 质量部" |
| TRIGGERS | Deviation → CAPA | "偏差-001 → CAPA-003" |
| REQUIRES | Device → Validation | "混合机 → IQ/OQ/PQ" |
| PRECEDES | SOP → SOP | "清场 → 生产" |
| MANDATES | Regulation → SOP | "GMP第84条 → 偏差调查程序" |
| GENERATES | AuditFinding → CAPA | "审计发现-001 → CAPA-005" |
| VERIFIED_BY | CAPA → Validation | "CAPA-003 → 清洁验证" |

**抽取方法**:

1. **规则模板**（高精度）:
   - `"X 偏差 触发 Y CAPA" → TRIGGERS
   - `"X 规程 属于 Y 章节" → BELONGS_TO
   - `"设备 X 需要 Y 验证" → REQUIRES

2. **LLM 辅助抽取**（召回率）:
   - 对规则无法覆盖的关系，用 LLM 判断

```rust
pub struct RelationExtractor {
    rule_templates: Vec<RelationTemplate>,
    llm: Option<OllamaClient>,    // 可选，用于复杂关系
}

pub struct RelationTemplate {
    pattern: Regex,
    relation_type: RelationType,
    direction: (EntityType, EntityType),  // src → dst
}
```

### 3.4 Step 3: 多维文档分类

**目标**: 为每篇文档打 GMP 分类标签，支持分类过滤检索。

**分类维度**:

| 维度 | 值域 | 示例 |
|------|------|------|
| **主题** | 偏差/变更/验证/设备/物料/文件/人员/厂房 | 偏差 |
| **子主题** | 偏差调查/CAPA/偏差分级/... | 偏差调查 |
| **文档类型** | SOP/记录/报告/规程/法规/标准 | SOP |
| **法规体系** | GMP/EU GMP/FDA/WHO/中国GMP | 中国GMP |
| **风险等级** | 高/中/低 | 高 |
| **生命周期阶段** | 研发/上市/退市 | 上市 |
| **部门归属** | 生产部/质量部/工程部/仓储部 | 质量部 |

**分类实现**:

```rust
// 方法: 规则 + LLM 二阶段
pub struct DocClassifier {
    // 阶段1: 规则快速分类（基于标题/关键词）
    rule_classifier: RuleBasedClassifier,
    // 阶段2: LLM 精细分类（针对规则未命中的文档）
    llm_classifier: Option<LlmClassifier>,
}

impl DocClassifier {
    pub fn classify(&self, doc: &Document) -> DocTaxonomy {
        // 1. 提取标题关键词
        let title_kw = extract_keywords(&doc.title);
        // 2. 规则匹配
        if let Some(tax) = self.rule_classifier.match(&title_kw) {
            return tax;
        }
        // 3. LLM 分类（可选，成本高）
        self.llm_classifier.as_ref().map(|c| c.classify(doc))
    }
}
```

**输出**:

```sql
CREATE TABLE doc_taxonomy (
    doc_id      TEXT PRIMARY KEY,
    topic       TEXT,                  -- '偏差'
    sub_topic   TEXT,                  -- '偏差调查'
    doc_type    TEXT,                  -- 'SOP'
    regulation  TEXT,                  -- '中国GMP'
    risk_level  TEXT,                  -- '高'
    department  TEXT,                  -- '质量部'
    confidence  REAL DEFAULT 1.0,
    created_at  INTEGER
);
```

### 3.5 Step 4: 相关性预评分

**目标**: 在索引阶段预计算"文档与 GMP 核心领域的相关程度"，检索时可用于过滤/增强。

**评分维度**:

| 维度 | 权重 | 计算方法 |
|------|------|---------|
| 术语密度 | 0.3 | 标准 GMP 术语出现频率 / 文档长度 |
| 法规引用度 | 0.25 | 引用 GMP 条款数量 / 文档长度 |
| 流程完整性 | 0.2 | 是否包含"目的/范围/步骤/记录"四段式 |
| 实体密度 | 0.15 | 识别出的 GMP 实体数量 / 文档长度 |
| 部门覆盖度 | 0.1 | 涉及部门数量 |

**输出**:

```sql
CREATE TABLE doc_relevance (
    doc_id       TEXT PRIMARY KEY,
    overall_score REAL NOT NULL,       -- 0.0 ~ 1.0
    term_density  REAL,
    reg_citation  REAL,
    process_completeness REAL,
    entity_density REAL,
    dept_coverage REAL,
    created_at   INTEGER
);
```

### 3.6 Step 5: 图谱写入

**目标**: 将抽取的实体和关系批量写入 Layer 2（知识图谱层）。

**写入流程**:

```
NER 输出 (doc_entities)
       ↓
关系抽取输出 (doc_relations)
       ↓
多维分类输出 (doc_taxonomy)
       ↓
相关性评分输出 (doc_relevance)
       ↓
   ┌────────────────────────┐
   │   图谱批量写入模块      │
   │  GraphStore.batch_write() │
   └────────────────────────┘
       ↓
Layer 2: 知识图谱更新
       ↓
Layer 1: 文档表补充字段
       ↓
Layer 3: 检索通道可选使用图谱数据
```

---

## 四、实现计划

### 4.1 工作阶段

```
Phase 0: 术语库构建（Week 1）- 独立于代码
Phase 1: Layer 0 管线实现（Week 2-3）- NER + 关系抽取 + 分类
Phase 2: Layer 2 增强（Week 4）- 图谱写入 + 查询接口
Phase 3: Layer 3 集成（Week 5）- 分类过滤 + 相关性增强检索
Phase 4: 测试与优化（Week 6）- 408 场景回归测试
```

### 4.2 详细任务

#### Phase 0: GMP 术语库构建（Week 1）

| 任务 | 负责人 | 工具 | 产出 |
|------|--------|------|------|
| T0.1 提取 GMP 文档高频术语 | 脚本 | Python + jieba | `term_candidates.txt` |
| T0.2 审核并标准化术语 | 领域专家 | 手工 | `gmp_terms.csv` |
| T0.3 建立同义词映射表 | 领域专家 | 手工 | `synonyms.json` |
| T0.4 建立上下位层次 | 领域专家 | 手工 | `taxonomy_hierarchy.json` |
| T0.5 导入 SQLite 表 | 脚本 | Python + sqlite3 | `gmp_terms` 表 |

**术语库初始规模目标**: 600+ 术语，300+ 同义词组

#### Phase 1: Layer 0 管线（Week 2-3）

| 任务 | 依赖 | 产出 |
|------|------|------|
| T1.1 NER 实体识别器 | T0.5 | `crates/gmp-ner/` crate |
| T1.2 关系抽取器 | T1.1 | `crates/gmp-re/` crate |
| T1.3 多维分类器 | T0.5 | `crates/gmp-classifier/` crate |
| T1.4 相关性评分器 | T1.1, T1.2 | `crates/gmp-scorer/` crate |
| T1.5 批量处理脚本 | T1.1~T1.4 | `scripts/kg/build_knowledge_graph.sh` |
| T1.6 全量文档处理 | T1.5 | `doc_entities` + `doc_relations` + `doc_taxonomy` + `doc_relevance` 表 |

**验收标准**: 全量 1696 篇文档处理完成，NER 召回率 > 85%（抽样验证）

#### Phase 2: Layer 2 增强（Week 4）

| 任务 | 依赖 | 产出 |
|------|------|------|
| T2.1 图谱写入接口扩展 | T1.6 | `GraphStore.batch_write()` |
| T2.2 实体节点去重合并 | T1.1 | 合并同义词实体节点 |
| T2.3 Cypher 查询增强 | T2.1 | 支持按实体类型/关系类型查询 |
| T2.4 图谱可视化（可选） | T2.3 | `/graph/viz` API |

#### Phase 3: Layer 3 集成（Week 5）

| 任务 | 依赖 | 产出 |
|------|------|------|
| T3.1 分类过滤通道 | T1.3, T2.1 | Layer 3 可按 taxonomy 过滤 |
| T3.2 相关性增强排序 | T1.4 | Layer 3 检索结果 + relevance_score |
| T3.3 术语扩展检索 | T0.3 | 同义词自动展开检索 |
| T3.4 实体链接检索 | T1.1 | 查询词 → 实体 → 相关文档 |

#### Phase 4: 测试与优化（Week 6）

| 任务 | 依赖 | 产出 |
|------|------|------|
| T4.1 408 场景回归测试 | T3.1~T3.4 | `eval report v1.1.0` |
| T4.2 16 个失败案例逐一分析 | T4.1 | `failures_analysis.md` |
| T4.3 NER/关系抽取精度评估 | T1.1, T1.2 | `ner_reports/` |
| T4.4 性能压测 | T4.1 | QPS/P95 报告 |

### 4.3 里程碑

| 里程碑 | 日期 | 说明 |
|--------|------|------|
| M0: 术语库就绪 | 2026-05-29 | 600+ 术语入库 |
| M1: Layer 0 管线完成 | 2026-06-05 | NER+RE+分类+评分全通 |
| M2: Layer 2 增强完成 | 2026-06-08 | 图谱写入 + 查询 |
| M3: Layer 3 集成完成 | 2026-06-10 | 分类过滤 + 实体链接 |
| M4: 408 场景测试 | 2026-06-12 | v1.1.0 RC1 |
| M5: GA | 2026-06-15 | v1.1.0 发布 |

---

## 五、关键技术决策

### 5.1 NER 实现选择

| 方案 | 精度 | 速度 | 成本 | 推荐 |
|------|------|------|------|------|
| 纯规则 | 高（GMP术语） | 快 | 低 | **✅ 优先** |
| 微调 BERT NER | 最高 | 慢 | 高（GPU） | 备选 |
| LLM zero-shot | 高 | 慢 | 中 | 备选 |

**决策**: 阶段1用纯规则 + 术语库，阶段2评估是否需要 LLM 辅助

### 5.2 图数据库选型

当前使用 SQLRustGo `graph crate`（内存 + DiskGraphStore WAL），无需额外图数据库。

### 5.3 术语库维护

术语库需随 GMP 法规更新而更新，建议：
- 每季度人工审核一次术语库
- 新文档入库时自动触发 NER
- NER 识别到未收录术语 → 标记为"待审核"

---

## 六、对现有系统的侵入性

| 组件 | 侵入程度 | 说明 |
|------|---------|------|
| Layer 1 (存储) | 低 | 仅补充新字段（entity_tags, taxonomy_path, relevance_score） |
| Layer 2 (检索) | 中 | 新增分类过滤、相关性排序选项 |
| Layer 3 (重排) | 低 | 新增 relevance_score 作为排序因子之一 |
| Layer 4 (LLM) | 无 | 不受影响 |
| gmp-eval 评测 | 中 | 需更新 expected_doc_ids 处理逻辑（keyword → doc_id 映射） |

---

## 七、验收标准

| 指标 | 当前基线 | 目标 |
|------|---------|------|
| 场景通过率 | 96.1% | **98%+** |
| 失败案例根因覆盖率 | 0%（未分析） | **100%** |
| NER 召回率 | N/A | **> 85%** |
| 术语覆盖度 | N/A | **> 90%** |
| 图谱节点数 | ~100（手工） | **> 5000** |
| 图谱边数 | ~200（手工） | **> 20000** |

---

## 八、风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|---------|
| 术语库人工审核工作量大 | 高 | 分批审核，优先处理高频术语 |
| NER 精度不足 | 中 | LLM 辅助二次校验 |
| 全量文档处理时间长 | 低 | 增量处理 + 并行化 |
| 图谱写入性能 | 中 | 批量写入 + WAL 异步刷盘 |
