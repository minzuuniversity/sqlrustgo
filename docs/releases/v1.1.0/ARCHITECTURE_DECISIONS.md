# v1.1.0 架构决策记录 (ADR)

> **版本**: 1.1.0
> **更新日期**: 2026-05-22
> **维护人**: Hermes Agent + AI Pipeline

---

## ADR-001: 选择 SQLRustGo 作为统一数据底座

**状态**: 已接受

**背景**: GMP 内审检索系统需要同时支持关系型查询、向量检索、全文检索和图查询。传统方案需要组合多个独立数据库（SQLite/KuzuDB/向量扩展）。

**决策**: 使用 SQLRustGo 作为统一数据底座，集成：
- `storage` crate → RDBMS + FTS
- `vector` crate → HNSW/IVF 向量索引
- `graph` crate → DiskGraphStore + Cypher
- `qmd-bridge` crate → 混合检索
- `gmp` crate → GMP 合规特性

**后果**:
- 优势: 单二进制, 零外部数据库, 原子事务跨 RDBMS/向量/图
- 劣势: 依赖 SQLRustGo 版本更新
- 风险: SQLRustGo 增量补全 PR 的稳定性

---

## ADR-002: 选择 Ollama 统一 LLM 推理

**状态**: 已接受

**背景**: 系统需要 Embedding 模型 (bge-m3)、Reranker 模型 (bge-reranker-v2-m3)、LLM 模型 (qwen2.5:14b)。

**决策**: 统一使用 Ollama 本地部署所有模型，避免多推理框架。

**替代方案**:
- llama.cpp: 需分别管理多模型
- 云端 API: 数据隐私风险
- Transformers: 部署复杂性高

**后果**:
- 优势: 统一接口, 本地隐私保护, 易部署
- 劣势: 依赖 Ollama 服务可用性
- 缓解: 3 次重试 + 30s 超时

---

## ADR-003: RRF 融合代替加权求和

**状态**: 已接受

**背景**: SQLRustGo 现有 `qmd-bridge` 使用 `vector*0.4 + graph*0.3 + text*0.3` 线性加权融合，对不同量纲的分数不鲁棒。

**决策**: 采用 Reciprocal Rank Fusion (RRF), k=60, 规则通道 weight=2.0。

公式: `score(d) = Σ w_c / (k + rank(d, c))`

**后果**:
- 优势: 分数量纲无关, 无超参调优, 效果稳定
- 劣势: 丢失原始分数信息
- 缓解: Reranker 后续重排弥补精度损失

---

## ADR-004: CPU 优先，GPU 可选

**状态**: 已接受

**背景**: 目标硬件为 Z440 工作站 + RTX 5080，但新驱动可能存在兼容问题。

**决策**:
- 所有模型 (bge-m3, reranker, qwen2.5) 默认 CPU 模式运行
- Reranker 提供 GPU 加速选项（通过 Ollama 配置）
- 无法使用 GPU 时自动回退 CPU

**后果**:
- 优势: 兼容性最大化
- 劣势: Reranker 在纯 CPU 下 2-5s/chunk
- 缓解: Reranker 先 run top-5，不够扩大

---

## ADR-005: CLI first, Web API later

**状态**: 已接受

**背景**: v1.0.0 需要快速交付 391 场景验证。

**决策**: 优先实现 CLI 接口（clap），v1.1.0 规划 Web API。

**后果**:
- 优势: 开发速度快, 测试友好
- 劣势: 不便于远程访问
- 缓解: v1.1.0 已实现五层架构，Web API 列为 v1.2.0 规划

---

## ADR-006: 规则通道高权重 (2.0)

**状态**: 已接受

**背景**: agent-knowledge 结构化规则库是高精度知识来源，应在检索结果中优先呈现。

**决策**: RRF 融合中规则通道 weight=2.0 (其他通道=1.0)。

**后果**:
- 优势: 规则命中结果优先展示，提升 precision
- 劣势: 可能覆盖语义相关但非直接规则匹配的内容
- 缓解: 通过 Reranker 在规则命中和语义结果间再平衡

---

## ADR-007: HNSW 向量索引集成 (v1.1.0 新增)

**状态**: 已接受

**背景**: v1.0.0 向量搜索无索引，全量遍历计算 cosine similarity，在大规模向量集上性能不足。

**决策**: 引入 HNSW 索引，通过 SqlrustgoAdapter 持久化，支持增量构建。

**技术方案**:
- 使用 sqlrustgo_vector crate 的 HNSW 实现
- 通过 SqlrustgoAdapter 将 HNSW 索引持久化到磁盘
- 向量维度: bge-m3 1024d
- M=16, ef_construction=200

**后果**:
- 优势: 检索性能提升 100x+，O(1) 内存占用可控
- 劣势: 构建索引耗时较长（约 10-20 分钟）
- 缓解: 增量索引策略，仅对新文档构建

---

## ADR-008: Layer 0 离线知识抽取管线 (v1.1.0 新增)

**状态**: 已接受

**背景**: v1.1.0 前系统缺乏术语标准化和语义理解能力，导致同义词检索失败。

**决策**: 新增五层架构 Layer 0（离线预计算），包含：
- GMP 术语库 (gmp_terms): 508+ 术语, 4040 行 JSON
- NER 实体识别 (doc_entities)
- 关系抽取 (doc_relations)
- 多维分类 (doc_taxonomy)
- 相关性评分 (doc_relevance)

**后果**:
- 优势: 术语标准化，同义词自动映射，检索召回率显著提升
- 劣势: 离线管线增加索引构建时间
- 缓解: 增量管线，仅处理变更文档

---

## ADR-009: SqlrustgoAdapter 持久化方案 (v1.1.0 新增)

**状态**: 已接受

**背景**: 图索引和 HNSW 索引需要跨会话持久化，避免每次重启重新构建。

**决策**: 实现 SqlrustgoAdapter，将 HNSW 索引和图数据通过 SQLRustGo 统一存储层持久化。

**技术细节**:
- 图节点/边存储于 `graph_nodes` / `graph_edges` 表
- HNSW 索引参数存储于 `vector_index_meta` 表
- WAL 模式保证崩溃一致性

**后果**:
- 优势: 索引持久化，重启无需重建
- 劣势: 持久化开销略高
- 缓解: 后台异步写入
