//! #5209 regression: an equality predicate must reach the index, and
//! reaching it must not change the answer.
//!
//! # The defect
//!
//! `try_extract_pk_eq_with_col` recognised exactly one WHERE shape: the
//! whole clause being a single `<pk col> = <literal>` equality. Everything
//! else fell through to a full table scan — including `id = 1 AND k = 1`,
//! and any equality on a column carrying a real `CREATE INDEX`.
//!
//! The index was also dead code on the read path: the single call site of
//! `scan_with_index` passed `&Value::Null` as the key, which matches no
//! row.
//!
//! Measured on FileStorage at 10,000 rows, before the fix:
//! `WHERE id = 1` 25 µs (flat), `WHERE id = 1 AND k = 1` 4.23 ms (13x
//! growth from 1,000 rows), `WHERE k = 1` on an indexed column 3.23 ms.
//!
//! # What makes this safe to route
//!
//! `BPlusTree` used to store one `u32` per key, so seeking a non-unique
//! column silently dropped every row but the last inserted under that key.
//! PR #5213 changed the value to `Vec<u32>`. Routing a query through an
//! index is only an optimisation if it returns the same rows the full
//! scan did, so the non-unique case is asserted here explicitly — it is
//! the one that regressed when an earlier version of this routing landed.

use sqlrustgo_storage::FileStorage;
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn engine_with(dir: &Path) -> sqlrustgo::ExecutionEngine<FileStorage> {
    let storage = FileStorage::new_with_buffer_config(dir.to_path_buf(), 1_000_000, true)
        .expect("FileStorage");
    sqlrustgo::ExecutionEngine::<FileStorage>::new(Arc::new(parking_lot::RwLock::new(storage)))
}

fn count(e: &mut sqlrustgo::ExecutionEngine<FileStorage>, sql: &str) -> usize {
    e.execute(sql)
        .unwrap_or_else(|err| panic!("{sql} failed: {err}"))
        .rows
        .len()
}

fn tmpdir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("rt5209_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("mkdir");
    d
}

/// A non-unique indexed column must return **every** matching row.
///
/// This is the assertion that failed when this routing was first written:
/// the index stored one row id per key, so this returned 1 instead of ~142.
#[test]
fn non_unique_index_column_returns_every_matching_row() {
    let dir = tmpdir("nonuniq");
    let mut e = engine_with(&dir);
    e.execute("CREATE TABLE t(id INT PRIMARY KEY, k INT)")
        .expect("create");
    for cs in (1..=1_000).step_by(500) {
        let vals: Vec<String> = (cs..cs + 500)
            .map(|i| {
                let r = i % 7;
                format!("({i},{r})")
            })
            .collect();
        e.execute(&format!("INSERT INTO t VALUES {}", vals.join(",")))
            .expect("insert");
    }
    e.flush().ok();
    e.execute("CREATE INDEX idx_k ON t(k)")
        .expect("create index");

    // i ranges 1..=1000, so residues 1..=6 occur 143 times and residue 0
    // occurs floor(1000/7) = 142 times. Asserting the exact counts is what
    // makes this a regression test: the failure signature of the old
    // single-value-per-key index was a count of exactly 1 for every probe.
    for k in 0..7i64 {
        let expected = if k == 0 { 142 } else { 143 };
        assert_eq!(
            count(&mut e, &format!("SELECT id FROM t WHERE k = {k}")),
            expected as usize,
            "#5209: every row carrying k = {k} must come back through the \
             index; the pre-#5213 index returned only the last row inserted \
             for that key"
        );
    }

    let _ = std::fs::remove_dir_all(&dir);
}

/// The two-column form that motivated the fix.
#[test]
fn multi_column_equality_matches_the_full_scan() {
    let dir = tmpdir("multicol");
    let mut e = engine_with(&dir);
    e.execute("CREATE TABLE t(id INT PRIMARY KEY, k INT)")
        .expect("create");
    for cs in (1..=2_000).step_by(500) {
        let vals: Vec<String> = (cs..cs + 500).map(|i| format!("({i},{i})")).collect();
        e.execute(&format!("INSERT INTO t VALUES {}", vals.join(",")))
            .expect("insert");
    }
    e.flush().ok();

    assert_eq!(
        count(&mut e, "SELECT id FROM t WHERE id = 1 AND k = 1"),
        1,
        "#5209: the two-column predicate must still find its row"
    );
    assert_eq!(
        count(&mut e, "SELECT id FROM t WHERE id = 1 AND k = 999"),
        0,
        "#5209: an index seek on the first conjunct must not drop the rest \
         of the filter — returning a row here would be a correctness bug"
    );
    assert_eq!(count(&mut e, "SELECT id FROM t WHERE id = 1"), 1);

    let _ = std::fs::remove_dir_all(&dir);
}

/// `OR` must not be decomposed. Picking one branch's equality to seek on
/// would return that branch's rows and silently drop the other branch.
#[test]
fn or_predicate_is_not_routed_to_an_index_seek() {
    let dir = tmpdir("or");
    let mut e = engine_with(&dir);
    e.execute("CREATE TABLE t(id INT PRIMARY KEY, k INT)")
        .expect("create");
    for cs in (1..=500).step_by(250) {
        let vals: Vec<String> = (cs..cs + 250).map(|i| format!("({i},{i})")).collect();
        e.execute(&format!("INSERT INTO t VALUES {}", vals.join(",")))
            .expect("insert");
    }
    e.flush().ok();

    // `id = 1 OR id = 400` must return both rows. If the router picked the
    // first equality and seeked, this would return 1.
    assert_eq!(
        count(&mut e, "SELECT id FROM t WHERE id = 1 OR id = 400"),
        2,
        "#5209: an OR chain must keep full-scan semantics"
    );
    assert_eq!(
        count(&mut e, "SELECT id FROM t WHERE id = 1 OR id = 9999"),
        1
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// No index for the column: the query must still return the right answer,
/// just by the ordinary scan.
#[test]
fn unindexed_column_still_returns_correct_rows() {
    let dir = tmpdir("noidx");
    let mut e = engine_with(&dir);
    e.execute("CREATE TABLE t(id INT PRIMARY KEY, k INT)")
        .expect("create");
    for cs in (1..=500).step_by(250) {
        let vals: Vec<String> = (cs..cs + 250).map(|i| format!("({i},{i})")).collect();
        e.execute(&format!("INSERT INTO t VALUES {}", vals.join(",")))
            .expect("insert");
    }
    e.flush().ok();

    assert_eq!(count(&mut e, "SELECT id FROM t WHERE k = 7"), 1);
    assert_eq!(count(&mut e, "SELECT id FROM t WHERE k = 9999"), 0);

    let _ = std::fs::remove_dir_all(&dir);
}

/// A non-equality predicate must not be routed either — `>` has no single
/// key to seek on.
#[test]
fn range_predicate_is_not_routed_to_an_index_seek() {
    let dir = tmpdir("range");
    let mut e = engine_with(&dir);
    e.execute("CREATE TABLE t(id INT PRIMARY KEY, k INT)")
        .expect("create");
    for cs in (1..=1_000).step_by(500) {
        let vals: Vec<String> = (cs..cs + 500).map(|i| format!("({i},{i})")).collect();
        e.execute(&format!("INSERT INTO t VALUES {}", vals.join(",")))
            .expect("insert");
    }
    e.flush().ok();
    e.execute("CREATE INDEX idx_k ON t(k)")
        .expect("create index");

    assert_eq!(count(&mut e, "SELECT id FROM t WHERE k > 900"), 100);
    assert_eq!(count(&mut e, "SELECT id FROM t WHERE k < 100"), 99);

    let _ = std::fs::remove_dir_all(&dir);
}
