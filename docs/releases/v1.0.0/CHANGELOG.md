# Changelog

## [1.0.0] - 2026-05-21 (Draft)

### Added
- 项目脚手架：Cargo workspace + CLI 框架 (clap)
- Layer 1 数据底座：SQLRustGo 统一存储 (RDBMS + Vector + FTS + Graph)
- 数据导入：1696 gmp-md 文档 → gmp_documents + gmp_document_chunks
- 向量索引：bge-m3 1024d 嵌入 + HNSW 索引
- 图谱构建：agent-knowledge → 实体节点 + 关系边 (≥700 节点, ≥1500 边)
- RuleChannel：agent-knowledge YAML 规则匹配，weight=2.0
- VectorChannel：bge-m3 语义相似度检索 (recall@20 ≥ 70%)
- FtsChannel：BM25 + FMM GMP 词典分词 (recall@20 ≥ 50%)
- GraphChannel：Cypher + BFS 邻居扩展图谱召回
- RRF 融合引擎：四通道 Reciprocal Rank Fusion (k=60)
- Reranker：bge-reranker-v2-m3 Cross-Encoder 重排
- LLM 问答：qwen2.5:14b RAG 生成 + 意图分类器
- CLI 全命令：index / search / evaluate / shell
- 391 场景测试集评估
- 用户手册 + 部署指南