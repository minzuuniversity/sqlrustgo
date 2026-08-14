//! PR-SHOW-TABLES — P1 backlog fix for v3.7.0
//!
//! **Issue**: `SHOW TABLES` returned "Unsupported statement type" because
//! `ExecutionEngine::execute()` had no match arm for `Statement::Show`.
//! See docs/releases/v3.7.0/GA_GAP_REPORT.md §3.1.
//!
//! **Fix**: Add `Statement::Show` dispatch + `execute_show_databases` and
//! `execute_show_tables` handlers using `StorageEngine::list_tables()`.
//!
//! **Phase 2a migration**: driven through the wire protocol via the
//! embedded `start_ephemeral` harness (see
//! `openspec/changes/mysql-server-canonical-entry/specs/wire-protocol-execution/spec.md`).

#[path = "../../common/mod.rs"]
mod common;

use common::MySqlTestClient;
use sqlrustgo_mysql_server::testing::EphemeralConfig;

fn clean_client() -> MySqlTestClient {
    MySqlTestClient::connect_with_config(EphemeralConfig {
        bootstrap_tables: false,
        slow_query_log: None,
        metrics_port: None,
        ..EphemeralConfig::default()
    })
    .expect("ephemeral server (clean catalog) + raw client should come up")
}

#[test]
fn show_tables_on_empty_db_returns_empty_result() {
    let mut client = clean_client();
    let rows = client
        .query_rows("SHOW TABLES")
        .expect("SHOW TABLES should succeed");
    assert_eq!(
        rows.len(),
        0,
        "SHOW TABLES on empty DB should return 0 rows, got {}",
        rows.len()
    );
}

#[test]
fn show_tables_lists_all_created_tables() {
    let mut client = clean_client();
    client
        .exec("CREATE TABLE t1 (id INTEGER)")
        .expect("CREATE t1");
    client
        .exec("CREATE TABLE t2 (id INTEGER, name TEXT)")
        .expect("CREATE t2");
    client
        .exec("CREATE TABLE t3 (id INTEGER)")
        .expect("CREATE t3");

    let rows = client
        .query_rows("SHOW TABLES")
        .expect("SHOW TABLES should succeed");
    assert_eq!(rows.len(), 3, "expected 3 tables, got {:?}", rows);
    let names: Vec<&str> = rows.iter().map(|r| r[0].as_str()).collect();
    assert!(names.contains(&"t1"));
    assert!(names.contains(&"t2"));
    assert!(names.contains(&"t3"));
}

#[test]
fn show_databases_returns_one_row() {
    let mut client = clean_client();
    let rows = client
        .query_rows("SHOW DATABASES")
        .expect("SHOW DATABASES should succeed");
    assert!(
        !rows.is_empty(),
        "SHOW DATABASES should return at least one row, got 0"
    );
}

#[test]
fn show_tables_after_drop_reflects_drop() {
    let mut client = clean_client();
    client
        .exec("CREATE TABLE keep_me (id INTEGER)")
        .expect("CREATE keep_me");
    client
        .exec("CREATE TABLE drop_me (id INTEGER)")
        .expect("CREATE drop_me");
    client.exec("DROP TABLE drop_me").expect("DROP drop_me");

    let rows = client
        .query_rows("SHOW TABLES")
        .expect("SHOW TABLES should succeed");
    let names: Vec<&str> = rows.iter().map(|r| r[0].as_str()).collect();
    assert!(names.contains(&"keep_me"));
    assert!(
        !names.contains(&"drop_me"),
        "drop_me should not appear after DROP"
    );
}

#[test]
fn show_columns_returns_column_metadata() {
    let mut client = clean_client();
    client
        .exec("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL, email TEXT)")
        .expect("CREATE users table");

    let rows = client
        .query_rows("SHOW COLUMNS FROM users")
        .expect("SHOW COLUMNS should succeed");

    assert_eq!(rows.len(), 3, "expected 3 columns, got {:?}", rows.len());

    let fields: Vec<&str> = rows.iter().map(|r| r[0].as_str()).collect();
    assert!(fields.contains(&"id"));
    assert!(fields.contains(&"name"));
    assert!(fields.contains(&"email"));
}

#[test]
fn show_columns_with_like_pattern() {
    let mut client = clean_client();
    client
        .exec("CREATE TABLE products (id INTEGER, name TEXT, price FLOAT, description TEXT)")
        .expect("CREATE products table");

    let rows = client
        .query_rows("SHOW COLUMNS FROM products LIKE 'name'")
        .expect("SHOW COLUMNS LIKE should succeed");

    assert_eq!(
        rows.len(),
        1,
        "expected 1 column matching 'name', got {}",
        rows.len()
    );
    assert_eq!(rows[0][0].as_str(), "name");
}

#[test]
fn show_columns_nonexistent_table_returns_error() {
    let mut client = clean_client();
    let result = client.query_rows("SHOW COLUMNS FROM nonexistent");
    assert!(
        result.is_err(),
        "SHOW COLUMNS for nonexistent table should fail"
    );
}

#[test]
fn show_index_on_table_without_catalog_returns_empty() {
    let mut client = clean_client();
    client
        .exec("CREATE TABLE orders (id INTEGER PRIMARY KEY, user_id INTEGER, total FLOAT)")
        .expect("CREATE orders table");

    let rows = client
        .query_rows("SHOW INDEX FROM orders")
        .expect("SHOW INDEX should succeed");

    assert!(
        rows.is_empty(),
        "expected empty (no catalog), got {} rows: {:?}",
        rows.len(),
        rows
    );
}

#[test]
fn show_index_nonexistent_table_returns_error() {
    let mut client = clean_client();
    let result = client.query_rows("SHOW INDEX FROM nonexistent");
    assert!(
        result.is_err(),
        "SHOW INDEX for nonexistent table should fail"
    );
}

#[test]
fn describe_table_returns_columns() {
    let mut client = clean_client();
    client
        .exec("CREATE TABLE items (id INTEGER PRIMARY KEY, data TEXT)")
        .expect("CREATE items table");

    let rows = client
        .query_rows("DESCRIBE items")
        .expect("DESCRIBE should succeed");

    assert_eq!(rows.len(), 2, "expected 2 columns, got {}", rows.len());
    assert_eq!(rows[0][0].as_str(), "id");
    assert_eq!(rows[1][0].as_str(), "data");
}
