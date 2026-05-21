# GMP-Platform v1.0.0 偏离分析报告

> 日期: 2026-05-21
> 作者: AI Agent (via GOVERNANCE_STANDARD.md §2 OpenSpec)
> 目的: 架构偏离审查 + Issue #30 目标评估

---

## 一、架构符合度分析

### 1.1 四层架构实现状态

| Layer | 目标组件 | 实现状态 | 代码行数 | 偏离 |
|-------|---------|---------|---------|------|
| Layer 1 | SQLRustGo storage/vector/graph | ⚠️ 部分 | gmp-storage: 382行 | HNSW/Graph 未落地 |
| Layer 2 | 四通道 (Rule/Vector/FTS/Graph) | ⚠️ 部分 | channel.rs: 405行 | Vector 返回空，Graph 逻辑存疑 |
| Layer 3 | RRF + Reranker | ⚠️ RRF可，Reranker未接入 | fusion.rs: 完整 | **Reranker 未实现** |
| Layer 4 | LLM (Ollama + Intent) | ✅ 框架完整 | gmp-llm: 204行 | 需 Ollama 运行时验证 |

### 1.2 具体偏离项

#### 偏离 #1: Reranker 未接入 ⛔ 严重

**设计目标**: BGE-Reranker-v2-m3 Cross-Encoder 重排
**实际情况**:
- `gmp-reranker/src/lib.rs` 是**空文件** (0行)
- `rerank: _` 在 CLI 中未使用（占位符）
- `bge-reranker-v2-m3` 未在任何检索流程中被调用

```rust
// gmp-cli main.rs:177
rerank: _,  // ← 未传递给任何函数
```

**影响**: Layer 3 不完整，RRF 输出结果未经重排，答案质量下降

#### 偏离 #2: Vector Channel 返回空 ⛔ 严重

**设计目标**: `recall@20 ≥ 70%`
**实际情况**:
```rust
// channel.rs:175-188
pub async fn search(&self, query: &str, top_k: usize) -> Vec<ScoredChunk> {
    match self.get_embedding(query).await {
        Ok(embedding) => {
            let _ = embedding;
            vec![]  // ← 永远返回空！HNSW 查询未实现
        }
        Err(_) => vec![],
    }
}
```

**影响**: 向量通道永远返回空结果，四通道融合退化为三通道

#### 偏离 #3: Graph Channel 逻辑存疑 ⚠️ 中等

**设计目标**: Cypher + BFS 邻居扩展，≥700 节点 + ≥1500 边
**实际情况**:
```rust
// channel.rs:281-300
pub fn search(&self, query: &str, top_k: usize) -> Vec<ScoredChunk> {
    let entities = extract_entities(query);
    let mut results = vec![];
    // BFS 扩展逻辑存在，但使用 gmp_storage::StorageEngine
    // 需要验证 StorageEngine 是否真正支持 Graph 查询
}
```

**风险**: 图谱查询依赖 gmp_storage，但 gmp-storage 未实现 graph crate

#### 偏离 #4: FtsChannel 实现存疑 ⚠️ 中等

**设计目标**: BM25 recall@20 ≥ 50%
**实际情况**:
```rust
// channel.rs:252-280
pub fn search(&self, query: &str, top_k: usize) -> Vec<ScoredChunk> {
    let config = gmp_storage::StorageConfig {
        data_dir: "./data".to_string(),
        db_path: db,
    };
    let storage = gmp_storage::StorageEngine::new(config)?;
    // BM25 评分查询存在
    // 但需要验证 SQLRustGo FTS 是否正确集成
}
```

---

## 二、Issue #8-20 与原始目标的偏离

### 2.1 Issue 映射表（设计 vs 实现）

| Issue | 设计目标 | 实现状态 | 偏离程度 |
|-------|---------|---------|---------|
| #8 M1 | 项目脚手架 | ✅ Cargo workspace 完整 | 无偏离 |
| #9 M2 | 数据导入 + 索引基础设施 | ⚠️ 导入框架有，HNSW/Graph 存储未实现 | 中等偏离 |
| #10 M3 | 四通道检索引擎 | ⚠️ 通道框架有，向量/图谱实际返回空/存疑 | 严重偏离 |
| #11 M4 | RRF + Reranker | ⚠️ RRF 完整，Reranker 未实现 | 严重偏离 |
| #12 M4 | LLM 问答 + 意图分类 | ✅ gmp-llm 完整，框架可 | 无严重偏离 |
| #13 M3 | CLI 全命令 | ✅ index/search/evaluate/shell 完整 | 无偏离 |
| #14 M5 | 391 场景测试报告 | ⚠️ 框架有，未实际运行 | 待验证 |
| #15 M5 | 用户手册 + 部署文档 | ✅ 文档完整 | 无偏离 |
| #16 P1 | 索引增量更新 | ⚠️ Issue 已关闭但代码未验证 | 存疑 |
| #17 P1 | 性能压测 | ⚠️ Issue 已关闭但代码未验证 | 存疑 |
| #18 P2 | 中国 GMP 条文 | ⚠️ Issue 已关闭但代码未验证 | 存疑 |
| #19 P2 | 设备手册集成 | ⚠️ Issue 已关闭但代码未验证 | 存疑 |
| #20 P2 | Web API | ⚠️ Issue 已关闭但代码未验证 | 存疑 |

### 2.2 Issue 关闭标准分析

**问题**: Issues #16-20 在没有代码实现证据的情况下被关闭

根据 GOVERNANCE_STANDARD.md §2.3 禁止模式:
```
❌ 禁止: 在没有 PR 证据的情况下关闭 Issue
```

但这些 Issue 的 PR #26 合并时没有明确列出 #16-20 的具体代码变更。

---

## 三、Issue #30 目标评估

### 3.1 Issue #30 原始目标 vs 实际

| Issue #30 定义目标 | 实际实现 | 差距 |
|-------------------|---------|------|
| Layer 2 四通道完整 | ⚠️ 缺 HNSW/Graph 实际查询 | **未完成** |
| Layer 3 Reranker | ⛔ 未实现 | **未完成** |
| Layer 4 LLM | ✅ 框架完整 | 基本完成 |
| 391 场景 ≥ 80% | ⚠️ 未实际运行 | **未验证** |
| 单二进制 | ✅ cargo build 成功 | 完成 |

### 3.2 根本原因

**偏离的根源**: Issue #30 定义时假设 PR #26 包含四通道 + RRF + 增量更新 + 性能 + Web API 的完整实现，但实际上 PR #26 的 commit message 只说"四通道检索引擎 + RRF 融合"，没有 Reranker/HNSW/Graph 的具体实现代码。

**误判**: Issues #16-20 被关闭，但代码中没有对应的实际实现（通过代码搜索验证：gmp-reranker 是空文件，向量通道返回空）。

---

## 四、调整建议

### 4.1 Issue 重新打开（高优先级）

| Issue | 当前状态 | 建议 | 理由 |
|-------|---------|------|------|
| #10 | closed | **重新打开** | Vector 返回空，Graph 存疑 |
| #11 | closed | **重新打开** | Reranker 未实现 |
| #16-20 | closed | **验证后重新打开** | 无实现证据 |

### 4.2 修复计划

#### 紧急修复（v1.0.1 PATCH）

1. **修复 Vector Channel**: 实现 HNSW 向量查询，连接到 SQLRustGo vector crate
2. **接入 Reranker**: 实现 `gmp-reranker`，在 search 命令中加入 `--rerank` 逻辑
3. **验证 Graph Channel**: 确认 gmp_storage 是否真正支持 graph 查询

#### 短期修复（v1.1.0 MINOR）

4. 索引增量更新实际实现
5. 性能压测实际运行
6. Web API 实际实现

### 4.3 Issue #30 调整建议

**建议**: 将 Issue #30 拆分为两个 Issue:
- **Issue #30**: Layer 2/3 核心检索引擎修复（当前阻塞）
- **Issue #21**: Layer 4 LLM + CLI 完整实现（已基本完成）

---

## 五、Governance 合规检查

### 5.1 六大原则检查

| 原则 | 状态 | 说明 |
|------|------|------|
| 1. SSOT | ✅ | 规范定义清晰 |
| 2. 门禁强制 | ⚠️ | B-Gate B9 未实际验证 |
| 3. Issue 闭环 | ❌ | Issues #10-11, #16-20 误关闭 |
| 4. 版本延续 | ⚠️ | 未完成任务未正确映射 |
| 5. 证据驱动 | ❌ | 缺少代码实现证据 |
| 6. OpenSpec | ✅ | 本报告遵循 OpenSpec |

### 5.2 禁止模式检查

| 禁止 | 状态 |
|------|------|
| 无 SPEC 开发 | ✅ 未违反（本次是分析任务）|
| 无分析设计 | ✅ 未违反 |
| 无 PR 证据关闭 Issue | ❌ **违反**: #16-20 无 PR 证据即关闭 |

---

## 六、结论

### 6.1 开发是否偏离？**是，严重偏离**

- **Reranker 未实现**: `gmp-reranker` 是空文件
- **向量通道返回空**: 实际四通道退化为三通道
- **Graph 通道存疑**: 依赖未验证的 storage graph 功能
- **Issues #16-20 误关闭**: 无实现证据

### 6.2 Issue #30 是否符合原目标？**不符合**

原始目标定义过于乐观，将"代码框架"等同于"功能实现"，导致:
- Issue #30 body 声称"Layer 2/3/4 全部完成"
- 实际上 Layer 3 缺少 Reranker，Layer 2 缺向量和图谱

### 6.3 调整建议优先级

| 优先级 | 动作 |
|--------|------|
| **P0** | 修复 Reranker 接入 + Vector Channel HNSW |
| **P0** | 重新打开 Issues #10, #11 |
| **P1** | 验证 Graph Channel 实际功能 |
| **P1** | 验证/重新打开 #16-20 |
| **P2** | 更新 Issue #30 状态描述 |

---

## 附录: 验证命令

```bash
# 验证 Vector 通道
cargo run -- search "偏差分级" --channel vector
# 预期: 返回空（当前行为）→ 应返回结果

# 验证 Reranker
cargo run -- search "验证要求" --rerank
# 预期: 结果未重排（Reranker 空实现）

# 检查 gmp-reranker
cat crates/gmp-reranker/src/lib.rs
# 预期: 空文件（已确认）

# 检查 Vector search 实现
grep -n "HNSW\|hnsw\|vector" crates/gmp-storage/src/*.rs
# 预期: 无结果（vector crate 未使用）
```
