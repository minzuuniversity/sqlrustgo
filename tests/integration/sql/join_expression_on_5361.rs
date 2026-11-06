//! #5361: 合法的 `LEFT JOIN` 被拒绝 —— ON 子句含表达式时直接报错。
//!
//! ```sql
//! SELECT l.id, l.x, r.id
//! FROM t_join_l l LEFT JOIN t_join_l r ON l.id = r.id + 1
//! ```
//!
//! MySQL / sqlite / postgres 都正常返回（**左表全保留**，未匹配侧为 NULL）。
//! SQLRustGo 却直接报错：
//!
//! ```text
//! Join condition must reference one column from each side
//! ```
//!
//! 缺陷不在「命名」，而在**能力缺口**：ON 子句被要求必须是裸列对，
//! `r.id + 1` 是表达式 ⇒ 被拒。任何 ON 含算术、函数或列间比较的 JOIN 都不可用
//! —— 对报表类查询（`ON a.x = b.y * 2`、`ON date(a.d) = b.d`）是硬阻断。
//!
//! 修法：表达式没法当哈希键 ⇒ 退化成**笛卡尔积 + ON 后置过滤**
//! （与 non-equi 的处理方式一致）。但那条路径此前**没有外连接补 NULL**，
//! 所以光改路由还不够：左表行在匹配不上时必须以 NULL 补齐，而不是消失。

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, Value};
use std::sync::Arc;

fn fresh() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

fn seed(e: &mut ExecutionEngine<MemoryStorage>) {
    e.execute("CREATE TABLE t_join_l (id INT, x TEXT)").unwrap();
    e.execute("INSERT INTO t_join_l VALUES (1, 'one'), (2, 'two')")
        .unwrap();
}

/// 本 issue 的最小复现：LEFT JOIN + ON 里的算术表达式。
///
/// sqlite 的答案（issue 记的对照基准）：
/// ```text
/// id,x,id
/// 1,one,NULL
/// 2,two,1
/// ```
#[test]
fn left_join_with_an_arithmetic_on_clause_is_accepted_and_pads_with_null() {
    let mut e = fresh();
    seed(&mut e);

    let r = e
        .execute(
            "SELECT l.id, l.x, r.id FROM t_join_l l LEFT JOIN t_join_l r \
             ON l.id = r.id + 1 ORDER BY l.id",
        )
        .expect("a legal LEFT JOIN must not be rejected");
    assert_eq!(r.rows.len(), 2, "both left rows must be present");
    // l.id = 1 ⇒ 需要 r.id = 0 ⇒ 无匹配 ⇒ 右补 NULL
    assert_eq!(r.rows[0][0], Value::Integer(1));
    assert_eq!(r.rows[0][1], Value::Text("one".to_string()));
    assert_eq!(
        r.rows[0][2],
        Value::Null,
        "the unmatched left row must be padded with NULL, not dropped"
    );
    // l.id = 2 ⇒ 需要 r.id = 1 ⇒ 匹配到 (1, 'one')
    assert_eq!(r.rows[1][0], Value::Integer(2));
    assert_eq!(r.rows[1][1], Value::Text("two".to_string()));
    assert_eq!(r.rows[1][2], Value::Integer(1));
}

/// INNER JOIN + 表达式：只返回真正匹配的配对（未匹配的左表行不出现）。
#[test]
fn inner_join_with_an_arithmetic_on_clause_returns_only_matching_pairs() {
    let mut e = fresh();
    seed(&mut e);

    let r = e
        .execute(
            "SELECT l.id, r.id FROM t_join_l l JOIN t_join_l r ON l.id = r.id + 1 \
             ORDER BY l.id",
        )
        .expect("a legal JOIN must not be rejected");
    assert_eq!(r.rows.len(), 1, "only the matching pair may be returned");
    assert_eq!(r.rows[0][0], Value::Integer(2));
    assert_eq!(r.rows[0][1], Value::Integer(1));
}

/// 函数调用在 ON 里同样合法（`ON date(a.d) = b.d` 这类）。
#[test]
fn join_with_a_function_call_in_the_on_clause_is_accepted() {
    let mut e = fresh();
    e.execute("CREATE TABLE a (id INT)").unwrap();
    e.execute("CREATE TABLE b (id INT)").unwrap();
    e.execute("INSERT INTO a VALUES (1), (2)").unwrap();
    e.execute("INSERT INTO b VALUES (2)").unwrap();

    let r = e
        .execute("SELECT a.id, b.id FROM a LEFT JOIN b ON abs(a.id) = b.id ORDER BY a.id")
        .expect("a legal JOIN must not be rejected");
    assert_eq!(r.rows.len(), 2, "both left rows must be present");
    assert_eq!(r.rows[0][2 - 1], Value::Null, "a.id=1 has no match");
    assert_eq!(r.rows[1][1], Value::Integer(2), "a.id=2 matches b.id=2");
}

/// 反向回归：ON 子句是**裸列对**时，哈希连接的快路径不受影响。
#[test]
fn plain_column_on_clause_is_unaffected() {
    let mut e = fresh();
    seed(&mut e);

    let r = e
        .execute("SELECT l.id, r.id FROM t_join_l l LEFT JOIN t_join_l r ON l.id = r.id")
        .expect("plain join still works");
    assert_eq!(r.rows.len(), 2, "each left row matches itself");
    for row in &r.rows {
        assert_eq!(row[0], row[1]);
    }
}

/// 反向回归：non-equi 的 ON（`a.x < b.y`）此前就能跑，行为不得改变。
#[test]
fn non_equi_on_clause_is_unaffected() {
    let mut e = fresh();
    e.execute("CREATE TABLE a (x INT)").unwrap();
    e.execute("CREATE TABLE b (y INT)").unwrap();
    e.execute("INSERT INTO a VALUES (1), (5)").unwrap();
    e.execute("INSERT INTO b VALUES (1), (5)").unwrap();

    let r = e
        .execute("SELECT a.x, b.y FROM a JOIN b ON a.x < b.y")
        .expect("non-equi join still works");
    assert_eq!(r.rows.len(), 1);
    assert_eq!(r.rows[0][0], Value::Integer(1));
    assert_eq!(r.rows[0][1], Value::Integer(5));
}
