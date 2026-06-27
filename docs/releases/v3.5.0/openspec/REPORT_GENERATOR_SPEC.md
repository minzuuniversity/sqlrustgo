# SPEC: Report Generator Agent

> **Version**: 1.0
> **Date**: 2026-05-26
> **Issue**: #1367
> **Status**: SPEC
> **Assignee**: hermes-macmini

## 1. 概述

Report Generator Agent 提供自然语言到 GMP 报表的转换能力。用户用自然语言描述需求，系统自动从 GMP 数据源检索相关记录并生成结构化报表。

## 2. 功能范围

### 2.1 输入

- 自然语言查询（如"本周批次通过率"、"上月偏差汇总"）
- 可选：报表格式（markdown/json/pdf）、时间范围、语言

### 2.2 输出

- Markdown 报表（含表格、统计数据）
- JSON 结构化数据
- PDF 导出（future）

### 2.3 报表类型

| 类型 | 查询关键词 | 数据源 |
|------|-----------|--------|
| 批次汇总 | 本周/月批次、通过率、合格率 | batch_records |
| 合规报告 | 偏差、变更、纠正措施、CAPA | deviation_records |
| 设备报告 | 设备状态、OEE、预测维护 | equipment_records |
| 审计摘要 | 审计链、操作记录 | audit_logs |

## 3. 技术设计

### 3.1 模块结构

```
crates/compliance-engine/src/report/
├── lib.rs          # 模块入口
├── agent.rs        # ReportGenerator Agent
├── intent.rs       # 意图解析（LLM 调用）
├── retrieval.rs    # 数据检索接口
├── formatter.rs    # 报表格式化
└── types.rs        # 类型定义
```

### 3.2 ReportGenerator Agent

```rust
pub struct ReportGenerator {
    llm: LLMOrchestrator,
    retrieval: GMPRetrievalV3,
}

impl ReportGenerator {
    /// 生成报表
    pub async fn generate(&self, request: &ReportRequest) -> ReportResult;
    
    /// 流式生成（用于长报表）
    pub fn generate_stream(&self, request: &ReportRequest) -> impl Stream<Item = ReportChunk>;
}
```

### 3.3 接口定义

```rust
// 请求
pub struct ReportRequest {
    pub query: String,           // 自然语言查询
    pub report_type: ReportType, // 报表类型（自动推断或指定）
    pub format: ReportFormat,    // 输出格式
    pub language: Option<String>,// 语言 zh/en/ja
    pub time_range: Option<TimeRange>,
}

// 报表类型
pub enum ReportType {
    BatchSummary,    // 批次汇总
    Compliance,     // 合规报告
    Equipment,       // 设备报告
    AuditSummary,    // 审计摘要
    Auto,            // 自动推断
}

// 输出
pub struct ReportResult {
    pub report_id: String,
    pub title: String,
    pub content: String,         // Markdown
    pub data: Option<serde_json::Value>, // JSON 结构化数据
    pub metadata: ReportMetadata,
    pub generated_at: DateTime<Utc>,
}

pub struct ReportMetadata {
    pub query: String,
    pub report_type: ReportType,
    pub format: ReportFormat,
    pub record_count: usize,
    pub confidence: f32,
    pub generation_time_ms: u64,
}
```

### 3.4 意图解析（Intent Parsing）

LLM 从自然语言提取结构化参数：

```rust
pub struct ParsedIntent {
    pub report_type: ReportType,
    pub time_range: Option<TimeRange>,
    pub filters: Vec<QueryFilter>,
    pub grouping: Option<String>,  // 按日/周/月分组
    pub metrics: Vec<String>,      // 指标列表
}
```

Prompt 模板：
```
你是一个 GMP 报表助手。请从用户查询中提取报表参数。

用户查询：{query}

请输出 JSON：
{
  "report_type": "批次汇总|合规报告|设备报告|审计摘要|自动",
  "time_range": {"start": "YYYY-MM-DD", "end": "YYYY-MM-DD"} | null,
  "filters": [{"field": "字段名", "operator": "eq|gt|lt", "value": "值"}],
  "grouping": "day|week|month|null",
  "metrics": ["指标1", "指标2"]
}
```

### 3.5 数据检索

通过 GMPRetrievalV3 查询历史数据（复用现有检索基础设施）。

### 3.6 格式化

```rust
pub trait ReportFormatter {
    fn format(&self, data: &ReportData, template: &ReportTemplate) -> String;
}

pub struct MarkdownFormatter;
pub struct JsonFormatter;
```

## 4. 验收标准

| 标准 | 要求 |
|------|------|
| 自然语言理解 | 能解析"本周批次通过率"、"上月偏差"等常见查询 |
| 报表生成 | 生成 Markdown，含表格和统计数据 |
| JSON 输出 | 提供结构化 JSON 格式 |
| 准确率 | 指标计算与数据源一致 |
| 多语言 | 支持 zh/en/ja 输出 |
| 跨语言报告 | 支持生成其他语言的报表（#1411） |
| 延迟 | 生成延迟 < 3s |

## 5. 依赖

- LLM Orchestrator（复用 ComplianceJudge 的 OllamaClient trait）
- GMPRetrievalV3（数据检索）
- chrono（时间处理）
- tokio（异步运行时）

## 6. 文件清单

| 文件 | 操作 |
|------|------|
| `crates/compliance-engine/src/report/mod.rs` | 新建 |
| `crates/compliance-engine/src/report/agent.rs` | 新建 |
| `crates/compliance-engine/src/report/intent.rs` | 新建 |
| `crates/compliance-engine/src/report/retrieval.rs` | 新建 |
| `crates/compliance-engine/src/report/formatter.rs` | 新建 |
| `crates/compliance-engine/src/report/types.rs` | 新建 |
| `crates/compliance-engine/src/lib.rs` | 修改：添加 `pub mod report;` |
| `crates/compliance-engine/Cargo.toml` | 修改：添加 serde_json 依赖 |

## 7. 测试策略

- 意图解析：Mock LLM，验证参数提取
- 格式化：Golden test，验证 Markdown 输出格式
- 集成测试：Mock 检索层，验证端到端生成
