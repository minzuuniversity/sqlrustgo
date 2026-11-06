// storage-semantics: dml-insert-column-validation-5409
//! #5409: `INSERT … (列名列表)` 里的**未知列名**必须报错，不能被静默丢成 NULL。
//!
//! 此前引擎**完全不校验**列名列表：未知列名既不报错、也不被拒绝，而是被
//! `materialise_default_tokens` 的 `position()` 映射静默丢弃 ⇒ INSERT 进去的是
//! 一行全 NULL。
//!
//! 危害不在「报错更好看」，而在**假绿**：任何依赖「列名写错会失败」的装载脚本，
//! 都会在当前引擎上静默写入全 NULL 行，后续 `WHERE` 匹配不到任何行，而断言照样
//! 通过 —— 这正是 `tpch_compliance_test` 当年用
//! `INSERT INTO lineitem (c0, c1, ..., c15) VALUES (...)` 装载 62000 行全 NULL
//! 却仍然通过断言的原因。
//!
//! MySQL 的正确行为：`ERROR 1054 (42S22): Unknown column 'c0' in 'field list'`。

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, Value};
use std::sync::Arc;

fn fresh() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

fn count_rows(e: &mut ExecutionEngine<MemoryStorage>, table: &str) -> usize {
    let r = e
        .execute(&format!("SELECT COUNT(*) FROM {}", table))
        .unwrap();
    match &r.rows[0][0] {
        Value::Integer(n) => *n as usize,
        other => panic!("expected Integer count, got {:?}", other),
    }
}

/// 本 issue 的核心：未知列名必须**报错**，且**不得**留下任何行。
#[test]
fn unknown_column_in_insert_column_list_is_rejected_and_inserts_nothing() {
    let mut e = fresh();
    e.execute("CREATE TABLE t (a INTEGER, b TEXT)").unwrap();

    let err = e
        .execute("INSERT INTO t (c0, c1) VALUES (1, 2)")
        .expect_err("unknown column must be rejected");
    // MySQL 兼容：ER_BAD_FIELD_ERROR(1054) / SQLSTATE 42S22
    assert_eq!(
        err.mysql_error_code(),
        1054,
        "expected ER_BAD_FIELD_ERROR, got {:?}",
        err
    );
    assert_eq!(err.sqlstate(), "42S22", "expected SQLSTATE 42S22");

    // 关键：**没有插进去**。修复前这里是 1（一行全 NULL）。
    assert_eq!(
        count_rows(&mut e, "t"),
        0,
        "a rejected INSERT must not leave a row behind"
    );
}

/// 只报「第一个」未知列名就够 —— 但报出来的必须是**真的**那个列名，
/// 而不是笼统的「列不存在」。
#[test]
fn the_reported_column_is_the_offending_one() {
    let mut e = fresh();
    e.execute("CREATE TABLE t (a INTEGER, b TEXT)").unwrap();

    let err = e
        .execute("INSERT INTO t (a, nope) VALUES (1, 2)")
        .expect_err("unknown column must be rejected");
    let msg = err.to_string();
    assert!(
        msg.contains("nope"),
        "error must name the offending column, got {:?}",
        msg
    );
    assert_eq!(
        err.mysql_error_code(),
        1054,
        "expected ER_BAD_FIELD_ERROR, got {:?}",
        err
    );
}

/// `INSERT … SELECT` 走的是**另一条**映射路径（`map_select_result_to_records`），
/// 但校验点在分叉之前 ⇒ 同样必须报错。
#[test]
fn unknown_column_in_insert_select_is_rejected() {
    let mut e = fresh();
    e.execute("CREATE TABLE t (a INTEGER, b TEXT)").unwrap();
    e.execute("CREATE TABLE src (x INTEGER)").unwrap();
    e.execute("INSERT INTO src VALUES (7)").unwrap();

    let err = e
        .execute("INSERT INTO t (c0) SELECT x FROM src")
        .expect_err("unknown column must be rejected");
    assert_eq!(
        err.mysql_error_code(),
        1054,
        "expected ER_BAD_FIELD_ERROR, got {:?}",
        err
    );
    assert_eq!(count_rows(&mut e, "t"), 0);
}

/// 防「校验太严」的反向回归：列名**大小写不敏感**（与映射规则一致），
/// 否则会出现「校验通过但映射不上」或「合法 INSERT 被拒」的裂缝。
#[test]
fn column_names_are_matched_case_insensitively() {
    let mut e = fresh();
    e.execute("CREATE TABLE t (a INTEGER, b TEXT)").unwrap();

    e.execute("INSERT INTO t (A, B) VALUES (1, 'x')")
        .expect("column names are case-insensitive in MySQL");
    assert_eq!(count_rows(&mut e, "t"), 1);
}

/// 防「校验太严」的反向回归：合法的列名列表必须照常工作，
/// 没列出的列取默认值 / NULL。
#[test]
fn a_valid_column_list_still_inserts() {
    let mut e = fresh();
    e.execute("CREATE TABLE t (a INTEGER, b TEXT)").unwrap();

    e.execute("INSERT INTO t (b) VALUES ('x')").unwrap();
    assert_eq!(count_rows(&mut e, "t"), 1);

    let r = e.execute("SELECT a, b FROM t").unwrap();
    match &r.rows[0][0] {
        Value::Null => {}
        other => panic!("column a was not listed, expect NULL, got {:?}", other),
    }
    match &r.rows[0][1] {
        Value::Text(s) => assert_eq!(s, "x"),
        other => panic!("expected the inserted text, got {:?}", other),
    }
}

/// `INSERT … SELECT` 路径上的列名匹配也必须大小写不敏感 —— 这条路径原本
/// 自带一个检查，但那个检查是**大小写敏感**的，于是
/// `INSERT INTO t (A) SELECT ...` 会被拒、`INSERT INTO t (A) VALUES ...` 却能过。
#[test]
fn insert_select_column_names_are_matched_case_insensitively() {
    let mut e = fresh();
    e.execute("CREATE TABLE t (a INTEGER, b TEXT)").unwrap();
    e.execute("CREATE TABLE src (x INTEGER)").unwrap();
    e.execute("INSERT INTO src VALUES (7)").unwrap();

    e.execute("INSERT INTO t (A) SELECT x FROM src")
        .expect("column names are case-insensitive in MySQL");
    assert_eq!(count_rows(&mut e, "t"), 1);
}

/// 不带列名列表的 INSERT 不受影响（按位置对齐）。
#[test]
fn insert_without_a_column_list_is_unaffected() {
    let mut e = fresh();
    e.execute("CREATE TABLE t (a INTEGER, b TEXT)").unwrap();

    e.execute("INSERT INTO t VALUES (1, 'x')").unwrap();
    assert_eq!(count_rows(&mut e, "t"), 1);
}
