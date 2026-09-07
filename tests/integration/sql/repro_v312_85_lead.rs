//! V312-85 / Issue #4753: LEAD() window function parser + executor

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage};
use sqlrustgo_parser::parse;
use std::sync::Arc;

fn fresh_mem() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

#[test]
fn lead_parser_trace() {
    let tests = vec![
        "SELECT LEAD(a) OVER (ORDER BY b) FROM t",
        "SELECT LEAD(a, 1) OVER (ORDER BY b) FROM t",
        "SELECT lead(a, 1) OVER (ORDER BY b) FROM t",
        "SELECT lead(val, 1) OVER (ORDER BY id) FROM t",
        "SELECT LAG(a, 1) OVER (ORDER BY b) FROM t",
        "SELECT lag(val, 1) OVER (ORDER BY id) FROM t",
    ];
    for sql in tests {
        let r = parse(sql);
        match r {
            Ok(_) => println!("OK:   {sql}"),
            Err(e) => println!("FAIL: {sql}  ->  {e}"),
        }
    }
}

#[test]
fn lead_returns_next_row_value() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(id int, val int)").unwrap();
    x.execute("INSERT INTO t VALUES (1, 10)").unwrap();
    x.execute("INSERT INTO t VALUES (2, 20)").unwrap();
    x.execute("INSERT INTO t VALUES (3, 30)").unwrap();
    let r = x
        .execute("SELECT id, val, lead(val, 1) OVER (ORDER BY id) AS nxt FROM t")
        .expect("LEAD should parse");
    let rows = r.rows;
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0][2], sqlrustgo::Value::Integer(20));
    assert_eq!(rows[1][2], sqlrustgo::Value::Integer(30));
    assert_eq!(rows[2][2], sqlrustgo::Value::Null);
}

#[test]
fn lead_with_default() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(id int, val int)").unwrap();
    x.execute("INSERT INTO t VALUES (1, 10)").unwrap();
    x.execute("INSERT INTO t VALUES (2, 20)").unwrap();
    let r = x
        .execute("SELECT id, lead(val, 1, 99) OVER (ORDER BY id) AS nxt FROM t")
        .expect("LEAD with default");
    let rows = r.rows;
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0][1], sqlrustgo::Value::Integer(20));
    assert_eq!(rows[1][1], sqlrustgo::Value::Integer(99));
}

#[test]
fn lead_partition_by() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(g int, id int, val int)").unwrap();
    x.execute("INSERT INTO t VALUES (1, 1, 10)").unwrap();
    x.execute("INSERT INTO t VALUES (1, 2, 20)").unwrap();
    x.execute("INSERT INTO t VALUES (2, 1, 100)").unwrap();
    x.execute("INSERT INTO t VALUES (2, 2, 200)").unwrap();
    let r = x
        .execute("SELECT g, id, lead(val, 1) OVER (PARTITION BY g ORDER BY id) AS nxt FROM t")
        .expect("LEAD with PARTITION");
    assert_eq!(r.rows.len(), 4);
    assert_eq!(r.rows[0][2], sqlrustgo::Value::Integer(20));
    assert_eq!(r.rows[1][2], sqlrustgo::Value::Null);
    assert_eq!(r.rows[2][2], sqlrustgo::Value::Integer(200));
    assert_eq!(r.rows[3][2], sqlrustgo::Value::Null);
}
