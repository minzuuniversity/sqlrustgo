# GMP-Platform v1.0.0 测试体系设计

## 一、测试分层架构

```
┌─────────────────────────────────────────────────┐
│           391 场景测试 (手动/CI)                 │  ← 真实 GMP 场景，端到端
├─────────────────────────────────────────────────┤
│           集成测试 (tests/)                      │  ← 多 crate 协作
├─────────────────────────────────────────────────┤
│           单元测试 (#[test])                    │  ← 单个函数/模块
├─────────────────────────────────────────────────┤
│           性能压测 (cargo bench / bench cmd)    │  ← P50/P95/P99
└─────────────────────────────────────────────────┘
```

## 二、单元测试设计

### 2.1 gmp-core (types.rs)

| 测试目标 | 测试用例 |
|---------|---------|
| ScoredChunk | 排序、克隆、序列化 |
| ChannelType | variant 覆盖、JSON 序列化 |
| KnowledgeRule | from_yaml、验证 |
| 边界条件 | 空 content、超长 doc_id |

### 2.2 gmp-storage (storage.rs)

| 测试目标 | 测试用例 |
|---------|---------|
| StorageEngine::new | 内存临时目录初始化 |
| StorageEngine::index_documents | 空目录、小文档集(3-5个) |
| StorageEngine::search_fts | 关键词匹配、空查询、多结果 |
| StorageEngine::search_vectors | cosine sim 排序、top_k 限制 |
| StorageEngine::append_document | 单文档追加、ID 冲突处理 |
| StorageEngine::get_document_count | 初始 0、插入后 +N |
| cosine_sim | 单位向量、正交向量、边界值 |
| VectorSearchResult/SearchResult | from_row 解析 |

### 2.3 gmp-retrieval

| 模块 | 测试目标 | 测试用例 |
|------|---------|---------|
| fusion.rs | RrfFusion::fuse | 单通道、双通道、多通道、RRF 计算验证 |
| channel.rs | RuleChannel | 规则匹配、关键词命中、分类匹配 |
| channel.rs | VectorChannel | mock storage、embedding 调用 |
| channel.rs | FtsChannel | FTS 搜索、结果排序 |
| channel.rs | GraphChannel | 实体提取、规则匹配 |
| channel.rs | FourChannelEngine | 四通道并行、结果聚合 |

### 2.4 gmp-reranker

| 测试目标 | 测试用例 |
|---------|---------|
| Reranker trait | objects_satisfy |
| NoopReranker | 返回原始顺序 |
| OllamaReranker | mock HTTP 响应、cosine 计算 |

### 2.5 gmp-llm

| 测试目标 | 测试用例 |
|---------|---------|
| IntentClassifier | 7 种意图分类 |
| parse_llama_chat_response | JSON 解析、错误处理 |

## 三、集成测试设计

### 3.1 tests/ 目录结构

```
tests/
├── storage_integration.rs    # StorageEngine 全生命周期
├── retrieval_integration.rs  # 四通道 + 融合
├── reranker_integration.rs   # Reranker 端到端
└── cli_smoke.rs             # CLI 命令 smoke test
```

### 3.2 storage_integration.rs

```
- test_storage_lifecycle: new → index → search → validate
- test_incremental_update: append → search → rebuild_vector
- test_document_crud: create → read → delete (标记删除)
- test_concurrent_search: 多线程并发读
```

### 3.3 retrieval_integration.rs

```
- test_four_channel_fusion: 四通道结果合并
- test_rrf_with_weights: 不同权重下的 RRF
- test_top_k_limiting: 结果数量限制
```

## 四、性能压测设计

### 4.1 bench 命令改进

```rust
// 输出格式
Benchmark Report
===============
Total queries: 800 (8 queries × 100 rounds)
Throughput: 26.7 qps

Latency Distribution
--------------------
P50:  234ms
P95:  567ms
P99:  891ms
Min:  123ms
Max:  1203ms

Per-Query Latency
-----------------
GMP原料药管理要求: P50=220ms
洁净区环境监测:     P50=245ms
...
```

### 4.2 性能目标

| 指标 | 目标 | Alpha 基线 |
|------|------|-----------|
| P50 | < 500ms | 待测量 |
| P95 | < 1500ms | 待测量 |
| P99 | < 3000ms | 待测量 |

## 五、测试数据

### 5.1 单元测试用小数据集

使用硬编码的测试数据，不依赖外部文件：
- 3-5 个测试文档 (markdown)
- 2-3 条测试规则 (YAML 格式)
- 测试查询字符串

### 5.2 集成测试用数据

使用 `gmp-md/` 中的真实数据子集（如果存在），否则使用嵌入的测试数据。

## 六、执行方式

```bash
# 单元测试
cargo test

# 集成测试
cargo test --test '*'

# 性能压测
cargo run --release -- bin gmp-retrieve bench --concurrent 1 --rounds 10

# 完整测试套件
cargo test --all && cargo run --release -- bin gmp-retrieve bench --concurrent 1 --rounds 10
```

## 七、验收条件

- [ ] gmp-core: ≥ 5 个测试用例
- [ ] gmp-storage: ≥ 15 个测试用例  
- [ ] gmp-retrieval: ≥ 15 个测试用例
- [ ] gmp-reranker: ≥ 5 个测试用例
- [ ] gmp-llm: ≥ 3 个测试用例
- [ ] 集成测试: ≥ 4 个测试文件
- [ ] bench 命令输出完整统计数据
- [ ] `cargo test` 全部通过
