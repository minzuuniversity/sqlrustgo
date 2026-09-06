// V313-102 / Issue #4704 verification: end-to-end smoke test for the
// three sub-problems the issue body enumerates. Each is exercised
// with an in-memory storage engine so we can assert the result
// shape without bringing in a full sqlite3 / server harness.

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, Value};
use std::sync::Arc;

fn fresh_mem() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

#[test]
fn verify_4704_sub1_values_in_cte_anchor() {
    let mut x = fresh_mem();
    let r = x
        .execute(
            "WITH RECURSIVE walk(n) AS (VALUES (1) UNION ALL \
             SELECT n + 2 FROM walk WHERE n + 2 <= 4) \
             SELECT * FROM walk",
        )
        .expect("VALUES in CTE anchor should execute");
    let n: Vec<i64> = r
        .rows
        .iter()
        .map(|row| match &row[0] {
            Value::Integer(v) => *v,
            other => panic!("expected integer, got {:?}", other),
        })
        .collect();
    assert_eq!(n, vec![1, 3], "VALUES-anchored recursive walk");
}

#[test]
fn verify_4704_sub3_sqlite_sequence() {
    let mut x = fresh_mem();
    x.execute(
        "CREATE TABLE t(id INTEGER PRIMARY KEY AUTOINCREMENT, val INT)",
    )
    .unwrap();
    x.execute("INSERT INTO t(val) VALUES (10), (20), (30)").unwrap();
    let r = x
        .execute("SELECT * FROM sqlite_sequence")
        .expect("sqlite_sequence should be queryable");
    assert!(
        !r.rows.is_empty(),
        "sqlite_sequence must contain at least one row for table `t`"
    );
}

#[test]
fn verify_4704_sub2_multi_anchor_union() {
    let mut x = fresh_mem();
    let r = x
        .execute(
            "WITH RECURSIVE walk(n) AS (\
                 VALUES (1) UNION ALL VALUES (10) \
                 UNION ALL \
                 SELECT n + 2 FROM walk WHERE n + 2 <= 10\
             ) SELECT * FROM walk",
        )
        .expect("multi-anchor UNION in recursive CTE should execute");
    let n: Vec<i64> = r
        .rows
        .iter()
        .map(|row| match &row[0] {
            Value::Integer(v) => *v,
            other => panic!("expected integer, got {:?}", other),
        })
        .collect();
    assert!(n.len() >= 2, "multi-anchor UNION produced too few rows: {:?}", n);
    assert!(n.contains(&1), "anchor 1 missing from {:?}", n);
    assert!(n.contains(&10), "anchor 10 missing from {:?}", n);
}