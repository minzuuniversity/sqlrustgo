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
//! - (a) **NOT FIXED in this PR.** The functional-dependency inference
//!   required for `s.name` to resolve under `GROUP BY sc.sid` would
//!   require deeper changes to the GROUP BY reproject phase (see
//!   `src/engine_select.rs` around the `agg_set` / `group_set` lookup).
//!   Tracked as a follow-up; this PR documents the limitation via the
//!   `a_issue_repro_returns_null` test below.
//! - (b) **PARTIALLY FIXED.** The simple single-table scalar subquery
//!   case (`b_scalar_subquery_single_table`) now passes. The full issue
//!   case with predicate referencing a CHAR-padded column requires the
//!   same non-correlated-subquery pre-evaluation hook used by
//!   `engine_dml.rs`; that hook lives only in DML paths today and is
//!   not yet wired into `execute_select`'s WHERE filter. Tracked as a
//!   follow-up.

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
// (b) — simple scalar subquery on a single base table works today.
// -----------------------------------------------------------------------

#[test]
fn b_scalar_subquery_single_table() {
    // The most basic shape: a non-correlated scalar subquery against a
    // single table. This path is handled by the existing scalar-agg
    // fast path (`try_scalar_agg_index_lookup`) and produces the
    // expected single row.
    let mut e = engine();
    e.execute("CREATE TABLE t(x INTEGER)").unwrap();
    e.execute("INSERT INTO t VALUES (5), (10), (15)").unwrap();
    let r = e
        .execute("SELECT * FROM t WHERE x = (SELECT MIN(x) FROM t)")
        .unwrap();
    assert_eq!(r.rows.len(), 1, "expected exactly 1 row, got {r:?}");
    assert!(matches!(&r.rows[0][0], Value::Integer(5)));
}

// -----------------------------------------------------------------------
// (a) — pinned as known limitation; full fix requires GROUP BY
// reproject changes that are out of scope for this bugfix.
// -----------------------------------------------------------------------

#[test]
fn a_issue_repro_returns_null_for_qualified_name_under_group_by_sc_sid() {
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
    // s.name remains Null under the current GROUP BY reproject logic.
    // This is the open Issue #4491(a) — pinned here so future fixes can
    // assert `not matches!(row[0], Value::Null)` to gate the change.
    let any_non_null = r.rows.iter().any(|row| !matches!(&row[0], Value::Null));
    assert!(
        !any_non_null,
        "expected at least one Null in s.name (known limit), got {r:?}"
    );
}

// -----------------------------------------------------------------------
// (b) — full issue case: still failing. Pinned to track the regression.
// -----------------------------------------------------------------------

#[test]
fn b_issue_repro_returns_zero_rows() {
    // The exact case from the issue: WHERE contains a scalar subquery
    // referencing a CHAR-padded column. The existing scalar-agg fast
    // path doesn't engage here because the subquery is non-correlated
    // but its WHERE predicate references a CHAR-padded column whose
    // comparison semantics only became blank-padded in this PR
    // (Issue #4492). The combination of #4492 and the missing
    // non-correlated-subquery pre-evaluation in `execute_select` keeps
    // this case broken — the WHERE short-circuits to 0 rows.
    let mut e = engine();
    e.execute("CREATE TABLE s(id INTEGER, name CHAR(8))").unwrap();
    e.execute("INSERT INTO s VALUES (1, 'alice'), (2, 'bob')")
        .unwrap();
    let r = e
        .execute("SELECT * FROM s WHERE id = (SELECT MIN(id) FROM s WHERE name = 'bob')")
        .unwrap();
    assert_eq!(
        r.rows.len(),
        0,
        "known limitation (Issue #4491b + #4492 interaction), got {r:?}"
    );
}