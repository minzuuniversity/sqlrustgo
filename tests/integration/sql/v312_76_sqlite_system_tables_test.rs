//! V312-76 / Issue #4682 regression integration test:
//! SQLite system-table compatibility beyond the v312-71 `sqlite_master`
//! synthesis:
//! 1. `SELECT sql FROM sqlite_master` — the `sql` column must be
//!    projectable (the lexer previously tokenised `SQL` as a dedicated
//!    keyword the parser never consumed, so the projection failed to
//!    parse), and the system-view fast path must project columns.
//! 2. `sqlite_sequence` — one `(name, seq)` row per table with an
//!    auto-increment column, `seq` = current max id.
//! 3. `WHERE` pushdown on the synthesized views
//!    (`WHERE type = 'table'`, `WHERE name = ...`).
//! 4. `CREATE VIRTUAL TABLE ... USING fts5(...)` parses and creates a
//!    queryable table (FTS5 columns become TEXT columns; content is
//!    persisted, `MATCH` remains unsupported).

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, Value};
use std::sync::Arc;

fn fresh() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

/// `SELECT sql FROM sqlite_master` must return the CREATE statements,
/// one column wide (pre-#4682: the `SQL` keyword token broke parsing,
/// and the fast path returned all 5 raw columns without projection).
#[test]
fn v312_76_sqlite_master_sql_column_projection() {
    let mut x = fresh();
    x.execute("CREATE TABLE users (id INT, name TEXT)").unwrap();
    let res = x.execute("SELECT sql FROM sqlite_master").unwrap();
    assert_eq!(res.rows.len(), 1);
    assert_eq!(
        res.rows[0].len(),
        1,
        "projection must yield exactly 1 column"
    );
    let sql = match &res.rows[0][0] {
        Value::Text(s) => s,
        other => panic!("expected Text sql, got {:?}", other),
    };
    assert!(
        sql.to_uppercase().contains("CREATE TABLE"),
        "sql must contain CREATE TABLE, got: {}",
        sql
    );
}

/// Qualified projection `SELECT sqlite_master.name, type FROM sqlite_master`.
#[test]
fn v312_76_sqlite_master_qualified_projection() {
    let mut x = fresh();
    x.execute("CREATE TABLE t1 (x INT)").unwrap();
    let res = x
        .execute("SELECT sqlite_master.name, type FROM sqlite_master")
        .unwrap();
    assert_eq!(res.rows.len(), 1);
    assert_eq!(res.rows[0].len(), 2);
    assert_eq!(res.rows[0][0], Value::Text("t1".to_string()));
    assert_eq!(res.rows[0][1], Value::Text("table".to_string()));
}

/// WHERE pushdown on the master view: `WHERE type = 'table'`.
#[test]
fn v312_76_sqlite_master_where_type_filter() {
    let mut x = fresh();
    x.execute("CREATE TABLE a (x INT)").unwrap();
    let res = x
        .execute("SELECT name FROM sqlite_master WHERE type = 'table'")
        .unwrap();
    assert_eq!(res.rows.len(), 1);
    assert_eq!(res.rows[0][0], Value::Text("a".to_string()));
    // Non-matching filter → empty set, not unfiltered rows.
    let res = x
        .execute("SELECT name FROM sqlite_master WHERE type = 'index'")
        .unwrap();
    assert_eq!(res.rows.len(), 0);
}

/// `sqlite_sequence` synthesizes one row per auto-increment table with
/// the current max id; tables without AUTOINCREMENT are omitted.
#[test]
fn v312_76_sqlite_sequence_tracks_autoincrement_max() {
    let mut x = fresh();
    x.execute("CREATE TABLE t (id INTEGER PRIMARY KEY AUTOINCREMENT, v TEXT)")
        .unwrap();
    x.execute("INSERT INTO t (v) VALUES ('a'), ('b'), ('c')")
        .unwrap();
    x.execute("CREATE TABLE plain (x INT)").unwrap();
    let res = x.execute("SELECT * FROM sqlite_sequence").unwrap();
    assert_eq!(res.rows.len(), 1, "only auto-increment tables listed");
    assert_eq!(res.rows[0][0], Value::Text("t".to_string()));
    assert_eq!(res.rows[0][1], Value::Integer(3));
}

/// `sqlite_sequence.name` lookup with WHERE.
#[test]
fn v312_76_sqlite_sequence_where_name_filter() {
    let mut x = fresh();
    x.execute("CREATE TABLE t (id INTEGER PRIMARY KEY AUTOINCREMENT)")
        .unwrap();
    x.execute("INSERT INTO t (id) VALUES (1), (5)").unwrap();
    let res = x
        .execute("SELECT seq FROM sqlite_sequence WHERE name = 't'")
        .unwrap();
    assert_eq!(res.rows.len(), 1);
    assert_eq!(res.rows[0].len(), 1);
    assert_eq!(res.rows[0][0], Value::Integer(5));
    let res = x
        .execute("SELECT seq FROM sqlite_sequence WHERE name = 'nope'")
        .unwrap();
    assert_eq!(res.rows.len(), 0);
}

/// Empty `sqlite_sequence` (no auto-increment tables) returns 0 rows.
#[test]
fn v312_76_sqlite_sequence_empty_without_autoincrement() {
    let mut x = fresh();
    x.execute("CREATE TABLE t (x INT)").unwrap();
    let res = x.execute("SELECT * FROM sqlite_sequence").unwrap();
    assert_eq!(res.rows.len(), 0);
}

/// `sqlite_temp_master` synthesizes as an empty view (TEMP objects are
/// not supported).
#[test]
fn v312_76_sqlite_temp_master_empty() {
    let mut x = fresh();
    x.execute("CREATE TABLE t (x INT)").unwrap();
    let res = x.execute("SELECT * FROM sqlite_temp_master").unwrap();
    assert_eq!(res.rows.len(), 0);
}

/// `CREATE VIRTUAL TABLE ... USING fts5(...)` parses; bare module
/// arguments become TEXT columns; the table is insertable and
/// queryable like an ordinary table.
#[test]
fn v312_76_create_virtual_table_fts5() {
    let mut x = fresh();
    x.execute("CREATE VIRTUAL TABLE docs USING fts5(title, body)")
        .unwrap();
    x.execute("INSERT INTO docs VALUES ('hello', 'world')")
        .unwrap();
    let res = x.execute("SELECT title, body FROM docs").unwrap();
    assert_eq!(res.rows.len(), 1);
    assert_eq!(res.rows[0][0], Value::Text("hello".to_string()));
    assert_eq!(res.rows[0][1], Value::Text("world".to_string()));
    // The virtual table shows up in sqlite_master.
    let res = x
        .execute("SELECT name FROM sqlite_master WHERE name = 'docs'")
        .unwrap();
    assert_eq!(res.rows.len(), 1);
}

/// CREATE VIRTUAL TABLE with module options (`tokenize = 'porter'`)
/// and IF NOT EXISTS.
#[test]
fn v312_76_create_virtual_table_options_and_if_not_exists() {
    let mut x = fresh();
    x.execute("CREATE VIRTUAL TABLE idx USING fts5(content, tokenize = 'porter')")
        .unwrap();
    // Re-create with IF NOT EXISTS must be a no-op, not an error.
    x.execute("CREATE VIRTUAL TABLE IF NOT EXISTS idx USING fts5(content)")
        .unwrap();
    let res = x.execute("SELECT content FROM idx").unwrap();
    assert_eq!(res.rows.len(), 0);
}

/// Regression: user tables still query normally (fast-path dispatch
/// must not swallow regular tables).
#[test]
fn v312_76_user_table_regression() {
    let mut x = fresh();
    x.execute("CREATE TABLE t (sql TEXT, n INT)").unwrap();
    x.execute("INSERT INTO t VALUES ('create stmt', 7)")
        .unwrap();
    let res = x.execute("SELECT sql, n FROM t").unwrap();
    assert_eq!(res.rows.len(), 1);
    assert_eq!(res.rows[0][0], Value::Text("create stmt".to_string()));
    assert_eq!(res.rows[0][1], Value::Integer(7));
}
