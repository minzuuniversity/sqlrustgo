//! C01-05 · ca01_file_undo — FileStorage 事务不变量契约测试（INV-01/INV-02）
//!
//! 契约来源：docs/releases/v4.1.0/refactoring/TRANSACTION_INVARIANTS.md
//!   INV-01 rollback 精确恢复原多重集、每个 mutation 只撤销一次
//!   INV-02 相同行值独立 RowId（无主键表相同行删除回滚不得丢行，关联 #5908）
//! 语义：未知产品假设不强求 RED；失败即真实缺陷捕获（记入 RED 台账），不得弱化断言。

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
    let storage = Arc::new(RwLock::new(fs));
    ExecutionEngine::new(storage)
}

/// INV-01a：事务内 insert+delete+update 混合，ROLLBACK 后多重集与事务前完全一致（含重复行）
#[test]
fn inv01a_rollback_restores_exact_multiset() {
    let dir = tempfile::TempDir::new().unwrap();
    let mut e = file_engine(dir.path());
    e.execute("CREATE TABLE t (id INT, v INT)").unwrap();
    e.execute("INSERT INTO t VALUES (1,10),(2,20)").unwrap();
    e.execute("INSERT INTO t VALUES (1,10)").unwrap();
    let before = rows(&mut e, "SELECT id, v FROM t");
    assert_eq!(before.len(), 3, "seed: 2+1 duplicated rows");

    e.execute("BEGIN").unwrap();
    e.execute("INSERT INTO t VALUES (3,30)").unwrap();
    e.execute("DELETE FROM t WHERE id = 1").unwrap();
    e.execute("UPDATE t SET v = 99 WHERE id = 2").unwrap();
    e.execute("ROLLBACK").unwrap();

    let after = rows(&mut e, "SELECT id, v FROM t");
    let mut b = before.clone();
    let mut a = after.clone();
    b.sort();
    a.sort();
    assert_eq!(
        a, b,
        "INV-01: rollback must restore the exact pre-tx multiset"
    );
}

/// INV-01b：同事务内对同一行连续两次 UPDATE，ROLLBACK 恰好撤销一次恢复原值
#[test]
fn inv01b_mutation_revoked_exactly_once() {
    let dir = tempfile::TempDir::new().unwrap();
    let mut e = file_engine(dir.path());
    e.execute("CREATE TABLE t (id INT PRIMARY KEY, v INT)")
        .unwrap();
    e.execute("INSERT INTO t VALUES (1, 7)").unwrap();

    e.execute("BEGIN").unwrap();
    e.execute("UPDATE t SET v = 8 WHERE id = 1").unwrap();
    e.execute("UPDATE t SET v = 9 WHERE id = 1").unwrap();
    e.execute("ROLLBACK").unwrap();

    let after = rows(&mut e, "SELECT v FROM t WHERE id = 1");
    assert_eq!(
        after,
        vec![vec!["Integer(7)".to_string()]],
        "INV-01: exactly-once undo must restore the original value 7"
    );
}

/// INV-02 探针：无主键表两行完全相同 ⇒ 独立 RowId；删除一行回滚后必须仍是 2 行
/// （已知关联缺陷 #5908；若 RED 记入 RED 台账，不得弱化断言）
#[test]
fn inv02_identical_rows_have_independent_rowids() {
    let dir = tempfile::TempDir::new().unwrap();
    let mut e = file_engine(dir.path());
    e.execute("CREATE TABLE t (id INT, v INT)").unwrap();
    e.execute("INSERT INTO t VALUES (1, 10)").unwrap();
    e.execute("INSERT INTO t VALUES (1, 10)").unwrap();

    e.execute("BEGIN").unwrap();
    e.execute("DELETE FROM t WHERE id = 1").unwrap();
    e.execute("ROLLBACK").unwrap();

    let after = rows(&mut e, "SELECT id, v FROM t");
    assert_eq!(
        after.len(),
        2,
        "INV-02: identical rows must be independently addressable; rollback must restore (see #5908)"
    );
}
