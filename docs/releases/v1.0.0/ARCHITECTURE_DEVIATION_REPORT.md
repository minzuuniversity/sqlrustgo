# GMP-Platform v1.0.0 偏离分析报告

> 日期: 2026-05-21（更新）
> 作者: AI Agent (via GOVERNANCE_STANDARD.md §2 OpenSpec)
> 目的: 架构偏离审查 + Issue #30 目标评估

---

## 一、架构符合度分析

### 1.1 四层架构实现状态（2026-05-21 更新）

| Layer | 目标组件 | 实现状态 | 代码行数 | 偏离 |
|-------|---------|---------|---------|------|
| Layer 1 | SQLRustGo storage/vector/graph | ✅ storage+vector 已实现 | gmp-storage: 525行 | Graph 无独立 crate |
| Layer 2 | 四通道 (Rule/Vector/FTS/Graph) | ✅ 全部有实现 | channel.rs: 410行 | Graph 非 Cypher BFS |
| Layer 3 | RRF + Reranker | ✅ 全部已实现 | fusion.rs + reranker | — |
| Layer 4 | LLM (Ollama + Intent) | ✅ 框架完整 | gmp-llm: 204行 | 需 Ollama 运行时验证 |

**更新说明**: 
- ✅ Reranker 已实现（PR #36）
- ✅ Vector Channel 已修复（PR #37）
- ⚠️ Graph Channel 有实现但非 Cypher BFS

---

### 1.2 已修复的 P0 偏离

#### 修复 #1: Reranker 未实现 ✅ 已修复（PR #36）

- **修复内容**: 实现 `gmp-reranker/src/lib.rs`
- **实现方案**: OllamaReranker（cosine similarity via embeddings）+ NoopReranker fallback
- **CLI 集成**: `Search --rerank` 标志启用
- **PR**: #36 已合并到 main + develop/v1.0.0

#### 修复 #2: Vector Channel 返回空 ✅ 已修复（PR #37）

- **修复内容**: 添加 `gmp_embeddings` 表 + `search_vectors()` + 连接 VectorChannel
- **实现方案**: SQLite BLOB 存储向量 + cosine similarity 搜索
- **PR**: #37 已合并到 main + develop/v1.0.0

---

### 1.3 需要说明的偏离项

#### 说明 #3: Graph Channel 实现方式不同

**设计目标**: Cypher + BFS 邻居扩展，≥700 节点 + ≥1500 边

**实际情况**:
```rust
// channel.rs:273-310
pub fn search(&self, query: &str, top_k: usize) -> Vec<ScoredChunk> {
    let entities = extract_entities(query);
    let mut results = Vec::new();
    for rule in &self.rules {
        let mut score = 0.0;
        // 基于 KnowledgeRule 的关键词匹配
        if rule.category.to_lowercase().contains(&query_lower) {
            score += 2.0;
        }
        // 实体匹配
        for entity in &entities {
            if rule.keywords.iter().any(|kw| kw.to_lowercase().contains(entity))
                || rule.description.to_lowercase().contains(entity)
            {
                score += 1.0;
            }
        }
        // ...
    }
}
```

**评估**: Graph Channel **有实际实现**，但使用规则匹配而非 Cypher 图查询。这是简化实现，提供了有用的图谱感知检索功能。

**是否需要修复**: 不需要 P0 处理。实现有实际价值。

---

## 二、Issues #16-20 分析

### 2.1 Issues #16-17: 验收条件全未实现

| Issue | 验收条件 | 实现状态 |
|-------|---------|---------|
| #16 增量更新 | `[ ]` 单文档追加不重建全库 | ❌ 无实现证据 |
| #16 增量更新 | `[ ]` 增量向量更新可用 | ❌ 无实现证据 |
| #17 性能压测 | `[ ]` `gmp-retrieve bench` 输出统计 | ❌ 无实现证据 |
| #17 性能压测 | `[ ]` 稳定性通过 50 轮 | ❌ 无实现证据 |

**问题**: Issues #16, #17 验收条件全部为 `[ ]`（未勾选），但 issue 状态为 `closed`。这违反了 Governance §3.2 的证据标准。

**建议**: Issues #16, #17 应重新打开。

### 2.2 Issues #18-20: 错误关联到 v1.0.0

| Issue | 标注 Milestone | 实际情况 |
|-------|---------------|---------|
| #18 中国 GMP 条文补充 | v1.1.0 | ❌ 不是 v1.0.0 范围 |
| #19 设备手册数据集成 | v1.1.0 | ❌ 不是 v1.0.0 范围 |
| #20 Web API 模式 | v1.1.0 | ❌ 不是 v1.0.0 范围 |

**问题**: Issues #18-20 的 Milestone 明确标注 v1.1.0，但被错误地标记为 v1.0.0 GA 的完成项。

**建议**: Issues #18-20 不应计入 v1.0.0 完成状态。

---

## 三、Issue #30 目标评估

### 3.1 原始定义 vs 实际

| Issue #30 定义 | 实际情况 |
|---------------|---------|
| 四层架构完整 | ⚠️ Layer 1-3 核心完成，Graph 实现方式不同 |
| Issue #8-20 全部完成 | ❌ Issues #16-17 无实现证据，#18-20 不属于 v1.0.0 |
| 代码可运行 | ✅ 编译通过 |

### 3.2 v1.0.0 真实完成度

| 类别 | 实际完成项 |
|------|-----------|
| Layer 1 | storage, vector storage |
| Layer 2 | Rule, Vector, FTS, Graph 四个通道 |
| Layer 3 | RRF fusion + Reranker |
| Layer 4 | LLM 框架 |
| CLI | 可运行的检索 CLI |
| Issues #8-15 | ✅ 全部有实现 |
| Issues #16-17 | ❌ 无实现证据 |
| Issues #18-20 | ⚠️ v1.1.0 范围 |

---

## 四、建议

### 4.1 Issues #16-17 应重新打开

Issues #16, #17 没有实现证据但被关闭，违反 Governance §3.2。建议 Human Architect 决策是否实际需要这些功能。

### 4.2 Issues #18-20 不应计入 v1.0.0

这三个 Issue 的 Milestone 明确是 v1.1.0，不应计入 v1.0.0 GA 完成状态。

### 4.3 偏离报告更新状态

本报告已更新：
- ✅ Reranker P0 已修复（PR #36）
- ✅ Vector Channel P0 已修复（PR #37）
- ✅ Graph Channel 确认有实际实现（非空）
- ⚠️ Issues #16-17 无实现证据
- ⚠️ Issues #18-20 不属于 v1.0.0

---

## 五、历史记录

| 日期 | 动作 | PR/Issue |
|------|------|----------|
| 2026-05-21 | Reranker 实现 | PR #36 |
| 2026-05-21 | Vector Channel 修复 | PR #37 |
| 2026-05-21 | 架构偏离初次报告 | Issue #34 |
| 2026-05-21 | 本次更新 | — |
