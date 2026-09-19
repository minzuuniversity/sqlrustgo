//! Regression tests for issue #4846 — CHAR(n) PAD SPACE semantics.
//!
//! Background (issue body, 2026-09-07):
//! CHAR(n) values are stored blank-padded to n bytes; `WHERE id='U1'` on a
//! CHAR(10) column should return the same row as `WHERE id='U1        '`
//! and as the same query against SQLite/MySQL/PostgreSQL. Prior to the
//! fix, the `sql_compare` entry point used strict `PartialEq` and the
//! `eq_cross` / `compare_values` Text arms also used strict `==`/
//! `cmp`, so `WHERE id='U1'` failed to match the stored `'U1        '`.
//!
//! The fix trims trailing whitespace on both sides before comparison
//! in `sql_compare`, `eq_cross`, `compare_values`, and the parallel
//! `merge.rs` / `exchange.rs` Text arms. SQLite/MySQL/PostgreSQL all
//! apply this PAD SPACE collation for CHAR by default (SQL:1992 §4.4.3
//! and §8.2.3).
//!
//! The BustubX-EDU teaching baseline (`teaching-seed.sql` CHAR primary
//! keys) was unable to point-look up rows via short literals like
//! `WHERE user_id='U20190001'` — this regression test covers the
//! minimum reproducer.

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::MemoryStorage;
use sqlrustgo_types::Value;
use std::sync::Arc;

fn create_engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

#[test]
fn test_issue_4846_char10_short_literal_matches_padded_storage() {
    // Issue #4846 minimum reproducer.
    let mut engine = create_engine();
    engine
        .execute("CREATE TABLE c(id CHAR(10), nm VARCHAR(20))")
        .unwrap();
    engine.execute("INSERT INTO c VALUES ('U1','tom')").unwrap();

    // Short literal must match the stored blank-padded value.
    let r = engine
        .execute("SELECT count(*) FROM c WHERE id='U1'")
        .unwrap();
    assert_eq!(
        r.rows[0][0],
        Value::Integer(1),
        "short literal must match CHAR(10) padded value (issue #4846)"
    );

    // Padded literal must also match.
    let r = engine
        .execute("SELECT count(*) FROM c WHERE id='U1        '")
        .unwrap();
    assert_eq!(
        r.rows[0][0],
        Value::Integer(1),
        "padded literal must match CHAR(10) padded value (issue #4846)"
    );

    // Negative match still works.
    let r = engine
        .execute("SELECT count(*) FROM c WHERE id='X'")
        .unwrap();
    assert_eq!(r.rows[0][0], Value::Integer(0));
}

#[test]
fn test_issue_4846_char10_like_prefix() {
    // Issue #4846 also notes that `LIKE 'U%'` (the actual prefix of the
    // stored value) should return 1 row. The text is stored as
    // 'U20190001' (then CHAR-padded to 'U20190001  '), so a literal
    // `LIKE 'U20190001%'` is the correct reproducer.
    let mut engine = create_engine();
    engine
        .execute("CREATE TABLE c(id CHAR(10), nm VARCHAR(20))")
        .unwrap();
    engine
        .execute("INSERT INTO c VALUES ('U20190001','alice')")
        .unwrap();
    let r = engine
        .execute("SELECT count(*) FROM c WHERE id LIKE 'U20190001%'")
        .unwrap();
    assert_eq!(r.rows[0][0], Value::Integer(1));
}

#[test]
fn test_issue_4846_char2_padded_compare_with_lt_gt() {
    // Ordering operators should also PAD SPACE.
    let mut engine = create_engine();
    engine
        .execute("CREATE TABLE t(id INTEGER, s CHAR(2))")
        .unwrap();
    engine
        .execute("INSERT INTO t VALUES (1, 'A '), (2, 'B ')")
        .unwrap();
    // 'A ' < 'B ' even though 'A' and 'B' are different chars; the
    // rstrip semantics still hold for ordering.
    let r = engine
        .execute("SELECT count(*) FROM t WHERE s < 'B'")
        .unwrap();
    assert_eq!(r.rows[0][0], Value::Integer(1));
}
