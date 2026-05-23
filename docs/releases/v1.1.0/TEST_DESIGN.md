# GMP-Platform v1.1.0 测试体系设计

> **版本**: 1.1.0
> **更新日期**: 2026-05-22
> **PR**: #83 (HNSW集成+SqlrustgoAdapter持久化)
> **Gate**: L0-Alpha/Beta/RC 全部 PASS

---

## 一、测试分层架构

```
┌─────────────────────────────────────────────────┐
│           408 场景测试 (手动/CI)                 │  ← 真实 GMP 场景，端到端
├─────────────────────────────────────────────────┤
│           集成测试 (tests/)                      │  ← 多 crate 协作
├─────────────────────────────────────────────────┤
│           单元测试 (#[test])                    │  ← 单个函数/模块
├─────────────────────────────────────────────────┤
│           性能压测 (cargo bench / bench cmd)    │  ← P50/P95/P99
└─────────────────────────────────────────────────┘
```

### 与 v1.0.0 差异

| 层级 | v1.0.0 | v1.1.0 变更 |
|------|--------|-------------|
| 场景测试 | 391 场景 | +17 场景 (408 总计) |
| HNSW 索引 | 无 | sqlrustgo_vector HNSW (M=16, ef=200/100) |
| Layer 0 | 无 | 新增 gmp-kg/gmp-classifier/gmp-re/gmp-scorer/gmp-graph |
| 持久化 | 内存 | SqlrustgoAdapter WAL 持久化 |

---

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
| StorageEngine::search_vectors | HNSW cosine sim 排序、top_k 限制 |
| StorageEngine::append_document | 单文档追加、ID 冲突处理 |
| StorageEngine::get_document_count | 初始 0、插入后 +N |
| cosine_sim | 单位向量、正交向量、边界值 |
| VectorSearchResult/SearchResult | from_row 解析 |

### 2.3 gmp-retrieval

| 模块 | 测试目标 | 测试用例 |
|------|---------|---------|
| fusion.rs | RrfFusion::fuse | 单通道、双通道、多通道、RRF 计算验证 |
| channel.rs | RuleChannel | 规则匹配、关键词命中、分类匹配 |
| channel.rs | VectorChannel | HNSW mock storage、embedding 调用 |
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

### 2.6 gmp-kg (Layer 0)

| 模块 | 测试目标 | 测试用例 |
|------|---------|---------|
| kg_storage.rs | KgStorage CRUD | 五张 Layer 0 表的增删改查 |
| classifier.rs | 6 维度分类 | topic/doc_type/risk_level/department/target_object/requisite_action |
| re.rs | 关系抽取 | 10+ 关系模板匹配 |
| scorer.rs | 5 维度评分 | 术语密度/法规引用度/流程完整性/实体密度/部门覆盖度 |
| graph_writer.rs | 图写入 | Layer 0 → Layer 2 节点/边创建 |

### 2.7 SqlrustgoAdapter 持久化

| 测试目标 | 测试用例 |
|---------|---------|
| HNSW 持久化 | 写入 → 重启 → 读取，索引一致性验证 |
| 图索引持久化 | 节点/边跨会话一致性 |
| WAL 恢复 | 模拟崩溃后数据完整性 |

---

## 三、集成测试设计

### 3.1 tests/ 目录结构

```
tests/
├── storage_integration.rs    # StorageEngine 全生命周期 + HNSW
├── retrieval_integration.rs  # 四通道 + 融合
├── reranker_integration.rs   # Reranker 端到端
├── layer0_integration.rs     # Layer 0 知识抽取管线 ★ 新增 ★
├── persistence_integration.rs # SqlrustgoAdapter 持久化 ★ 新增 ★
└── cli_smoke.rs             # CLI 命令 smoke test
```

### 3.2 storage_integration.rs

```
- test_storage_lifecycle: new → index → search → validate
- test_hnsw_search: HNSW 向量检索精度验证
- test_incremental_update: append → search → rebuild_vector
- test_document_crud: create → read → delete (标记删除)
- test_concurrent_search: 多线程并发读
```

### 3.3 retrieval_integration.rs

```
- test_four_channel_fusion: 四通道结果合并
- test_rrf_with_weights: 不同权重下的 RRF
- test_top_k_limiting: 结果数量限制
- test_hnsw_fallback: 向量 fallback 机制 ★ 新增 ★
```

### 3.4 layer0_integration.rs (新增)

```
- test_ner_extraction: 实体识别流程
- test_relation_extraction: 关系抽取验证
- test_classification: 6 维度分类
- test_relevance_scoring: 5 维度评分
- test_kg_crud: 五张表 CRUD 操作
- test_graph_write: Layer 0 → Layer 2 同步
```

### 3.5 persistence_integration.rs (新增)

```
- test_hnsw_persistence: 索引重启后一致性
- test_graph_persistence: 图索引持久化
- test_wal_recovery: 崩溃恢复验证
```

---

## 四、性能压测设计

### 4.1 bench 命令输出格式

```rust
// 输出格式
Benchmark Report
===============
Total queries: 800 (8 queries × 100 rounds)
Throughput: 154.0 qps  // v1.0.0: 26.7 qps

Latency Distribution
--------------------
P50:  ~4ms
P95:  6.5ms            // v1.0.0: 567ms
P99:  ~10ms
Min:  ~2ms
Max:  ~15ms

Per-Query Latency
-----------------
GMP原料药管理要求: P50=3.8ms
洁净区环境监测:     P50=4.1ms
...
```

### 4.2 性能目标

| 指标 | v1.0.0 基线 | v1.1.0 目标 | v1.1.0 实际 |
|------|-------------|-------------|-------------|
| QPS | 26.7 | > 100 | **154.0** |
| P50 | 234ms | < 50ms | **~4ms** |
| P95 | 567ms | < 100ms | **6.5ms** |
| P99 | 891ms | < 200ms | **~10ms** |

### 4.3 HNSW 参数

| 参数 | 值 | 说明 |
|------|-----|------|
| M | 16 | 最大连接数 |
| ef_construction | 200 | 构建时动态列表大小 |
| ef_search | 100 | 搜索时动态列表大小 |
| dims | 1024 | bge-m3 向量维度 |

---

## 五、测试数据

### 5.1 单元测试用小数据集

使用硬编码的测试数据，不依赖外部文件：
- 3-5 个测试文档 (markdown)
- 2-3 条测试规则 (YAML 格式)
- 测试查询字符串

### 5.2 集成测试用数据

使用 `gmp-md/` 中的真实数据子集（如果存在），否则使用嵌入的测试数据。

### 5.3 Layer 0 测试数据

| 数据 | 规模 |
|------|------|
| GMP 术语 | 508+ 术语, 4040 JSON 同义词 |
| 测试文档 | 5-10 份 markdown |
| 分类模板 | 6 维度 YAML |

---

## 六、执行方式

```bash
# 单元测试
cargo test

# 集成测试
cargo test --test '*'

# Layer 0 专项测试
cargo test --test layer0_integration

# 持久化专项测试
cargo test --test persistence_integration

# 性能压测
cargo run --release -- bin gmp-retrieve bench --concurrent 1 --rounds 10

# 完整测试套件
cargo test --all && cargo run --release -- bin gmp-retrieve bench --concurrent 1 --rounds 10
```

---

## 七、验收条件

### 7.1 单元测试覆盖率

| Crate | v1.0.0 最低要求 | v1.1.0 最低要求 |
|-------|----------------|----------------|
| gmp-core | ≥ 5 个测试用例 | ≥ 5 个测试用例 |
| gmp-storage | ≥ 15 个测试用例 | ≥ 18 个测试用例 (新增 HNSW) |
| gmp-retrieval | ≥ 15 个测试用例 | ≥ 18 个测试用例 (新增 fallback) |
| gmp-reranker | ≥ 5 个测试用例 | ≥ 5 个测试用例 |
| gmp-llm | ≥ 3 个测试用例 | ≥ 3 个测试用例 |
| gmp-kg | 无 | ≥ 15 个测试用例 (Layer 0 新增) |

### 7.2 集成测试覆盖率

| 测试文件 | v1.0.0 最低要求 | v1.1.0 最低要求 |
|----------|----------------|----------------|
| storage_integration.rs | ≥ 4 个测试 | ≥ 5 个测试 (新增 HNSW) |
| retrieval_integration.rs | ≥ 3 个测试 | ≥ 4 个测试 (新增 fallback) |
| layer0_integration.rs | 无 | ≥ 5 个测试 (新增) |
| persistence_integration.rs | 无 | ≥ 3 个测试 (新增) |

### 7.3 场景测试

| 指标 | v1.0.0 | v1.1.0 目标 | v1.1.0 实际 |
|------|--------|-------------|-------------|
| 场景总数 | 391 | 408 | **408** |
| 通过率 | 96.1% | > 98% | **99.8%** |
| 通过数量 | 376 | > 400 | **407** |

### 7.4 性能门禁

- [ ] `cargo test --all` 全部通过
- [ ] 408 场景通过率 ≥ 99%
- [ ] bench QPS ≥ 100
- [ ] P95 延迟 ≤ 10ms
- [ ] L0-Alpha/Beta/RC 全部 PASS
