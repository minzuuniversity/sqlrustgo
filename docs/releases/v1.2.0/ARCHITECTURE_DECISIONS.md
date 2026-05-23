# GMP-Platform v1.2.0 架构决策记录

> **版本**: v1.2.0 | **日期**: 2026-05-23 | **分支**: `develop/v1.2.0`

---

## ADR-001: 四层检索架构演进

### 状态: 已接受

### 背景

v1.2.0 需要在 v1.1.0 基础上实现向量通道完全集成和 NL→Cypher 转换器，解决图查询场景 0% 通过率问题。

### 决策

保持四层架构不变，增强 Layer 2 检索引擎：

```
Layer 1: SQLRustGo Storage (RDBMS + Vector + FTS + Graph)
    ↓
Layer 2: Retrieval Engine
    ├── RuleChannel (YAML, weight=2.0)
    ├── VectorChannel (HNSW fully integrated)
    ├── FtsChannel (BM25 + LIKE n-gram)
    └── GraphChannel (NL→Cypher converter)
    ↓
Layer 3: Reranker (bge-reranker-v2-m3)
    ↓
Layer 4: LLM (qwen2.5:14b)
```

### 理由

1. 四层架构已在 v1.0.0/v1.1.0 验证，无需重构
2. 向量通道问题仅是集成问题，非架构问题
3. NL→Cypher 可作为 GraphChannel 的前置转换层

### 后果

- **正面**: 改动最小，风险可控
- **负面**: GraphChannel 依赖 LLM 进行 NL→Cypher 转换，增加延迟

---

## ADR-002: HNSW 向量索引完全集成方案

### 状态: 已接受

### 背景

v1.1.0 PR #73 引入了 HnswVectorIndex，但未集成到 RetrievalEngine，导致向量通道通过率 0/89。

### 决策

在 `RetrievalEngine::search()` 中使用 `tokio::join!` 并行执行四通道：

```rust
pub async fn search(&self, query: &str, top_k: usize) -> Vec<ScoredChunk> {
    let (fts_results, vector_results, cypher_results, rule_results) = tokio::join!(
        self.fts_channel.search(query, top_k),
        self.vector_channel.search(&self.embed(query), top_k),
        self.cypher_engine.query(query),  // 需 NL→Cypher
        self.rule_channel.search(query, top_k),
    );

    self.rrf_fusion(fts_results, vector_results, cypher_results, rule_results, top_k)
}
```

### 理由

1. `tokio::join!` 可并行执行四个通道，最大化吞吐量
2. RRF 融合在所有通道结果返回后执行
3. 向量嵌入需调用 Ollama，仍有网络开销

### 后果

- **正面**: 向量通道启用，预期通过率 50%+
- **负面**: Ollama 网络延迟仍是瓶颈

---

## ADR-003: NL → Cypher 转换器设计

### 状态: 已接受

### 背景

图查询类别（部门角色检索、多跳因果推理）0% 通过，因为 CypherEngine 已实现但无法从自然语言查询自动转换为 Cypher。

### 决策

采用 LLM辅助转换方案：

```rust
pub structNlToCypherConverter {
    llm: Arc<OllamaClient>,
}

implNlToCypherConverter {
    pub async fn convert(&self, nl_query: &str) -> Result<String> {
        let prompt = format!(
            "将以下自然语言查询转换为 OpenCypher:\n{}\n\
            只返回 Cypher 查询，不包含解释。",
            nl_query
        );
        self.llm.generate(&prompt).await
    }
}
```

### 理由

1. LLM 已集成（qwen2.5:14b），复用现有依赖
2. GMP 领域查询相对结构化，转换准确率可接受
3. 避免手工编写规则映射

### 后果

- **正面**: 图查询能力大幅提升
- **负面**: LLM 调用增加 500ms-1s 延迟

---

## ADR-004: LLM-as-Judge 评估指标

### 状态: 已接受

### 背景

当前评估使用精确 `doc_id` 字符串匹配，导致语义相关的文档被误判为失败（如 "物料验收规程" vs "原料入库验收规程"）。

### 决策

引入双轨评估：

1. **精确匹配** (Primary): doc_id 完全相等
2. **语义相似度** (Secondary): LLM 判断 chunk_content 与 expected 语义相关性

```rust
pub enum JudgeResult {
    ExactMatch,      // doc_id 完全相等
    SemanticMatch,   // LLM 判断语义相关
    NoMatch,         // 不相关
}
```

### 理由

1. 保留精确匹配作为主要指标（符合 GMP 可追溯要求）
2. 语义评估作为辅助，帮助识别误判
3. 不影响现有通过率计算逻辑

### 后果

- **正面**: 评估更准确，误判率降低
- **负面**: LLM 调用增加评估时间

---

## ADR-005: Z440 图索引持久化

### 状态: 已接受

### 背景

CypherEngine::execute() 在 v1.1.0 中是 stub 实现，图索引无法持久化。

### 决策

完整实现 Cypher 执行引擎：

1. **Pattern Analyzer**: 解析 MATCH, WHERE, RETURN 等子句
2. **Graph Index Lookup**: 通过 SqlrustgoAdapter 查询 DiskGraphStore
3. **Result Formatter**: 格式化返回结果

```rust
impl CypherEngine {
    pub async fn execute(&self, query: &str) -> Result<Vec<CypherResult>> {
        let ast = self.parser.parse(query)?;
        let plan = self.planner.plan(&ast)?;
        self.executor.execute(&plan).await
    }
}
```

### 理由

1. 图数据持久化是 GMP 合规要求
2. 崩溃恢复需要完整的持久化支持
3. 与 SqlrustgoAdapter 已有的 DiskGraphStore 对接

### 后果

- **正面**: 图数据可靠持久化
- **负面**: 实现复杂度增加

---

## ADR-006: gmp-api 连接池优化

### 状态: 已接受

### 背景

gmp-api v1.1.0 使用简单连接模式，高并发下性能受限。

### 决策

引入连接池：

```rust
pub struct ApiServer {
    pool: sqlrustgo::ConnectionPool,
    max_connections: usize,
}

impl ApiServer {
    pub async fn new(pool_size: usize) -> Result<Self> {
        let pool = sqlrustgo::ConnectionPool::builder()
            .max_connections(pool_size)
            .build()
            .await?;
        Ok(Self { pool, max_connections: pool_size })
    }
}
```

### 理由

1. 连接复用减少连接建立开销
2. 限制最大连接数防止资源耗尽
3. 与 sqlrustgo Phase 4 规划一致

### 后果

- **正面**: API 并发能力 +50%
- **负面**: 需要配置连接池大小

---

## ADR-007: 向后兼容性策略

### 状态: 已接受

### 背景

v1.2.0 引入新功能（NL→Cypher、LLM-as-Judge），需确保向后兼容。

### 决策

1. **API 向后兼容**: 所有现有端点保持不变
2. **CLI 参数兼容**: 新增 `--graph` 和 `--judge-mode` 为可选参数
3. **数据库 Schema**: 仅新增表/索引，不修改现有结构

### 理由

1. v1.1.0 用户应能平滑升级
2. GMP 环境变更控制要求严格
3. 减少回归测试工作量

### 后果

- **正面**: 升级风险低
- **负面**: 需要维护兼容性代码

---

## 变更记录

| ADR | 状态 | 日期 | 说明 |
|-----|------|------|------|
| ADR-001 | 已接受 | 2026-05-23 | 四层架构演进 |
| ADR-002 | 已接受 | 2026-05-23 | HNSW 完全集成 |
| ADR-003 | 已接受 | 2026-05-23 | NL→Cypher 转换器 |
| ADR-004 | 已接受 | 2026-05-23 | LLM-as-Judge |
| ADR-005 | 已接受 | 2026-05-23 | Z440 图索引持久化 |
| ADR-006 | 已接受 | 2026-05-23 | 连接池优化 |
| ADR-007 | 已接受 | 2026-05-23 | 向后兼容性 |
