# v3.5.0 Changelog

> **Version**: 3.5.0
> **Date**: TBD
> **Status**: 开发中 (Alpha)
> **Branch**: `develop/v3.5.0`
> **From**: v3.4.0 GA (`d934228b`)

---

## v3.5.0 (TBD) - 开发中

### 规划中

#### AI Agent Layer（P0）— `gmp-api`

**AI 偏差调查助手**
- Deviation Investigator Agent — 自然语言查询审计链，返回引用证据
- AI 辅助根因分析，降低偏差调查时间
- Issue: #1361

**LLM 合规判断引擎**
- Compliance Judge Agent — 规则引擎 + LLM 自动判断批次放行
- 100% 可解释输出，每条判断附证据链
- Issue: #1362

**GMP Retrieval v3**
- BM25 + Vector + Graph + FTS 四路融合（RRF k=60）+ Reranker
- 接入 gmp-api，`/api/v1/gmp/retrieve` 统一入口
- Issue: #1363

#### LLM 本地推理（P0）— `gmp-llm` 新crate

**Ollama 集成**
- 本地 LLM 推理（qwen2.5:7b / llama3 / bge-m3 embedding）
- SSE 流式输出（Server-Sent Events）
- Issue: #1364, #1365

#### 预测性维护（P1）— `gmp-api`

**Device Predictor Agent**
- 时序数据 + 设备故障预测，提前 24h 预警
- Issue: #1366

**Report Generator Agent**
- 自然语言报表生成，Markdown/JSON 多格式导出
- Issue: #1367

**Audit Chain Summarization**
- 审计链 AI 摘要，提取关键事件
- Issue: #1368

**Rule Recommendation Engine**
- 基于历史违规数据自动推荐新合规规则
- Issue: #1369

#### 延续 Issue

**DiskGraphStore Cypher**
- v3.4.0 延期功能，完成 Cypher 执行支持
- Issue: #1326

---

## v3.4.0 (2026-05-24) - GA

See [../v3.4.0/CHANGELOG.md](../v3.4.0/CHANGELOG.md)