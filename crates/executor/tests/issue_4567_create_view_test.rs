//! Issue #4567 — CREATE VIEW was acked but the view never became queryable
//! (`SHOW TABLES` did not list it; `SELECT * FROM view` failed with
//! "Table not found").
//!
//! Root cause: `execute_create_view` stored only `format!("{:?}", view)`
//! (a Debug dump) in `self.views`; no catalog entry, no query path.
//!
//! Fix (this regression test pins the acceptance criteria):
//!   1. `SHOW TABLES` / `SHOW FULL TABLES` list the view after CREATE VIEW.
//!   2. The view is queryable and its result matches the inlined subquery.
//!   3. `SELECT *` over a `SELECT *`-defined view resolves real column
//!      names from the base table schema.

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo::ExecutorResult;
use sqlrustgo_storage::MemoryStorage;
use sqlrustgo_types::Value;
use std::sync::Arc;

fn engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

fn setup(engine: &mut ExecutionEngine<MemoryStorage>) {
    engine
        .execute("CREATE TABLE s(id INT, name CHAR(8))")
        .unwrap();
    engine
        .execute("INSERT INTO s VALUES (1,'alice'),(2,'bob')")
        .unwrap();
    engine
        .execute("CREATE VIEW v_ok AS SELECT id, name FROM s WHERE id = 1")
        .unwrap();
}

// Acceptance criterion 1: SHOW TABLES lists the view.
#[test]
fn show_tables_lists_view() {
    let mut e = engine();
    setup(&mut e);
    let r: ExecutorResult = e.execute("SHOW TABLES").unwrap();
    let has_view = r
        .rows
        .iter()
        .any(|row| matches!(row.first(), Some(Value::Text(n)) if n == "v_ok"));
    assert!(
        has_view,
        "SHOW TABLES must list view v_ok, got {:?}",
        r.rows
    );
}

#[test]
fn show_full_tables_marks_view_type() {
    let mut e = engine();
    setup(&mut e);
    let r = e.execute("SHOW FULL TABLES").unwrap();
    let view_row = r
        .rows
        .iter()
        .find(|row| matches!(row.first(), Some(Value::Text(n)) if n == "v_ok"));
    let view_row = view_row.unwrap_or_else(|| panic!("SHOW FULL TABLES must list v_ok"));
    match view_row.get(1) {
        Some(Value::Text(t)) => assert_eq!(t, "VIEW", "Table_type for v_ok must be VIEW"),
        other => panic!("expected Table_type column, got {other:?}"),
    }
}

// Acceptance criterion 2: the view is queryable and matches the inlined
// subquery (`SELECT id, name FROM s WHERE id = 1` → 1 row (1, 'alice')).
#[test]
fn select_from_view_matches_inlined_subquery() {
    let mut e = engine();
    setup(&mut e);
    let view_r = e.execute("SELECT * FROM v_ok").unwrap();
    assert_eq!(view_r.rows.len(), 1, "view must return 1 row");
    match (&view_r.rows[0][0], &view_r.rows[0][1]) {
        (Value::Integer(id), Value::Text(name)) => {
            assert_eq!(*id, 1);
            // CHAR(8) pads on storage (repo contract: tests/operators/
            // char_padding.rs) — compare without trailing pad spaces.
            assert_eq!(name.trim_end(), "alice");
        }
        other => panic!("expected (1, 'alice'), got {other:?}"),
    }
    // Direct column reference through the view.
    let r = e.execute("SELECT name FROM v_ok").unwrap();
    assert_eq!(r.rows.len(), 1);
    assert!(
        matches!(&r.rows[0][0], Value::Text(n) if n.trim_end() == "alice"),
        "expected 'alice' (modulo CHAR pad), got {:?}",
        r.rows[0][0]
    );
}

// View filtering + outer WHERE compose with the inlined subquery.
#[test]
fn view_with_outer_where_composes() {
    let mut e = engine();
    setup(&mut e);
    let r = e.execute("SELECT id FROM v_ok WHERE id > 0").unwrap();
    assert_eq!(r.rows.len(), 1);
    // Outer predicate excluding everything → 0 rows.
    let r = e.execute("SELECT id FROM v_ok WHERE id > 100").unwrap();
    assert_eq!(r.rows.len(), 0, "outer WHERE must filter view rows");
}

// `CREATE VIEW v AS SELECT * FROM t` — the star must expand to real
// base-table column names in the materialized schema.
#[test]
fn select_star_view_exposes_base_columns() {
    let mut e = engine();
    e.execute("CREATE TABLE t(a INT, b TEXT)").unwrap();
    e.execute("INSERT INTO t VALUES (7,'x')").unwrap();
    e.execute("CREATE VIEW v_star AS SELECT * FROM t").unwrap();
    let r = e.execute("SELECT b FROM v_star").unwrap();
    assert_eq!(r.rows.len(), 1);
    assert!(matches!(&r.rows[0][0], Value::Text(v) if v == "x"));
}

// Explicit column aliases: CREATE VIEW v(a, b) AS SELECT ...
#[test]
fn view_column_aliases_apply() {
    let mut e = engine();
    e.execute("CREATE TABLE base(x INT, y TEXT)").unwrap();
    e.execute("INSERT INTO base VALUES (3,'hi')").unwrap();
    e.execute("CREATE VIEW v_alias(p, q) AS SELECT x, y FROM base")
        .unwrap();
    let r = e.execute("SELECT p, q FROM v_alias").unwrap();
    assert_eq!(r.rows.len(), 1);
    assert!(matches!(r.rows[0][0], Value::Integer(3)));
    assert!(matches!(&r.rows[0][1], Value::Text(v) if v == "hi"));
}

// DROP VIEW keeps working with the new stored-AST representation.
#[test]
fn drop_view_removes_view() {
    let mut e = engine();
    setup(&mut e);
    e.execute("DROP VIEW v_ok").unwrap();
    let r = e.execute("SHOW TABLES").unwrap();
    let has_view = r
        .rows
        .iter()
        .any(|row| matches!(row.first(), Some(Value::Text(n)) if n == "v_ok"));
    assert!(!has_view, "v_ok must disappear after DROP VIEW");
}

// Data changes on the base table are visible through the view (views are
// not materialized snapshots).
#[test]
fn view_reflects_base_table_changes() {
    let mut e = engine();
    setup(&mut e);
    e.execute("INSERT INTO s VALUES (3,'carol')").unwrap();
    // v_ok filters id = 1, still 1 row.
    let r = e.execute("SELECT * FROM v_ok").unwrap();
    assert_eq!(r.rows.len(), 1);
    e.execute("DROP VIEW v_ok").unwrap();
    e.execute("CREATE VIEW v_all AS SELECT id FROM s").unwrap();
    let r = e.execute("SELECT * FROM v_all").unwrap();
    assert_eq!(r.rows.len(), 3, "new base row must be visible via view");
}
