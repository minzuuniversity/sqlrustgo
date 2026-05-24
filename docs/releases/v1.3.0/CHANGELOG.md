# Changelog

## [1.3.0] - 2026-05-24 (Beta)

### Added

- **Event Bus Crate** (`gmp-event-bus`): 全新事件驱动架构 — `GmpEvent` 类型系统、`InMemoryBus` 发布订阅、`AuditHandler` 审计处理器 (commit `0104cb9`, PR #133, Issue #131)
- **Audit Middleware**: `AuditContext` 请求级上下文、`AuditEmitter` 便捷 API、`hash_event` 完整性校验 (commit `7380869`, Issue #132)
- **Binary BLOB Embeddings + HNSW auto-build**: 向量数据以 binary BLOB 存储，`init()` 时自动构建 HNSW 索引，解决长期遗留的向量索引缺失问题 (commit `3aeccf0`, Issue #122)
- **DiskGraphStore WAL Persistence**: 图数据 WAL 预写日志持久化，支持 CypherEngine 重启恢复 (commit `cf65908`, Issue #123)
- **MQTT Gateway**: `MqttGateway` 设备集成，支持称重设备、温湿度传感器等 IoT 设备事件上报 (commit `28d1af5`, PR #136, Issue #125)
- **MQTT Integration via Event Bus**: MQTT 事件通过 Event Bus 路由到 AuditHandler

### Changed

- `gmp-storage`: `init()` 行为变更 — 每次初始化时自动重建 HNSW 索引，不再依赖手动触发
- `crates/gmp-cli`: `Commands::Populate` 标记为 DEPRECATED，body 简化为提示信息

### Fixed

- `gmp-storage`: `TempDir::keep()` 替代废弃的 `into_path()` (clippy fix, commit `4dbd8a3`)
- `gmp-cli`: 修复 `Commands::Populate` 中未定义变量导致的 E0425 编译错误 (commit `021f075`)
- clippy warnings: 清理 7 个 pre-existing warnings（unused imports, unnecessary_unwrap, useless_vec, module-inception）
- cargo audit: 0 vulnerabilities（3 个传递依赖警告已 acknowledge）

### Performance

- HNSW 索引初始化性能改善（binary BLOB 直接加载 vs 分行存储）

### Known Issues

- 向量索引仍为空（BLOB 存储就绪，13809 embeddings 待生成流水线填充）
- Layer 0 知识抽取管线（P0-3~7）尚未开发
- MQTT 设备集成需实际设备测试

---

## [1.2.0] - 2026-05-23 (Beta)

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

- 向量通道：HNSW 索引查询边界条件
- E2E 测试框架稳定性

---

## [1.1.0] - 2026-05-22 (GA)

### Added

- 集成测试套件：多通道融合集成测试
- 向量通道完善：HNSW 索引查询优化

### Changed

- BM25 算法修复：全文检索评分逻辑优化
- 保持 v1.0.0 基线：96.1% 通过率，MRR 0.497，QPS 8.3

### Fixed

- BM25 评分 bug：全文检索中 BM25 公式计算错误
- Reranker fallback：优化 Ollama 不可用时的 fallback 逻辑
- 中文 FTS 分词支持 — jieba tokenizer 集成

---

## [1.0.0] - 2026-05-21 (GA)

### Added

- 项目脚手架：Cargo workspace + CLI 框架
- Layer 1 数据底座：SQLRustGo 统一存储（RDBMS + Vector + FTS + Graph）
- 四通道检索：Rule + Vector + FTS + Graph
- RRF 融合引擎：Reciprocal Rank Fusion (k=60)
- Reranker：Ollama cosine similarity 重排
- LLM 问答：qwen2.5:14b RAG 生成
- 391 场景测试集评估 — 96.1% 通过率
