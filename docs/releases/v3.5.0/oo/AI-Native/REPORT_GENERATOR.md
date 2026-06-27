# Report Generator Agent

> **Version**: 1.0
> **Date**: 2026-05-26
> **Issue**: #1367
> **Status**: Planning

## 一、功能描述

自然语言报表生成，一句话说"本周批次通过率"自动生成报告。

## 二、支持的报表类型

| 报表类型 | 输出格式 | 说明 |
|---------|---------|------|
| 批次汇总 | Markdown/PDF | 本周/月批次通过率、合格率 |
| 合规报告 | Markdown/PDF | 偏差、变更、纠正措施 |
| 设备报告 | Markdown/JSON | 设备状态、预测性维护 |
| 审计摘要 | Markdown | 长审计链摘要 |

## 三、技术方案

### 3.1 自然语言理解

```rust
pub struct ReportGenerator {
    llm: LLMOrchestrator,
    retrieval: GMPRetrievalV3,
}

impl ReportGenerator {
    pub fn generate(&self, query: &str) -> Report {
        // 1. 解析查询意图
        let intent = self.llm.parse_intent(query);
        
        // 2. 检索相关数据
        let data = self.retrieval.query(&intent).await;
        
        // 3. 生成报表
        self.llm.generate_report(&intent, &data)
    }
}
```

## 四、验收标准

- 支持自然语言查询
- Markdown/JSON 输出
- 跨语言报告 (#1411)
