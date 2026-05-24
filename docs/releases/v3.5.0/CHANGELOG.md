# v3.5.0 Changelog

> **Version**: 3.5.0
> **Date**: TBD
> **Status**: 开发中 (Alpha)
> **Branch**: `develop/v3.5.0`
> **From**: v3.4.0 GA (`d934228b`)

---

## v3.5.0 (TBD) - 开发中

### 规划中

#### AI Agent Layer（P0）— `sqlrustgo-gmp-api`

**AI 偏差调查助手**
- Deviation Investigator Agent — 自然语言查询审计链，返回引用证据
- AI 辅助根因分析，降低偏差调查时间
- Issue: #1360

**LLM 合规判断引擎**
- Compliance Judge Agent — 规则引擎 + LLM 自动判断批次放行
- 100% 可解释输出，每条判断附证据链
- Issue: #1361

**GMP Retrieval v3**
- BM25 + Vector + Graph 三路融合（RRF k=60）
- 接入 gmp-api，`/api/v1/gmp/retrieve` 统一入口
- Issue: #1362

#### LLM 本地推理（P0）— `sqlrustgo-gmp-llm`

**Ollama 集成**
- 本地 LLM 推理（qwen2.5:7b / llama3）
- bge-m3 embedding 文本向量化
- SSE 流式输出（Server-Sent Events）
- Issue: #1363, #1364

**本地模型支持**
- 支持 Ollama 本地部署（企业合规要求）
- 支持 OpenAI-compatible API fallback
- Issue: #1363

#### 预测性维护（P1）— `sqlrustgo-gmp-api`

**Device Predictor Agent**
- 时序数据 + 设备故障预测
- 提前 24h 预警
- Issue: #1365

---

## v3.4.0 (2026-05-24) - GA

See [../v3.4.0/CHANGELOG.md](../v3.4.0/CHANGELOG.md)