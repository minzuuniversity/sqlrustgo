# SQLRustGo 增量补全开发计划

> 目标: 将 SQLRustGo 补齐为 GMP 合规内审检索系统的统一数据底座
> 发给: Hermes Agent | 日期: 2026-05-20
> 仓库: https://gitcode.com/BreavHeart/sqlrustgo | 当前版本: develop/v3.2.0 RC

---

## 0. 现状总结

SQLRustGo 已具备 RDBMS + 向量检索 + 图数据库的完整内核，但缺失以下 5 项关键能力：

| # | 模块 | 当前状态 | 目标 |
|---|------|---------|------|
| 1 | FTS BM25 | 只有布尔AND倒排索引（无序 doc_id 集合） | BM25 评分排序 Top-K |
| 2 | RRF 融合 | 加权线性求和（`vector*0.4 + graph*0.3 + text*0.3`） | 标准 Reciprocal Rank Fusion（k=60） |
| 3 | Reranker 集成 | 不存在 | BGE-Reranker-v2-m3 跨编码器重排 |
| 4 | LLM Chat | 只有 Embedding（Ollama embedding） | Ollama Chat Completion 答案生成 |
| 5 | 规则召回权重 | `gmp::scenarios` 定义了场景但未嵌入检索管道 | 规则通道在 RRF 中高优先级 boost |

---

## 1. 增强一: FTS 增加 BM25 评分（P0）

### 1.1 目标
替换当前 `storage/src/fts/inverted_index.rs` 的布尔AND搜索，输出按 BM25 分数降序的 Top-K 结果。

### 1.2 修改范围

**文件**: `crates/storage/src/fts/inverted_index.rs`

**新增结构**:

```rust
/// BM25 评分器
pub struct Bm25Scorer {
    /// 词频饱和参数（默认 1.2）
    pub k1: f32,
    /// 文档长度归一化参数（默认 0.75）
    pub b: f32,
    /// 平均文档长度（索引时计算）
    pub avg_dl: f32,
    /// 总文档数
    pub doc_count: u64,
    /// 文档长度映射: doc_id → 词数
    pub doc_lengths: HashMap<u64, usize>,
    /// 文档频率: term → 包含该词的文档数
    pub doc_freq: HashMap<String, u64>,
}

impl Bm25Scorer {
    /// 新建评分器
    pub fn new(k1: f32, b: f32) -> Self;

    /// 添加文档时更新统计（索引阶段调用）
    pub fn add_document(&mut self, doc_id: u64, term_count: usize);

    /// 为单对 (term, doc_id) 计算 BM25 分数
    /// tf = 该词在该文档中出现的次数
    /// doc_len = 该文档的总词数
    pub fn score(&self, term: &str, doc_id: u64, tf: u64, doc_len: usize) -> f32;

    /// 批量搜索：返回 (doc_id, bm25_score) 降序 Top-K
    pub fn search_ranked(&self, query: &str, index: &InvertedIndex, top_k: usize) -> Vec<(u64, f32)>;
}
```

**BM25 公式**:

```
IDF = ln((N - df + 0.5) / (df + 0.5) + 1.0)
TF_norm = ((k1 + 1) * tf) / (tf + k1 * (1 - b + b * dl / avg_dl))
score = IDF * TF_norm
```

### 1.3 接口变更

InvertedIndex 新增方法：

```rust
impl InvertedIndex {
    // 【已有】: search(&self, query: &str) -> Vec<u64>        // 布尔AND
    // 【新増】: search_with_scores(&self, query: &str, top_k: usize) -> Vec<(u64, f32)>
    pub fn search_with_scores(&self, query: &str, scorer: &Bm25Scorer, top_k: usize) -> Vec<(u64, f32)>;
}
```

### 1.4 测试要求

| 测试项 | 说明 | 通过标准 |
|--------|------|----------|
| `test_bm25_idf` | 仅出现在1个文档中的词 vs 出现在所有文档中的词，IDF值应有显著差异 | df=1 时 IDF >> df=N 时 IDF |
| `test_bm25_tf_saturation` | tf=1 vs tf=10，分数增长率递减 | tf=10 时分数增长 < 10× |
| `test_bm25_ranked_order` | "偏差管理" vs "仓库温湿度"，前者分数应更高 | 排序正确 |
| `test_bm25_chinese_tokenizer` | 中文查询 "偏差分级标准" 的正确评分 | 返回相关结果，非空 |
| `test_bm25_vs_boolean_regression` | 确保原布尔接口不受影响 | `search()` 返回与改前一致 |

---

## 2. 增强二: RRF 融合替代加权求和（P0）

### 2.1 目标
将 `qmd-bridge/src/hybrid.rs` 和 `unified-query/src/fusion.rs` 中的加权线性融合替换为标准 Reciprocal Rank Fusion。

### 2.2 修改范围

**文件 1**: `crates/qmd-bridge/src/hybrid.rs`

在现有 `HybridSearchConfig` 后新增 `RrfConfig`：

```rust
/// RRF (Reciprocal Rank Fusion) 配置
#[derive(Debug, Clone)]
pub struct RrfConfig {
    /// RRF 平滑常数（默认 60）
    pub k: f32,
    /// 各通道权重（可选，用于优先通道 boost）
    pub vector_weight: f32,    // 默认 1.0
    pub graph_weight: f32,     // 默认 1.0
    pub text_weight: f32,      // 默认 1.0
    pub rule_weight: f32,      // 默认 2.0（规则命中优先）
}

impl Default for RrfConfig {
    fn default() -> Self {
        Self { k: 60.0, vector_weight: 1.0, graph_weight: 1.0, text_weight: 1.0, rule_weight: 2.0 }
    }
}
```

**新增 RRF 函数**:

```rust
/// Reciprocal Rank Fusion
///
/// score = Σ w_c / (k + rank_c)
/// 其中 w_c 是通道 c 的权重, rank_c 是结果在通道 c 中的排名(从1开始)
pub fn reciprocal_rank_fusion(
    result_sets: &[Vec<(String, f32)>],  // 每个通道的 [(id, original_score)]，已按分数降序
    config: &RrfConfig,
    top_k: usize,
) -> Vec<(String, f32)>;
```

### 2.3 HybridSearchConfig 变更

```rust
// 旧的 weighted scoring 保留向后兼容（标记 deprecated）
// 新增 RRF 模式
pub struct HybridSearchConfig {
    pub vector_weight: f32,   // [deprecated] 仅用于非RRF模式
    pub graph_weight: f32,    // [deprecated]
    pub text_weight: f32,     // [deprecated]
    pub limit: usize,
    pub enable_rerank: bool,
    pub rrf: Option<RrfConfig>,  // 【新增】若设置则使用RRF模式
}
```

### 2.4 同步修改

**文件 2**: `crates/unified-query/src/fusion.rs`

`ResultFusion::fuse()` 同样支持 RRF 模式。

### 2.5 测试要求

| 测试项 | 说明 | 通过标准 |
|--------|------|----------|
| `test_rrf_basic` | 2个通道各有Top-3，验证RRF分数 | 分数=1/(60+rank) 的加权和 |
| `test_rrf_weighted` | 某通道权重×2，该通道结果排名提升 | 加权通道的结果在最终排序中更靠前 |
| `test_rrf_partial_channel` | 某通道无结果（空列表），其他通道正常工作 | 无报错，正确融合剩余通道 |
| `test_rrf_vs_weighted` | 同一输入对比加权和 vs RRF，RRF不依赖原始分数量纲 | RRF结果更稳定（同一结果在不同通道的分数差不同时，加权和排序不稳定） |
| `test_rrf_rule_boost` | 规则通道权重2.0，规则匹配结果应排在第一 | 规则命中的doc排Top-1 |

---

## 3. 增强三: Reranker 跨编码器集成（P0）

### 3.1 目标
新增 `crates/reranker/` crate，提供 BGE-Reranker-v2-m3 集成能力，在 RRF 融合后对 Top-N 结果重排序。

### 3.2 新建文件结构

```
crates/reranker/
  Cargo.toml
  src/
    lib.rs           — Reranker trait + OllamaCrossEncoder
    error.rs         — 错误类型
```

### 3.3 核心接口

```rust
/// 重排序器 trait
#[async_trait]
pub trait Reranker: Send + Sync {
    /// 对 query-document 对进行相关性评分
    /// 返回 (文档ID, 相关性分数) 列表，按分数降序
    async fn rerank(
        &self,
        query: &str,
        documents: &[(String, String)],  // [(doc_id, doc_content)]
        top_k: usize,
    ) -> RerankerResult<Vec<(String, f32)>>;
    
    /// 重排序器名称
    fn name(&self) -> &str;
    
    /// 是否可用
    fn is_available(&self) -> bool;
}

/// Ollama 托管的 Cross-Encoder (BGE-Reranker-v2-m3)
pub struct OllamaReranker {
    base_url: String,
    model: String,
    client: reqwest::Client,
}

impl OllamaReranker {
    pub fn new(base_url: &str, model: &str) -> Self;
}

#[async_trait]
impl Reranker for OllamaReranker {
    async fn rerank(
        &self,
        query: &str,
        documents: &[(String, String)],
        top_k: usize,
    ) -> RerankerResult<Vec<(String, f32)>> {
        // 调用 Ollama API: POST /api/embeddings 两次（query + doc）
        // 计算 cosine_similarity(query_embedding, doc_embedding)
        // 按分数降序排序，截取 top_k
    }
}

/// 本地 CPU fallback（无 Ollama 时的空操作）
pub struct NoopReranker;

#[async_trait]
impl Reranker for NoopReranker {
    async fn rerank(&self, _query: &str, documents: &[(String, String)], top_k: usize) -> ... {
        // 直接返回原排序的 top_k 条
    }
}
```

**Cargo.toml** 依赖:

```toml
[package]
name = "sqlrustgo-reranker"
version = "0.1.0"
edition = "2021"

[dependencies]
async-trait = "0.1"
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }
thiserror = "2.0"
```

### 3.4 集成到 qmd-bridge

在 `qmd-bridge/src/hybrid.rs` 中，当 `enable_rerank=true` 时调用 Reranker：

```rust
impl HybridSearcher {
    pub async fn search_with_rerank(
        &self,
        query: &HybridQuery,
        reranker: &dyn Reranker,
    ) -> HybridResult {
        // Step 1: 向量/文本/图谱 多路检索
        // Step 2: RRF 融合
        // Step 3: Reranker 重排 Top-N
        let candidates: Vec<(String, String)> = fused_results.iter()
            .take(30).map(|r| (r.id.clone(), r.content.clone())).collect();
        let reranked = reranker.rerank(&query.text, &candidates, self.config.limit).await?;
        // ...
    }
}
```

### 3.5 测试要求

| 测试项 | 说明 | 通过标准 |
|--------|------|----------|
| `test_reranker_available` | Ollama bge-reranker-v2-m3 可连通 | `is_available()` 返回 true |
| `test_reranker_rank_improvement` | 查询+候选集 → 重排后相关文档排名提升 | MRR 提升 ≥ 10% |
| `test_reranker_noop_fallback` | 无 Ollama 时 NoopReranker 正常工作 | 不崩溃，返回原排序 |
| `test_reranker_batch` | 一次处理 30 条候选 | < 3s (CPU) 或 < 500ms (GPU) |
| `test_reranker_empty_input` | 空候选集 | 返回空列表，不崩溃 |

---

## 4. 增强四: LLM Chat Completion 集成（P1）

### 4.1 目标
在 `gmp` crate 或新建 `crates/llm/` 提供 Ollama Chat Completion 接口，支持基于检索上下文的答案生成。

### 4.2 新建文件结构

```
crates/gmp/src/
  llm.rs             — LLM Client (Chat + Answer Generation)
```

或如果作为独立 crate：

```
crates/llm/
  Cargo.toml
  src/
    lib.rs           — LlmClient trait + OllamaLlmClient
    prompt.rs        — Prompt 模板管理
    error.rs
```

### 4.3 核心接口

```rust
/// LLM 客户端 trait
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// 发送 Chat Completion 请求
    async fn chat(
        &self,
        messages: &[ChatMessage],
        temperature: f32,
        max_tokens: usize,
    ) -> LlmResult<String>;
    
    /// 基于上下文生成答案（RAG 核心方法）
    async fn generate_answer(
        &self,
        question: &str,
        contexts: &[RetrievedDocument],
        instructions: Option<&str>,
    ) -> LlmResult<GeneratedAnswer>;
}

/// 聊天消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,   // "system" | "user" | "assistant"
    pub content: String,
}

/// 检索到的文档上下文
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievedDocument {
    pub doc_id: String,
    pub title: String,
    pub content: String,  // chunk content
    pub score: f32,
    pub source: String,   // 来源通道: "vector"|"fts"|"graph"|"rule"
}

/// 生成的答案
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedAnswer {
    pub answer: String,
    pub citations: Vec<Citation>,    // 引用来源
    pub confidence: Option<f32>,     // 置信度
    pub needs_human_review: bool,    // 是否需要人工确认
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    pub doc_id: String,
    pub title: String,
    pub snippet: String,
}
```

### 4.4 Ollama 实现

```rust
pub struct OllamaLlmClient {
    base_url: String,
    model: String,
    client: reqwest::Client,
    prompt_template: PromptTemplate,
}

impl OllamaLlmClient {
    /// model: "qwen2.5:14b" 或 "qwen2.5:32b"
    pub fn new(base_url: &str, model: &str) -> Self;
}
```

**Ollama Chat API 调用**:

```rust
// POST http://localhost:11434/api/chat
// Body: {"model": "qwen2.5:14b", "messages": [...], "stream": false, 
//         "options": {"temperature": 0.3, "num_predict": 1024}}
```

### 4.5 RAG Prompt 模板

```rust
impl Default for PromptTemplate {
    fn default() -> Self {
        Self {
            system: r#"你是 GMP 合规审核 AI 助手。请严格基于提供的材料回答问题。
如果材料不足以回答，请明确说明"当前材料无法回答此问题"。
不要编造任何信息。"#.to_string(),
            user_template: r#"## 相关文档
{contexts}

## 问题
{question}

## 回答要求
1. 给出明确答案
2. 在每句话后标注来源文档编号，如 [doc-001]
3. 如有不确定性，在句末标注 [需人工确认]"#.to_string(),
        }
    }
}
```

### 4.6 测试要求

| 测试项 | 说明 | 通过标准 |
|--------|------|----------|
| `test_llm_chat_basic` | "你好" → 获得非空回复 | 返回字符串，非空 |
| `test_llm_generate_answer` | 输入问题 + 3条上下文 → 生成含引用的答案 | 答案包含引用标记 [doc-xxx] |
| `test_llm_context_faithfulness` | 上下文包含 "偏差分级为重大/一般/微小" → 问 "偏差有哪些级别" → 答案应准确 | 答案正确引用上下文 |
| `test_llm_no_context` | 上下文为空 → 问问题 → 返回"无法回答" | `needs_human_review=true`，答案说明无法回答 |
| `test_llm_timeout` | 设置 30s 超时 → 不无限等待 | 超时返回错误，不崩溃 |
| `test_llm_citation_parsing` | 验证 citations 字段正确提取 | 引用数量 ≥ 1，doc_id 有效 |

---

## 5. 增强五: 规则召回优先级权重（P1）

### 5.1 目标
让 `agent-knowledge` YAML 规则匹配结果在 RRF 融合中获得最高优先级，确保确定性规则直接通过。

### 5.2 修改范围

**文件 1**: `crates/qmd-bridge/src/hybrid.rs`

`HybridSearcher` 增加规则通道支持：

```rust
pub struct HybridSearcher {
    config: HybridSearchConfig,
    rrf_config: RrfConfig,        // 【新增】
    rule_matcher: Option<RuleMatcher>,  // 【新增】
}

// 新增方法
impl HybridSearcher {
    pub fn with_rule_matcher(mut self, matcher: RuleMatcher) -> Self;
    pub fn set_rule_boost(mut self, weight: f32) -> Self;
    
    /// 完整搜索管道: 向量 + 文本 + 图谱 + 规则 → RRF → Reranker
    pub async fn search_all(
        &self,
        query: &str,
        vector_results: Vec<SearchResult>,
        text_results: Vec<SearchResult>,
        graph_results: Vec<SearchResult>,
        reranker: Option<&dyn Reranker>,
    ) -> HybridResult;
}
```

### 5.3 RuleMatcher 实现

```rust
/// YAML 规则匹配器
pub struct RuleMatcher {
    rules: HashMap<String, Vec<(String, String)>>,  // category → [(key, value)]
}

impl RuleMatcher {
    /// 从 agent-knowledge/ 目录加载 YAML 文件
    pub fn from_directory(path: &str) -> Result<Self>;
    
    /// 匹配规则：query 关键词 → 返回匹配的规则结果
    pub fn match_rules(&self, query: &str, top_k: usize) -> Vec<SearchResult>;
}
```

### 5.4 规则优先级策略

```
if rule_matcher.has_direct_match(query):
    rule_results.rank = [1, 2, 3, ...]  # 强制排在 RRF 中的前几位
    RRF 中 rule_weight = 2.0  # 加倍权重
else:
    rule_results = []  # 无命中时不影响其他通道
```

### 5.5 测试要求

| 测试项 | 说明 | 通过标准 |
|--------|------|----------|
| `test_rule_direct_match` | "偏差分级标准" → 规则库有直接匹配 | 返回结果，排名第一 |
| `test_rule_partial_match` | "混合机偏差" → 部分关键词匹配规则 | 返回结果，rank ≤ 3 |
| `test_rule_no_match` | "今天天气怎么样" → 无规则匹配 | 返回空，不影响其他通道 |
| `test_rule_boost_in_rrf` | 规则命中文档在 RRF 积分 > 同等排名的向量结果 | 规则结果的 RRF 分数 = 2.0/(60+1) = 0.0328 |
| `test_rule_load_from_yaml` | 从 agent-knowledge/ 加载 → 规则数 ≥ 55 | 解析成功，规则数量正确 |

---

## 6. 开发与集成顺序

```
Day 1-2: 增强一 (BM25)    → cargo test --package sqlrustgo-storage
Day 3-4: 增强二 (RRF)     → cargo test --package sqlrustgo-qmd-bridge
Day 5-6: 增强五 (规则权重) → cargo test --package sqlrustgo-qmd-bridge
Day 7-9: 增强三 (Reranker) → cargo test --package sqlrustgo-reranker
Day 10-11: 增强四 (LLM)   → cargo test --package sqlrustgo-gmp
```

### 依赖关系

```
增强一 (BM25) ──┐
                ├──→ 增强二 (RRF) ──→ 增强三 (Reranker)
增强五 (规则)  ─┘                          │
                                           └──→ 增强四 (LLM)
```

- RRF 依赖 BM25 提供排序后的文本结果
- Reranker 依赖 RRF 提供融合后的候选集
- LLM 依赖 Reranker 提供最终上下文

---

## 7. 全链路集成测试要求

完成全部 5 项增强后，需执行一次端到端集成测试：

| 测试场景 | 输入 | 期望输出 |
|----------|------|----------|
| 端到端检索 | "偏差分级标准" | BM25+向量+规则三通道召回 → RRF融合 → Reranker重排 → 返回Top-10 |
| 规则优先验证 | "发现偏差后应该先登记还是先处理" | 规则通道命中"偏差处理流程"规则 → 排Top-1 |
| LLM答案生成 | "混合机转速异常偏差应该怎么登记" | 检索上下文 → LLM生成带引用的答案 |
| 空结果降级 | "火星GMP规范" | 无匹配 → 返回空列表，不崩溃，LLM返回"无法回答" |
| 并发安全 | 10并发 × 100次查询 | 无竞态条件，无内存泄漏 |

### 全链路测试命令

```bash
# 编译
cargo build --release

# 单元测试
cargo test --all

# 集成测试
cargo test --test gmp_retrieval_pipeline

# 性能测试
cargo bench --bench retrieval_benchmark
```

---

## 8. 给 Hermes 的开发指引

### 8.1 开发分支

```bash
git checkout -b feature/gmp-retrieval-enhancements
```

### 8.2 新增 crate 清单

| Crate | 路径 | 说明 |
|-------|------|------|
| `sqlrustgo-reranker` | `crates/reranker/` | 新建 |

### 8.3 修改 crate 清单

| Crate | 文件 | 说明 |
|-------|------|------|
| `sqlrustgo-storage` | `crates/storage/src/fts/inverted_index.rs` | 增加 BM25 评分 |
| `sqlrustgo-qmd-bridge` | `crates/qmd-bridge/src/hybrid.rs` | 替换 RRF + 规则通道 |
| `sqlrustgo-unified-query` | `crates/unified-query/src/fusion.rs` | 同步 RRF 模式 |
| `sqlrustgo-gmp` | `crates/gmp/src/llm.rs` (新建) | LLM Chat 客户端 |

### 8.4 遵守的代码规范

- 所有公开 API 添加 Rust doc 注释（`///`）
- 新增模块添加 `#![deny(missing_docs)]`
- 每个 `pub fn` 至少覆盖一个 `#[test]`
- 错误处理使用 `thiserror` derive
- Async 方法使用 `async-trait` crate

---

> **预计工作量**: 2 人 × 1 周，或 1 人 × 2 周
> **交付标准**: 全部单元测试通过 + 端到端集成测试通过