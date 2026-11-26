use sqlrustgo_storage::binary_storage_v2::BinaryTableStorageV2;
use sqlrustgo_storage::engine::{ColumnDefinition, StorageEngine, Value};
use tempfile::TempDir;

fn setup() -> (TempDir, BinaryTableStorageV2) {
    let dir = TempDir::new().unwrap();
    let mut engine = BinaryTableStorageV2::new(dir.path().to_path_buf()).unwrap();
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
#[ignore = "T4 slice not implemented: BinaryTableStorageV2.scan() returns an empty result set, so rows written by insert() are never visible (silent success)"]
fn test_insert_then_scan_visible() {
    let (_d, mut eng) = setup();
    eng.insert("t", vec![vec![Value::Integer(1)]]).unwrap();
    eng.insert("t", vec![vec![Value::Integer(2)]]).unwrap();
    let rows = eng.scan("t").unwrap();
    assert_eq!(
        rows.len(),
        2,
        "STUB_SILENT_SUCCESS: scan returned empty after 2 inserts"
    );
}

#[test]
#[ignore = "T4 slice not implemented: BinaryTableStorageV2.delete() returns 0 and deletes nothing instead of the real affected-row count"]
fn test_delete_returns_real_count() {
    let (_d, mut eng) = setup();
    eng.insert("t", vec![vec![Value::Integer(10)]]).unwrap();
    eng.insert("t", vec![vec![Value::Integer(20)]]).unwrap();
    let affected = eng.delete("t", &[Value::Integer(10)]).unwrap();
    assert_eq!(affected, 1, "delete must return actual rows removed");
    let remaining = eng.scan("t").unwrap();
    assert_eq!(remaining.len(), 1);
    // Verify the surviving row is the one we did NOT delete
    assert_eq!(
        remaining[0],
        vec![Value::Integer(20)],
        "delete removed wrong row"
    );
}

#[test]
#[ignore = "T4 slice not implemented: BinaryTableStorageV2.update() returns 0 and applies nothing instead of the real affected-row count"]
fn test_update_returns_real_count_and_persists() {
    let (_d, mut eng) = setup();
    eng.insert("t", vec![vec![Value::Integer(1)]]).unwrap();
    eng.insert("t", vec![vec![Value::Integer(2)]]).unwrap();

    let affected = eng
        .update("t", &[Value::Integer(1)], &[(0, Value::Integer(99))])
        .unwrap();
    assert_eq!(affected, 1, "update must return actual rows modified");

    // --- scan verification: data must actually be persisted on disk ---
    let rows = eng.scan("t").unwrap();
    assert_eq!(rows.len(), 2, "update must not add or remove rows");

    // Locate the updated row (id was 1, now value is 99)
    let updated = rows.iter().find(|r| r.contains(&Value::Integer(99)));
    assert!(
        updated.is_some(),
        "STUB_SILENT_SUCCESS: update returned Ok(1) but scan shows value 99 was never written"
    );

    // Verify the untouched row is still intact
    let untouched = rows.iter().find(|r| r.contains(&Value::Integer(2)));
    assert!(
        untouched.is_some(),
        "update must not corrupt unrelated rows; id=2 should remain unchanged"
    );
}

#[test]
#[ignore = "T4 slice not implemented: BinaryTableStorageV2.begin_transaction() returns Ok on a non-transactional backend instead of rejecting the call"]
fn test_transaction_methods_reject_on_non_tx_backend() {
    let (_d, mut eng) = setup();

    // Contract assumption: on a non-transactional backend, begin/commit/rollback
    // MUST return Err (not Ok(()) silently, and not panic).

    let begin_result = eng.begin_transaction();
    assert!(
        begin_result.is_err(),
        "STUB_SILENT_SUCCESS: begin_transaction() returned Ok on a non-transactional backend"
    );

    let commit_result = eng.commit_transaction();
    assert!(
        commit_result.is_err(),
        "STUB_SILENT_SUCCESS: commit_transaction() returned Ok on a non-transactional backend"
    );

    let rollback_result = eng.rollback_transaction();
    assert!(
        rollback_result.is_err(),
        "STUB_SILENT_SUCCESS: rollback_transaction() returned Ok on a non-transactional backend"
    );
}
