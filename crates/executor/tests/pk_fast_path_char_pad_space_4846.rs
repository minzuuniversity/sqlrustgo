//! #4846: a CHAR primary key must not take the PK point-lookup fast path.
//!
//! `execute_select` replaces `scan + evaluate_where_clause` with
//! `storage.scan_pk` whenever the WHERE clause is exactly
//! `pk_col = <literal>` — and returns the scanned row **without
//! re-checking it against the WHERE clause**. That substitution is only
//! sound when `scan_pk` agrees with `sql_compare`.
//!
//! For CHAR it does not. A CHAR(10) holding `'U1'` is stored as
//! `"U1        "`, and PAD SPACE comparison says `id = 'U1'` matches it.
//! The default `scan_pk` is `row.first() == Some(&pk)` — strict
//! equality, no padding rule — so it finds nothing.
//!
//! The loss is silent and shape-dependent, which is what makes it worth
//! pinning. Measured before the fix on this exact fixture:
//!
//! ```text
//! WHERE id = 'U1'          -> 0 rows
//! WHERE id = 'U1' AND 1=1  -> 1 row     (falls back to the scan path)
//! ```
//!
//! The two differ only in conjunct count, because
//! `try_extract_pk_eq_with_col` returns `None` for anything that is not
//! a bare `col = literal` — so the second form takes the ordinary scan
//! path, which does apply PAD SPACE.

use sqlrustgo::MemoryExecutionEngine;
use sqlrustgo_types::Value;

fn as_i64(v: &Value) -> i64 {
    v.as_integer()
        .unwrap_or_else(|| panic!("expected integer, got {v:?}"))
}

fn engine_with(ddl: &str) -> MemoryExecutionEngine {
    let mut e = MemoryExecutionEngine::with_memory();
    e.execute(ddl).unwrap();
    e
}

/// #4846 headline: a single `=` on a CHAR column used to match nothing.
#[test]
fn single_equality_on_char_column_matches() {
    let mut e = engine_with("CREATE TABLE c(id CHAR(10), nm VARCHAR(20))");
    e.execute("INSERT INTO c VALUES ('U1','tom')").unwrap();

    let r = e.execute("SELECT count(*) FROM c WHERE id = 'U1'").unwrap();
    assert_eq!(
        as_i64(&r.rows[0][0]),
        1,
        "PAD SPACE: a CHAR(10) holding 'U1' is stored padded, and 'U1' must match it"
    );
}

/// The bare `col = literal` form is the one that takes the fast path, so
/// it is the only one that regressed. The conjunct form already worked
/// and must keep working — a fix that quietly pushed everything to a
/// slower path would hide a future regression behind it.
#[test]
fn conjunct_form_agrees_with_the_bare_form() {
    let mut e = engine_with("CREATE TABLE c(id CHAR(10), nm VARCHAR(20))");
    e.execute("INSERT INTO c VALUES ('U1','tom'),('U2','jerry')")
        .unwrap();

    let bare = as_i64(
        &e.execute("SELECT count(*) FROM c WHERE id = 'U1'")
            .unwrap()
            .rows[0][0],
    );
    let with_extra = as_i64(
        &e.execute("SELECT count(*) FROM c WHERE id = 'U1' AND 1=1")
            .unwrap()
            .rows[0][0],
    );
    assert_eq!(bare, 1);
    assert_eq!(
        bare, with_extra,
        "adding a tautological conjunct must not change the answer"
    );
}

/// Projection, not just count(*) — the fast path returned `rows` straight
/// through, so a wrong count could have masked a right row set and vice
/// versa.
#[test]
fn projection_through_the_fast_path_is_correct() {
    let mut e = engine_with("CREATE TABLE c(id CHAR(10), nm VARCHAR(20))");
    e.execute("INSERT INTO c VALUES ('U1','tom'),('U2','jerry')")
        .unwrap();

    let rows = e
        .execute("SELECT id, nm FROM c WHERE id = 'U2'")
        .unwrap()
        .rows;
    assert_eq!(rows.len(), 1, "got {rows:?}");
    assert_eq!(rows[0][0], Value::Text("U2        ".to_string()));
    assert_eq!(rows[0][1], Value::Text("jerry".to_string()));
}

/// VARCHAR must keep the fast path: trailing spaces are significant
/// there, so `scan_pk`'s strict equality is the correct semantics and
/// PAD SPACE must NOT be applied. If this test ever fails because CHAR
/// was routed correctly, it means the guard grew too wide.
#[test]
fn varchar_still_uses_strict_equality() {
    let mut e = engine_with("CREATE TABLE c(id VARCHAR(10), nm VARCHAR(20))");
    e.execute("INSERT INTO c VALUES ('U1','tom'),('U2','jerry')")
        .unwrap();

    assert_eq!(
        as_i64(
            &e.execute("SELECT count(*) FROM c WHERE id = 'U1'")
                .unwrap()
                .rows[0][0]
        ),
        1
    );
    // A padded literal must NOT match a VARCHAR holding the short value.
    assert_eq!(
        as_i64(
            &e.execute("SELECT count(*) FROM c WHERE id = 'U1        '")
                .unwrap()
                .rows[0][0]
        ),
        0,
        "VARCHAR is not PAD SPACE: trailing spaces are part of the value"
    );
}

/// A missing value must stay missing rather than matching some row.
#[test]
fn absent_key_still_returns_no_rows() {
    let mut e = engine_with("CREATE TABLE c(id CHAR(10), nm VARCHAR(20))");
    e.execute("INSERT INTO c VALUES ('U1','tom')").unwrap();
    assert_eq!(
        as_i64(
            &e.execute("SELECT count(*) FROM c WHERE id = 'ZZ'")
                .unwrap()
                .rows[0][0]
        ),
        0,
        "falling back to the scan path must not turn into a match-all"
    );
}
