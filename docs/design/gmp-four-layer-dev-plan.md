# GMP 合规内审检索系统 — 四层架构开发计划

> 版本: v1.0 | 日期: 2026-05-20 | 状态: 待启动

---

## 1. 概述

### 1.1 目标

实现 GMP 合规内审检索系统的四层融合架构，完成 391 个场景的全量测试，目标通过率 > 80%（当前最优 44.5%）。

### 1.2 架构回顾

```
Layer 4: LLM 推理与问答 ← 新增: 意图分类 + 多跳推理 + 答案生成
Layer 3: 知识图谱实体推理 ← 新增: 实体节点 + 语义关系边 + 路径推理
Layer 2: 语义检索与重排   ← 改进: 向量检索 + FTS5全文 + 规则匹配 + Reranker
Layer 1: 数据整合         ← 已完成: gmp-md (1696文档) + agent-knowledge (55+规则)
```

### 1.3 数据资产

| 资产 | 路径 | 规模 | 状态 |
|------|------|------|------|
| GMP 原始文档 | gmp-md/ | 1696 份 MD | ✅ 已完成 |
| 脱敏文档 | gmp-md-masking/ | 1696 份 MD | ✅ 已完成 |
| 结构化规则 | agent-knowledge/ | 55+ YAML | ✅ 已完成 |
| 391 测试场景 | gmp_kg_build/test_results_final/ | 391 条 JSON | ✅ 已提取 |
| 现有测试基线 | gmp_kg_build/ | v22: 44.5% | ✅ 已记录 |

---

## 2. 开发阶段

### Phase A: Layer 2 — 语义检索与重排引擎 (Week 1-2)

**目标**: 替换现有 Python v22 引擎，提升基础检索通过率至 > 65%

#### A1: 向量化引擎 (Day 1-3)

```yaml
方案: Rust + bge-m3 (1024d) 本地推理
依赖: 
  - fastembed-rs (Rust crate, 支持 bge-m3)
  - SQLite 向量扩展 (vec0 或自定义 BLOB)
数据: gmp-md/ 全部 1696 文档
实现:
  - chunk: 512 tokens, overlap 50
  - embed: bge-m3 1024d, GPU可选
  - store: SQLite BLOB + cosine 相似度
  - index: IVF 近似索引 (可选)
```

**任务清单**:
- [ ] 实现 `VectorEngine` trait + `SqliteVectorStore`
- [ ] 实现 bge-m3 嵌入批次处理 (batch=32)
- [ ] 向量化全部 1696 文档 (预估: 512 chunks × 1696 ≈ 870K chunks)
- [ ] 搭建评估流水线: 输入 query → 返回 top-k → 计算 recall@20

#### A2: FTS5 全文检索增强 (Day 4-5)

```yaml
方案: SQLite FTS5 + FMM 分词器 (Rust 实现)
依赖: rusqlite, FTS5 tokenizer API
增强:
  - GMP 词典: 194K 词条 (继承 gmp_kg_build)
  - FMM 分词: 前向最大匹配, 2-8 字窗口
  - 同义词扩展: 79组领域锚定词
存储:
  - CREATE VIRTUAL TABLE docs_fts USING fts5(title, content, tokenize='gmp_fmm')
```

**任务清单**:
- [ ] 实现 `FmmTokenizer` (Rust, 复用 Python 词典)
- [ ] 注册为 SQLite FTS5 自定义分词器
- [ ] 批量入库 1696 文档到 FTS5 索引
- [ ] 评估: 关键词检索 recall@20

#### A3: 规则结构化匹配 (Day 6-7)

```yaml
方案: YAML 解析 + 规则键值三元组索引
数据: agent-knowledge/L0-trust-zone/rules/*.yaml
实现:
  - 解析 YAML 规则文件
  - 提取: (category, key, value) 三元组
  - 索引: HashMap<String, Vec<(key, value)>>
  - 匹配: query 关键词 → category → 返回规则
```

**任务清单**:
- [ ] 实现 `RuleMatcher`: YAML解析 + 三元组索引
- [ ] 覆盖全部 55+ 规则文件
- [ ] 评估: 25 场景规则命中率 (当前基线 64%)

#### A4: 多路融合 + Reranker (Day 8-10)

```yaml
方案: RRF (k=60) 融合 + bge-reranker-v2-m3 重排
通道权重:
  - 向量语义: 权重 0.4
  - FTS5 全文: 权重 0.35
  - 规则匹配: 权重 0.25
RRF参数: k=60
Reranker: bge-reranker-v2-m3 (GPU加速, CPU fallback)
```

**任务清单**:
- [ ] 实现 `ReciprocalRankFusion` with 可配置权重
- [ ] 集成 bge-reranker-v2-m3
- [ ] 端到端评估: 391 场景
- [ ] 目标: Baseline 通过率 > 65%

#### A5: Python v2.0 回归对比 (Day 11-12)

```yaml
方案: 用 Python 实现同架构对比版本
目的: 快速验证架构级策略选择, 对比 Rust vs Python 性能
实现:
  - bge-m3 via sentence-transformers
  - rank_bm25 替代 FTS5
  - RRF 同参数
验证: 391 场景
```

**任务清单**:
- [ ] Python 版 Layer 2 实现
- [ ] 391 场景评估
- [ ] 与 Rust 版性能对比
- [ ] 确定最终 Layer 2 接口

**Phase A 里程碑**:
- [ ] Rust/Python 双版本可运行
- [ ] 391 场景基线 > 65%
- [ ] 生成 Layer 2 评估报告

---

### Phase B: Layer 3 — 知识图谱实体推理 (Week 2-4)

**目标**: 构建 GMP 领域实体知识图谱，支持 2-hop 路径推理

#### B1: 实体模型设计 (Day 1-2)

```yaml
实体节点类型:
  - SOP:        标准操作规程 (521个)
  - Device:     设备 (混合机/压片机/灭菌柜/...)
  - Material:   物料 (原料药/辅料/包装材料)
  - Deviation:  偏差类型 (重大/一般/微小)
  - Validation: 验证类型 (DQ/IQ/OQ/PQ/清洁/工艺)
  - CAPA:       纠正预防 (CAPA记录)
  - Regulation: 法规条款 (中国GMP 2010 版 313条)
  - Department: 部门 (生产部/质量部/设备部/...)
  - Chapter:    章节 (CH01-CH09)
  - AuditFinding: 审计发现

关系边类型:
  - TRIGGERS:      偏差 → CAPA
  - FOLLOWS_SOP:   流程 → SOP
  - REQUIRES:      设备 → 验证
  - MANDATES:      法规 → SOP
  - BELONGS_TO:    SOP → 章节
  - MANAGED_BY:    SOP → 部门
  - STORED_AT:     物料 → 库房
  - GENERATES:     审计发现 → CAPA
  - VERIFIED_BY:   CAPA → 验证
  - PRECEDES:      文档 → 文档 (流程依赖)
```

**任务清单**:
- [ ] 定义实体 Schema (Rust struct)
- [ ] 定义关系 Schema
- [ ] 实现图存储 (petgraph / SQLite 邻接表)
- [ ] 定义 Cypher-like 查询 DSL

#### B2: 实体提取与关系构建 (Day 3-5)

```yaml
自动提取 (基于 agent-knowledge YAML):
  - SOP → 章节/部门: 从 YAML 分类树提取
  - SOP → SOP: 从 workflows/ 流程定义提取
  - SOP → 法规: 从 rules/ 法规引用提取
  - 设备 → SOP: 从 rules/ 关键词匹配提取
  - 物料 → SOP: 同上

手动补充 (需领域专家):
  - 偏差 → CAPA 触发规则 (10条)
  - CAPA → 验证 闭环规则 (5条)
  - 审计发现 → CAPA (3条)
  - 设备 → 验证方案 映射 (20个设备)
  - 法规 → SOP 映射 (50条核心条款)
```

**任务清单**:
- [ ] 实现自动提取管线
- [ ] 从 agent-knowledge 提取 500+ 节点, 1500+ 边
- [ ] 手动标注 50 条语义关系
- [ ] 实体数量目标: > 700 节点, > 3000 边

#### B3: 图推理引擎 (Day 6-8)

```yaml
图数据库: SQLite (邻接表) + petgraph (内存图)
查询类型:
  1. 邻居查询: "SOP-A 的前置文档有哪些?" → 1-hop neighbors
  2. 路径查询: "偏差如何触发CAPA并验证关闭?" → shortest path
  3. 子图扩展: "设备A相关的所有SOP和验证" → 2-hop subgraph
  4. 法规检索: "GMP第84条的关联SOP" → 法规→SOP

混合检索增强:
  - Layer 2 Top-k 结果 → 图扩展 (获取关联文档)
  - 图邻居 boost: 关联文档额外加权 0.15
```

**任务清单**:
- [ ] 实现 `GraphEngine` 查询接口
- [ ] 实现邻居 + 路径 + 子图查询
- [ ] 集成到 RRF 融合管线
- [ ] 评估: 图增强对 391 场景的提升 (目标 +8%)
- [ ] 目标: Layer 2+3 融合 > 73%

#### B4: 中国 GMP 条文索引 (Day 9-10)

```yaml
数据: APPX-A-GMP法规条款与解读 (53份)
任务:
  - 提取 313 条 GMP 2010 条款号 → 条文映射
  - 建立 "条款 → 相关 SOP" 连接
  - 索引到 Layer 2 检索通道
```

**任务清单**:
- [ ] 解析条款号 (如 "GMP 第84条")
- [ ] 提取全文条款
- [ ] 建立条款-SOP 映射
- [ ] 评估: 条款查询场景

**Phase B 里程碑**:
- [ ] 图谱 > 700 节点, > 3000 边
- [ ] 图增强融合 Layer 2+3 通过率 > 73%
- [ ] 条款查询场景全部可应答

---

### Phase C: Layer 4 — LLM 推理与问答 (Week 4-5)

**目标**: 集成本地 LLM，实现意图分类、多跳推理、答案生成

#### C1: LLM 基础设施 (Day 1-2)

```yaml
LLM选择:
  主推理: Qwen3-14B-Q4 (Ollama, CPU 16核 可运行)
  轻量分类: Qwen3-1.7B (Ollama, 快速意图分类)
  Embedding: bge-m3 (已有)
  Reranker: bge-reranker-v2-m3 (已有)

部署: Ollama REST API (http://localhost:11434)
```

**任务清单**:
- [ ] 确认 Ollama 可用性
- [ ] 部署 Qwen3-14B-Q4 + Qwen3-1.7B
- [ ] 测试推理延迟 (目标: < 5s 生成)

#### C2: 意图分类器 (Day 3-4)

```yaml
意图类型:
  - retrieve_sop:        "获取 XX SOP 原文"
  - check_compliance:    "XX 是否符合 GMP 要求?"
  - compare_regulation:  "中国GMP和EU GMP在XX上的差异"
  - trace_process:       "XX 流程的完整步骤"
  - classify_incident:   "这种情况属于偏差还是变更?"
  - generate_checklist:  "生成 XX 检查清单"
  - analyze_root_cause:  "XX 问题的可能根因"

实现: Qwen3-1.7B + few-shot prompt
```

**任务清单**:
- [ ] 构造 100 条标注的 Intent 训练样本
- [ ] 实现 IntentClassifier
- [ ] 准确率目标: > 90%
- [ ] 集成到检索管线开头

#### C3: 多跳推理链 (Day 5-7)

```yaml
推理流程:
  1. Intent → 确定查询类型
  2. Layer 2 → 检索候选文档 Top-20
  3. Layer 3 → 图扩展关联文档对比条款
  4. Layer 4 → LLM 生成推理链 + 引用来源

Prompt 模板:
"""
你是GMP合规审核AI助手。请基于以下材料回答:
## 相关文档
{retrieved_docs}
## 相关规则  
{rules_and_graph}
## 用户问题
{query}
## 回答要求
1. 直接回答
2. 引用来源 [文档编号]
3. 不确定处标注 [需人工确认]
"""
```

**任务清单**:
- [ ] 实现 `ReasoningChain`
- [ ] 实现引用溯源 (citation linking)
- [ ] 评估: 50 条复杂多跳查询
- [ ] 准确率目标: > 70%

#### C4: 答案生成与质量评估 (Day 8-10)

```yaml
评估方法: 自动 + 人工双重评估
自动指标:
  - 精确匹配率 (retrieval-based)
  - 答案含源率 (citation coverage)
  - 幻觉检测 (factual consistency)
人工评估:
  - 30 条验证集
  - 正确性/完整性/有用性 1-5 评分
```

**任务清单**:
- [ ] 实现自动评估管线
- [ ] 30 条人工评估
- [ ] 生成评估报告
- [ ] 目标: 自动指标 > 80%, 人工评分 > 4.0

**Phase C 里程碑**:
- [ ] Layer 2+3+4 全链路可用
- [ ] 意图分类准确率 > 90%
- [ ] 多跳推理准确率 > 70%
- [ ] 全链路通过率目标: > 80%

---

### Phase D: Rust 核心重写 (Week 5-7)

**目标**: 将 Python Phase C 验证的架构用 Rust 重新实现，提升性能至 < 1s/query

#### D1: CLI 重构 (Day 1-3)

```rust
// 统一 CLI
gmp-retrieve search "偏差分级标准" --output json
gmp-retrieve search "偏差分级标准" --mode explain  # 含推理链
gmp-retrieve index --source gmp-md/
gmp-retrieve evaluate --test-set 391_scenarios.json
```

**任务清单**:
- [ ] 定义 clap CLI 接口
- [ ] 实现 index/search/evaluate 子命令
- [ ] JSON/Markdown 双输出格式

#### D2: 性能优化 (Day 4-7)

```yaml
优化目标:
  - 冷启动: < 30s (加载模型+索引)
  - 单次查询: < 1s (不含LLM推理)
  - 含LLM推理: < 5s
  - 内存: < 4GB

优化策略:
  - mmap 索引文件 (零拷贝)
  - 向量量化 (INT8 可选)
  - 缓存常用查询结果
  - Rayon 并行化多通道检索
```

**任务清单**:
- [ ] mmap 索引文件
- [ ] 并行化多路检索
- [ ] 压测: 10并发 × 100轮
- [ ] profiling + 热点优化

#### D3: 稳定性测试 (Day 8-10)

```yaml
测试矩阵:
  - 391 场景 × 5 次重复
  - 10 并发 × 100 轮
  - 100 小时连续运行 (无内存泄漏)
```

**任务清单**:
- [ ] 稳定性测试
- [ ] 内存泄漏检测 (valgrind)
- [ ] 错误处理完整性

**Phase D 里程碑**:
- [ ] Rust 单二进制, < 50MB
- [ ] 391 场景通过率 > 80%
- [ ] 单查询 < 1s, 含LLM < 5s
- [ ] 稳定性验证通过

---

### Phase E: 391 场景验证与报告 (Week 7)

**目标**: 完整 391 场景回归测试，输出详细对比报告

#### E1: 测试集准备 (Day 1-2)

```yaml
测试源:
  - gmp_kg_build/test_results_final/391_test_simple_results.json
  - gmp_kg_build/test_results_final/164_test_expert.json (v3)

场景分类:
  - 图谱查询: 15
  - 验证与确认: 15
  - 自检与审计: 15
  - 风险管理: 15
  - 物料管理: 15
  - 设备管理: 15
  - 偏差管理: 15
  - 变更控制: 15
  - 文件与数据完整性: 15
  - 生产管理: 15
  - 质量控制: 15
  - 质量管理体系: 15
  - 法规对比: 15
  - 多跳推理: 15
  - 复杂场景: 211
```

**任务清单**:
- [ ] 解析 391 场景 JSON
- [ ] 按分类归类
- [ ] 标记难度 (easy/medium/hard)
- [ ] 建立 ground truth 参考答案

#### E2: 自动评估管线 (Day 3-4)

```yaml
评估指标:
  PASS/FAIL: 检索结果是否包含正确答案
  Recall@5: Top-5 是否命中
  MRR: Mean Reciprocal Rank
  NDCG@10: Normalized Discounted Cumulative Gain
  Latency: P50/P95/P99

对比维度:
  1. 本系统 (Layer 2+3+4)
  2. gmp_kg_build v22 基线 (44.5%)
  3. gmp_kg_build v3 峰值 (92.7% on 164)
  4. 纯向量检索
  5. 纯关键词检索
```

**任务清单**:
- [ ] 实现自动化评估脚本
- [ ] 跑全部 391 场景 × 5 次
- [ ] 生成每个场景的 PASS/FAIL + 命中结果

#### E3: 失败分析 + 修复迭代 (Day 5-6)

**任务清单**:
- [ ] 统计各分类通过率
- [ ] 识别 Top-5 失败模式
- [ ] 诊断根因 (知识缺口 / 语义偏差 / 图谱不足 / LLM幻觉)
- [ ] 快速修复 (Day 5-6)
- [ ] 回归测试

#### E4: 最终报告 (Day 7)

**输出**:
1. 391 场景详细结果表
2. 跨系统对比雷达图
3. 改进建议和下一步方向
4. 代码仓库 + 文档

**Phase E 里程碑**:
- [ ] 391 场景完整结果
- [ ] 通过率 > 80% (确认/修正预估)
- [ ] 输出最终报告

---

## 3. 时间线与资源

| 阶段 | 名称 | 周期 | 并行度 | 交付物 |
|------|------|------|--------|--------|
| Phase A | Layer 2 语义检索 | Week 1-2 | 1人 | Rust/Python 双版本 + 评估报告 |
| Phase B | Layer 3 知识图谱 | Week 2-4 | 1人 | 实体图谱 + 图推理引擎 |
| Phase C | Layer 4 LLM推理 | Week 4-5 | 1人 | 全链路可用原型 |
| Phase D | Rust 核心重写 | Week 5-7 | 1人 | 单二进制 CLI (< 50MB) |
| Phase E | 391 场景验证 | Week 7 | 1人 | 完整测试报告 |

**总周期**: 7 周 (1 人) 或 4 周 (2 人并行 Phase A+B)

**硬件要求**:
- CPU: 16 核以上
- RAM: 32GB+
- GPU: 可选 (加速 bge-reranker)
- 磁盘: 10GB+ (模型 + 索引)

---

## 4. 风险评估与缓解

| 风险 | 概率 | 影响 | 缓解策略 |
|------|------|------|----------|
| GPU 不可用 (RTX 5080 兼容) | 高 | 中 | bge-m3 CPU 推理可接受 (~2s) |
| 向量化 1696 文档时间过长 | 中 | 低 | 批次 + 并行, 一次性处理 |
| LLM 幻觉导致错误答案 | 中 | 高 | 引用溯源 + 人工抽查 + 不确定性标注 |
| 知识缺口无法弥补 | 低 | 高 | Layer 3 手动标注 + 中国GMP条文提取 |
| 391 场景 < 80% | 中 | 中 | 分层分析, 识别可修复项, 如未达预期则标注开放项 |

---

## 5. 391 场景测试基准

### 5.1 现有基线

| 系统 | 版本 | 场景数 | 通过率 | 时间 |
|------|------|--------|--------|------|
| gmp_kg_build | v2 | 164 | **92.7%** | 2026-04-11 |
| gmp_kg_build | v11 | 219 | **62.6%** | 2026-04-12 |
| gmp_kg_build | v22 | 391 | **44.5%** | 2026-04-16 |
| gmp-qa-package | v1.0 | 20 | **100%** | 2026-04 |
| 本系统 (预估) | v1.0 | 391 | **> 80%** | 2026-07 |

### 5.2 通过率预估分解

| 通道 | 单独通过率 | 累积通过率 |
|------|-----------|-----------|
| 纯 FTS5 BM25 (v22 基线) | ~45% | 45% |
| + bge-m3 向量语义 (Phase A) | +15% | ~60% |
| + 规则结构化匹配 | +5% | ~65% |
| + RRF 融合 + Reranker | +5% | ~70% |
| + 知识图谱实体扩展 (Phase B) | +8% | ~78% |
| + LLM 推理与多跳 (Phase C) | +5% | **~83%** |

### 5.3 无法覆盖的场景预估 (~17% = 66/391)

| 类型 | 数量 | 原因 | 缓解 |
|------|------|------|------|
| 跨法规对比 (中美欧日) | 20 | 缺少 EU/FDA/PMDA 条文 | 标注为后续补充 |
| 极冷门 SOP | 20 | 语料库未覆盖 | 标注为知识缺口 |
| 高度情境依赖 | 15 | 需要操作工现场信息 | 标注需人工辅助 |
| LLM 幻觉/错误 | 11 | 模型能力边界 | 不确定性标注 |

---

## 6. 附录

### A. 关键依赖清单

```toml
# Rust CLI 依赖 (Cargo.toml)
[dependencies]
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rusqlite = { version = "0.31", features = ["bundled", "vtab"] }
petgraph = "0.6"
fastembed = "3"          # bge-m3 embedding
rayon = "1.10"           # 并行计算
reqwest = { version = "0.12", features = ["json", "blocking"] }  # Ollama API
```

### B. 测试场景示例

```json
{
  "id": "DEV-001",
  "domain": "偏差管理",
  "query": "生产过程中发现中间产品含量不合格，应该如何处理？",
  "difficulty": "medium",
  "expected_sop": "GMP-02-DEV-001",
  "expected_key_points": ["偏差登记", "立即上报", "隔离产品", "调查启动"],
  "layer2_expected_hit": true,
  "layer3_expected_path": "Deviation → SOP-DEV-001 → CAPA",
  "layer4_expected_answer": "应按照偏差处理规程..."
}
```

### C. 脱敏数据集说明

gmp-md-masking/ 目录包含 1696 份 GMP 合规文档，已通过脚本 `scripts/mask_gmp_md.py` 完成 7 类脱敏。可安全提交到公有仓库或云端。MASKING-MAP.json 仅本地保存，已加入 .gitignore。