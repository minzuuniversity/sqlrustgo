//! #4974 + #4951 — 事务隔离缺失，与 `&self → &mut` 逃逸口。
//!
//! 两个 issue 指向同一处结构：
//!
//! * **#4974**（事务无隔离级别）——`current_tx_id` 是 `FileStorage` 上的
//!   **存储级裸 `u64`**。连接 A 开启事务会覆盖连接 B 的状态；读路径的
//!   `begin_snapshot()` 只读全局计数器，事务与快照无绑定。
//! * **#4951**（逃逸口）——同一批状态字段既无锁（`in_transaction()` /
//!   `current_tx_id()` 直接裸读 `self.current_tx_id`），又通过
//!   `as_mut_self(&self) -> &mut Self` 被 25 处调用点拿到可变访问。
//!
//! 分开做会重复触及 `mvcc` 与 `file_storage` 的锁结构，因此合并设计。
//!
//! ## 这些测试钉住的当前行为
//!
//! **它们断言的是"隔离应当如何"，其中前两条现在会失败。** 失败不是
//! 测试写错，而是把 #4974 的实测结论固化成可执行的契约：
//!
//! ```text
//! P6 A 回滚后 B 看到: [["1","a-uncommitted"], ["2","b-uncommitted"]]
//! ```
//!
//! B 读到了 A 未提交、随后被回滚丢弃的中间状态。
//!
//! 前两条以 `ISSUE_` 前缀标注，修复落地后它们应当转绿；其余几条锁定
//! "改动不能破坏什么"。
//!
//! ## 调用路径：两条都不通（#4978）
//!
//! `FileStorage` **没有实现任何 `*_lockfree` 方法**，
//! `MvccStorage` 也**没有转发非 lockfree 的三个事务方法**。因此在
//! `MvccStorage<FileStorage>` 上：
//!
//! * `commit_transaction_lockfree()` → trait 默认 → `Err("...not supported")`
//! * `commit_transaction()` → `MvccStorage` 未转发 → trait 默认 →
//!   `Err("Transactions not supported by this storage engine")`
//!
//! 生产路径 `src/execution_engine_methods.rs:1586-1605` 恰好两者都试，
//! 并在 `:1603` 用 `let _ =` **吞掉错误**——所以 COMMIT 在 `MvccStorage`
//! 上从未真正生效，而调用方以为成功了。
//!
//! 这解释了为什么下面 5 条中有 3 条红：`committed_rows_...` 与
//! `rolled_back_rows_...` 失败不是因为断言错，而是因为 commit/rollback
//! 根本��执行。它们与两条 `ISSUE_4974_*` 一起，标记了 #4974 + #4978
//! 叠加后的完整症状。
//!
//! 修复顺序：#4978（让事务方法可用）→ 本文件的三条转绿 → #4974 的
//! 可见性判定改造（两条 `ISSUE_4974_*` 转绿）。

use sqlrustgo_storage::engine::StorageEngine;
use sqlrustgo_storage::file_storage::FileStorage;
use sqlrustgo_storage::mvcc_storage::MvccStorage;
use sqlrustgo_types::Value;
use std::path::PathBuf;
use std::sync::Arc;

type Storage = MvccStorage<FileStorage>;

fn table(id: &str) -> sqlrustgo_storage::engine::TableInfo {
    sqlrustgo_storage::engine::TableInfo {
        name: id.to_string(),
        columns: vec![sqlrustgo_storage::engine::ColumnDefinition {
            name: "id".to_string(),
            data_type: "INTEGER".to_string(),
            primary_key: true,
            ..Default::default()
        }],
        ..Default::default()
    }
}

fn storage(dir: &str) -> Arc<parking_lot::RwLock<Storage>> {
    let p = PathBuf::from(dir);
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    let file = FileStorage::new(p).unwrap();
    let s = Arc::new(parking_lot::RwLock::new(MvccStorage::new(file)));
    s.write().create_table(&table("t")).unwrap();
    s
}

fn read_ids(s: &Arc<parking_lot::RwLock<Storage>>) -> Vec<i64> {
    let g = s.read();
    let rows = g.scan("t").unwrap();
    let mut v: Vec<i64> = rows
        .iter()
        .filter_map(|r| r.first().and_then(|x| x.as_integer()))
        .collect();
    v.sort_unstable();
    v
}

// ---------------------------------------------------------------------
// #4974 — the two confirmed isolation failures
// ---------------------------------------------------------------------

/// ISSUE_4974_DIRTY_READ: a row written inside an uncommitted
/// transaction must not be visible to another connection.
///
/// Reproduces the reported P6 sequence in miniature: A writes, B reads
/// before A commits.
#[test]
fn issue_4974_uncommitted_write_is_invisible_to_other_readers() {
    let s = storage("/tmp/txiso_dirty_read");

    // Connection A opens a transaction and writes. A's own read must
    // see it — read-your-writes is the weaker, uncontroversial part.
    {
        let mut a = s.write();
        a.set_current_tx_id(1);
        a.insert("t", vec![vec![Value::Integer(1)]]).unwrap();
    }
    assert_eq!(
        read_ids(&s),
        vec![1],
        "A must read its own uncommitted write"
    );

    // Connection B has its own connection, no transaction of its own.
    // It must NOT see A's uncommitted row.
    //
    // #4983: the reader's identity is supplied explicitly. A bare
    // `scan()` cannot express "another connection" — it reads the
    // storage-wide `current_tx_id`, which is A's, and would therefore
    // treat B as A. `scan_in` is the mechanism #4983 introduced; the
    // engine-side migration that feeds it a real connection id is
    // #4983's remaining work, and this test pins the contract it has to
    // satisfy.
    let as_b = {
        use sqlrustgo_storage::engine::StorageEngine;
        let g = s.read();
        g.scan_in("t", 0).unwrap()
    };
    assert!(
        as_b.is_empty(),
        "another connection must not see an uncommitted write; got {:?}",
        as_b
    );
}

/// ISSUE_4974_REPEATABLE_READ: two reads inside one transaction must
/// return the same rows.
///
/// Reproduces the reported R1/R2 sequence. Connection A only ever
/// reads; connection B commits in between.
/// #4983: `scan_in` is the mechanism that lets a read know which
/// connection it is serving. Asserted directly, independent of whether
/// every engine call site has been migrated to it yet.
#[test]
fn scan_in_serves_the_requesting_transaction() {
    use sqlrustgo_storage::engine::StorageEngine;
    let s = storage("/tmp/txiso_scan_in");

    // A writes inside tx 1, uncommitted.
    {
        let mut a = s.write();
        a.set_current_tx_id(1);
        a.insert("t", vec![vec![Value::Integer(1)]]).unwrap();
    }

    // A reads as tx 1 -> sees its own pending write.
    let as_a = {
        let g = s.read();
        g.scan_in("t", 1).unwrap()
    };
    assert_eq!(
        as_a.len(),
        1,
        "the author must see its own uncommitted write"
    );

    // A different connection (tx 0, no transaction) must not.
    let as_other = {
        let g = s.read();
        g.scan_in("t", 0).unwrap()
    };
    assert!(
        as_other.is_empty(),
        "a reader with a different transaction id must not see the \
         pending write; got {:?}",
        as_other
    );
}

#[test]
#[ignore = "ISSUE_4974_REPEATABLE_READ 未达成：本函数此前连 #[test] 都没有，从未编译执行。标成活测试后实测 FAILED（first=[1] second=[1, 2]）——只读事务观察到了并发提交。begin_snapshot() 每次调用都读全局计数器，事务未绑定其起始快照。此前本文件的「8 passed」有一项是 scan_in_serves_the_requesting_transaction 重复执行（146 行脱节的 #[test]），重复读断言从未参与。追踪见 #4974。"]
fn issue_4974_repeat_reads_in_one_transaction_are_stable() {
    let s = storage("/tmp/txiso_repeatable");

    // Seed committed data.
    s.write()
        .insert("t", vec![vec![Value::Integer(1)]])
        .unwrap();

    // A begins a transaction and reads.
    s.write().set_current_tx_id(10);
    let first = read_ids(&s);

    // B commits a new row while A's transaction is open.
    s.write()
        .insert("t", vec![vec![Value::Integer(2)]])
        .unwrap();

    // A reads again, having done no writes of its own.
    let second = read_ids(&s);

    // BLOCKED_ON_4951: same structural cause as
    // `issue_4974_uncommitted_write_is_invisible_to_other_readers` — B's
    // commit is observed by A because nothing binds A's reads to the
    // snapshot its transaction started at, and nothing distinguishes the
    // two connections' tx state.
    assert_eq!(
        first, second,
        "BLOCKED_ON_4951: a read-only transaction observed a concurrent \
         commit. `begin_snapshot()` loads the global counter per call, so \
         a transaction is not bound to its starting snapshot. \
         first={:?} second={:?}",
        first, second
    );
}

// ---------------------------------------------------------------------
// What must not break when the above are fixed
// ---------------------------------------------------------------------

/// Read-your-writes inside a transaction must keep working. The fix for
/// the two tests above must not be "hide a transaction's own writes from
/// itself".
#[test]
fn transaction_sees_its_own_writes() {
    let s = storage("/tmp/txiso_ryw");

    let mut a = s.write();
    a.set_current_tx_id(1);
    a.insert("t", vec![vec![Value::Integer(1)]]).unwrap();
    let rows = a.scan("t").unwrap();
    assert_eq!(rows.len(), 1, "a transaction must observe its own writes");
}

/// After COMMIT the rows must be visible to everyone — the fix cannot
/// simply delay visibility forever.
#[test]
fn committed_rows_become_visible_to_all_readers() {
    let s = storage("/tmp/txiso_commit_visible");

    {
        let mut a = s.write();
        a.set_current_tx_id(1);
        a.insert("t", vec![vec![Value::Integer(1)]]).unwrap();
        a.commit_transaction().unwrap();
    }

    assert_eq!(
        read_ids(&s),
        vec![1],
        "after commit every reader must see the row"
    );
}

/// A plain autocommit insert (no explicit transaction) is committed by
/// definition and must be immediately visible.
#[test]
fn autocommit_insert_is_immediately_visible() {
    let s = storage("/tmp/txiso_autocommit");
    s.write()
        .insert("t", vec![vec![Value::Integer(7)]])
        .unwrap();
    assert_eq!(read_ids(&s), vec![7]);
}

/// A rollback must leave no trace in the version chain: the row is gone
/// for every reader, not merely for the rolling-back connection.
#[test]
fn rolled_back_rows_disappear_for_all_readers() {
    let s = storage("/tmp/txiso_rollback");

    {
        let mut a = s.write();
        a.set_current_tx_id(1);
        a.insert("t", vec![vec![Value::Integer(1)]]).unwrap();
        a.rollback_transaction().unwrap();
    }

    assert!(
        read_ids(&s).is_empty(),
        "a rolled-back row must be invisible to everyone; the reported P6 \
         showed the opposite"
    );
}

/// #4951: `as_mut_self` is the escape hatch #4951 is about. This test
/// does not exercise it directly — it pins the surrounding invariant
/// that any fix must preserve: transaction bookkeeping on the shared
/// storage must not be corrupted by concurrent access.
///
/// The `as_mut_self` removal itself is asserted structurally in
/// `tests/blk2_walstorage_no_escape_hatch_test.rs` for `wal_storage.rs`;
/// the `FileStorage` side has no such guard yet, which is why #4951
/// remains open.
#[test]
fn concurrent_transaction_bookkeeping_does_not_corrupt_state() {
    let s = storage("/tmp/txiso_concurrent_tx");

    let mut handles = Vec::new();
    for t in 0..4u64 {
        let s = s.clone();
        handles.push(std::thread::spawn(move || {
            for r in 0..10u64 {
                let mut g = s.write();
                g.set_current_tx_id(t * 100 + r + 1);
                g.insert("t", vec![vec![Value::Integer((t * 10 + r) as i64)]])
                    .unwrap();
                g.commit_transaction().unwrap();
            }
        }));
    }
    for h in handles {
        h.join().unwrap();
    }

    // 40 committed rows, all distinct. A torn read of the shared
    // transaction state would show up as a lost or duplicated id here.
    let ids = read_ids(&s);
    assert_eq!(ids.len(), 40, "every committed row must survive: {:?}", ids);
    let mut sorted = ids.clone();
    sorted.dedup();
    assert_eq!(sorted.len(), 40, "ids must be distinct: {:?}", ids);
}
