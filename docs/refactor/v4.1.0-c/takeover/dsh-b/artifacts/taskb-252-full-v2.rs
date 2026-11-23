```rust
use sqlrustgo_storage::binary_storage_v2::BinaryTableStorageV2;
use sqlrustgo_storage::engine::{ColumnDefinition, StorageEngine, Value};
use tempfile::TempDir;

fn setup() -> (TempDir, BinaryTableStorageV2) {
    let dir = TempDir::new().unwrap();
    let engine = BinaryTableStorageV2::new(dir.path().to_path_buf());
    engine
        .create_table(
            "t",
            vec![ColumnDefinition {
                name: "id".into(),
                data_type: "BIGINT".into(),
                nullable: false,
                primary_key: true,
                ..Default::default()
            }],
        )
        .unwrap();
    (dir, engine)
}

#[test]
fn test_insert_then_scan_visible() {
    let (_d, eng) = setup();
    eng.insert("t", &[Value::BigInt(1)]).unwrap();
    eng.insert("t", &[Value::BigInt(2)]).unwrap();
    let rows = eng.scan("t").unwrap();
    assert_eq!(rows.len(), 2, "STUB_SILENT_SUCCESS: scan returned empty after 2 inserts");
}

#[test]
fn test_delete_returns_real_count() {
    let (_d, eng) = setup();
    eng.insert("t", &[Value::BigInt(10)]).unwrap();
    eng.insert("t", &[Value::BigInt(20)]).unwrap();
    let affected = eng.delete("t", &[Value::BigInt(10)]).unwrap();
    assert_eq!(affected, 1, "delete must return actual rows removed");
    let remaining = eng.scan("t").unwrap();
    assert_eq!(remaining.len(), 1);
}

#[test]
fn test_update_returns_real_count() {
    let (_d, eng) = setup();
    eng.insert("t", &[Value::BigInt(1)]).unwrap();
    eng.insert("t", &[Value::BigInt(2)]).unwrap();
    let affected = eng.update("t", &[Value::BigInt(1)], &[Value::BigInt(99)]).unwrap();
    assert_eq!(affected, 1, "update must return actual rows modified");
}

#[test]
fn test_transaction_methods_reject_on_non_tx_backend() {
    let (_d, eng) = setup();
    // begin/commit/rollback must return Err, never Ok(()) silently
    let begin = eng.begin_transaction();
    assert!(begin.is_err(), "STUB_SILENT_SUCCESS: begin_transaction returned Ok on non-tx backend");
    let commit = eng.commit_transaction();
    assert!(commit.is_err(), "STUB_SILENT_SUCCESS: commit_transaction returned Ok on non-tx backend");
    let rollback = eng.rollback_transaction();
    assert!(rollback.is_err(), "STUB_SILENT_SUCCESS: rollback_transaction returned Ok on non-tx backend");
}
```