# GMP-Platform v1.1.0 测试覆盖率报告

> **版本**: v1.1.0
> **日期**: 2026-05-22
> **维护人**: Hermes Agent + AI Pipeline

---

## 一、测试覆盖概览

| 类别 | 数量 | 状态 |
|------|------|------|
| 单元测试 | 137+ | ✅ |
| 集成测试 | 7+ | ✅ |
| 场景测试 | 408 | ✅ |
| 性能测试 | CLI可用 | ✅ |
| **总计** | **550+** | |

---

## 二、单元测试覆盖

### 2.1 按 Crate 分布

| Crate | 测试数 | 覆盖内容 |
|-------|--------|---------|
| gmp-storage | 20+ | FTS、向量、存储 CRUD |
| gmp-retrieval | 30+ | 四通道、RRF 融合 |
| gmp-reranker | 10+ | cosine similarity、rerank |
| gmp-llm | 15+ | intent classification、chat |
| gmp-kg | 20+ | 术语库 CRUD、NER、RE |
| gmp-classifier | 15+ | 多维分类 |
| gmp-scorer | 10+ | 相关性评分 |
| gmp-graph | 10+ | 图写入、查询 |
| gmp-cli | 7+ | CLI 命令 |

### 2.2 关键路径覆盖

| 功能路径 | 测试覆盖 |
|---------|---------|
| index -> FTS -> search | ✅ |
| index -> vector -> search | ✅ |
| index -> graph -> search | ✅ |
| search -> RRF -> rerank | ✅ |
| search -> LLM -> chat | ✅ |
| Layer 0: term -> NER -> RE -> classify -> score | ✅ |

---

## 三、场景测试覆盖

### 3.1 408 场景测试结果

| 指标 | 数值 |
|------|------|
| 总场景数 | 408 |
| 通过数 | 407 |
| 失败数 | 1 |
| 通过率 | 99.8% |

### 3.2 场景分类覆盖

| 类别 | 场景数 | 通过率 |
|------|--------|--------|
| 偏差管理 | 50+ | 100% |
| CAPA | 40+ | 100% |
| 验证 | 45+ | 100% |
| 变更控制 | 35+ | 100% |
| 审计 | 38+ | 100% |
| 物料管理 | 40+ | 100% |
| 设备管理 | 35+ | 100% |
| 生产管理 | 40+ | 100% |
| 质量管理 | 45+ | 100% |
| 其他 | 40+ | 97.5% |

---

## 四、性能测试覆盖

### 4.1 性能指标覆盖

| 指标 | 测试方法 | 阈值 | 实际 | 状态 |
|------|---------|------|------|------|
| QPS | gmp bench | > 50 | 154.0 | ✅ |
| P95 延迟 | gmp bench | < 10ms | 6.5ms | ✅ |
| P50 延迟 | gmp bench | < 5ms | ~2ms | ✅ |
| 并发稳定性 | gmp bench --concurrent 4 | 0 崩溃 | 0 崩溃 | ✅ |

### 4.2 压测场景

| 场景 | 并发数 | 轮次 | QPS | P95 |
|------|--------|------|-----|-----|
| 全通道检索 | 4 | 10 | 154.0 | 6.5ms |

---

## 五、集成测试覆盖

### 5.1 组件集成

| 集成测试 | 描述 | 状态 |
|---------|------|------|
| test_full_index_and_search_cycle | 索引->搜索完整流程 | ✅ |
| test_incremental_reindex_detects_changes | 增量检测变化 | ✅ |
| test_validate_with_multiple_docs | 多文档验证 | ✅ |
| test_search_fts_result_scoring | 评分排序 | ✅ |
| test_hnsw_vector_search | HNSW 向量搜索 | ✅ |
| test_sqlrustgo_adapter_persistence | SqlrustgoAdapter 持久化 | ✅ |
| test_layer0_pipeline | Layer 0 管线 | ✅ |

---

## 六、门禁覆盖

| Gate | 检查项 | 状态 |
|------|--------|------|
| L0-Alpha | A1-A9 (9项) | ✅ PASS |
| L0-Beta | B1-B10 (10项) | ✅ PASS |
| L0-RC | R1-R12 (12项) | ✅ PASS |
| GA | G1-G7 (7项) | ✅ PASS |

**门禁总检查项**: 38 项
**门禁通过项**: 38 项
**门禁通过率**: 100%

---

## 七、覆盖率总结

| 维度 | 覆盖率 | 状态 |
|------|--------|------|
| 单元测试 | 137+ tests | ✅ |
| 集成测试 | 7+ tests | ✅ |
| 场景测试 | 408 scenarios | ✅ |
| 性能测试 | 全部指标 | ✅ |
| 门禁检查 | 38/38 | ✅ |

**综合评估**: v1.1.0 测试覆盖达到优秀水平，所有核心功能和性能指标均有测试覆盖。
