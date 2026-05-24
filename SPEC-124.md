# SPEC-124: GMP 合规性审核检索系统 — 实现计划

## 状态总结 (2026-05-25)

### FTS 健康检查结果

| 测试环境 | 结果 | 说明 |
|---------|------|------|
| v1.3.0 单元测试 | ✅ 4/4 通过 | FTS 基本功能正常 |
| v3_report 408 场景 | ✅ 95.3% (389/408) | FTS 在实际场景中工作正常 |
| v2_report 408 场景 | ❌ 0% (0/408) | **旧版本代码/数据问题，非 FTS 本身问题** |

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
1. [ ] 完成 `find_by_doc_id()` — 直接从 `gmp_document_chunks` 表查询
2. [ ] 完成 `find_by_code()` — 从 `gmp_documents` 表通过 code 列查找
3. [ ] 完成 `find_by_chapter()` — 从 `gmp_documents` 表通过 chapter 列查找
4. [ ] 实现 `extract_entities()` — 用 jieba 分词 + 正则匹配 GMP 术语
5. [ ] 单元测试

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
1. [ ] 实现 `search_fts_with_scores()` → FTS 通道
2. [ ] 实现 `search_like_with_extra()` → LIKE 通道
3. [ ] 实现 `search_vectors()` → 向量通道
4. [ ] 实现 `rrf_fuse()` — RRF 融合算法
5. [ ] 集成测试
6. [ ] 408 场景测试

---

## 3. 验收标准

### 3.1 Layer 0 验收

| 测试用例 | 预期 | 状态 |
|---------|------|------|
| `find_by_doc_id("GMP-03-REC-001")` | 返回该 doc 的所有 chunk | TODO |
| `find_by_code("偏差调查")` | 返回 code 包含"偏差调查"的 chunk | TODO |
| `find_by_chapter("CH03-机构与人员")` | 返回该 chapter 的所有 chunk | TODO |
| `extract_entities("灭菌柜压力异常")` | 返回 `[{term: "灭菌柜", type: "设备"}, ...]` | TODO |

### 3.2 Layer 4 RRF 验收

| 测试用例 | 预期 | 状态 |
|---------|------|------|
| RRF 返回结果数量 | = top_k | TODO |
| RRF 评分合理性 | FTS 高分 doc 排名靠前 | TODO |
| 408 场景通过率 | >= 95% | TODO |

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