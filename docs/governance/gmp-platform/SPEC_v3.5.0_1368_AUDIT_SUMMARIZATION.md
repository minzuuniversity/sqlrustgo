# SPEC: Audit Chain AI Summarization (#1368)

## 1. Concept & Vision

An AI-powered summarization engine that distills long GMP audit chains into actionable insights. Instead of scrolling through hundreds of audit entries, operators get a concise briefing of what happened, who did it, and what matters.

**Design Philosophy**: "Audit chains are narratives, not logs" — surface the story, not just the events.

---

## 2. System Architecture

### 2.1 Agent Structure

```
AuditChainSummarizer
├── ChainLoader          # Fetch and validate audit chain
├── EventExtractor       # Identify key events from raw entries
├── TemporalGrouper      # Group events by phase/operation
├── AlertGenerator       # Detect anomalies requiring immediate attention
└── LLMComposer          # Generate human-readable summary
```

---

## 3. Functionality Specification

### 3.1 Core Features

#### F1: Long Context Summarization
- **Input**: Audit chain with 100+ entries
- **Processing**:
  - Event clustering by type (Create/Update/Delete/Approve/Reject)
  - Temporal grouping (by hour/day/operation phase)
  - Entity extraction (user, document, equipment involved)
- **Output**: Hierarchical summary (3 levels: Executive → Detailed → Raw)

#### F2: Key Event Extraction
- **Key Events**:
  - First deviation from standard procedure
  - Signature approvals/rejections
  - Critical parameter out-of-range
  - Role changes or access anomalies
  - CAPA initiation
- **Priority Scoring**: Critical > Major > Minor > Info

#### F3: Alert Generation
- **Alert Triggers**:
  - Unusual sequence of events (potential fraud)
  - Missing required signatures
  - Approval by same person who created record
  - Bulk modifications outside business hours
- **Alert Format**:
  ```json
  {
    "type": "security_concern",
    "severity": "major",
    "description": "Record approved by creator",
    "evidence": ["entry_id_123", "entry_id_456"],
    "recommendation": "Require second approver for high-risk changes"
  }
  ```

#### F4: Anomaly Scoring
- **Metrics**:
  - Completeness (missing required fields?)
  - Consistency (contradictions with other records?)
  - Timeliness (approvals within SLA?)
  - Authenticity (signature verification)

### 3.2 Data Model

```rust
struct AuditSummary {
    chain_id: String,
    entity_type: String,         // Deviation/CAPA/Batch/etc
    entity_id: String,
    time_range: TimeRange,
    total_events: usize,
    key_events: Vec<KeyEvent>,
    anomaly_score: f64,         // 0-100
    alerts: Vec<Alert>,
    executive_summary: String,
    detailed_summary: String,
    generated_at: i64,
}

struct KeyEvent {
    event_id: String,
    event_type: String,
    description: String,
    actor: String,
    timestamp: i64,
    importance: Importance,      // Critical/Major/Minor
    related_entities: Vec<String>,
}

struct Alert {
    alert_type: AlertType,
    severity: Severity,
    description: String,
    evidence: Vec<String>,
    recommendation: String,
}
```

### 3.3 API Surface

```rust
trait AuditChainSummarizer {
    /// Summarize a single audit chain
    async fn summarize(&self, chain_id: &str) -> Result<AuditSummary>;
    
    /// Summarize multiple chains (e.g., all deviations in a month)
    async fn batch_summarize(&self, query: &AuditQuery) -> Result<Vec<AuditSummary>>;
    
    /// Generate alerts only (faster, for real-time monitoring)
    async fn check_alerts(&self, chain_id: &str) -> Result<Vec<Alert>>;
    
    /// Get chain health score
    async fn health_score(&self, chain_id: &str) -> Result<ChainHealth>;
}
```

---

## 4. Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| AC1 | Summarization latency < 1s for 100-entry chain | Benchmark |
| AC2 | Support chains up to 1000 entries | Stress test |
| AC3 | Extract key events with > 90% accuracy | Manual evaluation |
| AC4 | Generate actionable alerts for anomalies | Red team test |
| AC5 | Store summary history for audit | Integration test |
| AC6 | No hallucination: cite entry IDs in summary | Manual review |

---

## 5. LLM Prompt Strategy

```
System: You are a GMP audit analyst. Summarize audit chains accurately.
        Always cite entry IDs for key claims. Flag uncertainty explicitly.
        Prioritize: deviations from standard, approvals, critical events.

User: Summarize this audit chain:
Chain ID: {chain_id}
Entity: {entity_type} / {entity_id}
Time Range: {start} to {end}
Total Events: {count}

Events:
{events_json}

Provide:
1. Executive Summary (2 sentences max)
2. Key Events (numbered list, max 10)
3. Anomalies (if any, with entry IDs)
4. Recommendations (if any)
```

---

## 6. Out of Scope (v1.0)

- Real-time streaming audit monitoring
- Predictive anomaly detection
- Cross-chain relationship analysis
- Audit report PDF generation
