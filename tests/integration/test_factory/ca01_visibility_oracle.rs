//! C01-14 · ca01_visibility_oracle — Savepoint 隔离与可见性 oracle（INV-02 / INV-04 / INV-13 / INV-15）
//!
//! 契约来源：docs/releases/v4.1.0/refactoring/TRANSACTION_INVARIANTS.md
//!   INV-02 相同行值独立 RowId；UPDATE 主键不改 RowId
//!   INV-04 savepoint cursor 仅属本 tx，部分回滚不污染顶层恢复
//!   INV-13 approved 隔离语义由真实并发 + 版本/物理主账联合证明
//!   INV-15 索引/聚类/cache 与同一 committed RowStore 发布/撤销一致
//!
//! 语义分类（Owner 2026-11-21 §一）：
//!   本文件为 **Contract Test**（验证正确产品行为）——断言方向 = 目标契约；
//!   当前实现不满足则 RED（记入 RED Ledger），**不得**改写预期换取绿色。
//!   另含 **Regression Test** 子例（保护已正确行为）。
//!
//! backend：FileStorage（生产路径）。

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::engine::StorageEngine;
use sqlrustgo_storage::FileStorage;
use std::sync::Arc;

fn rows(e: &mut ExecutionEngine<impl StorageEngine + 'static>, sql: &str) -> Vec<Vec<String>> {
    e.execute(sql)
        .unwrap_or_else(|err| panic!("{} failed: {:?}", sql, err))
        .rows
        .iter()
        .map(|r| r.iter().map(|v| format!("{:?}", v)).collect())
        .collect()
}

fn file_engine(dir: &std::path::Path) -> ExecutionEngine<FileStorage> {
    let fs = FileStorage::new(dir.to_path_buf()).unwrap();
    ExecutionEngine::new(Arc::new(RwLock::new(fs)))
}

/// INV-04a（Contract）：SAVEPOINT 后的部分回滚只撤销该 savepoint 之后的变化，
/// savepoint 之前的变化必须保留；顶层 ROLLBACK 再撤销剩余部分。
#[test]
fn inv04a_rollback_to_savepoint_preserves_earlier_work() {
    let dir = tempfile::TempDir::new().unwrap();
    let mut e = file_engine(dir.path());
    e.execute("CREATE TABLE t (id INT PRIMARY KEY, v INT)").unwrap();
    e.execute("INSERT INTO t VALUES (1, 10)").unwrap();

    e.execute("BEGIN").unwrap();
    e.execute("INSERT INTO t VALUES (2, 20)").unwrap(); // savepoint 之前
    e.execute("SAVEPOINT sp1").unwrap();
    e.execute("INSERT INTO t VALUES (3, 30)").unwrap(); // savepoint 之后
    e.execute("ROLLBACK TO SAVEPOINT sp1").unwrap();

    let mid = rows(&mut e, "SELECT id FROM t");
    assert_eq!(
        mid.len(),
        2,
        "INV-04: ROLLBACK TO SAVEPOINT must undo only work after the savepoint \
         (rows 1,2 remain; row 3 must be gone)"
    );

    // 顶层 ROLLBACK 撤销 savepoint 之前的插入 ⇒ 回到事务前
    e.execute("ROLLBACK").unwrap();
    let after = rows(&mut e, "SELECT id FROM t");
    assert_eq!(
        after.len(),
        1,
        "INV-04: top-level ROLLBACK must still undo the pre-savepoint insert"
    );
}

/// INV-04b（Contract）：另一个事务的 SAVEPOINT 不得影响本事务的恢复链。
/// 两个独立引擎（共享存储）各自开启 savepoint；A 的部分回滚不得污染 B 的可见状态。
#[test]
fn inv04b_savepoint_cursor_is_transaction_local() {
    let dir = tempfile::TempDir::new().unwrap();
    let fs = FileStorage::new(dir.path().to_path_buf()).unwrap();
    let storage = Arc::new(RwLock::new(fs));
    let mut a = ExecutionEngine::new(storage.clone());
    let mut b = ExecutionEngine::new(storage.clone());

    a.execute("CREATE TABLE t (id INT PRIMARY KEY, v INT)").unwrap();
    a.execute("INSERT INTO t VALUES (1, 10)").unwrap();

    a.execute("BEGIN").unwrap();
    a.execute("SAVEPOINT sa").unwrap();
    a.execute("INSERT INTO t VALUES (2, 20)").unwrap();

    b.execute("BEGIN").unwrap();
    b.execute("SAVEPOINT sb").unwrap();
    b.execute("INSERT INTO t VALUES (3, 30)").unwrap();
    b.execute("ROLLBACK TO SAVEPOINT sb").unwrap();

    // A 的 savepoint 链不受 B 影响：A 回滚到 sa 后只应撤销自身 sa 之后的插入
    a.execute("ROLLBACK TO SAVEPOINT sa").unwrap();
    let a_view = rows(&mut a, "SELECT id FROM t");
    assert_eq!(
        a_view.len(),
        1,
        "INV-04: transaction-local savepoint cursor — A must see only its own post-savepoint work undone"
    );

    b.execute("ROLLBACK").unwrap();
    a.execute("ROLLBACK").unwrap();
}

/// INV-02（Contract）：UPDATE 主键不得改变行身份——UPDATE 改主键后再回滚，
/// 原主键值必须完整恢复（不是"新行 + 删旧行"）。
#[test]
fn inv02_update_primary_key_rollback_restores_original_key() {
    let dir = tempfile::TempDir::new().unwrap();
    let mut e = file_engine(dir.path());
    e.execute("CREATE TABLE t (id INT PRIMARY KEY, v INT)").unwrap();
    e.execute("INSERT INTO t VALUES (1, 10)").unwrap();
    let before = rows(&mut e, "SELECT id, v FROM t");

    e.execute("BEGIN").unwrap();
    e.execute("UPDATE t SET id = 99 WHERE id = 1").unwrap();
    e.execute("ROLLBACK").unwrap();

    assert_eq!(
        rows(&mut e, "SELECT id, v FROM t"),
        before,
        "INV-02: UPDATE of primary key must not change row identity; rollback restores the original key"
    );
}

/// INV-13（Contract）：事务内未提交状态不得对其他连接可见（read-committed 下限）。
/// 本例以同存储双引擎的可见性为判据（真实并发语义由 INV-13 的并发矩阵另行覆盖）。
#[test]
fn inv13_uncommitted_state_invisible_to_peer_engine() {
    let dir = tempfile::TempDir::new().unwrap();
    let fs = FileStorage::new(dir.path().to_path_buf()).unwrap();
    let storage = Arc::new(RwLock::new(fs));
    let mut a = ExecutionEngine::new(storage.clone());
    let mut b = ExecutionEngine::new(storage.clone());

    a.execute("CREATE TABLE t (id INT PRIMARY KEY, v INT)").unwrap();
    a.execute("INSERT INTO t VALUES (1, 10)").unwrap();

    a.execute("BEGIN").unwrap();
    a.execute("INSERT INTO t VALUES (2, 20)").unwrap(); // 未提交
    let peer_view = rows(&mut b, "SELECT id FROM t");
    assert_eq!(
        peer_view.len(),
        1,
        "INV-13: uncommitted state must not be visible to another connection (read-committed floor)"
    );

    a.execute("COMMIT").unwrap();
    assert_eq!(
        rows(&mut b, "SELECT id FROM t").len(),
        2,
        "INV-13: after COMMIT the row must become visible to the peer connection"
    );
}

/// INV-15（Regression）：已提交状态在索引投影与主 RowStore 上必须一致（保护既有正确行为）。
#[test]
fn inv15_regression_index_and_rowstore_agree_at_committed_state() {
    let dir = tempfile::TempDir::new().unwrap();
    let mut e = file_engine(dir.path());
    e.execute("CREATE TABLE t (id INT PRIMARY KEY, v INT)").unwrap();
    e.execute("INSERT INTO t VALUES (1, 10), (2, 20), (3, 30)").unwrap();
    e.execute("CREATE INDEX idx_t_v ON t (v)").unwrap();

    let indexed = rows(&mut e, "SELECT id FROM t WHERE v = 20");
    assert_eq!(
        indexed,
        vec![vec!["Integer(2)".to_string()]],
        "INV-15 (regression): index projection must agree with the committed RowStore"
    );
}
