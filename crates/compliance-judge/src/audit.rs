//! Audit Log
//!
//! Immutable audit trail for compliance judgments using WAL persistence and hash chain verification.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    /// Unique operation ID
    pub operation_id: String,
    /// Timestamp (ISO 8601)
    pub timestamp: String,
    /// Judgment request hash
    pub request_hash: String,
    /// Judgment result
    pub result: String,
    /// Confidence score
    pub confidence_score: f64,
    /// Previous entry hash (for chain verification)
    pub previous_hash: String,
    /// Hash of this entry
    pub entry_hash: String,
}

/// Errors for audit operations
#[derive(Error, Debug)]
pub enum AuditError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Hash verification failed")]
    HashVerificationFailed,
    #[error("Chain integrity violation at entry {0}")]
    ChainIntegrityViolation(String),
    #[error("Entry not found: {0}")]
    NotFound(String),
}

/// Audit log writer with WAL persistence
pub struct AuditLogWriter {
    entries: Vec<AuditLogEntry>,
    last_hash: String,
}

impl AuditLogWriter {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            last_hash: "genesis".to_string(),
        }
    }

    /// Add an entry to the audit log
    pub fn add_entry(&mut self, entry: AuditLogEntry) -> Result<(), AuditError> {
        // Verify chain continuity
        if entry.previous_hash != self.last_hash {
            return Err(AuditError::ChainIntegrityViolation(entry.operation_id.clone()));
        }

        // Verify entry hash
        let computed_hash = Self::compute_hash(&entry);
        if computed_hash != entry.entry_hash {
            return Err(AuditError::HashVerificationFailed);
        }

        self.last_hash = entry.entry_hash.clone();
        self.entries.push(entry);
        Ok(())
    }

    /// Compute SHA-256 hash of an entry (excluding the entry_hash field itself)
    fn compute_hash(entry: &AuditLogEntry) -> String {
        let mut hasher = Sha256::new();
        hasher.update(&entry.operation_id);
        hasher.update(&entry.timestamp);
        hasher.update(&entry.request_hash);
        hasher.update(&entry.result);
        hasher.update(&entry.confidence_score.to_le_bytes());
        hasher.update(&entry.previous_hash);
        format!("{:x}", hasher.finalize())
    }

    /// Create a new entry with proper hashing
    pub fn create_entry(
        operation_id: String,
        request_hash: String,
        result: String,
        confidence_score: f64,
    ) -> AuditLogEntry {
        let timestamp = chrono_now();
        let previous_hash = "genesis".to_string(); // Would be set from last entry in real impl

        let mut entry = AuditLogEntry {
            operation_id,
            timestamp,
            request_hash,
            result,
            confidence_score,
            previous_hash,
            entry_hash: String::new(),
        };

        entry.entry_hash = Self::compute_hash(&entry);
        entry
    }

    /// Persist entries to WAL (simplified - real impl would use sqlrustgo-storage WAL)
    pub fn persist(&self, path: &std::path::Path) -> Result<(), AuditError> {
        let json = serde_json::to_string(&self.entries)
            .map_err(|e| AuditError::SerializationError(e.to_string()))?;
        std::fs::write(path, json)?;
        Ok(())
    }
}

impl Default for AuditLogWriter {
    fn default() -> Self {
        Self::new()
    }
}

/// Audit log reader for querying
pub struct AuditLogReader {
    entries: Vec<AuditLogEntry>,
}

impl AuditLogReader {
    pub fn load(path: &std::path::Path) -> Result<Self, AuditError> {
        let json = std::fs::read_to_string(path)?;
        let entries: Vec<AuditLogEntry> = serde_json::from_str(&json)
            .map_err(|e| AuditError::SerializationError(e.to_string()))?;
        Ok(Self { entries })
    }

    /// Query by operation ID
    pub fn get_by_operation_id(&self, operation_id: &str) -> Option<&AuditLogEntry> {
        self.entries.iter().find(|e| e.operation_id == operation_id)
    }

    /// Query by time range
    pub fn get_by_time_range(&self, start: &str, end: &str) -> Vec<&AuditLogEntry> {
        self.entries
            .iter()
            .filter(|e| e.timestamp.as_str() >= start && e.timestamp.as_str() <= end)
            .collect()
    }

    /// Verify hash chain integrity
    pub fn verify_chain(&self) -> Result<(), AuditError> {
        let mut expected_previous = "genesis".to_string();

        for entry in &self.entries {
            if entry.previous_hash != expected_previous {
                return Err(AuditError::ChainIntegrityViolation(entry.operation_id.clone()));
            }

            // Verify entry hash
            let mut hasher = Sha256::new();
            hasher.update(&entry.operation_id);
            hasher.update(&entry.timestamp);
            hasher.update(&entry.request_hash);
            hasher.update(&entry.result);
            hasher.update(&entry.confidence_score.to_le_bytes());
            hasher.update(&entry.previous_hash);
            let computed = format!("{:x}", hasher.finalize());

            if computed != entry.entry_hash {
                return Err(AuditError::HashVerificationFailed);
            }

            expected_previous = entry.entry_hash.clone();
        }

        Ok(())
    }

    /// Get all entries
    pub fn all(&self) -> &[AuditLogEntry] {
        &self.entries
    }
}

fn chrono_now() -> String {
    // Simplified - would use chrono::Utc::now().to_rfc3339()
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap();
    format!("{}", duration.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_verify_entry() {
        let entry = AuditLogWriter::create_entry(
            "op-001".to_string(),
            "req-hash-123".to_string(),
            "COMPLIANT".to_string(),
            0.95,
        );

        assert_eq!(entry.operation_id, "op-001");
        assert!(!entry.entry_hash.is_empty());
    }

    #[test]
    fn test_hash_chain() {
        let mut writer = AuditLogWriter::new();

        let entry1 = AuditLogWriter::create_entry(
            "op-001".to_string(),
            "req-hash-1".to_string(),
            "COMPLIANT".to_string(),
            0.95,
        );

        let mut entry2 = AuditLogWriter::create_entry(
            "op-002".to_string(),
            "req-hash-2".to_string(),
            "NON_COMPLIANT".to_string(),
            0.85,
        );
        entry2.previous_hash = entry1.entry_hash.clone();
        entry2.entry_hash = AuditLogWriter::compute_hash(&entry2);

        writer.add_entry(entry1).unwrap();
        writer.add_entry(entry2).unwrap();

        assert_eq!(writer.entries.len(), 2);
    }

    #[test]
    fn test_verify_chain() {
        let mut writer = AuditLogWriter::new();

        let entry1 = AuditLogWriter::create_entry(
            "op-001".to_string(),
            "req-hash-1".to_string(),
            "COMPLIANT".to_string(),
            0.95,
        );

        writer.add_entry(entry1).unwrap();

        let reader = AuditLogReader {
            entries: writer.entries.clone(),
        };

        assert!(reader.verify_chain().is_ok());
    }
}
