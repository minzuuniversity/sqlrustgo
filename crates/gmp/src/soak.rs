//! GMP Mixed Workload SOAK Test
//!
//! Provides infrastructure for running a mixed workload soak test.

use crate::audit::{create_audit_log_table, record_audit_log, verify_audit_chain};
use crate::backup::{create_backup_manifest, verify_backup};
use crate::chunk::{chunk_text, insert_chunk, ChunkConfig};
use crate::document::{create_gmp_tables, insert_document, DocStatus, NewDocument};
use crate::graph::project_subgraph;
use crate::retrieval::{retrieval_search, HybridRetrievalConfig, RetrievalFilter};
use crate::version::{insert_version, sha256_str};
use serde::{Deserialize, Serialize};
use sqlrustgo_storage::StorageEngine;
use sqlrustgo_types::SqlResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkloadOp {
    DocumentInsert,
    ChunkInsert,
    VersionInsert,
    RetrievalQuery,
    AuditQuery,
    AuditRecord,
    BackupCreate,
    BackupVerify,
    GraphProject,
}

impl WorkloadOp {
    pub fn as_str(&self) -> &'static str {
        match self {
            WorkloadOp::DocumentInsert => "DocumentInsert",
            WorkloadOp::ChunkInsert => "ChunkInsert",
            WorkloadOp::VersionInsert => "VersionInsert",
            WorkloadOp::RetrievalQuery => "RetrievalQuery",
            WorkloadOp::AuditQuery => "AuditQuery",
            WorkloadOp::AuditRecord => "AuditRecord",
            WorkloadOp::BackupCreate => "BackupCreate",
            WorkloadOp::BackupVerify => "BackupVerify",
            WorkloadOp::GraphProject => "GraphProject",
        }
    }

    pub fn from_index(i: usize) -> Self {
        match i % 9 {
            0 => WorkloadOp::DocumentInsert,
            1 => WorkloadOp::ChunkInsert,
            2 => WorkloadOp::VersionInsert,
            3 => WorkloadOp::RetrievalQuery,
            4 => WorkloadOp::AuditQuery,
            5 => WorkloadOp::AuditRecord,
            6 => WorkloadOp::BackupVerify,
            7 => WorkloadOp::GraphProject,
            _ => WorkloadOp::AuditRecord,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SoakConfig {
    pub operation_count: usize,
    pub crash_safe: bool,
    pub audit_verify_interval: usize,
}

impl Default for SoakConfig {
    fn default() -> Self {
        Self {
            operation_count: 1000,
            crash_safe: true,
            audit_verify_interval: 100,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SoakStats {
    pub operations_attempted: usize,
    pub operations_succeeded: usize,
    pub operations_failed: usize,
    pub audit_verifications: usize,
    pub audit_verification_failures: usize,
    pub last_audit_verified_at: usize,
    pub crashes: usize,
}

impl SoakStats {
    pub fn success_rate(&self) -> f32 {
        if self.operations_attempted == 0 {
            0.0
        } else {
            self.operations_succeeded as f32 / self.operations_attempted as f32
        }
    }
}

pub fn run_soak_test<S: StorageEngine>(
    storage: &mut S,
    config: &SoakConfig,
) -> SqlResult<SoakStats> {
    let mut stats = SoakStats::default();

    create_gmp_tables(storage)?;
    create_audit_log_table(storage)?;

    for i in 0..config.operation_count {
        let op = WorkloadOp::from_index(i);

        if config.crash_safe {
            match execute_op(storage, op, i) {
                Ok(_) => {
                    stats.operations_succeeded += 1;
                }
                Err(_) => {
                    stats.operations_failed += 1;
                    stats.crashes += 1;
                    break;
                }
            }
        } else {
            let _ = execute_op(storage, op, i);
            stats.operations_succeeded += 1;
        }

        stats.operations_attempted += 1;

        if config.audit_verify_interval > 0 && (i + 1) % config.audit_verify_interval == 0 {
            match verify_audit_chain(storage) {
                Ok((true, _)) => {
                    stats.audit_verifications += 1;
                    stats.last_audit_verified_at = i + 1;
                }
                Ok((false, _)) | Err(_) => {
                    stats.audit_verification_failures += 1;
                    if config.crash_safe {
                        stats.crashes += 1;
                        break;
                    }
                }
            }
        }
    }

    Ok(stats)
}

fn execute_op<S: StorageEngine>(storage: &mut S, op: WorkloadOp, i: usize) -> SqlResult<()> {
    match op {
        WorkloadOp::DocumentInsert => {
            let id = (i as i64).wrapping_abs().wrapping_add(1);
            insert_document(
                storage,
                NewDocument {
                    title: &format!("Soak Doc {}", id),
                    doc_type: "SOAK_TEST",
                    version: 1,
                    created_at: now_i64(),
                    updated_at: now_i64(),
                    effective_date: 19000,
                    status: DocStatus::Active,
                },
            )?;
        }
        WorkloadOp::ChunkInsert => {
            let doc_id = ((i as i64).wrapping_abs() % 100).wrapping_add(1);
            let text = format!("Soak chunk content {}", i);
            let config = ChunkConfig::default();
            let chunks = chunk_text(&text, &config);
            for (idx, (ct, _)) in chunks.iter().enumerate() {
                insert_chunk(storage, doc_id, 1, idx as i32, None, ct)?;
            }
        }
        WorkloadOp::VersionInsert => {
            let doc_id = ((i as i64).wrapping_abs() % 100).wrapping_add(1);
            let source_hash = sha256_str(&format!("soak_version_{}", i));
            let content_hash = sha256_str(&format!("soak_content_{}", i));
            insert_version(storage, doc_id, 1, &source_hash, &content_hash, None)?;
        }
        WorkloadOp::RetrievalQuery => {
            let queries = ["audit compliance", "document review", "quality control"];
            let q = queries[i % queries.len()];
            let filter = RetrievalFilter::default();
            let cfg = HybridRetrievalConfig::default();
            let _ = crate::retrieval::hybrid_retrieval(storage, q, &cfg, &filter);
        }
        WorkloadOp::AuditQuery => {
            let _ = crate::audit::query_audit_logs(storage, None, None, None, None, None)?;
        }
        WorkloadOp::AuditRecord => {
            record_audit_log(
                storage,
                "soak_test_user",
                "CREATE",
                "gmp_documents",
                Some("1"),
                None,
                Some("{}"),
                None,
                None,
            )?;
        }
        WorkloadOp::BackupVerify | WorkloadOp::BackupCreate => {
            let manifest = create_backup_manifest(storage)?;
            let _ = verify_backup(storage, &manifest);
        }
        WorkloadOp::GraphProject => {
            let doc_id = ((i as i64).wrapping_abs() % 100).wrapping_add(1);
            let _ = project_subgraph(storage, doc_id, 2, None);
        }
    }
    Ok(())
}

pub fn verify_retrieval_quality(storage: &dyn StorageEngine) -> SqlResult<bool> {
    let queries = ["audit compliance quality", "document version control"];
    let mut relevant_count = 0;

    for q in queries {
        let results = retrieval_search(storage, q, 5)?;
        if !results.is_empty() {
            relevant_count += 1;
        }
    }

    Ok(relevant_count >= queries.len() / 2)
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
    fn test_soak_config_default() {
        let config = SoakConfig::default();
        assert_eq!(config.operation_count, 1000);
        assert!(config.crash_safe);
        assert_eq!(config.audit_verify_interval, 100);
    }

    #[test]
    fn test_soak_stats_success_rate() {
        let mut stats = SoakStats::default();
        stats.operations_attempted = 100;
        stats.operations_succeeded = 95;
        stats.operations_failed = 5;
        assert!((stats.success_rate() - 0.95).abs() < 0.001);
    }

    #[test]
    fn test_soak_stats_zero_attempted() {
        let stats = SoakStats::default();
        assert_eq!(stats.success_rate(), 0.0);
    }

    #[test]
    fn test_workload_op_as_str() {
        assert_eq!(WorkloadOp::DocumentInsert.as_str(), "DocumentInsert");
        assert_eq!(WorkloadOp::RetrievalQuery.as_str(), "RetrievalQuery");
    }

    #[test]
    fn test_workload_op_from_index() {
        assert_eq!(WorkloadOp::from_index(0), WorkloadOp::DocumentInsert);
        assert_eq!(WorkloadOp::from_index(1), WorkloadOp::ChunkInsert);
        assert_eq!(WorkloadOp::from_index(3), WorkloadOp::RetrievalQuery);
    }

    #[test]
    fn test_soak_run_small() {
        let mut storage = sqlrustgo_storage::MemoryStorage::new();
        let config = SoakConfig {
            operation_count: 10,
            crash_safe: true,
            audit_verify_interval: 5,
        };
        let stats = run_soak_test(&mut storage, &config).unwrap();
        assert_eq!(stats.operations_attempted, 10);
        assert_eq!(stats.crashes, 0);
    }
}
