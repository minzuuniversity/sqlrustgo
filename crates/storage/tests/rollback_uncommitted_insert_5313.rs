//! #5313 ROLLBACK 回归红测试 — 回滚后未提交 INSERT 行不得残留 inner。
//!
//! #5313 把 `MvccStorage::delete_in_db` 从 "先 inner 真删（`delete_collect_pks_in_db`）
//! 再按 PK tombstone MVCC" 改为 `delete_scoped`：只对 MVCC `matched_visible`
//! 的行调 inner delete。回滚 INSERT 时 undo 记录 `delete(table, key)`，
//! 但未提交行不在 MVCC 可见集 → matched=0 → inner delete 被跳过 →
//! 该行留在 `FileStorage::insert_buffer`，被 `scan_in_tx_db` 的 merge
//! （`inner_rows.len() > out.len()` 且 reader_tx 无 snapshot）合回结果集。
//!
//! 本测试在生产栈（ParallelWalStorage<MvccStorage<FileStorage>>）上复现
//! mysql-server `rollback_leaves_other_connections_commits_intact` 的存储层
//! 最小路径：BEGIN → INSERT → ROLLBACK → 无事务 SELECT。
//!
//! #5313 修复前（基线）：inner 真删，回滚后 SELECT 为空 → PASS。
//! #5313 修复后（当前）：inner 跳过，回滚后 SELECT 残留行 → FAIL。

use sqlrustgo_storage::{
    BoxStorageEngine, ColumnDefinition, FileStorage, MemoryWalManager, MvccStorage,
    ParallelWalStorage, StorageEngine, TableInfo,
};
use sqlrustgo_types::Value;
use std::path::PathBuf;

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("rb_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn columns() -> TableInfo {
    TableInfo {
        name: "t".to_string(),
        columns: vec![
            ColumnDefinition {
                name: "id".to_string(),
                data_type: "INT".to_string(),
                primary_key: true,
                ..Default::default()
            },
            ColumnDefinition {
                name: "v".to_string(),
                data_type: "TEXT".to_string(),
                ..Default::default()
            },
        ],
        ..Default::default()
    }
}

fn stack(tag: &str) -> BoxStorageEngine {
    let file = FileStorage::new(dir(tag)).expect("FileStorage");
    let mvcc = MvccStorage::new(file);
    BoxStorageEngine::new(ParallelWalStorage::new(mvcc, MemoryWalManager::new()))
}

/// executor undo 路径回滚一个未提交的 INSERT 后，该行不得出现在无事务 SELECT 中。
///
/// 复现 executor `rollback_transaction` 的存储层序列：
///   1. `set_current_tx_id_shared(tx_id)` + `rollback_transaction_lockfree_for(tx_id)`
///   2. `rollback_with_undo` 对 Insert undo 调 `storage.delete(table, key)`
#[test]
fn executor_undo_rollback_discards_uncommitted_insert() {
    let mut s = stack("executor_undo");
    s.create_database("default").expect("create db");
    s.create_table_in_db("default", &columns())
        .expect("create table");

    let tx = s.begin_transaction().expect("begin");
    s.insert_in_db(
        "default",
        "t",
        vec![vec![Value::Integer(10), Value::Text("a".to_string())]],
    )
    .expect("insert");

    // 复现 executor rollback_transaction 前半段：释放 tx 状态 + discard buffer。
    s.set_current_tx_id_shared(tx);
    let _ = s.rollback_transaction_lockfree_for(tx);

    // 复现 rollback_with_undo 对 Insert undo 的操作：delete(table, key)。
    // key = [Integer(10)]（PK 列值）。
    s.delete("t", &[Value::Integer(10)]).expect("undo delete");

    // 回滚后：无事务 SELECT（reader_tx=0）不得看到该行。
    let after = s.scan_in_tx_db("default", "t", 0).expect("scan after undo");
    assert!(
        after.is_empty(),
        "executor undo rollback must discard uncommitted INSERT; residual rows in inner: {after:?}"
    );
}

/// 对照：已提交的 INSERT 在 COMMIT 后仍可见（确认测试栈事务路径正常）。
#[test]
fn commit_persists_insert() {
    let mut s = stack("committed_insert");
    s.create_database("default").expect("create db");
    s.create_table_in_db("default", &columns())
        .expect("create table");

    let _tx = s.begin_transaction().expect("begin");
    s.insert_in_db(
        "default",
        "t",
        vec![vec![Value::Integer(20), Value::Text("b".to_string())]],
    )
    .expect("insert");
    s.commit_transaction().expect("commit");

    let after = s
        .scan_in_tx_db("default", "t", 0)
        .expect("scan after commit");
    assert_eq!(
        after.len(),
        1,
        "committed INSERT must survive COMMIT: {after:?}"
    );
}
