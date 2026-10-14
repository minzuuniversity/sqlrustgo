//! #5305: EXPLAIN must report `est_rows` and `est_cost` for each scan
//! operator (per A05-V410-003 spec: 计划/页访问量).
//!
//! V4.1.0 P1: A05 finder (Issue #5305) reports that EXPLAIN emits
//! operator names only — no cost, no estimated rows, no page-visit
//! counter. This test pins the est_rows / est_cost suffix on scan
//! lines so subsequent optimizer work has a regression target.

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, Value};
use std::sync::Arc;

fn fresh() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

fn explain_lines(e: &mut ExecutionEngine<MemoryStorage>, sql: &str) -> Vec<String> {
    let r = e.execute(sql).expect("EXPLAIN must execute");
    r.rows
        .iter()
        .map(|row| match &row[0] {
            Value::Text(s) => s.clone(),
            other => format!("{other:?}"),
        })
        .collect()
}

#[test]
fn explain_emits_est_rows_for_index_scan() {
    let mut e = fresh();
    e.execute("CREATE TABLE t (id INT PRIMARY KEY, val INT)")
        .unwrap();
    e.execute("CREATE INDEX idx_val ON t(val)").unwrap();
    for i in 0..4 {
        e.execute(&format!("INSERT INTO t VALUES ({i}, {i})"))
            .unwrap();
    }
    let lines = explain_lines(&mut e, "EXPLAIN SELECT * FROM t WHERE val > 0");
    let scan = lines
        .iter()
        .find(|l| l.starts_with("IndexScan") || l.starts_with("SeqScan"))
        .expect("EXPLAIN must emit a scan line");
    assert!(
        scan.contains("est_rows=4"),
        "#5305 IndexScan must carry est_rows=4 (4 rows in t), got: {scan}"
    );
    assert!(
        scan.contains("est_cost="),
        "#5305 IndexScan must carry est_cost=..., got: {scan}"
    );
}

#[test]
fn explain_emits_est_rows_for_seq_scan() {
    let mut e = fresh();
    e.execute("CREATE TABLE t (id INT PRIMARY KEY, val INT)")
        .unwrap();
    for i in 0..3 {
        e.execute(&format!("INSERT INTO t VALUES ({i}, {i})"))
            .unwrap();
    }
    // No index → SeqScan.
    let lines = explain_lines(&mut e, "EXPLAIN SELECT * FROM t");
    let scan = lines
        .iter()
        .find(|l| l.starts_with("SeqScan"))
        .expect("EXPLAIN must emit SeqScan when no index");
    assert!(
        scan.contains("est_rows=3"),
        "#5305 SeqScan must carry est_rows=3, got: {scan}"
    );
}

#[test]
fn explain_emits_est_rows_for_join() {
    let mut e = fresh();
    e.execute("CREATE TABLE u (id INT PRIMARY KEY, name TEXT)")
        .unwrap();
    e.execute("CREATE TABLE o (id INT PRIMARY KEY, user_id INT)")
        .unwrap();
    for i in 0..2 {
        e.execute(&format!("INSERT INTO u VALUES ({i}, 'n{i}')"))
            .unwrap();
        e.execute(&format!("INSERT INTO o VALUES ({i}, {i})"))
            .unwrap();
    }
    let lines = explain_lines(
        &mut e,
        "EXPLAIN SELECT u.name, o.id FROM u JOIN o ON u.id = o.user_id",
    );
    // Both tables must carry est_rows=N.
    let u_line = lines
        .iter()
        .find(|l| l.contains("u") && (l.starts_with("IndexScan") || l.starts_with("SeqScan")))
        .expect("EXPLAIN must emit a scan line for u");
    let o_line = lines
        .iter()
        .find(|l| l.contains("o") && (l.starts_with("IndexScan") || l.starts_with("SeqScan")))
        .expect("EXPLAIN must emit a scan line for o");
    assert!(
        u_line.contains("est_rows=2"),
        "u line missing est_rows=2: {u_line}"
    );
    assert!(
        o_line.contains("est_rows=2"),
        "o line missing est_rows=2: {o_line}"
    );
}

#[test]
fn explain_est_rows_format_stable() {
    // Lock the exact format string: `IndexScan t (est_rows=N, est_cost=M.N)`
    // and `SeqScan t (est_rows=N, est_cost=M.N)`. Future optimizers that
    // change the format must update this test.
    let mut e = fresh();
    e.execute("CREATE TABLE t (id INT PRIMARY KEY, val INT)")
        .unwrap();
    e.execute("CREATE INDEX idx_val ON t(val)").unwrap();
    e.execute("INSERT INTO t VALUES (1, 10)").unwrap();
    let lines = explain_lines(&mut e, "EXPLAIN SELECT * FROM t WHERE val > 0");
    let scan = lines
        .iter()
        .find(|l| l.starts_with("IndexScan"))
        .expect("IndexScan must be present");
    let suffix_open = scan.find("(est_rows=").expect("(est_rows= suffix required");
    let suffix_close = scan[suffix_open..]
        .find(')')
        .expect("suffix must close with )");
    let suffix = &scan[suffix_open..=suffix_open + suffix_close];
    assert_eq!(suffix, "(est_rows=1, est_cost=0.1)");
}
