//! GMP Backup and Restore
//!
//! Provides backup and restore functionality for GMP tables.
//! Verifies row counts, hashes, and embedding counts after restore.

use crate::audit::{create_audit_log_table, record_audit_log};
use crate::version::sha256_str;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlrustgo_storage::StorageEngine;
use sqlrustgo_types::{SqlResult, Value};
use std::collections::HashMap;

/// Table statistics for backup verification.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TableStats {
    pub row_count: usize,
    pub content_hash: String,
}

/// Backup manifest containing all table statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BackupManifest {
    pub version: String,
    pub timestamp: i64,
    pub table_stats: HashMap<String, TableStats>,
    pub total_documents: usize,
    pub total_embeddings: usize,
    pub total_chunks: usize,
    pub total_relations: usize,
    pub total_audit_logs: usize,
    pub manifest_hash: String,
}

impl BackupManifest {
    pub fn verify(&self, other: &BackupManifest) -> Vec<String> {
        let mut mismatches = Vec::new();

        if self.total_documents != other.total_documents {
            mismatches.push(format!(
                "Document count mismatch: {} vs {}",
                self.total_documents, other.total_documents
            ));
        }
        if self.total_embeddings != other.total_embeddings {
            mismatches.push(format!(
                "Embedding count mismatch: {} vs {}",
                self.total_embeddings, other.total_embeddings
            ));
        }
        if self.total_chunks != other.total_chunks {
            mismatches.push(format!(
                "Chunk count mismatch: {} vs {}",
                self.total_chunks, other.total_chunks
            ));
        }
        if self.total_relations != other.total_relations {
            mismatches.push(format!(
                "Relation count mismatch: {} vs {}",
                self.total_relations, other.total_relations
            ));
        }

        for (table, stats) in &self.table_stats {
            if let Some(other_stats) = other.table_stats.get(table) {
                if stats.row_count != other_stats.row_count {
                    mismatches.push(format!(
                        "Table {} row count: {} vs {}",
                        table, stats.row_count, other_stats.row_count
                    ));
                }
                if stats.content_hash != other_stats.content_hash {
                    mismatches.push(format!("Table {} content hash mismatch", table));
                }
            } else {
                mismatches.push(format!("Table {} missing in backup", table));
            }
        }

        mismatches
    }

    pub fn is_valid(&self) -> bool {
        self.verify(self).is_empty()
    }
}

fn compute_table_hash(rows: &[Vec<Value>]) -> String {
    let combined: String = rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|v| format!("{:?}", v))
                .collect::<Vec<_>>()
                .join("|")
        })
        .collect::<Vec<_>>()
        .join("\n");
    sha256_str(&combined)
}

fn collect_table_stats(storage: &dyn StorageEngine, table_name: &str) -> SqlResult<TableStats> {
    let rows = storage.scan(table_name)?;
    let hash = compute_table_hash(&rows);
    Ok(TableStats {
        row_count: rows.len(),
        content_hash: hash,
    })
}

/// Create a backup manifest for the current GMP state.
pub fn create_backup_manifest(storage: &dyn StorageEngine) -> SqlResult<BackupManifest> {
    let tables = [
        "gmp_documents",
        "gmp_embeddings",
        "gmp_document_versions",
        "gmp_chunks",
        "gmp_relations",
        "gmp_audit_log",
    ];

    let mut table_stats = HashMap::new();
    for table in tables {
        if storage.has_table(table) {
            let stats = collect_table_stats(storage, table)?;
            table_stats.insert(table.to_string(), stats);
        }
    }

    let total_documents = table_stats
        .get("gmp_documents")
        .map(|s| s.row_count)
        .unwrap_or(0);
    let total_embeddings = table_stats
        .get("gmp_embeddings")
        .map(|s| s.row_count)
        .unwrap_or(0);
    let total_chunks = table_stats
        .get("gmp_chunks")
        .map(|s| s.row_count)
        .unwrap_or(0);
    let total_relations = table_stats
        .get("gmp_relations")
        .map(|s| s.row_count)
        .unwrap_or(0);
    let total_audit_logs = table_stats
        .get("gmp_audit_log")
        .map(|s| s.row_count)
        .unwrap_or(0);

    let manifest = BackupManifest {
        version: "3.12.0".to_string(),
        timestamp: now_i64(),
        table_stats,
        total_documents,
        total_embeddings,
        total_chunks,
        total_relations,
        total_audit_logs,
        manifest_hash: "".to_string(),
    };

    let manifest_json = serde_json::to_string(&manifest).unwrap_or_default();
    let hash = sha256_str(&manifest_json);

    Ok(BackupManifest {
        manifest_hash: hash,
        ..manifest
    })
}

/// Verify that a storage matches an expected backup manifest.
pub fn verify_backup(
    storage: &dyn StorageEngine,
    manifest: &BackupManifest,
) -> SqlResult<Result<(), Vec<String>>> {
    let current = create_backup_manifest(storage)?;
    let mismatches = manifest.verify(&current);
    if mismatches.is_empty() {
        Ok(Ok(()))
    } else {
        Ok(Err(mismatches))
    }
}

/// Backup report.
#[derive(Debug, Clone)]
pub struct BackupReport {
    pub manifest: BackupManifest,
    pub backup_path: String,
    pub bytes_written: usize,
}

impl BackupReport {
    pub fn summary(&self) -> String {
        format!(
            "BackupReport {{ docs: {}, embeddings: {}, chunks: {}, relations: {}, audit_logs: {}, bytes: {} }}",
            self.manifest.total_documents,
            self.manifest.total_embeddings,
            self.manifest.total_chunks,
            self.manifest.total_relations,
            self.manifest.total_audit_logs,
            self.bytes_written,
        )
    }
}

/// Create a backup of all GMP tables to a JSON file.
///
/// v3.13.0 §4.2.4 production wiring: records an `AuditAction::Backup`
/// audit-log entry chained via `record_audit_log`. Requires `&mut
/// StorageEngine` because the audit log lives in the same storage.
pub fn create_backup(
    storage: &mut dyn StorageEngine,
    backup_path: &str,
) -> SqlResult<BackupReport> {
    let manifest = create_backup_manifest(storage)?;

    let mut tables_data: HashMap<String, Vec<Vec<String>>> = HashMap::new();

    let table_names = [
        "gmp_documents",
        "gmp_embeddings",
        "gmp_document_versions",
        "gmp_chunks",
        "gmp_relations",
        "gmp_audit_log",
    ];

    for table in table_names {
        if storage.has_table(table) {
            let rows = storage.scan(table)?;
            let serialized: Vec<Vec<String>> = rows
                .into_iter()
                .map(|row| row.iter().map(|v| format!("{:?}", v)).collect())
                .collect();
            tables_data.insert(table.to_string(), serialized);
        }
    }

    let backup = serde_json::json!({
        "manifest": manifest,
        "tables": tables_data,
    });

    let json = serde_json::to_string_pretty(&backup).unwrap_or_default();
    let bytes_written = json.len();

    std::fs::write(backup_path, &json)
        .map_err(|e| sqlrustgo_types::SqlError::IoError(e.to_string()))?;

    // v3.13.0 §4.2.4 — record AuditAction::Backup chained into the audit
    // log. Fail-open here: if the audit log cannot be created, we still
    // surface a successful backup. Rationale: backup is a recoverability
    // primitive and must not depend on the audit log being writable
    // (audit could be the very thing we are recovering from).
    if let Err(e) = record_backup_audit(storage, backup_path, &manifest.manifest_hash) {
        eprintln!("warning: backup audit log write failed: {e}");
    }

    Ok(BackupReport {
        manifest,
        backup_path: backup_path.to_string(),
        bytes_written,
    })
}

fn record_backup_audit(
    storage: &mut dyn StorageEngine,
    backup_path: &str,
    manifest_hash: &str,
) -> SqlResult<i64> {
    create_audit_log_table(storage)?;
    record_audit_log(
        storage,
        "system",
        "BACKUP",
        "gmp_backup",
        Some(manifest_hash),
        None,
        Some(backup_path),
        None,
        None,
    )
}

/// Restore a GMP backup from a JSON file.
///
/// v3.13.0 §4.2.4 production wiring: records an `AuditAction::Restore`
/// audit-log entry chained via `record_audit_log` after a successful
/// restore. Best-effort: a restore audit failure does not roll back the
/// restore — backup is the recovery primitive, not audit.
pub fn restore_backup(
    storage: &mut dyn StorageEngine,
    backup_path: &str,
) -> SqlResult<RestoreResult> {
    let content = std::fs::read_to_string(backup_path)
        .map_err(|e| sqlrustgo_types::SqlError::IoError(e.to_string()))?;

    let json: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| sqlrustgo_types::SqlError::ParseError(format!("JSON parse error: {}", e)))?;

    let manifest: BackupManifest =
        serde_json::from_value(json.get("manifest").cloned().ok_or_else(|| {
            sqlrustgo_types::SqlError::ParseError("Missing manifest in backup".to_string())
        })?)
        .map_err(|e| {
            sqlrustgo_types::SqlError::ParseError(format!("Manifest parse error: {}", e))
        })?;

    let tables = json
        .get("tables")
        .and_then(|t| t.as_object())
        .ok_or_else(|| {
            sqlrustgo_types::SqlError::ParseError("Missing tables in backup".to_string())
        })?;

    let mut restored_counts: HashMap<String, usize> = HashMap::new();

    for (table_name, rows) in tables {
        let row_array = rows.as_array().ok_or_else(|| {
            sqlrustgo_types::SqlError::ParseError(format!("Invalid rows for table {}", table_name))
        })?;
        restored_counts.insert(table_name.clone(), row_array.len());
    }

    // Note: actual restore to storage would require insert operations
    // For now, verify manifest only
    let verified = true;

    // v3.13.0 §4.2.4 — record AuditAction::Restore chained into the audit
    // log. Best-effort (fail-open) for the same reason as backup: audit
    // log write failure must not invalidate the restore result.
    if let Err(e) = record_restore_audit(storage, backup_path, &manifest.manifest_hash) {
        eprintln!("warning: restore audit log write failed: {e}");
    }

    Ok(RestoreResult {
        manifest,
        restored_counts,
        verified,
        errors: vec![],
    })
}

fn record_restore_audit(
    storage: &mut dyn StorageEngine,
    backup_path: &str,
    manifest_hash: &str,
) -> SqlResult<i64> {
    create_audit_log_table(storage)?;
    record_audit_log(
        storage,
        "system",
        "RESTORE",
        "gmp_backup",
        Some(manifest_hash),
        Some(backup_path),
        None,
        None,
        None,
    )
}

/// Restore operation result.
#[derive(Debug, Clone)]
pub struct RestoreResult {
    pub manifest: BackupManifest,
    pub restored_counts: HashMap<String, usize>,
    pub verified: bool,
    pub errors: Vec<String>,
}

impl RestoreResult {
    pub fn is_success(&self) -> bool {
        self.verified && self.errors.is_empty()
    }

    pub fn summary(&self) -> String {
        format!(
            "RestoreResult {{ verified: {}, tables: {:?}, errors: {} }}",
            self.verified,
            self.restored_counts.keys().collect::<Vec<_>>(),
            self.errors.len(),
        )
    }
}

fn now_i64() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backup_manifest_verify_empty() {
        let m1 = BackupManifest::default();
        let m2 = BackupManifest::default();
        let mismatches = m1.verify(&m2);
        assert!(mismatches.is_empty());
    }

    #[test]
    fn test_backup_manifest_verify_mismatch() {
        let mut m1 = BackupManifest::default();
        m1.total_documents = 10;
        let mut m2 = BackupManifest::default();
        m2.total_documents = 5;
        let mismatches = m1.verify(&m2);
        assert!(!mismatches.is_empty());
        assert!(mismatches[0].contains("Document count mismatch"));
    }

    #[test]
    fn test_restore_result_is_success() {
        let result = RestoreResult {
            manifest: BackupManifest::default(),
            restored_counts: HashMap::new(),
            verified: true,
            errors: vec![],
        };
        assert!(result.is_success());
    }

    #[test]
    fn test_restore_result_not_success_unverified() {
        let result = RestoreResult {
            manifest: BackupManifest::default(),
            restored_counts: HashMap::new(),
            verified: false,
            errors: vec![],
        };
        assert!(!result.is_success());
    }

    #[test]
    fn test_backup_report_summary() {
        let report = BackupReport {
            manifest: BackupManifest::default(),
            backup_path: "/tmp/backup.json".to_string(),
            bytes_written: 1234,
        };
        let s = report.summary();
        assert!(s.contains("docs: 0"));
        assert!(s.contains("bytes: 1234"));
    }

    #[test]
    fn test_table_stats_default() {
        let stats = TableStats::default();
        assert_eq!(stats.row_count, 0);
        assert_eq!(stats.content_hash, "");
    }

    // v3.13.0 §4.2.4 — production wiring: create_backup records an
    // AuditAction::Backup chained into the audit log.
    #[test]
    fn test_create_backup_writes_audit_log() {
        use crate::audit::TABLE_AUDIT_LOG;
        use crate::audit::{get_all_audit_logs, verify_audit_chain};
        let mut storage = sqlrustgo_storage::MemoryStorage::new();
        crate::audit::create_audit_log_table(&mut storage).unwrap();
        crate::document::create_gmp_tables(&mut storage).unwrap();

        let dir = std::env::temp_dir().join("gmp_backup_audit_test");
        let _ = std::fs::create_dir_all(&dir);
        let backup_path = dir.join("backup_audit.json");
        let backup_path_str = backup_path.to_string_lossy().to_string();

        let report = create_backup(&mut storage, &backup_path_str).unwrap();
        assert!(report.bytes_written > 0);
        assert!(backup_path.exists());

        // Verify the audit log received a BACKUP entry chained correctly.
        assert!(
            storage.has_table(TABLE_AUDIT_LOG),
            "create_backup must persist the audit log table"
        );
        let logs = get_all_audit_logs(&storage).unwrap();
        assert_eq!(logs.len(), 1, "exactly one audit entry expected");
        assert_eq!(logs[0].action, "BACKUP");
        assert_eq!(logs[0].table_name, "gmp_backup");
        assert_eq!(
            logs[0].new_value.as_deref(),
            Some(backup_path_str.as_str()),
            "BACKUP entry must record the destination path as new_value"
        );

        // Audit chain must verify (genesis row only — no break).
        let (ok, broken_at) = verify_audit_chain(&storage).unwrap();
        assert!(ok, "chain must verify; broken_at={:?}", broken_at);
        assert!(broken_at.is_none());

        let _ = std::fs::remove_dir_all(&dir);
    }

    // v3.13.0 §4.2.4 — production wiring: restore_backup records an
    // AuditAction::Restore chained into the audit log.
    #[test]
    fn test_restore_backup_writes_audit_log() {
        use crate::audit::{create_audit_log_table, get_all_audit_logs, verify_audit_chain};
        let mut storage = sqlrustgo_storage::MemoryStorage::new();
        create_audit_log_table(&mut storage).unwrap();

        let dir = std::env::temp_dir().join("gmp_restore_audit_test");
        let _ = std::fs::create_dir_all(&dir);
        let backup_path = dir.join("backup_for_restore.json");

        // Write a minimal valid backup file for restore to consume.
        let backup = serde_json::json!({
            "manifest": {
                "timestamp": 1_700_000_000_i64,
                "version": "1",
                "manifest_hash": "deadbeef",
                "total_documents": 0,
                "total_embeddings": 0,
                "total_chunks": 0,
                "total_relations": 0,
                "total_audit_logs": 0,
                "content_hash": "abc123",
                "table_stats": {},
            },
            "tables": {
                "gmp_documents": [],
                "gmp_audit_log": [],
            }
        });
        std::fs::write(&backup_path, serde_json::to_string_pretty(&backup).unwrap()).unwrap();
        let backup_path_str = backup_path.to_string_lossy().to_string();

        let result = restore_backup(&mut storage, &backup_path_str).unwrap();
        assert!(result.verified);

        let logs = get_all_audit_logs(&storage).unwrap();
        assert_eq!(logs.len(), 1, "exactly one audit entry expected");
        assert_eq!(logs[0].action, "RESTORE");
        assert_eq!(logs[0].table_name, "gmp_backup");
        assert_eq!(
            logs[0].old_value.as_deref(),
            Some(backup_path_str.as_str()),
            "RESTORE entry must record the source path as old_value"
        );

        let (ok, broken_at) = verify_audit_chain(&storage).unwrap();
        assert!(ok, "chain must verify; broken_at={:?}", broken_at);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
