# GMP-Platform v1.1.0 规格说明书

> 版本: v1.1.0 | 日期: 2026-05-22 | 状态: 草稿
> 基于 DeepSeek AI 分析 + 用户决策

---

## 一、背景与问题

### 1.1 当前四层架构的缺陷

| 缺失能力 | 影响 | 体现 |
|---------|------|------|
| GMP 实体识别 (NER) | 无法理解文档中的领域概念 | "偏差调查" ≠ "偏差处理" 被视为不同词 |
| 语义关系抽取 | 无法建立文档间深层关联 | 跨文档流程-结果关系丢失 |
| 多维文档分类 | 缺乏稳定的 GMP 分类体系 | 检索结果缺乏结构性 |
| 相关性预计算 | 查询时无预判文档质量 | 大量无关文档进入重排 |
| 术语标准化 | 同义词未映射到标准术语 | "OOS" ≠ "超标" 检索丢失 |

### 1.2 根因分析（16个失败场景）

当前评测 `expected_doc_ids = ["审计计划", "审计"]`，但 FTS 只做字面匹配：
- "审计计划" 必须在文档内容中出现
- 若文档写的是"年度审计方案"（同义词），则检索失败
- 根本原因：**没有术语标准化 + 没有语义扩展**

### 1.3 AI 分析结论

基于 DeepSeek + 行业最佳实践（参考 49 个网页）：
1. **必须增加 Layer 0 离线知识抽取管线** — 从"数据"到"知识"的转化
2. **Qwen2-14B 可用于 NER** — 通用 LLM 对 GMP 场景足够灵活，医疗专用模型非必须
3. **混合架构最优** — 规则 + LLM 辅助，平衡精度/成本/可解释性
4. **GMP 术语来源** — 现有 `agent-knowledge/L0-trust-zone/` YAML 文件可直接复用

---

## 二、目标架构

### 2.1 五层架构

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
├─────────────────────────────────────────────────────────────────┤
│ Layer 1: 统一数据存储层 (SQLRustGo)                             │
│   RDBMS: gmp_documents + gmp_document_chunks                    │
│   Vector: gmp_embeddings (HNSW, bge-m3 1024d)                  │
│   FTS: gmp_chunks_fts (BM25 + GMP 词典)                         │
│   Graph: graph_nodes + graph_edges (DiskGraphStore + WAL)        │
├─────────────────────────────────────────────────────────────────┤
│ Layer 0: 离线知识抽取管线 ★ 新增 ★                              │
│   Step 0: GMP 术语库 → Step 1: NER → Step 2: 关系抽取           │
│   Step 3: 多维分类 → Step 4: 相关性评分 → Step 5: 图谱写入      │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 Layer 0 数据流

```
GMP 文档库 (1696 篇)
       ↓
┌─────────────────────────────────────────────────────┐
│ Step 0: GMP 术语库                                  │
│   输入: YAML 实体定义 + 规则文件                   │
│   输出: gmp_terms 表 (600+ 术语, 300+ 同义词组)     │
└─────────────────────────────────────────────────────┘
       ↓
┌─────────────────────────────────────────────────────┐
│ Step 1: NER 实体识别                                │
│   方法: 规则（YAML 实体定义）+ Qwen2-14B 辅助      │
│   输出: doc_entities 表                             │
└─────────────────────────────────────────────────────┘
       ↓
┌─────────────────────────────────────────────────────┐
│ Step 2: 关系抽取                                    │
│   方法: 规则模板 + YAML 规则文件                   │
│   输出: doc_relations 表                           │
└─────────────────────────────────────────────────────┘
       ↓
┌─────────────────────────────────────────────────────┐
│ Step 3: 多维文档分类                                │
│   维度: topic/doc_type/risk_level/department        │
│   输出: doc_taxonomy 表                             │
└─────────────────────────────────────────────────────┘
       ↓
┌─────────────────────────────────────────────────────┐
│ Step 4: 相关性预评分                                │
│   维度: 术语密度/法规引用度/流程完整性/实体密度    │
│   输出: doc_relevance 表                           │
└─────────────────────────────────────────────────────┘
       ↓
┌─────────────────────────────────────────────────────┐
│ Step 5: 图谱写入                                    │
│   输出: Layer 2 知识图谱更新                        │
│   目标: >5000 节点, >20000 边                       │
└─────────────────────────────────────────────────────┘
```

---

## 三、GMP 实体与关系规范

### 3.1 实体类型（来源: agent-knowledge/L0-trust-zone/entities/）

| 实体类型 | 英文名 | 来源文件 | 示例 |
|---------|--------|---------|------|
| 设备 | Equipment | device.yaml | 混合机、压片机、烘干箱 |
| 物料 | Material | material.yaml | 原料药、辅料、中间体 |
| 人员角色 | Operator | operator.yaml | 质量授权人、生产负责人 |
| 验证活动 | ValidationActivity | validation-change-entities.yaml | IQ/OQ/PQ、工艺验证 |
| 变更申请 | ChangeRequest | validation-change-entities.yaml | 工艺变更、设备变更 |
| 风险评估 | RiskAssessment | validation-change-entities.yaml | FMEA、HAZOP |
| 偏差 | Deviation | (隐含) | 重大偏差、一般偏差 |
| CAPA | CAPA | (隐含) | 纠正措施、预防措施 |
| SOP/规程 | SOP | (隐含) | 清洁验证方案 |
| 法规条款 | Regulation | (隐含) | GMP第84条 |

### 3.2 关系类型

| 关系 | 方向 | 示例 | 来源 |
|------|------|------|------|
| BELONGS_TO | SOP → Chapter | "清洁验证 → 第7章" | rules/*.yaml |
| MANAGED_BY | SOP → Department | "偏差调查 → 质量部" | rules/*.yaml |
| TRIGGERS | Deviation → CAPA | "偏差-001 → CAPA-003" | deviation-capa-rules.yaml |
| REQUIRES | Device → Validation | "混合机 → IQ/OQ/PQ" | validation-rules.yaml |
| PRECEDES | SOP → SOP | "清场 → 生产" | workflows/*.yaml |
| MANDATES | Regulation → SOP | "GMP第84条 → 偏差调查程序" | qms-rules.yaml |
| GENERATES | AuditFinding → CAPA | "审计发现 → CAPA" | qms-rules.yaml |
| VERIFIED_BY | CAPA → Validation | "CAPA → 清洁验证" | deviation-capa-rules.yaml |

---

## 四、技术方案

### 4.1 NER 实现方案

**选择：混合架构（规则 + LLM）**

| 阶段 | 方法 | 工具 | 适用场景 |
|------|------|------|---------|
| MVP | 纯规则 | jieba + 正则 | 高频 GMP 术语（偏差、SOP、设备） |
| 扩展 | LLM 辅助 | Qwen2-14B (Ollama) | 复杂实体、上下文消歧 |
| 生产 | 微调 UIE | Qwen2-7B + LoRA | 大规模批量处理（可选） |

**NER Prompt 模板**：

```
从以下文本中，抽取 GMP 领域实体。
实体类型: {SOP, 偏差, 设备, 物料, 人员角色, 验证类型, 法规条款}

示例:
文本: "生产线上的混合机A在运行中转速不稳定，导致产品粒度不合规"
回答: {"设备": "混合机A", "物料": null, "SOP": null, "偏差": "产品粒度不合规"}

文本:
{user_input}

回答:
```

### 4.2 图数据库选型

**当前**：SQLRustGo graph crate（内存 + DiskGraphStore WAL）
**无需新增外部图数据库**

### 4.3 Qwen2-14B 部署

- 本地 Ollama 部署
- 显存需求：~12-16GB（14B 模型）
- 替代方案：Qwen2-7B（~8GB，适合 Mac Mini）

---

## 五、数据模型

### 5.1 gmp_terms 表

```sql
CREATE TABLE gmp_terms (
    term_id       INTEGER PRIMARY KEY,
    canonical_term TEXT NOT NULL,        -- 标准词: "偏差"
    term_type     TEXT NOT NULL,        -- 'entity'|'process'|'regulation'|'document'
    synonyms      TEXT NOT NULL,        -- JSON: ["偏离","异常","偏差事件"]
    hypernyms     TEXT,                 -- 上位词: "质量事件"
    hyponyms      TEXT,                 -- 下位词: ["重大偏差","一般偏差"]
    gmp_chapter   TEXT,                 -- 关联GMP章节: "第7章"
    created_at    INTEGER
);
```

### 5.2 doc_entities 表

```sql
CREATE TABLE doc_entities (
    id          INTEGER PRIMARY KEY,
    doc_id      TEXT NOT NULL,
    entity_type TEXT NOT NULL,          -- 'SOP'|'Deviation'|'Device'|...
    surface_form TEXT NOT NULL,         -- 原文中的表面形式
    canonical_term TEXT NOT NULL,       -- 标准化到 gmp_terms
    start_pos   INTEGER,
    end_pos     INTEGER,
    confidence  REAL DEFAULT 1.0,
    created_at  INTEGER
);
```

### 5.3 doc_relations 表

```sql
CREATE TABLE doc_relations (
    id           INTEGER PRIMARY KEY,
    doc_id       TEXT NOT NULL,
    relation_type TEXT NOT NULL,        -- 'TRIGGERS'|'BELONGS_TO'|...
    src_entity   TEXT NOT NULL,
    dst_entity   TEXT NOT NULL,
    confidence   REAL DEFAULT 1.0,
    created_at   INTEGER
);
```

### 5.4 doc_taxonomy 表

```sql
CREATE TABLE doc_taxonomy (
    doc_id      TEXT PRIMARY KEY,
    topic       TEXT,                   -- '偏差'
    sub_topic   TEXT,                   -- '偏差调查'
    doc_type    TEXT,                   -- 'SOP'
    risk_level  TEXT,                   -- '高'
    department  TEXT,                   -- '质量部'
    confidence  REAL DEFAULT 1.0,
    created_at  INTEGER
);
```

### 5.5 doc_relevance 表

```sql
CREATE TABLE doc_relevance (
    doc_id       TEXT PRIMARY KEY,
    overall_score REAL NOT NULL,        -- 0.0 ~ 1.0
    term_density REAL,
    reg_citation REAL,
    process_completeness REAL,
    entity_density REAL,
    dept_coverage REAL,
    created_at   INTEGER
);
```

---

## 六、实施计划

### 6.1 阶段

| 阶段 | 名称 | 周期 | 核心产出 |
|------|------|------|---------|
| Phase 0 | 术语库构建 | Week 1 | 600+ 术语入库 |
| Phase 1 | Layer 0 管线 | Week 2-3 | NER+RE+分类+评分全通 |
| Phase 2 | Layer 2 增强 | Week 4 | 图谱写入 + 查询 |
| Phase 3 | Layer 3 集成 | Week 5 | 分类过滤 + 实体链接 |
| Phase 4 | 测试与优化 | Week 6 | 408 场景回归 |

### 6.2 里程碑

| 里程碑 | 日期 | 说明 |
|--------|------|------|
| M0: 术语库就绪 | 2026-05-29 | 600+ 术语入库 |
| M1: Layer 0 管线完成 | 2026-06-05 | NER+RE+分类+评分全通 |
| M2: Layer 2 增强完成 | 2026-06-08 | 图谱写入 + 查询 |
| M3: Layer 3 集成完成 | 2026-06-10 | 分类过滤 + 实体链接 |
| M4: RC1 | 2026-06-11 | 首次 RC 构建 |
| M5: GA | 2026-06-12 | v1.1.0 发布 |

### 6.3 详细任务

#### Phase 0: GMP 术语库构建（Week 1）

| 任务 | 依赖 | 产出 |
|------|------|------|
| T0.1 解析 YAML 实体文件，提取术语 | agent-knowledge | `term_candidates.json` |
| T0.2 从 YAML 规则文件提取同义词 | rules/*.yaml | `synonyms.json` |
| T0.3 从 CLASSIFICATION-INDEX 提取分类 | CLASSIFICATION-INDEX.md | `taxonomy_hierarchy.json` |
| T0.4 补充 GMP 法规标准术语 | 手工 | `gmp_regulation_terms.json` |
| T0.5 导入 SQLite gmp_terms 表 | T0.1~T0.4 | `gmp_terms` 表 |

**验收标准**: 600+ 术语，300+ 同义词组

#### Phase 1: Layer 0 管线（Week 2-3）

| 任务 | 依赖 | 产出 |
|------|------|------|
| T1.1 NER 实体识别器 | T0.5 | `crates/gmp-ner/` crate |
| T1.2 关系抽取器 | T1.1 | `crates/gmp-re/` crate |
| T1.3 多维分类器 | T0.5 | `crates/gmp-classifier/` crate |
| T1.4 相关性评分器 | T1.1, T1.2 | `crates/gmp-scorer/` crate |
| T1.5 批量处理脚本 | T1.1~T1.4 | `scripts/kg/build_knowledge_graph.sh` |
| T1.6 全量文档处理 | T1.5 | 4张新表数据 |

**验收标准**: 全量 1696 篇文档处理完成，NER 召回率 > 85%

#### Phase 2: Layer 2 增强（Week 4）

| 任务 | 依赖 | 产出 |
|------|------|------|
| T2.1 图谱写入接口扩展 | T1.6 | `GraphStore.batch_write()` |
| T2.2 实体节点去重合并 | T1.1 | 合并同义词实体节点 |
| T2.3 Cypher 查询增强 | T2.1 | 支持按实体/关系类型查询 |
| T2.4 图谱可视化（可选） | T2.3 | `/graph/viz` API |

#### Phase 3: Layer 3 集成（Week 5）

| 任务 | 依赖 | 产出 |
|------|------|------|
| T3.1 分类过滤通道 | T1.3, T2.1 | Layer 3 可按 taxonomy 过滤 |
| T3.2 相关性增强排序 | T1.4 | 检索结果 + relevance_score |
| T3.3 术语扩展检索 | T0.3 | 同义词自动展开检索 |
| T3.4 实体链接检索 | T1.1 | 查询词 → 实体 → 相关文档 |

#### Phase 4: 测试与优化（Week 6）

| 任务 | 依赖 | 产出 |
|------|------|------|
| T4.1 408 场景回归测试 | T3.1~T3.4 | `eval report v1.1.0` |
| T4.2 16 个失败案例分析 | T4.1 | `failures_analysis.md` |
| T4.3 NER/RE 精度评估 | T1.1, T1.2 | `ner_reports/` |
| T4.4 性能压测 | T4.1 | QPS/P95 报告 |

---

## 七、验收标准

| 指标 | 当前基线 | v1.1.0 目标 |
|------|---------|-------------|
| 场景通过率 | 96.1% | **98%+** |
| 失败案例根因覆盖率 | 0% | **100%** |
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

---

## 九、现有资源复用

### 9.1 agent-knowledge/L0-trust-zone/entities/

| 文件 | 内容 | 复用方式 |
|------|------|---------|
| device.yaml | 设备实体定义 | 直接导入 gmp_terms |
| material.yaml | 物料实体定义 | 直接导入 gmp_terms |
| operator.yaml | 人员角色定义 | 直接导入 gmp_terms |
| validation-change-entities.yaml | 验证/变更实体 | 直接导入 gmp_terms |

### 9.2 agent-knowledge/L0-trust-zone/rules/

| 文件 | 内容 | 复用方式 |
|------|------|---------|
| deviation-capa-rules-enhanced.yaml | 偏差/CAPA 规则 | 关系抽取模板 |
| qms-rules.yaml | QMS 规则 | 关系抽取模板 |
| change-control-rules.yaml | 变更控制规则 | 关系抽取模板 |
| validation-rules.yaml | 验证规则 | REQUIRES 关系 |
| facility-equipment-rules.yaml | 厂房设备规则 | 关系抽取模板 |
| production-rules.yaml | 生产规则 | 关系抽取模板 |
| data-integrity-doc-rules.yaml | 数据完整性规则 | 关系抽取模板 |
| material-warehouse-rules.yaml | 物料仓库规则 | 关系抽取模板 |
| weighing-rules.yaml | 称量规则 | 关系抽取模板 |

### 9.3 agent-knowledge/CLASSIFICATION-INDEX.md

- L0-trust-zone 分类体系 → doc_taxonomy 分类维度
- L1-sanitized-zone 分类体系 → 参考

---

**规格版本**: 1.0
**审核状态**: 草稿
**下次审查**: 2026-05-29 (M0 里程碑)
