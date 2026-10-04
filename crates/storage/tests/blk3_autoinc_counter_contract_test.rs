//! #4964 复核补测：per-table 计数器真正修复的场景。
//!
//! `blk3_auto_increment_per_table_test.rs`（PR #4964 自带）的 3 个测试
//! **无法**证明该 PR 修复了任何缺陷：把 `MemoryStorage` 的 per-table
//! `AtomicU64` 计数器换回原来的 `MAX(现有行)+1` 扫描分配，那 3 个测试
//! 仍然全部通过。原因是 `insert(&mut self)` 在单个 `MemoryStorage`
//! 实例内被完全串行化，且行在 `insert` 末尾同步写入 `self.tables`
//! （`crates/storage/src/engine.rs`，`self.tables.entry(..).extend(padded)`），
//! 所以 `MAX`-扫描在那种拓扑下本就正确——并发"都看到空表"的情形不成立。
//!
//! 本文件锁定 per-table 计数器**确实**改变行为的场景。
//!
//! ## 变异验证
//!
//! 把计数器换回 `MAX`-扫描（保留 #4962 的 `scan_with_filter` 签名变更，
//! 否则整 crate 编译不过）：
//!
//! ```text
//! PROBE ids after delete-then-auto = [1]     ← 回退后：复用了已删除的 id
//! test probe_delete_then_auto ... FAILED
//! ```
//!
//! 保留 per-table 计数器：
//!
//! ```text
//! PROBE ids after delete-thon-auto = [101]   ← 计数器记住了已分配的上界
//! test probe_delete_then_auto ... ok
//! ```

use sqlrustgo_storage::engine::{ColumnDefinition, MemoryStorage, StorageEngine, TableInfo, Value};

fn table_with_auto_inc(name: &str) -> TableInfo {
    TableInfo {
        name: name.to_string(),
        columns: vec![ColumnDefinition {
            name: "id".to_string(),
            data_type: "INTEGER".to_string(),
            auto_increment: true,
            primary_key: true,
            ..Default::default()
        }],
        ..Default::default()
    }
}

fn ids_of(s: &MemoryStorage, table: &str) -> Vec<i64> {
    s.scan(table)
        .expect("scan")
        .iter()
        .map(|r| match r[0] {
            Value::Integer(i) => i,
            _ => -1,
        })
        .collect()
}

/// MySQL/InnoDB 语义：AUTO_INCREMENT 计数器**不因删除而回退**。
///
/// 显式插入 id=100 → 删除该行 → 再自动插入。
///
/// - `MAX(现有行)+1` 分配：删掉唯一一行后表空了，max 无从取得，
///   退回 `unwrap_or(1)`，**分配出 id=1**——复用了早已用过的 id。
/// - per-table 计数器：显式 id=100 已把计数器抬到 100，
///   下一次 `fetch_add` 得 101。
///
/// 这是 #4964 相对被它替换的实现**唯一**可观测的行为差异。
#[test]
fn deleted_ids_are_never_reused() {
    let mut s = MemoryStorage::new();
    s.create_table(&table_with_auto_inc("t")).expect("create");

    s.insert("t", vec![vec![Value::Integer(100)]])
        .expect("insert 100");
    let deleted = s.delete("t", &[Value::Integer(100)]).expect("delete");
    assert_eq!(deleted, 1, "the explicit row must actually be deleted");

    s.insert("t", vec![vec![Value::Null]]).expect("auto insert");
    let ids = ids_of(&s, "t");

    assert_eq!(
        ids,
        vec![101],
        "AUTO_INCREMENT must not reuse an id that was already issued, \
         even after the row is deleted (MySQL/InnoDB semantics)"
    );
}

/// 同一语义的批量形态：删掉尾部若干行后，计数器仍停在删除前的上界。
#[test]
fn deleting_a_tail_run_does_not_rewind_the_counter() {
    let mut s = MemoryStorage::new();
    s.create_table(&table_with_auto_inc("t")).expect("create");

    // 显式铺 1..=5
    for id in 1..=5 {
        s.insert("t", vec![vec![Value::Integer(id)]])
            .expect("insert");
    }
    // 逐个删掉尾部 4、5（`delete` 的 filters 是 AND 语义，多值需分别调用）
    assert_eq!(s.delete("t", &[Value::Integer(4)]).expect("del 4"), 1);
    assert_eq!(s.delete("t", &[Value::Integer(5)]).expect("del 5"), 1);

    s.insert("t", vec![vec![Value::Null]]).expect("auto insert");
    let ids = ids_of(&s, "t");

    assert_eq!(
        ids,
        vec![1, 2, 3, 6],
        "counter must continue from 6, not rewind to 4 (which 4 was) \
         nor skip to 7 (the removed rows are not holes)"
    );
}

/// 计数器是 per-table 的：一张表的分配不得抬高另一张表。
///
/// `MAX`-扫描实现天然满足此条（每表独立扫描），per-table 计数器也满足；
/// 但这条是防止后续把计数器误做成全局单例的护栏。
#[test]
fn counters_are_independent_per_table() {
    let mut s = MemoryStorage::new();
    s.create_table(&table_with_auto_inc("a")).expect("create a");
    s.create_table(&table_with_auto_inc("b")).expect("create b");

    s.insert("a", vec![vec![Value::Integer(50)]]).expect("a:50");
    s.insert("b", vec![vec![Value::Null]]).expect("b:auto");
    s.insert("a", vec![vec![Value::Null]]).expect("a:auto");

    assert_eq!(ids_of(&s, "a"), vec![50, 51], "table a continues from 50");
    assert_eq!(
        ids_of(&s, "b"),
        vec![1],
        "table b must start at 1 — table a's 50 must not lift b's counter"
    );
}

/// DROP 语义：删除整表后重新建表，计数器必须重新从 1 开始。
///
/// MySQL 语义下 `DROP TABLE` 会丢弃该表的 AUTO_INCREMENT 计数。
/// per-table 计数器在 `drop_table` 时必须清掉条目，否则重建后会从旧上界继续。
#[test]
fn drop_table_resets_the_counter() {
    let mut s = MemoryStorage::new();
    s.create_table(&table_with_auto_inc("t")).expect("create");
    s.insert("t", vec![vec![Value::Integer(900)]])
        .expect("insert 900");

    s.drop_table("t").expect("drop");
    s.create_table(&table_with_auto_inc("t")).expect("recreate");
    s.insert("t", vec![vec![Value::Null]]).expect("auto insert");

    assert_eq!(
        ids_of(&s, "t"),
        vec![1],
        "DROP TABLE must discard the table's AUTO_INCREMENT high-water mark"
    );
}
