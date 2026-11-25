//! E2E regression for Gitea #4708 — Chinese identifiers, MySQL backticks and
//! comments through the whole engine path (parse → executor → storage).
//!
//! Issue #4708 was closed on 2026-09-03 with sub-bugs #1 (Chinese table/column
//! names) and #3 (MySQL backticks) recorded as OR-downgrades; the block-comment
//! gap found on 2026-09-30 was fixed in the lexer on 2026-10-08. The parser
//! layer is covered by `crates/parser/tests/wp_a_legacy.rs`; this file covers
//! what that file cannot — the DDL/DML/SELECT path that actually stores and
//! reads the values.
//!
//! refs: docs/releases/v4.1.0/evidence/wp-a-reverify-2026-11-26/wp_a_reverify.txt

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, Value};
use std::sync::Arc;

fn fresh() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

fn extract_int(res: &sqlrustgo::ExecutorResult, row: usize, col: usize) -> i64 {
    match &res.rows[row][col] {
        Value::Integer(n) => *n,
        other => panic!("expected Integer at [{}][{}], got {:?}", row, col, other),
    }
}

fn extract_text(res: &sqlrustgo::ExecutorResult, row: usize, col: usize) -> String {
    match &res.rows[row][col] {
        Value::Text(s) => s.clone(),
        other => panic!("expected Text at [{}][{}], got {:?}", row, col, other),
    }
}

#[test]
fn issue_4708_chinese_table_and_column_names_roundtrip() {
    let mut engine = fresh();
    engine
        .execute("CREATE TABLE 用户 (编号 INTEGER, 名称 TEXT)")
        .expect("CREATE TABLE with Chinese identifiers must succeed");
    engine
        .execute("INSERT INTO 用户 (编号, 名称) VALUES (1, '张三')")
        .expect("INSERT into a Chinese-named table must succeed");
    engine
        .execute("INSERT INTO 用户 (编号, 名称) VALUES (2, '李四')")
        .expect("second INSERT must succeed");

    let res = engine
        .execute("SELECT 编号, 名称 FROM 用户")
        .expect("SELECT with Chinese identifiers must succeed");
    assert_eq!(res.rows.len(), 2, "both rows must come back");
    assert_eq!(extract_int(&res, 0, 0), 1);
    assert_eq!(extract_text(&res, 0, 1), "张三");
    assert_eq!(extract_int(&res, 1, 0), 2);
    assert_eq!(extract_text(&res, 1, 1), "李四");
}

#[test]
fn issue_4708_mysql_backtick_identifiers() {
    let mut engine = fresh();
    engine
        .execute("CREATE TABLE `订单` (`订单号` INTEGER, `金额` INTEGER)")
        .expect("CREATE TABLE with backtick-quoted identifiers must succeed");
    engine
        .execute("INSERT INTO `订单` (`订单号`, `金额`) VALUES (7, 42)")
        .expect("INSERT with backtick-quoted identifiers must succeed");

    let res = engine
        .execute("SELECT `订单号`, `金额` FROM `订单`")
        .expect("SELECT with backtick-quoted identifiers must succeed");
    assert_eq!(res.rows.len(), 1);
    assert_eq!(extract_int(&res, 0, 0), 7);
    assert_eq!(extract_int(&res, 0, 1), 42);
}

#[test]
fn issue_4708_block_and_line_comments_are_ignored_by_the_engine() {
    let mut engine = fresh();
    engine
        .execute("CREATE TABLE t (id INTEGER) /* trailing block comment */")
        .expect("block comment after DDL must be ignored");
    engine
        .execute("/* leading block comment */ INSERT INTO t (id) VALUES (1)")
        .expect("leading block comment must be ignored");
    engine
        .execute("INSERT INTO t (id) VALUES (2) -- 中文行注释")
        .expect("Chinese line comment must be ignored");

    let res = engine
        .execute("SELECT id FROM t /* block comment in the middle */")
        .expect("block comment inside a statement must be ignored");
    assert_eq!(res.rows.len(), 2, "both rows must come back");
}
