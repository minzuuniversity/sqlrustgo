//! #4995 — `ON DUPLICATE KEY UPDATE` 的赋值必须真的落到行上。
//!
//! # 为什么这个文件存在
//!
//! 缺陷被藏了很久，因为**所有已有的 ODKU 用例都在错误路径上**：它们
//! 断言的是 `VALUES(col)` 报 1064（解析器缺陷），也就是「ODKU 失败了」。
//! 于是一个「ODKU 成功返回但一个字节都没改」的缺陷，没有任何用例能
//! 看见 —— 每个用例都停在更早的解析阶段，根本走不到赋值。
//!
//! 缺陷本体在 `MvccStorage::update`：它更新完 inner 引擎之后，回到
//! **MVCC 版本链**里重扫，把扫到的行当作「新版本」`put` 回去。而
//! `inner.update(...)` 只改 inner，链里存的仍是更新前的行 —— 于是追加
//! 上去的所谓新版本内容是**旧行**。读取时按 PK 从链尾取最新版本，新的
//! 旧行版本把真实数据挡住了，`UPDATE` 永久不可见。
//!
//! 端到端表现（真实 MySQL 协议）：
//!
//! ```text
//! INSERT INTO t VALUES (1,200,'b') ON DUPLICATE KEY UPDATE k=222, v='zz';
//! -- 无错误
//! SELECT id,k,v FROM t;   -->  1  100  a      （修复前；应为 1 222 zz）
//! ```
//!
//! 重复键被正确识别、语句成功返回，**唯独赋值那段没接上**。

use sqlrustgo_storage::engine::ColumnDefinition;
use sqlrustgo_storage::{MvccStorage, StorageEngine, TableInfo, Value};

fn col(name: &str, pk: bool) -> ColumnDefinition {
    ColumnDefinition {
        name: name.to_string(),
        data_type: "INT".to_string(),
        nullable: true,
        primary_key: pk,
        ..Default::default()
    }
}

fn info() -> TableInfo {
    TableInfo {
        name: "t".to_string(),
        columns: vec![col("id", true), col("k", false)],
        ..Default::default()
    }
}

fn i(v: i64) -> Value {
    Value::Integer(v)
}

/// 读回 `id` 那一行的 `k`。
fn read_k(s: &MvccStorage<sqlrustgo_storage::FileStorage>) -> Option<i64> {
    s.scan("t")
        .ok()?
        .into_iter()
        .next()
        .and_then(|r| r.get(1).and_then(|v| v.as_integer()))
}

fn storage(dir: &std::path::Path) -> MvccStorage<sqlrustgo_storage::FileStorage> {
    let f = sqlrustgo_storage::FileStorage::new(dir.to_path_buf()).expect("FileStorage");
    let mut s = MvccStorage::new(f);
    s.create_table(&info()).expect("create_table");
    s
}

/// 走一遍「开事务 -> 写 -> 提交」，镜像 autocommit 的真实路径。
///
/// 真实服务端每次 DML 都由 `begin_implicit_dml_tx` 先 `set_current_tx_id`，
/// 写完再 `commit_implicit_dml_tx` 提交。直接裸调 `update` 而不开事务，
/// `FileStorage::update` 的事务门会直接放行，写根本没落到 inner —— 那样
/// 测的就不是这个缺陷了。
fn in_tx(
    s: &mut MvccStorage<sqlrustgo_storage::FileStorage>,
    tx: u64,
    f: impl FnOnce(&mut MvccStorage<sqlrustgo_storage::FileStorage>),
) {
    s.set_current_tx_id(tx);
    f(s);
    s.commit_transaction().expect("commit");
    // `FileStorage::update` only walks `data.rows`, while `insert`
    // parks rows in `insert_buffer` and `commit_transaction` does not
    // auto-flush them (its own comment says so). The server flushes between
    // statements; the component test has to do the same or the UPDATE would
    // be matching against an empty `data.rows` and silently reporting 0.
    s.flush().expect("flush");
}

fn tmp(tag: &str) -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!("odku4995_{tag}"));
    let _ = std::fs::remove_dir_all(&p);
    p
}

/// #4995 核心用例：走 `update` 之后，**读回来的必须是新值**。
///
/// 这一条就是原来缺失的断言 —— 没有任何既有用例检查过它。
#[test]
fn update_must_be_visible_to_a_following_read() {
    let dir = tmp("visible");
    let mut s = storage(&dir);

    in_tx(&mut s, 1, |s| { s.insert("t", vec![vec![i(1), i(100)]]).expect("insert"); });
    assert_eq!(read_k(&s), Some(100), "insert should be visible");

    // This is exactly what `apply_odku` does for
    // `ON DUPLICATE KEY UPDATE k=222`.
    in_tx(&mut s, 2, |s| { let n = s.update("t", &[i(1)], &[(1, i(222))]).expect("update"); let _ = n;});

    assert_eq!(
        read_k(&s),
        Some(222),
        "#4995: UPDATE returned success but the row still reads 100 — the new \
         MVCC version carried the OLD contents. (a read that re-scans the \
         version chain instead of the inner engine re-puts the pre-update row)"
    );
}

/// 多个 PK：只有 `filters` 指名的那一行该变。
#[test]
fn update_only_touches_the_filtered_pk() {
    let dir = tmp("multi");
    let mut s = storage(&dir);
    in_tx(&mut s, 1, |s| {
        s.insert(
            "t",
            vec![vec![i(1), i(100)], vec![i(2), i(200)], vec![i(3), i(300)]],
        )
        .expect("insert");
    });

    in_tx(&mut s, 2, |s| { s.update("t", &[i(2)], &[(1, i(999))]).expect("update"); });

    let rows = s.scan("t").expect("scan");
    let got: Vec<(i64, i64)> = rows
        .iter()
        .map(|r| {
            (
                r[0].as_integer().unwrap(),
                r[1].as_integer().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        got,
        vec![(1, 100), (2, 999), (3, 300)],
        "#4995: only the filtered PK may change"
    );
}

/// 连续两次更新：链尾最新版本必须是最后一次的值。
///
/// 这条盯的是「从链尾取最新版本」这条读取规则 —— 旧的错误实现追加的
/// 新版本永远排在链尾，单次更新看不出来，两次就会露馅。
#[test]
fn second_update_wins_over_the_first() {
    let dir = tmp("twice");
    let mut s = storage(&dir);
    in_tx(&mut s, 1, |s| { s.insert("t", vec![vec![i(1), i(0)]]).expect("insert"); });

    for (n, want) in [11i64, 22, 33].into_iter().enumerate() {
        let tx = 10 + n as u64;
        in_tx(&mut s, tx, |s| { s.update("t", &[i(1)], &[(1, i(want))]).expect("update"); });
        assert_eq!(read_k(&s), Some(want), "after update to {want}");
    }
}

/// 未提交时读不到，提交后才可见 —— 更新路径的隔离语义。
#[test]
fn update_inside_a_transaction_is_not_visible_until_commit() {
    let dir = tmp("iso");
    let mut s = storage(&dir);
    in_tx(&mut s, 1, |s| { s.insert("t", vec![vec![i(1), i(100)]]).expect("insert"); });

    s.set_current_tx_id(7);
    s.update("t", &[i(1)], &[(1, i(555))]).expect("update");
    // Same connection still sees its own write ...
    assert_eq!(read_k(&s), Some(555), "own uncommitted write is visible");
    // ... and a different reader (tx 0) must not.
    let other = s.scan_in("t", 0).expect("scan_in as other tx");
    let other_k = other
        .first()
        .and_then(|r| r.get(1))
        .and_then(|v| v.as_integer());
    assert_eq!(
        other_k,
        Some(100),
        "#4974/#4995: an uncommitted UPDATE must not be visible to another \
         transaction"
    );

    s.commit_transaction().expect("commit");
    assert_eq!(read_k(&s), Some(555));
}
