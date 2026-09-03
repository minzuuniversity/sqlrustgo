//! V312-73 / Issue #4649 regression integration test:
//! `LEFT JOIN ... USING (col)` and `INNER JOIN ... USING (cols)` must
//! apply USING-merge semantics — match on `t1.col = t2.col` for each
//! column in the list, and project each USING column exactly once in
//! the output. Before this fix, the parser silently dropped the USING
//! clause and the join collapsed to a Cartesian product (or a
//! cross-product-with-ON-true LEFT JOIN that returned N_left × N_right
//! rows).

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, Value};
use std::sync::Arc;

fn fresh() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

#[test]
fn v312_73_inner_join_using_single_col() {
    let mut x = fresh();
    x.execute("CREATE TABLE t1 (id INT, a TEXT)").unwrap();
    x.execute("CREATE TABLE t2 (id INT, b TEXT)").unwrap();
    x.execute("INSERT INTO t1 VALUES (1, 'x'), (2, 'y'), (3, 'z')")
        .unwrap();
    x.execute("INSERT INTO t2 VALUES (2, 'B'), (3, 'C'), (4, 'D')")
        .unwrap();
    let res = x
        .execute("SELECT t1.id, t1.a, t2.b FROM t1 INNER JOIN t2 USING (id) ORDER BY t1.id")
        .unwrap();
    assert_eq!(res.rows.len(), 2);
    assert_eq!(res.rows[0][0], Value::Integer(2));
    assert_eq!(res.rows[0][1], Value::Text("y".to_string()));
    assert_eq!(res.rows[0][2], Value::Text("B".to_string()));
    assert_eq!(res.rows[1][0], Value::Integer(3));
    assert_eq!(res.rows[1][1], Value::Text("z".to_string()));
    assert_eq!(res.rows[1][2], Value::Text("C".to_string()));
}

#[test]
fn v312_73_inner_join_using_no_cross_product() {
    // V312-73 regression: the previous bug returned N_left * N_right rows
    // (the Cartesian product) because USING was silently dropped. With
    // USING-merge, only rows whose USING column matches are returned.
    let mut x = fresh();
    x.execute("CREATE TABLE t1 (id INT)").unwrap();
    x.execute("CREATE TABLE t2 (id INT)").unwrap();
    x.execute("INSERT INTO t1 VALUES (1), (2), (3)").unwrap();
    x.execute("INSERT INTO t2 VALUES (10), (20)").unwrap();
    let res = x
        .execute("SELECT COUNT(*) FROM t1 INNER JOIN t2 USING (id)")
        .unwrap();
    // No overlapping ids, so 0 rows match.
    assert_eq!(res.rows.len(), 1);
    assert_eq!(res.rows[0][0], Value::Integer(0));
}

#[test]
fn v312_73_left_join_using_preserves_unmatched_left() {
    // V312-73 / Issue #4649: the original bug — `LEFT JOIN ... USING (col)`
    // returned N_left * N_right rows. Verify LEFT JOIN produces exactly
    // N_left rows (each left row matched on the USING column or null-
    // padded), not a Cartesian product.
    let mut x = fresh();
    x.execute("CREATE TABLE t1 (id INT, a TEXT)").unwrap();
    x.execute("CREATE TABLE t2 (id INT, b TEXT)").unwrap();
    x.execute("INSERT INTO t1 VALUES (1, 'x'), (2, 'y'), (3, 'z')")
        .unwrap();
    x.execute("INSERT INTO t2 VALUES (2, 'B')").unwrap();
    let res = x
        .execute("SELECT t1.id, t1.a, t2.b FROM t1 LEFT JOIN t2 USING (id) ORDER BY t1.id")
        .unwrap();
    assert_eq!(res.rows.len(), 3);
    assert_eq!(res.rows[0][0], Value::Integer(1));
    assert_eq!(res.rows[0][1], Value::Text("x".to_string()));
    assert!(matches!(res.rows[0][2], Value::Null));
    assert_eq!(res.rows[1][0], Value::Integer(2));
    assert_eq!(res.rows[1][1], Value::Text("y".to_string()));
    assert_eq!(res.rows[1][2], Value::Text("B".to_string()));
    assert_eq!(res.rows[2][0], Value::Integer(3));
    assert_eq!(res.rows[2][1], Value::Text("z".to_string()));
    assert!(matches!(res.rows[2][2], Value::Null));
}

#[test]
fn v312_73_using_projects_single_column() {
    // V312-73: USING must project each USING column exactly once in
    // the output rows. A SELECT * with USING(col) returns one col per
    // side, with the duplicate right-side USING column dropped.
    let mut x = fresh();
    x.execute("CREATE TABLE t1 (id INT, a INT)").unwrap();
    x.execute("CREATE TABLE t2 (id INT, b INT)").unwrap();
    x.execute("INSERT INTO t1 VALUES (1, 10)").unwrap();
    x.execute("INSERT INTO t2 VALUES (1, 20)").unwrap();
    let res = x
        .execute("SELECT * FROM t1 INNER JOIN t2 USING (id)")
        .unwrap();
    assert_eq!(res.rows.len(), 1);
    // Layout: [t1.id, t1.a, t2.b]  — t2.id is projected away.
    assert_eq!(res.rows[0].len(), 3);
    assert_eq!(res.rows[0][0], Value::Integer(1));
    assert_eq!(res.rows[0][1], Value::Integer(10));
    assert_eq!(res.rows[0][2], Value::Integer(20));
}

#[test]
fn v312_73_using_multi_columns() {
    // V312-73: USING (col1, col2) matches on the conjunction and projects
    // both USING columns exactly once.
    let mut x = fresh();
    x.execute("CREATE TABLE t1 (id INT, code INT, a TEXT)").unwrap();
    x.execute("CREATE TABLE t2 (id INT, code INT, b TEXT)").unwrap();
    x.execute("INSERT INTO t1 VALUES (1, 100, 'x1'), (1, 200, 'x2')")
        .unwrap();
    x.execute("INSERT INTO t2 VALUES (1, 100, 'B1'), (1, 300, 'B2')")
        .unwrap();
    let res = x
        .execute(
            "SELECT * FROM t1 INNER JOIN t2 USING (id, code) ORDER BY t1.a",
        )
        .unwrap();
    assert_eq!(res.rows.len(), 1);
    // Layout: [t1.id, t1.code, t1.a, t2.b]  — both right USING cols dropped.
    assert_eq!(res.rows[0].len(), 4);
    assert_eq!(res.rows[0][0], Value::Integer(1));
    assert_eq!(res.rows[0][1], Value::Integer(100));
    assert_eq!(res.rows[0][2], Value::Text("x1".to_string()));
    assert_eq!(res.rows[0][3], Value::Text("B1".to_string()));
}