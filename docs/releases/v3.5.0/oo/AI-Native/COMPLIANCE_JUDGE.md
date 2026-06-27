# Compliance Judge Agent

> **Version**: 1.0
> **Date**: 2026-05-26
> **Issue**: #1362
> **Status**: Planning

## 一、功能描述

LLM 合规判断引擎，规则引擎 + LLM 自动判断批次放行，带可解释证据链。

## 二、判断流程

1. 接收批次数据 + 规则集
2. 规则引擎预过滤（快速判断）
3. LLM 深度分析（复杂情况）
4. 返回判断结果 + 证据链

## 三、技术方案

### 3.1 规则引擎集成

```rust
pub struct ComplianceJudge {
    rule_engine: RuleEngine,
    llm: LLMOrchestrator,
}

impl ComplianceJudge {
    pub fn judge(&self, batch: Batch, rules: Vec<Rule>) -> Judgment {
        // 1. 规则引擎快速判断
        let quick_verdict = self.rule_engine.evaluate(&batch, &rules);
        
        // 2. LLM 深度分析（仅复杂情况）
        if quick_verdict.needs_llm() {
            return self.llm.analyze(&batch, &rules);
        }
        
        quick_verdict
    }
}
```

## 四、验收标准

- 输入批次数据 + 规则，输出放行建议 + 理由
- 100% 可解释输出，每条判断附证据链
- G9 Gate: AI Explainability 100%

## 五、依赖

- GMP Batch API
- GMP Rule Engine
- Ollama 集成 (#1364)
