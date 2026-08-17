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

// ============================================================================
// SHOW CREATE TABLE — V312-56A / 56A-R1
// ============================================================================
//
// Integration tests for `SHOW CREATE TABLE` (V312-56A residual #4251 / 56A-R1).
// Parser ✅ (`crates/parser/tests/parser_coverage_tests.rs:1383`) and
// executor ✅ (`execute_show_create_table` in `src/engine_ddl.rs:415`).
// This file closes the integration-coverage gap.

#[test]
fn show_create_table_returns_single_ddl_row() {
    let mut client = clean_client();
    client
        .exec("CREATE TABLE widgets (id INTEGER PRIMARY KEY, name TEXT)")
        .expect("CREATE widgets table");

    let rows = client
        .query_rows("SHOW CREATE TABLE widgets")
        .expect("SHOW CREATE TABLE should succeed");

    assert_eq!(
        rows.len(),
        1,
        "SHOW CREATE TABLE should return exactly 1 row, got {}",
        rows.len()
    );
    let ddl = rows[0][0].as_str();
    assert!(
        ddl.starts_with("CREATE TABLE widgets ("),
        "DDL should start with table name + opening paren, got: {}",
        ddl
    );
    assert!(ddl.contains("id"), "DDL should contain `id` column, got: {}", ddl);
    assert!(
        ddl.contains("name"),
        "DDL should contain `name` column, got: {}",
        ddl
    );
}

#[test]
fn show_create_table_preserves_not_null_clause() {
    let mut client = clean_client();
    client
        .exec(
            "CREATE TABLE accounts (id INTEGER PRIMARY KEY, login TEXT NOT NULL, bio TEXT)",
        )
        .expect("CREATE accounts table");

    let rows = client
        .query_rows("SHOW CREATE TABLE accounts")
        .expect("SHOW CREATE TABLE should succeed");

    assert_eq!(rows.len(), 1);
    let ddl = rows[0][0].as_str();
    assert!(
        ddl.contains("login") && ddl.contains("NOT NULL"),
        "DDL should preserve `NOT NULL` on `login`, got: {}",
        ddl
    );
    // `bio` is nullable — it should NOT carry NOT NULL.
    assert!(
        ddl.contains("bio") && !ddl.contains("bio NOT NULL"),
        "DDL should NOT mark nullable `bio` as NOT NULL, got: {}",
        ddl
    );
}

#[test]
fn show_create_table_nonexistent_returns_error() {
    let mut client = clean_client();
    let result = client.query_rows("SHOW CREATE TABLE ghost_table");
    assert!(
        result.is_err(),
        "SHOW CREATE TABLE for a nonexistent table should fail"
    );
}

#[test]
fn show_create_table_after_alter_adds_new_column() {
    let mut client = clean_client();
    client
        .exec("CREATE TABLE ledger (id INTEGER PRIMARY KEY, amount INTEGER)")
        .expect("CREATE ledger table");

    // Capture pre-alter DDL.
    let pre = client
        .query_rows("SHOW CREATE TABLE ledger")
        .expect("SHOW CREATE TABLE before ALTER");
    assert_eq!(pre.len(), 1);
    let pre_ddl = pre[0][0].as_str();
    assert!(pre_ddl.contains("amount"));
    assert!(
        !pre_ddl.contains("memo"),
        "pre-alter DDL should not mention memo, got: {}",
        pre_ddl
    );

    // Add a column and re-introspect.
    client
        .exec("ALTER TABLE ledger ADD COLUMN memo TEXT")
        .expect("ALTER TABLE ADD COLUMN");

    let post = client
        .query_rows("SHOW CREATE TABLE ledger")
        .expect("SHOW CREATE TABLE after ALTER");
    assert_eq!(post.len(), 1);
    let post_ddl = post[0][0].as_str();
    assert!(
        post_ddl.contains("memo"),
        "post-alter DDL should contain `memo`, got: {}",
        post_ddl
    );
    assert!(
        post_ddl.contains("amount"),
        "post-alter DDL should still contain `amount`, got: {}",
        post_ddl
    );
}

#[test]
fn show_create_table_roundtrip_yields_equivalent_schema() {
    let mut client = clean_client();
    client
        .exec(
            "CREATE TABLE roundtrip_src (id INTEGER PRIMARY KEY, label TEXT NOT NULL, score INTEGER)",
        )
        .expect("CREATE roundtrip_src");

    let rows = client
        .query_rows("SHOW CREATE TABLE roundtrip_src")
        .expect("SHOW CREATE TABLE roundtrip_src");
    assert_eq!(rows.len(), 1);
    let ddl = rows[0][0].as_str();

    // The introspected DDL must contain every column from the original CREATE
    // (the executor currently does not preserve PRIMARY KEY in the
    // reconstructed DDL — that limitation is documented separately; this
    // test pins the column-list invariant only).
    assert!(ddl.contains("id"), "DDL must contain `id`, got: {}", ddl);
    assert!(
        ddl.contains("label"),
        "DDL must contain `label`, got: {}",
        ddl
    );
    assert!(
        ddl.contains("score"),
        "DDL must contain `score`, got: {}",
        ddl
    );
    // NOT NULL on `label` must round-trip through the executor.
    assert!(
        ddl.contains("label TEXT NOT NULL"),
        "DDL must preserve NOT NULL on `label`, got: {}",
        ddl
    );

    // DESCRIBE must still report the original 3 columns.
    let cols = client
        .query_rows("DESCRIBE roundtrip_src")
        .expect("DESCRIBE roundtrip_src");
    assert_eq!(cols.len(), 3, "expected 3 columns, got {}", cols.len());
    let col_names: Vec<&str> = cols.iter().map(|r| r[0].as_str()).collect();
    assert!(col_names.contains(&"id"));
    assert!(col_names.contains(&"label"));
    assert!(col_names.contains(&"score"));
}
