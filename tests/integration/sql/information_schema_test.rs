//! Tests for `information_schema` exposure (V312-56A / 56A-R2 / #4251).
//!
//! V312-56A acceptance criteria require `information_schema.tables`,
//! `information_schema.columns`, and `information_schema.indexes` to have
//! either:
//! - A real `SELECT FROM information_schema.<view>` SQL path, OR
//! - An explicit unsupported error
//!
//! 56A-R2 wires the real SQL path: the parser accepts the `schema.table`
//! dot-qualified form, and `execute_select` short-circuits to a virtual
//! catalog reader that projects the requested columns and applies
//! single-column equality + AND-chained WHERE filters.
//!
//! The tests below verify the wired path end-to-end. To populate the
//! in-memory `Catalog` we use the `sqlrustgo_catalog` API directly
//! (because the production CREATE TABLE path doesn't auto-register in
//! the catalog — that gap is a separate ticket tracked outside
//! V312-56A scope).
use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, StorageEngine};
use sqlrustgo_catalog::{
    index::IndexInfo, schema::Schema, Catalog, ColumnDefinition, DataType, Table,
};
use std::sync::Arc;

/// Build an engine wired with a Catalog containing two tables
/// (`users`, `orders`) in the default `public` schema and one extra
/// index on `users`. We build the schema bottom-up (Schema::new +
/// `.add_table()` consuming) and add it to the catalog's default
/// database. The default `Database::new` already creates an empty
/// `public` schema; since `Schema::add_table` consumes self we build
/// a fresh Schema rather than mutating the default in place.
fn engine_with_catalog() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));

    let users_table = Table::new(
        "users",
        vec![
            ColumnDefinition::new("id", DataType::Integer).not_null(),
            ColumnDefinition::new("name", DataType::Text).not_null(),
            ColumnDefinition::new("email", DataType::Text),
        ],
    )
    .primary_key(vec!["id".to_string()])
    .unwrap()
    .add_index(IndexInfo::new(
        "idx_users_email",
        "users",
        vec!["email".to_string()],
    ));

    let orders_table = Table::new(
        "orders",
        vec![
            ColumnDefinition::new("id", DataType::Integer).not_null(),
            ColumnDefinition::new("user_id", DataType::Integer).not_null(),
            ColumnDefinition::new("total", DataType::Float),
        ],
    )
    .primary_key(vec!["id".to_string()])
    .unwrap();

    // Build a populated schema from scratch (Schema::add_table consumes
    // self so we cannot re-use the default empty schema returned by
    // Database::new).
    let populated_schema = Schema::new("myschema")
        .add_table(users_table)
        .expect("add users")
        .add_table(orders_table)
        .expect("add orders");

    // `with_default_database` creates a fresh database with its own
    // empty default "public" schema. We add our populated "myschema"
    // schema alongside (no name collision).
    let mut catalog = Catalog::with_default_database("test_catalog", "test_db");
    catalog
        .get_database_mut("test_db")
        .expect("default db")
        .add_schema(populated_schema)
        .expect("add myschema");

    let catalog_arc = Arc::new(RwLock::new(catalog));
    ExecutionEngine::with_catalog(storage, catalog_arc)
}

#[test]
fn select_from_information_schema_tables_returns_registered_tables() {
    let mut e = engine_with_catalog();
    let r = e
        .execute("SELECT * FROM information_schema.tables")
        .unwrap();
    // Catalog has 1 schema ("public") × 2 tables = 2 rows.
    assert_eq!(
        r.rows.len(),
        2,
        "expected 2 rows in information_schema.tables (one per registered table), got {}",
        r.rows.len()
    );
    let table_names: Vec<String> = r
        .rows
        .iter()
        .map(|row| match &row[2] {
            sqlrustgo::Value::Text(s) => s.clone(),
            other => panic!("expected Text at column 2 (table_name), got {:?}", other),
        })
        .collect();
    assert!(table_names.contains(&"users".to_string()));
    assert!(table_names.contains(&"orders".to_string()));
}

#[test]
fn select_from_information_schema_columns_returns_columns() {
    let mut e = engine_with_catalog();
    let r = e
        .execute("SELECT column_name FROM information_schema.columns")
        .unwrap();
    // `users` has 3 cols, `orders` has 3 cols → 6 rows total.
    assert_eq!(
        r.rows.len(),
        6,
        "expected 6 column rows, got {}",
        r.rows.len()
    );
}

#[test]
fn select_from_information_schema_indexes_returns_indexes() {
    let mut e = engine_with_catalog();
    let r = e
        .execute("SELECT index_name FROM information_schema.indexes")
        .unwrap();
    // users has primary key (idx_pk_users) + idx_users_email → 2 rows.
    // (orders also has a primary key so it contributes 1 more.)
    assert!(
        r.rows.len() >= 2,
        "expected >=2 index rows, got {}",
        r.rows.len()
    );
    let names: Vec<String> = r
        .rows
        .iter()
        .filter_map(|row| match &row[0] {
            sqlrustgo::Value::Text(s) => Some(s.clone()),
            _ => None,
        })
        .collect();
    assert!(names.iter().any(|n| n.contains("users")));
}

#[test]
fn select_information_schema_tables_with_where_filters_to_one_row() {
    let mut e = engine_with_catalog();
    let r = e
        .execute("SELECT table_name FROM information_schema.tables WHERE table_name = 'users'")
        .unwrap();
    assert_eq!(
        r.rows.len(),
        1,
        "WHERE table_name = 'users' must yield exactly 1 row"
    );
    match &r.rows[0][0] {
        sqlrustgo::Value::Text(s) => assert_eq!(s, "users"),
        other => panic!("expected Text('users'), got {:?}", other),
    }
}

#[test]
fn select_information_schema_columns_with_table_name_filter() {
    let mut e = engine_with_catalog();
    let r = e
        .execute("SELECT column_name FROM information_schema.columns WHERE table_name = 'orders'")
        .unwrap();
    // orders has 3 columns: id, user_id, total
    assert_eq!(r.rows.len(), 3);
    let cols: Vec<String> = r
        .rows
        .iter()
        .filter_map(|row| match &row[0] {
            sqlrustgo::Value::Text(s) => Some(s.clone()),
            _ => None,
        })
        .collect();
    assert!(cols.contains(&"id".to_string()));
    assert!(cols.contains(&"user_id".to_string()));
    assert!(cols.contains(&"total".to_string()));
}

#[test]
fn select_information_schema_unknown_view_errors() {
    let mut e = engine_with_catalog();
    let r = e.execute("SELECT * FROM information_schema.does_not_exist");
    assert!(r.is_err(), "unknown information_schema view must error");
}

#[test]
fn select_information_schema_with_no_catalog_returns_empty_rows() {
    // Engine without a Catalog — virtual table must return empty rows,
    // not error (matches MySQL/PG behaviour for an unconfigured schema).
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut e = ExecutionEngine::new(storage);
    let r = e
        .execute("SELECT * FROM information_schema.tables")
        .unwrap();
    assert!(
        r.rows.is_empty(),
        "information_schema on a no-catalog engine must yield empty rows"
    );
}

#[test]
fn parser_accepts_information_schema_dot_qualified_table() {
    // Sanity check: the parser must accept the dot form without an
    // explicit catalog wired. We just ensure no parse error.
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut e = ExecutionEngine::new(storage);
    // Empty catalog → empty result, but parse must succeed.
    let r = e
        .execute("SELECT * FROM information_schema.tables")
        .unwrap();
    assert!(r.rows.is_empty());
}
