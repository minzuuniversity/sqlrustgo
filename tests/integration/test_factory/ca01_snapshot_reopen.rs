//! C01-06 · ca01_snapshot_reopen — 未提交私有状态不可见（INV-07）
//!
//! INV-07 未提交私有状态不能进入 committed snapshot / recovery 可见基线
//! 探针：
//!   a) 同存储实例双引擎（会话语义）：B 看不到 A 未提交的行；COMMIT 后可见
//!   b) 持久化基线：flush 后重开，未提交数据不得进入 recovery 基线
//!   c) ROLLBACK + flush + 重开：快照只含 committed
//! 语义：未知产品假设不强求 RED；失败即缺陷捕获；不得弱化断言。
//! 注：持久化 API 契约为显式 flush_all_buffers()+flush()（见 e2e_crash_recovery_proof 参照）。

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

fn count_of(e: &mut ExecutionEngine<impl StorageEngine + 'static>) -> usize {
    rows(e, "SELECT id, v FROM t").len()
}

/// INV-07a：同存储实例双引擎——B 看不到 A 未提交的行（committed snapshot 边界）
#[test]
fn inv07a_uncommitted_invisible_to_other_engine() {
    let dir = tempfile::TempDir::new().unwrap();
    let fs = FileStorage::new(dir.path().to_path_buf()).unwrap();
    let storage = Arc::new(RwLock::new(fs));
    let mut a = ExecutionEngine::new(storage.clone());
    a.execute("CREATE TABLE t (id INT, v INT)").unwrap();
    a.execute("INSERT INTO t VALUES (1,10)").unwrap();

    let mut b = ExecutionEngine::new(storage.clone());

    a.execute("BEGIN").unwrap();
    a.execute("INSERT INTO t VALUES (2,20)").unwrap(); // 未提交
    assert_eq!(
        count_of(&mut b),
        1,
        "INV-07: uncommitted row invisible to engine B"
    );

    a.execute("COMMIT").unwrap();
    assert_eq!(
        count_of(&mut b),
        2,
        "INV-07: committed row visible after COMMIT"
    );
}

/// INV-07b：flush 后重开——未提交数据不得进入 recovery 可见基线
#[test]
fn inv07b_reopen_baseline_excludes_uncommitted() {
    let dir = tempfile::TempDir::new().unwrap();
    {
        let (mut a, storage) = open(dir.path());
        a.execute("CREATE TABLE t (id INT, v INT)").unwrap();
        a.execute("INSERT INTO t VALUES (1,10)").unwrap(); // committed（autocommit）
        a.execute("BEGIN").unwrap();
        a.execute("INSERT INTO t VALUES (2,20)").unwrap(); // 未提交
                                                           // 显式持久化（API 契约）：flush 时未提交状态不得随盘
        storage.write().flush_all_buffers().unwrap();
        storage.write().flush().unwrap();
    }
    let (mut e2, _) = open(dir.path());
    assert_eq!(
        rows(&mut e2, "SELECT id, v FROM t"),
        vec![vec!["Integer(1)".to_string(), "Integer(10)".to_string()]],
        "INV-07: recovery baseline must exclude uncommitted (2,20)"
    );
}

/// INV-07c：ROLLBACK + flush + 重开——快照只含 committed
#[test]
fn inv07c_rollback_then_reopen_is_clean() {
    let dir = tempfile::TempDir::new().unwrap();
    {
        let (mut a, storage) = open(dir.path());
        a.execute("CREATE TABLE t (id INT, v INT)").unwrap();
        a.execute("INSERT INTO t VALUES (1,10)").unwrap();
        a.execute("BEGIN").unwrap();
        a.execute("INSERT INTO t VALUES (2,20)").unwrap();
        a.execute("ROLLBACK").unwrap();
        storage.write().flush_all_buffers().unwrap();
        storage.write().flush().unwrap();
    }
    let (mut e2, _) = open(dir.path());
    assert_eq!(
        rows(&mut e2, "SELECT id, v FROM t"),
        vec![vec!["Integer(1)".to_string(), "Integer(10)".to_string()]],
        "INV-07: rollback + reopen must show exactly the committed rows"
    );
}

fn open(dir: &std::path::Path) -> (ExecutionEngine<FileStorage>, Arc<RwLock<FileStorage>>) {
    let fs = FileStorage::new(dir.to_path_buf()).unwrap();
    let storage = Arc::new(RwLock::new(fs));
    (ExecutionEngine::new(storage.clone()), storage)
}
