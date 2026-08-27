//! Regression tests for Issue #4517 — window functions `OVER (PARTITION BY)`
//! end-to-end through the projection path.
//!
//! MySQL 8 / PostgreSQL window functions let you compute a per-row aggregate
//! or ordinal value over a partition of the input rows. Before this fix,
//! `src/engine_select.rs` projected every column via `evaluate_expression_with_seq`,
//! which has no `Expression::WindowCall` arm — so `avg(x) OVER (PARTITION BY k)`
//! returned `NULL` for every row, even though the parser already produced a
//! valid `WindowCall` AST node.
//!
//! Coverage:
//! * `avg(x) OVER (PARTITION BY k)` — distinct per-partition averages
//! * `sum(x) OVER (PARTITION BY k)` — distinct per-partition sums
//! * `count(x) / count(*) OVER (PARTITION BY k)` — per-partition row counts
//! * `min(x) / max(x) OVER (PARTITION BY k)` — per-partition extremes
//! * `ROW_NUMBER() OVER (PARTITION BY k ORDER BY x)` — ordinals reset per partition
//! * `RANK() OVER (ORDER BY x)` — peer ties share rank, gaps follow
//! * `DENSE_RANK() OVER (ORDER BY x)` — peer ties share rank, no gaps
//! * Multiple `PARTITION BY` columns — partition by composite key
//! * Empty PARTITION BY clause — single global window
//! * `OVER (ORDER BY x)` (no PARTITION BY) — single window ordered

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::MemoryStorage;
use sqlrustgo_storage::Value;
use std::sync::Arc;

fn engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

fn float_value(v: &Value) -> Option<f64> {
    match v {
        Value::Float(f) => Some(*f),
        Value::Integer(n) => Some(*n as f64),
        _ => None,
    }
}

fn int_value(v: &Value) -> Option<i64> {
    match v {
        Value::Integer(n) => Some(*n),
        _ => None,
    }
}

fn row_at(rows: &[Vec<Value>], idx: usize) -> &[Value] {
    &rows[idx]
}

#[test]
fn avg_over_partition_by_returns_per_partition_average() {
    // 4 rows, 2 partitions of 2 rows each:
    //   c1: (90 + 88) / 2 = 89
    //   c2: (92 + 91) / 2 = 91.5
    let mut e = engine();
    e.execute("CREATE TABLE score (studentno TEXT, courseno TEXT, final INTEGER)")
        .unwrap();
    e.execute(
        "INSERT INTO score VALUES ('a', 'c1', 90), ('b', 'c1', 88), ('c', 'c2', 92), ('d', 'c2', 91)",
    )
    .unwrap();

    let r = e
        .execute("SELECT studentno, courseno, avg(final) OVER (PARTITION BY courseno) FROM score")
        .unwrap();

    assert_eq!(r.rows.len(), 4);

    // Group rows by courseno column (index 1) — result is in storage order
    // unless ORDER BY is applied.
    let mut by_course: std::collections::HashMap<String, Vec<&Vec<Value>>> =
        std::collections::HashMap::new();
    for row in &r.rows {
        let course = match &row[1] {
            Value::Text(s) => s.clone(),
            _ => panic!("courseno must be TEXT, got {:?}", row[1]),
        };
        by_course.entry(course).or_default().push(row);
    }

    let c1_avg = float_value(&by_course["c1"][0][2]).expect("avg is float");
    let c2_avg = float_value(&by_course["c2"][0][2]).expect("avg is float");
    assert!((c1_avg - 89.0).abs() < 1e-9, "c1 avg = {c1_avg}, want 89.0");
    assert!((c2_avg - 91.5).abs() < 1e-9, "c2 avg = {c2_avg}, want 91.5");
}

#[test]
fn sum_over_partition_by_returns_per_partition_sum() {
    // c1: 90 + 88 = 178; c2: 92 + 91 = 183.
    let mut e = engine();
    e.execute("CREATE TABLE t (g TEXT, v INTEGER)").unwrap();
    e.execute("INSERT INTO t VALUES ('a', 10), ('a', 20), ('b', 5), ('b', 7), ('b', 3)")
        .unwrap();

    let r = e
        .execute("SELECT g, sum(v) OVER (PARTITION BY g) FROM t")
        .unwrap();
    assert_eq!(r.rows.len(), 5);

    // Per-partition sums: 'a' = 30, 'b' = 15.
    let mut sums: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    for row in &r.rows {
        let g = match &row[0] {
            Value::Text(s) => s.clone(),
            _ => panic!(),
        };
        let s = int_value(&row[1]).expect("sum is integer");
        sums.insert(g, s);
    }
    assert_eq!(sums.get("a").copied(), Some(30));
    assert_eq!(sums.get("b").copied(), Some(15));
}

#[test]
fn count_over_partition_by_returns_row_count() {
    let mut e = engine();
    e.execute("CREATE TABLE t (g TEXT, v INTEGER)").unwrap();
    e.execute("INSERT INTO t VALUES ('a', 1), ('a', 2), ('a', 3), ('b', 10), ('b', 20)")
        .unwrap();

    let r = e
        .execute("SELECT g, count(*) OVER (PARTITION BY g) FROM t")
        .unwrap();
    assert_eq!(r.rows.len(), 5);

    let mut counts: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    for row in &r.rows {
        let g = match &row[0] {
            Value::Text(s) => s.clone(),
            _ => panic!(),
        };
        let c = int_value(&row[1]).expect("count is integer");
        counts.insert(g, c);
    }
    assert_eq!(counts.get("a").copied(), Some(3));
    assert_eq!(counts.get("b").copied(), Some(2));
}

#[test]
fn min_max_over_partition_by_returns_extremes() {
    let mut e = engine();
    e.execute("CREATE TABLE t (g TEXT, v INTEGER)").unwrap();
    e.execute("INSERT INTO t VALUES ('a', 5), ('a', 1), ('a', 9), ('b', 7), ('b', 3)")
        .unwrap();

    // Two window calls in one query — both must be precomputed.
    let r = e
        .execute("SELECT g, min(v) OVER (PARTITION BY g), max(v) OVER (PARTITION BY g) FROM t")
        .unwrap();
    assert_eq!(r.rows.len(), 5);

    let mut mins: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    let mut maxs: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    for row in &r.rows {
        let g = match &row[0] {
            Value::Text(s) => s.clone(),
            _ => panic!(),
        };
        let mn = int_value(&row[1]).unwrap();
        let mx = int_value(&row[2]).unwrap();
        mins.insert(g.clone(), mn);
        maxs.insert(g, mx);
    }
    assert_eq!(mins.get("a").copied(), Some(1));
    assert_eq!(mins.get("b").copied(), Some(3));
    assert_eq!(maxs.get("a").copied(), Some(9));
    assert_eq!(maxs.get("b").copied(), Some(7));
}

#[test]
fn row_number_over_partition_by_resets_per_partition() {
    let mut e = engine();
    e.execute("CREATE TABLE t (g TEXT, v INTEGER)").unwrap();
    e.execute("INSERT INTO t VALUES ('a', 3), ('a', 1), ('a', 2), ('b', 9), ('b', 7)")
        .unwrap();

    let r = e
        .execute("SELECT g, v, ROW_NUMBER() OVER (PARTITION BY g ORDER BY v) FROM t")
        .unwrap();
    assert_eq!(r.rows.len(), 5);

    // Each partition should see row numbers 1..n (n = partition size).
    let mut seen: std::collections::HashMap<String, Vec<i64>> = std::collections::HashMap::new();
    for row in &r.rows {
        let g = match &row[0] {
            Value::Text(s) => s.clone(),
            _ => panic!(),
        };
        let rn = int_value(&row[2]).expect("row_number");
        seen.entry(g).or_default().push(rn);
    }
    let mut a = seen.remove("a").unwrap();
    a.sort();
    assert_eq!(a, vec![1, 2, 3]);
    let mut b = seen.remove("b").unwrap();
    b.sort();
    assert_eq!(b, vec![1, 2]);
}

#[test]
fn rank_over_order_by_with_peers_shares_rank_and_gaps() {
    // Standard SQL RANK semantics: peer rows share rank; a gap follows a tie.
    //   (10, 20, 20, 20, 30) → ranks (1, 2, 2, 2, 5)
    // The three 20s share rank 2; the next distinct key (30) is rank
    // 5 because the engine counts rows strictly less (4 of them), not
    // distinct group transitions. The gap (no rank 3 or 4) is the
    // defining feature that distinguishes RANK from DENSE_RANK.
    let mut e = engine();
    e.execute("CREATE TABLE t (v INTEGER)").unwrap();
    e.execute("INSERT INTO t VALUES (10), (20), (20), (20), (30)")
        .unwrap();

    let r = e
        .execute("SELECT v, RANK() OVER (ORDER BY v) FROM t")
        .unwrap();
    assert_eq!(r.rows.len(), 5);

    let pairs: Vec<(i64, i64)> = r
        .rows
        .iter()
        .map(|row| (int_value(&row[0]).unwrap(), int_value(&row[1]).unwrap()))
        .collect();
    assert_eq!(pairs, vec![(10, 1), (20, 2), (20, 2), (20, 2), (30, 5)]);
}

#[test]
fn dense_rank_over_order_by_with_peers_no_gaps() {
    // Standard SQL DENSE_RANK semantics: peer rows share rank; the rank
    // after a tie increments by 1 (no gap). (10, 20, 20, 20, 30) →
    // ranks (1, 2, 2, 2, 3) — note 30 is rank 3, not rank 5 (the gap
    // that would appear under RANK is the defining distinction).
    let mut e = engine();
    e.execute("CREATE TABLE t (v INTEGER)").unwrap();
    e.execute("INSERT INTO t VALUES (10), (20), (20), (20), (30)")
        .unwrap();

    let r = e
        .execute("SELECT v, DENSE_RANK() OVER (ORDER BY v) FROM t")
        .unwrap();
    assert_eq!(r.rows.len(), 5);

    let pairs: Vec<(i64, i64)> = r
        .rows
        .iter()
        .map(|row| {
            let v = int_value(&row[0]).unwrap();
            let rk = int_value(&row[1]).unwrap();
            (v, rk)
        })
        .collect();
    assert_eq!(
        pairs,
        vec![(10, 1), (20, 2), (20, 2), (20, 2), (30, 3)],
        "DENSE_RANK must close the gap after a tie (no 1→4 jump)"
    );
}

#[test]
fn multiple_partition_columns_partition_by_composite_key() {
    // Two partition columns: (g1, g2) → partition key.
    let mut e = engine();
    e.execute("CREATE TABLE t (g1 TEXT, g2 TEXT, v INTEGER)")
        .unwrap();
    e.execute(
        "INSERT INTO t VALUES ('x', 'p', 1), ('x', 'p', 2), ('x', 'q', 3), ('y', 'p', 10), ('y', 'p', 20)",
    )
    .unwrap();

    let r = e
        .execute("SELECT g1, g2, sum(v) OVER (PARTITION BY g1, g2) FROM t")
        .unwrap();
    assert_eq!(r.rows.len(), 5);

    // Composite-key sums:
    //   (x, p) = 1+2 = 3
    //   (x, q) = 3
    //   (y, p) = 30
    let mut sums: std::collections::HashMap<(String, String), i64> =
        std::collections::HashMap::new();
    for row in &r.rows {
        let g1 = match &row[0] {
            Value::Text(s) => s.clone(),
            _ => panic!(),
        };
        let g2 = match &row[1] {
            Value::Text(s) => s.clone(),
            _ => panic!(),
        };
        let s = int_value(&row[2]).unwrap();
        sums.insert((g1, g2), s);
    }
    assert_eq!(sums.get(&("x".into(), "p".into())).copied(), Some(3));
    assert_eq!(sums.get(&("x".into(), "q".into())).copied(), Some(3));
    assert_eq!(sums.get(&("y".into(), "p".into())).copied(), Some(30));
}

#[test]
fn empty_partition_by_uses_global_window() {
    // No PARTITION BY → single global window.
    let mut e = engine();
    e.execute("CREATE TABLE t (v INTEGER)").unwrap();
    e.execute("INSERT INTO t VALUES (1), (2), (3), (4)")
        .unwrap();

    let r = e.execute("SELECT v, sum(v) OVER () FROM t").unwrap();
    assert_eq!(r.rows.len(), 4);

    for row in &r.rows {
        let s = int_value(&row[1]).unwrap();
        assert_eq!(s, 10, "global sum must be 1+2+3+4 = 10");
    }
}

#[test]
fn over_order_by_without_partition_by_uses_global_ordered_window() {
    let mut e = engine();
    e.execute("CREATE TABLE t (v INTEGER)").unwrap();
    e.execute("INSERT INTO t VALUES (30), (10), (20)").unwrap();

    let r = e
        .execute("SELECT v, ROW_NUMBER() OVER (ORDER BY v) FROM t")
        .unwrap();
    assert_eq!(r.rows.len(), 3);

    // Result rows preserve the input (insertion) order: the window's
    // ordering affects only the ordinal value, not the row order of the
    // projection (no outer ORDER BY clause).
    let pairs: Vec<(i64, i64)> = r
        .rows
        .iter()
        .map(|row| (int_value(&row[0]).unwrap(), int_value(&row[1]).unwrap()))
        .collect();

    // Sort by the ROW_NUMBER to verify the global ordering was applied
    // internally — row numbered 1 should be (10), row numbered 2 should
    // be (20), row numbered 3 should be (30).
    let mut by_rank = pairs.clone();
    by_rank.sort_by_key(|(_, rk)| *rk);
    assert_eq!(by_rank, vec![(10, 1), (20, 2), (30, 3)]);
}

#[test]
fn combined_partition_and_order_by_dense_rank() {
    // Two partitions, both with peer ties — DENSE_RANK resets per partition.
    let mut e = engine();
    e.execute("CREATE TABLE t (g TEXT, v INTEGER)").unwrap();
    e.execute("INSERT INTO t VALUES ('a', 10), ('a', 10), ('a', 20), ('b', 5), ('b', 5), ('b', 5)")
        .unwrap();

    let r = e
        .execute("SELECT g, v, DENSE_RANK() OVER (PARTITION BY g ORDER BY v) FROM t")
        .unwrap();
    assert_eq!(r.rows.len(), 6);

    let mut by_g: std::collections::HashMap<String, Vec<(i64, i64)>> =
        std::collections::HashMap::new();
    for row in &r.rows {
        let g = match &row[0] {
            Value::Text(s) => s.clone(),
            _ => panic!(),
        };
        let v = int_value(&row[1]).unwrap();
        let rk = int_value(&row[2]).unwrap();
        by_g.entry(g).or_default().push((v, rk));
    }

    // Group 'a': two (10) with rank 1, then (20) with rank 2.
    // Group 'b': three (5) with rank 1.
    let mut a = by_g.remove("a").unwrap();
    a.sort_by_key(|(v, _)| *v);
    assert_eq!(a, vec![(10, 1), (10, 1), (20, 2)]);

    let mut b = by_g.remove("b").unwrap();
    b.sort_by_key(|(v, _)| *v);
    assert_eq!(b, vec![(5, 1), (5, 1), (5, 1)]);
}

#[test]
fn single_row_partition_returns_aggregate() {
    // Edge case: a partition with one row returns the row's own value
    // for SUM/AVG/MIN/MAX — common in window-function correctness tests.
    let mut e = engine();
    e.execute("CREATE TABLE t (g TEXT, v INTEGER)").unwrap();
    e.execute("INSERT INTO t VALUES ('a', 42), ('b', 100), ('b', 200)")
        .unwrap();

    let r = e
        .execute("SELECT g, v, avg(v) OVER (PARTITION BY g) FROM t")
        .unwrap();

    // 'a' partition has one row → avg = 42; 'b' partition has two rows → avg = 150.
    let mut by_g: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
    for row in &r.rows {
        let g = match &row[0] {
            Value::Text(s) => s.clone(),
            _ => panic!(),
        };
        let avg = float_value(&row[2]).unwrap();
        by_g.insert(g, avg);
    }
    assert_eq!(by_g.get("a").copied(), Some(42.0));
    assert_eq!(by_g.get("b").copied(), Some(150.0));
}

#[test]
fn non_window_columns_coexist_with_window_columns() {
    // The row-projection pre-computation only fires for WindowCall columns;
    // other columns must still be evaluated per-row via the existing path.
    let mut e = engine();
    e.execute("CREATE TABLE t (g TEXT, v INTEGER)").unwrap();
    e.execute("INSERT INTO t VALUES ('a', 5), ('b', 10), ('b', 20)")
        .unwrap();

    let r = e
        .execute("SELECT g, v, v + 1 AS v_plus_one, sum(v) OVER (PARTITION BY g) FROM t")
        .unwrap();
    assert_eq!(r.rows.len(), 3);

    for row in &r.rows {
        let v = int_value(&row[1]).unwrap();
        let v_plus_one = int_value(&row[2]).unwrap();
        let s = int_value(&row[3]).unwrap();
        assert_eq!(
            v_plus_one,
            v + 1,
            "non-window arithmetic column must still evaluate per-row"
        );
        let g = match &row[0] {
            Value::Text(s) => s.clone(),
            _ => panic!(),
        };
        let expected_sum = match g.as_str() {
            "a" => 5,
            "b" => 30,
            _ => panic!("unexpected g"),
        };
        assert_eq!(s, expected_sum);
    }
    let _ = row_at;
}
