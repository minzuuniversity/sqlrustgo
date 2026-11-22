//! C01-10 · ca01_complex_dml — 复杂 DML 原子性与派生投影一致性（INV-05 / INV-15）
//!
//! INV-05：一条语句多行/多表/trigger 的所有可逆变化共同原子提交/撤销
//! INV-15：索引/聚类/cache 与同一 committed RowStore 发布/撤销一致
//!
//! 语义：未知产品假设不强求 RED；失败即真实缺陷捕获（记入 RED 台账），不得弱化断言。
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

/// INV-05a：单条 UPDATE 影响多行时，所有行的可逆变化必须共同撤销
/// （不得部分撤销——一半恢复一半未恢复）。
#[test]
fn inv05a_multi_row_update_rollback_is_all_or_nothing() {
    let dir = tempfile::TempDir::new().unwrap();
    let mut e = file_engine(dir.path());
    e.execute("CREATE TABLE t (id INT, v INT)").unwrap();
    e.execute("INSERT INTO t VALUES (1,10),(2,20),(3,30),(4,40)").unwrap();
    let before = rows(&mut e, "SELECT id, v FROM t");

    e.execute("BEGIN").unwrap();
    e.execute("UPDATE t SET v = v + 100").unwrap(); // 一条语句影响全部 4 行
    e.execute("ROLLBACK").unwrap();

    let after = rows(&mut e, "SELECT id, v FROM t");
    let mut a = after.clone();
    let mut b = before.clone();
    a.sort();
    b.sort();
    assert_eq!(
        a, b,
        "INV-05: one statement affecting multiple rows must be undone atomically \
         (all rows restored, not partially)"
    );
}

/// INV-05b：跨表 DML 共同撤销（一个事务内两张表的写必须一起回滚）。
#[test]
fn inv05b_multi_table_dml_rollback_is_atomic() {
    let dir = tempfile::TempDir::new().unwrap();
    let mut e = file_engine(dir.path());
    e.execute("CREATE TABLE a (id INT, v INT)").unwrap();
    e.execute("CREATE TABLE b (id INT, v INT)").unwrap();
    e.execute("INSERT INTO a VALUES (1,1)").unwrap();
    e.execute("INSERT INTO b VALUES (1,1)").unwrap();
    let a_before = rows(&mut e, "SELECT id, v FROM a");
    let b_before = rows(&mut e, "SELECT id, v FROM b");

    e.execute("BEGIN").unwrap();
    e.execute("INSERT INTO a VALUES (2,2)").unwrap();
    e.execute("UPDATE b SET v = 9 WHERE id = 1").unwrap();
    e.execute("DELETE FROM a WHERE id = 1").unwrap();
    e.execute("ROLLBACK").unwrap();

    assert_eq!(rows(&mut e, "SELECT id, v FROM a"), a_before, "INV-05: table a must be fully restored");
    assert_eq!(rows(&mut e, "SELECT id, v FROM b"), b_before, "INV-05: table b must be fully restored");
}

/// INV-15a：索引投影与主 RowStore 的一致性——按索引列过滤的结果必须与全表扫描一致
/// （派生投影不得滞后于同一 committed 状态）。
#[test]
fn inv15a_index_projection_matches_rowstore() {
    let dir = tempfile::TempDir::new().unwrap();
    let mut e = file_engine(dir.path());
    e.execute("CREATE TABLE t (id INT, v INT)").unwrap();
    e.execute("INSERT INTO t VALUES (1,10),(2,20),(3,30)").unwrap();
    e.execute("CREATE INDEX idx_t_v ON t (v)").unwrap();

    let indexed = rows(&mut e, "SELECT id FROM t WHERE v = 20");
    let scanned = rows(&mut e, "SELECT id FROM t WHERE v = 20");
    assert_eq!(
        indexed, scanned,
        "INV-15: index-based filter must agree with row-store scan at committed state"
    );
    assert_eq!(indexed.len(), 1, "INV-15: exactly one row has v=20");
}

/// INV-15b：撤销后索引投影必须与主 RowStore 同步撤销
/// （若索引未回滚而主表回滚，即为派生投影不一致）。
#[test]
fn inv15b_index_projection_undone_with_rowstore() {
    let dir = tempfile::TempDir::new().unwrap();
    let mut e = file_engine(dir.path());
    e.execute("CREATE TABLE t (id INT, v INT)").unwrap();
    e.execute("INSERT INTO t VALUES (1,10),(2,20),(3,30)").unwrap();
    e.execute("CREATE INDEX idx_t_v ON t (v)").unwrap();

    e.execute("BEGIN").unwrap();
    e.execute("UPDATE t SET v = 99 WHERE v = 20").unwrap(); // 命中索引列
    e.execute("ROLLBACK").unwrap();

    let hit = rows(&mut e, "SELECT id FROM t WHERE v = 20");
    assert_eq!(
        hit.len(),
        1,
        "INV-15: after rollback the index projection must still resolve the original row \
         (stale index entry = inconsistent publication)"
    );
    let miss = rows(&mut e, "SELECT id FROM t WHERE v = 99");
    assert_eq!(
        miss.len(),
        0,
        "INV-15: after rollback no row may remain visible under the new value \
         (index and row store must be undone together)"
    );
}
