//! Regression tests for Issue #4674 (V312-RC-GA PR-A5) —
//! CHAR_LENGTH / CHARACTER_LENGTH on a column reference returns the
//! column-DEFAULT length instead of the row's actual string length.
//!
//! Reproducer from issue #4674 body:
//!
//! ```sql
//! CREATE TABLE t(name VARCHAR(50));
//! INSERT INTO t VALUES ('abc'), ('hello');
//! SELECT CHAR_LENGTH(name) FROM t;
//! ```
//!
//! Expected: `3`, `5` (UTF-8 codepoint count of row values).
//!
//! Issue body reported actual `50, 50` (column-VARCHAR(50) default length
//! surfacing through CHAR_LENGTH because column reference was not substituted
//! with the row's value before `chars().count()` ran). That symptom is no
//! longer reproducible in current HEAD (`c67d4fddc0`, post-V312-67+#4731
//! scalar-subquery work) — the column DOES substitute correctly. The tests
//! below lock that behavior in as anti-regression and cover the
//! CHARACTER_LENGTH alias + literal arg paths as well.
//!
//! Per V312-RC-GA Triage §3 PR-A5 / WP-B entry for #4674, the GA cut **must**
//! produce accurate `CHAR_LENGTH(name) = N` for short ASCII strings; if
//! regression reintroduces the column-default-length symptom, this test
//! fails closed.
//!
//! Per V312-RC-GA Triage §3 PR-A5 / WP-B entry for #4674, the GA cut **must**
//! produce accurate `CHAR_LENGTH(name) = N` for short ASCII strings, otherwise
//! B-track queries on name columns silently mislead users.

use sqlrustgo::{ExecutionEngine, MemoryStorage};

fn engine() -> ExecutionEngine<MemoryStorage> {
    ExecutionEngine::new(std::sync::Arc::new(parking_lot::RwLock::new(
        MemoryStorage::new(),
    )))
}

#[test]
fn char_length_on_short_ascii_returns_codepoint_count() {
    let mut e = engine();
    e.execute("CREATE TABLE t(name VARCHAR(50))").unwrap();
    e.execute("INSERT INTO t VALUES ('abc'), ('hello')")
        .unwrap();
    let result = e.execute("SELECT CHAR_LENGTH(name) FROM t").unwrap();
    let rows = result.rows;
    assert_eq!(rows.len(), 2, "expected 2 rows");
    assert_eq!(
        rows[0][0].to_sql_string().parse::<i64>().unwrap(),
        3,
        "row 0 ('abc') should give CHAR_LENGTH = 3"
    );
    assert_eq!(
        rows[1][0].to_sql_string().parse::<i64>().unwrap(),
        5,
        "row 1 ('hello') should give CHAR_LENGTH = 5"
    );
}

#[test]
fn character_length_on_short_ascii_returns_codepoint_count() {
    let mut e = engine();
    e.execute("CREATE TABLE t(name VARCHAR(50))").unwrap();
    e.execute("INSERT INTO t VALUES ('abc')").unwrap();
    let result = e
        .execute("SELECT CHARACTER_LENGTH(name) FROM t")
        .unwrap();
    let rows = result.rows;
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0][0].to_sql_string().parse::<i64>().unwrap(),
        3,
        "CHARACTER_LENGTH(name) should be 3, not 50"
    );
}

#[test]
fn char_length_on_literal_returns_codepoint_count() {
    // Literal arg path — should already work pre-fix.
    let mut e = engine();
    let result = e.execute("SELECT CHAR_LENGTH('hello')").unwrap();
    let rows = result.rows;
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0][0].to_sql_string().parse::<i64>().unwrap(),
        5,
        "CHAR_LENGTH('hello') literal should be 5"
    );
}
