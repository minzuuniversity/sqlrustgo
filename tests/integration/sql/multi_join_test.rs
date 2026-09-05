//! F-10 multi-join coverage tests (EXEC-07)
//! Verifies that 3-way joins execute through ExecutionEngine and produce
//! correct row counts. Note: 4-way joins and CROSS JOIN are pending
//! parser support (tracked in #2987 / #2991).
use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::sync::Arc;

fn fresh_engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

#[test]
fn test_three_way_inner_join_count() {
    let mut e = fresh_engine();
    e.execute("CREATE TABLE a (id INTEGER, x INTEGER)").unwrap();
    e.execute("CREATE TABLE b (id INTEGER, y INTEGER)").unwrap();
    e.execute("CREATE TABLE c (id INTEGER, z INTEGER)").unwrap();
    e.execute("INSERT INTO a VALUES (1,10),(2,20),(3,30)")
        .unwrap();
    e.execute("INSERT INTO b VALUES (1,100),(2,200),(3,300),(4,400)")
        .unwrap();
    e.execute("INSERT INTO c VALUES (1,1000),(2,2000),(3,3000)")
        .unwrap();
    let r = e
        .execute("SELECT COUNT(*) FROM a JOIN b ON a.id = b.id JOIN c ON b.id = c.id")
        .unwrap();
    assert_eq!(r.rows.len(), 1);
    let count = match &r.rows[0][0] {
        sqlrustgo::Value::Integer(n) => *n,
        _ => panic!("expected integer count"),
    };
    assert_eq!(count, 3, "expected 3 from 3-way join");
}

#[test]
fn test_left_join_with_null_match() {
    let mut e = fresh_engine();
    e.execute("CREATE TABLE a (id INTEGER, name TEXT)").unwrap();
    e.execute("CREATE TABLE b (id INTEGER, value INTEGER)")
        .unwrap();
    e.execute("INSERT INTO a VALUES (1,'x'),(2,'y'),(3,'z')")
        .unwrap();
    e.execute("INSERT INTO b VALUES (1,100),(3,300)").unwrap();
    let r = e
        .execute("SELECT a.id, b.value FROM a LEFT JOIN b ON a.id = b.id")
        .unwrap();
    assert_eq!(r.rows.len(), 3, "LEFT JOIN must emit one row per left");
    let id2 = r
        .rows
        .iter()
        .find(|row| matches!(&row[0], sqlrustgo::Value::Integer(2)))
        .expect("id=2 must be present");
    assert!(
        matches!(&id2[1], sqlrustgo::Value::Null),
        "id=2 should have NULL for b.value, got {:?}",
        id2[1]
    );
}

#[test]
fn test_two_way_inner_join_count() {
    // Baseline: simple 2-way join produces the expected cartesian-with-filter count.
    let mut e = fresh_engine();
    e.execute("CREATE TABLE a (id INTEGER)").unwrap();
    e.execute("CREATE TABLE b (id INTEGER)").unwrap();
    e.execute("INSERT INTO a VALUES (1),(2),(3),(4)").unwrap();
    e.execute("INSERT INTO b VALUES (1),(2),(3)").unwrap();
    let r = e
        .execute("SELECT COUNT(*) FROM a JOIN b ON a.id = b.id")
        .unwrap();
    let count = match &r.rows[0][0] {
        sqlrustgo::Value::Integer(n) => *n,
        _ => panic!("expected integer count"),
    };
    assert_eq!(count, 3, "a∩b should have 3 matching rows");
}

#[test]
fn test_left_join_no_matches_yields_left_rows() {
    // Edge case: right side is empty -> LEFT JOIN should still emit
    // one row per left row with NULLs on the right.
    let mut e = fresh_engine();
    e.execute("CREATE TABLE a (id INTEGER)").unwrap();
    e.execute("CREATE TABLE b (id INTEGER)").unwrap();
    e.execute("INSERT INTO a VALUES (1),(2),(3)").unwrap();
    let r = e
        .execute("SELECT a.id, b.id FROM a LEFT JOIN b ON a.id = b.id")
        .unwrap();
    assert_eq!(
        r.rows.len(),
        3,
        "LEFT JOIN with empty right still emits 3 rows"
    );
}


#[test]
fn test_right_join_preserves_right_rows() {
    // Issue #4639: RIGHT JOIN should preserve all right-side rows
    let mut e = fresh_engine();
    e.execute("CREATE TABLE a (id INTEGER, name TEXT)").unwrap();
    e.execute("CREATE TABLE b (id INTEGER, city TEXT)").unwrap();
    // a has 1,2,3; b has 1,3
    e.execute("INSERT INTO a VALUES (1,'alice'),(2,'bob'),(3,'charlie')")
        .unwrap();
    e.execute("INSERT INTO b VALUES (1,'NY'),(3,'LA')").unwrap();
    // RIGHT JOIN: all rows from b should appear
    let r = e
        .execute("SELECT a.id, a.name, b.id, b.city FROM a RIGHT JOIN b ON a.id = b.id")
        .unwrap();
    // b has 2 rows (id=1, id=3), both should appear
    assert_eq!(r.rows.len(), 2, "RIGHT JOIN must emit one row per right");
    // Find the row for b.id=1 - should have a.name='alice'
    let row1 = r.rows.iter().find(|row| matches!(&row[2], sqlrustgo::Value::Integer(1))).unwrap();
    assert!(matches!(&row1[1], sqlrustgo::Value::Text(t) if t == "alice"), "id=1 should match alice");
    // Find the row for b.id=3 - should have a.name='charlie'
    let row3 = r.rows.iter().find(|row| matches!(&row[2], sqlrustgo::Value::Integer(3))).unwrap();
    assert!(matches!(&row3[1], sqlrustgo::Value::Text(t) if t == "charlie"), "id=3 should match charlie");
}
