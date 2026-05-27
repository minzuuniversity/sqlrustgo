## Context

Compliance Judge Agent 是 GMP Platform v3.5.0 P0 功能之一。需要实现一个基于 LLM 的实时合规判断引擎，评估操作是否符合 GMP 规范。

**当前状态**:
- GMP 平台已有基础框架 (sqlrustgo-gmp)
- 已有 Ollama 集成的基础设施 (sqlrustgo-ollama)

**约束**:
- 实时合规判断延迟 < 500ms
- 需要与本地 LLM (Ollama) 集成
- 合规判断需要可审计

## Goals / Non-Goals

**Goals:**
- 实现 LLM-powered Compliance Judge Agent
- 支持多语言合规规则解析
- 提供判断置信度和依据
- 记录完整审计日志

**Non-Goals:**
- 不实现规则管理 UI (后续版本)
- 不实现规则自动学习/更新 (后续版本)
- 不实现多租户隔离 (不在当前范围)

## Decisions

### 1. Agent Architecture: Single-Pass Judge

**Decision**: 采用单次 LLM 调用完成合规判断
**Rationale**: 500ms 延迟要求限制了多轮对话架构

**Alternatives Considered**:
- Multi-turn agent: 太慢，不满足延迟要求
- Rule-based + LLM fallback: 复杂度高，不适合初始版本

### 2. Ollama Integration Pattern

**Decision**: 使用 reqwest HTTP client 调用 Ollama API
**Rationale**: Ollama 提供标准 REST API，已有类似集成模式

**Alternatives Considered**:
- llama.cpp direct binding: 复杂度高，维护成本大
- External Ollama service: 增加部署复杂度

### 3. Confidence Scoring

**Decision**: 基于 LLM response 的 logprob 计算置信度
**Rationale**: Ollama 支持返回 logprob，可用于置信度估算

### 4. Audit Log Storage

**Decision**: 使用 sqlrustgo-storage 的 WAL 日志机制
**Rationale**: 复用现有事务和持久化基础设施

## Risks / Trade-offs

- **[Risk] LLM 响应时间不稳定** → **Mitigation**: 实现超时机制和 fallback 策略
- **[Risk] 合规规则歧义** → **Mitigation**: 返回 "需要人工审核" 状态，不做假判断
- **[Risk] Ollama 版本兼容性** → **Mitigation**: 抽象 OllamaClient trait，便于 mock 测试

## Open Questions

1. 具体的合规规则格式 (YAML/JSON/DSL)?
2. 判断结果缓存策略?
3. 是否需要支持批量合规判断?
