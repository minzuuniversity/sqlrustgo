//! V312-58 / PR #4780 — Window function frame coverage.
//!
//! Each test is one of:
//! - SQL:1999 reference behaviour assertion (positive case we want to pass).
//! - Reproducer for a known issue with a documented expected value.
//!
//! Tables:
//!   - `t(id, val)` with rows (1,10), (2,20), (3,30) (ordered by id)
//!   - `tw` same shape for additional frame tests
//!
//! Each assertion checks a single window-function / frame-clause combination
//! using small int values so that the failure messages stay readable.

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, Value};
use std::sync::Arc;

fn fresh() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

fn setup_table(x: &mut ExecutionEngine<MemoryStorage>) {
    x.execute("CREATE TABLE t(id int, val int)").unwrap();
    x.execute("INSERT INTO t VALUES (1, 10)").unwrap();
    x.execute("INSERT INTO t VALUES (2, 20)").unwrap();
    x.execute("INSERT INTO t VALUES (3, 30)").unwrap();
}

fn as_int(v: &Value) -> Option<i64> {
    match v {
        Value::Integer(i) => Some(*i),
        _ => None,
    }
}

// ============================================================================
// Issue #4689 — LEAD / LAG with offset and default
// ============================================================================

#[test]
fn lag_offset_2_default_0() {
    let mut x = fresh();
    setup_table(&mut x);
    let r = x
        .execute("SELECT id, LAG(val, 2, 0) OVER (ORDER BY id) AS lag2 FROM t ORDER BY id")
        .unwrap();
    assert_eq!(r.rows.len(), 3);
    // id=1: no row 2 back -> default 0
    assert_eq!(as_int(&r.rows[0][1]), Some(0));
    // id=2: no row 2 back -> default 0
    assert_eq!(as_int(&r.rows[1][1]), Some(0));
    // id=3: 2 rows back -> val at id=1 -> 10
    assert_eq!(as_int(&r.rows[2][1]), Some(10));
}

#[test]
fn lead_offset_1_default_0() {
    let mut x = fresh();
    setup_table(&mut x);
    let r = x
        .execute("SELECT id, LEAD(val, 1, 0) OVER (ORDER BY id) AS lead1 FROM t ORDER BY id")
        .unwrap();
    assert_eq!(r.rows.len(), 3);
    assert_eq!(as_int(&r.rows[0][1]), Some(20)); // id=1 -> id=2 val=20
    assert_eq!(as_int(&r.rows[1][1]), Some(30)); // id=2 -> id=3 val=30
    assert_eq!(as_int(&r.rows[2][1]), Some(0)); // id=3 -> default
}

// ============================================================================
// Issue #4707 — FIRST_VALUE / LAST_VALUE / NTH_VALUE
//
// SQL:1999 §6.10: default frame is `RANGE BETWEEN UNBOUNDED PRECEDING AND
// CURRENT ROW`, so FIRST_VALUE over an ORDER BY returns the first row of
// the partition (the smallest sort key), LAST_VALUE returns the current
// row (since the default frame excludes following rows), and NTH_VALUE
// returns the n-th row within the frame.
// ============================================================================

#[test]
fn first_value_default_frame() {
    let mut x = fresh();
    setup_table(&mut x);
    let r = x
        .execute("SELECT id, FIRST_VALUE(val) OVER (ORDER BY id) AS fv FROM t ORDER BY id")
        .unwrap();
    assert_eq!(r.rows.len(), 3);
    // Every row's FIRST_VALUE under default frame = first row of partition (10).
    for row in &r.rows {
        assert_eq!(
            as_int(&row[1]),
            Some(10),
            "FIRST_VALUE should be 10 (first row of partition)"
        );
    }
}

#[test]
fn last_value_default_frame_is_current_row() {
    // SQL:1999 default frame is UNBOUNDED PRECEDING AND CURRENT ROW.
    // LAST_VALUE under that frame returns the CURRENT row (not the last row of
    // the partition). To get the actual last row, callers must specify
    // `ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING`.
    let mut x = fresh();
    setup_table(&mut x);
    let r = x
        .execute("SELECT id, LAST_VALUE(val) OVER (ORDER BY id) AS lv FROM t ORDER BY id")
        .unwrap();
    assert_eq!(r.rows.len(), 3);
    // Per SQL standard, LAST_VALUE with default frame returns the current row.
    assert_eq!(as_int(&r.rows[0][1]), Some(10));
    assert_eq!(as_int(&r.rows[1][1]), Some(20));
    assert_eq!(as_int(&r.rows[2][1]), Some(30));
}

#[test]
fn last_value_unbounded_frame() {
    let mut x = fresh();
    setup_table(&mut x);
    let r = x
        .execute(
            "SELECT id, LAST_VALUE(val) OVER ( \
                 ORDER BY id ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING \
             ) AS lv FROM t ORDER BY id",
        )
        .unwrap();
    assert_eq!(r.rows.len(), 3);
    for row in &r.rows {
        assert_eq!(
            as_int(&row[1]),
            Some(30),
            "LAST_VALUE with full frame should be 30"
        );
    }
}

#[test]
fn nth_value_default_frame_n2() {
    let mut x = fresh();
    setup_table(&mut x);
    let r = x
        .execute("SELECT id, NTH_VALUE(val, 2) OVER (ORDER BY id) AS nv FROM t ORDER BY id")
        .unwrap();
    assert_eq!(r.rows.len(), 3);
    // Default frame is UNBOUNDED PRECEDING AND CURRENT ROW.
    // - row 1 (id=1): frame has 1 row, n=2 out of range -> NULL
    assert_eq!(r.rows[0][1], Value::Null);
    // - row 2 (id=2): frame has 2 rows, n=2 -> 20
    assert_eq!(as_int(&r.rows[1][1]), Some(20));
    // - row 3 (id=3): frame has 3 rows, n=2 -> 20 (still the 2nd row)
    assert_eq!(as_int(&r.rows[2][1]), Some(20));
}

// ============================================================================
// Issue #4706 — Window frame clauses
//
// The parser recognises ROWS / RANGE / GROUPS BETWEEN ... AND ... but the
// executor historically ignored the frame and aggregated over the whole
// partition. These tests verify that the executor honours the frame.
// ============================================================================

#[test]
fn sum_rows_one_preceding_and_current_row() {
    let mut x = fresh();
    setup_table(&mut x);
    let r = x
        .execute(
            "SELECT id, SUM(val) OVER ( \
                 ORDER BY id ROWS BETWEEN 1 PRECEDING AND CURRENT ROW \
             ) AS s FROM t ORDER BY id",
        )
        .unwrap();
    assert_eq!(r.rows.len(), 3);
    // Row 1 (id=1): frame is just row 1 -> 10
    assert_eq!(as_int(&r.rows[0][1]), Some(10));
    // Row 2 (id=2): frame is rows 1,2 -> 30
    assert_eq!(as_int(&r.rows[1][1]), Some(30));
    // Row 3 (id=3): frame is rows 2,3 -> 50
    assert_eq!(as_int(&r.rows[2][1]), Some(50));
}

#[test]
fn sum_rows_unbounded_preceding_following() {
    let mut x = fresh();
    setup_table(&mut x);
    let r = x
        .execute(
            "SELECT id, SUM(val) OVER ( \
                 ORDER BY id ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING \
             ) AS s FROM t ORDER BY id",
        )
        .unwrap();
    assert_eq!(r.rows.len(), 3);
    for row in &r.rows {
        assert_eq!(
            as_int(&row[1]),
            Some(60),
            "full partition sum must equal 60"
        );
    }
}

#[test]
fn sum_rows_unbounded_preceding_current_row() {
    // Running sum: 10, 30, 60
    let mut x = fresh();
    setup_table(&mut x);
    let r = x
        .execute(
            "SELECT id, SUM(val) OVER ( \
                 ORDER BY id ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW \
             ) AS s FROM t ORDER BY id",
        )
        .unwrap();
    assert_eq!(r.rows.len(), 3);
    assert_eq!(as_int(&r.rows[0][1]), Some(10));
    assert_eq!(as_int(&r.rows[1][1]), Some(30));
    assert_eq!(as_int(&r.rows[2][1]), Some(60));
}

#[test]
fn sum_rows_current_row_unbounded_following() {
    // Window grows forward: id=1 -> 60, id=2 -> 50, id=3 -> 30
    let mut x = fresh();
    setup_table(&mut x);
    let r = x
        .execute(
            "SELECT id, SUM(val) OVER ( \
                 ORDER BY id ROWS BETWEEN CURRENT ROW AND UNBOUNDED FOLLOWING \
             ) AS s FROM t ORDER BY id",
        )
        .unwrap();
    assert_eq!(r.rows.len(), 3);
    assert_eq!(as_int(&r.rows[0][1]), Some(60));
    assert_eq!(as_int(&r.rows[1][1]), Some(50));
    assert_eq!(as_int(&r.rows[2][1]), Some(30));
}

#[test]
fn sum_rows_one_preceding_one_following() {
    let mut x = fresh();
    setup_table(&mut x);
    let r = x
        .execute(
            "SELECT id, SUM(val) OVER ( \
                 ORDER BY id ROWS BETWEEN 1 PRECEDING AND 1 FOLLOWING \
             ) AS s FROM t ORDER BY id",
        )
        .unwrap();
    assert_eq!(r.rows.len(), 3);
    // id=1: rows {1,2} -> 10+20 = 30
    assert_eq!(as_int(&r.rows[0][1]), Some(30));
    // id=2: rows {1,2,3} -> 60
    assert_eq!(as_int(&r.rows[1][1]), Some(60));
    // id=3: rows {2,3} -> 20+30 = 50
    assert_eq!(as_int(&r.rows[2][1]), Some(50));
}

#[test]
fn avg_rows_one_preceding_and_current_row() {
    let mut x = fresh();
    setup_table(&mut x);
    let r = x
        .execute(
            "SELECT id, AVG(val) OVER ( \
                 ORDER BY id ROWS BETWEEN 1 PRECEDING AND CURRENT ROW \
             ) AS a FROM t ORDER BY id",
        )
        .unwrap();
    assert_eq!(r.rows.len(), 3);
    match &r.rows[0][1] {
        Value::Float(f) => assert!((*f - 10.0).abs() < 1e-9),
        Value::Integer(i) => assert_eq!(*i, 10),
        _ => panic!("expected numeric avg"),
    }
    match &r.rows[1][1] {
        Value::Float(f) => assert!((*f - 15.0).abs() < 1e-9, "expected avg=15, got {}", f),
        Value::Integer(i) => assert_eq!(*i, 15),
        _ => panic!("expected numeric avg"),
    }
    match &r.rows[2][1] {
        Value::Float(f) => assert!((*f - 25.0).abs() < 1e-9, "expected avg=25, got {}", f),
        Value::Integer(i) => assert_eq!(*i, 25),
        _ => panic!("expected numeric avg"),
    }
}

#[test]
fn count_rows_one_preceding_and_current_row() {
    let mut x = fresh();
    setup_table(&mut x);
    let r = x
        .execute(
            "SELECT id, COUNT(val) OVER ( \
                 ORDER BY id ROWS BETWEEN 1 PRECEDING AND CURRENT ROW \
             ) AS c FROM t ORDER BY id",
        )
        .unwrap();
    assert_eq!(r.rows.len(), 3);
    // id=1: 1 row in frame
    assert_eq!(as_int(&r.rows[0][1]), Some(1));
    // id=2: 2 rows in frame
    assert_eq!(as_int(&r.rows[1][1]), Some(2));
    // id=3: 2 rows in frame
    assert_eq!(as_int(&r.rows[2][1]), Some(2));
}

// ============================================================================
// Issue #4689 / #4706 / #4707 — Partition-aware frames
//
// The previous executor ignored PARTITION BY in some paths; verify that
// the frame is applied WITHIN each partition (not across them).
// ============================================================================

#[test]
fn sum_partition_by_full_partition() {
    // 2 partitions: g=A has rows (1,10),(2,20); g=B has row (3,30).
    // Within each partition, UNBOUNDED PRECEDING AND CURRENT ROW should
    // equal running sum of that partition.
    let mut x = fresh();
    x.execute("CREATE TABLE pt(g text, id int, val int)")
        .unwrap();
    x.execute("INSERT INTO pt VALUES ('A', 1, 10)").unwrap();
    x.execute("INSERT INTO pt VALUES ('A', 2, 20)").unwrap();
    x.execute("INSERT INTO pt VALUES ('B', 3, 30)").unwrap();
    let r = x
        .execute(
            "SELECT g, id, SUM(val) OVER ( \
                 PARTITION BY g ORDER BY id ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW \
             ) AS s FROM pt ORDER BY g, id",
        )
        .unwrap();
    assert_eq!(r.rows.len(), 3);
    assert_eq!(r.rows[0][0], Value::Text("A".into()));
    assert_eq!(as_int(&r.rows[0][2]), Some(10));
    assert_eq!(r.rows[1][0], Value::Text("A".into()));
    assert_eq!(as_int(&r.rows[1][2]), Some(30));
    assert_eq!(r.rows[2][0], Value::Text("B".into()));
    assert_eq!(as_int(&r.rows[2][2]), Some(30));
}

#[test]
fn first_value_partition_by_orders_within_partition() {
    let mut x = fresh();
    x.execute("CREATE TABLE pv(g text, id int, val int)")
        .unwrap();
    x.execute("INSERT INTO pv VALUES ('A', 1, 10)").unwrap();
    x.execute("INSERT INTO pv VALUES ('A', 2, 20)").unwrap();
    x.execute("INSERT INTO pv VALUES ('B', 3, 30)").unwrap();
    x.execute("INSERT INTO pv VALUES ('B', 4, 40)").unwrap();
    let r = x
        .execute(
            "SELECT g, id, FIRST_VALUE(val) OVER ( \
                 PARTITION BY g ORDER BY id \
             ) AS fv FROM pv ORDER BY g, id",
        )
        .unwrap();
    assert_eq!(r.rows.len(), 4);
    // Partition A: every row's FIRST_VALUE = 10
    assert_eq!(as_int(&r.rows[0][2]), Some(10));
    assert_eq!(as_int(&r.rows[1][2]), Some(10));
    // Partition B: every row's FIRST_VALUE = 30
    assert_eq!(as_int(&r.rows[2][2]), Some(30));
    assert_eq!(as_int(&r.rows[3][2]), Some(30));
}
