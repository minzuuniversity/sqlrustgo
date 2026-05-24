# SPEC-124: GMP 合规性审核检索系统 — 实现计划

## 状态总结 (2026-05-25)

### ✅ 全部完成 — 2026-05-25

| 阶段 | 状态 | PR | 说明 |
|------|------|----|------|
| FTS 健康检查 | ✅ 95.3% (389/408) | — | FTS InvertedIndex 工作正常 |
| Layer 0 精确查找 | ✅ 已完成 | #150, #152 | find_by_doc_id/code/chapter + extract_entities |
| Layer 4 RRF fusion | ✅ 已完成 | #152, #154 | FTS + LIKE + Vector 三通道 RRF |
| 7 项性能改进 | ✅ 已完成 | #141-#149 | LRU缓存/同义词表/LLM/并行/规则外置/BM25/HNSW |

**结论**: FTS InvertedIndex AND 语义是设计正确的中文多token搜索行为。v2_report 的 0% 是旧版本遗留问题，当前代码库 FTS 工作正常。

---

## 1. 问题描述 (已解决)

~~**现象**: FTS search 返回空结果，实际数据库中有匹配的文档。~~

**已确认**: 当前代码库 (`develop/v1.3.0` + `feature/issue-124-layer0-pipeline`) 的 FTS 功能正常，95.3% 场景通过率验证了这一点。

---

## 2. 待实现功能

### Layer 0: exact doc_id/code/chapter 精确查找

**目标**: 实现精确 doc_id/code/chapter 查找，跳过 FTS/向量检索。

**文件**: `crates/gmp-kg/src/storage.rs`

**API 设计**:
```rust
impl KgStorage {
    /// Find chunks by exact doc_id
    pub fn find_by_doc_id(&self, doc_id: &str) -> Result<Vec<ChunkResult>>;

    /// Find chunks by document code (e.g., "GMP-03-REC-001")
    pub fn find_by_code(&self, code: &str) -> Result<Vec<ChunkResult>>;

    /// Find chunks by chapter (e.g., "CH03-机构与人员")
    pub fn find_by_chapter(&self, chapter: &str) -> Result<Vec<ChunkResult>>;

    /// Extract entities from query text
    pub fn extract_entities(&self, text: &str) -> Result<Vec<ExtractedEntity>>;
}
```

**数据源**: `gmp_terms` + `extraction_rules` 表（见 seed_terms.rs 已有）

**实现步骤**:
1. [x] 完成 `find_by_doc_id()` — StorageEngine 中实现，基于 chunk_cache
2. [x] 完成 `find_by_code()` — 部分匹配 doc_id 或 title
3. [x] 完成 `find_by_chapter()` — 部分匹配 chapter 字段
4. [x] 实现 `extract_entities()` — 基于 600+ GMP seed_terms 匹配
5. [x] 单元测试 — 50+6+7=63 全部通过

### Layer 4: RRF fusion (FTS + LIKE + 向量)

**目标**: 实现 RRF (Reciprocal Rank Fusion) 融合三通道检索结果。

**文件**: `crates/gmp-retrieval/src/channel.rs` 或 `crates/gmp-storage/src/storage.rs`

**API 设计**:
```rust
pub struct FusionConfig {
    pub fts_weight: f32,      // default 0.4
    pub like_weight: f32,    // default 0.3
    pub vector_weight: f32,  // default 0.3
    pub rrf_k: usize,         // default 60
}

impl StorageEngine {
    /// Hybrid search with RRF fusion
    pub fn hybrid_search_rrf(
        &self,
        query: &str,
        top_k: usize,
        config: FusionConfig,
    ) -> Result<Vec<SearchResult>>;
}
```

**RRF 算法**:
```
score(d) = Σ 1 / (k + rank(d))
where rank(d) is the position of document d in each channel's result list
```

**实现步骤**:
1. [x] 实现 `search_fts_with_scores()` → FTS 通道 (已有)
2. [x] 实现 `search_like_with_extra()` → LIKE 通道 (已有)
3. [x] 实现 `search_vectors()` → 向量通道 (已有，Ollama embeddings)
4. [x] 实现 `hybrid_search_rrf()` — RRF 融合算法
5. [x] 集成测试 — 63 passed
6. [x] 408 场景测试 — 95.3% (v3_report)

---

## 3. 验收标准

### 3.1 Layer 0 验收

| 测试用例 | 预期 | 状态 |
|---------|------|------|
| `find_by_doc_id("GMP-03-REC-001")` | 返回该 doc 的所有 chunk | ✅ PR #150 |
| `find_by_code("偏差调查")` | 返回 code 包含"偏差调查"的 chunk | ✅ PR #150 |
| `find_by_chapter("CH03-机构与人员")` | 返回该 chapter 的所有 chunk | ✅ PR #150 |
| `extract_entities("灭菌柜压力异常")` | 返回 `[{term: "灭菌柜", type: "设备"}, ...]` | ✅ PR #150 |

### 3.2 Layer 4 RRF 验收

| 测试用例 | 预期 | 状态 |
|---------|------|------|
| RRF 返回结果数量 | = top_k | ✅ PR #152, #154 |
| RRF 评分合理性 | FTS+LIKE+Vector 融合排名 | ✅ PR #154 |
| 408 场景通过率 | >= 95% | ✅ 95.3% |

---

## 4. 相关文件

| 文件 | 作用 |
|------|------|
| `crates/gmp-kg/src/storage.rs` | KgStorage 实现 Layer 0 |
| `crates/gmp-kg/src/seed_terms.rs` | 600+ GMP 术语数据 |
| `crates/gmp-kg/src/types.rs` | 类型定义 |
| `crates/gmp-storage/src/storage.rs` | StorageEngine Layer 4 RRF |
| `crates/gmp-eval/src/main.rs` | 408 场景评测 |

---

## 5. 修改记录

| 日期 | 修改人 | 修改内容 |
|------|--------|---------|
| 2026-05-24 | AI Agent | 初始分析并生成 SPEC-124.md |
| 2026-05-25 | AI Agent | 更新：确认 FTS 正常，聚焦 Layer 0/4 实现 |
| 2026-05-25 | AI Agent | **完成**: Layer 0 (PR #150) + RRF (PR #152,#154) + 7项性能改进 (PR #141-#149) |