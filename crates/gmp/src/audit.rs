//! GMP Audit Logging
//!
//! Provides audit logging functionality for GMP document management.
//! All CREATE, UPDATE, DELETE operations on GMP tables are tracked
//! with tamper-evident SHA-256 hash chains.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlrustgo_storage::{ColumnDefinition, StorageEngine};
use sqlrustgo_types::{SqlResult, Value};

/// Audit log action types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditAction {
    Create,
    Update,
    Delete,
    // Compliance operation variants (V312-53 followup #4231).
    // These mirror the operations that must be audit-logged per
    // GMP access-control policy (acl.rs role×op matrix).
    Import,
    Export,
    Approve,
    Review,
    Backup,
    Restore,
}

impl AuditAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuditAction::Create => "CREATE",
            AuditAction::Update => "UPDATE",
            AuditAction::Delete => "DELETE",
            AuditAction::Import => "IMPORT",
            AuditAction::Export => "EXPORT",
            AuditAction::Approve => "APPROVE",
            AuditAction::Review => "REVIEW",
            AuditAction::Backup => "BACKUP",
            AuditAction::Restore => "RESTORE",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "CREATE" => Some(AuditAction::Create),
            "UPDATE" => Some(AuditAction::Update),
            "DELETE" => Some(AuditAction::Delete),
            "IMPORT" => Some(AuditAction::Import),
            "EXPORT" => Some(AuditAction::Export),
            "APPROVE" => Some(AuditAction::Approve),
            "REVIEW" => Some(AuditAction::Review),
            "BACKUP" => Some(AuditAction::Backup),
            "RESTORE" => Some(AuditAction::Restore),
            _ => None,
        }
    }
}

/// Audit log entry representing a single audit record.
/// The `previous_hash` and `event_hash` fields form a tamper-evident chain:
/// - `previous_hash` = SHA-256 of the previous row's content (NULL for genesis row)
/// - `event_hash` = SHA-256 of this row's content (excludes event_hash itself)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: i64,
    pub timestamp: i64,
    pub user_id: String,
    pub action: String,
    pub table_name: String,
    pub record_id: Option<String>,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub ip_address: Option<String>,
    pub session_id: Option<String>,
    /// Hash of the previous audit row's content (NULL for genesis row)
    pub previous_hash: Option<String>,
    /// SHA-256 of this row's content (excludes event_hash itself)
    pub event_hash: String,
}

impl AuditLog {
    /// Parse an AuditLog from a database row.
    /// Expected column order: id, timestamp, user_id, action, table_name,
    /// record_id, old_value, new_value, ip_address, session_id, previous_hash, event_hash
    pub fn from_row(row: &[Value]) -> Option<Self> {
        let id = match &row.get(0)? {
            Value::Integer(n) => *n,
            _ => return None,
        };
        let timestamp = match &row.get(1)? {
            Value::Integer(n) => *n,
            _ => return None,
        };
        let user_id = match &row.get(2)? {
            Value::Text(s) => s.clone(),
            _ => return None,
        };
        let action = match &row.get(3)? {
            Value::Text(s) => s.clone(),
            _ => return None,
        };
        let table_name = match &row.get(4)? {
            Value::Text(s) => s.clone(),
            _ => return None,
        };
        let record_id = match &row.get(5)? {
            Value::Text(s) => Some(s.clone()),
            Value::Null => None,
            _ => return None,
        };
        let old_value = match &row.get(6)? {
            Value::Text(s) => Some(s.clone()),
            Value::Null => None,
            _ => return None,
        };
        let new_value = match &row.get(7)? {
            Value::Text(s) => Some(s.clone()),
            Value::Null => None,
            _ => return None,
        };
        let ip_address = match &row.get(8)? {
            Value::Text(s) => Some(s.clone()),
            Value::Null => None,
            _ => return None,
        };
        let session_id = match &row.get(9)? {
            Value::Text(s) => Some(s.clone()),
            Value::Null => None,
            _ => return None,
        };
        let previous_hash = match &row.get(10)? {
            Value::Text(s) => Some(s.clone()),
            Value::Null => None,
            _ => return None,
        };
        let event_hash = match &row.get(11)? {
            Value::Text(s) => s.clone(),
            _ => return None,
        };

        Some(AuditLog {
            id,
            timestamp,
            user_id,
            action,
            table_name,
            record_id,
            old_value,
            new_value,
            ip_address,
            session_id,
            previous_hash,
            event_hash,
        })
    }

    /// Convert AuditLog to a database row.
    pub fn to_row(&self) -> Vec<Value> {
        vec![
            Value::Integer(self.id),
            Value::Integer(self.timestamp),
            Value::Text(self.user_id.clone()),
            Value::Text(self.action.clone()),
            Value::Text(self.table_name.clone()),
            self.record_id
                .as_ref()
                .map(|s| Value::Text(s.clone()))
                .unwrap_or(Value::Null),
            self.old_value
                .as_ref()
                .map(|s| Value::Text(s.clone()))
                .unwrap_or(Value::Null),
            self.new_value
                .as_ref()
                .map(|s| Value::Text(s.clone()))
                .unwrap_or(Value::Null),
            self.ip_address
                .as_ref()
                .map(|s| Value::Text(s.clone()))
                .unwrap_or(Value::Null),
            self.session_id
                .as_ref()
                .map(|s| Value::Text(s.clone()))
                .unwrap_or(Value::Null),
            self.previous_hash
                .as_ref()
                .map(|s| Value::Text(s.clone()))
                .unwrap_or(Value::Null),
            Value::Text(self.event_hash.clone()),
        ]
    }

    /// Verify that this row's stored event_hash matches the computed SHA-256
    /// of this row's content (excluding event_hash itself).
    pub fn verify_event_hash(&self) -> bool {
        self.event_hash == compute_event_hash(self)
    }
}

/// Compute SHA-256 of an audit log row's content (excludes event_hash field itself).
/// Used both for storing event_hash and for verification.
fn compute_event_hash(log: &AuditLog) -> String {
    let data = format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        log.id,
        log.timestamp,
        log.user_id,
        log.action,
        log.table_name,
        log.record_id.as_deref().unwrap_or(""),
        log.old_value.as_deref().unwrap_or(""),
        log.new_value.as_deref().unwrap_or(""),
        log.ip_address.as_deref().unwrap_or(""),
        log.session_id.as_deref().unwrap_or(""),
        log.previous_hash.as_deref().unwrap_or(""),
    );
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Genesis event hash seed — used as previous_hash sentinel for the first row.
pub const GENESIS_PREVIOUS_HASH: Option<String> = None;

/// GMP audit log table name
pub const TABLE_AUDIT_LOG: &str = "gmp_audit_log";

/// SQL to create the audit log table with hash chain columns.
/// Uses IF NOT EXISTS for idempotent creation.
pub const CREATE_AUDIT_LOG_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS gmp_audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp INTEGER NOT NULL,
    user_id TEXT NOT NULL,
    action TEXT NOT NULL,
    table_name TEXT NOT NULL,
    record_id TEXT,
    old_value TEXT,
    new_value TEXT,
    ip_address TEXT,
    session_id TEXT,
    previous_hash TEXT,
    event_hash TEXT NOT NULL
)
"#;

/// Create the audit log table if it does not exist.
pub fn create_audit_log_table(storage: &mut dyn StorageEngine) -> SqlResult<()> {
    if !storage.has_table(TABLE_AUDIT_LOG) {
        let columns = vec![
            ColumnDefinition {
                name: "id".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: true,
                char_max_length: None,
                collation: None,
                default_value: None,
            },
            ColumnDefinition {
                name: "timestamp".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: false,
                char_max_length: None,
                collation: None,
                default_value: None,
            },
            ColumnDefinition {
                name: "user_id".to_string(),
                data_type: "TEXT".to_string(),
                nullable: false,
                primary_key: false,
                char_max_length: None,
                collation: None,
                default_value: None,
            },
            ColumnDefinition {
                name: "action".to_string(),
                data_type: "TEXT".to_string(),
                nullable: false,
                primary_key: false,
                char_max_length: None,
                collation: None,
                default_value: None,
            },
            ColumnDefinition {
                name: "table_name".to_string(),
                data_type: "TEXT".to_string(),
                nullable: false,
                primary_key: false,
                char_max_length: None,
                collation: None,
                default_value: None,
            },
            ColumnDefinition {
                name: "record_id".to_string(),
                data_type: "TEXT".to_string(),
                nullable: true,
                primary_key: false,
                char_max_length: None,
                collation: None,
                default_value: None,
            },
            ColumnDefinition {
                name: "old_value".to_string(),
                data_type: "TEXT".to_string(),
                nullable: true,
                primary_key: false,
                char_max_length: None,
                collation: None,
                default_value: None,
            },
            ColumnDefinition {
                name: "new_value".to_string(),
                data_type: "TEXT".to_string(),
                nullable: true,
                primary_key: false,
                char_max_length: None,
                collation: None,
                default_value: None,
            },
            ColumnDefinition {
                name: "ip_address".to_string(),
                data_type: "TEXT".to_string(),
                nullable: true,
                primary_key: false,
                char_max_length: None,
                collation: None,
                default_value: None,
            },
            ColumnDefinition {
                name: "session_id".to_string(),
                data_type: "TEXT".to_string(),
                nullable: true,
                primary_key: false,
                char_max_length: None,
                collation: None,
                default_value: None,
            },
            ColumnDefinition {
                name: "previous_hash".to_string(),
                data_type: "TEXT".to_string(),
                nullable: true,
                primary_key: false,
                char_max_length: None,
                collation: None,
                default_value: None,
            },
            ColumnDefinition {
                name: "event_hash".to_string(),
                data_type: "TEXT".to_string(),
                nullable: false,
                primary_key: false,
                char_max_length: None,
                collation: None,
                default_value: None,
            },
        ];
        storage.create_table(&sqlrustgo_storage::TableInfo {
            name: TABLE_AUDIT_LOG.to_string(),
            columns,
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            compression: None,
            collations: std::collections::HashMap::new(),
            partition_info: None,
        })?;
    }
    Ok(())
}

/// Get the last audit log row's event_hash, or None if no audit rows exist.
pub fn get_last_event_hash(storage: &dyn StorageEngine) -> SqlResult<Option<String>> {
    let rows = storage.scan(TABLE_AUDIT_LOG)?;
    let last = rows
        .into_iter()
        .filter_map(|r| AuditLog::from_row(&r))
        .max_by_key(|l| l.id);
    Ok(last.map(|l| l.event_hash))
}

/// Record an audit log entry with hash chain.
/// Computes `previous_hash` from the last audit row and `event_hash` from this row's content.
#[allow(clippy::too_many_arguments)]
pub fn record_audit_log(
    storage: &mut dyn StorageEngine,
    user_id: &str,
    action: &str,
    table_name: &str,
    record_id: Option<&str>,
    old_value: Option<&str>,
    new_value: Option<&str>,
    ip_address: Option<&str>,
    session_id: Option<&str>,
) -> SqlResult<i64> {
    // Get the next ID
    let rows = storage.scan(TABLE_AUDIT_LOG)?;
    let next_id = rows
        .iter()
        .filter_map(|r| match r.get(0)? {
            Value::Integer(n) => Some(*n),
            _ => None,
        })
        .max()
        .unwrap_or(0)
        + 1;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    // Get the previous row's event_hash for chaining
    let previous_hash = get_last_event_hash(storage)?;

    // Build the audit log entry for hash computation
    let log_entry = AuditLog {
        id: next_id,
        timestamp,
        user_id: user_id.to_string(),
        action: action.to_string(),
        table_name: table_name.to_string(),
        record_id: record_id.map(|s| s.to_string()),
        old_value: old_value.map(|s| s.to_string()),
        new_value: new_value.map(|s| s.to_string()),
        ip_address: ip_address.map(|s| s.to_string()),
        session_id: session_id.map(|s| s.to_string()),
        previous_hash: previous_hash.clone(),
        event_hash: String::new(), // placeholder
    };

    let event_hash = compute_event_hash(&log_entry);

    let row = vec![
        Value::Integer(next_id),
        Value::Integer(timestamp),
        Value::Text(user_id.to_string()),
        Value::Text(action.to_string()),
        Value::Text(table_name.to_string()),
        record_id
            .map(|s| Value::Text(s.to_string()))
            .unwrap_or(Value::Null),
        old_value
            .map(|s| Value::Text(s.to_string()))
            .unwrap_or(Value::Null),
        new_value
            .map(|s| Value::Text(s.to_string()))
            .unwrap_or(Value::Null),
        ip_address
            .map(|s| Value::Text(s.to_string()))
            .unwrap_or(Value::Null),
        session_id
            .map(|s| Value::Text(s.to_string()))
            .unwrap_or(Value::Null),
        previous_hash
            .as_ref()
            .map(|s| Value::Text(s.clone()))
            .unwrap_or(Value::Null),
        Value::Text(event_hash),
    ];

    storage.insert(TABLE_AUDIT_LOG, vec![row])?;
    Ok(next_id)
}

/// Verify the entire audit hash chain.
/// Returns (ok, broken_at_id) — (true, None) if chain is intact,
/// (false, Some(id)) if broken at the first mismatched row.
pub fn verify_audit_chain(storage: &dyn StorageEngine) -> SqlResult<(bool, Option<i64>)> {
    let rows = storage.scan(TABLE_AUDIT_LOG)?;
    let logs: Vec<AuditLog> = rows
        .into_iter()
        .filter_map(|r| AuditLog::from_row(&r))
        .collect();

    for (i, log) in logs.iter().enumerate() {
        // Check event_hash matches computed hash
        if !log.verify_event_hash() {
            return Ok((false, Some(log.id)));
        }
        // Check previous_hash chain
        if i == 0 {
            // Genesis row: previous_hash must be None
            if log.previous_hash.is_some() {
                return Ok((false, Some(log.id)));
            }
        } else {
            let prev = &logs[i - 1];
            if log.previous_hash.as_ref() != Some(&prev.event_hash) {
                return Ok((false, Some(log.id)));
            }
        }
    }
    Ok((true, None))
}

/// Query audit logs with optional filters.
pub fn query_audit_logs(
    storage: &dyn StorageEngine,
    start_time: Option<i64>,
    end_time: Option<i64>,
    user_id: Option<&str>,
    action: Option<&str>,
    table_name: Option<&str>,
) -> SqlResult<Vec<AuditLog>> {
    let rows = storage.scan(TABLE_AUDIT_LOG)?;

    let logs = rows
        .into_iter()
        .filter_map(|row| {
            let log = AuditLog::from_row(&row)?;

            if let Some(start) = start_time {
                if log.timestamp < start {
                    return None;
                }
            }
            if let Some(end) = end_time {
                if log.timestamp > end {
                    return None;
                }
            }
            if let Some(uid) = user_id {
                if log.user_id != uid {
                    return None;
                }
            }
            if let Some(act) = action {
                if log.action != act {
                    return None;
                }
            }
            if let Some(tbl) = table_name {
                if log.table_name != tbl {
                    return None;
                }
            }

            Some(log)
        })
        .collect();

    Ok(logs)
}

/// Get all audit logs ordered by id.
pub fn get_all_audit_logs(storage: &dyn StorageEngine) -> SqlResult<Vec<AuditLog>> {
    let rows = storage.scan(TABLE_AUDIT_LOG)?;
    let mut logs: Vec<AuditLog> = rows
        .into_iter()
        .filter_map(|row| AuditLog::from_row(&row))
        .collect();
    logs.sort_by_key(|l| l.id);
    Ok(logs)
}

/// Get audit log by ID.
pub fn get_audit_log_by_id(storage: &dyn StorageEngine, id: i64) -> SqlResult<Option<AuditLog>> {
    let rows = storage.scan(TABLE_AUDIT_LOG)?;
    let log = rows
        .into_iter()
        .filter_map(|row| AuditLog::from_row(&row))
        .find(|log| log.id == id);
    Ok(log)
}

/// Get audit statistics for a time period.
#[derive(Debug, Clone)]
pub struct AuditStats {
    pub total_records: i64,
    pub create_count: i64,
    pub update_count: i64,
    pub delete_count: i64,
    pub by_user: Vec<UserCount>,
    pub by_table: Vec<TableCount>,
}

#[derive(Debug, Clone)]
pub struct UserCount {
    pub user_id: String,
    pub count: i64,
}

#[derive(Debug, Clone)]
pub struct TableCount {
    pub table_name: String,
    pub count: i64,
}

pub fn get_audit_stats(
    storage: &dyn StorageEngine,
    start_time: Option<i64>,
    end_time: Option<i64>,
) -> SqlResult<AuditStats> {
    let logs = query_audit_logs(storage, start_time, end_time, None, None, None)?;

    let total_records = logs.len() as i64;
    let create_count = logs.iter().filter(|l| l.action == "CREATE").count() as i64;
    let update_count = logs.iter().filter(|l| l.action == "UPDATE").count() as i64;
    let delete_count = logs.iter().filter(|l| l.action == "DELETE").count() as i64;

    let mut user_counts: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    for log in &logs {
        *user_counts.entry(log.user_id.clone()).or_insert(0) += 1;
    }
    let by_user = user_counts
        .into_iter()
        .map(|(user_id, count)| UserCount { user_id, count })
        .collect();

    let mut table_counts: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    for log in &logs {
        *table_counts.entry(log.table_name.clone()).or_insert(0) += 1;
    }
    let by_table = table_counts
        .into_iter()
        .map(|(table_name, count)| TableCount { table_name, count })
        .collect();

    Ok(AuditStats {
        total_records,
        create_count,
        update_count,
        delete_count,
        by_user,
        by_table,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_log(id: i64, timestamp: i64, user_id: &str) -> AuditLog {
        AuditLog {
            id,
            timestamp,
            user_id: user_id.to_string(),
            action: "CREATE".to_string(),
            table_name: "gmp_documents".to_string(),
            record_id: Some("1".to_string()),
            old_value: None,
            new_value: Some(r#"{"title":"Test"}"#.to_string()),
            ip_address: Some("192.168.1.1".to_string()),
            session_id: Some("session123".to_string()),
            previous_hash: None,
            event_hash: String::new(),
        }
    }

    #[test]
    fn test_audit_action_conversion() {
        assert_eq!(AuditAction::from_str("CREATE"), Some(AuditAction::Create));
        assert_eq!(AuditAction::from_str("create"), Some(AuditAction::Create));
        assert_eq!(AuditAction::from_str("UPDATE"), Some(AuditAction::Update));
        assert_eq!(AuditAction::from_str("DELETE"), Some(AuditAction::Delete));
        assert_eq!(AuditAction::from_str("UNKNOWN"), None);
        assert_eq!(AuditAction::Create.as_str(), "CREATE");
    }

    #[test]
    fn test_create_audit_table() {
        let mut storage = sqlrustgo_storage::MemoryStorage::new();
        create_audit_log_table(&mut storage).unwrap();
        assert!(storage.has_table(TABLE_AUDIT_LOG));
    }

    #[test]
    fn test_record_and_query_audit_log() {
        let mut storage = sqlrustgo_storage::MemoryStorage::new();
        create_audit_log_table(&mut storage).unwrap();

        let log_id = record_audit_log(
            &mut storage,
            "user1",
            "CREATE",
            "gmp_documents",
            Some("1"),
            None,
            Some(r#"{"title":"Test"}"#),
            Some("192.168.1.1"),
            Some("session123"),
        )
        .unwrap();

        assert!(log_id > 0);

        let logs = query_audit_logs(&storage, None, None, None, None, None).unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].user_id, "user1");
        assert_eq!(logs[0].action, "CREATE");
        assert_eq!(logs[0].table_name, "gmp_documents");
    }

    #[test]
    fn test_compliance_action_variants_roundtrip() {
        // V312-53 followup #4231: AuditAction must cover compliance ops
        // (Import, Export, Approve, Review, Backup, Restore). Each must
        // round-trip through as_str / from_str and survive a hash-chain
        // record_audit_log cycle.
        let cases: [(AuditAction, &str); 6] = [
            (AuditAction::Import, "IMPORT"),
            (AuditAction::Export, "EXPORT"),
            (AuditAction::Approve, "APPROVE"),
            (AuditAction::Review, "REVIEW"),
            (AuditAction::Backup, "BACKUP"),
            (AuditAction::Restore, "RESTORE"),
        ];
        for (variant, expected_str) in cases.iter() {
            assert_eq!(variant.as_str(), *expected_str, "as_str for {:?}", variant);
            let parsed = AuditAction::from_str(expected_str)
                .unwrap_or_else(|| panic!("from_str({}) must parse", expected_str));
            assert_eq!(parsed, *variant, "round-trip mismatch for {}", expected_str);
        }

        // Record one of each and verify chain remains intact.
        let mut storage = sqlrustgo_storage::MemoryStorage::new();
        create_audit_log_table(&mut storage).unwrap();
        let action_strs: Vec<&str> = cases.iter().map(|(_, s)| *s).collect();
        for s in &action_strs {
            record_audit_log(
                &mut storage,
                "compliance-user",
                s,
                "gmp_documents",
                Some("doc-1"),
                None,
                None,
                None,
                None,
            )
            .unwrap();
        }
        let (ok, broken_at) = verify_audit_chain(&storage).unwrap();
        assert!(
            ok,
            "chain must be intact after 6 compliance ops; broken_at={:?}",
            broken_at
        );
        let logs = query_audit_logs(&storage, None, None, None, None, None).unwrap();
        assert_eq!(logs.len(), 6);
        let actions: Vec<&str> = logs.iter().map(|l| l.action.as_str()).collect();
        assert_eq!(
            actions,
            vec!["IMPORT", "EXPORT", "APPROVE", "REVIEW", "BACKUP", "RESTORE"]
        );
    }

    #[test]
    fn test_event_hash_deterministic() {
        let log = make_log(1, 1000, "user1");
        let h1 = compute_event_hash(&log);
        let h2 = compute_event_hash(&log);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64); // SHA-256 hex
    }

    #[test]
    fn test_event_hash_different_inputs() {
        let log1 = make_log(1, 1000, "user1");
        let log2 = make_log(2, 1000, "user1");
        let h1 = compute_event_hash(&log1);
        let h2 = compute_event_hash(&log2);
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_verify_event_hash() {
        let mut log = make_log(1, 1000, "user1");
        log.event_hash = compute_event_hash(&log);
        assert!(log.verify_event_hash());
        log.event_hash = "invalid".to_string();
        assert!(!log.verify_event_hash());
    }

    #[test]
    fn test_hash_chain_two_rows() {
        let mut storage = sqlrustgo_storage::MemoryStorage::new();
        create_audit_log_table(&mut storage).unwrap();

        record_audit_log(
            &mut storage,
            "u1",
            "CREATE",
            "gmp_documents",
            Some("1"),
            None,
            None,
            None,
            None,
        )
        .unwrap();
        record_audit_log(
            &mut storage,
            "u2",
            "UPDATE",
            "gmp_documents",
            Some("1"),
            None,
            None,
            None,
            None,
        )
        .unwrap();

        let (ok, broken_at) = verify_audit_chain(&storage).unwrap();
        assert!(ok, "expected chain intact, broken at {:?}", broken_at);
    }

    #[test]
    fn test_hash_chain_genesis_previous_hash_none() {
        let mut storage = sqlrustgo_storage::MemoryStorage::new();
        create_audit_log_table(&mut storage).unwrap();

        record_audit_log(
            &mut storage,
            "u1",
            "CREATE",
            "gmp_documents",
            Some("1"),
            None,
            None,
            None,
            None,
        )
        .unwrap();

        let logs = get_all_audit_logs(&storage).unwrap();
        assert!(logs[0].previous_hash.is_none());
    }

    #[test]
    fn test_hash_chain_tamper_detection() {
        // Real tamper integration test:
        // 1. Insert N audit log rows forming a valid chain.
        // 2. Verify chain is intact.
        // 3. Mutate a stored row's column (action) via StorageEngine::update_if.
        // 4. Verify chain now reports tamper at the mutated row.
        // Column order: id(0), timestamp(1), user_id(2), action(3),
        // table_name(4), record_id(5), old_value(6), new_value(7),
        // ip_address(8), session_id(9), previous_hash(10), event_hash(11).
        let mut storage = sqlrustgo_storage::MemoryStorage::new();
        create_audit_log_table(&mut storage).unwrap();

        // 1. Insert 3 rows forming a chain.
        record_audit_log(
            &mut storage,
            "u1",
            "CREATE",
            "gmp_documents",
            Some("1"),
            None,
            None,
            None,
            None,
        )
        .unwrap();
        record_audit_log(
            &mut storage,
            "u1",
            "UPDATE",
            "gmp_documents",
            Some("1"),
            None,
            None,
            None,
            None,
        )
        .unwrap();
        record_audit_log(
            &mut storage,
            "u1",
            "DELETE",
            "gmp_documents",
            Some("1"),
            None,
            None,
            None,
            None,
        )
        .unwrap();

        // 2. Chain must be intact before tampering.
        let (ok, broken_at) = verify_audit_chain(&storage).unwrap();
        assert!(
            ok,
            "chain must be intact after writes; broken_at={:?}",
            broken_at
        );
        assert!(broken_at.is_none());

        // 3. Mutate row id=2 (action column 3) from "UPDATE" to "TAMPERED".
        // Use a RowFilter that matches only the row with id=2.
        let filter: sqlrustgo_storage::RowFilter = Box::new(|r: &sqlrustgo_storage::Record| {
            matches!(r.first(), Some(sqlrustgo_types::Value::Integer(2)))
        });
        let mutation = sqlrustgo_storage::RowMutation::new(
            vec![(3, sqlrustgo_types::Value::Text("TAMPERED".to_string()))],
            0xDEADBEEFu64,
        );
        let updated = storage
            .update_if("gmp_audit_log", &filter, &mutation)
            .expect("update_if should succeed against in-memory storage");
        assert_eq!(updated, 1, "exactly one row should be mutated");

        // 4. Chain must now report tamper at row id=2.
        let (ok, broken_at) = verify_audit_chain(&storage).unwrap();
        assert!(
            !ok,
            "chain must be reported broken after tampering a stored row"
        );
        assert_eq!(
            broken_at,
            Some(2),
            "tamper must be detected at the mutated row id"
        );
    }

    #[test]
    fn test_hash_chain_tamper_detection_negative_no_mutate() {
        // Sanity: this is the pre-existing behavior — verify_audit_chain
        // against an untouched chain returns (true, None). Kept as a
        // named test so the absence of tampering is a distinct assertion.
        let mut storage = sqlrustgo_storage::MemoryStorage::new();
        create_audit_log_table(&mut storage).unwrap();

        record_audit_log(
            &mut storage,
            "u1",
            "CREATE",
            "gmp_documents",
            Some("1"),
            None,
            None,
            None,
            None,
        )
        .unwrap();
        record_audit_log(
            &mut storage,
            "u1",
            "UPDATE",
            "gmp_documents",
            Some("1"),
            None,
            None,
            None,
            None,
        )
        .unwrap();

        let (ok, broken_at) = verify_audit_chain(&storage).unwrap();
        assert!(ok);
        assert!(broken_at.is_none());
    }

    // v3.13.0 §4.2.2 — embedding tamper detection (cross-table).
    //
    // Honest disclosure: `verify_audit_chain` only protects `gmp_audit_log`
    // itself. Cross-table tamper detection on `gmp_embeddings` and
    // `gmp_relations` relies on the discipline that every mutation must
    // record a corresponding audit-log entry. We test that discipline
    // here:
    //
    //   1. Embedding row exists in `gmp_embeddings`.
    //   2. Audit log records CREATE + DELETE actions on that row.
    //   3. `verify_audit_chain` is still intact (audit log self-protecting).
    //   4. The audit-log timeline correctly reflects the action sequence.
    //
    // Out-of-band tamper of `gmp_embeddings` (no audit entry) cannot be
    // detected by `verify_audit_chain` alone — that requires a future
    // cross-table hash chain, which is intentionally NOT in this sprint.
    #[test]
    fn test_embedding_tamper_audit_log_action_trail() {
        use crate::embedding::TABLE_EMBEDDINGS;
        let mut storage = sqlrustgo_storage::MemoryStorage::new();
        create_audit_log_table(&mut storage).unwrap();
        crate::vector_search::create_embeddings_table(&mut storage).unwrap();

        // 1. Seed an embedding row.
        let emb: Vec<f32> = (0..crate::embedding::EMBEDDING_DIM)
            .map(|i| i as f32 * 0.001)
            .collect();
        crate::vector_search::upsert_embedding(&mut storage, 42, &emb, "hash").unwrap();

        // 2. Audit the CREATE action.
        record_audit_log(
            &mut storage,
            "user1",
            "CREATE",
            TABLE_EMBEDDINGS,
            Some("42"),
            None,
            None,
            None,
            None,
        )
        .unwrap();

        // 3. Tamper the embedding JSON column directly via update_if.
        let filter: sqlrustgo_storage::RowFilter = Box::new(|r: &sqlrustgo_storage::Record| {
            matches!(r.first(), Some(sqlrustgo_types::Value::Integer(42)))
        });
        let mutation = sqlrustgo_storage::RowMutation::new(
            vec![(
                1, // embedding column (TEXT)
                sqlrustgo_types::Value::Text("[0.0]".to_string()),
            )],
            0xC0FFEE,
        );
        let updated = storage
            .update_if(TABLE_EMBEDDINGS, &filter, &mutation)
            .expect("update_if must succeed against in-memory storage");
        assert_eq!(updated, 1, "exactly one embedding row should be mutated");

        // 4. Audit log self-protecting: chain still intact because the
        //    audit log itself was not mutated — only gmp_embeddings was.
        let (ok, broken_at) = verify_audit_chain(&storage).unwrap();
        assert!(
            ok,
            "audit log chain must remain intact after out-of-band embedding tamper; broken_at={:?}",
            broken_at
        );
        assert!(
            broken_at.is_none(),
            "audit log chain must not report a false-positive tamper"
        );

        // 5. The audit timeline still records the CREATE action but has
        //    no UPDATE entry — this is the gap that future cross-table
        //    hash chaining would close.
        let logs = get_all_audit_logs(&storage).unwrap();
        assert_eq!(logs.len(), 1, "exactly one audit entry exists");
        assert_eq!(logs[0].action, "CREATE");
        assert_eq!(logs[0].table_name, TABLE_EMBEDDINGS);
        assert_eq!(logs[0].record_id.as_deref(), Some("42"));
    }

    // v3.13.0 §4.2.2 — relation/graph tamper detection (cross-table).
    //
    // Same honest disclosure as `test_embedding_tamper_audit_log_action_trail`:
    // the audit log chain protects itself only. Relation tamper is
    // detectable via the action trail (every mutation should record a
    // corresponding audit entry) but not via `verify_audit_chain` alone.
    #[test]
    fn test_relation_tamper_audit_log_action_trail() {
        use crate::schema::{RelationType, TABLE_RELATIONS};
        let mut storage = sqlrustgo_storage::MemoryStorage::new();
        create_audit_log_table(&mut storage).unwrap();
        crate::document::create_gmp_tables(&mut storage).unwrap();

        // 1. Seed a relation edge.
        crate::relation::insert_relation(
            &mut storage,
            Some(1), // source_doc_id
            None,    // source_chunk_id
            &RelationType::Sop,
            Some(2), // target_doc_id
            None,    // target_chunk_id
            None,    // properties
        )
        .unwrap();

        // 2. Audit the CREATE.
        record_audit_log(
            &mut storage,
            "user2",
            "CREATE",
            TABLE_RELATIONS,
            Some("1"),
            None,
            None,
            None,
            None,
        )
        .unwrap();

        // 3. Tamper the relation_type column (column 3) directly.
        //    Column order per relation.rs::insert_relation: id(0),
        //    source_doc_id(1), source_chunk_id(2), relation_type(3),
        //    target_doc_id(4), target_chunk_id(5), properties(6),
        //    created_at(7).
        let filter: sqlrustgo_storage::RowFilter = Box::new(|r: &sqlrustgo_storage::Record| {
            matches!(r.first(), Some(sqlrustgo_types::Value::Integer(1)))
        });
        let mutation = sqlrustgo_storage::RowMutation::new(
            vec![(3, sqlrustgo_types::Value::Text("TAMPERED".to_string()))],
            0xBADF00D,
        );
        let updated = storage
            .update_if(TABLE_RELATIONS, &filter, &mutation)
            .expect("update_if must succeed");
        assert_eq!(updated, 1, "exactly one relation row should be mutated");

        // 4. Audit log self-protecting.
        let (ok, broken_at) = verify_audit_chain(&storage).unwrap();
        assert!(
            ok,
            "audit log chain must remain intact after out-of-band relation tamper; broken_at={:?}",
            broken_at
        );

        // 5. Timeline reflects CREATE only — no UPDATE entry.
        let logs = get_all_audit_logs(&storage).unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].action, "CREATE");
        assert_eq!(logs[0].table_name, TABLE_RELATIONS);
    }

    // v3.13.0 §4.2.2 — positive: CREATE + UPDATE + DELETE all chained.
    //
    // Demonstrates that the audit log hash chain correctly tracks the
    // full lifecycle of an embedding row (or relation row) when each
    // mutation is properly recorded. This is the supported path —
    // anything else is a process bug.
    #[test]
    fn test_audit_log_lifecycle_for_embedding_full_trail() {
        use crate::embedding::TABLE_EMBEDDINGS;
        let mut storage = sqlrustgo_storage::MemoryStorage::new();
        create_audit_log_table(&mut storage).unwrap();
        crate::vector_search::create_embeddings_table(&mut storage).unwrap();

        // CREATE
        record_audit_log(
            &mut storage,
            "user1",
            "CREATE",
            TABLE_EMBEDDINGS,
            Some("99"),
            None,
            None,
            None,
            None,
        )
        .unwrap();
        // UPDATE
        record_audit_log(
            &mut storage,
            "user1",
            "UPDATE",
            TABLE_EMBEDDINGS,
            Some("99"),
            None,
            None,
            None,
            None,
        )
        .unwrap();
        // DELETE
        record_audit_log(
            &mut storage,
            "user1",
            "DELETE",
            TABLE_EMBEDDINGS,
            Some("99"),
            None,
            None,
            None,
            None,
        )
        .unwrap();

        // All three chained entries must verify.
        let (ok, broken_at) = verify_audit_chain(&storage).unwrap();
        assert!(
            ok,
            "full-lifecycle chain must verify; broken_at={:?}",
            broken_at
        );
        assert!(broken_at.is_none());

        // Timeline reflects all three actions in order.
        let logs = get_all_audit_logs(&storage).unwrap();
        assert_eq!(logs.len(), 3);
        assert_eq!(logs[0].action, "CREATE");
        assert_eq!(logs[1].action, "UPDATE");
        assert_eq!(logs[2].action, "DELETE");
        // previous_hash chain links every row.
        assert!(
            logs[0].previous_hash.is_none(),
            "genesis row has no previous_hash"
        );
        assert_eq!(
            logs[1].previous_hash.as_deref(),
            Some(logs[0].event_hash.as_str())
        );
        assert_eq!(
            logs[2].previous_hash.as_deref(),
            Some(logs[1].event_hash.as_str())
        );
    }

    #[test]
    fn test_audit_stats() {
        let mut storage = sqlrustgo_storage::MemoryStorage::new();
        create_audit_log_table(&mut storage).unwrap();

        record_audit_log(
            &mut storage,
            "user1",
            "CREATE",
            "gmp_documents",
            Some("1"),
            None,
            None,
            None,
            None,
        )
        .unwrap();
        record_audit_log(
            &mut storage,
            "user1",
            "UPDATE",
            "gmp_documents",
            Some("1"),
            None,
            None,
            None,
            None,
        )
        .unwrap();
        record_audit_log(
            &mut storage,
            "user2",
            "CREATE",
            "gmp_documents",
            Some("2"),
            None,
            None,
            None,
            None,
        )
        .unwrap();
        record_audit_log(
            &mut storage,
            "user2",
            "DELETE",
            "gmp_documents",
            Some("1"),
            None,
            None,
            None,
            None,
        )
        .unwrap();

        let stats = get_audit_stats(&storage, None, None).unwrap();
        assert_eq!(stats.total_records, 4);
        assert_eq!(stats.create_count, 2);
        assert_eq!(stats.update_count, 1);
        assert_eq!(stats.delete_count, 1);
        assert_eq!(stats.by_user.len(), 2);
    }

    #[test]
    fn test_audit_log_filtering() {
        let mut storage = sqlrustgo_storage::MemoryStorage::new();
        create_audit_log_table(&mut storage).unwrap();

        record_audit_log(
            &mut storage,
            "user1",
            "CREATE",
            "gmp_documents",
            Some("1"),
            None,
            None,
            None,
            None,
        )
        .unwrap();
        record_audit_log(
            &mut storage,
            "user2",
            "CREATE",
            "gmp_documents",
            Some("2"),
            None,
            None,
            None,
            None,
        )
        .unwrap();
        record_audit_log(
            &mut storage,
            "user1",
            "DELETE",
            "gmp_document_contents",
            Some("1"),
            None,
            None,
            None,
            None,
        )
        .unwrap();

        let user1_logs = query_audit_logs(&storage, None, None, Some("user1"), None, None).unwrap();
        assert_eq!(user1_logs.len(), 2);

        let create_logs =
            query_audit_logs(&storage, None, None, None, Some("CREATE"), None).unwrap();
        assert_eq!(create_logs.len(), 2);

        let content_logs = query_audit_logs(
            &storage,
            None,
            None,
            None,
            None,
            Some("gmp_document_contents"),
        )
        .unwrap();
        assert_eq!(content_logs.len(), 1);
    }

    #[test]
    fn test_get_last_event_hash() {
        let mut storage = sqlrustgo_storage::MemoryStorage::new();
        create_audit_log_table(&mut storage).unwrap();

        assert!(get_last_event_hash(&storage).unwrap().is_none());

        record_audit_log(
            &mut storage,
            "u1",
            "CREATE",
            "gmp_documents",
            Some("1"),
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let h1 = get_last_event_hash(&storage).unwrap();
        assert!(h1.is_some());

        record_audit_log(
            &mut storage,
            "u2",
            "CREATE",
            "gmp_documents",
            Some("2"),
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let h2 = get_last_event_hash(&storage).unwrap();
        assert!(h2.is_some());
        assert_ne!(h1, h2);
    }
}
