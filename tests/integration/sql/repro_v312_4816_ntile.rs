//! V312-86 / Issue #4816 — NTILE(n) bucket distribution.
//!
//! SQL:1999 §6.10: NTILE(n) divides an ordered partition into `n` buckets
//! as evenly as possible. Each bucket gets either ⌈total/n⌉ or ⌊total/n⌋
//! rows; larger buckets come first.
//!
//! The previous implementation used `ceil(pos * n / total)` which produces
//! a different distribution. The standard formula is
//! `bucket = floor((pos-1) * n / total) + 1` (clamped to `[1, n]`).
//!
//! Reference example: 7 rows / NTILE(4) → bucket sizes 2,2,2,1
//!   pos=1,2 → bucket 1
//!   pos=3,4 → bucket 2
//!   pos=5,6 → bucket 3
//!   pos=7   → bucket 4
//!
//! Reference example: 10 rows / NTILE(3) → bucket sizes 4,3,3
//!   pos=1..4   → bucket 1
//!   pos=5..7   → bucket 2
//!   pos=8..10  → bucket 3
//!
//! Reference example: 3 rows / NTILE(5) → 3 buckets have 1 row, 2 buckets empty.
//!   pos=1 → bucket 1
//!   pos=2 → bucket 2
//!   pos=3 → bucket 3
//!   (pos=4..5 → no rows, never observed)

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, Value};
use std::sync::Arc;

fn fresh() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

fn as_int(v: &Value) -> Option<i64> {
    match v {
        Value::Integer(i) => Some(*i),
        _ => None,
    }
}

#[test]
fn ntile_4_over_7_rows_distribution() {
    // The anchor case from issue #4816.
    let mut x = fresh();
    x.execute("CREATE TABLE t (id INT)").unwrap();
    for v in 1..=7 {
        x.execute(&format!("INSERT INTO t VALUES ({})", v))
            .unwrap();
    }
    let r = x
        .execute(
            "SELECT id, NTILE(4) OVER (ORDER BY id) AS bucket FROM t ORDER BY id",
        )
        .expect("NTILE(4) OVER (ORDER BY id) must succeed");
    assert_eq!(r.rows.len(), 7);
    let expected = [(1, 1), (2, 1), (3, 2), (4, 2), (5, 3), (6, 3), (7, 4)];
    for (i, (id, bkt)) in expected.iter().enumerate() {
        assert_eq!(as_int(&r.rows[i][0]), Some(*id), "row[{}] id", i);
        assert_eq!(
            as_int(&r.rows[i][1]),
            Some(*bkt),
            "row[{}] bucket (id={}): expected {}, got {:?}",
            i,
            id,
            bkt,
            r.rows[i][1]
        );
    }
}

#[test]
fn ntile_3_over_10_rows_distribution() {
    // 10 rows / 3 buckets → 4,3,3
    let mut x = fresh();
    x.execute("CREATE TABLE t (id INT)").unwrap();
    for v in 1..=10 {
        x.execute(&format!("INSERT INTO t VALUES ({})", v))
            .unwrap();
    }
    let r = x
        .execute(
            "SELECT id, NTILE(3) OVER (ORDER BY id) AS bucket FROM t ORDER BY id",
        )
        .expect("NTILE(3) OVER (ORDER BY id) must succeed");
    assert_eq!(r.rows.len(), 10);
    let expected = [
        (1, 1),
        (2, 1),
        (3, 1),
        (4, 1),
        (5, 2),
        (6, 2),
        (7, 2),
        (8, 3),
        (9, 3),
        (10, 3),
    ];
    for (i, (id, bkt)) in expected.iter().enumerate() {
        assert_eq!(as_int(&r.rows[i][0]), Some(*id), "row[{}] id", i);
        assert_eq!(
            as_int(&r.rows[i][1]),
            Some(*bkt),
            "row[{}] bucket (id={}): expected {}, got {:?}",
            i,
            id,
            bkt,
            r.rows[i][1]
        );
    }
}

#[test]
fn ntile_more_buckets_than_rows() {
    // 3 rows / 5 buckets → buckets 1,2,3 get 1 row each; 4,5 are empty.
    let mut x = fresh();
    x.execute("CREATE TABLE t (id INT)").unwrap();
    for v in 1..=3 {
        x.execute(&format!("INSERT INTO t VALUES ({})", v))
            .unwrap();
    }
    let r = x
        .execute(
            "SELECT id, NTILE(5) OVER (ORDER BY id) AS bucket FROM t ORDER BY id",
        )
        .expect("NTILE(5) over 3 rows must succeed");
    assert_eq!(r.rows.len(), 3);
    assert_eq!(as_int(&r.rows[0][1]), Some(1));
    assert_eq!(as_int(&r.rows[1][1]), Some(2));
    assert_eq!(as_int(&r.rows[2][1]), Some(3));
}

#[test]
fn ntile_default_arg_is_one_bucket() {
    // NTILE() with no args defaults to n=1; every row gets bucket 1.
    let mut x = fresh();
    x.execute("CREATE TABLE t (id INT)").unwrap();
    for v in 1..=4 {
        x.execute(&format!("INSERT INTO t VALUES ({})", v))
            .unwrap();
    }
    let r = x
        .execute(
            "SELECT id, NTILE() OVER (ORDER BY id) AS bucket FROM t ORDER BY id",
        )
        .expect("NTILE() default arg must succeed");
    assert_eq!(r.rows.len(), 4);
    for row in &r.rows {
        assert_eq!(as_int(&row[1]), Some(1));
    }
}

#[test]
fn ntile_one_bucket_is_all_rows() {
    // NTILE(1) — every row in bucket 1.
    let mut x = fresh();
    x.execute("CREATE TABLE t (id INT)").unwrap();
    for v in 1..=5 {
        x.execute(&format!("INSERT INTO t VALUES ({})", v))
            .unwrap();
    }
    let r = x
        .execute(
            "SELECT id, NTILE(1) OVER (ORDER BY id) AS bucket FROM t ORDER BY id",
        )
        .expect("NTILE(1) must succeed");
    assert_eq!(r.rows.len(), 5);
    for row in &r.rows {
        assert_eq!(as_int(&row[1]), Some(1));
    }
}

#[test]
fn ntile_with_partition_by() {
    // NTILE must respect PARTITION BY — each partition's buckets are sized
    // independently of other partitions.
    let mut x = fresh();
    x.execute("CREATE TABLE t(g INT, id INT)").unwrap();
    // Group A: 5 rows → NTILE(2) → 3 rows in bucket 1, 2 in bucket 2
    for i in 1..=5 {
        x.execute(&format!("INSERT INTO t VALUES (1, {})", i))
            .unwrap();
    }
    // Group B: 2 rows → NTILE(2) → 1 row in bucket 1, 1 in bucket 2
    for i in 1..=2 {
        x.execute(&format!("INSERT INTO t VALUES (2, {})", i))
            .unwrap();
    }
    let r = x
        .execute(
            "SELECT g, id, NTILE(2) OVER (PARTITION BY g ORDER BY id) AS bucket \
             FROM t ORDER BY g, id",
        )
        .expect("NTILE with PARTITION BY must succeed");
    assert_eq!(r.rows.len(), 7);
    // Group A (g=1): id 1,2,3 → bucket 1; id 4,5 → bucket 2
    // Group B (g=2): id 1 → bucket 1; id 2 → bucket 2
    let expected = [
        (1, 1, 1),
        (1, 2, 1),
        (1, 3, 1),
        (1, 4, 2),
        (1, 5, 2),
        (2, 1, 1),
        (2, 2, 2),
    ];
    for (i, (g, id, bkt)) in expected.iter().enumerate() {
        assert_eq!(as_int(&r.rows[i][0]), Some(*g), "row[{}] g", i);
        assert_eq!(as_int(&r.rows[i][1]), Some(*id), "row[{}] id", i);
        assert_eq!(
            as_int(&r.rows[i][2]),
            Some(*bkt),
            "row[{}] bucket (g={}, id={}): expected {}, got {:?}",
            i,
            g,
            id,
            bkt,
            r.rows[i][2]
        );
    }
}