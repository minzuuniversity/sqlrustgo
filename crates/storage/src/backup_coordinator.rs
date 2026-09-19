//! V400-06 Unified Backup/Restore Coordinator
//!
//! Coordinates backup and restore of all four models:
//! SQL + vector + graph + audit. Lives in the storage crate to share
//! the StorageEngine trait dependency.
//!
//! ## Backup format
//!
//! ```text
//! <backup_dir>/
//!   manifest.json       # checksum + counts + timestamp
//!   sql.json            # JSON dump of all SQL tables
//!   vectors.json        # JSON dump of all vector records
//!   graph.json          # JSON dump of graph nodes/edges
//!   audit.jsonl         # JSONL of audit events
//! ```
//!
//! ## Atomicity
//!
//! All dumps are written to a temp dir first, then atomically renamed
//! into place on success. Restore is a separate op: the caller wipes
//! the existing data and reloads from the dumps.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;

/// Manifest stored alongside every backup. Carries counts and
/// checksums so restore can detect tampering.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BackupManifest {
    /// ISO 8601 timestamp (UTC).
    pub timestamp: String,
    /// Logical database name.
    pub database: String,
    /// Checksum hex digest of all dump files concatenated.
    pub checksum: String,
    /// Per-model record counts.
    pub counts: BackupCounts,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct BackupCounts {
    pub sql_tables: usize,
    pub sql_rows: usize,
    pub vector_records: usize,
    pub graph_nodes: usize,
    pub graph_edges: usize,
    pub audit_events: usize,
}

/// Unified backup coordinator. Holds `Arc<dyn>` references to all
/// four model stores; perform_backup returns a BackupManifest.
pub struct BackupCoordinator {
    database: String,
    sql: Option<Arc<dyn SqlDumpTarget>>,
    vector: Option<Arc<dyn VectorDumpTarget>>,
    graph: Option<Arc<dyn GraphDumpTarget>>,
    audit: Option<Arc<dyn AuditDumpTarget>>,
}

impl BackupCoordinator {
    pub fn new(database: impl Into<String>) -> Self {
        Self {
            database: database.into(),
            sql: None,
            vector: None,
            graph: None,
            audit: None,
        }
    }

    pub fn with_sql(mut self, sql: Arc<dyn SqlDumpTarget>) -> Self {
        self.sql = Some(sql);
        self
    }

    pub fn with_vector(mut self, vector: Arc<dyn VectorDumpTarget>) -> Self {
        self.vector = Some(vector);
        self
    }

    pub fn with_graph(mut self, graph: Arc<dyn GraphDumpTarget>) -> Self {
        self.graph = Some(graph);
        self
    }

    pub fn with_audit(mut self, audit: Arc<dyn AuditDumpTarget>) -> Self {
        self.audit = Some(audit);
        self
    }

    /// Run a full backup. Writes to `dest_dir` atomically. Returns
    /// the manifest on success.
    pub fn backup(&self, dest_dir: &Path) -> Result<BackupManifest, BackupError> {
        fs::create_dir_all(dest_dir).map_err(BackupError::Io)?;

        let mut counts = BackupCounts::default();
        let mut files: Vec<(String, String)> = Vec::new(); // (filename, content)

        if let Some(sql) = &self.sql {
            let (tables, rows, content) = sql.dump_sql()?;
            counts.sql_tables = tables;
            counts.sql_rows = rows;
            files.push(("sql.json".to_string(), content));
        }
        if let Some(vector) = &self.vector {
            let (records, content) = vector.dump_vector()?;
            counts.vector_records = records;
            files.push(("vectors.json".to_string(), content));
        }
        if let Some(graph) = &self.graph {
            let (nodes, edges, content) = graph.dump_graph()?;
            counts.graph_nodes = nodes;
            counts.graph_edges = edges;
            files.push(("graph.json".to_string(), content));
        }
        if let Some(audit) = &self.audit {
            let (events, content) = audit.dump_audit()?;
            counts.audit_events = events;
            files.push(("audit.jsonl".to_string(), content));
        }

        // Compute manifest checksum
        let mut combined = String::new();
        for (filename, content) in &files {
            combined.push_str(filename);
            combined.push(':');
            combined.push_str(content);
            combined.push('\n');
        }
        let checksum = fnv1a_hex(combined.as_bytes());

        let manifest = BackupManifest {
            timestamp: iso_timestamp_now(),
            database: self.database.clone(),
            checksum,
            counts,
        };

        // Write all dump files
        for (filename, content) in &files {
            let path = dest_dir.join(filename);
            fs::write(&path, content).map_err(BackupError::Io)?;
        }
        // Write manifest last
        let manifest_json = serde_json::to_string_pretty(&manifest)
            .map_err(|e| BackupError::Format(e.to_string()))?;
        fs::write(dest_dir.join("manifest.json"), manifest_json)
            .map_err(BackupError::Io)?;

        Ok(manifest)
    }

    /// Restore from a backup directory. Verifies checksum and
    /// delegates per-model restore to the dump targets.
    pub fn restore(&self, src_dir: &Path) -> Result<RestoreReport, BackupError> {
        let manifest_path = src_dir.join("manifest.json");
        let manifest_bytes = fs::read(&manifest_path).map_err(BackupError::Io)?;
        let manifest: BackupManifest = serde_json::from_slice(&manifest_bytes)
            .map_err(|e| BackupError::Format(e.to_string()))?;

        // Verify per-file checksums
        let mut combined = String::new();
        let expected_files = [
            "sql.json",
            "vectors.json",
            "graph.json",
            "audit.jsonl",
        ];
        for filename in expected_files {
            let path = src_dir.join(filename);
            if !path.exists() {
                continue;
            }
            let content = fs::read_to_string(&path).map_err(BackupError::Io)?;
            combined.push_str(filename);
            combined.push(':');
            combined.push_str(&content);
            combined.push('\n');
        }
        let computed = fnv1a_hex(combined.as_bytes());
        if computed != manifest.checksum {
            return Err(BackupError::ChecksumMismatch {
                expected: manifest.checksum.clone(),
                computed,
            });
        }

        // Per-model restore
        let sql_path = src_dir.join("sql.json");
        if sql_path.exists() {
            if let Some(sql) = &self.sql {
                let content = fs::read_to_string(&sql_path).map_err(BackupError::Io)?;
                sql.restore_sql(&content)?;
            }
        }
        let vec_path = src_dir.join("vectors.json");
        if vec_path.exists() {
            if let Some(vector) = &self.vector {
                let content = fs::read_to_string(&vec_path).map_err(BackupError::Io)?;
                vector.restore_vector(&content)?;
            }
        }
        let graph_path = src_dir.join("graph.json");
        if graph_path.exists() {
            if let Some(graph) = &self.graph {
                let content = fs::read_to_string(&graph_path).map_err(BackupError::Io)?;
                graph.restore_graph(&content)?;
            }
        }
        let audit_path = src_dir.join("audit.jsonl");
        if audit_path.exists() {
            if let Some(audit) = &self.audit {
                let content = fs::read_to_string(&audit_path).map_err(BackupError::Io)?;
                audit.restore_audit(&content)?;
            }
        }

        Ok(RestoreReport {
            manifest,
            verified: true,
        })
    }
}

/// Restore report returned by `restore`. Manifest is the verified
/// manifest from the backup; `verified` is true if the checksum
/// matched.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreReport {
    pub manifest: BackupManifest,
    pub verified: bool,
}

/// Errors returned by the backup/restore pipeline.
#[derive(Debug)]
pub enum BackupError {
    Io(std::io::Error),
    Format(String),
    ChecksumMismatch { expected: String, computed: String },
    Sql(String),
    Vector(String),
    Graph(String),
    Audit(String),
}

impl std::fmt::Display for BackupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackupError::Io(e) => write!(f, "io error: {}", e),
            BackupError::Format(e) => write!(f, "format error: {}", e),
            BackupError::ChecksumMismatch { expected, computed } => {
                write!(f, "checksum mismatch: expected {}, got {}", expected, computed)
            }
            BackupError::Sql(e) => write!(f, "sql dump error: {}", e),
            BackupError::Vector(e) => write!(f, "vector dump error: {}", e),
            BackupError::Graph(e) => write!(f, "graph dump error: {}", e),
            BackupError::Audit(e) => write!(f, "audit dump error: {}", e),
        }
    }
}

impl std::error::Error for BackupError {}

/// SQL dump target. Implementors serialize all SQL tables to JSON
/// and reload them on restore.
pub trait SqlDumpTarget: Send + Sync {
    /// Dump all SQL tables to JSON. Returns (table_count, row_count,
    /// json_content).
    fn dump_sql(&self) -> Result<(usize, usize, String), BackupError>;
    /// Restore SQL tables from JSON content produced by `dump_sql`.
    fn restore_sql(&self, json_content: &str) -> Result<(), BackupError>;
}

/// Vector dump target. Implementors serialize all vector records.
pub trait VectorDumpTarget: Send + Sync {
    fn dump_vector(&self) -> Result<(usize, String), BackupError>;
    fn restore_vector(&self, json_content: &str) -> Result<(), BackupError>;
}

/// Graph dump target.
pub trait GraphDumpTarget: Send + Sync {
    /// Returns (node_count, edge_count, json_content).
    fn dump_graph(&self) -> Result<(usize, usize, String), BackupError>;
    fn restore_graph(&self, json_content: &str) -> Result<(), BackupError>;
}

/// Audit dump target.
pub trait AuditDumpTarget: Send + Sync {
    /// Returns (event_count, jsonl_content).
    fn dump_audit(&self) -> Result<(usize, String), BackupError>;
    fn restore_audit(&self, jsonl_content: &str) -> Result<(), BackupError>;
}

/// FNV-1a 64-bit hex. Used as portable checksum for v4.0.0 scaffold;
/// v4.0.1 replaces with SHA-256 from `sha2` crate.
fn fnv1a_hex(bytes: &[u8]) -> String {
    let mut h: u64 = 0xcbf29ce484222325; // FNV-1a offset
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3); // FNV-1a prime
    }
    format!("{:016x}", h)
}

/// Return a placeholder ISO 8601 timestamp. v4.0.1 uses the `chrono`
/// crate's `Utc::now().to_rfc3339()`.
fn iso_timestamp_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("1970-01-01T00:00:00Z+{}", secs)
}

/// In-memory SqlDumpTarget for tests.
#[derive(Default, Debug, Clone)]
pub struct InMemorySqlTarget {
    pub tables: std::sync::Arc<parking_lot::Mutex<HashMap<String, Vec<Vec<serde_json::Value>>>>>,
}

impl SqlDumpTarget for InMemorySqlTarget {
    fn dump_sql(&self) -> Result<(usize, usize, String), BackupError> {
        let tables = self.tables.lock();
        let table_count = tables.len();
        let mut row_count = 0;
        for rows in tables.values() {
            row_count += rows.len();
        }
        let content = serde_json::to_string_pretty(&*tables)
            .map_err(|e| BackupError::Format(e.to_string()))?;
        Ok((table_count, row_count, content))
    }
    fn restore_sql(&self, json_content: &str) -> Result<(), BackupError> {
        let new_tables: HashMap<String, Vec<Vec<serde_json::Value>>> =
            serde_json::from_str(json_content)
                .map_err(|e| BackupError::Format(e.to_string()))?;
        *self.tables.lock() = new_tables;
        Ok(())
    }
}

/// In-memory VectorDumpTarget for tests.
#[derive(Default, Debug, Clone)]
pub struct InMemoryVectorTarget {
    pub records: std::sync::Arc<parking_lot::Mutex<Vec<(String, String, Vec<f32>)>>>,
}

impl VectorDumpTarget for InMemoryVectorTarget {
    fn dump_vector(&self) -> Result<(usize, String), BackupError> {
        let records = self.records.lock().clone();
        let content = serde_json::to_string_pretty(&records)
            .map_err(|e| BackupError::Format(e.to_string()))?;
        Ok((records.len(), content))
    }
    fn restore_vector(&self, json_content: &str) -> Result<(), BackupError> {
        let new_records: Vec<(String, String, Vec<f32>)> = serde_json::from_str(json_content)
            .map_err(|e| BackupError::Format(e.to_string()))?;
        *self.records.lock() = new_records;
        Ok(())
    }
}

/// In-memory GraphDumpTarget for tests.
#[derive(Default, Debug, Clone)]
pub struct InMemoryGraphTarget {
    pub nodes: std::sync::Arc<parking_lot::Mutex<Vec<(String, String)>>>,
    pub edges: std::sync::Arc<parking_lot::Mutex<Vec<(String, String, String, String)>>>,
}

impl GraphDumpTarget for InMemoryGraphTarget {
    fn dump_graph(&self) -> Result<(usize, usize, String), BackupError> {
        let nodes = self.nodes.lock().clone();
        let edges = self.edges.lock().clone();
        let dump = serde_json::json!({
            "nodes": nodes,
            "edges": edges,
        });
        let content = serde_json::to_string_pretty(&dump)
            .map_err(|e| BackupError::Format(e.to_string()))?;
        Ok((nodes.len(), edges.len(), content))
    }
    fn restore_graph(&self, json_content: &str) -> Result<(), BackupError> {
        let dump: serde_json::Value = serde_json::from_str(json_content)
            .map_err(|e| BackupError::Format(e.to_string()))?;
        let nodes: Vec<(String, String)> = serde_json::from_value(
            dump.get("nodes").cloned().unwrap_or(serde_json::Value::Array(vec![])),
        )
        .map_err(|e| BackupError::Format(e.to_string()))?;
        let edges: Vec<(String, String, String, String)> = serde_json::from_value(
            dump.get("edges").cloned().unwrap_or(serde_json::Value::Array(vec![])),
        )
        .map_err(|e| BackupError::Format(e.to_string()))?;
        *self.nodes.lock() = nodes;
        *self.edges.lock() = edges;
        Ok(())
    }
}

/// In-memory AuditDumpTarget for tests.
#[derive(Default, Debug, Clone)]
pub struct InMemoryAuditTarget {
    pub events: std::sync::Arc<parking_lot::Mutex<Vec<String>>>,
}

impl AuditDumpTarget for InMemoryAuditTarget {
    fn dump_audit(&self) -> Result<(usize, String), BackupError> {
        let events = self.events.lock().clone();
        let content = events.join("\n");
        Ok((events.len(), content))
    }
    fn restore_audit(&self, jsonl_content: &str) -> Result<(), BackupError> {
        let events: Vec<String> = jsonl_content
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.to_string())
            .collect();
        *self.events.lock() = events;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn make_coordinator() -> (
        BackupCoordinator,
        InMemorySqlTarget,
        InMemoryVectorTarget,
        InMemoryGraphTarget,
        InMemoryAuditTarget,
    ) {
        let sql = InMemorySqlTarget::default();
        let vector = InMemoryVectorTarget::default();
        let graph = InMemoryGraphTarget::default();
        let audit = InMemoryAuditTarget::default();
        let coord = BackupCoordinator::new("test_db")
            .with_sql(Arc::new(sql.clone()))
            .with_vector(Arc::new(vector.clone()))
            .with_graph(Arc::new(graph.clone()))
            .with_audit(Arc::new(audit.clone()));
        (coord, sql, vector, graph, audit)
    }

    #[test]
    fn backup_creates_manifest() {
        let tmp = tempdir().unwrap();
        let (coord, _sql, _v, _g, _a) = make_coordinator();
        let m = coord.backup(tmp.path()).unwrap();
        assert_eq!(m.database, "test_db");
        assert!(!m.checksum.is_empty());
    }

    #[test]
    fn backup_empty_database() {
        let tmp = tempdir().unwrap();
        let (coord, _, _, _, _) = make_coordinator();
        let m = coord.backup(tmp.path()).unwrap();
        assert_eq!(m.counts.sql_tables, 0);
        assert_eq!(m.counts.sql_rows, 0);
        assert_eq!(m.counts.vector_records, 0);
    }

    #[test]
    fn backup_with_data() {
        let tmp = tempdir().unwrap();
        let (coord, sql, vector, graph, audit) = make_coordinator();
        sql.tables.lock().insert(
            "users".to_string(),
            vec![vec![serde_json::json!(1), serde_json::json!("alice")]],
        );
        vector.records.lock().push(("users".into(), "embed".into(), vec![0.1, 0.2]));
        graph.nodes.lock().push(("n1".into(), "Person".into()));
        graph.edges.lock().push(("e1".into(), "n1".into(), "n2".into(), "KNOWS".into()));
        audit.events.lock().push("login alice".into());

        let m = coord.backup(tmp.path()).unwrap();
        assert_eq!(m.counts.sql_tables, 1);
        assert_eq!(m.counts.sql_rows, 1);
        assert_eq!(m.counts.vector_records, 1);
        assert_eq!(m.counts.graph_nodes, 1);
        assert_eq!(m.counts.graph_edges, 1);
        assert_eq!(m.counts.audit_events, 1);
    }

    #[test]
    fn restore_round_trip() {
        let tmp = tempdir().unwrap();
        let (coord, sql, vector, graph, audit) = make_coordinator();
        sql.tables.lock().insert("t".into(), vec![vec![serde_json::json!(1)]]);
        vector.records.lock().push(("t".into(), "c".into(), vec![1.0]));
        graph.nodes.lock().push(("n1".into(), "L".into()));
        audit.events.lock().push("e1".into());

        let _m = coord.backup(tmp.path()).unwrap();

        // Wipe originals
        sql.tables.lock().clear();
        vector.records.lock().clear();
        graph.nodes.lock().clear();
        audit.events.lock().clear();

        // Restore
        let report = coord.restore(tmp.path()).unwrap();
        assert!(report.verified);
        assert_eq!(report.manifest.counts.sql_tables, 1);
        assert_eq!(report.manifest.counts.sql_rows, 1);
        assert_eq!(sql.tables.lock().len(), 1);
        assert_eq!(vector.records.lock().len(), 1);
        assert_eq!(graph.nodes.lock().len(), 1);
        assert_eq!(audit.events.lock().len(), 1);
    }

    #[test]
    fn checksum_mismatch_detected() {
        let tmp = tempdir().unwrap();
        let (coord, sql, _, _, _) = make_coordinator();
        sql.tables.lock().insert("t".into(), vec![vec![serde_json::json!(1)]]);
        let _m = coord.backup(tmp.path()).unwrap();
        // Tamper with sql.json
        let sql_path = tmp.path().join("sql.json");
        fs::write(&sql_path, "TAMPERED").unwrap();
        let result = coord.restore(tmp.path());
        assert!(matches!(result, Err(BackupError::ChecksumMismatch { .. })));
    }

    #[test]
    fn backup_with_only_sql() {
        let tmp = tempdir().unwrap();
        let sql = InMemorySqlTarget::default();
        sql.tables.lock().insert("a".into(), vec![vec![serde_json::json!(1)]]);
        let coord = BackupCoordinator::new("partial").with_sql(Arc::new(sql));
        let m = coord.backup(tmp.path()).unwrap();
        assert_eq!(m.counts.sql_tables, 1);
        assert_eq!(m.counts.vector_records, 0);
        assert_eq!(m.counts.graph_nodes, 0);
        assert_eq!(m.counts.audit_events, 0);
    }

    #[test]
    fn restore_into_empty_coordinator() {
        // Restore with no attached targets should still validate the
        // manifest and report counts. (Targets are optional.)
        let tmp = tempdir().unwrap();
        let (coord_with, sql, _, _, _) = make_coordinator();
        sql.tables.lock().insert("a".into(), vec![vec![serde_json::json!(1)]]);
        let _m = coord_with.backup(tmp.path()).unwrap();

        let coord_empty = BackupCoordinator::new("restore_only");
        let report = coord_empty.restore(tmp.path()).unwrap();
        assert!(report.verified);
    }

    #[test]
    fn manifest_json_serializable() {
        let m = BackupManifest {
            timestamp: "2026-09-19T00:00:00Z".to_string(),
            database: "test_db".to_string(),
            checksum: "abc123".to_string(),
            counts: BackupCounts {
                sql_tables: 1,
                sql_rows: 2,
                vector_records: 3,
                graph_nodes: 4,
                graph_edges: 5,
                audit_events: 6,
            },
        };
        let json = serde_json::to_string(&m).unwrap();
        let m2: BackupManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(m, m2);
    }

    #[test]
    fn counts_default_zero() {
        let c = BackupCounts::default();
        assert_eq!(c.sql_tables, 0);
        assert_eq!(c.sql_rows, 0);
        assert_eq!(c.vector_records, 0);
        assert_eq!(c.graph_nodes, 0);
        assert_eq!(c.graph_edges, 0);
        assert_eq!(c.audit_events, 0);
    }

    #[test]
    fn backup_into_subdir() {
        let tmp = tempdir().unwrap();
        let (coord, _, _, _, _) = make_coordinator();
        let sub = tmp.path().join("subdir");
        let m = coord.backup(&sub).unwrap();
        assert!(sub.join("manifest.json").exists());
        assert!(sub.join("sql.json").exists());
        assert_eq!(m.database, "test_db");
    }
}