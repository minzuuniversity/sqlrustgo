# Brainstorming: Reranker 实现 (gmp-reranker)

## 问题陈述

gmp-reranker/src/lib.rs 是空文件，BGE-Reranker-v2-m3 未实现。Layer 3 的 Reranker 层缺失，导致 RRF 输出的结果未经 Cross-Encoder 重排，影响答案质量。

## 5Why 深挖

**Why 1: Reranker 为什么是空的？**
- 因为原始 PR #26 的 commit message 没有包含 reranker 的具体实现

**Why 2: 为什么 commit 没有包含实现？**
- 因为 Issue #11 关闭时，将"框架存在"等同于"功能完成"

**Why 3: 为什么框架存在但功能没有？**
- 因为 VectorChannel.search() 调用后，rerank 参数被丢弃（`rerank: _`），没有传递给融合流程

**Why 4: 为什么 rerank 参数被丢弃？**
- 因为 `FourChannelEngine::search` 和 `RrfFusion::fuse` 的接口设计中没有集成 reranker 的位置

**Why 5: 为什么接口设计缺少 reranker？**
- 因为 Reranker 需要在 RRF 融合之后、返回结果之前介入，但当前架构没有定义这个步骤

## Causal Chain

```
原始问题: RRF 融合后直接返回，未经过 reranker 重排
     ↓
中间问题: RrfFusion.fuse() 输出 Vec<ScoredChunk> 后没有 rerank 调用
     ↓
接口问题: RetrievalEngine.search() 没有 rerank 参数
     ↓
CLI 问题: Search command 中 rerank: _ 未传递给检索引擎
     ↓
根因: gmp-reranker 空实现 + 架构缺少 rerank 集成点
```

## Root Causes (with evidence)

1. **Root Cause A: gmp-reranker 是空文件**
   - 证据: `cat crates/gmp-reranker/src/lib.rs` 输出为空
   - 影响: 无任何 rerank 函数可调用

2. **Root Cause B: CLI 层的 rerank 参数未传递**
   - 证据: `main.rs:177` - `rerank: _,`
   - 影响: 即使 reranker 实现，用户也无法启用

3. **Root Cause C: FourChannelEngine.search() 没有 rerank 集成点**
   - 证据: `channel.rs:365` - `pub async fn search(...) -> Vec<ScoredChunk>`
   - 影响: fusion 之后的结果无法 rerank

## Proposed Approach

### Approach A: 在 RrfFusion.fuse() 之后单独 rerank
**优点**: 最小改动，只加一个 rerank 函数
**缺点**: 需要修改 search 流程，rerank 是 post-processing

### Approach B: 改造 FourChannelEngine，添加 rerank 参数
**优点**: 架构上更清晰，rerank 是独立步骤
**缺点**: 需要改接口，影响面较大

### Approach C: 创建一个新的 RetrievalPipeline，先 fusion 再 rerank
**优点**: 最符合四层架构定义，rerank 是独立 Layer 3.5
**缺点**: 新增类型，改动最大

**推荐: Approach A** — 最少改动，在 `search` 命令中增加 rerank 分支，先 fusion 再 rerank。

## Open Questions

- [ ] Reranker 如何调用 Ollama 的 bge-reranker-v2-m3 API？
- [ ] Reranker 的输入是 query + top-k chunks，输出是重排后的 chunks，接口签名是什么？
- [ ] Reranker 是同步还是异步？
- [ ] Ollama 的 reranker API 是否支持 CPU 模式？

## Next Steps

1. 写 SPEC 确定接口
2. GitNexus 分析 Ollama reranker API
3. 实现 gmp-reranker/src/lib.rs
4. 修改 CLI 传递 rerank 参数
5. 修改 FourChannelEngine 集成 reranker
