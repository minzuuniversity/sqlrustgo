//! LOAD DATA LOCAL INFILE — TBL parsing and batched insert.
//!
//! TBL format (TPC-H standard):
//!   - One row per line, fields separated by `|`
//!   - Lines end with `|\n` (trailing pipe), but we tolerate `|\n` or `\n`
//!   - Empty field → NULL
//!   - Integer-parseable → i64
//!   - Float-parseable → f64
//!   - Otherwise → Text

use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::engine::MemoryStorage;
use sqlrustgo_storage::wal::MemoryWalManager;
use sqlrustgo_storage::wal_storage::WalStorage;
use sqlrustgo_storage::{FileBackedWalManager, FileStorage, StorageEngine};
use sqlrustgo_types::Value as SqlValue;

use crate::MySqlError;

pub fn parse_tbl_line(line: &str, expected_columns: usize) -> Result<Vec<SqlValue>, String> {
    let trimmed = line.trim_end_matches('\n').trim_end_matches('\r');
    // TPC-H .tbl: trailing `|` means last field is empty (counted)
    // We split on `|` and drop the trailing empty if present.
    let parts: Vec<&str> = trimmed.split('|').collect();
    let parts: Vec<&str> = if parts.last() == Some(&"") {
        parts[..parts.len() - 1].to_vec()
    } else {
        parts
    };

    if parts.len() < expected_columns {
        return Err(format!(
            "line has {} fields, expected at least {}: {:?}",
            parts.len(),
            expected_columns,
            line
        ));
    }

    let record: Vec<SqlValue> = parts[..expected_columns]
        .iter()
        .map(|v| {
            let s = v.trim();
            if s.is_empty() {
                SqlValue::Null
            } else if let Ok(i) = s.parse::<i64>() {
                SqlValue::Integer(i)
            } else if let Ok(f) = s.parse::<f64>() {
                SqlValue::Float(f)
            } else {
                SqlValue::Text(s.to_string())
            }
        })
        .collect();

    Ok(record)
}

/// Bulk-insert pre-parsed records directly into storage, bypassing
/// the SQL parser.
///
/// Returns the number of rows inserted (== rows.len() on success).
///
/// Generic over the storage backend so it works with both
/// `MemoryStorage` (used in unit tests) and `WalStorage<FileStorage,
/// FileBackedWalManager>` (the production ephemeral server's engine).
///
/// ## Why bypass the parser?
///
/// The previous implementation built a single
/// `INSERT INTO t VALUES (...), (...), ...` string (~2 MB for a 60 000-row
/// lineitem.tbl) and ran it through `engine.execute()`, which re-parses
/// the entire SQL string every call. On canonical SF=0.01 a 60 000-row
/// lineitem load took >5 min through that path.
///
/// `engine.bulk_insert_records(table, rows)` takes pre-parsed records
/// and hands them straight to `Storage::insert`, which writes to the
/// buffer pool + WAL in one go. No parser, no AST allocation, no
/// multi-MB string concatenation. Same transactional guarantees as a
/// SQL INSERT (auto-commit per call).
pub fn bulk_insert<S: StorageEngine + 'static>(
    engine: &mut ExecutionEngine<S>,
    table: &str,
    rows: Vec<Vec<SqlValue>>,
) -> Result<u64, MySqlError> {
    if rows.is_empty() {
        return Ok(0);
    }

    // #5527: 有键表必须查重复键 —— 此前这条路径直接写存储层，
    // 绕过了普通 INSERT 的重复键检查，重复导入**静默追加**。
    check_duplicate_keys(engine, table, &rows)?;

    // Convert Vec<Vec<SqlValue>> into the storage's Record type. They are
    // the same shape (Record == Vec<Value>), so the conversion is a
    // cheap type-coercion rather than a clone.
    let records: Vec<sqlrustgo_storage::Record> = rows;
    let n = engine
        .bulk_insert_records(table, records)
        .map_err(|e| MySqlError::Other(format!("bulk_insert_records failed: {}", e)))?;
    Ok(n)
}

/// #5527: `LOAD DATA` 的重复键检查。
///
/// 此前 `bulk_insert` 直接调 `storage.bulk_insert_records`，**完全绕开**了普通
/// `INSERT` 的重复键检查 ⇒ 重复导入同一批行会**静默追加**：
///
/// * 运维用 `LOAD DATA` 做幂等重导 ⇒ 得到一份行数翻倍的数据，
///   而且 `affected_rows` 只会报成功；
/// * 与 MySQL 不一致 —— MySQL 在**有主键**的表上会报
///   `ERROR 1062 (23000): Duplicate entry '1' for key 'PRIMARY'`。
///
/// 语义对齐 MySQL：
///
/// * 表**声明了**主键/唯一约束 ⇒ 重复键报错（1062 / 23000）；
/// * 表**没有**键 ⇒ 追加（与 MySQL 一致，MySQL 在无键表上也是追加）。
///
/// 检查范围：既有行 ∪ 本批已接受的行 —— 同一批里两行撞键同样是重复。
pub fn check_duplicate_keys<S: StorageEngine + 'static>(
    engine: &ExecutionEngine<S>,
    table: &str,
    records: &[sqlrustgo_storage::Record],
) -> Result<(), MySqlError> {
    use sqlrustgo::engine_helpers::{find_conflicting_key, key_entry_repr};

    let storage_arc = engine.storage_ref();
    let storage = storage_arc.read();
    let table_info = storage
        .get_table_info(table)
        .map_err(|e| MySqlError::Other(format!("table {}: {}", table, e)))?;

    // 无键表 ⇒ 追加语义（与 MySQL 一致），不做检查。
    let has_key = table_info.columns.iter().any(|c| c.primary_key)
        || !table_info.unique_constraints.is_empty();
    if !has_key {
        return Ok(());
    }

    let existing = storage
        .scan(table)
        .map_err(|e| MySqlError::Other(format!("scan {}: {}", table, e)))?;

    // 既有行 + 本批已接受的行，都要参与比对。
    let mut accepted: Vec<&sqlrustgo_storage::Record> =
        Vec::with_capacity(existing.len() + records.len());
    accepted.extend(existing.iter());
    for record in records {
        for other in &accepted {
            if let Some(key) = find_conflicting_key(other, record, &table_info) {
                let repr = key_entry_repr(&table_info, record, &key);
                return Err(MySqlError::Coded {
                    code: 1062,
                    sqlstate: "23000".to_string(),
                    message: format!("Duplicate entry '{}' for key '{}'", repr, key),
                });
            }
        }
        accepted.push(record);
    }
    Ok(())
}

/// T4.1: Try to override the WAL sync mode of `storage` to `mode`.
/// If `storage` is not a `WalStorage` (file-backed or memory), this is a no-op
/// and returns `None`.
///
/// Returns the original sync mode so the caller can restore it.
pub fn apply_wal_sync_mode_override(
    storage: &mut dyn StorageEngine,
    mode: sqlrustgo_storage::WalSyncMode,
) -> Option<sqlrustgo_storage::WalSyncMode> {
    // Try file-backed WalStorage first
    if let Some(result) = storage
        .as_any_mut()
        .downcast_mut::<WalStorage<FileStorage, FileBackedWalManager>>()
        .map(|wal_storage| {
            let original = wal_storage.sync_mode();
            wal_storage.set_sync_mode(mode);
            original
        })
    {
        return Some(result);
    }

    // Try memory-based WalStorage
    storage
        .as_any_mut()
        .downcast_mut::<WalStorage<MemoryStorage, MemoryWalManager>>()
        .map(|wal_storage| {
            let original = wal_storage.sync_mode();
            wal_storage.set_sync_mode(mode);
            original
        })
}

/// Restore the WAL sync mode to a previously captured original value.
/// If `storage` is not a `WalStorage` or `original` is `None`, this is a no-op.
pub fn restore_wal_sync_mode(
    storage: &mut dyn StorageEngine,
    original: Option<sqlrustgo_storage::WalSyncMode>,
) {
    if let Some(mode) = original {
        // Try file-backed WalStorage first
        if storage
            .as_any_mut()
            .downcast_mut::<WalStorage<FileStorage, FileBackedWalManager>>()
            .map(|wal_storage| {
                wal_storage.set_sync_mode(mode);
            })
            .is_some()
        {
            return;
        }

        // Try memory-based WalStorage
        let _ = storage
            .as_any_mut()
            .downcast_mut::<WalStorage<MemoryStorage, MemoryWalManager>>()
            .map(|wal_storage| {
                wal_storage.set_sync_mode(mode);
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parking_lot::RwLock;
    use sqlrustgo::{ExecutionEngine, MemoryStorage};
    use sqlrustgo_storage::wal::MemoryWalManager;
    use sqlrustgo_storage::wal_storage::WalSyncMode;
    use std::sync::Arc;

    #[test]
    fn test_bulk_insert_three_rows() {
        let storage = Arc::new(RwLock::new(MemoryStorage::new()));
        let mut engine = ExecutionEngine::new(storage.clone());
        engine
            .execute("CREATE TABLE t1 (id INTEGER, name TEXT)")
            .unwrap();

        let rows = vec![
            vec![SqlValue::Integer(1), SqlValue::Text("a".to_string())],
            vec![SqlValue::Integer(2), SqlValue::Text("b".to_string())],
            vec![SqlValue::Integer(3), SqlValue::Text("c".to_string())],
        ];
        let n = bulk_insert(&mut engine, "t1", rows).unwrap();
        assert_eq!(n, 3);

        let result = engine.execute("SELECT COUNT(*) FROM t1").unwrap();
        assert_eq!(result.rows.len(), 1);
    }

    #[test]
    fn test_parse_tbl_line_basic_text() {
        // TPC-H region.tbl: r_regionkey|i_name|r_comment|
        let line = "0|AFRICA|lar deposits. blithely final packages cajole|\n";
        let cols = vec![
            SqlValue::Integer(0),
            SqlValue::Text("AFRICA".to_string()),
            SqlValue::Text("lar deposits. blithely final packages cajole".to_string()),
        ];
        assert_eq!(parse_tbl_line(line, 3).unwrap(), cols);
    }

    #[test]
    fn test_parse_tbl_line_with_ints() {
        let line = "1|2|3|\n";
        assert_eq!(
            parse_tbl_line(line, 3).unwrap(),
            vec![
                SqlValue::Integer(1),
                SqlValue::Integer(2),
                SqlValue::Integer(3),
            ]
        );
    }

    #[test]
    fn test_parse_tbl_line_with_null() {
        // Empty field in middle → NULL
        let line = "1||3|\n";
        assert_eq!(
            parse_tbl_line(line, 3).unwrap(),
            vec![SqlValue::Integer(1), SqlValue::Null, SqlValue::Integer(3)],
        );
    }

    #[test]
    fn test_parse_tbl_line_too_few_fields_errors() {
        let line = "1|2|\n";
        let result = parse_tbl_line(line, 3);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expected at least 3"));
    }

    #[test]
    fn test_parse_tbl_line_no_trailing_pipe() {
        // Tolerate missing trailing pipe
        let line = "0|AFRICA|comment\n";
        assert_eq!(
            parse_tbl_line(line, 3).unwrap(),
            vec![
                SqlValue::Integer(0),
                SqlValue::Text("AFRICA".to_string()),
                SqlValue::Text("comment".to_string()),
            ]
        );
    }

    #[test]
    fn test_wal_sync_mode_restore_roundtrip() {
        // Create a WalStorage with known initial sync mode (Every)
        let inner = MemoryStorage::new();
        let wal = MemoryWalManager::new();
        let mut wal_storage = WalStorage::new(inner, wal).unwrap();

        // Verify initial mode is Every (default)
        assert_eq!(wal_storage.sync_mode(), WalSyncMode::Every);

        // Apply override: change to Batch(1)
        let original = apply_wal_sync_mode_override(&mut wal_storage, WalSyncMode::Batch(1));
        assert_eq!(original, Some(WalSyncMode::Every));
        assert_eq!(wal_storage.sync_mode(), WalSyncMode::Batch(1));

        // Restore to original mode
        restore_wal_sync_mode(&mut wal_storage, original);

        // Verify restoration: mode should be back to Every
        assert_eq!(wal_storage.sync_mode(), WalSyncMode::Every);
    }

    #[test]
    fn test_apply_wal_sync_mode_override_twice() {
        // Test calling apply twice verifies second call sees original mode
        let inner = MemoryStorage::new();
        let wal = MemoryWalManager::new();
        let mut wal_storage = WalStorage::new(inner, wal).unwrap();

        // First override
        let first_original = apply_wal_sync_mode_override(&mut wal_storage, WalSyncMode::Off);
        assert_eq!(first_original, Some(WalSyncMode::Every));

        // Second override - should capture Batch(1) as "original"
        let second_original = apply_wal_sync_mode_override(&mut wal_storage, WalSyncMode::Batch(1));
        assert_eq!(second_original, Some(WalSyncMode::Off));

        // Restore second capture
        restore_wal_sync_mode(&mut wal_storage, second_original);

        // Should be back to Off (the mode before second override)
        assert_eq!(wal_storage.sync_mode(), WalSyncMode::Off);
    }
}
