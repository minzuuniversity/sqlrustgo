//! Regression test for Issue #4491 (post PR #4493 residual scope).
//!
//! PR #4493 (commit f118dd896c) fixed the basic BUG-3a / BUG-3b cases.
//! The remaining scope (per the issue body updated 2026-08-27) is:
//!
//!   - JOIN + alias.column + GROUP BY returns Null when the shared
//!     JOIN key is a CHAR-typed (multi-byte) column, instead of an
//!     INT-typed one. The MySQL non-strict mode fallback in
//!     `src/engine_select.rs` (re-projection: scan original `rows`
//!     for the first row matching the current group key) fails to
//!     resolve `s.sname` against the JOIN schema when the join key
//!     column is CHAR(n).
//!
//! These tests are RED on HEAD (develop/v3.12.0 @ f6a161a9, post-PR-#4493)
//! and turn GREEN once the fix is in.
//!
//! Reference data: the issue body (`docs/reference/sqlrustgo-bug-report.md`
//! BUG-3a) specifies the 清华 A-track teaching schema:
//!
//!     s(studentno char(11) PK, sname char(8))
//!     sc(studentno char(11) FK, final int)
//!
//! Both `sname` and the JOIN key are CHAR-typed.

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, Value};
use std::sync::Arc;

/// CHAR-trimmed assertion (MySQL CHAR semantics: trailing blanks are
/// insignificant in comparisons but PRESERVED in stored values). We
/// compare semantic equivalence, not raw byte equality.
fn text_trimmed(v: &Value) -> String {
    match v {
        Value::Text(s) => s.trim_end().to_string(),
        Value::Null => "<Null>".to_string(),
        ref other => panic!("expected Text, got {:?}", other),
    }
}

fn fresh_engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

fn setup_char_typed_joined_tables(e: &mut ExecutionEngine<MemoryStorage>) {
    e.execute("create table s(studentno char(11), sname char(8))")
        .unwrap();
    e.execute("create table sc(studentno char(11), final int)")
        .unwrap();
    e.execute("insert into s values('18122210009','alice'),('18122221324','bob')")
        .unwrap();
    e.execute("insert into sc values('18122210009',82),('18122221324',91)")
        .unwrap();
}

/// BUG-3a residual (HEAD f6a161a9 pre-PR): with CHAR-typed shared JOIN
/// key, `s.sname` is Null while `avg(sc.final)` is correct.
///
/// Expected (MySQL/SQLite): 2 rows, sname ∈ {alice, bob}.
/// Pre-fix: 2 rows, sname = Null for both.
#[test]
fn bug_4491_a_join_alias_column_with_char_key_returns_value() {
    let mut e = fresh_engine();
    setup_char_typed_joined_tables(&mut e);

    let r = e
        .execute(
            "select s.sname, avg(sc.final) \
             from s join sc on s.studentno = sc.studentno \
             group by sc.studentno",
        )
        .unwrap();

    eprintln!("BUG-3a (char-key) rows:");
    for row in &r.rows {
        eprintln!("  {:?}", row);
    }
    assert_eq!(
        r.rows.len(),
        2,
        "BUG-3a (char-key): expected 2 rows, got {}",
        r.rows.len()
    );
    for row in &r.rows {
        let sname = text_trimmed(&row[0]);
        assert!(
            sname == "alice" || sname == "bob",
            "BUG-3a (char-key): expected sname ∈ {{alice, bob}}, got '{}'",
            sname
        );
    }
}

/// Positive control (already works on HEAD via PR #4493): same shape
/// with INT PK confirms the fix path is wired; this guards against
/// a regression in the INT branch.
#[test]
fn bug_4491_a_int_pk_baseline_still_works() {
    let mut e = fresh_engine();
    e.execute("create table s(id int, name char(8))").unwrap();
    e.execute("create table sc(sid int, cid int, final int)")
        .unwrap();
    e.execute("insert into s values(1, 'alice'), (2, 'bob')")
        .unwrap();
    e.execute("insert into sc values(1, 100, 90), (1, 101, 85), (2, 100, 70)")
        .unwrap();
    let r = e
        .execute(
            "select s.name, avg(sc.final) \
             from s join sc on s.id = sc.sid \
             group by sc.sid",
        )
        .unwrap();
    assert_eq!(r.rows.len(), 2);
    for row in &r.rows {
        let sname = text_trimmed(&row[0]);
        assert!(sname == "alice" || sname == "bob");
    }
}

/// BUG-3b residual sanity: scalar subquery in WHERE (per PR #4493).
/// This case is GREEN on HEAD; included so any future regression in
/// the correlated-vs-uncorrelated branching is caught immediately.
#[test]
fn bug_4491_b_scalar_subquery_in_where_works() {
    let mut e = fresh_engine();
    e.execute("create table s(id int, name char(8))").unwrap();
    e.execute("insert into s values(1, 'alice'), (2, 'bob')")
        .unwrap();
    let r = e
        .execute(
            "select * from s \
            where id = (select min(id) from s where name = 'bob')",
        )
        .unwrap();
    eprintln!("BUG-3b rows:");
    for row in &r.rows {
        eprintln!("  {:?}", row);
    }
    assert_eq!(
        r.rows.len(),
        1,
        "BUG-3b: expected 1 row, got {}",
        r.rows.len()
    );
    let id = match r.rows[0][0] {
        Value::Integer(i) => i,
        ref other => panic!("expected Integer id, got {:?}", other),
    };
    assert_eq!(id, 2, "BUG-3b: expected id=2 ('bob'), got {}", id);
}
