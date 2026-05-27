//! Audit Chain Summarizer types — aligned with SPEC_v3.5.0_1368_AUDIT_SUMMARIZATION.md

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Importance level for key events
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Importance {
    Critical,
    Major,
    Minor,
    Info,
}

impl Importance {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Critical => "critical",
            Self::Major => "major",
            Self::Minor => "minor",
            Self::Info => "info",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "critical" => Self::Critical,
            "major" => Self::Major,
            "minor" => Self::Minor,
            _ => Self::Info,
        }
    }
}

/// Alert severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Critical,
    Major,
    Minor,
    Info,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Critical => "critical",
            Self::Major => "major",
            Self::Minor => "minor",
            Self::Info => "info",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "critical" => Self::Critical,
            "major" => Self::Major,
            "minor" => Self::Minor,
            _ => Self::Info,
        }
    }
}

/// Alert type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertType {
    /// Unusual sequence of events (potential fraud)
    SecurityConcern,
    /// Missing required signatures
    MissingSignature,
    /// Approval by same person who created record
    SelfApproval,
    /// Bulk modifications outside business hours
    OffHoursBulkModification,
    /// Critical parameter out-of-range
    ParameterOutOfRange,
    /// Role changes or access anomalies
    AccessAnomaly,
    /// CAPA not initiated when required
    CapaMissing,
}

impl AlertType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SecurityConcern => "security_concern",
            Self::MissingSignature => "missing_signature",
            Self::SelfApproval => "self_approval",
            Self::OffHoursBulkModification => "off_hours_bulk_modification",
            Self::ParameterOutOfRange => "parameter_out_of_range",
            Self::AccessAnomaly => "access_anomaly",
            Self::CapaMissing => "capa_missing",
        }
    }
}

/// A key event extracted from an audit chain entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyEvent {
    /// Unique entry ID in the audit chain
    pub event_id: String,
    /// Event type (Create/Update/Delete/Approve/Reject)
    pub event_type: String,
    /// Human-readable description
    pub description: String,
    /// Actor who performed the action
    pub actor: String,
    /// Unix timestamp
    pub timestamp: i64,
    /// Importance level
    pub importance: Importance,
    /// Related entity IDs
    #[serde(default)]
    pub related_entities: Vec<String>,
}

impl KeyEvent {
    pub fn new(
        event_id: String,
        event_type: String,
        description: String,
        actor: String,
        timestamp: i64,
        importance: Importance,
    ) -> Self {
        Self {
            event_id,
            event_type,
            description,
            actor,
            timestamp,
            importance,
            related_entities: Vec::new(),
        }
    }
}

/// An alert generated from anomaly detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    /// Alert type
    pub alert_type: AlertType,
    /// Severity level
    pub severity: Severity,
    /// Human-readable description
    pub description: String,
    /// Evidence entry IDs
    #[serde(default)]
    pub evidence: Vec<String>,
    /// Recommended action
    pub recommendation: String,
}

impl Alert {
    pub fn new(
        alert_type: AlertType,
        severity: Severity,
        description: String,
        evidence: Vec<String>,
        recommendation: String,
    ) -> Self {
        Self {
            alert_type,
            severity,
            description,
            evidence,
            recommendation,
        }
    }

    /// Serialize to JSON matching the SPEC alert format
    pub fn to_spec_json(&self) -> serde_json::Value {
        serde_json::json!({
            "type": self.alert_type.as_str(),
            "severity": self.severity.as_str(),
            "description": self.description,
            "evidence": self.evidence,
            "recommendation": self.recommendation,
        })
    }
}

/// Chain health score components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainHealth {
    /// Overall health score 0-100
    pub score: f64,
    /// Completeness score 0-100 (missing required fields?)
    pub completeness: f64,
    /// Consistency score 0-100 (contradictions with other records?)
    pub consistency: f64,
    /// Timeliness score 0-100 (approvals within SLA?)
    pub timeliness: f64,
    /// Authenticity score 0-100 (signature verification)
    pub authenticity: f64,
}

/// Time range for a chain or query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainTimeRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

/// Complete audit chain summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditSummary {
    /// Chain identifier
    pub chain_id: String,
    /// Entity type (Deviation/CAPA/Batch/etc)
    pub entity_type: String,
    /// Entity identifier
    pub entity_id: String,
    /// Time range of the chain
    pub time_range: ChainTimeRange,
    /// Total number of events in chain
    pub total_events: usize,
    /// Extracted key events
    #[serde(default)]
    pub key_events: Vec<KeyEvent>,
    /// Anomaly score 0-100
    pub anomaly_score: f64,
    /// Generated alerts
    #[serde(default)]
    pub alerts: Vec<Alert>,
    /// Executive summary (2 sentences max)
    pub executive_summary: String,
    /// Detailed summary
    pub detailed_summary: String,
    /// Generation timestamp (unix ms)
    pub generated_at: i64,
}

/// Audit entry as loaded from storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub entry_id: String,
    pub chain_id: String,
    pub event_type: String,
    pub description: String,
    pub actor: String,
    pub timestamp: i64,
    pub fields: std::collections::HashMap<String, String>,
}

/// Query for batch summarizing multiple chains
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditQuery {
    /// Entity type filter
    pub entity_type: Option<String>,
    /// Time range filter
    pub time_range: Option<ChainTimeRange>,
    /// Minimum anomaly score threshold
    pub min_anomaly_score: Option<f64>,
    /// Limit results
    pub limit: Option<usize>,
}

impl Default for AuditQuery {
    fn default() -> Self {
        Self {
            entity_type: None,
            time_range: None,
            min_anomaly_score: None,
            limit: Some(100),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_importance_round_trip() {
        assert_eq!(Importance::from_str("critical"), Importance::Critical);
        assert_eq!(Importance::from_str("major"), Importance::Major);
        assert_eq!(Importance::from_str("MINOR"), Importance::Minor);
        assert_eq!(Importance::from_str("unknown"), Importance::Info);
    }

    #[test]
    fn test_severity_round_trip() {
        assert_eq!(Severity::from_str("critical"), Severity::Critical);
        assert_eq!(Severity::from_str("major"), Severity::Major);
    }

    #[test]
    fn test_alert_type_serialization() {
        let alert = Alert::new(
            AlertType::SelfApproval,
            Severity::Major,
            "Record approved by creator".to_string(),
            vec!["entry_123".to_string(), "entry_456".to_string()],
            "Require second approver for high-risk changes".to_string(),
        );
        let json = alert.to_spec_json();
        assert_eq!(json["type"], "self_approval");
        assert_eq!(json["severity"], "major");
        assert_eq!(
            json["evidence"],
            serde_json::json!(["entry_123", "entry_456"])
        );
    }

    #[test]
    fn test_chain_health_default() {
        let health = ChainHealth {
            score: 85.0,
            completeness: 90.0,
            consistency: 80.0,
            timeliness: 95.0,
            authenticity: 75.0,
        };
        assert!(health.score >= 0.0 && health.score <= 100.0);
    }
}
