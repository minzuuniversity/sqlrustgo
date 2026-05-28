//! Audit Chain Summarizer — implements SPEC_v3.5.0_1368_AUDIT_SUMMARIZATION.md
//!
//! ## Agent Structure
//!
//! ```text
//! AuditChainSummarizer
//! ├── ChainLoader          # Fetch and validate audit chain
//! ├── EventExtractor       # Identify key events from raw entries
//! ├── TemporalGrouper      # Group events by phase/operation
//! ├── AlertGenerator       # Detect anomalies requiring immediate attention
//! └── LLMComposer          # Generate human-readable summary
//! ```
//!
//! ## Core Features (per SPEC)
//!
//! - **F1**: Long Context Summarization (100+ entries → hierarchical summary)
//! - **F2**: Key Event Extraction (first deviation, signatures, critical params)
//! - **F3**: Alert Generation (anomalies, self-approval, off-hours bulk)
//! - **F4**: Anomaly Scoring (completeness, consistency, timeliness, authenticity)
//!
//! ## Acceptance Criteria
//!
//! | AC | Criteria | Target |
//! |----|----------|--------|
//! | AC1 | Summarization latency | < 1s for 100-entry chain |
//! | AC2 | Support chain size | up to 1000 entries |
//! | AC3 | Key event extraction accuracy | > 90% |
//! | AC4 | Alert generation | actionable alerts for anomalies |
//! | AC5 | Summary history storage | integration test |
//! | AC6 | No hallucination | cite entry IDs in summary |

use crate::types::{
    Alert, AlertType, AuditEntry, AuditQuery, AuditSummary, ChainHealth, ChainTimeRange,
    Importance, KeyEvent, Severity,
};
use async_trait::async_trait;
use chrono::{DateTime, Timelike, Utc};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use thiserror::Error;

/// Maximum entries to process without streaming
const MAX_DIRECT_ENTRIES: usize = 1000;
/// Default summarization timeout (per AC1: < 1s)
const SUMMARIZE_TIMEOUT_MS: u64 = 900;

/// LLM client trait for summary generation
#[async_trait]
pub trait SummarizerLlmClient: Send + Sync {
    async fn chat(&self, prompt: &str, model: &str) -> Result<LmResponse, LmError>;
}

/// LLM response wrapper
#[derive(Debug, Clone)]
pub struct LmResponse {
    pub content: String,
    pub model: Option<String>,
}

/// LLM error type
#[derive(Debug, Error)]
#[error("LLM error: {0}")]
pub struct LmError(pub String);

impl From<LmError> for SummarizerError {
    fn from(e: LmError) -> Self {
        SummarizerError::Llm(e.0)
    }
}

impl From<std::io::Error> for LmError {
    fn from(e: std::io::Error) -> Self {
        LmError(e.to_string())
    }
}

/// Mock LLM response for testing
#[derive(Debug, Clone)]
pub struct MockLmResponse {
    pub content: String,
}

#[async_trait]
impl SummarizerLlmClient for MockLmResponse {
    async fn chat(&self, _prompt: &str, _model: &str) -> Result<LmResponse, LmError> {
        Ok(LmResponse {
            content: self.content.clone(),
            model: Some("mock".to_string()),
        })
    }
}

/// Audit Chain Loader — fetches and validates audit chains
pub struct ChainLoader;

impl ChainLoader {
    /// Load audit entries for a chain (mock — replace with real storage)
    pub async fn load_chain(&self, chain_id: &str) -> Result<Vec<AuditEntry>, SummarizerError> {
        // TODO: Integrate with real GMP audit storage (gmp-api / provenance-graph)
        // For now, return a representative mock chain for demonstration
        let now = Utc::now();
        let entries = vec![
            AuditEntry {
                entry_id: format!("{}-001", chain_id),
                chain_id: chain_id.to_string(),
                event_type: "Create".to_string(),
                description: "偏差报告创建".to_string(),
                actor: "operator_zhang".to_string(),
                timestamp: (now - chrono::Duration::hours(48)).timestamp(),
                fields: [
                    ("deviation_type".to_string(), "Critical".to_string()),
                    ("status".to_string(), "Open".to_string()),
                ]
                .into_iter()
                .collect(),
            },
            AuditEntry {
                entry_id: format!("{}-002", chain_id),
                chain_id: chain_id.to_string(),
                event_type: "Update".to_string(),
                description: "偏差参数更新：温度超标".to_string(),
                actor: "operator_zhang".to_string(),
                timestamp: (now - chrono::Duration::hours(47)).timestamp(),
                fields: [
                    ("field".to_string(), "temperature".to_string()),
                    ("old_value".to_string(), "25°C".to_string()),
                    ("new_value".to_string(), "32°C".to_string()),
                ]
                .into_iter()
                .collect(),
            },
            AuditEntry {
                entry_id: format!("{}-003", chain_id),
                chain_id: chain_id.to_string(),
                event_type: "Approve".to_string(),
                description: "偏差批准 — 同一操作员".to_string(),
                actor: "operator_zhang".to_string(),
                timestamp: (now - chrono::Duration::hours(46)).timestamp(),
                fields: [
                    ("role".to_string(), "operator".to_string()),
                    ("self_approved".to_string(), "true".to_string()),
                ]
                .into_iter()
                .collect(),
            },
            AuditEntry {
                entry_id: format!("{}-004", chain_id),
                chain_id: chain_id.to_string(),
                event_type: "Reject".to_string(),
                description: "主管驳回 — 需要第二批准人".to_string(),
                actor: "supervisor_li".to_string(),
                timestamp: (now - chrono::Duration::hours(45)).timestamp(),
                fields: [(
                    "reason".to_string(),
                    "Self-approval not allowed for critical deviations".to_string(),
                )]
                .into_iter()
                .collect(),
            },
            AuditEntry {
                entry_id: format!("{}-005", chain_id),
                chain_id: chain_id.to_string(),
                event_type: "Approve".to_string(),
                description: "QA批准 — 偏差关闭".to_string(),
                actor: "qa_wang".to_string(),
                timestamp: (now - chrono::Duration::hours(44)).timestamp(),
                fields: [
                    ("role".to_string(), "qa".to_string()),
                    ("status".to_string(), "Closed".to_string()),
                ]
                .into_iter()
                .collect(),
            },
        ];
        Ok(entries)
    }
}

/// Event Extractor — identifies key events from raw entries
pub struct EventExtractor;

impl EventExtractor {
    /// Extract key events from raw audit entries per SPEC F2
    pub fn extract_key_events(&self, entries: &[AuditEntry]) -> Vec<KeyEvent> {
        let mut key_events = Vec::new();

        for entry in entries {
            let importance = Self::classify_importance(entry);
            if importance == Importance::Info && !Self::is_structurally_important(entry) {
                continue;
            }

            key_events.push(KeyEvent::new(
                entry.entry_id.clone(),
                entry.event_type.clone(),
                entry.description.clone(),
                entry.actor.clone(),
                entry.timestamp,
                importance,
            ));
        }

        key_events
    }

    /// Classify importance per SPEC F2 key events list
    fn classify_importance(entry: &AuditEntry) -> Importance {
        match entry.event_type.as_str() {
            "Reject" => Importance::Critical,
            "Approve" => {
                // Self-approval is critical
                if entry.fields.get("self_approved") == Some(&"true".to_string()) {
                    Importance::Critical
                } else {
                    Importance::Major
                }
            }
            "Update" => {
                // Critical parameter changes are major
                if entry.description.contains("超标") || entry.description.contains("out-of-range")
                {
                    Importance::Critical
                } else {
                    Importance::Minor
                }
            }
            "Create" => Importance::Major,
            "Delete" => Importance::Critical,
            _ => Importance::Info,
        }
    }

    /// Check if event is structurally important (required signatures, phase changes)
    fn is_structurally_important(entry: &AuditEntry) -> bool {
        entry.event_type == "Approve"
            || entry.event_type == "Reject"
            || entry.fields.contains_key("required_signature")
    }

    /// Group entries temporally (per F1 temporal grouping)
    pub fn group_by_phase<'a>(
        &self,
        entries: &'a [AuditEntry],
    ) -> HashMap<String, Vec<&'a AuditEntry>> {
        let mut groups: HashMap<String, Vec<&AuditEntry>> = HashMap::new();
        for entry in entries {
            let phase = Self::phase_from_timestamp(entry.timestamp);
            groups.entry(phase).or_default().push(entry);
        }
        groups
    }

    fn phase_from_timestamp(ts: i64) -> String {
        let dt = DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now);
        let hour = dt.hour();
        if hour < 6 {
            "夜间 (00:00-06:00)".to_string()
        } else if hour < 12 {
            "上午 (06:00-12:00)".to_string()
        } else if hour < 18 {
            "下午 (12:00-18:00)".to_string()
        } else {
            "晚间 (18:00-24:00)".to_string()
        }
    }
}

/// Alert Generator — detects anomalies per SPEC F3
pub struct AlertGenerator;

impl AlertGenerator {
    /// Generate alerts from audit entries and key events
    pub fn generate_alerts(&self, entries: &[AuditEntry], _key_events: &[KeyEvent]) -> Vec<Alert> {
        let mut alerts = Vec::new();

        // Alert 1: Self-approval detection (F3 trigger)
        alerts.extend(self.detect_self_approval(entries));

        // Alert 2: Missing required signatures
        alerts.extend(self.detect_missing_signatures(entries));

        // Alert 3: Off-hours bulk modifications
        alerts.extend(self.detect_off_hours_bulk(entries));

        // Alert 4: Critical parameter out of range
        alerts.extend(self.detect_param_out_of_range(entries));

        // Alert 5: Sequence anomalies (e.g., reject before required approval)
        alerts.extend(self.detect_sequence_anomalies(entries));

        // Deduplicate by entry_id
        let mut seen = std::collections::HashSet::new();
        alerts.retain(|a| {
            let key = format!(
                "{}:{}",
                a.alert_type.as_str(),
                a.evidence.first().unwrap_or(&String::new())
            );
            seen.insert(key)
        });

        alerts
    }

    fn detect_self_approval(&self, entries: &[AuditEntry]) -> Vec<Alert> {
        let mut alerts = Vec::new();
        for entry in entries {
            if entry.fields.get("self_approved") == Some(&"true".to_string()) {
                alerts.push(Alert::new(
                    AlertType::SelfApproval,
                    Severity::Major,
                    format!(
                        "Record {} approved by creator ({}) — violates separation of duties",
                        entry.entry_id, entry.actor
                    ),
                    vec![entry.entry_id.clone()],
                    "Require second approver for high-risk changes (critical deviations, batch release)".to_string(),
                ));
            }
        }
        alerts
    }

    fn detect_missing_signatures(&self, entries: &[AuditEntry]) -> Vec<Alert> {
        let mut alerts = Vec::new();
        // Critical deviations require QA + supervisor approval
        let has_critical = entries
            .iter()
            .any(|e| e.fields.get("deviation_type") == Some(&"Critical".to_string()));

        if has_critical {
            let approvers: std::collections::HashSet<_> = entries
                .iter()
                .filter(|e| e.event_type == "Approve")
                .map(|e| e.actor.clone())
                .collect();

            let has_qa = approvers.iter().any(|a| a.starts_with("qa_"));
            let has_supervisor = approvers.iter().any(|a| a.starts_with("supervisor_"));

            if !has_qa || !has_supervisor {
                let mut evidence = Vec::new();
                if let Some(e) = entries.first() {
                    evidence.push(e.entry_id.clone());
                }
                alerts.push(Alert::new(
                    AlertType::MissingSignature,
                    Severity::Major,
                    "Critical deviation missing required approvals (QA + supervisor)".to_string(),
                    evidence,
                    "Ensure both QA and supervisor approve critical deviations".to_string(),
                ));
            }
        }
        alerts
    }

    fn detect_off_hours_bulk(&self, entries: &[AuditEntry]) -> Vec<Alert> {
        let mut alerts = Vec::new();
        let off_hours_entries: Vec<_> = entries
            .iter()
            .filter(|e| {
                let hour = DateTime::from_timestamp(e.timestamp, 0)
                    .map(|dt| dt.hour())
                    .unwrap_or(0);
                !(6..22).contains(&hour) && e.event_type == "Update"
            })
            .collect();

        if off_hours_entries.len() >= 3 {
            alerts.push(Alert::new(
                AlertType::OffHoursBulkModification,
                Severity::Major,
                format!(
                    "{} bulk modifications detected outside business hours",
                    off_hours_entries.len()
                ),
                off_hours_entries.iter().map(|e| e.entry_id.clone()).collect(),
                "Review off-hours changes — require justification or restrict batch modifications to business hours".to_string(),
            ));
        }
        alerts
    }

    fn detect_param_out_of_range(&self, entries: &[AuditEntry]) -> Vec<Alert> {
        let mut alerts = Vec::new();
        for entry in entries {
            let is_out_of_range = entry.description.contains("超标")
                || entry.description.contains("out-of-range")
                || entry.description.contains("超出范围");

            if is_out_of_range {
                alerts.push(Alert::new(
                    AlertType::ParameterOutOfRange,
                    Severity::Critical,
                    format!("Critical parameter out of range: {}", entry.description),
                    vec![entry.entry_id.clone()],
                    "Initiate CAPA immediately and quarantine affected batch".to_string(),
                ));
            }
        }
        alerts
    }

    fn detect_sequence_anomalies(&self, entries: &[AuditEntry]) -> Vec<Alert> {
        let mut alerts = Vec::new();

        // Check: reject before any approval (unusual sequence)
        let reject_idx = entries.iter().position(|e| e.event_type == "Reject");
        let first_approve_idx = entries.iter().position(|e| e.event_type == "Approve");

        if let (Some(r_idx), Some(a_idx)) = (reject_idx, first_approve_idx) {
            if r_idx > a_idx {
                // Reject happened before first approval — possible fraud pattern
                let evidence: Vec<_> = entries
                    .iter()
                    .skip(r_idx.saturating_sub(2))
                    .take(3)
                    .map(|e| e.entry_id.clone())
                    .collect();
                alerts.push(Alert::new(
                    AlertType::SecurityConcern,
                    Severity::Major,
                    "Unusual event sequence: reject occurred after approvals — potential fraud pattern".to_string(),
                    evidence,
                    "Investigate event sequence. Require audit trail review for complex approval chains".to_string(),
                ));
            }
        }
        alerts
    }
}

/// Health Score Calculator — per SPEC F4
pub struct HealthScoreCalculator;

impl HealthScoreCalculator {
    /// Calculate chain health score
    pub fn calculate(&self, entries: &[AuditEntry], alerts: &[Alert]) -> ChainHealth {
        let completeness = self.score_completeness(entries);
        let consistency = self.score_consistency(entries);
        let timeliness = self.score_timeliness(entries);
        let authenticity = self.score_authenticity(entries, alerts);

        // Anomaly score from alerts reduces overall health
        let anomaly_penalty = alerts
            .iter()
            .map(|a| match a.severity {
                Severity::Critical => 20.0,
                Severity::Major => 10.0,
                Severity::Minor => 5.0,
                Severity::Info => 2.0,
            })
            .sum::<f64>();

        let score = (completeness + consistency + timeliness + authenticity) / 4.0
            - anomaly_penalty.clamp(0.0, 40.0);

        ChainHealth {
            score: score.clamp(0.0, 100.0),
            completeness,
            consistency,
            timeliness,
            authenticity,
        }
    }

    fn score_completeness(&self, entries: &[AuditEntry]) -> f64 {
        // Check required fields presence
        let required_fields = ["entry_id", "chain_id", "event_type", "actor", "timestamp"];
        let total_checks: usize = entries.len() * required_fields.len();
        let passed_checks: usize = entries
            .iter()
            .flat_map(|e| required_fields.iter().map(move |f| (f, &e.fields)))
            .filter(|(_, fields)| !fields.is_empty())
            .count();

        if total_checks == 0 {
            return 100.0;
        }
        (passed_checks as f64 / total_checks as f64 * 100.0).clamp(0.0, 100.0)
    }

    fn score_consistency(&self, entries: &[AuditEntry]) -> f64 {
        // Check for timestamp consistency (no future timestamps, monotonically reasonable)
        let now = Utc::now().timestamp();
        let has_future = entries.iter().any(|e| e.timestamp > now + 3600); // 1hr grace
        let has_negative = entries.iter().any(|e| e.timestamp < 0);

        if has_future || has_negative {
            50.0
        } else {
            100.0
        }
    }

    fn score_timeliness(&self, entries: &[AuditEntry]) -> f64 {
        // Check if approvals happened within reasonable time (e.g., < 72 hours from creation)
        let creates: Vec<_> = entries
            .iter()
            .filter(|e| e.event_type == "Create")
            .collect();
        let approves: Vec<_> = entries
            .iter()
            .filter(|e| e.event_type == "Approve")
            .collect();

        if creates.is_empty() || approves.is_empty() {
            return 100.0;
        }

        let first_create = creates.iter().map(|e| e.timestamp).min().unwrap_or(0);
        let last_approve = approves.iter().map(|e| e.timestamp).max().unwrap_or(0);

        let hours_diff = (last_approve - first_create) as f64 / 3600.0;
        if hours_diff > 72.0 {
            60.0 // Took too long
        } else if hours_diff > 48.0 {
            80.0
        } else {
            100.0
        }
    }

    fn score_authenticity(&self, _entries: &[AuditEntry], alerts: &[Alert]) -> f64 {
        // Presence of self-approval reduces authenticity
        let has_self_approval = alerts
            .iter()
            .any(|a| a.alert_type == AlertType::SelfApproval);

        if has_self_approval {
            60.0
        } else {
            100.0
        }
    }
}

/// LLM Composer — generates human-readable summaries
pub struct LLMComposer<'a> {
    llm_client: &'a dyn SummarizerLlmClient,
    model: String,
}

impl<'a> LLMComposer<'a> {
    pub fn new(llm_client: &'a dyn SummarizerLlmClient, model: &str) -> Self {
        Self {
            llm_client,
            model: model.to_string(),
        }
    }

    /// Build the summarization prompt per SPEC Section 5
    fn build_prompt(
        &self,
        chain_id: &str,
        entity_type: &str,
        entity_id: &str,
        time_range: &ChainTimeRange,
        total_events: usize,
        key_events: &[KeyEvent],
        alerts: &[Alert],
    ) -> String {
        let events_json = serde_json::to_string_pretty(key_events).unwrap_or_default();
        let alerts_json = serde_json::to_string_pretty(alerts).unwrap_or_default();

        format!(
            r#"你是一个 GMP 审计分析师。请准确总结以下审计链，始终引用 entry ID 标注关键结论。
        对于不确定的内容明确标注。

优先关注：偏离标准操作、批准行为、关键事件。

用户要求总结此审计链：
Chain ID: {chain_id}
Entity: {entity_type} / {entity_id}
Time Range: {start} 至 {end}
Total Events: {total_events}

Key Events:
{events_json}

Alerts:
{alerts_json}

请提供：
1. Executive Summary (最多2句)
2. Key Events (编号列表，最多10项)
3. Anomalies (如有，包含 entry IDs)
4. Recommendations (如有)"#,
            chain_id = chain_id,
            entity_type = entity_type,
            entity_id = entity_id,
            start = time_range.start.format("%Y-%m-%d %H:%M"),
            end = time_range.end.format("%Y-%m-%d %H:%M"),
            total_events = total_events,
            events_json = events_json,
            alerts_json = alerts_json,
        )
    }

    /// Parse LLM response into executive + detailed summaries
    fn parse_llm_response(&self, response: &str) -> (String, String) {
        // Try to extract executive summary and details from structured response
        let lines: Vec<&str> = response.lines().collect();
        let mut executive = String::new();
        let mut in_key_events = false;
        let mut in_anomalies = false;
        let mut detailed_parts = Vec::new();

        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Detect sections
            let lower = trimmed.to_lowercase();
            if lower.starts_with("executive summary")
                || lower.starts_with("1.") && executive.is_empty()
            {
                in_key_events = false;
                in_anomalies = false;
                // Capture content until next numbered section
                executive.push_str(trimmed);
                executive.push('\n');
                continue;
            }

            if lower.starts_with("key events") || lower.starts_with("2.") {
                in_key_events = true;
                in_anomalies = false;
                detailed_parts.push(trimmed);
                continue;
            }

            if lower.starts_with("anomalies") || lower.starts_with("3.") {
                in_key_events = false;
                in_anomalies = true;
                detailed_parts.push(trimmed);
                continue;
            }

            if lower.starts_with("recommendations") || lower.starts_with("4.") {
                in_key_events = false;
                in_anomalies = false;
                detailed_parts.push(trimmed);
                continue;
            }

            if in_key_events || in_anomalies || !executive.is_empty() {
                detailed_parts.push(trimmed);
            } else {
                executive.push_str(trimmed);
                executive.push('\n');
            }
        }

        let executive = if executive.is_empty() {
            response.lines().take(3).collect::<Vec<_>>().join(" ")
        } else {
            executive.trim().to_string()
        };

        let detailed = if detailed_parts.is_empty() {
            response.to_string()
        } else {
            detailed_parts.join("\n")
        };

        (executive, detailed)
    }

    /// Generate summaries using LLM
    pub async fn compose(
        &self,
        chain_id: &str,
        entity_type: &str,
        entity_id: &str,
        time_range: &ChainTimeRange,
        total_events: usize,
        key_events: &[KeyEvent],
        alerts: &[Alert],
    ) -> Result<(String, String), SummarizerError> {
        let prompt = self.build_prompt(
            chain_id,
            entity_type,
            entity_id,
            time_range,
            total_events,
            key_events,
            alerts,
        );

        let response = self.llm_client.chat(&prompt, &self.model).await?;
        let (executive, detailed) = self.parse_llm_response(&response.content);

        Ok((executive, detailed))
    }
}

/// Audit Chain Summarizer — main trait implementation per SPEC Section 3.3
#[async_trait]
pub trait AuditChainSummarizer: Send + Sync {
    /// Summarize a single audit chain
    async fn summarize(&self, chain_id: &str) -> Result<AuditSummary, SummarizerError>;

    /// Summarize multiple chains (e.g., all deviations in a month)
    async fn batch_summarize(
        &self,
        query: &AuditQuery,
    ) -> Result<Vec<AuditSummary>, SummarizerError>;

    /// Generate alerts only (faster, for real-time monitoring)
    async fn check_alerts(&self, chain_id: &str) -> Result<Vec<Alert>, SummarizerError>;

    /// Get chain health score
    async fn health_score(&self, chain_id: &str) -> Result<ChainHealth, SummarizerError>;
}

/// Default implementation of AuditChainSummarizer
pub struct DefaultAuditChainSummarizer<L: SummarizerLlmClient> {
    llm_client: Box<L>,
    model: String,
    chain_loader: ChainLoader,
    event_extractor: EventExtractor,
    alert_generator: AlertGenerator,
    health_calculator: HealthScoreCalculator,
}

impl<L: SummarizerLlmClient> DefaultAuditChainSummarizer<L> {
    pub fn new(llm_client: L, model: &str) -> Self {
        Self {
            llm_client: Box::new(llm_client),
            model: model.to_string(),
            chain_loader: ChainLoader,
            event_extractor: EventExtractor,
            alert_generator: AlertGenerator,
            health_calculator: HealthScoreCalculator,
        }
    }
}

#[async_trait]
impl<L: SummarizerLlmClient + 'static> AuditChainSummarizer for DefaultAuditChainSummarizer<L> {
    async fn summarize(&self, chain_id: &str) -> Result<AuditSummary, SummarizerError> {
        let start = Instant::now();

        // Step 1: Load chain (AC2: support up to 1000 entries)
        let entries = tokio::time::timeout(
            Duration::from_millis(200),
            self.chain_loader.load_chain(chain_id),
        )
        .await
        .map_err(|_| SummarizerError::Timeout)?
        .map_err(|e| SummarizerError::Load(e.to_string()))?;

        if entries.len() > MAX_DIRECT_ENTRIES {
            return Err(SummarizerError::ChainTooLarge(
                entries.len(),
                MAX_DIRECT_ENTRIES,
            ));
        }

        // Step 2: Extract key events (AC3: > 90% accuracy target — LLM-assisted)
        let key_events = self.event_extractor.extract_key_events(&entries);

        // Step 3: Generate alerts (AC4)
        let alerts = self.alert_generator.generate_alerts(&entries, &key_events);

        // Step 4: Calculate health score
        let health = self.health_calculator.calculate(&entries, &alerts);

        // Step 5: Time range
        let timestamps: Vec<_> = entries.iter().map(|e| e.timestamp).collect();
        let start_ts = *timestamps.iter().min().unwrap_or(&Utc::now().timestamp());
        let end_ts = *timestamps.iter().max().unwrap_or(&Utc::now().timestamp());
        let time_range = ChainTimeRange {
            start: DateTime::from_timestamp(start_ts, 0).unwrap_or_else(Utc::now),
            end: DateTime::from_timestamp(end_ts, 0).unwrap_or_else(Utc::now),
        };

        // Step 6: LLM compose (AC1: < 1s latency target)
        let llm_composer = LLMComposer::new(self.llm_client.as_ref(), &self.model);
        let (executive_summary, detailed_summary) = tokio::time::timeout(
            Duration::from_millis(SUMMARIZE_TIMEOUT_MS),
            llm_composer.compose(
                chain_id,
                "Deviation",
                chain_id,
                &time_range,
                entries.len(),
                &key_events,
                &alerts,
            ),
        )
        .await
        .map_err(|_| SummarizerError::Timeout)?
        .map_err(|e| SummarizerError::Llm(e.to_string()))?;

        let _elapsed_ms = start.elapsed().as_millis() as i64;

        Ok(AuditSummary {
            chain_id: chain_id.to_string(),
            entity_type: "Deviation".to_string(),
            entity_id: chain_id.to_string(),
            time_range,
            total_events: entries.len(),
            key_events,
            anomaly_score: 100.0 - health.score,
            alerts,
            executive_summary,
            detailed_summary,
            generated_at: Utc::now().timestamp_millis(),
        })
    }

    async fn batch_summarize(
        &self,
        query: &AuditQuery,
    ) -> Result<Vec<AuditSummary>, SummarizerError> {
        // TODO: Query GMP storage for matching chains
        // For now, return empty batch (real implementation would list chains from storage)
        let limit = query.limit.unwrap_or(100);
        let summaries = Vec::with_capacity(limit.min(10));

        // Placeholder: in real impl, enumerate chains from storage and summarize each
        // Use `batch_summarize` with streaming for chains > 10

        Ok(summaries)
    }

    async fn check_alerts(&self, chain_id: &str) -> Result<Vec<Alert>, SummarizerError> {
        let entries = self.chain_loader.load_chain(chain_id).await?;
        let key_events = self.event_extractor.extract_key_events(&entries);
        Ok(self.alert_generator.generate_alerts(&entries, &key_events))
    }

    async fn health_score(&self, chain_id: &str) -> Result<ChainHealth, SummarizerError> {
        let entries = self.chain_loader.load_chain(chain_id).await?;
        let alerts = self.alert_generator.generate_alerts(&entries, &[]);
        Ok(self.health_calculator.calculate(&entries, &alerts))
    }
}

/// Summarizer errors
#[derive(Debug, Error)]
pub enum SummarizerError {
    #[error("Chain load failed: {0}")]
    Load(String),

    #[error("LLM error: {0}")]
    Llm(String),

    #[error("Timeout after ...ms")]
    Timeout,

    #[error("Chain too large: {0} entries (max: {1})")]
    ChainTooLarge(usize, usize),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_entries() -> Vec<AuditEntry> {
        let now = Utc::now();
        vec![
            AuditEntry {
                entry_id: "dev-001-001".to_string(),
                chain_id: "dev-001".to_string(),
                event_type: "Create".to_string(),
                description: "Critical deviation created".to_string(),
                actor: "operator_zhang".to_string(),
                timestamp: (now - chrono::Duration::hours(48)).timestamp(),
                fields: [("deviation_type".to_string(), "Critical".to_string())]
                    .into_iter()
                    .collect(),
            },
            AuditEntry {
                entry_id: "dev-001-002".to_string(),
                chain_id: "dev-001".to_string(),
                event_type: "Update".to_string(),
                description: "温度超标: 32°C (limit: 30°C)".to_string(),
                actor: "operator_zhang".to_string(),
                timestamp: (now - chrono::Duration::hours(47)).timestamp(),
                fields: [
                    ("field".to_string(), "temperature".to_string()),
                    ("value".to_string(), "32°C".to_string()),
                ]
                .into_iter()
                .collect(),
            },
            AuditEntry {
                entry_id: "dev-001-003".to_string(),
                chain_id: "dev-001".to_string(),
                event_type: "Approve".to_string(),
                description: "Self-approved by creator".to_string(),
                actor: "operator_zhang".to_string(),
                timestamp: (now - chrono::Duration::hours(46)).timestamp(),
                fields: [("self_approved".to_string(), "true".to_string())]
                    .into_iter()
                    .collect(),
            },
        ]
    }

    #[test]
    fn test_event_extractor_extracts_key_events() {
        let extractor = EventExtractor;
        let entries = make_test_entries();
        let key_events = extractor.extract_key_events(&entries);

        // Should skip non-structural info events, keep important ones
        assert!(!key_events.is_empty());
        // Critical deviation (Create) is Major
        assert!(key_events.iter().any(|e| e.importance == Importance::Major));
        // Self-approval is Critical
        assert!(key_events
            .iter()
            .any(|e| e.importance == Importance::Critical));
    }

    #[test]
    fn test_event_extractor_phase_grouping() {
        let extractor = EventExtractor;
        let entries = make_test_entries();
        let groups = extractor.group_by_phase(&entries);
        assert!(!groups.is_empty());
    }

    #[test]
    fn test_alert_generator_detects_self_approval() {
        let generator = AlertGenerator;
        let entries = make_test_entries();
        let alerts = generator.generate_alerts(&entries, &[]);

        assert!(alerts
            .iter()
            .any(|a| a.alert_type == AlertType::SelfApproval));
    }

    #[test]
    fn test_alert_generator_detects_param_out_of_range() {
        let generator = AlertGenerator;
        let entries = make_test_entries();
        let alerts = generator.generate_alerts(&entries, &[]);

        assert!(alerts
            .iter()
            .any(|a| a.alert_type == AlertType::ParameterOutOfRange));
    }

    #[test]
    fn test_health_score_calculation() {
        let calculator = HealthScoreCalculator;
        let entries = make_test_entries();
        let alerts = vec![Alert::new(
            AlertType::SelfApproval,
            Severity::Major,
            "Self-approval detected".to_string(),
            vec!["dev-001-003".to_string()],
            "Require second approver".to_string(),
        )];
        let health = calculator.calculate(&entries, &alerts);

        assert!(health.score >= 0.0 && health.score <= 100.0);
        assert!(health.authenticity < 100.0); // Self-approval reduces authenticity
    }

    #[tokio::test]
    async fn test_summarizer_end_to_end() {
        use std::collections::HashMap;

        let mock_response = r#"Executive Summary:
该审计链记录了一起关键偏差的全生命周期过程，涉及创建、参数超标和自批准异常。

Key Events:
1. [dev-001-001] Create: Critical deviation created (operator_zhang, Major)
2. [dev-001-002] Update: 温度超标 (operator_zhang, Critical)
3. [dev-001-003] Approve: Self-approved by creator (operator_zhang, Critical)

Anomalies:
- Self-approval detected: entry dev-001-003
- Critical parameter out of range: entry dev-001-002

Recommendations:
- Require second approver for critical deviations
- Initiate CAPA for temperature excursion"#;

        let mock = MockLmResponse {
            content: mock_response.to_string(),
        };
        let summarizer: DefaultAuditChainSummarizer<MockLmResponse> =
            DefaultAuditChainSummarizer::new(mock, "mock-model");

        let result = summarizer.summarize("dev-001").await.unwrap();

        assert_eq!(result.chain_id, "dev-001");
        assert!(result.total_events >= 3);
        assert!(!result.key_events.is_empty());
        assert!(!result.alerts.is_empty());
        assert!(result.anomaly_score >= 0.0);
    }

    #[tokio::test]
    async fn test_check_alerts_only() {
        let mock = MockLmResponse {
            content: String::new(),
        };
        let summarizer: DefaultAuditChainSummarizer<MockLmResponse> =
            DefaultAuditChainSummarizer::new(mock, "mock-model");

        let alerts = summarizer.check_alerts("dev-001").await.unwrap();
        assert!(!alerts.is_empty());
    }

    #[tokio::test]
    async fn test_health_score_only() {
        let mock = MockLmResponse {
            content: String::new(),
        };
        let summarizer: DefaultAuditChainSummarizer<MockLmResponse> =
            DefaultAuditChainSummarizer::new(mock, "mock-model");

        let health = summarizer.health_score("dev-001").await.unwrap();
        assert!(health.score >= 0.0 && health.score <= 100.0);
    }
}
