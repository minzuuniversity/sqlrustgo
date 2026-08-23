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
use sqlrustgo_storage::wal_storage::WalStorage;
use sqlrustgo_storage::{FileBackedWalManager, FileStorage, StorageEngine};
use sqlrustgo_types::Value as SqlValue;

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
) -> Result<u64, String> {
    if rows.is_empty() {
        return Ok(0);
    }

    // Convert Vec<Vec<SqlValue>> into the storage's Record type. They are
    // the same shape (Record == Vec<Value>), so the conversion is a
    // cheap type-coercion rather than a clone.
    let records: Vec<sqlrustgo_storage::Record> = rows;
    let n = engine
        .bulk_insert_records(table, records)
        .map_err(|e| format!("bulk_insert_records failed: {}", e))?;
    Ok(n)
}

/// T4.1: Try to override the WAL sync mode of `storage` to `mode`.
/// If `storage` is not a `WalStorage<FileStorage, FileBackedWalManager>`
/// (e.g. it is a `BinaryTableStorage` used with the `binary` backend), this
/// is a no-op and returns `None`.
///
/// Returns the original sync mode so the caller can restore it.
pub fn apply_wal_sync_mode_override(
    storage: &mut dyn StorageEngine,
    mode: sqlrustgo_storage::WalSyncMode,
) -> Option<sqlrustgo_storage::WalSyncMode> {
    // downcast_mut requires 'static because we need to know the concrete type.
    storage
        .as_any_mut()
        .downcast_mut::<WalStorage<FileStorage, FileBackedWalManager>>()
        .map(|wal_storage| {
            let original = wal_storage.sync_mode();
            wal_storage.set_sync_mode(mode);
            original
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use parking_lot::RwLock;
    use sqlrustgo::{ExecutionEngine, MemoryStorage};
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
}
