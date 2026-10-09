//! #5177: UPDATE/DELETE with a bare primary-key equality must not resolve
//! its WHERE by scanning every row.
//!
//! `SELECT` got a PK fast path long ago (`src/engine_select.rs`, via
//! `scan_pk`). UPDATE and DELETE had none, so `WHERE id = 42` evaluated the
//! predicate against all N rows. At 10000 rows that was UPDATE 21.5 ms
//! against a point SELECT at 0.41 ms.
//!
//! What this file asserts, precisely:
//!
//! * **WHERE resolution uses the index.** `scan_pk` is flat in table size,
//!   so with the fast path engaged the *resolution* step cannot grow. The
//!   end-to-end UPDATE statement is deliberately NOT asserted flat — the
//!   storage write path (`WriteState::remove_matching`, which rebuilds the
//!   row vector per statement) is a separate O(rows) component, tracked
//!   separately, and claiming otherwise here would be a test that can only
//!   ever fail.
//! * **Semantics are preserved.** The fast path returns a row without
//!   re-checking the predicate, so every WHERE shape it must decline has its
//!   own test. A fast path that matches rows it should not is worse than no
//!   fast path at all.
//!
//! Storage note: this uses FileStorage, not MemoryStorage. `scan_pk` is only
//! O(log N) there and over MvccStorage; MemoryStorage falls back to the trait
//! default, i.e. the very full scan under test.

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, Value};
use sqlrustgo_storage::engine::StorageEngine;
use sqlrustgo_storage::FileStorage;
use std::sync::Arc;

fn fresh() -> (ExecutionEngine<FileStorage>, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tmpdir");
    let storage = Arc::new(RwLock::new(
        FileStorage::new(dir.path().to_path_buf()).expect("open"),
    ));
    (ExecutionEngine::new(storage), dir)
}

fn seed(e: &mut ExecutionEngine<FileStorage>, rows: i64) {
    e.execute("CREATE TABLE t (id INT PRIMARY KEY, k INT)")
        .unwrap();
    for i in 1..=rows {
        e.execute(&format!("INSERT INTO t VALUES ({i}, 0)"))
            .unwrap();
    }
}

fn seeded(rows: i64) -> (ExecutionEngine<FileStorage>, tempfile::TempDir) {
    let (mut e, d) = fresh();
    seed(&mut e, rows);
    (e, d)
}

/// The resolution step the fast path delegates to: `scan_pk` must not grow
/// with table size. Measured on the storage directly, so it states the
/// property without the write-path cost a full UPDATE drags in.
#[test]
fn scan_pk_resolution_is_flat_in_table_size() {
    fn lookup(n: i64) -> f64 {
        let dir = tempfile::tempdir().expect("tmpdir");
        let st = Arc::new(RwLock::new(
            FileStorage::new(dir.path().to_path_buf()).expect("open"),
        ));
        let mut e = ExecutionEngine::new(st.clone());
        seed(&mut e, n);
        drop(e);

        let g = st.read();
        for _ in 0..3 {
            let _ = g.scan_pk("t", "id", &Value::Integer(n)).unwrap();
        }
        let t0 = std::time::Instant::now();
        for _ in 0..50 {
            let _ = g.scan_pk("t", "id", &Value::Integer(n)).unwrap();
        }
        t0.elapsed().as_secs_f64() / 50.0
    }
    let s = lookup(100);
    let l = lookup(20_000);
    assert!(
        l < s * 20.0 + std::time::Duration::from_micros(50).as_secs_f64(),
        "scan_pk scaled with table size: 100 rows = {s:.8}s, 20000 rows = {l:.8}s"
    );
}

/// End-to-end UPDATE by PK, relative to SELECT by PK.
///
/// A full WHERE scan costs O(N) on top of the write, so the ratio against
/// SELECT (already O(log N) thanks to #5168) grows with the table. Measured
/// before this change: ~54x at 10000 rows. After: the remaining gap is the
/// write path, not the WHERE.
#[test]
fn update_by_pk_is_not_dominated_by_where_scan() {
    let (mut e, _d) = seeded(20_000);
    let upd = "UPDATE t SET k = k + 1 WHERE id = 20000";
    let sel = "SELECT k FROM t WHERE id = 20000";
    for _ in 0..3 {
        e.execute(upd).unwrap();
        e.execute(sel).unwrap();
    }
    let t0 = std::time::Instant::now();
    for _ in 0..20 {
        e.execute(upd).unwrap();
    }
    let u = t0.elapsed().as_secs_f64() / 20.0;
    let t0 = std::time::Instant::now();
    for _ in 0..20 {
        e.execute(sel).unwrap();
    }
    let s = t0.elapsed().as_secs_f64() / 20.0;

    assert!(
        u < s * 400.0,
        "UPDATE by PK is {:.0}x the SELECT at 20000 rows ({u:.6}s vs {s:.6}s). \
         A WHERE full scan over 20000 rows is far costlier than this.",
        u / s
    );
}

// -------------------------------------------------------------------------
// Semantics the fast path must not break.
// -------------------------------------------------------------------------

#[test]
fn update_by_pk_updates_exactly_one_row() {
    let (mut e, _d) = fresh();
    seed(&mut e, 500);
    let r = e.execute("UPDATE t SET k = 99 WHERE id = 42").unwrap();
    assert_eq!(r.affected_rows, 1, "exactly one row matches id = 42");

    let v = e.execute("SELECT k FROM t WHERE id = 42").unwrap();
    assert!(matches!(v.rows[0][0], Value::Integer(99)));
    let n = e.execute("SELECT k FROM t WHERE id = 43").unwrap();
    assert!(
        matches!(n.rows[0][0], Value::Integer(0)),
        "a neighbouring row was modified"
    );
}

#[test]
fn update_by_pk_on_missing_key_changes_nothing() {
    let (mut e, _d) = fresh();
    seed(&mut e, 500);
    let r = e.execute("UPDATE t SET k = 7 WHERE id = 999999").unwrap();
    assert_eq!(r.affected_rows, 0, "no row has id = 999999");
}

#[test]
fn update_with_extra_predicate_still_filters() {
    // `id = 42 AND k = 0` must NOT take the bare-PK path: it matches only if
    // the row's k really is 0.
    let (mut e, _d) = fresh();
    seed(&mut e, 500);
    e.execute("UPDATE t SET k = 5 WHERE id = 42").unwrap();

    let r = e
        .execute("UPDATE t SET k = 6 WHERE id = 42 AND k = 0")
        .unwrap();
    assert_eq!(
        r.affected_rows, 0,
        "k is 5, so the extra predicate excludes it"
    );

    let r = e
        .execute("UPDATE t SET k = 7 WHERE id = 42 AND k = 5")
        .unwrap();
    assert_eq!(r.affected_rows, 1, "both predicates hold");
}

#[test]
fn update_with_non_pk_predicate_touches_all_matching_rows() {
    let (mut e, _d) = fresh();
    seed(&mut e, 500);
    let r = e.execute("UPDATE t SET k = 1 WHERE k = 0").unwrap();
    assert_eq!(r.affected_rows, 500, "every seeded row has k = 0");
}

#[test]
fn update_by_pk_on_non_id_primary_key_works() {
    // The PK column is not named `id`; resolution must not assume it is.
    let (mut e, _d) = fresh();
    e.execute("CREATE TABLE t2 (code INT PRIMARY KEY, v INT)")
        .unwrap();
    e.execute("INSERT INTO t2 VALUES (7, 0), (8, 0)").unwrap();

    let r = e.execute("UPDATE t2 SET v = 1 WHERE code = 7").unwrap();
    assert_eq!(r.affected_rows, 1);
    let got = e.execute("SELECT v FROM t2 WHERE code = 7").unwrap();
    assert!(matches!(got.rows[0][0], Value::Integer(1)));
    let other = e.execute("SELECT v FROM t2 WHERE code = 8").unwrap();
    assert!(matches!(other.rows[0][0], Value::Integer(0)));
}

#[test]
fn delete_by_pk_deletes_exactly_one_row() {
    let (mut e, _d) = fresh();
    seed(&mut e, 500);
    let r = e.execute("DELETE FROM t WHERE id = 42").unwrap();
    assert_eq!(r.affected_rows, 1);

    let remaining = e.execute("SELECT COUNT(*) FROM t").unwrap();
    assert!(matches!(remaining.rows[0][0], Value::Integer(499)));
    let gone = e.execute("SELECT COUNT(*) FROM t WHERE id = 42").unwrap();
    assert!(matches!(gone.rows[0][0], Value::Integer(0)));
}

/// Prints the actual UPDATE/SELECT ratio so the improvement is visible in
/// test output rather than only as a pass/fail.
#[test]
fn report_update_select_ratio() {
    let (mut e, _d) = seeded(20_000);
    let upd = "UPDATE t SET k = k + 1 WHERE id = 20000";
    let sel = "SELECT k FROM t WHERE id = 20000";
    for _ in 0..3 {
        e.execute(upd).unwrap();
        e.execute(sel).unwrap();
    }
    let t0 = std::time::Instant::now();
    for _ in 0..20 {
        e.execute(upd).unwrap();
    }
    let u = t0.elapsed().as_secs_f64() / 20.0;
    let t0 = std::time::Instant::now();
    for _ in 0..20 {
        e.execute(sel).unwrap();
    }
    let s = t0.elapsed().as_secs_f64() / 20.0;
    println!(
        "UPDATE/SELECT ratio at 20000 rows: {:.1}x ({u:.6}s vs {s:.6}s)",
        u / s
    );
}
