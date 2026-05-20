# v1.0.0 架构决策记录 (ADR)

> **版本**: 1.0.0
> **更新日期**: 2026-05-21
> **维护人**: TRAE + DeepSeek-V4-Pro

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
- 缓解: v1.1.0 plan Web API (ISSUE #12)

---

## ADR-006: 规则通道高权重 (2.0)

**状态**: 已接受

**背景**: agent-knowledge 结构化规则库是高精度知识来源，应在检索结果中优先呈现。

**决策**: RRF 融合中规则通道 weight=2.0 (其他通道=1.0)。

**后果**:
- 优势: 规则命中结果优先展示，提升 precision
- 劣势: 可能覆盖语义相关但非直接规则匹配的内容
- 缓解: 通过 Reranker 在规则命中和语义结果间再平衡