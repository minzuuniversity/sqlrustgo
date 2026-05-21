# v1.0.0 功能矩阵

> **版本**: 1.0.0
> **更新日期**: 2026-05-21
> **阶段状态**: Alpha

---

## Layer 1: 数据存储层

| 功能 | 状态 | 说明 |
|------|------|------|
| 文档主表 (gmp_documents) | ⬜ | 1696 文档元数据存储 |
| 内容表 (gmp_document_chunks) | ⬜ | ~8700 chunks, BM25 文档长度 |
| 向量嵌入表 (gmp_embeddings) | ⬜ | bge-m3 1024d, HNSW 索引 |
| 图节点存储 (DiskGraphStore) | ⬜ | ≥ 700 实体节点 |
| 图关系边 | ⬜ | ≥ 1500 语义关系 |
| 文档-图 Link (GraphLink) | ⬜ | 原子操作关联文档与图 |
| GMP 审计链 (内置) | ✅ | SQLRustGo gmp crate |
| WAL 持久化 | ✅ | SQLRustGo storage engine |

---

## Layer 2: 检索引擎层

| 功能 | 状态 | 指标 |
|------|------|------|
| RuleChannel | ⬜ | 命中率 ≥ 64% |
| VectorChannel | ⬜ | recall@20 ≥ 70% |
| FtsChannel (BM25) | ⬜ | recall@20 ≥ 50% |
| GraphChannel (Cypher) | ⬜ | ≥ 3 关联文档 |
| FMM GMP 分词器 | ⬜ | 194K 词词典 |
| RRF 融合 (k=60) | ⬜ | 规则 weight=2.0 |
| 通道加权配置 | ⬜ | YAML 可配置 |

---

## Layer 3: Reranker 重排层

| 功能 | 状态 | 说明 |
|------|------|------|
| bge-reranker-v2-m3 | ⬜ | Cross-Encoder |
| CPU 模式 | ⬜ | 无 GPU 需求 |
| Noop 回退 | ⬜ | CPU 不可用时跳过 |
| Top-K 截断 | ⬜ | 可配置 |

---

## Layer 4: LLM 问答层

| 功能 | 状态 | 说明 |
|------|------|------|
| qwen2.5:14b 集成 | ⬜ | Ollama API |
| 意图分类器 (7种) | ⬜ | 准确率 ≥ 80% |
| RAG Prompt 模板 | ⬜ | 带引用要求 |
| 引用溯源 | ⬜ | [文档编号] 标注 |
| 手动确认标注 | ⬜ | [需人工确认] 标注 |
| 重试机制 | ⬜ | 3 次 + 30s 超时 |

---

## CLI 接口

| 功能 | 状态 | 说明 |
|------|------|------|
| `index` 子命令 | ⬜ | 文档导入 + 向量化 + 图谱构建 |
| `search` 子命令 | ⬜ | 单通道/全通道检索 |
| `search --explain` | ⬜ | 调试模式查看各通道分数 |
| `search --rerank` | ⬜ | 启用 Reranker |
| `evaluate` 子命令 | ⬜ | 391 场景批量评估 |
| `shell` 子命令 | ⬜ | 交互式问答 |
| `bench` 子命令 | ⬜ | 性能压测 |
| `index --validate` | ⬜ | 数据完整性验证 |

---

## 文档

| 文档 | 状态 | 说明 |
|------|------|------|
| DEVELOPMENT_PLAN.md | ✅ | 开发计划 |
| TEST_PLAN.md | ✅ | 测试计划 |
| RELEASE_GATE_CHECKLIST.md | ✅ | 门禁检查清单 |
| RELEASE_NOTES.md | ✅ | 发布说明 |
| CHANGELOG.md | ✅ | 变更日志 |
| FEATURE_MATRIX.md | ✅ | 本文档 |
| INTEGRATION_STATUS.md | ✅ | 集成状态 |
| PERFORMANCE_TARGETS.md | ✅ | 性能目标 |
| ARCHITECTURE_DECISIONS.md | ✅ | ADR |
| USER_MANUAL.md | ✅ | 用户手册 |
| DEPLOYMENT_GUIDE.md | ✅ | 部署指南 |

---

> 图例: ✅ 完成 | ⬜ 待开发 | ❌ 未支持