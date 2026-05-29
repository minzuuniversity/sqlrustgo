# AI Agent Architecture

> **Version**: 1.0
> **Date**: 2026-05-26
> **Issue**: #1360
> **Status**: Planning

## 一、概述

AI Agent Layer 是 v3.5.0 的核心新功能层，提供四个专业化 AI Agent：

| Agent | Issue | 职责 |
|-------|-------|------|
| Deviation Investigator | #1361 | 偏差调查与根因分析 |
| Compliance Judge | #1362 | 合规判断与批次放行 |
| Report Generator | #1367 | 自然语言报表生成 |
| Device Predictor | #1366 | 预测性设备维护 |

## 二、架构设计

### 2.1 Agent 公共接口

```rust
pub trait AIAgent {
    fn name(&self) -> &str;
    fn process(&self, input: AgentInput) -> AgentOutput;
    async fn process_stream(&self, input: AgentInput) -> Stream<AgentOutput>;
}
```

### 2.2 LLM Orchestrator

所有 Agent 通过统一的 LLM Orchestrator 访问 LLM 能力：

```
Agent → LLM Orchestrator → Ollama/OpenAI API
```

### 2.3 共享组件

- **Context Manager**: 管理对话上下文，支持多轮对话
- **Tool Registry**: 注册可用工具（检索、数据库、API）
- **Evidence Collector**: 收集 AI 推理证据，支持可解释性

## 三、部署架构

```
┌─────────────────────────────────────────┐
│           GMP API (sqlrustgo-gmp-api)    │
├─────────────────────────────────────────┤
│  Deviation │ Compliance │ Report │ Device│
│  Agent     │   Judge    │Generator│Predictor│
├─────────────────────────────────────────┤
│           LLM Orchestrator               │
│        (Ollama / OpenAI fallback)        │
└─────────────────────────────────────────┘
```

## 四、OpenSpec

See: `openspec/AI_AGENT_ARCHITECTURE.md`
