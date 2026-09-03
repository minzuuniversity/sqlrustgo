//! V312-64d / Issue #4664: regression tests for system-table introspection.
//!
//! Covers:
//!   - `sqlite_master` (and `sqlite_schema` alias) listing every user
//!     table/view/index/trigger with the original `CREATE …` SQL
//!   - `mysql.user` / `mysql.db` exposing the catalog's auth manager
//!   - WHERE filter on the system-table query (`type = 'table'`, etc.)
//!   - Unknown system-table names fall through to the regular scan path

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, Value};
use std::sync::Arc;

fn fresh_mem() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

fn fresh_mem_with_catalog() -> (
    ExecutionEngine<MemoryStorage>,
    Arc<RwLock<sqlrustgo_catalog::Catalog>>,
) {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let catalog = sqlrustgo_catalog::Catalog::new("main");
    let arc = Arc::new(RwLock::new(catalog));
    let engine = ExecutionEngine::with_catalog(storage, Arc::clone(&arc));
    (engine, arc)
}

fn as_text(v: &Value) -> String {
    match v {
        Value::Text(s) => s.clone(),
        Value::Integer(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Boolean(b) => b.to_string(),
        Value::Null => "NULL".to_string(),
        Value::Blob(b) => format!("<blob {} bytes>", b.len()),
        Value::Point(x, y) => format!("POINT({}, {})", x, y),
        Value::Json(s) => s.to_string(),
    }
}

fn row_text(row: &[Value]) -> Vec<String> {
    row.iter().map(as_text).collect()
}

#[test]
fn sqlite_master_empty_db() {
    let mut x = fresh_mem();
    let r = x.execute("SELECT * FROM sqlite_master").unwrap();
    assert_eq!(r.rows.len(), 0, "fresh DB must have no objects");
}

#[test]
fn sqlite_master_after_create_table() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(id INT PRIMARY KEY, name TEXT)")
        .unwrap();
    let r = x
        .execute("SELECT \"type\", name, \"sql\" FROM sqlite_master")
        .unwrap();
    assert_eq!(r.rows.len(), 1);
    let row = row_text(&r.rows[0]);
    assert_eq!(row[0], "table");
    assert_eq!(row[1], "t");
    assert!(
        row[2].to_uppercase().contains("CREATE TABLE"),
        "sql column should carry the original CREATE TABLE statement, got: {}",
        row[2]
    );
    assert!(row[2].contains("id"));
}

#[test]
fn sqlite_master_after_create_view() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE base(id INT, val INT)").unwrap();
    x.execute("INSERT INTO base VALUES (1, 100), (2, 200)")
        .unwrap();
    x.execute("CREATE VIEW v_base AS SELECT id, val FROM base")
        .unwrap();
    let r = x
        .execute("SELECT \"type\", name FROM sqlite_master ORDER BY \"type\", name")
        .unwrap();
    let rows: Vec<Vec<String>> = r.rows.iter().map(|r| row_text(r)).collect();
    assert!(rows
        .iter()
        .any(|row| row[0] == "view" && row[1] == "v_base"));
    assert!(rows.iter().any(|row| row[0] == "table" && row[1] == "base"));
}

#[test]
fn sqlite_master_after_create_index() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(id INT, val INT)").unwrap();
    x.execute("CREATE INDEX idx_t_val ON t(val)").unwrap();
    let r = x
        .execute("SELECT \"type\", name, tbl_name FROM sqlite_master WHERE \"type\" = 'index'")
        .unwrap();
    assert_eq!(r.rows.len(), 1, "expected exactly one index row");
    let row = row_text(&r.rows[0]);
    assert_eq!(row[0], "index");
    assert_eq!(row[1], "idx_t_val");
    assert_eq!(row[2], "t");
}

#[test]
fn sqlite_master_after_create_trigger() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(id INT)").unwrap();
    x.execute("CREATE TABLE log(msg TEXT)").unwrap();
    x.execute(
        "CREATE TRIGGER tr AFTER INSERT ON t FOR EACH ROW \
         BEGIN INSERT INTO log(msg) VALUES ('fired'); END",
    )
    .unwrap();
    let r = x
        .execute("SELECT \"type\", name, tbl_name FROM sqlite_master WHERE \"type\" = 'trigger'")
        .unwrap();
    assert_eq!(r.rows.len(), 1);
    let row = row_text(&r.rows[0]);
    assert_eq!(row[0], "trigger");
    assert_eq!(row[1], "tr");
    assert_eq!(row[2], "t");
}

#[test]
fn sqlite_master_where_type_eq_table() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t1(id INT)").unwrap();
    x.execute("CREATE TABLE t2(id INT)").unwrap();
    x.execute("CREATE TABLE t3(id INT)").unwrap();
    let r = x
        .execute("SELECT name FROM sqlite_master WHERE \"type\" = 'table' ORDER BY name")
        .unwrap();
    let names: Vec<String> = r.rows.iter().map(|row| as_text(&row[0])).collect();
    assert_eq!(
        names,
        vec!["t1".to_string(), "t2".to_string(), "t3".to_string()]
    );
}

#[test]
fn sqlite_master_four_objects_in_one_session() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(id INT, val INT)").unwrap();
    x.execute("CREATE VIEW v AS SELECT id FROM t").unwrap();
    x.execute("CREATE INDEX idx_v ON t(val)").unwrap();
    x.execute("CREATE TABLE log(msg TEXT)").unwrap();
    x.execute(
        "CREATE TRIGGER tr AFTER INSERT ON t FOR EACH ROW \
         BEGIN INSERT INTO log(msg) VALUES ('x'); END",
    )
    .unwrap();
    let r = x
        .execute("SELECT \"type\", name FROM sqlite_master")
        .unwrap();
    let mut by_type: std::collections::HashMap<String, Vec<String>> = Default::default();
    for row in &r.rows {
        let cells = row_text(row);
        by_type
            .entry(cells[0].clone())
            .or_default()
            .push(cells[1].clone());
    }
    assert!(by_type
        .get("table")
        .unwrap_or(&vec![])
        .contains(&"t".to_string()));
    assert!(by_type
        .get("table")
        .unwrap_or(&vec![])
        .contains(&"log".to_string()));
    assert!(by_type
        .get("view")
        .unwrap_or(&vec![])
        .contains(&"v".to_string()));
    assert!(by_type
        .get("index")
        .unwrap_or(&vec![])
        .contains(&"idx_v".to_string()));
    assert!(by_type
        .get("trigger")
        .unwrap_or(&vec![])
        .contains(&"tr".to_string()));
}

#[test]
fn sqlite_schema_alias_matches_sqlite_master() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(id INT)").unwrap();
    let r_master = x.execute("SELECT name FROM sqlite_master").unwrap();
    let r_schema = x.execute("SELECT name FROM sqlite_schema").unwrap();
    assert_eq!(r_master.rows.len(), r_schema.rows.len());
    let mut a: Vec<String> = r_master.rows.iter().map(|r| as_text(&r[0])).collect();
    let mut b: Vec<String> = r_schema.rows.iter().map(|r| as_text(&r[0])).collect();
    a.sort();
    b.sort();
    assert_eq!(a, b);
}

#[test]
fn mysql_user_select_with_auth() {
    let (mut x, _catalog) = fresh_mem_with_catalog();
    let r = x
        .execute("SELECT \"user\", host FROM mysql.\"user\"")
        .unwrap();
    assert_eq!(r.rows.len(), 0);
}

#[test]
fn mysql_db_select_returns_db_metadata() {
    let (mut x, _catalog) = fresh_mem_with_catalog();
    let r = x
        .execute("SELECT host, \"db\", \"user\" FROM mysql.\"db\"")
        .unwrap();
    assert_eq!(r.rows.len(), 0);
}

#[test]
fn unknown_system_table_falls_through() {
    let mut x = fresh_mem();
    let r = x.execute("SELECT * FROM information_schema.not_a_real_view");
    assert!(r.is_err(), "unknown system-table must surface an error");
}

#[test]
fn sqlite_master_sql_round_trips_columns() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(id INT PRIMARY KEY, name TEXT NOT NULL, qty INT)")
        .unwrap();
    let r = x
        .execute("SELECT \"sql\" FROM sqlite_master WHERE name = 't'")
        .unwrap();
    assert_eq!(r.rows.len(), 1);
    let sql = as_text(&r.rows[0][0]);
    assert!(sql.contains("id"), "sql must contain 'id' column: {}", sql);
    assert!(
        sql.contains("name"),
        "sql must contain 'name' column: {}",
        sql
    );
    assert!(
        sql.contains("qty"),
        "sql must contain 'qty' column: {}",
        sql
    );
    assert!(
        sql.contains("PRIMARY KEY"),
        "sql must surface PRIMARY KEY: {}",
        sql
    );
}

#[test]
fn sqlite_master_projection_with_unknown_column_errors() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(id INT)").unwrap();
    let r = x.execute("SELECT bogus FROM sqlite_master");
    assert!(r.is_err(), "unknown column must error");
    let msg = format!("{}", r.unwrap_err());
    assert!(
        msg.to_lowercase().contains("unknown") || msg.to_lowercase().contains("bogus"),
        "error must mention the unknown column, got: {}",
        msg
    );
}

#[test]
fn sqlite_master_ignores_internal_table_names() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE real_table(id INT)").unwrap();
    let r = x.execute("SELECT name FROM sqlite_master").unwrap();
    for row in &r.rows {
        let name = as_text(&row[0]);
        assert_ne!(
            name.to_ascii_lowercase(),
            "sqlite_master",
            "sqlite_master must not appear as a user-table row: {:?}",
            name
        );
    }
}
