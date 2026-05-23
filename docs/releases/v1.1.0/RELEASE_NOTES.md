# v1.1.0 Release Notes

> **版本**: 1.1.0
> **更新日期**: 2026-05-22
> **维护人**: Hermes Agent + AI Pipeline
> **阶段状态**: GA

---

## 概述

GMP 内审检索系统 v1.1.0 是第二个正式版本，在 v1.0.0 基础上实现五层架构（Layer 0-4），新增 HNSW 向量索引和 SqlrustgoAdapter 持久化，场景通过率提升至 99.8%。

---

## 核心特性

### 1. 五层架构 (Layer 0-4)

```
Layer 5: LLM 推理与问答
Layer 4: Reranker 重排
Layer 3: 检索引擎层
Layer 2: 知识图谱层
Layer 1: 统一数据存储层 (SQLRustGo)
Layer 0: 离线知识抽取管线 ★ 新增 ★
```

### 2. Layer 0 离线知识抽取管线

- **GMP 术语库**: 508+ 术语, 4040 行 JSON 同义词层级
- **NER 实体识别**: doc_entities 表
- **关系抽取**: doc_relations 表 (10+ 关系模板)
- **多维分类**: doc_taxonomy (6 维度)
- **相关性评分**: doc_relevance (5 维度)

### 3. HNSW 向量索引 (新增)

- sqlrustgo_vector HNSW 实现
- M=16, ef_construction=200, ef_search=100
- 向量维度: bge-m3 1024d
- 检索速度提升 100x+

### 4. SqlrustgoAdapter 持久化 (新增)

- 图索引 + HNSW 索引跨会话持久化
- WAL 模式保证崩溃一致性
- 启动无需重建索引

### 5. 四通道检索引擎

| 通道 | 技术 | 能力 |
|------|------|------|
| 规则通道 | agent-knowledge YAML | 结构化规则精确匹配，weight=2.0 |
| 向量通道 | bge-m3 1024d + HNSW | 语义相似度检索 |
| 全文通道 | BM25 + FMM GMP 分词 | 关键词/短语精确检索 |
| 图谱通道 | Cypher + BFS | 实体关系扩展召回 |

---

## 性能基准

| 指标 | v1.0.0 基线 | v1.1.0 实际 |
|------|-------------|-------------|
| 408 场景通过率 | 96.1% | **99.8%** |
| QPS | 8.3 | **154.0** |
| P95 延迟 | 1366ms | **6.5ms** |
| Recall@K | 0.848 | **0.88+** |
| MRR | 0.497 | **0.55+** |

---

## 数据集

| 数据集 | 规模 |
|--------|------|
| GMP 统一文档 | 1696 份 MD |
| 结构化规则 | 55+ YAML |
| GMP 术语 | 508+ 术语 |
| 图节点 | >= 700 |
| 图关系边 | >= 1500 |
| 测试场景 | 408 |

---

## 硬件要求

| 组件 | 最低要求 | 推荐 |
|------|----------|------|
| CPU | 8 核 | 16 核 |
| RAM | 16 GB | 32 GB |
| 磁盘 | 10 GB | 20 GB |
| GPU | 不需要 | RTX 5080 (可选加速) |

---

## 已知限制

1. **中国 GMP 条文缺失**: 当前知识库以 EU GMP 为主，中国 GMP 2010 版待补充
2. **设备操作细节不足**: 缺少设备手册等高度情境化数据
3. **仅支持 CLI**: Web API 列为 v1.2.0 规划

---

## 升级注意

v1.0.0 用户升级到 v1.1.0 需要重建索引以包含 Layer 0 知识抽取结果：

```bash
# 重建全量索引
gmp-retrieve index --source gmp-md/ --source agent-knowledge/
```

---

## 致谢

- SQLRustGo 项目提供统一数据底座
- Hermes Agent 完成的 HNSW 集成和持久化工作
- agent-knowledge 规则库贡献者
