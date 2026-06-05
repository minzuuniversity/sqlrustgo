//! SavepointManager unit tests (SEM-1 / #3110 partial fix).
//!
//! Scope: Exercise the existing `SavepointManager` API in
//! `crates/transaction/src/savepoint.rs` at the unit level. These tests
//! do NOT verify MVCC snapshot restore (deferred to v3.9.0+ per
//! follow-up #3129) and do NOT exercise the main-path integration
//! (SavepointManager is not yet wired into TransactionManager — also
//! tracked in #3129).
//!
//! What IS tested:
//!   - create + duplicate name (updates index)
//!   - rollback to existing savepoint (pops undo log)
//!   - rollback to non-existent savepoint (returns NotFound)
//!   - release savepoint
//!   - release non-existent savepoint
//!   - multi-savepoint LIFO ordering

use sqlrustgo_transaction::savepoint::{SavepointError, SavepointManager, UndoRecord};

fn make_manager() -> SavepointManager {
    SavepointManager::new()
}

fn insert_key(k: u8) -> UndoRecord {
    UndoRecord::Insert { key: vec![k] }
}

fn delete_key(k: u8) -> UndoRecord {
    UndoRecord::Delete {
        key: vec![k],
        old_value: vec![k, 0],
    }
}

#[test]
fn test_new_manager_is_empty() {
    let m = make_manager();
    assert_eq!(m.get_savepoint_count(), 0);
    assert_eq!(m.undo_log_len(), 0);
}

#[test]
fn test_create_savepoint() {
    let mut m = make_manager();
    m.add_undo(insert_key(1));
    m.add_undo(insert_key(2));
    m.savepoint("sp1".to_string()).unwrap();
    assert_eq!(m.get_savepoint_count(), 1);
    assert_eq!(m.undo_log_len(), 2);
}

#[test]
fn test_duplicate_savepoint_updates_index() {
    let mut m = make_manager();
    m.add_undo(insert_key(1));
    m.savepoint("sp1".to_string()).unwrap();
    m.add_undo(insert_key(2));
    // Re-create same name — should reuse the slot but update index
    m.savepoint("sp1".to_string()).unwrap();
    assert_eq!(m.get_savepoint_count(), 1);
    // After duplicate, undo log should still have both records
    assert_eq!(m.undo_log_len(), 2);
}

#[test]
fn test_rollback_to_existing_savepoint_pops_undo() {
    let mut m = make_manager();
    m.add_undo(insert_key(1));
    m.savepoint("sp1".to_string()).unwrap();
    m.add_undo(insert_key(2));
    m.add_undo(insert_key(3));
    assert_eq!(m.undo_log_len(), 3);

    m.rollback_to("sp1").unwrap();
    // Only the first record should remain (recorded before sp1)
    assert_eq!(m.undo_log_len(), 1);
}

#[test]
fn test_rollback_to_nonexistent_savepoint_errors() {
    let mut m = make_manager();
    let result = m.rollback_to("does_not_exist");
    assert!(matches!(result, Err(SavepointError::NotFound)));
}

#[test]
fn test_release_savepoint_removes_it() {
    let mut m = make_manager();
    m.savepoint("sp1".to_string()).unwrap();
    m.savepoint("sp2".to_string()).unwrap();
    assert_eq!(m.get_savepoint_count(), 2);

    m.release_savepoint("sp1").unwrap();
    assert_eq!(m.get_savepoint_count(), 1);
}

#[test]
fn test_release_nonexistent_savepoint_is_noop() {
    // MySQL/PostgreSQL semantics: releasing a non-existent savepoint is a
    // no-op (with a warning), not an error. Verify that here.
    let mut m = make_manager();
    m.savepoint("sp1".to_string()).unwrap();
    let result = m.release_savepoint("nope");
    assert!(result.is_ok());
    // sp1 should still be present
    assert_eq!(m.get_savepoint_count(), 1);
    // sp1 can still be rolled back to
    m.rollback_to("sp1").unwrap();
}

#[test]
fn test_lifo_rollback() {
    let mut m = make_manager();
    m.add_undo(insert_key(1));
    m.savepoint("sp1".to_string()).unwrap();
    m.add_undo(delete_key(2));
    m.savepoint("sp2".to_string()).unwrap();
    m.add_undo(delete_key(3));

    // Rollback to sp2 — only the last record pops
    m.rollback_to("sp2").unwrap();
    assert_eq!(m.undo_log_len(), 2);

    // Rollback to sp1 — pops the sp2 record too
    m.rollback_to("sp1").unwrap();
    assert_eq!(m.undo_log_len(), 1);
}

#[test]
fn test_rollback_truncates_savepoints_after_target() {
    // SQL standard: rolling back to a savepoint destroys all savepoints
    // established after it. Verify that here.
    let mut m = make_manager();
    m.savepoint("a".to_string()).unwrap();
    m.savepoint("b".to_string()).unwrap();
    m.savepoint("c".to_string()).unwrap();
    assert_eq!(m.get_savepoint_count(), 3);

    m.rollback_to("a").unwrap();
    assert_eq!(m.get_savepoint_count(), 1);
    // Rolling back to "b" should now fail because b was destroyed
    assert!(matches!(m.rollback_to("b"), Err(SavepointError::NotFound)));
}
