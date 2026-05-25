# SPEC: Report Generator Agent (#1367)

## 1. Concept & Vision

An AI agent that transforms natural language queries into structured GMP reports. Users describe what they want to know in plain Chinese/English, and the agent synthesizes data from multiple GMP sources into polished, audit-ready documents.

**Design Philosophy**: "Ask in any language, get audit-ready reports" — democratize data access for non-technical operators.

---

## 2. System Architecture

### 2.1 Agent Structure

```
ReportGeneratorAgent
├── IntentClassifier      # Parse user query intent
├── DataRetriever        # Multi-source data fetching (FTS/Vector/Graph)
├── TemplateEngine       # Report structure templates
├── ChartRenderer        # Data visualization
└── LLM synthesizer      # Compose final report
```

### 2.2 Report Types

| Report Type | Data Sources | Template |
|-------------|--------------|----------|
| Deviation Report | FTS + Audit Chain | deviance_template.md |
| CAPA Analysis | Graph + FTS | capa_template.md |
| Audit Summary | Audit Chain + LLM | audit_summary_template.md |
| Equipment Status | Time-series + Graph | equipment_template.md |
| Batch Record Review | All sources | batch_record_template.md |

---

## 3. Functionality Specification

### 3.1 Core Features

#### F1: Natural Language Query Understanding
- **Input**: "Show me all deviations from Q1 2026 related to equipment failures"
- **Processing**:
  - Entity extraction (dates, equipment names, deviation types)
  - Temporal parsing (Q1 2026, last 30 days, etc.)
  - Intent classification (report type, filters)
- **Output**: Structured query params

#### F2: Multi-Source Data Retrieval
- **Sources**:
  - FTS5: Deviation descriptions, CAPA text
  - Vector: Semantic similarity search
  - Graph: Entity relationships, workflow history
  - Audit Chain: Timestamped events
- **Fusion**: RRF (Reciprocal Rank Fusion) across sources

#### F3: Template-Based Report Generation
- **Template Engine**:
  ```rust
  struct ReportTemplate {
      name: String,
      sections: Vec<Section>,
      required_fields: Vec<String>,
      optional_fields: Vec<String>,
  }
  
  struct Section {
      title: String,
      content_type: ContentType, // Text/Table/Chart/Metric
      data_query: String,        // Natural language query for this section
      llm_instructions: String,  // How to synthesize this section
  }
  ```

#### F4: Multi-Format Export
- **Formats**: Markdown (default), JSON (structured), HTML (charts)
- **PDF/Excel**: Future v1.1 (requires external rendering)

#### F5: Interactive Refinement
- **User can ask**: "Make it more detailed", "Add equipment section", "Focus on root cause"
- **State tracking**: Maintain conversation context for follow-ups

### 3.2 Data Model

```rust
struct ReportRequest {
    natural_query: String,
    intent: ReportIntent,
    time_range: Option<TimeRange>,
    filters: Vec<Filter>,
    format: ExportFormat,
}

struct GeneratedReport {
    id: String,
    title: String,
    sections: Vec<ReportSection>,
    metadata: ReportMetadata,  // Generated at, author, data sources
    raw_data: serde_json::Value,  // For downstream processing
}

struct ReportSection {
    title: String,
    content: SectionContent,  // Text, table, or chart spec
    sources: Vec<DataSource>,
    confidence: f64,          // LLM confidence in this section
}
```

### 3.3 API Surface

```rust
trait ReportGeneratorAgent {
    /// Generate report from natural language query
    async fn generate(&self, query: &str) -> Result<GeneratedReport>;
    
    /// Refine existing report based on feedback
    async fn refine(&self, report_id: &str, feedback: &str) -> Result<GeneratedReport>;
    
    /// Export to specific format
    async fn export(&self, report_id: &str, format: ExportFormat) -> Result<Vec<u8>>;
    
    /// List available report templates
    fn list_templates(&self) -> Vec<ReportTemplate>;
}
```

---

## 4. Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| AC1 | Support natural language queries in Chinese and English | Unit tests |
| AC2 | Generate Markdown reports with proper structure | Integration test |
| AC3 | Support chart rendering (bar, line, pie) | Visual inspection |
| AC4 | Include data source citations in report | Manual review |
| AC5 | Response time < 10s for typical report | Benchmark |
| AC6 | Handle ambiguous queries gracefully | User acceptance test |

---

## 5. LLM Prompt Strategy

```
System: You are a GMP report assistant. Generate audit-ready reports based on retrieved data.
        Always cite your data sources. Flag uncertainty in the report.
        
User: {natural_query}

Context:
- Retrieved data: {retrieved_chunks}
- Report template: {template_name}
- Time range: {time_range}

Generate a report following the template structure. Include:
1. Executive summary (2-3 sentences)
2. Key findings (bullet points)
3. Detailed analysis
4. Supporting data (tables/charts)
5. Conclusions and recommendations
```

---

## 6. Out of Scope (v1.0)

- PDF generation
- Excel export
- Scheduled report delivery
- Report versioning
- Multi-tenant access control
