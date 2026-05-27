## Why

GMP (Good Manufacturing Practice) 合规判断需要 LLM 实时推理，评估操作是否符合规范。当前缺乏自动化、实时的合规判断能力，需要引入 Compliance Judge Agent。

## What Changes

- 新增 **Compliance Judge Agent** - 基于 LLM 的实时合规判断引擎
- 新增 **合规规则解析器** - 支持多语言合规规则解析和标准化
- 新增 **判断历史审计日志** - 记录所有合规判断及依据
- 新增 **Ollama 集成模块** - 与本地 LLM (Ollama) 集成
- 新增 **置信度评估系统** - 提供判断置信度和依据

## Capabilities

### New Capabilities
- `compliance-judge-agent`: LLM-powered agent that evaluates operations against GMP compliance rules in real-time
- `compliance-rule-parser`: Multi-language compliance rule parsing and standardization
- `compliance-audit-log`: Immutable audit trail for all compliance judgments
- `ollama-integration`: Local LLM integration via Ollama API
- `confidence-scoring`: Confidence score and reasoning explanation for judgments

### Modified Capabilities
- (none - this is a net-new feature)

## Impact

- New crate: `sqlrustgo-compliance-judge`
- New crate: `sqlrustgo-compliance-rules`
- Integration with existing `sqlrustgo-gmp` module
- Integration with Ollama API (HTTP client)
- New HTTP endpoints for compliance judgment requests
