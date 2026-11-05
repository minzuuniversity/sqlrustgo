//! #5394: `LEFT OUTER JOIN` 的 **ON 条件过滤**会丢弃左表行。
//!
//! 标准 SQL（SQLite / MySQL / PostgreSQL）的语义：左表每一行都必须出现在结果里；
//! 当 ON 条件匹配不上任何右表行时，该左表行以 NULL 补齐。
//!
//! 缺陷：join 执行器在**哈希键匹配**时就��� `left_matched` 记下了「这个左表行
//! 匹配上了」，而**完整的 ON 谓词是在那之后**才过滤的。于是「键相同、但 ON 条件
//! 不满足」的左表行：
//!
//! * 自己的结果行被 ON 过滤掉了；
//! * 又因为已经在 `left_matched` 里，不会被补 NULL；
//!
//! ⇒ **整行消失**。
//!
//! TPC-H Q13 正是这个形态（`ON c_custkey = o_custkey AND o_comment NOT LIKE
//! '%special%requests%'`）：引擎少了 71 个客户，而**结果行数不变、分组数不变**，
//! 只比对行数根本发现不了。

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, Value};
use std::sync::Arc;

fn fresh() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

/// 取某一列的整型取值，按升序排好。
fn ints(e: &mut ExecutionEngine<MemoryStorage>, sql: &str, col: usize) -> Vec<i64> {
    let r = e.execute(sql).unwrap();
    let mut out: Vec<i64> = r
        .rows
        .iter()
        .map(|row| match &row[col] {
            Value::Integer(n) => *n,
            other => panic!("expected Integer in column {}, got {:?}", col, other),
        })
        .collect();
    out.sort_unstable();
    out
}

/// 核心用例：左表行在 ON 条件把右表行全滤掉时，仍必须出现（补 NULL）。
#[test]
fn left_join_keeps_rows_whose_matches_are_all_filtered_by_the_on_clause() {
    let mut e = fresh();
    e.execute("CREATE TABLE c (id INT, name TEXT)").unwrap();
    e.execute("CREATE TABLE o (cid INT, comment TEXT)").unwrap();
    e.execute("INSERT INTO c VALUES (1, 'a'), (2, 'b'), (3, 'c')").unwrap();
    // 客户 2 的订单**全部**被 ON 条件过滤掉。
    // 注意 LIKE 模式里的 `%` 是通配符，所以两条注释都**必须**真的含
    // "special%requests%" —— 写成 'x%requests%' 并不匹配
    // '%special%requests%'（后者要求中间有 "special"），那一条会存活下来，
    // 于是这个用例根本碰不到缺陷。
    e.execute("INSERT INTO o VALUES (1, 'ok'), (2, 'special%requests%'), (2, 'a special%requests% b')")
        .unwrap();

    let ids = ints(
        &mut e,
        "SELECT c.id FROM c LEFT OUTER JOIN o \
         ON c.id = o.cid AND o.comment NOT LIKE '%special%requests%'",
        0,
    );
    // 三个客户**都必须**出现。修复前只有 2 个（客户 2 整行消失）。
    assert_eq!(
        ids,
        vec![1, 2, 3],
        "every left row must appear — a left row whose matches are all \
         filtered out by the ON clause must be padded with NULL, not dropped"
    );
}

/// COUNT 聚合下同样不能丢：被 ON 滤空的左表行必须以 **0** 计数出现。
/// 这正是 TPC-H Q13 的形态。
#[test]
fn left_join_count_aggregates_the_filtered_customer_as_zero() {
    let mut e = fresh();
    e.execute("CREATE TABLE c (id INT)").unwrap();
    e.execute("CREATE TABLE o (cid INT, comment TEXT)").unwrap();
    e.execute("INSERT INTO c VALUES (1), (2), (3)").unwrap();
    e.execute("INSERT INTO o VALUES (1, 'ok'), (2, 'special%requests%')").unwrap();

    let r = e
        .execute(
            "SELECT c.id, COUNT(o.cid) FROM c LEFT OUTER JOIN o \
             ON c.id = o.cid AND o.comment NOT LIKE '%special%requests%' \
             GROUP BY c.id ORDER BY c.id",
        )
        .unwrap();
    assert_eq!(
        r.rows.len(),
        3,
        "the filtered customer must still be counted (as 0), not disappear"
    );
    // 客户 1 有一个存活订单，客户 2 的订单全被滤掉 ⇒ 0，客户 3 没有订单 ⇒ 0
    let counts: Vec<i64> = r
        .rows
        .iter()
        .map(|row| match &row[1] {
            Value::Integer(n) => *n,
            other => panic!("expected Integer count, got {:?}", other),
        })
        .collect();
    assert_eq!(counts, vec![1, 0, 0]);
}

/// 反向回归：ON 条件**全部通过**时，行为不得改变（不能多补 NULL 行）。
#[test]
fn left_join_with_a_passing_on_clause_is_unchanged() {
    let mut e = fresh();
    e.execute("CREATE TABLE c (id INT)").unwrap();
    e.execute("CREATE TABLE o (cid INT, comment TEXT)").unwrap();
    e.execute("INSERT INTO c VALUES (1), (2)").unwrap();
    e.execute("INSERT INTO o VALUES (1, 'ok'), (2, 'ok')").unwrap();

    let ids = ints(
        &mut e,
        "SELECT c.id FROM c LEFT OUTER JOIN o \
         ON c.id = o.cid AND o.comment NOT LIKE '%special%requests%'",
        0,
    );
    assert_eq!(ids, vec![1, 2]);
}

/// 反向回归：ON 条件为真（无过滤）时，行数不得翻倍 ——
/// 补 NULL 的逻辑只在「确实没有存活配对」时才发生。
#[test]
fn left_join_without_any_filter_does_not_duplicate_rows() {
    let mut e = fresh();
    e.execute("CREATE TABLE c (id INT)").unwrap();
    e.execute("CREATE TABLE o (cid INT)").unwrap();
    e.execute("INSERT INTO c VALUES (1), (2)").unwrap();
    e.execute("INSERT INTO o VALUES (1), (1), (2)").unwrap();

    let r = e
        .execute("SELECT c.id, o.cid FROM c LEFT OUTER JOIN o ON c.id = o.cid")
        .unwrap();
    // 客户 1 匹配 2 行，客户 2 匹配 1 行 ⇒ 3 行
    assert_eq!(r.rows.len(), 3);
}

/// 反向回归：右表为空的左连接 ⇒ 每个左表行都补 NULL，行数 = 左表行数。
#[test]
fn left_join_against_an_empty_right_table_pads_every_left_row() {
    let mut e = fresh();
    e.execute("CREATE TABLE c (id INT)").unwrap();
    e.execute("CREATE TABLE o (cid INT)").unwrap();
    e.execute("INSERT INTO c VALUES (1), (2), (3)").unwrap();

    let r = e
        .execute("SELECT c.id, o.cid FROM c LEFT OUTER JOIN o ON c.id = o.cid")
        .unwrap();
    assert_eq!(r.rows.len(), 3, "every left row must be padded with NULL");
    for row in &r.rows {
        assert!(
            matches!(row[1], Value::Null),
            "the right side must be NULL, got {:?}",
            row[1]
        );
    }
}
