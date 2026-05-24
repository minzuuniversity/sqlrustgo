# v1.3.0 功能矩阵

> **版本**: 1.3.0
> **更新日期**: 2026-05-24
> **分支**: `develop/v1.3.0`
> **基准对比**: v1.2.0 → v1.3.0

---

## 五层架构概览

```
┌─────────────────────────────────────────────────────────────────────────┐
│ Layer 5: LLM 推理与问答                                                 │
│   qwen2.5:14b (Ollama) + 意图分类 + 答案生成 + 引用溯源               │
├─────────────────────────────────────────────────────────────────────────┤
│ Layer 4: Reranker 重排                                                 │
│   BGE-Reranker-v2-m3 (Cross-Encoder) + RRF 融合                       │
├─────────────────────────────────────────────────────────────────────────┤
│ Layer 3: 检索引擎层                                                     │
│   四通道: 规则/向量/全文/图谱 → Top-30 → RRF(k=60) → Top-20          │
├─────────────────────────────────────────────────────────────────────────┤
│ Layer 2: 知识图谱层                                                     │
│   GMP 实体节点 + 语义关系边 + Cypher 查询 + 路径推理                     │
│   节点: SOP/Deviation/CAPA/Validation/Device/Material 等                │
│   关系: TRIGGERS/BELONGS_TO/MANAGED_BY/REQUIRES/MANDATES 等           │
├─────────────────────────────────────────────────────────────────────────┤
│ Layer 1: 统一数据存储层 — SQLRustGo                                     │
│   RDBMS: gmp_documents + gmp_document_chunks                            │
│   Vector: gmp_embeddings_bin (HNSW, bge-m3 1024d, binary BLOB)         │
│   FTS: gmp_chunks_fts (InvertedIndex + jieba 分词)                     │
│   Graph: gmp_graph_nodes + gmp_graph_edges (DiskGraphStore + WAL)       │
├─────────────────────────────────────────────────────────────────────────┤
│ Layer 0: 离线知识抽取管线 ★ 初始化时运行 ★                              │
│   Step 0: GMP 术语库构建 (gmp_terms + gmp_synonyms)                    │
│   Step 1: NER 实体识别 → doc_entities → Layer 2 节点                    │
│   Step 2: 关系抽取 (RE) → doc_relations → Layer 2 边                   │
│   Step 3: 多维分类 → doc_taxonomy (topic/doc_type/risk/department)      │
│   Step 4: 相关性评分 → doc_relevance (term_density/reg_citation 等)     │
│   Step 5: 图谱写入 → Layer 2 更新 (gmp_graph_nodes + gmp_graph_edges)   │
└─────────────────────────────────────────────────────────────────────────┘
```

### 层级关系

```
Layer 0 输出 → Layer 2 (知识图谱节点/边补充)
Layer 0 输出 → Layer 1 (补充字段: entity_tags, taxonomy_path, relevance_score)
Layer 0 输出 → Layer 3 (检索时可选: 分类过滤/相关性增强)
Layer 0 输出 → Layer 4 (答案引用可溯源到图谱实体路径)
```

---

## Layer 0: 离线知识抽取管线

> ★ 初始化时运行一次，为检索系统补充结构化知识

| 功能 | 状态 | v1.2.0 | v1.3.0 | 说明 |
|------|------|---------|---------|------|
| GMP 术语库 (gmp_terms + gmp_synonyms) | 🔄 待开发 | ❌ | 🔄 待开发 | Issue #124 |
| NER 实体识别 | 🔄 待开发 | ❌ | 🔄 待开发 | Issue #124 |
| 关系抽取 (RE) | 🔄 待开发 | ❌ | 🔄 待开发 | Issue #124 |
| 多维分类 (taxonomy) | 🔄 待开发 | ❌ | 🔄 待开发 | Issue #124 |
| 相关性预评分 | 🔄 待开发 | ❌ | 🔄 待开发 | Issue #124 |
| 图谱写入 (Layer 2 更新) | 🔄 待开发 | ❌ | 🔄 待开发 | Issue #124 |

---

## Layer 1: 统一数据存储层

| 功能 | 状态 | v1.2.0 | v1.3.0 | 说明 |
|------|------|---------|---------|------|
| 文档主表 (gmp_documents) | ✅ GA | GA | GA | 1696 文档元数据 |
| 内容表 (gmp_document_chunks) | ✅ GA | GA | GA | ~8700 chunks |
| 向量嵌入表 (gmp_embeddings_bin) | ✅ **新增** | ❌ 分行 | ✅ 完整 | Binary BLOB + HNSW |
| HNSW 索引自动构建 | ✅ **新增** | ❌ 缺失 | ✅ 完整 | `init()` 时自动构建 |
| 图节点存储 (gmp_graph_nodes) | 🔄 开发中 | GA | 🔄 增强 | WAL 持久化开发中 |
| 图关系边 (gmp_graph_edges) | ✅ GA | GA | GA | |
| WAL 持久化 | 🔄 开发中 | ❌ | 🔄 增强 | Issue #123 |
| FTS 全文索引 (gmp_chunks_fts) | ✅ GA | GA | GA | InvertedIndex + jieba |

---

## Layer 2: 知识图谱层

| 功能 | 状态 | v1.2.0 | v1.3.0 | 说明 |
|------|------|---------|---------|------|
| Cypher 查询引擎 | 🔄 开发中 | GA | 🔄 增强 | DiskGraphStore WAL 依赖 |
| 节点类型 (SOP/Deviation/CAPA等) | 🔄 开发中 | GA | 🔄 增强 | Layer 0 管线补充 |
| 关系类型 (TRIGGERS/BELONGS_TO等) | 🔄 开发中 | GA | 🔄 增强 | Layer 0 管线补充 |
| 路径推理 (BFS/DFS) | 🔄 开发中 | GA | 🔄 增强 | WAL 依赖 |
| 图谱统计 (节点数/边数) | ✅ GA | GA | GA | |

---

## Layer 3: 检索引擎层

| 功能 | 状态 | v1.2.0 | v1.3.0 | 说明 |
|------|------|---------|---------|------|
| RuleChannel (weight=2.0) | ✅ GA | GA | GA | |
| VectorChannel (HNSW) | ✅ GA | GA | GA | Binary BLOB 存储 |
| FtsChannel (BM25) | ✅ GA | GA | GA | |
| GraphChannel (Cypher) | 🔄 开发中 | GA | 🔄 增强 | WAL 依赖 |
| RRF 融合 (k=60) | ✅ GA | GA | GA | |
| 向量索引生成 | 🔄 待开发 | ❌ | 🔄 待开发 | Issue #122 |

---

## Layer 4: Reranker 重排层

| 功能 | 状态 | v1.2.0 | v1.3.0 | 说明 |
|------|------|---------|---------|------|
| OllamaReranker | ✅ GA | GA | GA | cosine similarity |
| NoopReranker | ✅ GA | GA | GA | |
| BGE-Reranker-v2-m3 | 🔄 待集成 | ❌ | 🔄 待集成 | Cross-Encoder |

---

## Layer 5: LLM 推理与问答层

| 功能 | 状态 | v1.2.0 | v1.3.0 | 说明 |
|------|------|---------|---------|------|
| Ollama LLM Client | ✅ GA | GA | GA | |
| Intent Classification (7种) | ✅ GA | GA | GA | |
| RAG 生成 | ✅ GA | GA | GA | |
| 引用溯源 | 🔄 待增强 | 🔄 | 🔄 待增强 | Layer 0 图谱依赖 |

---

## 平台能力层

| 功能 | 状态 | v1.2.0 | v1.3.0 | 说明 |
|------|------|---------|---------|------|
| Event Bus (InMemoryBus) | ✅ **新增** | ❌ | ✅ 完整 | PR #133, Issue #131 |
| Audit Middleware | ✅ **新增** | ❌ | ✅ 完整 | Issue #132 |
| AuditContext | ✅ **新增** | ❌ | ✅ 完整 | |
| AuditEmitter | ✅ **新增** | ❌ | ✅ 完整 | |
| hash_event 工具 | ✅ **新增** | ❌ | ✅ 完整 | |
| MQTT Gateway | ✅ **新增** | ❌ | ✅ 完整 | PR #136, Issue #125 |
| JWT 认证 | ✅ GA | GA | GA | |
| gmp-server (MySQL 协议) | ✅ GA | GA | GA | |
| Web Dashboard | 🔄 增强 | 🔄 | 🔄 增强中 | |

---

## CLI / API / Tools

| 功能 | 状态 | v1.2.0 | v1.3.0 | 说明 |
|------|------|---------|---------|------|
| `gmp index` | ✅ GA | GA | GA | |
| `gmp search` | ✅ GA | GA | GA | |
| `gmp evaluate` | ✅ GA | GA | GA | |
| `gmp shell` | ✅ GA | GA | GA | |
| `gmp bench` | ✅ GA | GA | GA | |
| `gmp audit` | ✅ **新增** | ❌ | ✅ 完整 | commit `00a511e` |
| `gmp populate` | 🔄 DEPRECATED | ✅ | 🔄 废弃 | commit `021f075` |
| gmp-api REST API | 🔄 增强 | 🔄 | 🔄 增强中 | |

---

## 质量门禁

| 指标 | 状态 | v1.2.0 | v1.3.0 | 说明 |
|------|------|---------|---------|------|
| `cargo build --release` | ✅ PASS | ✅ | ✅ | |
| `cargo clippy -D warnings` | ✅ PASS | ✅ | ✅ | PR #135 |
| `cargo audit` | ✅ 0 vulns | ⚠️ | ✅ | PR #135 |
| `cargo fmt` | ✅ PASS | ✅ | ✅ | |
| 单元测试 | 🔄 待验证 | 🔄 | 🔄 | |
| 391 场景回归 | 🔄 待验证 | 96.08% | 🔄 | 无退化要求 |

---

## 开发工具与治理

| 功能 | 状态 | v1.2.0 | v1.3.0 | 说明 |
|------|------|---------|---------|------|
| GitHub Actions CI | ✅ | ✅ | ✅ | |
| L0-Alpha Gate | 🔄 待执行 | ✅ | 🔄 待执行 | |
| L0-Beta Gate | 🔄 待执行 | 🔄 | 🔄 待执行 | |
| L0-RC Gate | 🔄 待执行 | 🔄 | 🔄 待执行 | |
| 21-docs 文档体系 | ✅ Beta | 🔄 | ✅ 完成 | VERSION_DOCS_SPEC |

---

## P0 功能开发进度

| 功能 | Issue | 分支 | 状态 |
|------|-------|------|------|
| 向量索引生成流水线 (13809 embeddings) | #122 | `feature/issue-122-vector-embeddings` | 🔄 开发中 |
| DiskGraphStore WAL 持久化 | #123 | `feature/issue-123-disk-graph-wal` | ✅ 已合入 |
| Layer 0 知识抽取管线 (术语库+NER+RE+分类+预评分) | #124 | 待创建 | 🔄 规划中 |
