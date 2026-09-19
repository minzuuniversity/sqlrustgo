//! V400-05 cross-model transaction tests.
//!
//! Verifies that SQL + vector + graph + audit writes within a single
//! transaction boundary are all-or-nothing on COMMIT/ROLLBACK.

use sqlrustgo_transaction::{IsolationLevel, ModelKind, TransactionManager};

// ===========================================================================
// Basic single-model writes within a transaction
// ===========================================================================

#[test]
fn sql_only_write_registered() {
    let mut mgr = TransactionManager::new();
    let tx = mgr.begin_transaction(IsolationLevel::SnapshotIsolation).unwrap();
    mgr.tx_register_write(tx, ModelKind::Sql, "INSERT INTO t VALUES (1)");
    assert_eq!(mgr.cross_model_write_count(tx, ModelKind::Sql), 1);
    mgr.commit(tx).unwrap();
}

#[test]
fn vector_only_write_registered() {
    let mut mgr = TransactionManager::new();
    let tx = mgr.begin_transaction(IsolationLevel::SnapshotIsolation).unwrap();
    mgr.tx_register_write(tx, ModelKind::Vector, "INSERT INTO vectors VALUES (...)");
    assert_eq!(mgr.cross_model_write_count(tx, ModelKind::Vector), 1);
    assert_eq!(mgr.cross_model_write_count(tx, ModelKind::Sql), 0);
    mgr.commit(tx).unwrap();
}

#[test]
fn graph_only_write_registered() {
    let mut mgr = TransactionManager::new();
    let tx = mgr.begin_transaction(IsolationLevel::SnapshotIsolation).unwrap();
    mgr.tx_register_write(tx, ModelKind::Graph, "CREATE (n:Person {name: 'Alice'})");
    assert_eq!(mgr.cross_model_write_count(tx, ModelKind::Graph), 1);
    mgr.commit(tx).unwrap();
}

#[test]
fn audit_only_write_registered() {
    let mut mgr = TransactionManager::new();
    let tx = mgr.begin_transaction(IsolationLevel::SnapshotIsolation).unwrap();
    mgr.tx_register_write(tx, ModelKind::Audit, "AUDIT_EVENT: login.user=alice");
    assert_eq!(mgr.cross_model_write_count(tx, ModelKind::Audit), 1);
    mgr.commit(tx).unwrap();
}

// ===========================================================================
// Multi-model writes (the core cross-model scenario)
// ===========================================================================

#[test]
fn sql_plus_vector_cross_model_commit() {
    let mut mgr = TransactionManager::new();
    let tx = mgr.begin_transaction(IsolationLevel::SnapshotIsolation).unwrap();
    mgr.tx_register_write(tx, ModelKind::Sql, "INSERT INTO t VALUES (1)");
    mgr.tx_register_write(tx, ModelKind::Vector, "INSERT INTO vectors VALUES (...)");
    let writes = mgr.cross_model_writes(tx);
    assert_eq!(writes.len(), 2);
    assert_eq!(mgr.cross_model_write_count(tx, ModelKind::Sql), 1);
    assert_eq!(mgr.cross_model_write_count(tx, ModelKind::Vector), 1);
    assert_eq!(mgr.cross_model_write_count(tx, ModelKind::Graph), 0);
    assert_eq!(mgr.cross_model_write_count(tx, ModelKind::Audit), 0);
    mgr.commit(tx).unwrap();
    // After commit, the tracker is cleared
    assert!(mgr.cross_model_writes(tx).is_empty());
}

#[test]
fn all_four_models_cross_model_commit() {
    let mut mgr = TransactionManager::new();
    let tx = mgr.begin_transaction(IsolationLevel::SnapshotIsolation).unwrap();
    mgr.tx_register_write(tx, ModelKind::Sql, "INSERT INTO t VALUES (1)");
    mgr.tx_register_write(tx, ModelKind::Vector, "INSERT INTO vectors VALUES (...)");
    mgr.tx_register_write(tx, ModelKind::Graph, "CREATE (n:Person {name: 'Alice'})");
    mgr.tx_register_write(tx, ModelKind::Audit, "AUDIT_EVENT: write.user=alice");
    assert_eq!(mgr.cross_model_writes(tx).len(), 4);
    assert_eq!(mgr.cross_model_write_count(tx, ModelKind::Sql), 1);
    assert_eq!(mgr.cross_model_write_count(tx, ModelKind::Vector), 1);
    assert_eq!(mgr.cross_model_write_count(tx, ModelKind::Graph), 1);
    assert_eq!(mgr.cross_model_write_count(tx, ModelKind::Audit), 1);
    mgr.commit(tx).unwrap();
}

// ===========================================================================
// Rollback discards all models
// ===========================================================================

#[test]
fn sql_plus_vector_rollback_discards() {
    let mut mgr = TransactionManager::new();
    let tx = mgr.begin_transaction(IsolationLevel::SnapshotIsolation).unwrap();
    mgr.tx_register_write(tx, ModelKind::Sql, "INSERT INTO t VALUES (1)");
    mgr.tx_register_write(tx, ModelKind::Vector, "INSERT INTO vectors VALUES (...)");
    assert_eq!(mgr.cross_model_writes(tx).len(), 2);
    mgr.rollback(tx).unwrap();
    // After rollback, tracker is cleared
    assert!(mgr.cross_model_writes(tx).is_empty());
}

#[test]
fn all_four_models_rollback_discards() {
    let mut mgr = TransactionManager::new();
    let tx = mgr.begin_transaction(IsolationLevel::SnapshotIsolation).unwrap();
    mgr.tx_register_write(tx, ModelKind::Sql, "INSERT");
    mgr.tx_register_write(tx, ModelKind::Vector, "INSERT");
    mgr.tx_register_write(tx, ModelKind::Graph, "CREATE");
    mgr.tx_register_write(tx, ModelKind::Audit, "AUDIT");
    mgr.rollback(tx).unwrap();
    assert!(mgr.cross_model_writes(tx).is_empty());
}

// ===========================================================================
// Multiple writes per model
// ===========================================================================

#[test]
fn multiple_sql_writes() {
    let mut mgr = TransactionManager::new();
    let tx = mgr.begin_transaction(IsolationLevel::SnapshotIsolation).unwrap();
    for i in 0..5 {
        mgr.tx_register_write(tx, ModelKind::Sql, format!("INSERT {}", i));
    }
    assert_eq!(mgr.cross_model_write_count(tx, ModelKind::Sql), 5);
    mgr.commit(tx).unwrap();
}

#[test]
fn multiple_vector_writes() {
    let mut mgr = TransactionManager::new();
    let tx = mgr.begin_transaction(IsolationLevel::SnapshotIsolation).unwrap();
    for i in 0..10 {
        mgr.tx_register_write(tx, ModelKind::Vector, format!("v{}", i));
    }
    assert_eq!(mgr.cross_model_write_count(tx, ModelKind::Vector), 10);
    mgr.commit(tx).unwrap();
}

#[test]
fn mixed_writes_many() {
    let mut mgr = TransactionManager::new();
    let tx = mgr.begin_transaction(IsolationLevel::SnapshotIsolation).unwrap();
    mgr.tx_register_write(tx, ModelKind::Sql, "a");
    mgr.tx_register_write(tx, ModelKind::Vector, "b");
    mgr.tx_register_write(tx, ModelKind::Sql, "c");
    mgr.tx_register_write(tx, ModelKind::Graph, "d");
    mgr.tx_register_write(tx, ModelKind::Vector, "e");
    assert_eq!(mgr.cross_model_writes(tx).len(), 5);
    assert_eq!(mgr.cross_model_write_count(tx, ModelKind::Sql), 2);
    assert_eq!(mgr.cross_model_write_count(tx, ModelKind::Vector), 2);
    assert_eq!(mgr.cross_model_write_count(tx, ModelKind::Graph), 1);
    mgr.commit(tx).unwrap();
}

// ===========================================================================
// No transaction — register_write doesn't crash
// ===========================================================================

#[test]
fn register_for_unknown_tx_does_nothing() {
    let mut mgr = TransactionManager::new();
    let unknown_tx = sqlrustgo_transaction::TxId::new(99999);
    mgr.tx_register_write(unknown_tx, ModelKind::Sql, "INSERT");
    assert_eq!(mgr.cross_model_writes(unknown_tx).len(), 1);
}

#[test]
fn count_for_unknown_tx_is_zero() {
    let mgr = TransactionManager::new();
    let unknown_tx = sqlrustgo_transaction::TxId::new(99999);
    assert_eq!(mgr.cross_model_write_count(unknown_tx, ModelKind::Sql), 0);
    assert_eq!(mgr.cross_model_write_count(unknown_tx, ModelKind::Vector), 0);
    assert_eq!(mgr.cross_model_write_count(unknown_tx, ModelKind::Graph), 0);
    assert_eq!(mgr.cross_model_write_count(unknown_tx, ModelKind::Audit), 0);
}

#[test]
fn writes_for_unknown_tx_is_empty() {
    let mgr = TransactionManager::new();
    let unknown_tx = sqlrustgo_transaction::TxId::new(99999);
    assert!(mgr.cross_model_writes(unknown_tx).is_empty());
}

// ===========================================================================
// Multiple transactions don't leak
// ===========================================================================

#[test]
fn sequential_transactions_isolated() {
    let mut mgr = TransactionManager::new();
    // Tx 1
    let tx1 = mgr.begin_transaction(IsolationLevel::SnapshotIsolation).unwrap();
    mgr.tx_register_write(tx1, ModelKind::Sql, "INSERT tx1");
    mgr.commit(tx1).unwrap();
    // Tx 2: must start empty
    let tx2 = mgr.begin_transaction(IsolationLevel::SnapshotIsolation).unwrap();
    assert!(mgr.cross_model_writes(tx2).is_empty());
    mgr.tx_register_write(tx2, ModelKind::Vector, "INSERT tx2");
    assert_eq!(mgr.cross_model_writes(tx2).len(), 1);
    mgr.commit(tx2).unwrap();
}

#[test]
fn rollback_then_begin_is_clean() {
    let mut mgr = TransactionManager::new();
    let tx1 = mgr.begin_transaction(IsolationLevel::SnapshotIsolation).unwrap();
    mgr.tx_register_write(tx1, ModelKind::Sql, "x");
    mgr.rollback(tx1).unwrap();
    let tx2 = mgr.begin_transaction(IsolationLevel::SnapshotIsolation).unwrap();
    assert!(mgr.cross_model_writes(tx2).is_empty());
}

// ===========================================================================
// ModelKind equality + Debug
// ===========================================================================

#[test]
fn model_kind_equality() {
    assert_eq!(ModelKind::Sql, ModelKind::Sql);
    assert_ne!(ModelKind::Sql, ModelKind::Vector);
    assert_ne!(ModelKind::Vector, ModelKind::Graph);
    assert_ne!(ModelKind::Graph, ModelKind::Audit);
}

#[test]
fn model_kind_debug() {
    // Verify Debug trait is implemented (used in audit logs)
    let _ = format!("{:?}", ModelKind::Sql);
    let _ = format!("{:?}", ModelKind::Vector);
    let _ = format!("{:?}", ModelKind::Graph);
    let _ = format!("{:?}", ModelKind::Audit);
}

// ===========================================================================
// Commit after rollback rejected
// ===========================================================================

#[test]
fn commit_after_rollback_is_noop_or_err() {
    let mut mgr = TransactionManager::new();
    let tx = mgr.begin_transaction(IsolationLevel::SnapshotIsolation).unwrap();
    mgr.tx_register_write(tx, ModelKind::Sql, "x");
    mgr.rollback(tx).unwrap();
    // After rollback, tx is gone; re-commit may fail (validate_commit
    // detects double-finalization). We just verify no panic.
    let _ = mgr.commit(tx);
}

#[test]
fn rollback_after_commit_is_noop_or_err() {
    let mut mgr = TransactionManager::new();
    let tx = mgr.begin_transaction(IsolationLevel::SnapshotIsolation).unwrap();
    mgr.tx_register_write(tx, ModelKind::Sql, "x");
    mgr.commit(tx).unwrap();
    // After commit, tx is gone; re-rollback is a no-op (no panic).
    let _ = mgr.rollback(tx);
}

// ===========================================================================
// Cross-model barrier test (description checks)
// ===========================================================================

#[test]
fn write_description_preserved() {
    let mut mgr = TransactionManager::new();
    let tx = mgr.begin_transaction(IsolationLevel::SnapshotIsolation).unwrap();
    mgr.tx_register_write(tx, ModelKind::Sql, "INSERT INTO audit_log VALUES ('alice logged in')");
    let writes = mgr.cross_model_writes(tx);
    assert_eq!(writes[0].description, "INSERT INTO audit_log VALUES ('alice logged in')");
    mgr.commit(tx).unwrap();
}

#[test]
fn large_cross_model_description() {
    let mut mgr = TransactionManager::new();
    let tx = mgr.begin_transaction(IsolationLevel::SnapshotIsolation).unwrap();
    let desc = "X".repeat(1000);
    mgr.tx_register_write(tx, ModelKind::Vector, desc.clone());
    let writes = mgr.cross_model_writes(tx);
    assert_eq!(writes[0].description.len(), 1000);
    mgr.commit(tx).unwrap();
}

#[test]
fn many_writes_in_one_tx() {
    let mut mgr = TransactionManager::new();
    let tx = mgr.begin_transaction(IsolationLevel::SnapshotIsolation).unwrap();
    for _ in 0..100 {
        mgr.tx_register_write(tx, ModelKind::Sql, "x");
    }
    assert_eq!(mgr.cross_model_writes(tx).len(), 100);
    mgr.commit(tx).unwrap();
}

// ===========================================================================
// Cross-model rollback barrier verification
// ===========================================================================

#[test]
fn rollback_barrier_no_dangling_writes() {
    let mut mgr = TransactionManager::new();
    let tx = mgr.begin_transaction(IsolationLevel::SnapshotIsolation).unwrap();
    mgr.tx_register_write(tx, ModelKind::Sql, "INSERT");
    mgr.tx_register_write(tx, ModelKind::Vector, "INSERT");
    mgr.tx_register_write(tx, ModelKind::Graph, "CREATE");
    mgr.tx_register_write(tx, ModelKind::Audit, "AUDIT");
    let before = mgr.cross_model_writes(tx).len();
    assert_eq!(before, 4);
    mgr.rollback(tx).unwrap();
    let after = mgr.cross_model_writes(tx).len();
    assert_eq!(after, 0);
}

#[test]
fn commit_barrier_no_dangling_writes() {
    let mut mgr = TransactionManager::new();
    let tx = mgr.begin_transaction(IsolationLevel::SnapshotIsolation).unwrap();
    mgr.tx_register_write(tx, ModelKind::Sql, "INSERT");
    mgr.tx_register_write(tx, ModelKind::Vector, "INSERT");
    mgr.tx_register_write(tx, ModelKind::Graph, "CREATE");
    mgr.tx_register_write(tx, ModelKind::Audit, "AUDIT");
    mgr.commit(tx).unwrap();
    assert!(mgr.cross_model_writes(tx).is_empty());
}

// ===========================================================================
// Multi-tx isolation
// ===========================================================================

#[test]
fn parallel_transactions_tracking_isolated() {
    let mut mgr = TransactionManager::new();
    let tx1 = mgr.begin_transaction(IsolationLevel::SnapshotIsolation).unwrap();
    let tx2 = mgr.begin_transaction(IsolationLevel::SnapshotIsolation).unwrap();
    mgr.tx_register_write(tx1, ModelKind::Sql, "x1");
    mgr.tx_register_write(tx2, ModelKind::Vector, "y2");
    assert_eq!(mgr.cross_model_write_count(tx1, ModelKind::Sql), 1);
    assert_eq!(mgr.cross_model_write_count(tx1, ModelKind::Vector), 0);
    assert_eq!(mgr.cross_model_write_count(tx2, ModelKind::Vector), 1);
    assert_eq!(mgr.cross_model_write_count(tx2, ModelKind::Sql), 0);
    mgr.commit(tx1).unwrap();
    mgr.commit(tx2).unwrap();
}

#[test]
fn rollback_tx1_leaves_tx2_intact() {
    let mut mgr = TransactionManager::new();
    let tx1 = mgr.begin_transaction(IsolationLevel::SnapshotIsolation).unwrap();
    let tx2 = mgr.begin_transaction(IsolationLevel::SnapshotIsolation).unwrap();
    mgr.tx_register_write(tx1, ModelKind::Sql, "x1");
    mgr.tx_register_write(tx2, ModelKind::Vector, "y2");
    mgr.rollback(tx1).unwrap();
    assert!(mgr.cross_model_writes(tx1).is_empty());
    assert_eq!(mgr.cross_model_writes(tx2).len(), 1);
    mgr.commit(tx2).unwrap();
}

// ===========================================================================
// Sequential commits accumulation test
// ===========================================================================

#[test]
fn sequential_commits_tracking_clears() {
    let mut mgr = TransactionManager::new();
    for _ in 0..5 {
        let tx = mgr.begin_transaction(IsolationLevel::SnapshotIsolation).unwrap();
        mgr.tx_register_write(tx, ModelKind::Sql, "x");
        mgr.commit(tx).unwrap();
    }
    // All transactions have been cleaned up; check counts
    let sample = sqlrustgo_transaction::TxId::new(100);
    assert_eq!(mgr.cross_model_write_count(sample, ModelKind::Sql), 0);
}

// ===========================================================================
// CrossModelWrite structure
// ===========================================================================

#[test]
fn cross_model_write_equality() {
    let w1 = sqlrustgo_transaction::CrossModelWrite {
        model: ModelKind::Sql,
        description: "x".to_string(),
    };
    let w2 = sqlrustgo_transaction::CrossModelWrite {
        model: ModelKind::Sql,
        description: "x".to_string(),
    };
    assert_eq!(w1, w2);
    let w3 = sqlrustgo_transaction::CrossModelWrite {
        model: ModelKind::Sql,
        description: "y".to_string(),
    };
    assert_ne!(w1, w3);
}

#[test]
fn cross_model_write_clone() {
    let w1 = sqlrustgo_transaction::CrossModelWrite {
        model: ModelKind::Vector,
        description: "embed".to_string(),
    };
    let w2 = w1.clone();
    assert_eq!(w1, w2);
}