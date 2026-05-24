# v1.3.0 功能矩阵

> **版本**: 1.3.0
> **更新日期**: 2026-05-24
> **分支**: `develop/v1.3.0`
> **基准对比**: v1.2.0 → v1.3.0

---

## Layer 1: 数据存储层

| 功能 | 状态 | v1.2.0 | v1.3.0 | 说明 |
|------|------|---------|---------|------|
| 文档主表 (gmp_documents) | ✅ GA | GA | GA | 1696 文档元数据 |
| 内容表 (gmp_document_chunks) | ✅ GA | GA | GA | ~8700 chunks |
| 向量嵌入表 (gmp_embeddings) | ✅ GA | GA | GA | Binary BLOB + HNSW |
| HNSW 索引自动构建 | ✅ **新增** | ❌ 缺失 | ✅ 完整 | `init()` 时自动构建 |
| 图节点存储 (DiskGraphStore) | 🔄 开发中 | GA | 🔄 增强 | WAL 持久化开发中 |
| 图关系边 | ✅ GA | GA | GA | |
| WAL 持久化 | 🔄 开发中 | ❌ | 🔄 增强 | Issue #123 |
| Binary BLOB 存储 | ✅ **新增** | ❌ 分行 | ✅ 完整 | commit `3aeccf0` |

---

## Layer 2: 检索引擎层

| 功能 | 状态 | v1.2.0 | v1.3.0 | 说明 |
|------|------|---------|---------|------|
| RuleChannel (weight=2.0) | ✅ GA | GA | GA | |
| VectorChannel (HNSW) | ✅ GA | GA | GA | Binary BLOB 存储 |
| FtsChannel (BM25) | ✅ GA | GA | GA | |
| GraphChannel (Cypher) | 🔄 开发中 | GA | 🔄 增强 | WAL 依赖 |
| RRF 融合 (k=60) | ✅ GA | GA | GA | |
| 向量索引生成 | 🔄 待开发 | ❌ | 🔄 待开发 | Issue #122 |

---

## Layer 3: Reranker 层

| 功能 | 状态 | v1.2.0 | v1.3.0 | 说明 |
|------|------|---------|---------|------|
| OllamaReranker | ✅ GA | GA | GA | cosine similarity |
| NoopReranker | ✅ GA | GA | GA | |
| BGE-Reranker-v2-m3 | 🔄 待集成 | 🔄 | 🔄 待集成 | |

---

## Layer 4: LLM 层

| 功能 | 状态 | v1.2.0 | v1.3.0 | 说明 |
|------|------|---------|---------|------|
| Ollama LLM Client | ✅ GA | GA | GA | |
| Intent Classification (7种) | ✅ GA | GA | GA | |
| RAG 生成 | ✅ GA | GA | GA | |

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
| 21-docs 文档体系 | 🔄 Beta | 🔄 | 🔄 进行中 | VERSION_DOCS_SPEC |

---

## P0 功能开发进度

| 功能 | Issue | 分支 | 状态 |
|------|-------|------|------|
| 向量索引生成流水线 | #122 | `feature/issue-122-vector-embeddings` | 🔄 开发中 |
| DiskGraphStore WAL 持久化 | #123 | `feature/issue-123-disk-graph-wal` | 🔄 开发中 |
| Layer 0 术语库 | #124 | 待创建 | 🔄 规划中 |
| Layer 0 NER 实体识别 | — | — | 🔄 规划中 |
| Layer 0 关系抽取 | — | — | 🔄 规划中 |
| Layer 0 多维分类 | — | — | 🔄 规划中 |
| Layer 0 相关性预评分 | — | — | 🔄 规划中 |
