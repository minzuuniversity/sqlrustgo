//! #5117: a multi-table `UPDATE` must actually update.
//!
//! # The defect
//!
//! `execute_update_multi_table` decided which tables a statement touched
//! with
//!
//! ```text
//! let table_has_update = resolved_set
//!     .iter()
//     .any(|(col, _)| col.starts_with(&format!("{}.", table_prefix)));
//! if !table_has_update { continue; }
//! ```
//!
//! A bare column name — `UPDATE t1, t2 SET k = 99`, which is what MySQL
//! means by "set k on both" — starts with neither `t1.` nor `t2.`, so
//! **every table was skipped** and the statement changed nothing. But
//! `total_count += 1` still ran, so it returned `affected_rows = 1`.
//!
//! A silent no-op that reports success is worse than an error: it is
//! indistinguishable, to the caller, from a WHERE clause that matched
//! nothing — a normal outcome that needs no investigation.
//!
//! A second defect sat beside it: the SET loop matched the combined schema
//! with `name.ends_with(".<col>")`, which finds the FIRST match only. So
//! even with qualified names (`SET t1.k = …, t2.k = …`) both writes landed
//! in `t1.k`'s slot.
//!
//! Measured on the baseline (`8bc4aac10c9`), single database, no
//! transactions:
//!
//! ```text
//! UPDATE t1 SET k = 55                -> affected=1   t1.k becomes 55   OK
//! UPDATE t1, t2 SET k = 99            -> affected=1   nothing changes  WRONG
//! UPDATE t1, t2 SET k = 77 WHERE id=1 -> affected=1   nothing changes  WRONG
//! DELETE t1, t2 FROM t1, t2           -> affected=2   both emptied     OK
//! ```

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::{FileStorage, StorageEngine};
use sqlrustgo_types::Value;
use std::sync::Arc;

fn engine() -> ExecutionEngine<FileStorage> {
    let dir = std::env::temp_dir().join(format!("mt_update_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    ExecutionEngine::new(Arc::new(RwLock::new(FileStorage::new(dir).unwrap())))
}

/// `t1` and `t2`, both with `(id, k)`, one row each.
fn seed(x: &mut ExecutionEngine<FileStorage>) {
    x.execute("CREATE TABLE t1 (id INT PRIMARY KEY, k INT)")
        .unwrap();
    x.execute("CREATE TABLE t2 (id INT PRIMARY KEY, k INT)")
        .unwrap();
    x.execute("INSERT INTO t1 VALUES (1, 10)").unwrap();
    x.execute("INSERT INTO t2 VALUES (1, 20)").unwrap();
}

fn k(x: &mut ExecutionEngine<FileStorage>, table: &str) -> i64 {
    let rows = x.execute(&format!("SELECT k FROM {table}")).unwrap().rows;
    match &rows[0][0] {
        Value::Integer(n) => *n,
        other => panic!("{table}: expected integer, got {other:?}"),
    }
}

/// The bare-name form, which used to match neither table.
#[test]
fn bare_column_name_updates_every_table() {
    let mut x = engine();
    seed(&mut x);

    x.execute("UPDATE t1, t2 SET k = 99").unwrap();

    assert_eq!(k(&mut x, "t1"), 99, "t1 was not updated");
    assert_eq!(k(&mut x, "t2"), 99, "t2 was not updated by a bare SET");
}

/// Same shape with a WHERE clause, which reported the same false success.
#[test]
fn bare_column_name_with_where_updates_every_table() {
    let mut x = engine();
    seed(&mut x);

    x.execute("UPDATE t1, t2 SET k = 77 WHERE id = 1").unwrap();

    assert_eq!(k(&mut x, "t1"), 77, "t1 was not updated");
    assert_eq!(
        k(&mut x, "t2"),
        77,
        "t2 was not updated with a WHERE clause"
    );
}

/// Qualified names must reach DIFFERENT columns.
///
/// This is the `ends_with` defect: both writes resolved to the first
/// matching slot, so `t2.k` was written into `t1.k`.
#[test]
fn qualified_names_update_their_own_tables() {
    let mut x = engine();
    seed(&mut x);

    x.execute("UPDATE t1, t2 SET t1.k = 1, t2.k = 2").unwrap();

    assert_eq!(k(&mut x, "t1"), 1, "t1.k did not get its own value");
    assert_eq!(k(&mut x, "t2"), 2, "t2.k did not get its own value");
}

/// A qualified SET still targets only the table it names (V312-84).
#[test]
fn qualified_name_leaves_the_other_table_alone() {
    let mut x = engine();
    seed(&mut x);

    x.execute("UPDATE t1, t2 SET t1.k = 5").unwrap();

    assert_eq!(k(&mut x, "t1"), 5, "t1 was not updated");
    assert_eq!(k(&mut x, "t2"), 20, "a t1-qualified SET reached into t2");
}

/// Aliases are the other way a prefix appears.
#[test]
fn aliases_are_resolved_to_their_tables() {
    let mut x = engine();
    seed(&mut x);

    x.execute("UPDATE t1 AS a, t2 AS b SET a.k = 7, b.k = 8")
        .unwrap();

    assert_eq!(k(&mut x, "t1"), 7, "alias `a` did not reach t1");
    assert_eq!(k(&mut x, "t2"), 8, "alias `b` did not reach t2");
}

/// Control group: a WHERE that matches nothing must report nothing.
///
/// The whole harm of the original defect was that "changed nothing" and
/// "reported 1" arrived together. This pins the other direction: when
/// genuinely nothing matches, the count must be 0 — otherwise fixing the
/// routing would leave a caller unable to tell success from no-op.
#[test]
fn a_where_that_matches_nothing_reports_zero() {
    let mut x = engine();
    seed(&mut x);

    let r = x
        .execute("UPDATE t1, t2 SET k = 9 WHERE id = 4242")
        .unwrap();

    assert_eq!(
        r.affected_rows, 0,
        "a statement that matched nothing must not report affected rows"
    );
    assert_eq!(k(&mut x, "t1"), 10, "t1 changed despite matching nothing");
    assert_eq!(k(&mut x, "t2"), 20, "t2 changed despite matching nothing");
}

/// Control group: single-table UPDATE keeps working.
#[test]
fn single_table_update_still_works() {
    let mut x = engine();
    seed(&mut x);

    x.execute("UPDATE t1 SET k = 55").unwrap();
    assert_eq!(k(&mut x, "t1"), 55);
    assert_eq!(k(&mut x, "t2"), 20);
}
