//! Regression tests for Issue #4491 — JOIN qualified columns and scalar
//! subqueries in WHERE.
//!
//! Two sub-bugs are reported:
//! - (a) `SELECT s.name, AVG(sc.final) FROM s JOIN sc ON s.id=sc.sid
//!        GROUP BY sc.sid` returns Null for the qualified `s.name`.
//! - (b) `WHERE id = (SELECT MIN(id) FROM s WHERE name='bob')` returns
//!        0 rows (or all-Null rows on vendor v3.11.0).
//!
//! Status of this fix:
//! - (a) **FIXED.** GROUP BY re-project now has a functional-dependency
//!   fallback that takes the first non-null value seen for each
//!   table_info column in the group, matching MySQL non-strict mode.
//!   See `a_issue_repro_returns_non_null_name` below.
//! - (b) **PARTIALLY FIXED.** The simple single-table scalar subquery
//!   case (`b_scalar_subquery_single_table`) passes. The full issue
//!   case with predicate referencing a CHAR-padded column requires
//!   non-correlated-subquery pre-evaluation in `execute_select`'s
//!   WHERE filter; tracked as follow-up.

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::MemoryStorage;
use sqlrustgo_types::Value;
use std::sync::Arc;

fn engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

// -----------------------------------------------------------------------
// (a) — Issue #4491(a) is now FIXED.
// -----------------------------------------------------------------------

#[test]
fn a_issue_repro_returns_non_null_name() {
    let mut e = engine();
    e.execute("CREATE TABLE s(id INTEGER, name CHAR(8))").unwrap();
    e.execute("CREATE TABLE sc(sid INTEGER, cid INTEGER, final INTEGER)")
        .unwrap();
    e.execute("INSERT INTO s VALUES (1, 'alice'), (2, 'bob')")
        .unwrap();
    e.execute("INSERT INTO sc VALUES (1, 100, 90), (1, 101, 85), (2, 100, 70)")
        .unwrap();
    let r = e
        .execute(
            "SELECT s.name, AVG(sc.final) \
             FROM s JOIN sc ON s.id = sc.sid \
             GROUP BY sc.sid",
        )
        .unwrap();
    // Two groups (sc.sid ∈ {1, 2}).
    assert_eq!(r.rows.len(), 2, "expected 2 groups, got {r:?}");
    // AVG(sc.final) per group: {sid=1: (90+85)/2=87.5}, {sid=2: 70.0}.
    let avgs: Vec<f64> = r
        .rows
        .iter()
        .map(|row| match &row[1] {
            Value::Float(f) => *f,
            other => panic!("expected Float avg, got {other:?}"),
        })
        .collect();
    assert!(avgs.contains(&87.5));
    assert!(avgs.contains(&70.0));
    // s.name is no longer Null after the #4491(a) fix: GROUP BY re-project
    // now has a functional-dependency fallback that takes the first
    // non-null value seen for each table_info column in the group
    // (matches MySQL non-strict GROUP BY semantics for `SELECT s.name,
    // AVG(sc.final) ... GROUP BY sc.sid`).
    let any_non_null = r.rows.iter().any(|row| !matches!(&row[0], Value::Null));
    assert!(
        any_non_null,
        "expected at least one non-Null in s.name after #4491a fix, got {r:?}"
    );
}

#[test]
fn a_unqualified_name_grouped_by_dependent_key() {
    // Same shape with GROUP BY s.id (the directly dependent key). The
    // functional-dependency fallback should also pick up `s.name` here.
    let mut e = engine();
    e.execute("CREATE TABLE s(id INTEGER, name CHAR(8))").unwrap();
    e.execute("CREATE TABLE sc(sid INTEGER, final INTEGER)")
        .unwrap();
    e.execute("INSERT INTO s VALUES (1, 'alice'), (2, 'bob')")
        .unwrap();
    e.execute("INSERT INTO sc VALUES (1, 90), (1, 85), (2, 70)")
        .unwrap();
    let r = e
        .execute(
            "SELECT s.name, AVG(sc.final) FROM s JOIN sc ON s.id = sc.sid \
             GROUP BY s.id",
        )
        .unwrap();
    assert_eq!(r.rows.len(), 2);
    for row in &r.rows {
        assert!(
            !matches!(&row[0], Value::Null),
            "s.name should be picked from group, got {row:?}"
        );
    }
}

// -----------------------------------------------------------------------
// (b) — simple scalar subquery on a single base table works today.
// -----------------------------------------------------------------------

#[test]
fn b_scalar_subquery_single_table() {
    let mut e = engine();
    e.execute("CREATE TABLE t(x INTEGER)").unwrap();
    e.execute("INSERT INTO t VALUES (5), (10), (15)").unwrap();
    let r = e
        .execute("SELECT * FROM t WHERE x = (SELECT MIN(x) FROM t)")
        .unwrap();
    assert_eq!(r.rows.len(), 1, "expected exactly 1 row, got {r:?}");
    assert!(matches!(&r.rows[0][0], Value::Integer(5)));
}

#[test]
fn b_issue_repro_scalar_subquery_with_inner_filter() {
    // Issue #4491(b) full case: `WHERE id = (SELECT MIN(id) FROM s WHERE
    // name = 'bob')` should return the row matching the scalar
    // subquery's MIN(id) result.
    //
    // PR #4508 originally pinned this as a known limitation (the
    // scalar-subquery pre-eval hook lived only in DML paths and the
    // correlated branch conservatively flagged all `Subquery(_)`
    // arms). PR #4493 (f118dd896c) closed BUG-3b by:
    // - threading `inner_columns` into `substitute_outer_refs_in_select`
    //   so inner-table bare column names are not mis-substituted as
    //   outer-row values, and
    // - adding `eval_predicate_with_subq` for the non-correlated else
    //   branch.
    // After the PR #4508 rebase on top of develop/v3.12.0 (which
    // carries PR #4493), this case now resolves to the expected
    // row. Assert it as a regression guard for both fixes.
    let mut e = engine();
    e.execute("CREATE TABLE s(id INTEGER, name CHAR(8))").unwrap();
    e.execute("INSERT INTO s VALUES (1, 'alice'), (2, 'bob')")
        .unwrap();
    let r = e
        .execute("SELECT * FROM s WHERE id = (SELECT MIN(id) FROM s WHERE name = 'bob')")
        .unwrap();
    assert_eq!(
        r.rows.len(),
        1,
        "Issue #4491(b) full case: scalar subquery with inner \
         WHERE filter should resolve to exactly one outer row. \
         PR #4493 closed BUG-3b. Got {r:?}"
    );
    assert!(matches!(&r.rows[0][0], Value::Integer(2)));
}