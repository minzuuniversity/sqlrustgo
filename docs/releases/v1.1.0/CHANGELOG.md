# Changelog

## [1.1.0] - 2026-05-22

### Added
- **Layer 0 离线知识抽取管线**: GMP 术语库 + NER + 关系抽取 + 多维分类 + 相关性评分
- **gmp_terms 表**: 508+ 术语, 4040 行 JSON 同义词层级
- **HNSW 向量索引**: 通过 sqlrustgo_vector 实现，O(1) 内存检索
- **SqlrustgoAdapter 持久化**: 图索引 + HNSW 索引跨会话持久化
- **五层架构**: Layer 0 (知识抽取) → Layer 1 (存储) → Layer 2 (图谱) → Layer 3 (检索) → Layer 4 (LLM)
- **gmp-classifier**: 规则分类器，6 维度 (topic/doc_type/risk_level/department/target_object/requisite_action)
- **gmp-re**: 关系抽取器，10+ 关系模板
- **gmp-scorer**: 相关性评分器，5 维度加权 (术语密度/法规引用度/流程完整性/实体密度/部门覆盖度)
- **gmp-graph**: 图写入器，Layer 0 → Layer 2 知识图谱更新
- **gmp-kg**: KgStorage，五张 Layer 0 表的 CRUD 操作
- **向量+FTS 混合检索**: gmp-eval 向量 fallback
- **408 场景测试集**: 相比 v1.0.0 增加 17 个场景

### Changed
- **QPS 优化**: 从 8.3 提升至 154.0 (18.5x)
- **P95 延迟优化**: 从 1366ms 降至 6.5ms (210x)
- **场景通过率**: 从 96.1% 提升至 99.8% (407/408)
- **Recall@K**: 从 0.848 提升至 0.88+
- **MRR**: 从 0.497 提升至 0.55+

### Fixed
- **图索引无持久化**: 通过 SqlrustgoAdapter 解决 (Issue D-001)
- **向量 fallback 缺失**: gmp-eval 现在支持向量搜索 fallback (Issue D-003)
- **16 场景失败**: 通过 Layer 0 术语标准化解决 (Issue D-002)

### Performance
- 向量搜索: 全量遍历 → HNSW 索引，检索速度提升 100x+
- 整体 QPS: 8.3 → 154.0
- P95 延迟: 1366ms → 6.5ms

---

## [1.0.0] - 2026-05-21

### Added
- 项目脚手架：Cargo workspace + CLI 框架 (clap)
- Layer 1 数据底座：SQLRustGo 统一存储 (RDBMS + Vector + FTS + Graph)
- 数据导入：1696 gmp-md 文档 → gmp_documents + gmp_document_chunks
- 向量索引：bge-m3 1024d 嵌入 + HNSW 索引
- 图谱构建：agent-knowledge → 实体节点 + 关系边 (≥700 节点, ≥1500 边)
- RuleChannel：agent-knowledge YAML 规则匹配，weight=2.0
- VectorChannel：bge-m3 语义相似度检索 (recall@20 ≥ 70%)
- FtsChannel：BM25 + FMM GMP 分词 (recall@20 ≥ 50%)
- GraphChannel：Cypher + BFS 邻居扩展图谱召回
- RRF 融合引擎：四通道 Reciprocal Rank Fusion (k=60)
- Reranker：bge-reranker-v2-m3 Cross-Encoder 重排
- LLM 问答：qwen2.5:14b RAG 生成 + 意图分类器
- CLI 全命令：index / search / evaluate / shell
- 391 场景测试集评估
- 用户手册 + 部署指南
