//! G2 INT-2 ParallelExecutor Oracle (V4 fix)
//!
//! 用 in-process ExecutionEngine 跑相同 query, 验证 parallel_degree=1
//! (sequential) vs parallel_degree=4 (parallel) 产生**完全一致**的结果集.
//!
//! Oracle: 同一 query 的 sequential 结果是 ground truth, parallel 必须 MATCH.

#[path = "../../common/mod.rs"]
mod common;

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage};
use sqlrustgo_types::Value;
use std::sync::Arc;

fn make_engine(parallel_degree: usize) -> ExecutionEngine<MemoryStorage> {
    let mut engine = ExecutionEngine::new(Arc::new(RwLock::new(MemoryStorage::new())));
    engine.set_parallel_degree(parallel_degree);
    engine
}

fn setup_table(engine: &mut ExecutionEngine<MemoryStorage>) {
    engine
        .execute("CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT, val INTEGER)")
        .unwrap();
    for i in 1..=50 {
        engine
            .execute(&format!(
                "INSERT INTO t VALUES ({}, 'name_{}', {})",
                i,
                i,
                i * 10
            ))
            .unwrap();
    }
}

#[test]
fn g2_int2_parallel_matches_sequential() {
    let mut seq = make_engine(1);
    let mut par = make_engine(4);
    setup_table(&mut seq);
    setup_table(&mut par);

    let queries = [
        "SELECT COUNT(*) FROM t",
        "SELECT SUM(val) FROM t",
        "SELECT val FROM t WHERE val > 200 ORDER BY val LIMIT 5",
        "SELECT id, name FROM t WHERE id <= 10 ORDER BY id DESC",
        "SELECT COUNT(*) FROM t WHERE name LIKE 'name_1%'",
    ];

    let mut all_match = true;
    for (i, sql) in queries.iter().enumerate() {
        let seq_r = seq.execute(sql).expect("seq failed");
        let par_r = par.execute(sql).expect("par failed");
        if seq_r.rows != par_r.rows {
            all_match = false;
            eprintln!(
                "[FAIL] query {}: {} row_count seq={} par={}",
                i,
                sql,
                seq_r.rows.len(),
                par_r.rows.len()
            );
        } else {
            eprintln!("[OK] query {}: {} row_count={}", i, sql, seq_r.rows.len());
        }
    }

    assert!(
        all_match,
        "parallel_degree=4 results diverge from sequential oracle"
    );
}

#[test]
fn g2_int2_full_table_scan_count() {
    let mut engine = make_engine(4);
    setup_table(&mut engine);
    let r = engine.execute("SELECT COUNT(*) FROM t").unwrap();
    assert_eq!(r.rows.len(), 1, "COUNT(*) should return 1 row");
    assert_eq!(
        r.rows[0][0],
        Value::Integer(50),
        "Oracle: 50 rows in t (setup inserted 50)"
    );
}

#[test]
fn g2_int2_where_filter_count() {
    let mut engine = make_engine(4);
    setup_table(&mut engine);
    let r = engine
        .execute("SELECT COUNT(*) FROM t WHERE val > 250")
        .unwrap();
    assert_eq!(r.rows.len(), 1);
    let count = match &r.rows[0][0] {
        Value::Integer(n) => *n,
        _ => panic!("expected Int, got {:?}", r.rows[0][0]),
    };
    assert_eq!(count, 25, "Oracle: vals 260..500 in steps of 10 = 25 rows");
}
