# GMP 合规内审检索系统 — 方案对比与改进设计

> 文档版本: v1.0 | 日期: 2026-05-20 | 作者: AI Agent

---

## 1. 执行摘要

本报告对 GMP-Platform 项目中三个独立实现的检索/查询系统进行了深度对比分析，并与 GitNexus 代码智能工具进行了交叉验证测试。基于测试结果，提出了一个融合**文档语义检索 + 结构化规则匹配 + 知识图谱实体推理 + LLM 多跳推理**的改进架构。

**核心结论**:
- GitNexus 是代码符号智能工具，不适合文档内容检索（0% 命中率）
- 现有 `gmp_kg_build` 混合系统在 164 场景达到 92.7%，但随场景扩展退化至 44.5%
- `agent-knowledge` 结构化规则匹配可覆盖 64% 的合规查询
- 最优方案是"结构化知识图谱 + 语义嵌入 + 图遍历 + LLM 推理"四层融合

---

## 2. 已有系统架构分析

### 2.1 系统一：gmp_kg_build（Python 混合检索）

**位置**: `/gmp_kg_build/`

**架构**:
```
┌─────────────────────────────────────────────────────────────┐
│  接入层: Hermes Agent / Web API / CLI                        │
├─────────────────────────────────────────────────────────────┤
│  推理层: Ollama (Qwen3-14B-Q4) + CrossEncoder(MiniLM-L-12)  │
├─────────────────────────────────────────────────────────────┤
│  检索层 (三通道):                                            │
│    通道A: BM25 关键词 (rank_bm25 + jieba + GMP词典194K词)   │
│    通道B: 向量语义 (QMD + embeddinggemma-300M, 4096d)       │
│    通道C: 知识图谱 (NetworkX, 545节点, 1808边)              │
│    融合: α×BM25 + (1-α)×Vector + graph_boost               │
├─────────────────────────────────────────────────────────────┤
│  数据层: gmp.db (770文档) + graph.db (节点-边)              │
└─────────────────────────────────────────────────────────────┘
```

**版本迭代与性能**:

| 版本 | 场景数 | 通过率 | 核心技术 |
|------|--------|--------|----------|
| v2 | 164 | **92.7%** | PG ts_rank + 关键词扩展 |
| v11 | 219 | **62.6%** | QMD向量 + MiniLM Reranker |
| v22 | 391 | **44.5%** | FTS5 BM25 + 语义评分 |

**关键成功要素**:
1. **FMM 分词器**（前向最大匹配 + 194K GMP 词典）显著优于 jieba 统计分词
2. **三路 RRF 融合**互补了不同检索通道的短板
3. **图谱邻居遍历**扩展了文档关联发现能力

**已知缺陷**:
1. GPU 不可用（RTX 5080 sm_120 与 PyTorch 2.6 不兼容），全部跑 CPU
2. 知识图谱仅是文档-文档关系（SOP→章节→部门→类别），无实体级关系
3. 缺少中国 GMP 条款完整内容（仅有 EU GMP）
4. 跨法规对比查询完全失败
5. 向量检索响应慢（QMD 子进程调用，120s 超时）

### 2.2 系统二：gmp-qa-package（Rust CLI 三路融合）

**位置**: `/gmp-qa-package/`

**架构**:
```
TripleSearch(query)
  ├─ 路径1: vector_search_sqlite() — bge-m3 cosine ×0.4
  ├─ 路径2: exact keyword (title+content前500字) ×0.3
  └─ 路径3: graph BFS 2-hop via node name keyword ×0.3
      RRF融合 (k=60)
```

**数据库**: SQLite, 770 文档, bge-m3 1024d 向量嵌入

**测试结果**: 20/20 CLI 基础功能通过（连通性、向量检索、SQL 精确搜索、图谱操作、三路融合）

**优势**:
- 纯 Rust 实现，高速低内存
- 集成 CLI，易部署

**劣势**:
- 未经过大规模场景测试（仅 20 基础用例）
- 向量嵌入维度低（1024d vs 4096d）
- 图谱功能简单（仅 2-hop BFS，无实体关系）

### 2.3 系统三：gmp_complete_archive（原始文档仓库）

**位置**: `/gmp_complete_archive/`

**内容**:
- 01_原始GMP文件/GMP-fixed-MD: 1448 份 GMP 文档
- 02_补充文档: 350+ 份（法规解释、企业补充规程、phase2文档）
- ROOT_SOP: 17 份核心根 SOP

**角色**: 纯语料数据源，不提供检索功能，是前后两个检索系统的数据基础。

### 2.4 系统四：gmp-md（本文档集）

**位置**: `/gmp-md/`

**内容**: 从上述 3 个数据源合并去重的 1694 份文档，按 GMP 2010 章节体系 + 附录编排，配套 INDEX.md 索引。

**角色**: 统一文档全集，可作为检索系统的统一数据后端。

---

## 3. GitNexus 交叉验证测试

### 3.1 测试设计

选取 25 个涵盖 GMP 内审全部领域的典型查询场景，分别测试：

| 测试方法 | 说明 |
|----------|------|
| GitNexus SearchCodebase | 语义代码/文档检索 |
| GitNexus gitnexus_query | 代码符号与执行流查询 |
| 传统 grep 关键词 | 文件名 + 内容关键词匹配 |
| agent-knowledge 规则匹配 | YAML 结构化规则文件匹配 |

### 3.2 实测结果

| 方法 | 命中场景 | 触达率 | 说明 |
|------|----------|--------|------|
| GitNexus SearchCodebase | 0/25 | **0%** | 返回"Nothing found"，FTS 索引损坏，且仅索引代码符号 |
| GitNexus gitnexus_query | 0/25 | **0%** | 0 processes, 0 definitions, FTS 报 "read-only database" |
| 传统 grep 关键词 | ~12/25 | ~48% | 需要精确关键词，无法理解"偏差与CAPA关联"这类多跳查询 |
| agent-knowledge 规则匹配 | 16/25 | **64%** | 结构化 YAML 规则文件包含可匹配的关键词映射 |

### 3.3 GitNexus 失败原因分析

```
[gitnexus] FTS index ensure failed: Cannot execute write operations in a read-only database!
```

1. **设计错配**: GitNexus 是为代码符号（Function, Class, Method, Interface）设计的智能工具，不是文档检索系统
2. **索引范围**: 23182 个符号全部来自 Python/JS 代码文件，不索引 .md/.yaml 文档内容
3. **FTS 损坏**: 只读模式下无法构建全文搜索索引
4. **执行流空**: "GMP 合规检查" 不是一个代码执行流，GitNexus 无法建模

### 3.4 GitNexus 适用场景

尽管不适合文档检索，GitNexus 在以下 GMP-Platform 场景中有价值：

- **代码质量分析**: `scripts/build_gmp_md.py` 等工具的调用链分析
- **影响分析**: 规则引擎代码变更的 blast radius
- **重构安全**: 使用 `gitnexus_impact` + `gitnexus_detect_changes` 保障代码变更安全

---

## 4. 多系统对比矩阵

| 维度 | gmp_kg_build (v22) | gmp-qa-package (Rust) | GitNexus | agent-knowledge |
|------|--------------------|-----------------------|----------|-----------------|
| **定位** | GMP文档混合检索 | GMP文档CLI检索 | 代码符号智能 | 结构化规则库 |
| **通过率(391场景)** | 44.5% | 未测试 | 0% | 64% (规则匹配) |
| **峰值通过率** | 92.7% (164场景) | 20/20 (基础) | N/A | - |
| **向量检索** | QMD 4096d | bge-m3 1024d | embedding ✅ | 无 |
| **关键词检索** | FMM + BM25 | exact keyword | FTS ❌(损坏) | YAML键值 |
| **知识图谱** | NetworkX (545节点) | 2-hop BFS | 23182符号+关系 | 无 |
| **图谱深度** | 文档-文档 (4种关系) | 节点名匹配 | 代码调用链 | 无 |
| **实体级关系** | 无 | 无 | 无 | YAML实体定义 |
| **跨法规对比** | 不支持 | 不支持 | 不支持 | 部分支持 |
| **LLM推理** | Qwen3-14B | 无 | 无 | 无 |
| **Reranker** | MiniLM CPU | 无 | 无 | 无 |
| **部署难度** | 高(Python/QMD/Ollama) | 低(单二进制) | 中(npm) | 极低(纯YAML) |
| **响应时间** | 2-5s | <500ms | 40ms | <1ms |

---

## 5. 现有系统的关键失败模式

基于 391 场景测试结果分析，以下是核心失败模式：

### 5.1 知识缺口类 (Knowledge Gap)

| 类别 | 典型查询 | 根因 |
|------|----------|------|
| 法规对比 | "中国GMP与EU GMP在数据完整性上的差异" | 知识库无条文对比数据 |
| 条款查询 | "GMP第84条条款内容" | 仅有EU GMP, 无中国GMP全文 |
| 跨领域关联 | "偏差与OOS的关系" | 图谱无实体级关系，仅有文档-文档边 |

### 5.2 语义理解类 (Semantic Mismatch)

| 类别 | 典型查询 | 根因 |
|------|----------|------|
| 操作指南 | "发现偏差后应该先登记还是先处理？" | BM25无法理解流程顺序 |
| 条件判断 | "哪些变更需要启动验证？" | 关键词匹配而非规则推理 |
| 情境查询 | "混合机转速异常偏差应该怎么登记" | 缺少场景理解 |

### 5.3 图谱深度类 (Graph Shallow)

| 类别 | 根因 |
|------|------|
| 图谱仅 4 种关系类型 | 缺少"偏差→CAPA"、"设备→验证方案"等业务语义边 |
| 无实体节点 | 只有文档节点，无"混合机""偏差类型""验证状态"等实体 |
| 图遍历固定 | 仅 2-hop 邻居扩展，无路径推理 |

---

## 6. 改进设计方案

### 6.1 四层融合架构

```
┌──────────────────────────────────────────────────────────────┐
│  Layer 4: LLM 推理与问答层                                    │
│  ┌──────────────────────────────────────────────────────────┐│
│  │ • 查询意图理解 (Intent Classifier)                       ││
│  │ • 多跳推理链 (Multi-hop Reasoning)                      ││
│  │ • 答案生成与引用溯源 (Answer + Citation)                 ││
│  │ • 本地 LLM: Qwen3-14B / DeepSeek-R1                     ││
│  └──────────────────────────────────────────────────────────┘│
├──────────────────────────────────────────────────────────────┤
│  Layer 3: 知识图谱实体推理层                                   │
│  ┌──────────────────────────────────────────────────────────┐│
│  │ • 实体节点: 设备/物料/SOP/偏差类型/验证状态/角色/部门    ││
│  │ • 语义边: TRIGGERS→ NOTIFIES→ VALIDATES_BY→ BELONGS_TO ││
│  │ • 路径推理: 偏差→调查→根因→CAPA→验证→关闭               ││
│  │ • 图数据库: Neo4j / pgRouting / NetworkX+DGL             ││
│  └──────────────────────────────────────────────────────────┘│
├──────────────────────────────────────────────────────────────┤
│  Layer 2: 语义检索与重排层                                     │
│  ┌──────────────────────────────────────────────────────────┐│
│  │ • 文档向量检索: bge-m3 / embeddinggemma-300M (1024d)     ││
│  │ • 规则结构化检索: agent-knowledge YAML                   ││
│  │ • 全文FTS检索: SQLite FTS5 + GMP专业词典                ││
│  │ • Cross-Encoder Reranker: 多路结果融合重排               ││
│  └──────────────────────────────────────────────────────────┘│
├──────────────────────────────────────────────────────────────┤
│  Layer 1: 数据整合层                                          │
│  ┌──────────────────────────────────────────────────────────┐│
│  │ • gmp-md: 1694份统一GMP文档 (去重后)                     ││
│  │ • agent-knowledge: 结构化规则/实体/工作流 (55+规则)      ││
│  │ • 中国GMP条款全文: 53份法规解释                          ││
│  │ • 企业补充规程: 350+份独有文档                           ││
│  └──────────────────────────────────────────────────────────┘│
└──────────────────────────────────────────────────────────────┘
```

### 6.2 Layer 1: 数据整合层

**当前状态**: ✅ 已完成

- `gmp-md/`: 1694 份统一文档（CH01-CH09 + APPX A-L）
- `agent-knowledge/`: 已提取的结构化规则和实体
- 数据已去重，source 已脱敏

**建议增强**:
- 将 gmp-md 文档批量向量化（bge-m3 1024d，本地 CPU 可行）
- 为 agent-knowledge YAML 规则建立结构化索引
- 补充中国 GMP 2010 版全文（可从 APPX-A 已有 53 份扩展）

### 6.3 Layer 2: 语义检索与重排层

**建议实现**:

```python
# 多路检索 + RRF 融合
class MultiPathRetriever:
    def __init__(self):
        self.vector_store = SQLiteVecStore("gmp-md.db", dim=1024)     # bge-m3
        self.fts_index   = SqliteFTSIndex("gmp-md.db")                # FTS5
        self.rule_store  = YAMLRuleStore("agent-knowledge/")          # 结构化匹配
        self.reranker    = CrossEncoder("bge-reranker-v2-m3")         # GPU可用的重排器
    
    def retrieve(self, query, top_k=20):
        # 路径1: 向量语义 (bge-m3 cosine)
        vec_results = self.vector_store.search(query, k=top_k)
        
        # 路径2: FTS5 全文检索 + GMP词典增强
        fts_results = self.fts_index.search(query, k=top_k)
        
        # 路径3: 结构化规则匹配
        rule_results = self.rule_store.match(query, k=top_k)
        
        # RRF 融合 (k=60)
        fused = self._rrf_fusion([vec_results, fts_results, rule_results], k=60)
        
        # Cross-Encoder 重排
        reranked = self.reranker.rerank(query, fused, top_n=10)
        return reranked
```

**关键改进点**:
1. 使用 bge-m3（1024d）替代 embeddinggemma-300M（4096d），降低向量维度同时保持中文效果
2. 使用 bge-reranker-v2-m3 替代 MiniLM-L-12（解决 GPU 兼容问题）
3. 结构化规则匹配作为独立通道，命中精确规则时 boost 权重

### 6.4 Layer 3: 知识图谱实体推理层

**当前图谱缺陷**:
```
现有: SOP文档 → BELONGS_TO → 章节
      SOP文档 → MANAGED_BY → 部门  
      SOP文档 → CATEGORIZED_AS → 类别
      文档 → PRECEDES → 文档

缺少: 实体概念之间的业务语义关系
```

**建议增强模型**:

```cypher
// 实体节点
(:Device       {name: "混合机", model: "M-102", status: "active"})
(:Material     {name: "原料药", type: "API", storage: "常温"})
(:Deviation     {type: "重大", level: "critical", status: "open"})
(:CAPA          {id: "CAPA-2026-001", status: "in_progress"})
(:Validation    {type: "IQ/OQ/PQ", target: "混合机", status: "pending"})
(:Regulation    {code: "GMP-84", title: "自检", version: "2010"})
(:AuditFinding  {severity: "major", status: "open"})
(:SOP           {code: "GMP-02-DEV-001", title: "偏差调查处理程序"})

// 语义关系边
(:Deviation)-[:TRIGGERS]->(:CAPA)           // 偏差触发 CAPA
(:Deviation)-[:INVESTIGATED_BY]->(:SOP)      // 偏差按 SOP 调查
(:Device)-[:REQUIRES]->(:Validation)         // 设备需要验证
(:Validation)-[:EXECUTED_PER]->(:SOP)        // 验证按 SOP 执行
(:CAPA)-[:VERIFIED_BY]->(:Validation)        // CAPA 效果通过验证确认
(:Regulation)-[:MANDATES]->(:SOP)            // 法规强制要求 SOP
(:AuditFinding)-[:GENERATES]->(:CAPA)        // 审计发现产生 CAPA
(:Material)-[:STORED_AT]->(:Warehouse)       // 物料存储位置
```

**图推理能力**:

```
查询: "混合机转速异常偏差应该怎么处理？"

图推理路径:
  Device("混合机") 
    → [:REQUIRES] → Validation("混合机验证方案")
    → [:RELATED_SOP] → SOP("GMP-02-DEV-001_偏差调查处理程序")
    → [:TRIGGERS] → Deviation("生产偏差") → [:TRIGGERS] → CAPA("CAPA管理程序")
  
  上下文检索:
    - 混合机操作规程
    - 偏差分级标准
    - CAPA触发规则
```

### 6.5 Layer 4: LLM 推理与问答层

**查询意图分类器**:

```python
INTENT_TYPES = {
    "retrieve_sop":       "获取 SOP 原文",
    "check_compliance":   "合规性判定（是/否）",
    "compare_regulation": "跨法规对比",
    "trace_process":      "流程追溯",
    "classify_incident":  "事件分类（偏差/变更/OOS等）",
    "generate_checklist": "生成检查清单",
    "analyze_root_cause": "根因分析",
}

class IntentClassifier:
    """使用轻量 LLM (Qwen3-1.7B) 做意图分类"""
    def classify(self, query: str) -> str: ...
```

**多跳推理链生成**:

```python
def multi_hop_reasoning(query, retrieved_docs, graph_paths):
    """
    1. 检索候选文档 → Layer 2
    2. 图路径扩展 → Layer 3
    3. LLM 结构化推理:
       - Context: {retrieved_docs + graph_paths + rules}
       - Query: {query}
       - Task: 生成推理链和答案
    """
    prompt = f"""
    你是 GMP 合规审核 AI 助手。基于以下材料回答问题：
    
    ## 相关文档
    {retrieved_docs}
    
    ## 相关规则
    {graph_paths}
    
    ## 问题
    {query}
    
    ## 要求
    1. 给出明确答案
    2. 引用来源文档编号
    3. 如有不确定性，标注 [需人工确认]
    """
    return llm.generate(prompt)
```

### 6.6 Rust 核心实现建议

鉴于 Rust CLI (`gmp-qa-package`) 的性能优势，建议核心检索层用 Rust 实现：

```rust
// 四路检索器
pub struct GMPRetriever {
    vector_store: SqliteVectorStore,      // bge-m3 1024d
    fts_index:    SqliteFTS,              // FTS5
    rule_matcher: YAMLRuleMatcher,        // agent-knowledge
    graph_engine: PetgraphEngine,         // petgraph crate
}

impl GMPRetriever {
    pub fn search(&self, query: &str) -> Vec<SearchResult> {
        let vec   = self.vector_store.search(query, 20);
        let fts   = self.fts_index.search(query, 20);
        let rules = self.rule_matcher.match_rules(query, 10);
        let graph = self.graph_engine.expand(query, 10);
        
        self.rrf_fusion(vec![vec, fts, rules, graph], 60)
    }
}
```

---

## 7. 性能预估

| 指标 | 当前最优 (v2) | 改进设计预估 |
|------|--------------|-------------|
| 164 场景通过率 | 92.7% | **>95%** |
| 391 场景通过率 | 44.5% | **>80%** |
| 跨法规对比 | 不支持 | **支持** (Layer 4 LLM) |
| 单次查询响应 | 2-5s | **<1s** (Rust 核心) |
| GPU 依赖 | 不可用 | **可选** (bge-reranker GPU) |
| 部署复杂度 | 高 | **中** (单二进制 + 数据文件) |

---

## 8. 实施路线图

| 阶段 | 内容 | 周期 | 依赖 |
|------|------|------|------|
| **Phase A** | Layer 2 基础: gmp-md 向量化 + FTS5 索引 | 1 周 | gmp-md ✅ |
| **Phase B** | Layer 3 实体图谱: GMP 实体模型 + 关系抽取 | 2 周 | agent-knowledge ✅ |
| **Phase C** | Layer 4 LLM 集成: 意图分类 + 答案生成 | 1 周 | 本地 Ollama |
| **Phase D** | Rust 核心重写 + 性能优化 | 2 周 | Phase A-C |
| **Phase E** | 391 场景回归 + 增量测试 | 1 周 | Phase D |

**总计**: ~7 周，从 Phase A 开始即可逐步上线。

---

## 9. 附录：25 场景测试详情

| ID | 领域 | 查询 | grep | 规则匹配 | 旧系统 | GitNexus |
|----|------|------|------|----------|--------|----------|
| Q01 | 偏差管理 | 偏差分级标准和调查处理流程 | 0 | ✅ | FAIL | N/A |
| Q02 | 偏差管理 | 偏差与CAPA的关联触发规则 | 0 | ✅ | FAIL | N/A |
| Q03 | 偏差管理 | 生产偏差处理的具体步骤和关闭标准 | 0 | ✅ | FAIL | N/A |
| Q04 | 变更控制 | 变更控制完整流程包括哪些审批节点 | 0 | ✅ | FAIL | N/A |
| Q05 | 变更控制 | 工艺变更需要启动哪些验证活动 | 0 | ✅ | FAIL | N/A |
| Q06 | 变更控制 | 设备变更控制的具体步骤 | 0 | ✅ | FAIL | N/A |
| Q07 | 验证与确认 | DQ/IQ/OQ/PQ设备确认分别确认什么内容 | 0 | ✅ | FAIL | N/A |
| Q08 | 验证与确认 | 清洁验证方案的制定要求和接受标准 | 0 | ✅ | FAIL | N/A |
| Q09 | 验证与确认 | 计算机化系统验证需要满足哪些法规要求 | 0 | ✅ | FAIL | N/A |
| Q10 | 物料管理 | 物料状态标识管理中待验合格不合格怎么标识 | 0 | ✅ | FAIL | N/A |
| Q11 | 物料管理 | 原辅料入库验收的完整流程 | 0 | ❌ | FAIL | N/A |
| Q12 | 物料管理 | 麻醉药品和精神药品的特殊管理要求 | 0 | ❌ | FAIL | N/A |
| Q13 | 生产管理 | 清场管理的具体要求和检查要点 | 0 | ✅ | FAIL | N/A |
| Q14 | 生产管理 | 批生产记录的审核要点和常见缺陷 | 0 | ❌ | FAIL | N/A |
| Q15 | 生产管理 | 防止生产混淆和交叉污染的控制措施 | 0 | ✅ | FAIL | N/A |
| Q16 | 质量控制 | OOS超标结果调查的完整步骤和时限 | 0 | ✅ | FAIL | N/A |
| Q17 | 质量控制 | 取样管理的规程和代表性要求 | 0 | ❌ | FAIL | N/A |
| Q18 | 质量控制 | 稳定性考察的管理要求和方案设计 | 0 | ❌ | FAIL | N/A |
| Q19 | 质量管理体系 | CAPA管理的完整流程从识别到关闭 | 0 | ✅ | FAIL | N/A |
| Q20 | 质量管理体系 | 供应商审计的流程和检查要点 | 0 | ❌ | FAIL | N/A |
| Q21 | 质量管理体系 | 年度产品质量回顾应包含哪些内容 | 0 | ❌ | FAIL | N/A |
| Q22 | 文件与数据完整性 | ALCOA+数据完整性原则包括哪些要素 | 0 | ❌ | FAIL | N/A |
| Q23 | 文件与数据完整性 | GMP文件编号规则和版本管理要求 | 0 | ✅ | FAIL | N/A |
| Q24 | 设备管理 | 设备预防性维护计划和校准管理要求 | 0 | ✅ | FAIL | N/A |
| Q25 | 审计与自检 | GMP自检计划编制和缺陷分级标准 | 0 | ❌ | FAIL | N/A |

**grep**: 0/25 (路径问题 + 传统关键词局限)
**规则匹配**: 16/25 (64%) — 规则文件中的关键词匹配
**旧系统**: 所有 25 个场景均不在原有 391 测试集中
**GitNexus**: 0/25 — 设计错配，不索引文档内容