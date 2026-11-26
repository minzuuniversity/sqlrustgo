//! V312-91 / Issue #4669 — DROP INDEX executor wiring.
//!
//! Before this fix, `execute_drop_index` in `src/execution_engine.rs:1134`
//! unconditionally returned `Err(SqlError::ExecutionError(
//! "DROP INDEX not fully supported yet"))`, so every DROP INDEX failed at
//! runtime even though the parser produced a valid `Statement::DropIndex`.
//!
//! Tests cover:
//! - Basic DROP INDEX after CREATE INDEX — no error, no row in
//!   `sqlite_master` introspection afterwards
//! - DROP INDEX IF EXISTS on missing index — no error (silent skip)
//! - DROP INDEX without IF EXISTS on missing index — runtime error
//!   (matches MySQL/SQLite/PG semantics: "Index does not exist")
//! - Re-CREATE INDEX with the same name after DROP — succeeds
//!   (proves the prior index really was removed from the catalog)
//! - DROP INDEX after multi-column CREATE INDEX — also works

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, Value};
use std::sync::Arc;

fn fresh_mem() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

/// Helper: query `sqlite_master` (via PR #4733 introspection path) and
/// return the rows. We use the original_sql column to distinguish index
/// rows from table/view rows (CREATE INDEX has "CREATE INDEX ..." prefix).
fn sqlite_master_index_names(e: &mut ExecutionEngine<MemoryStorage>) -> Vec<String> {
    let r = e
        .execute(
            "SELECT name FROM sqlite_master \
             WHERE type = 'index' AND name NOT LIKE 'sqlite_%' \
             ORDER BY name",
        )
        .expect("sqlite_master introspection query must succeed");
    r.rows
        .iter()
        .map(|row| match &row[0] {
            Value::Text(s) => s.clone(),
            other => panic!("expected TEXT, got {:?}", other),
        })
        .collect()
}

// ============================================================================
// Issue #4669 — exact issue body case (drop succeeds, name disappears)
// ============================================================================

#[test]
fn drop_index_basic_4669() {
    // V312-91 / Issue #4669: the placeholder error "DROP INDEX not fully
    // supported yet" must NOT be returned. After DROP, the index must
    // vanish from sqlite_master (no orphan catalog row).
    let mut e = fresh_mem();
    e.execute("CREATE TABLE t(a INT, b INT)").unwrap();
    e.execute("CREATE INDEX idx_ab ON t(a, b)").unwrap();

    // Sanity: index visible in catalog before DROP.
    let before = sqlite_master_index_names(&mut e);
    assert!(
        before.iter().any(|n| n == "idx_ab"),
        "CREATE INDEX must register idx_ab in sqlite_master; got {:?}",
        before
    );

    // The actual fix: DROP INDEX must succeed.
    let r = e.execute("DROP INDEX idx_ab");
    assert!(
        r.is_ok(),
        "DROP INDEX must succeed; got error: {:?}",
        r.err()
    );

    // After DROP: idx_ab must NOT appear in sqlite_master.
    let after = sqlite_master_index_names(&mut e);
    assert!(
        !after.iter().any(|n| n == "idx_ab"),
        "DROP INDEX must remove idx_ab from sqlite_master; got {:?}",
        after
    );
}

// ============================================================================
// IF EXISTS — silent skip when missing
// ============================================================================

#[test]
fn drop_index_if_exists_missing_is_ok_4669() {
    let mut e = fresh_mem();
    e.execute("CREATE TABLE t(a INT)").unwrap();
    // Never created idx_missing — DROP IF EXISTS must succeed silently.
    let r = e.execute("DROP INDEX IF EXISTS idx_missing");
    assert!(
        r.is_ok(),
        "DROP INDEX IF EXISTS on missing name must not error; got {:?}",
        r.err()
    );
}

// ============================================================================
// Without IF EXISTS — must error (preserve existing semantics)
// ============================================================================

#[test]
fn drop_index_missing_without_if_exists_errors_4669() {
    let mut e = fresh_mem();
    e.execute("CREATE TABLE t(a INT)").unwrap();
    let r = e.execute("DROP INDEX idx_missing");
    assert!(
        r.is_err(),
        "DROP INDEX without IF EXISTS on missing name must error; got Ok"
    );
    // The error must mention the missing index — surfaces a useful
    // diagnostic instead of silently succeeding.
    let err = format!("{:?}", r.err().unwrap());
    assert!(
        err.contains("idx_missing")
            || err.to_lowercase().contains("not exist")
            || err.to_lowercase().contains("no such"),
        "Error must reference the missing index name or 'not exist'; got: {}",
        err
    );
}

// ============================================================================
// Re-CREATE after DROP — proves the prior index was really removed
// ============================================================================

#[test]
fn recreate_index_after_drop_succeeds_4669() {
    // If DROP were a no-op (still returning the old placeholder error),
    // CREATE INDEX would fail with "index already exists". The very fact
    // that re-CREATE succeeds proves the DROP actually removed the row.
    let mut e = fresh_mem();
    e.execute("CREATE TABLE t(a INT)").unwrap();
    e.execute("CREATE INDEX idx_a ON t(a)").unwrap();
    e.execute("DROP INDEX idx_a").unwrap();
    // Must not error with "index already exists" or any other failure.
    let r = e.execute("CREATE INDEX idx_a ON t(a)");
    assert!(
        r.is_ok(),
        "Re-CREATE INDEX after DROP must succeed; got error: {:?}",
        r.err()
    );
    // And the index is visible again.
    let names = sqlite_master_index_names(&mut e);
    assert!(
        names.iter().any(|n| n == "idx_a"),
        "Re-CREATED idx_a must appear in sqlite_master; got {:?}",
        names
    );
}

// ============================================================================
// Multi-column index path — same fix path but a different CREATE shape
// ============================================================================

#[test]
fn drop_index_multi_column_4669() {
    let mut e = fresh_mem();
    e.execute("CREATE TABLE t(a INT, b INT, c INT)").unwrap();
    e.execute("CREATE INDEX idx_abc ON t(a, b, c)").unwrap();
    let r = e.execute("DROP INDEX idx_abc");
    assert!(
        r.is_ok(),
        "DROP INDEX on multi-column index must succeed; got: {:?}",
        r.err()
    );
    let names = sqlite_master_index_names(&mut e);
    assert!(
        !names.iter().any(|n| n == "idx_abc"),
        "idx_abc must be gone after DROP; got {:?}",
        names
    );
}

// ============================================================================
// V312-95 / Issue #4810 — BustubX-EDU differential_test.py P3-DDL-001
//
// The differential test issues the exact 4-statement sequence
//   DROP TABLE IF EXISTS t;
//   CREATE TABLE t (id INT, name TEXT);
//   INSERT INTO t VALUES (1, 'a'), (2, 'b'), (3, 'c');
//   CREATE INDEX idx_t_name ON t(name);
//   DROP INDEX idx_t_name;
// and asserts the final DROP does not error. Before PR #4789, `execute_drop_index`
// was a placeholder that returned "DROP INDEX not fully supported yet" for
// every call, so this sequence failed at the DROP step. With PR #4789 the
// executor now scans `list_all_indexes()` to recover the (table, name)
// composite key and routes to `storage.drop_index(table, name)`. This test
// pins the fix to the exact differential-test SQL flow.
// ============================================================================

#[test]
fn drop_index_differential_test_p3_ddl_001_4810() {
    let mut e = fresh_mem();
    // 1. DROP TABLE IF EXISTS — must not error on first run.
    e.execute("DROP TABLE IF EXISTS t")
        .expect("DROP TABLE IF EXISTS on missing table must succeed");

    // 2. CREATE TABLE — must succeed.
    e.execute("CREATE TABLE t (id INT, name TEXT)")
        .expect("CREATE TABLE must succeed");

    // 3. INSERT — note: the differential-test SQL uses bare unquoted a/b/c
    //    which would be parsed as column references and error. We use the
    //    realistic quoted form to model how a real test driver would
    //    serialize a parameterised INSERT.
    e.execute("INSERT INTO t VALUES (1, 'a'), (2, 'b'), (3, 'c')")
        .expect("INSERT must succeed");

    // 4. CREATE INDEX — must succeed.
    e.execute("CREATE INDEX idx_t_name ON t(name)")
        .expect("CREATE INDEX must succeed");

    // Sanity: index visible before DROP.
    let before = sqlite_master_index_names(&mut e);
    assert!(
        before.iter().any(|n| n == "idx_t_name"),
        "CREATE INDEX must register idx_t_name in sqlite_master; got {:?}",
        before
    );

    // 5. DROP INDEX — the bug-report step. Before PR #4789 this raised
    //    `DROP INDEX failed: index 'idx_t_name' does not exist`.
    let r = e.execute("DROP INDEX idx_t_name");
    assert!(
        r.is_ok(),
        "DROP INDEX after CREATE INDEX must succeed (Issue #4810 anchor case); \
         got error: {:?}",
        r.err()
    );

    // After DROP: idx_t_name must vanish from sqlite_master.
    let after = sqlite_master_index_names(&mut e);
    assert!(
        !after.iter().any(|n| n == "idx_t_name"),
        "idx_t_name must be gone after DROP; got {:?}",
        after
    );
}

// ============================================================================
// Issue #4669 sub-bug 2 — function index `CREATE INDEX i ON t(UPPER(a))`
//
// Before 2026-11-26 the parser took `UPPER` as a **column name**, so the
// executor looked for a column literally named "UPPER" and failed with the
// misleading `Column not found`. The parser now records the entry as an
// expression, and the executor refuses the statement with a message that
// says what is actually missing (this engine does not materialise
// expression indexes) instead of pretending the column does not exist.
// ============================================================================

#[test]
fn function_index_is_rejected_with_a_clear_error_4669() {
    let mut e = fresh_mem();
    e.execute("CREATE TABLE t(a INT, b INT)").unwrap();
    let err = e
        .execute("CREATE INDEX idx_up ON t(UPPER(a))")
        .expect_err("function indexes must be refused, not silently mis-indexed");
    let msg = err.to_string();
    assert!(
        msg.contains("not supported"),
        "the error must say the feature is unsupported, got: {msg}"
    );
    assert!(
        !msg.contains("Column not found"),
        "the misleading 'Column not found' must be gone, got: {msg}"
    );
    // A refused statement must not leave an index behind.
    let names = sqlite_master_index_names(&mut e);
    assert!(
        !names.iter().any(|n| n == "idx_up"),
        "no index row may survive the refusal, got {names:?}"
    );
}

// ============================================================================
// Issue #4669 sub-bug 3 — partial index `CREATE INDEX i ON t(a) WHERE a > 200`
//
// Before 2026-11-26 the WHERE clause was never parsed and the top-level
// parser tolerated the trailing tokens ⇒ the predicate was **silently
// dropped** and a FULL index was created ("静默接受但实际未生效"). The
// parser now records the predicate, and the executor refuses the statement.
// ============================================================================

#[test]
fn partial_index_is_rejected_with_a_clear_error_4669() {
    let mut e = fresh_mem();
    e.execute("CREATE TABLE t(a INT, b INT)").unwrap();
    e.execute("INSERT INTO t VALUES (1, 1), (300, 2)").unwrap();
    let err = e
        .execute("CREATE INDEX idx_partial ON t(a) WHERE a > 200")
        .expect_err("partial indexes must be refused, not silently turned into full indexes");
    let msg = err.to_string();
    assert!(
        msg.contains("partial indexes"),
        "the error must name the partial-index form, got: {msg}"
    );
    // The refusal must not leave a full index pretending to be partial.
    let names = sqlite_master_index_names(&mut e);
    assert!(
        !names.iter().any(|n| n == "idx_partial"),
        "no index row may survive the refusal, got {names:?}"
    );
}

// ============================================================================
// Regression guard — the ordinary single-column index still works.
// ============================================================================

#[test]
fn plain_column_index_still_works_4669() {
    let mut e = fresh_mem();
    e.execute("CREATE TABLE t(a INT, b INT)").unwrap();
    e.execute("CREATE INDEX idx_a ON t(a)")
        .expect("a plain column index must still be accepted");
    let names = sqlite_master_index_names(&mut e);
    assert!(
        names.iter().any(|n| n == "idx_a"),
        "idx_a must be in the catalog, got {names:?}"
    );
}
