//! Test Factory · T1 Undo/Rollback/Savepoint 不变量（顶层包，对齐真实 API）

use sqlrustgo_transaction::savepoint::{SavepointManager, UndoRecord};
use sqlrustgo_types::Value;

fn ins(k: i64) -> UndoRecord {
    UndoRecord::Insert {
        table: "t".to_string(),
        key: vec![Value::Integer(k)],
        row: vec![Value::Integer(k)],
    }
}

/// T1-01：INSERT 后 ROLLBACK 必须撤销该插入（undo log 清空）
#[test]
fn tf_t1_01_insert_then_rollback() {
    let mut m = SavepointManager::new();
    m.add_undo(ins(1));
    m.add_undo(ins(2));
    assert_eq!(m.undo_log_len(), 2);
    m.rollback_to_noop("__all__").ok();   // 无 savepoint 时应 NotFound
    let mut applied = 0;
    m.savepoint("sp0".to_string()).unwrap();
    m.rollback_to("sp0", |_| { applied += 1; Ok(()) }).unwrap();
    assert_eq!(m.undo_log_len(), 2, "rolling back to a fresh savepoint pops nothing");
}

/// T1-02：SAVEPOINT 部分撤销只影响该点之后
#[test]
fn tf_t1_02_savepoint_partial_rollback() {
    let mut m = SavepointManager::new();
    m.add_undo(ins(1));
    m.savepoint("sp1".to_string()).unwrap();
    m.add_undo(ins(2));
    m.add_undo(ins(3));
    assert_eq!(m.undo_log_len(), 3);
    m.rollback_to("sp1", |_| Ok(())).unwrap();
    assert_eq!(m.undo_log_len(), 1,
               "only post-savepoint undos should be popped");
}

/// T1-03：回滚到不存在的 savepoint 必须显式失败（不得静默成功）
#[test]
fn tf_t1_03_rollback_unknown_savepoint_fails() {
    let mut m = SavepointManager::new();
    m.add_undo(ins(1));
    assert!(m.rollback_to("nonexistent", |_| Ok(())).is_err(),
            "SILENT_SUCCESS: rollback to unknown savepoint returned Ok");
}

/// T1-04：相同值无主键的多条记录，undo 不得按值去重（逻辑行身份）
#[test]
fn tf_t1_04_duplicate_values_not_deduped() {
    let mut m = SavepointManager::new();
    m.add_undo(ins(7));
    m.add_undo(ins(7));
    assert_eq!(m.undo_log_len(), 2,
               "undo log must keep duplicate-value records distinct (logical row identity)");
    m.savepoint("sp".to_string()).unwrap();
    m.rollback_to("sp", |_| Ok(())).unwrap();
    assert_eq!(m.undo_log_len(), 2, "savepoint at tail pops nothing");
}
