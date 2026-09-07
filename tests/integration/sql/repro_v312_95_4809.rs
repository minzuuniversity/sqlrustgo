//! V312-95 v2 / Issue #4809 — `FROM t INDEXED BY idx` and `FROM t NOT
//! INDEXED` SQLite-style hints silently ignored.
//!
//! Before this fix, `parse_optional_alias` consumed the bare identifier
//! `INDEXED` as the table alias (because `INDEXED` is not a reserved
//! keyword — it lands as `Token::Identifier("INDEXED")`). The parser
//! then mishandled the trailing `BY` token and silently DROPPED the
//! WHERE clause entirely. A query like:
//!
//!   SELECT * FROM t INDEXED BY idx_t_name WHERE name = 'a'
//!
//! parsed as:
//!
//!   SelectStatement { table: "t|INDEXED", where_clause: None, ... }
//!
//! — returning every row in the table with no filter.
//!
//! This file pins both halves of the fix:
//!
//! 1. Parser (crates/parser/src/parser.rs):
//!    - `parse_optional_alias` no longer consumes `INDEXED` as alias.
//!    - `parse_optional_index_hint` (new helper) recognises
//!      `INDEXED BY <ident>` and `NOT INDEXED`.
//!    - Two new fields on `SelectStatement`: `from_indexed_by`,
//!      `from_not_indexed`.
//!
//! 2. Executor (src/engine_select.rs): before the scan runs, the
//!    `INDEXED BY <name>` hint is validated — the index must exist and
//!    must belong to the queried table. Errors otherwise.
//!
//! `NOT INDEXED` is a planner hint that does not require validation;
//! we rely on the default sequential scan path which sqlrustgo
//! already implements.

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, Value};
use std::sync::Arc;

fn fresh_mem() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

/// Insert N rows of the form (id INT, name TEXT).
fn seed_two_col_table(e: &mut ExecutionEngine<MemoryStorage>, table: &str, n: usize) {
    e.execute(&format!("CREATE TABLE {}(id INT, name TEXT)", table))
        .expect("CREATE TABLE must succeed");
    for i in 1..=n {
        e.execute(&format!(
            "INSERT INTO {} VALUES ({}, 'name{}')",
            table, i, i
        ))
        .expect("INSERT must succeed");
    }
}

// ============================================================================
// Issue #4809 anchor case — WHERE clause must be preserved + hint parsed
// ============================================================================

#[test]
fn indexed_by_valid_preserves_where_4809() {
    // Before the fix, this query returned ALL 3 rows because the parser
    // ate `INDEXED` as the alias and dropped the WHERE clause. The
    // anchor case for #4809 is the SELECT below; we assert:
    //   1. parsing succeeds (no error),
    //   2. the WHERE clause is honoured (only matching row returned),
    //   3. the indexed-by index exists in the catalog.
    let mut e = fresh_mem();
    seed_two_col_table(&mut e, "t", 3);
    e.execute("CREATE INDEX idx_t_name ON t(name)")
        .expect("CREATE INDEX must succeed");

    let r = e
        .execute("SELECT id FROM t INDEXED BY idx_t_name WHERE name = 'name2'")
        .expect("SELECT with INDEXED BY must succeed (hint is valid)");
    assert_eq!(
        r.rows.len(),
        1,
        "WHERE must be honoured (root cause of #4809); got {} rows",
        r.rows.len()
    );
    assert_eq!(r.rows[0][0], Value::Integer(2));
}

// ============================================================================
// INDEXED BY name that does NOT exist — must error, not silently succeed
// ============================================================================

#[test]
fn indexed_by_missing_index_errors_4809() {
    let mut e = fresh_mem();
    seed_two_col_table(&mut e, "t", 2);

    let r = e.execute("SELECT * FROM t INDEXED BY idx_does_not_exist");
    assert!(
        r.is_err(),
        "INDEXED BY <unknown index> must error; got Ok (silent ignore)"
    );
    let err = format!("{}", r.unwrap_err());
    assert!(
        err.contains("idx_does_not_exist") || err.to_lowercase().contains("index"),
        "error must mention the missing index; got: {}",
        err
    );
}

// ============================================================================
// INDEXED BY name exists but is on a different table — must error
// ============================================================================

#[test]
fn indexed_by_wrong_table_errors_4809() {
    let mut e = fresh_mem();
    seed_two_col_table(&mut e, "t1", 2);
    seed_two_col_table(&mut e, "t2", 2);
    e.execute("CREATE INDEX idx_t2_name ON t2(name)")
        .expect("CREATE INDEX on t2 must succeed");

    // Index idx_t2_name exists but belongs to t2, not t1.
    let r = e.execute("SELECT * FROM t1 INDEXED BY idx_t2_name");
    assert!(
        r.is_err(),
        "INDEXED BY with index on wrong table must error; got Ok"
    );
    let err = format!("{}", r.unwrap_err());
    assert!(
        err.contains("idx_t2_name") || err.contains("t1") || err.contains("t2"),
        "error must reference the mismatch (index name + the two tables); got: {}",
        err
    );
}

// ============================================================================
// NOT INDEXED — must not error and must return all rows
// ============================================================================

#[test]
fn not_indexed_returns_all_rows_4809() {
    let mut e = fresh_mem();
    seed_two_col_table(&mut e, "t", 3);
    e.execute("CREATE INDEX idx_t_id ON t(id)")
        .expect("CREATE INDEX must succeed");

    let r = e
        .execute("SELECT id FROM t NOT INDEXED WHERE id > 1")
        .expect("NOT INDEXED must succeed");
    assert_eq!(
        r.rows.len(),
        2,
        "WHERE id > 1 must apply (id 2 and 3); got {} rows",
        r.rows.len()
    );
}

// ============================================================================
// Regression guard — bare INDEXED alone (without BY) must error, not crash
// ============================================================================

#[test]
fn indexed_without_by_errors_cleanly_4809() {
    let mut e = fresh_mem();
    seed_two_col_table(&mut e, "t", 1);

    let r = e.execute("SELECT * FROM t INDEXED");
    // The parser used to silently consume `INDEXED` as the table alias;
    // we want a clean error rather than either a silent success-with-bad-
    // result OR a panic. Index hints without `BY` are not valid SQLite
    // syntax.
    assert!(
        r.is_err(),
        "bare `INDEXED` (without BY <name>) must produce a clean parse error; got Ok"
    );
}

// ============================================================================
// Regression guard — query WITHOUT INDEXED BY must still work as before
// ============================================================================

#[test]
fn plain_select_still_works_4809() {
    let mut e = fresh_mem();
    seed_two_col_table(&mut e, "t", 3);

    let r = e
        .execute("SELECT id FROM t WHERE name = 'name2'")
        .expect("plain SELECT must still work after the fix");
    assert_eq!(r.rows.len(), 1);
    assert_eq!(r.rows[0][0], Value::Integer(2));
}

// ============================================================================
// INDEXED BY combined with bare inline alias (`FROM t x`) — both must
// coexist. We use the bare-alias form (NOT `AS x`) because `FROM t AS
// x` is a known pre-existing bug unrelated to #4809 (out of scope).
// ============================================================================

#[test]
fn indexed_by_with_bare_alias_4809() {
    let mut e = fresh_mem();
    seed_two_col_table(&mut e, "t", 3);
    e.execute("CREATE INDEX idx_t_id ON t(id)")
        .expect("CREATE INDEX must succeed");

    let r = e
        .execute("SELECT id FROM t x INDEXED BY idx_t_id WHERE id = 2")
        .expect("INDEXED BY combined with bare inline alias must succeed");
    assert_eq!(r.rows.len(), 1);
    assert_eq!(r.rows[0][0], Value::Integer(2));
}
