# Issue 13 Spec: CLI 全命令增强

## 现状
- `index` ✅ 完整
- `search` ✅ 完整（--channel, --rerank, --explain）
- `evaluate` ✅ 完整
- `shell` ⚠️ 仅支持 `search`, `help`, `index`
- `bench` ✅ 完整

## 需要增强

### Shell 命令增强
| 命令 | 功能 |
|------|------|
| `route <query>` | 意图分类 + 通道路由 |
| `rag <query>` | LLM RAG 问答 |
| `stats` | 索引统计（文档数/chunks/各通道） |
| `channel <c> <q>` | 指定通道搜索 |
| `quit/exit` | 退出 |

### 实现方式
- `route`/`rag`: 调用 `gmp_llm::LlmClient`（同步）
- `stats`: 调用 `gmp_storage::StorageEngine::validate()`
- `channel`: 复用现有 `search_fts` 逻辑

### 约束
- Shell 必须是同步 REPL，不能用 async
- route/rag 调用 LLM client（async），需用 `tokio::runtime::Runtime::new().block_on()`
