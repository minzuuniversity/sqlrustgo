# Changelog

## [1.2.0] - 2026-05-23 (开发中)

### Added
- Agent 编排层：DSL 生成 + 防火墙规则 + 流程编排 (commit bf736a6)
- E2E 测试套件：TS-WEI-001~010 端到端测试 + 防火墙中文注入规则 (commit e7cb8fd)
- JWT 认证：电子签名 + 身份验证 (commit e1f7277)
- 合规审计报告：OpenAPI 规范 + 法规合规验证 (commit 49f7394)
- 用户手册：GMP 称重工作流 + FAQ (commit f776772)
- 移动端 CLI 模拟器：Mobile CLI 仿真支持 (commit bb8727e)
- 性能基准测试：OCR/ASR/DSL 性能测试 (commit a575fc6)
- 闭环节验证：6步验证流水线 (commit 71358d7)

### Changed
- AgentEngine：编排层架构升级，支持复杂工作流
- E2E 测试：从单元测试扩展到全链路测试
- 安全模块：从基础认证升级到 JWT + 电子签名
- 文档体系：用户手册 + API 规范 + 合规报告

### Fixed
- 待补充

### Performance
- 待补充

### Known Issues
- 向量索引未生成: 13809 embeddings 待填充 (需 Ollama bge-m3 运行时)
- cargo audit 待执行

---

## [1.1.0] - 2026-05-22 (GA)

### Added
- 集成测试套件：新增多通道融合集成测试 (commit 3bd664f)
- 向量通道完善：HNSW 索引查询优化 (commit 85bcc2a)
- Reranker 改进：CPU fallback 更加健壮，Cross-Encoder 调用逻辑优化 (commit cd81665)

### Changed
- BM25 算法修复：全文检索评分逻辑优化，提升关键词匹配相关性 (commit 8583e33)
- 保持 v1.0.0 基线：96.1% 通过率，MRR 0.497，QPS 8.3

### Fixed
- BM25 评分bug：修复全文检索中 BM25 公式计算错误
- Reranker fallback：优化 Ollama 不可用时的 fallback 逻辑
- 向量通道：修复 HNSW 索引查询边界条件

### Performance
- 向量查询延迟：P50 优化
- RRF 融合：四通道融合稳定性提升

### Known Issues
- 向量索引未生成: 13809 embeddings 待填充 (需 Ollama bge-m3 运行时)
- cargo audit 待执行

---

## [1.0.0] - 2026-05-21 (GA)

### Added
- 项目脚手架：Cargo workspace + CLI 框架 (clap)
- Layer 1 数据底座：SQLRustGo 统一存储 (RDBMS + Vector + FTS + Graph)
- 数据导入：1696 gmp-md 文档 → gmp_documents + gmp_document_chunks
- 向量索引：bge-m3 1024d 嵌入 + HNSW 索引
- 图谱构建：agent-knowledge → 实体节点 + 关系边 (≥700 节点, ≥1500 边)
- RuleChannel：agent-knowledge YAML 规则匹配，weight=2.0
- VectorChannel：bge-m3 语义相似度检索 (recall@20 ≥ 70%)
- FtsChannel：FTS5 关键词全文检索
- GraphChannel：Cypher + BFS 邻居扩展图谱召回
- RRF 融合引擎：四通道 Reciprocal Rank Fusion (k=60)
- Reranker：Ollama cosine similarity 重排 + Noop fallback
- LLM 问答：qwen2.5:14b RAG 生成 + 意图分类器(7种)
- CLI 全命令：index / search / evaluate / shell / bench
- 391 场景测试集评估 — R-Gate PASSED (96.1%, 392/408)
- gmp-eval crate：关键词内容匹配评估
- gmp-server crate：MySQL 协议独立部署
- 单元测试：135 tests 全部通过
- 用户手册 + 部署指南

### Changed
- SQLRustGo Phase 1-3 集成：SqlrustgoAdapter + HNSW + Cypher + gmp-server (PR #45)
- 向量通道：从 O(N) 全表扫描升级为 HNSW O(log N) 索引
- 图谱通道：从关键词匹配升级为 Cypher 引擎
- rusqlite 0.32 → 0.39 版本兼容性适配
- Runtime panic 修复：全局 Runtime 复用 (PR #43)
- 通道路由修复：search_sync 不再重复创建 Runtime (PR #43)

### Fixed
- Clippy 零警告 (PR #43, #49)
- search_sync Runtime 重复创建导致 panic (PR #43)
- 中文 FTS 分词支持 — jieba tokenizer 集成 (branch: fix/chinese-fts-jieba)
- #16-#20 遗留项全部完成 (PR #52+#53):
  - 增量索引 (`add_document`/`delete_document`/`incremental_stats`)
  - 性能压测 (多通道 bench + JSON 报告)
  - 中国 GMP 条文 (8 章 31 条 YAML)
  - 设备手册数据 (8 设备 SOP Markdown)
  - Web API RESTful (gmp-api crate, actix-web, 3 端点)

### Known Issues
- 向量索引未生成: 13809 embeddings 待填充 (需 Ollama bge-m3 运行时)
- Git Tag v1.1.0 + Gitea Release 待创建
