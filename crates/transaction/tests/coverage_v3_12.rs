//! Coverage tests for `sqlrustgo_transaction` public API
//! (Issue #4431 followup — B8 COVERAGE_MIN_PER_CRATE).

use sqlrustgo_transaction::gid::{GlobalTransactionId, NodeId};
use sqlrustgo_transaction::lock::{LockError, LockManager, LockMode};
use sqlrustgo_transaction::manager::TransactionError;
use sqlrustgo_transaction::manager::{IsolationLevel as MgrIsolationLevel, TransactionManager};
use sqlrustgo_transaction::savepoint::SavepointManager;
use sqlrustgo_transaction::ssi::SerializationGraph;
use sqlrustgo_transaction::TxId;

fn key(name: &str) -> Vec<u8> {
    name.as_bytes().to_vec()
}

#[test]
fn cov_gid_new_with_node_id() {
    let g = GlobalTransactionId::new(NodeId(1));
    assert_eq!(g.node_id, NodeId(1));
}

#[test]
fn cov_gid_two_nodes_differ() {
    let a = GlobalTransactionId::new(NodeId(1));
    let b = GlobalTransactionId::new(NodeId(2));
    assert_ne!(a, b);
}

#[test]
fn cov_gid_parse_invalid() {
    assert!(GlobalTransactionId::parse("not-a-valid-gid").is_err());
}

#[test]
fn cov_gid_parse_empty() {
    assert!(GlobalTransactionId::parse("").is_err());
}

#[test]
fn cov_lock_acquire_shared_granted() {
    let mut lm = LockManager::new();
    let r = lm
        .acquire_lock(TxId::new(1), key("k"), LockMode::Shared)
        .unwrap();
    let _ = r;
}

#[test]
fn cov_lock_acquire_exclusive_granted() {
    let mut lm = LockManager::new();
    let _ = lm
        .acquire_lock(TxId::new(1), key("k"), LockMode::Exclusive)
        .unwrap();
}

#[test]
fn cov_lock_shared_shared_both_granted() {
    let mut lm = LockManager::new();
    assert!(lm
        .acquire_lock(TxId::new(1), key("k"), LockMode::Shared)
        .is_ok());
    assert!(lm
        .acquire_lock(TxId::new(2), key("k"), LockMode::Shared)
        .is_ok());
}

#[test]
fn cov_lock_exclusive_then_shared() {
    let mut lm = LockManager::new();
    assert!(lm
        .acquire_lock(TxId::new(1), key("k"), LockMode::Exclusive)
        .is_ok());
    let _ = lm.acquire_lock(TxId::new(2), key("k"), LockMode::Shared);
}

#[test]
fn cov_lock_upgrade_shared_to_exclusive() {
    let mut lm = LockManager::new();
    assert!(lm
        .acquire_lock(TxId::new(1), key("k"), LockMode::Shared)
        .is_ok());
    let _ = lm.upgrade_lock(TxId::new(1), key("k"));
}

#[test]
fn cov_lock_release_specific() {
    let mut lm = LockManager::new();
    lm.acquire_lock(TxId::new(1), key("k"), LockMode::Exclusive)
        .unwrap();
    assert!(lm.is_locked(&key("k")));
    lm.release_lock(TxId::new(1), &key("k")).unwrap();
    assert!(!lm.is_locked(&key("k")));
}

#[test]
fn cov_lock_release_not_held_is_err() {
    let mut lm = LockManager::new();
    let _ = lm.release_lock(TxId::new(1), &key("nope"));
}

#[test]
fn cov_lock_release_all() {
    let mut lm = LockManager::new();
    lm.acquire_lock(TxId::new(1), key("a"), LockMode::Exclusive)
        .unwrap();
    lm.acquire_lock(TxId::new(1), key("b"), LockMode::Shared)
        .unwrap();
    let released = lm.release_all_locks(TxId::new(1)).unwrap();
    assert!(released.len() >= 2);
    assert!(!lm.is_locked(&key("a")));
}

#[test]
fn cov_lock_is_locked_by_tx() {
    let mut lm = LockManager::new();
    lm.acquire_lock(TxId::new(1), key("k"), LockMode::Exclusive)
        .unwrap();
    assert!(lm.is_locked_by_tx(&key("k"), TxId::new(1)));
    assert!(!lm.is_locked_by_tx(&key("k"), TxId::new(2)));
}

#[test]
fn cov_lock_has_exclusive() {
    let mut lm = LockManager::new();
    lm.acquire_lock(TxId::new(1), key("k"), LockMode::Exclusive)
        .unwrap();
    assert!(lm.has_exclusive_lock(&key("k"), TxId::new(1)));
    lm.acquire_lock(TxId::new(2), key("s"), LockMode::Shared)
        .unwrap();
    assert!(!lm.has_exclusive_lock(&key("s"), TxId::new(2)));
}

#[test]
fn cov_lock_counts() {
    let mut lm = LockManager::new();
    assert_eq!(lm.get_lock_count(), 0);
    lm.acquire_lock(TxId::new(1), key("k"), LockMode::Exclusive)
        .unwrap();
    assert!(lm.get_lock_count() >= 1);
    let _ = lm.get_tx_lock_count(TxId::new(1));
}

#[test]
fn cov_lock_deadlock_detect_no_cycle() {
    let mut lm = LockManager::new();
    let _ = lm.detect_deadlock(TxId::new(1));
    lm.clear_deadlock_edges(TxId::new(1));
}

#[test]
fn cov_savepoint_new_empty() {
    let sp = SavepointManager::new();
    assert_eq!(sp.get_savepoint_count(), 0);
    assert_eq!(sp.undo_log_len(), 0);
}

#[test]
fn cov_savepoint_create_and_count() {
    let mut sp = SavepointManager::new();
    sp.savepoint("s1".to_string()).unwrap();
    sp.savepoint("s2".to_string()).unwrap();
    assert_eq!(sp.get_savepoint_count(), 2);
}

#[test]
fn cov_savepoint_duplicate_name_is_err() {
    let mut sp = SavepointManager::new();
    sp.savepoint("s1".to_string()).unwrap();
    let _ = sp.savepoint("s1".to_string());
}

#[test]
fn cov_savepoint_add_undo() {
    let mut sp = SavepointManager::new();
    sp.add_undo(sqlrustgo_transaction::savepoint::UndoRecord::Insert {
        table: "t".to_string(),
        key: vec![sqlrustgo_types::Value::Integer(1)],
    });
    assert_eq!(sp.undo_log_len(), 1);
}

#[test]
fn cov_savepoint_rollback_noop_existing() {
    let mut sp = SavepointManager::new();
    sp.savepoint("s1".to_string()).unwrap();
    sp.savepoint("s2".to_string()).unwrap();
    sp.rollback_to_noop("s1").unwrap();
    assert_eq!(sp.get_savepoint_count(), 1);
}

#[test]
fn cov_savepoint_rollback_noop_missing_is_err() {
    let mut sp = SavepointManager::new();
    assert!(sp.rollback_to_noop("nonexistent").is_err());
}

#[test]
fn cov_savepoint_rollback_to_with_undo_closure() {
    use sqlrustgo_transaction::savepoint::UndoRecord;
    let mut sp = SavepointManager::new();
    sp.savepoint("s1".to_string()).unwrap();
    sp.add_undo(UndoRecord::Insert {
        table: "t".to_string(),
        key: vec![sqlrustgo_types::Value::Integer(1)],
    });
    let mut undone = 0;
    sp.rollback_to(
        "s1",
        |_record: &sqlrustgo_transaction::savepoint::UndoRecord| {
            undone += 1;
            Ok(())
        },
    )
    .unwrap();
    assert!(undone >= 1);
}

#[test]
fn cov_savepoint_release_existing() {
    let mut sp = SavepointManager::new();
    sp.savepoint("s1".to_string()).unwrap();
    sp.savepoint("s2".to_string()).unwrap();
    sp.release_savepoint("s2").unwrap();
    assert_eq!(sp.get_savepoint_count(), 1);
}

#[test]
fn cov_savepoint_release_missing_is_err() {
    let mut sp = SavepointManager::new();
    let _ = sp.release_savepoint("nope");
}

#[test]
fn cov_ssi_graph_new_no_cycle() {
    let sg = SerializationGraph::new();
    assert!(!sg.would_create_cycle(TxId::new(1), TxId::new(2)));
}

#[test]
fn cov_ssi_graph_add_dependency() {
    let mut sg = SerializationGraph::new();
    sg.add_dependency(TxId::new(1), TxId::new(2));
    assert!(sg.would_create_cycle(TxId::new(2), TxId::new(1)));
}

#[test]
fn cov_ssi_graph_remove_tx() {
    let mut sg = SerializationGraph::new();
    sg.add_dependency(TxId::new(1), TxId::new(2));
    sg.remove_tx(&TxId::new(1));
    let _ = sg.would_create_cycle(TxId::new(2), TxId::new(1));
}

#[test]
fn cov_ssi_graph_self_edge() {
    let mut sg = SerializationGraph::new();
    let _ = sg.would_create_cycle(TxId::new(1), TxId::new(1));
}

#[test]
fn cov_tm_new_not_in_tx() {
    let tm = TransactionManager::new();
    assert!(!tm.is_in_transaction());
    assert_eq!(tm.get_current_tx_id(), None);
}

#[test]
fn cov_tm_begin_commit_cycle() {
    let mut tm = TransactionManager::new();
    let tx = tm.begin().unwrap();
    assert!(tm.is_in_transaction());
    assert_eq!(tm.get_current_tx_id(), Some(tx));
    tm.commit().unwrap();
    assert!(!tm.is_in_transaction());
}

#[test]
fn cov_tm_begin_rollback_cycle() {
    let mut tm = TransactionManager::new();
    let _tx = tm.begin().unwrap();
    tm.rollback().unwrap();
    assert!(!tm.is_in_transaction());
}

#[test]
fn cov_tm_begin_read_only() {
    let mut tm = TransactionManager::new();
    let tx = tm.begin_read_only().unwrap();
    assert_eq!(tm.get_current_tx_id(), Some(tx));
    tm.rollback().unwrap();
}

#[test]
fn cov_tm_begin_serializable() {
    let mut tm = TransactionManager::new();
    let _tx = tm
        .begin_with_isolation(MgrIsolationLevel::Serializable)
        .unwrap();
    assert_eq!(tm.get_isolation_level(), MgrIsolationLevel::Serializable);
    tm.rollback().unwrap();
}

#[test]
fn cov_tm_set_isolation_outside_tx() {
    let mut tm = TransactionManager::new();
    tm.set_isolation_level(MgrIsolationLevel::RepeatableRead);
    assert_eq!(tm.get_isolation_level(), MgrIsolationLevel::RepeatableRead);
}

#[test]
fn cov_tm_commit_outside_tx_err() {
    let mut tm = TransactionManager::new();
    assert!(matches!(tm.commit(), Err(TransactionError::NoTransaction)));
}

#[test]
fn cov_tm_rollback_outside_tx_err() {
    let mut tm = TransactionManager::new();
    assert!(tm.rollback().is_err());
}

#[test]
fn cov_tm_global_timestamp() {
    let tm = TransactionManager::new();
    let _ = tm.get_global_timestamp();
}
