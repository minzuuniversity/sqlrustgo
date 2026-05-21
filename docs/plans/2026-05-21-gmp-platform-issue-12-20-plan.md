# GMP-Platform Issue 12-20 Implementation Plan

## Status Summary

| Issue | Title | Status | Work Needed |
|-------|-------|--------|-------------|
| #10 | 四通道检索引擎 | ✅ Merged | - |
| #11 | RRF 融合 + Reranker | ✅ Merged | - |
| #12 | LLM 问答 + 意图分类器 | 🔄 Partial | Chat CLI cmd |
| #13 | CLI 全命令 | 🔄 Partial | Shell/Eval/Bench full impl |
| #14 | 391 场景测试 | 🔄 None | Test corpus + eval runner |
| #15 | 用户手册 + 部署指南 | 🔄 None | Markdown docs |
| #16 | 索引增量更新 | 🔄 None | Storage incremental API |
| #17 | 性能压测 + 基准报告 | 🔄 None | Bench full impl |
| #18 | 中国GMP条文补充 | 🔄 None | YAML KB expansion |
| #19 | 设备手册数据集成 | 🔄 None | Import pipeline |
| #20 | Web API RESTful | 🔄 None | Actix-web HTTP server |

## Component Analysis

### Existing Crates (1825 LOC total)
- `gmp-core` (175 LOC): error.rs, types.rs — DONE
- `gmp-storage` (377 LOC): storage.rs, document.rs — partial, needs incremental update
- `gmp-retrieval` (457 LOC): channel.rs, engine.rs, fusion.rs — DONE (4-channel + RRF)
- `gmp-reranker` (1 LOC): EMPTY — needs reranker impl
- `gmp-llm` (204 LOC): chat, intent classification, route_query, generate_answer — DONE
- `gmp-cli` (567 LOC): Index/Search done, Shell/Evaluate/Bench placeholders

## Issue 12: LLM 问答集成 + 意图分类器

**已完成**: gmp-llm 模块 (204 LOC)
**待完成**: Chat CLI command in gmp-cli

```rust
// crates/gmp-cli/src/main.rs — add to Commands enum:
Chat {
    /// Query text
    query: String,
    /// LLM model
    #[arg(long, default_value = "llama3.2")]
    model: String,
    /// Max context tokens
    #[arg(long, default_value_t = 4096)]
    max_context: usize,
    /// Enable intent routing
    #[arg(long, default_value_t = true)]
    route: bool,
}
```

## Issue 13: CLI 全命令

| Command | Status | Implementation |
|---------|--------|----------------|
| `index` | ✅ | Full implementation |
| `search` | ✅ | Full implementation |
| `evaluate` | ⚠️ | Placeholder — needs test runner |
| `shell` | ⚠️ | Basic loop, needs real commands |
| `bench` | ⚠️ | Placeholder — needs benchmark runner |

### Shell Interactive Commands to Implement
- `search <query>` — invoke FourChannelEngine
- `route <query>` — show intent classification
- `rag <query>` — generate answer with RAG context
- `stats` — show index statistics
- `help` / `exit`

## Issue 14: 391 场景测试

**输入**: GMP 法规问答对 JSON corpus
**实现**: Evaluate command with:
- `TestScenario { query, expected_channel, min_relevance_score }`
- Precision/Recall/F1 metrics per channel
- JSON report output

## Issue 15: 用户手册 + 部署指南

- `docs/user-manual.md` — CLI usage guide
- `docs/deployment.md` — Docker, Ollama setup
- `docs/architecture.md` — system design

## Issue 16: 索引增量更新

StorageEngine needs:
```rust
// Add to StorageEngine:
pub fn add_documents(&mut self, docs: Vec<Document>) -> Result<()>
pub fn delete_document(&mut self, doc_id: &str) -> Result<()>
pub fn update_document(&mut self, doc: Document) -> Result<()>
```

## Issue 17: 性能压测 + 基准报告

Bench command needs:
- Concurrent query benchmarking (tokio)
- QPS measurement per channel
- Latency percentiles (p50/p95/p99)
- HTML report generation

## Issue 18: 中国 GMP 条文补充

- Add China GMP specific YAML rules to `gmp-md/`
- Key areas: sterile drugs, biologicals, active pharmaceutical ingredients

## Issue 19: 设备手册数据集成

- Support CSV/Excel import for device manuals
- Chunking + embedding pipeline
- `gmp-cli import --source <path>`

## Issue 20: Web API RESTful

New `gmp-api` crate with Actix-web:
- `POST /api/search` — search endpoint
- `POST /api/chat` — RAG chat endpoint  
- `GET /api/stats` — index statistics
- `POST /api/index` — trigger indexing

## Implementation Order

1. **Issue 12** — Chat command (1 file, ~80 LOC)
2. **Issue 13** — Shell + Evaluate + Bench full impl (~300 LOC)
3. **Issue 16** — Storage incremental API (~100 LOC)
4. **Issue 17** — Bench real implementation (~150 LOC)
5. **Issue 14** — 391 test corpus + eval runner
6. **Issue 15** — Docs
7. **Issue 18-20** — depends on #12-13 foundation

## Branch Strategy

Each issue: `feature/gmp-issue{N}-{short-name}`
PR target: `main` (then sync to develop/v1.0.0)
