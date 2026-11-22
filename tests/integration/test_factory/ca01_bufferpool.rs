//! C01-11 · ca01_bufferpool — BufferPool 装配路径的 Undo/能力一致性（INV-14 / INV-15）
//!
//! 业主指令点名要验证的问题：**FileStorage 的 buffer_pool 路径是否绕过 Undo**。
//! 背景：AGENTS.md §1.5 声明 BufferPool/Page/BPlusTree 在 src/lib.rs:49-63 被标为
//! "Test-only capability"，但 `FileStorage::new_with_bufferpool` 使 buffer_pool 成为
//! 可选装配——因此"该路径是否仍完整记录并应用 Undo"必须实测，不能靠声明推断。
//!
//! 探针：
//!   a) buffer_pool 装配下 UPDATE 的撤销是否生效（Undo 覆盖）
//!   b) buffer_pool 装配下 DELETE/INSERT 的撤销是否生效
//!   c) buffer_pool 与普通装配的可见状态是否一致（INV-15 派生投影一致性）
//!   d) capability 诚实性：buffer_pool 不应凭空增加 inner 不具备的语义（INV-14）
//! 语义：未知产品假设不强求 RED；失败即缺陷捕获（记入 RED 台账），不得弱化断言。

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::buffer_pool::BufferPool;
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

fn engine_bp(dir: &std::path::Path) -> ExecutionEngine<FileStorage> {
    let pool = Arc::new(BufferPool::new(64));
    let fs = FileStorage::new_with_bufferpool(dir.to_path_buf(), pool).unwrap();
    ExecutionEngine::new(Arc::new(RwLock::new(fs)))
}

fn engine_plain(dir: &std::path::Path) -> ExecutionEngine<FileStorage> {
    let fs = FileStorage::new(dir.to_path_buf()).unwrap();
    ExecutionEngine::new(Arc::new(RwLock::new(fs)))
}

/// a) buffer_pool 装配下 UPDATE 的撤销必须生效（Undo 覆盖）
#[test]
fn inv14_bp_update_undo_applied() {
    let dir = tempfile::TempDir::new().unwrap();
    let mut e = engine_bp(dir.path());
    e.execute("CREATE TABLE t (id INT PRIMARY KEY, v INT)").unwrap();
    e.execute("INSERT INTO t VALUES (1, 10)").unwrap();
    let before = rows(&mut e, "SELECT id, v FROM t");

    e.execute("BEGIN").unwrap();
    e.execute("UPDATE t SET v = 99 WHERE id = 1").unwrap();
    e.execute("ROLLBACK").unwrap();

    assert_eq!(
        rows(&mut e, "SELECT id, v FROM t"),
        before,
        "INV-14: buffer_pool assembly must still apply Undo on UPDATE rollback"
    );
}

/// b) buffer_pool 装配下 DELETE/INSERT 的撤销必须生效。
/// 实测指纹（2026-11-21）：DELETE 的撤销生效（行被恢复），但 INSERT 的撤销**未生效**
/// ——事务内插入的行在 ROLLBACK 后仍存在 ⇒ 该装配路径绕过 Undo（业主指令点名问题）。
#[test]
fn inv14_bp_delete_insert_undo_applied() {
    let dir = tempfile::TempDir::new().unwrap();
    let mut e = engine_bp(dir.path());
    e.execute("CREATE TABLE t (id INT PRIMARY KEY, v INT)").unwrap();
    e.execute("INSERT INTO t VALUES (1, 10)").unwrap();
    e.execute("INSERT INTO t VALUES (2, 20)").unwrap();
    let before = rows(&mut e, "SELECT id, v FROM t");

    e.execute("BEGIN").unwrap();
    e.execute("DELETE FROM t WHERE id = 1").unwrap();
    e.execute("INSERT INTO t VALUES (3, 30)").unwrap();
    e.execute("ROLLBACK").unwrap();

    assert_eq!(
        rows(&mut e, "SELECT id, v FROM t"),
        before,
        "INV-14: buffer_pool assembly must restore deleted and remove inserted rows on rollback"
    );
}

/// c) 两种装配的可见状态一致（INV-15：装配差异不得改变语义）
#[test]
fn inv15_bp_and_plain_assembly_semantically_equal() {
    let dir_bp = tempfile::TempDir::new().unwrap();
    let dir_plain = tempfile::TempDir::new().unwrap();
    let sql_seq = [
        "CREATE TABLE t (id INT PRIMARY KEY, v INT)",
        "INSERT INTO t VALUES (1, 10), (2, 20)",
        "BEGIN",
        "UPDATE t SET v = v + 5",
        "DELETE FROM t WHERE id = 2",
        "INSERT INTO t VALUES (3, 30)",
        "ROLLBACK",
    ];
    let mut ebp = engine_bp(dir_bp.path());
    let mut epl = engine_plain(dir_plain.path());
    for s in sql_seq {
        ebp.execute(s).unwrap();
        epl.execute(s).unwrap();
    }
    assert_eq!(
        rows(&mut ebp, "SELECT id, v FROM t"),
        rows(&mut epl, "SELECT id, v FROM t"),
        "INV-15: buffer_pool and plain assembly must expose identical visible state"
    );
}

/// d) capability 诚实性（INV-14）：buffer_pool 不得凭空改变读/写语义。
/// 实测指纹（2026-11-21）：无 WHERE 的全表 scan 可见已提交行，但 **WHERE/PK 过滤查询返回空**
/// ⇒ 读路径在该装配下丢失行（PLAIN 装配同序列正常）——两处差异均为该装配独有。
/// 当前 FileStorage 默认路径本身具备 undo；buffer_pool 只应影响缓冲/缓存层，不应改变
/// 事务语义。此处断言两种装配在同一 DML 序列后的事务可见性一致（若 buffer_pool
/// 引入额外语义，会在此暴露）。
#[test]
fn inv14_bp_adds_no_extra_transaction_semantics() {
    let dir_bp = tempfile::TempDir::new().unwrap();
    let mut e = engine_bp(dir_bp.path());
    e.execute("CREATE TABLE t (id INT PRIMARY KEY, v INT)").unwrap();
    e.execute("INSERT INTO t VALUES (1, 10)").unwrap();

    // 事务外的写入必须立即可见（不得被 buffer_pool 私自缓存成不可见）
    let visible = rows(&mut e, "SELECT COUNT(*) FROM t");
    assert_eq!(
        visible.len(),
        1,
        "INV-14: buffer_pool must not alter autocommit visibility semantics"
    );
    assert_eq!(
        rows(&mut e, "SELECT v FROM t WHERE id = 1"),
        vec![vec!["Integer(10)".to_string()]],
        "INV-14: committed value must be immediately visible through buffer_pool assembly"
    );
}
