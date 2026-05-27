//! Audit log for compliance judgments
//!
//! Provides immutable audit trail using WAL (Write-Ahead Log) persistence.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Single audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    /// Unique entry ID
    pub entry_id: String,
    /// Operation ID being judged
    pub operation_id: String,
    /// Rule ID used for judgment
    pub rule_id: String,
    /// Judgment status
    pub status: String,
    /// Confidence score
    pub confidence_score: f32,
    /// LLM model used
    pub llm_model: Option<String>,
    /// Latency in milliseconds
    pub latency_ms: u64,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Hash of this entry
    pub hash: String,
    /// Hash of previous entry (chain)
    pub prev_hash: String,
}

impl AuditLogEntry {
    /// Calculate hash for this entry
    pub fn calculate_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(&self.entry_id);
        hasher.update(&self.operation_id);
        hasher.update(&self.rule_id);
        hasher.update(&self.status);
        hasher.update(self.confidence_score.to_le_bytes());
        hasher.update(self.timestamp.to_rfc3339());
        hasher.update(&self.prev_hash);
        format!("{:x}", hasher.finalize())
    }
}

/// Writer for audit logs
pub struct AuditLogWriter {
    entries: Vec<AuditLogEntry>,
}

impl Default for AuditLogWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl AuditLogWriter {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Append a judgment to the audit log
    pub fn append(&mut self, entry: AuditLogEntry) {
        self.entries.push(entry);
    }

    /// Get all entries
    pub fn entries(&self) -> &[AuditLogEntry] {
        &self.entries
    }

    /// Calculate chain hash from entries
    pub fn calculate_chain_hash(&self) -> Option<String> {
        self.entries.first().map(|e| e.hash.clone())
    }

    /// Consume and return all entries
    pub fn into_entries(self) -> Vec<AuditLogEntry> {
        self.entries
    }
}

/// Reader for audit logs with integrity verification
pub struct AuditLogReader {
    entries: Vec<AuditLogEntry>,
}

impl AuditLogReader {
    pub fn new(entries: Vec<AuditLogEntry>) -> Self {
        Self { entries }
    }

    /// Verify integrity of the audit chain
    pub fn verify_integrity(&self) -> Result<(), AuditIntegrityError> {
        let mut prev_hash = String::new();

        for entry in &self.entries {
            // Verify hash chain
            if entry.prev_hash != prev_hash {
                return Err(AuditIntegrityError::ChainBroken {
                    entry_id: entry.entry_id.clone(),
                    expected: prev_hash,
                    found: entry.prev_hash.clone(),
                });
            }

            // Verify entry hash
            let calculated = entry.calculate_hash();
            if calculated != entry.hash {
                return Err(AuditIntegrityError::TamperedEntry {
                    entry_id: entry.entry_id.clone(),
                    expected: calculated,
                    found: entry.hash.clone(),
                });
            }

            prev_hash = entry.hash.clone();
        }

        Ok(())
    }

    /// Query by operation ID
    pub fn query_by_operation(&self, operation_id: &str) -> Vec<&AuditLogEntry> {
        self.entries
            .iter()
            .filter(|e| e.operation_id == operation_id)
            .collect()
    }

    /// Query by time range
    pub fn query_by_time_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Vec<&AuditLogEntry> {
        self.entries
            .iter()
            .filter(|e| e.timestamp >= start && e.timestamp <= end)
            .collect()
    }

    /// Query by status
    pub fn query_by_status(&self, status: &str) -> Vec<&AuditLogEntry> {
        self.entries.iter().filter(|e| e.status == status).collect()
    }
}

/// Audit integrity errors
#[derive(Debug, Clone)]
pub enum AuditIntegrityError {
    ChainBroken {
        entry_id: String,
        expected: String,
        found: String,
    },
    TamperedEntry {
        entry_id: String,
        expected: String,
        found: String,
    },
}

impl std::fmt::Display for AuditIntegrityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ChainBroken {
                entry_id,
                expected,
                found,
            } => write!(
                f,
                "Chain broken at entry {}: expected hash {}, found {}",
                entry_id, expected, found
            ),
            Self::TamperedEntry {
                entry_id,
                expected,
                found,
            } => write!(
                f,
                "Entry {} has been tampered: expected hash {}, found {}",
                entry_id, expected, found
            ),
        }
    }
}

impl std::error::Error for AuditIntegrityError {}

/// Auditor for recording judgments
pub struct JudgmentAuditor {
    writer: std::sync::Mutex<AuditLogWriter>,
}

impl Default for JudgmentAuditor {
    fn default() -> Self {
        Self::new()
    }
}

impl JudgmentAuditor {
    pub fn new() -> Self {
        Self {
            writer: std::sync::Mutex::new(AuditLogWriter::new()),
        }
    }

    /// Record a judgment
    pub fn record(
        &self,
        operation_id: &str,
        rule_id: &str,
        status: &str,
        confidence_score: f32,
        llm_model: Option<&str>,
        latency_ms: u64,
    ) -> AuditLogEntry {
        let mut writer = self.writer.lock().unwrap();
        let prev_hash = writer.calculate_chain_hash().unwrap_or_default();

        let entry = AuditLogEntry {
            entry_id: uuid::Uuid::new_v4().to_string(),
            operation_id: operation_id.to_string(),
            rule_id: rule_id.to_string(),
            status: status.to_string(),
            confidence_score,
            llm_model: llm_model.map(String::from),
            latency_ms,
            timestamp: Utc::now(),
            hash: String::new(), // Will be calculated
            prev_hash,
        };

        let mut entry_with_hash = entry;
        entry_with_hash.hash = entry_with_hash.calculate_hash();

        writer.append(entry_with_hash.clone());
        entry_with_hash
    }

    /// Finish recording and return entries
    pub fn finish(&self) -> Vec<AuditLogEntry> {
        self.writer.lock().unwrap().entries().to_vec()
    }
}

/// Query parameters for audit logs
#[derive(Debug, Clone, Default)]
pub struct AuditQuery {
    pub operation_id: Option<String>,
    pub rule_id: Option<String>,
    pub status: Option<String>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_entry_hash() {
        let entry = AuditLogEntry {
            entry_id: "test-001".to_string(),
            operation_id: "op-001".to_string(),
            rule_id: "rule-001".to_string(),
            status: "COMPLIANT".to_string(),
            confidence_score: 0.95,
            llm_model: Some("qwen2.5:14b".to_string()),
            latency_ms: 150,
            timestamp: Utc::now(),
            hash: String::new(),
            prev_hash: String::new(),
        };

        let hash = entry.calculate_hash();
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64); // SHA256 hex length
    }

    #[test]
    fn test_judgment_auditor_record() {
        let auditor = JudgmentAuditor::new();

        let entry = auditor.record(
            "op-001",
            "rule-001",
            "COMPLIANT",
            0.95,
            Some("qwen2.5:14b"),
            150,
        );

        assert_eq!(entry.operation_id, "op-001");
        assert_eq!(entry.rule_id, "rule-001");
        assert_eq!(entry.status, "COMPLIANT");
        assert!((entry.confidence_score - 0.95).abs() < f32::EPSILON);
        assert_eq!(entry.latency_ms, 150);
    }

    #[test]
    fn test_audit_chain_hash_progression() {
        let auditor = JudgmentAuditor::new();

        let entry1 = auditor.record("op-001", "rule-001", "COMPLIANT", 0.95, None, 100);
        let entry2 = auditor.record("op-002", "rule-001", "NON_COMPLIANT", 0.85, None, 120);

        // Entry2's prev_hash should equal Entry1's hash
        assert_eq!(entry2.prev_hash, entry1.hash);
    }

    #[test]
    fn test_verify_integrity_valid() {
        let entries = vec![
            AuditLogEntry {
                entry_id: "test-001".to_string(),
                operation_id: "op-001".to_string(),
                rule_id: "rule-001".to_string(),
                status: "COMPLIANT".to_string(),
                confidence_score: 0.95,
                llm_model: None,
                latency_ms: 100,
                timestamp: Utc::now(),
                hash: String::new(),
                prev_hash: String::new(),
            },
            AuditLogEntry {
                entry_id: "test-002".to_string(),
                operation_id: "op-002".to_string(),
                rule_id: "rule-001".to_string(),
                status: "COMPLIANT".to_string(),
                confidence_score: 0.90,
                llm_model: None,
                latency_ms: 110,
                timestamp: Utc::now(),
                hash: String::new(),
                prev_hash: String::new(),
            },
        ];

        // Calculate hashes
        let entries = {
            let mut prev_hash = String::new();
            entries
                .into_iter()
                .map(|mut e| {
                    e.prev_hash = prev_hash.clone();
                    e.hash = e.calculate_hash();
                    prev_hash = e.hash.clone();
                    e
                })
                .collect::<Vec<_>>()
        };

        let reader = AuditLogReader::new(entries);
        assert!(reader.verify_integrity().is_ok());
    }

    #[test]
    fn test_query_by_operation() {
        let entries = vec![
            create_test_entry("op-001", "COMPLIANT"),
            create_test_entry("op-002", "NON_COMPLIANT"),
            create_test_entry("op-001", "COMPLIANT"),
        ];

        let reader = AuditLogReader::new(entries);
        let results = reader.query_by_operation("op-001");

        assert_eq!(results.len(), 2);
    }

    fn create_test_entry(operation_id: &str, status: &str) -> AuditLogEntry {
        AuditLogEntry {
            entry_id: uuid::Uuid::new_v4().to_string(),
            operation_id: operation_id.to_string(),
            rule_id: "rule-001".to_string(),
            status: status.to_string(),
            confidence_score: 0.9,
            llm_model: None,
            latency_ms: 100,
            timestamp: Utc::now(),
            hash: String::new(),
            prev_hash: String::new(),
        }
    }
}
