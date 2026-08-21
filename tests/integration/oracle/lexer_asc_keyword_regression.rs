//! Regression test for V312-58 / Issue #4375: the lexer was missing the
//! `"ASC" => Token::Asc` mapping in its keyword table, so `ORDER BY col ASC,
//! ...` silently broke — the `ASC` token was emitted as a bare
//! `Identifier("ASC")`, parse_order_by's `Some(Token::Asc)` arm never
//! fired, the ASC token was left unconsumed, and the trailing
//! `, next_col, ... LIMIT n` chain was skipped. TPC-H Q1 (uses `ASC`)
//! and Q2 (`ORDER BY s_acctbal ASC, n_name, s_name, p_partkey LIMIT 20`)
//! both manifested as "everything after the first ASC is dropped".
//!
//! Run:
//!   cargo test --test lexer_asc_keyword_regression --all-features

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::sync::Arc;

fn engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut e = ExecutionEngine::new(storage.clone());
    e.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, val TEXT NOT NULL)")
        .unwrap();
    e.execute("INSERT INTO t VALUES (1, 'b'), (2, 'a'), (3, 'c')")
        .unwrap();
    e
}

#[test]
fn asc_keyword_consumed_in_order_by_single_item() {
    let mut e = engine();
    let r = e
        .execute("SELECT val FROM t ORDER BY val ASC")
        .unwrap_or_else(|err| panic!("ORDER BY val ASC failed: {}", err));
    let got: Vec<String> = r
        .rows
        .iter()
        .map(|row| match &row[0] {
            sqlrustgo::Value::Text(s) => s.clone(),
            other => panic!("unexpected cell type: {:?}", other),
        })
        .collect();
    assert_eq!(got, vec!["a", "b", "c"], "ASC must order ascending");
}

#[test]
fn asc_with_limit_returns_correct_rows() {
    let mut e = engine();
    let r = e
        .execute("SELECT val FROM t ORDER BY val ASC LIMIT 2")
        .unwrap_or_else(|err| panic!("ORDER BY val ASC LIMIT 2 failed: {}", err));
    let got: Vec<String> = r
        .rows
        .iter()
        .map(|row| match &row[0] {
            sqlrustgo::Value::Text(s) => s.clone(),
            other => panic!("unexpected cell type: {:?}", other),
        })
        .collect();
    assert_eq!(got, vec!["a", "b"], "ASC + LIMIT must cap at first two ASC");
}

#[test]
fn asc_with_limit_one_returns_single_row() {
    // The canonical Q2 SQL uses `LIMIT 20` after an ASC-keyed ORDER BY.
    // Without the lexer fix, `LIMIT 20` was silently dropped — the
    // SELECT returned the full ordered set instead of the first 20.
    // This test pins the Q2-shaped symptom at a tiny scale.
    let mut e = engine();
    let r = e
        .execute("SELECT val FROM t ORDER BY val ASC LIMIT 1")
        .unwrap_or_else(|err| panic!("ORDER BY val ASC LIMIT 1 failed: {}", err));
    assert_eq!(r.rows.len(), 1, "ASC + LIMIT 1 must return exactly one row");
    let got = match &r.rows[0][0] {
        sqlrustgo::Value::Text(s) => s.clone(),
        other => panic!("unexpected cell type: {:?}", other),
    };
    assert_eq!(got, "a", "smallest ASC value is 'a'");
}

#[test]
fn mixed_asc_desc_list_preserves_all_items() {
    // The bug also dropped everything after the first ASC, so a
    // multi-item list like `ORDER BY a ASC, b DESC, c ASC LIMIT n`
    // would only carry `a ASC` and silently truncate the rest
    // (including LIMIT). This test pins the multi-item form.
    let mut e = engine();
    e.execute("CREATE TABLE m (a INTEGER NOT NULL, b INTEGER NOT NULL, c INTEGER NOT NULL)")
        .unwrap();
    e.execute(
        "INSERT INTO m VALUES (1, 2, 100), (1, 1, 50), (2, 0, 25), (1, 2, 75)",
    )
    .unwrap();
    let r = e
        .execute("SELECT c FROM m ORDER BY a ASC, b DESC, c ASC LIMIT 3")
        .unwrap_or_else(|err| panic!("multi-key ORDER BY ASC LIMIT 3 failed: {}", err));
    let got: Vec<i64> = r
        .rows
        .iter()
        .map(|row| match &row[0] {
            sqlrustgo::Value::Integer(v) => *v,
            other => panic!("unexpected cell type: {:?}", other),
        })
        .collect();
    // Expected order: a ASC then b DESC then c ASC, capped at 3.
    // Rows sorted: (1,2,75), (1,2,100), (1,1,50), (2,0,25) -> first 3 = 75,100,50.
    assert_eq!(got, vec![75, 100, 50], "multi-key ASC/DESC list must sort correctly");
}