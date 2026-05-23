# v1.1.0 功能矩阵

> **版本**: 1.1.0
> **更新日期**: 2026-05-22
> **阶段状态**: GA

---

## Layer 0: 离线知识抽取管线 (新增)

| 功能 | 状态 | 说明 |
|------|------|------|
| GMP 术语库 (gmp_terms) | ✅ | 508+ 术语, 4040 行 JSON |
| NER 实体识别 (doc_entities) | ✅ | 规则分类器 |
| 关系抽取 (doc_relations) | ✅ | 10+ 关系模板 |
| 多维分类 (doc_taxonomy) | ✅ | 6 维度 |
| 相关性评分 (doc_relevance) | ✅ | 5 维度加权 |

---

## Layer 1: 数据存储层

| 功能 | 状态 | 说明 |
|------|------|------|
| 文档主表 (gmp_documents) | ✅ | 1696 文档元数据存储 |
| 内容表 (gmp_document_chunks) | ✅ | chunks 存储 |
| 向量嵌入表 (gmp_embeddings) | ✅ | bge-m3 1024d, HNSW 索引 |
| 图节点存储 (DiskGraphStore) | ✅ | WAL 持久化 |
| 图关系边 | ✅ | SqlrustgoAdapter |
| HNSW 索引 | ✅ | M=16, ef=200 |
| GMP 审计链 (内置) | ✅ | SQLRustGo gmp crate |
| WAL 持久化 | ✅ | SqlrustgoAdapter |

---

## Layer 2: 检索引擎层

| 功能 | 状态 | 指标 |
|------|------|------|
| RuleChannel | ✅ | 命中率 >= 64% |
| VectorChannel (HNSW) | ✅ | recall@20 >= 88% |
| FtsChannel (BM25) | ✅ | recall@20 >= 50% |
| GraphChannel (Cypher) | ✅ | >= 3 关联文档 |
| FMM GMP 分词器 | ✅ | 194K 词词典 |
| RRF 融合 (k=60) | ✅ | 规则 weight=2.0 |
| 通道加权配置 | ✅ | YAML 可配置 |

---

## Layer 3: Reranker 重排层

| 功能 | 状态 | 说明 |
|------|------|------|
| bge-reranker-v2-m3 | ✅ | Cross-Encoder |
| CPU 模式 | ✅ | 无 GPU 需求 |
| Noop 回退 | ✅ | CPU 不可用时跳过 |
| Top-K 截断 | ✅ | 可配置 |

---

## Layer 4: LLM 问答层

| 功能 | 状态 | 说明 |
|------|------|------|
| qwen2.5:14b 集成 | ✅ | Ollama API |
| 意图分类器 (7种) | ✅ | 准确率 >= 80% |
| RAG Prompt 模板 | ✅ | 带引用要求 |
| 引用溯源 | ✅ | [文档编号] 标注 |
| 重试机制 | ✅ | 3 次 + 30s 超时 |

---

## Layer 5: LLM 推理 (新增)

| 功能 | 状态 | 说明 |
|------|------|------|
| 多跳推理 | ✅ | Ollama qwen2.5:14b |
| 答案生成 | ✅ | 带引用溯源 |
| 可解释引用 | ✅ | [需人工确认] 标注 |

---

## CLI 接口

| 功能 | 状态 | 说明 |
|------|------|------|
| index 子命令 | ✅ | 文档导入 + 向量化 + 图谱 + Layer 0 |
| search 子命令 | ✅ | 单通道/全通道检索 |
| search --explain | ✅ | 调试模式查看各通道分数 |
| search --rerank | ✅ | 启用 Reranker |
| evaluate 子命令 | ✅ | 408 场景批量评估 |
| shell 子命令 | ✅ | 交互式问答 |
| bench 子命令 | ✅ | 性能压测 |
| index --validate | ✅ | 数据完整性验证 |

---

## 文档

| 文档 | 状态 | 说明 |
|------|------|------|
| DEVELOPMENT_PLAN.md | ✅ | 开发计划 |
| FIVE-LAYER-ANALYSIS.md | ✅ | 五层架构分析 |
| gmp_terms.json | ✅ | 4040 行术语库 |
| gmp_terms.sql | ✅ | 796 行 Schema |
| SPEC.md | ✅ | 规格说明书 |
| TEST_PLAN.md | ✅ | 测试计划 |
| 00-release-summary.md | ✅ | 发布摘要 |
| ARCHITECTURE_DECISIONS.md | ✅ | ADR |
| CHANGELOG.md | ✅ | 变更日志 |
| COMPREHENSIVE_STATUS_REPORT.md | ✅ | 综合性状态报告 |
| DEPLOYMENT_GUIDE.md | ✅ | 部署指南 |
| EVALUATION_REPORT.md | ✅ | 评估报告 |
| FEATURE_MATRIX.md | ✅ | 本文档 |
| INCREMENTAL_UPDATE_GUIDE.md | ✅ | 增量更新指南 |
| INTEGRATION_STATUS.md | ✅ | 集成状态 |
| PERFORMANCE_REPORT.md | ✅ | 性能报告 |
| RELEASE_GATE_CHECKLIST.md | ✅ | 门禁检查清单 |
| RELEASE_NOTES.md | ✅ | 发布说明 |
| USER_MANUAL.md | ✅ | 用户手册 |
| COVERAGE_REPORT.md | ✅ | 覆盖率报告 |
| ARCHITECTURE_DEVIATION_REPORT.md | ✅ | 架构偏离报告 |

---

> 图例: ✅ 完成 | ⬜ 待开发 | ❌ 未支持
