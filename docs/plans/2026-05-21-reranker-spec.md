# SPEC: gmp-reranker 实现 + CLI 集成

## 现状分析

### 现有代码结构

| 文件 | 内容 | 问题 |
|------|------|------|
| `crates/gmp-reranker/src/lib.rs` | **空文件** | ❌ 无任何实现 |
| `crates/gmp-core/src/types.rs` | `ScoredChunk { chunk_id, doc_id, content, score, source }` | 已有数据结构 |
| `crates/gmp-retrieval/src/fusion.rs` | `RrfFusion::fuse()` → `Vec<ScoredChunk>` | 融合后无 rerank |
| `crates/gmp-retrieval/src/channel.rs` | `FourChannelEngine::search()` 异步，调用 fusion.fuse() | 无 rerank 集成点 |
| `crates/gmp-cli/src/main.rs` | `Search { query, channel, rerank: _, explain, db }` | `rerank: _` 未传递 |

### Ollama Reranker API 分析

Ollama 的 reranker 模型（bge-reranker-v2-m3）通过 `/api/embeddings` 接口工作：
- 输入: query + document texts
- 输出: cosine similarity scores
- 模式: Cross-Encoder（query-document 对联合编码）

**已知参考**: `docs/design/sqlrustgo-incremental-enhancement-plan.md` §Reranker 定义了 trait 接口。

### 关键依赖

- `async-trait = "0.1"` — async fn in traits（需要添加到 gmp-reranker/Cargo.toml）
- `reqwest` — HTTP 客户端（已通过 workspace 传递）
- `gmp-core` — ScoredChunk 类型

## 需求分析

### 功能边界

1. **Reranker Trait**: 定义 `rerank(query, candidates, top_k)` async 接口
2. **OllamaReranker**: 调用 Ollama embeddings API 计算 cosine similarity，重排结果
3. **NoopReranker**: 无 Ollama 时的 fallback，直接返回原排序
4. **CLI 集成**: `--rerank` 参数传递给检索引擎，启用重排
5. **FourChannelEngine 集成**: fusion 后调用 reranker（条件性）

### 验收标准

| 条件 | 验证方法 |
|------|---------|
| `gmp-reranker` 编译通过 | `cargo build -p gmp-reranker` |
| `Reranker` trait 实现了 `rerank()` | `cargo test -p gmp-reranker` |
| OllamaReranker 可用时正确重排 | 需要 Ollama + bge-reranker-v2-m3 |
| `--rerank` 参数生效 | `cargo run -- search "偏差" --rerank` 输出变化 |
| 无 Ollama 时不崩溃 | 不启动 Ollama 时 search 正常返回 |

## 实现方案

### 1. gmp-reranker/src/lib.rs

```rust
use async_trait::async_trait;
use gmp_core::ScoredChunk;
use reqwest::Client;
use std::sync::Arc;

/// Reranker trait - re-rank candidates using a cross-encoder model
#[async_trait]
pub trait Reranker: Send + Sync {
    async fn rerank(
        &self,
        query: &str,
        candidates: Vec<ScoredChunk>,
        top_k: usize,
    ) -> anyhow::Result<Vec<ScoredChunk>>;

    fn name(&self) -> &str;
    fn is_available(&self) -> bool;
}

/// Ollama-backed reranker using bge-reranker-v2-m3
pub struct OllamaReranker {
    client: Client,
    base_url: String,
    model: String,
}

impl OllamaReranker {
    pub fn new(base_url: String, model: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
            model,
        }
    }

    async fn score(&self, query: &str, doc: &str) -> anyhow::Result<f32> {
        // Get both embeddings via Ollama /api/embeddings
        // Compute cosine similarity
        // Return score
    }
}

#[async_trait]
impl Reranker for OllamaReranker {
    async fn rerank(
        &self,
        query: &str,
        candidates: Vec<ScoredChunk>,
        top_k: usize,
    ) -> anyhow::Result<Vec<ScoredChunk>> {
        if candidates.is_empty() {
            return Ok(candidates);
        }

        // Score all candidates
        let mut scored = Vec::new();
        for chunk in candidates {
            match self.score(query, &chunk.content).await {
                Ok(s) => scored.push((s, chunk)),
                Err(_) => {
                    // Fallback: keep original score
                    scored.push((chunk.score, chunk));
                }
            }
        }

        // Sort by new score descending
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // Return top_k
        Ok(scored.into_iter().take(top_k).map(|(_, c)| c).collect())
    }

    fn name(&self) -> &str { "ollama-bge-reranker-v2-m3" }
    fn is_available(&self) -> bool { true }
}

/// No-op reranker for when Ollama is unavailable
pub struct NoopReranker;

#[async_trait]
impl Reranker for NoopReranker {
    async fn rerank(
        &self,
        _query: &str,
        candidates: Vec<ScoredChunk>,
        top_k: usize,
    ) -> anyhow::Result<Vec<ScoredChunk>> {
        Ok(candidates.into_iter().take(top_k).collect())
    }
    fn name(&self) -> &str { "noop" }
    fn is_available(&self) -> bool { true }
}

/// Create reranker based on availability
pub fn create_reranker(ollama_url: &str, model: &str) -> Arc<dyn Reranker> {
    if reqwest::Client::new()
        .get(format!("{}/api/tags", ollama_url))
        .send()
        .is_ok()
    {
        Arc::new(OllamaReranker::new(ollama_url.to_string(), model.to_string()))
    } else {
        Arc::new(NoopReranker)
    }
}
```

### 2. gmp-retrieval/src/engine.rs

修改 `RetrievalEngine.search()` 添加 rerank 参数：

```rust
pub struct RetrievalEngine {
    config: RetrievalEngineConfig,
    four_channel: FourChannelEngine,
    reranker: Option<Arc<dyn Reranker>>,  // NEW
}

impl RetrievalEngine {
    pub fn new(config: RetrievalEngineConfig, reranker: Option<Arc<dyn Reranker>>) -> Self {
        Self { config, four_channel: FourChannelEngine::new(...), reranker }
    }

    pub async fn search(&self, query: &str, channel: &str, rerank: bool, top_k: usize) -> Vec<ScoredChunk> {
        let results = self.four_channel.search(query, top_k).await;

        if rerank {
            if let Some(ref r) = self.reranker {
                if r.is_available() {
                    return r.rerank(query, results, top_k).await.unwrap_or(results);
                }
            }
        }
        results
    }
}
```

### 3. CLI main.rs

```rust
Commands::Search { query, channel, rerank, explain, db } => {
    let reranker = gmp_reranker::create_reranker(&ollama_url, &reranker_model);
    let engine = RetrievalEngine::new(config, Some(reranker));
    let results = engine.search(&query, &channel, rerank, 20).await;
    // ...
}
```

### 4. gmp-reranker/Cargo.toml

```toml
[package]
name = "gmp-reranker"
version = "1.0.0"
edition = "2021"

[dependencies]
gmp-core = { path = "../gmp-core" }
async-trait = "0.1"
reqwest = { workspace = true }
anyhow = { workspace = true }
tokio = { workspace = true }
serde = { workspace = true }
```

## 工作量估算

| 任务 | 文件 | 复杂度 |
|------|------|--------|
| 实现 Reranker trait + OllamaReranker + NoopReranker | `gmp-reranker/src/lib.rs` | 中 |
| 添加 async-trait 依赖 | `gmp-reranker/Cargo.toml` | 低 |
| 修改 RetrievalEngine 添加 reranker 字段和逻辑 | `gmp-retrieval/src/engine.rs` | 低 |
| 修改 CLI 传递 rerank 参数 | `gmp-cli/src/main.rs` | 低 |

**总计**: 4 个文件，约 200-250 行代码

## 风险评估

| 风险 | 影响 | 缓解 |
|------|------|------|
| Ollama reranker API 格式不确定 | 中 | 使用 embedding cosine similarity 作为 fallback |
| async_trait 兼容性问题 | 低 | edition 2021 + rust 1.80+ 支持 |
| Reranker 调用慢（CPU） | 低 | top_k 限制为 20 |
